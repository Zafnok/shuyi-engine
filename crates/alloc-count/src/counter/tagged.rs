//! [`CountingAlloc::with_tags`]: a counter that also counts by memory tag
//! ([`super::tags`]), each block carrying its tag in a header in front of
//! it.

use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

use super::tags::{self, TAGS};
use super::{CountingAlloc, Word};
use crate::TagCounts;

/// What a [`CountingAlloc::with_tags`] counts by tag: for each of the
/// [`TAGS`], its live bytes and blocks, the bytes ever allocated under
/// it, and the peak of its live bytes.
///
/// **The header.** Each block carries the tag it counts under in a header
/// in front of it of `max(align, 8)` bytes, for the block's alignment
/// `align`: the block keeps its alignment, and the tag (a `u16`) sits in the header's
/// last two bytes, just before the block. The inner allocator is asked
/// for the block and its header, at the block's alignment. The counts
/// are the bytes asked for: a header's bytes are never counted.
pub struct Tagged {
    tags: [TagWords; TAGS],
}

impl core::fmt::Debug for Tagged {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Tagged").finish_non_exhaustive()
    }
}

/// One tag's counts.
struct TagWords {
    live: Word,
    blocks: Word,
    allocated: Word,
    peak: Word,
}

impl TagWords {
    const fn new() -> Self {
        Self {
            live: Word::new(),
            blocks: Word::new(),
            allocated: Word::new(),
            peak: Word::new(),
        }
    }
}

impl Tagged {
    const fn new() -> Self {
        Self {
            tags: [const { TagWords::new() }; TAGS],
        }
    }

    /// A block of `size` bytes now counts under `tag`.
    fn add(&self, tag: u16, size: usize) {
        let words = &self.tags[tags::slot(tag)];
        let live = words.live.add(size);
        words.peak.max(live);
        words.blocks.add(1);
        words.allocated.add(size);
    }

    /// A block of `size` bytes no longer counts under `tag`.
    fn remove(&self, tag: u16, size: usize) {
        let words = &self.tags[tags::slot(tag)];
        words.live.sub(size);
        words.blocks.sub(1);
    }
}

/// The header in front of a block of alignment `align`: `align` bytes, or
/// 8 if that is less, so the block after it keeps its alignment.
pub(crate) fn header_bytes(align: usize) -> usize {
    align.max(8)
}

/// The layout asked of the inner allocator for `layout`'s block and its
/// header, and the header's bytes; `None` if that is too big for a
/// layout.
fn outer(layout: Layout) -> Option<(Layout, usize)> {
    let header = header_bytes(layout.align());
    let size = layout.size().checked_add(header)?;
    let outer = Layout::from_size_align(size, layout.align()).ok()?;
    Some((outer, header))
}

/// Writes `tag` in the header in front of `block`.
///
/// # Safety
///
/// `block` is a block of this module's, with its header in front of it.
unsafe fn write_tag(block: *mut u8, tag: u16) {
    // SAFETY: the header is at least 8 bytes in front of `block`, inside
    // the inner allocator's block, so its last two are ours to write; they
    // may be unaligned (a block of alignment 1).
    unsafe { block.sub(2).cast::<u16>().write_unaligned(tag) };
}

/// The tag in the header in front of `block`.
///
/// # Safety
///
/// As [`write_tag`], with a tag written.
unsafe fn read_tag(block: *mut u8) -> u16 {
    // SAFETY: as `write_tag`'s, and `write_tag` wrote these two bytes.
    unsafe { block.sub(2).cast::<u16>().read_unaligned() }
}

impl<A> CountingAlloc<A, Tagged> {
    /// A counter around `inner` that also counts by memory tag
    /// ([`tags`](crate::tags)): each block counts under the tag current
    /// on its thread when it was allocated, or last reallocated. It costs
    /// a header in front of each block ([`Tagged`]).
    pub const fn with_tags(inner: A) -> Self {
        Self {
            inner,
            live: Word::new(),
            peak: Word::new(),
            allocs: Word::new(),
            tags: Tagged::new(),
        }
    }

    /// Every tag with bytes live or ever allocated under it, by number,
    /// with its class, name and counts. It allocates the list: never call
    /// it from inside an allocator.
    pub fn rows(&self) -> Vec<TagCounts> {
        let names = tags::names();
        let wide = |n: usize| u64::try_from(n).unwrap_or(u64::MAX);
        let mut rows = Vec::new();
        for ((tag, words), (class, name)) in (0u16..).zip(&self.tags.tags).zip(names) {
            let (live, allocated) = (words.live.get(), words.allocated.get());
            if live == 0 && allocated == 0 {
                continue;
            }
            let mut row = TagCounts::new(tag, class, name);
            row.live_bytes = wide(live);
            row.live_blocks = u32::try_from(words.blocks.get()).unwrap_or(u32::MAX);
            row.allocated_bytes = wide(allocated);
            row.peak_bytes = wide(words.peak.get());
            rows.push(row);
        }
        rows
    }
}

impl<A: GlobalAlloc> CountingAlloc<A, Tagged> {
    /// A block for `layout` behind its header, from `inner` (zeroed or
    /// not), tagged with this thread's current tag and counted.
    ///
    /// # Safety
    ///
    /// `alloc`'s contract: `layout`'s size is above zero.
    unsafe fn alloc_tagged(&self, layout: Layout, zeroed: bool) -> *mut u8 {
        self.allocs.add(1);
        let Some((outer, header)) = outer(layout) else {
            return ptr::null_mut();
        };
        // SAFETY: `outer` is bigger than `layout`, so above zero.
        let base = unsafe {
            if zeroed {
                self.inner.alloc_zeroed(outer)
            } else {
                self.inner.alloc(outer)
            }
        };
        if base.is_null() {
            return base;
        }
        // SAFETY: `base` has `header` bytes and the block's after them.
        let block = unsafe { base.add(header) };
        let tag = tags::current();
        // SAFETY: `block` has its header in front of it.
        unsafe { write_tag(block, tag) };
        self.grew(layout.size());
        self.tags.add(tag, layout.size());
        block
    }
}

// SAFETY: every block handed out is inside a block `inner` gave for a
// layout of the same alignment, `header_bytes` after its start: a multiple
// of the alignment, so the block is aligned, and the block's size fits
// after it. Each call gives `inner` back the start and layout it gave.
unsafe impl<A: GlobalAlloc> GlobalAlloc for CountingAlloc<A, Tagged> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller keeps `alloc`'s contract.
        unsafe { self.alloc_tagged(layout, false) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: as `alloc`.
        unsafe { self.alloc_tagged(layout, true) }
    }

    unsafe fn dealloc(&self, block: *mut u8, layout: Layout) {
        // A block allocated with `layout` had an outer layout.
        let Some((outer, header)) = outer(layout) else {
            return;
        };
        // SAFETY: the caller says `block` came from this allocator with
        // `layout`, so `alloc_tagged` gave it, with its header, `header`
        // bytes after the start `inner` gave for `outer`.
        let tag = unsafe { read_tag(block) };
        // SAFETY: as above: the start and layout `inner` gave.
        unsafe { self.inner.dealloc(block.sub(header), outer) };
        self.live.sub(layout.size());
        self.tags.remove(tag, layout.size());
    }

    unsafe fn realloc(&self, block: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.allocs.add(1);
        let Some((outer, header)) = outer(layout) else {
            return ptr::null_mut();
        };
        // The new outer size must make a layout too, or nothing changes.
        let new_outer = new_size.checked_add(header);
        let Some(new_outer) =
            new_outer.filter(|&n| Layout::from_size_align(n, layout.align()).is_ok())
        else {
            return ptr::null_mut();
        };
        // SAFETY: as `dealloc`'s: `block` is this allocator's, with
        // `layout`, `header` bytes after the start `inner` gave for
        // `outer`; and `new_outer` makes a valid layout at its alignment.
        let old_tag = unsafe { read_tag(block) };
        // SAFETY: as above.
        let base = unsafe { self.inner.realloc(block.sub(header), outer, new_outer) };
        if base.is_null() {
            // The block is as it was, under its tag.
            return base;
        }
        // SAFETY: `inner` kept the header's bytes and the block's (up to
        // the smaller size) at `base`, so the block is `header` after it.
        let moved = unsafe { base.add(header) };
        let tag = tags::current();
        // SAFETY: `moved` has its header in front of it.
        unsafe { write_tag(moved, tag) };
        self.resized(layout.size(), new_size);
        self.tags.remove(old_tag, layout.size());
        self.tags.add(tag, new_size);
        moved
    }
}
