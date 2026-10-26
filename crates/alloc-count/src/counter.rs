//! [`CountingAlloc`], its memory tags and the words it counts in: this
//! crate's only `unsafe` (ADR-0046 §9), each block with the reason it
//! holds.

use core::alloc::{GlobalAlloc, Layout};

use crate::Counts;

mod tagged;
pub mod tags;

pub use tagged::Tagged;

/// A global allocator around `inner` that counts what is asked of it: the
/// bytes live, their peak, and the calls to `alloc`, `alloc_zeroed` and
/// `realloc`. It counts each [`Layout`]'s size, never the inner
/// allocator's own overhead.
///
/// Make it a `static`, with `#[global_allocator]` to count a whole
/// program, or called directly as a test does. A program with
/// `shuyi-app` or `shuyi-cli` already has one (the `global` module says
/// which).
///
/// ```
/// use shuyi_alloc_count::CountingAlloc;
/// use std::alloc::System;
///
/// static HEAP: CountingAlloc<System> = CountingAlloc::new(System);
///
/// assert_eq!(HEAP.live_bytes(), 0);
/// ```
///
/// [`CountingAlloc::new`] counts the whole; [`CountingAlloc::with_tags`]
/// also counts by memory tag ([`tags`]), with a header in front of each
/// block ([`Tagged`]).
///
/// On a target with atomics each count is one atomic read-modify-write, so
/// any thread may allocate; on one without (an old ARM console) the
/// counts are plain cells, and the target has one thread.
#[derive(Debug)]
pub struct CountingAlloc<A, T = Untagged> {
    inner: A,
    live: Word,
    peak: Word,
    allocs: Word,
    tags: T,
}

/// What a [`CountingAlloc::new`] counts by tag: nothing, and no header in
/// front of a block.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Untagged;

impl<A> CountingAlloc<A> {
    /// A counter around `inner`, nothing counted yet. It counts the whole,
    /// with no header in front of a block: for a bench, and a console that
    /// counts its heap without tags.
    pub const fn new(inner: A) -> Self {
        Self {
            inner,
            live: Word::new(),
            peak: Word::new(),
            allocs: Word::new(),
            tags: Untagged,
        }
    }
}

impl<A, T> CountingAlloc<A, T> {
    /// The bytes allocated and not yet freed.
    pub fn live_bytes(&self) -> usize {
        self.live.get()
    }

    /// The most bytes live at once since the start or the last
    /// [`reset_peak`](Self::reset_peak).
    pub fn peak_bytes(&self) -> usize {
        self.peak.get()
    }

    /// Starts the peak again from the bytes live now. A tag's peak
    /// ([`TagCounts::peak_bytes`](crate::TagCounts::peak_bytes)) stays.
    pub fn reset_peak(&self) {
        self.peak.set(self.live.get());
    }

    /// The calls to `alloc`, `alloc_zeroed` and `realloc` since the last
    /// take (or the start), read and reset in one step; at most
    /// `u32::MAX`.
    pub fn take_allocs(&self) -> u32 {
        u32::try_from(self.allocs.take()).unwrap_or(u32::MAX)
    }

    /// All three: the live bytes, their peak, and the calls, taken
    /// ([`take_allocs`](Self::take_allocs)).
    pub fn counts(&self) -> Counts {
        let wide = |n: usize| u64::try_from(n).unwrap_or(u64::MAX);
        Counts::new(
            wide(self.live_bytes()),
            wide(self.peak_bytes()),
            self.take_allocs(),
        )
    }

    /// Counts a call that asked for `size` bytes and got `ptr`.
    fn allocated(&self, ptr: *mut u8, size: usize) {
        self.allocs.add(1);
        if !ptr.is_null() {
            self.grew(size);
        }
    }

    /// `size` more bytes live, and the peak raised to them.
    fn grew(&self, size: usize) {
        let live = self.live.add(size);
        self.peak.max(live);
    }

    /// A block of `old` bytes now `new` bytes: the live bytes moved by the
    /// difference.
    fn resized(&self, old: usize, new: usize) {
        if new >= old {
            self.grew(new - old);
        } else {
            self.live.sub(old - new);
        }
    }
}

// SAFETY: every method hands its call to `inner` unchanged, under the
// caller's own guarantees, and only counts around it: the memory it gives
// is `inner`'s, as `inner` gives it.
unsafe impl<A: GlobalAlloc> GlobalAlloc for CountingAlloc<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller keeps `alloc`'s contract (a size above zero),
        // which is all `inner.alloc` asks.
        let ptr = unsafe { self.inner.alloc(layout) };
        self.allocated(ptr, layout.size());
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: as `alloc`: the caller's layout, unchanged.
        let ptr = unsafe { self.inner.alloc_zeroed(layout) };
        self.allocated(ptr, layout.size());
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller says `ptr` came from this allocator, so from
        // `inner`, with this `layout`.
        unsafe { self.inner.dealloc(ptr, layout) };
        self.live.sub(layout.size());
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller says `ptr` came from this allocator, so from
        // `inner`, with `layout`, and that `new_size` is valid for it.
        let moved = unsafe { self.inner.realloc(ptr, layout, new_size) };
        self.allocs.add(1);
        if !moved.is_null() {
            self.resized(layout.size(), new_size);
        }
        moved
    }
}

/// One count, a machine word, on a target with atomics: each change one
/// atomic read-modify-write.
#[cfg(target_has_atomic = "ptr")]
#[derive(Debug)]
struct Word(core::sync::atomic::AtomicUsize);

#[cfg(target_has_atomic = "ptr")]
impl Word {
    const fn new() -> Self {
        Self(core::sync::atomic::AtomicUsize::new(0))
    }

    fn get(&self) -> usize {
        self.0.load(core::sync::atomic::Ordering::Relaxed)
    }

    fn set(&self, n: usize) {
        self.0.store(n, core::sync::atomic::Ordering::Relaxed);
    }

    /// Adds `n`, wrapping, and gives the new count.
    fn add(&self, n: usize) -> usize {
        let old = self.0.fetch_add(n, core::sync::atomic::Ordering::Relaxed);
        old.wrapping_add(n)
    }

    fn sub(&self, n: usize) {
        self.0.fetch_sub(n, core::sync::atomic::Ordering::Relaxed);
    }

    /// Raises the count to `n`, if `n` is more.
    fn max(&self, n: usize) {
        self.0.fetch_max(n, core::sync::atomic::Ordering::Relaxed);
    }

    /// The count, and 0 in its place.
    fn take(&self) -> usize {
        self.0.swap(0, core::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(not(target_has_atomic = "ptr"))]
use single::Word;

/// One count as a plain cell, for a target without atomics: such a target
/// (the GBA, the DS) runs one thread, so no two threads ever share it.
/// Compiled for the tests too, which call it from one thread.
#[cfg(any(test, not(target_has_atomic = "ptr")))]
mod single {
    use core::cell::Cell;

    #[derive(Debug)]
    pub(super) struct Word(Cell<usize>);

    // SAFETY: only a target without atomics shares it, as a `static`'s
    // count, and such a target has one thread: no two threads ever reach
    // the cell.
    #[cfg(not(target_has_atomic = "ptr"))]
    unsafe impl Sync for Word {}

    impl Word {
        pub(super) const fn new() -> Self {
            Self(Cell::new(0))
        }

        pub(super) fn get(&self) -> usize {
            self.0.get()
        }

        pub(super) fn set(&self, n: usize) {
            self.0.set(n);
        }

        /// Adds `n`, wrapping, and gives the new count.
        pub(super) fn add(&self, n: usize) -> usize {
            let new = self.0.get().wrapping_add(n);
            self.0.set(new);
            new
        }

        pub(super) fn sub(&self, n: usize) {
            self.0.set(self.0.get().wrapping_sub(n));
        }

        /// Raises the count to `n`, if `n` is more.
        pub(super) fn max(&self, n: usize) {
            self.0.set(self.0.get().max(n));
        }

        /// The count, and 0 in its place.
        pub(super) fn take(&self) -> usize {
            self.0.replace(0)
        }
    }
}

#[cfg(test)]
mod tests;
