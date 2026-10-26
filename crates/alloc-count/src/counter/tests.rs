//! An instance called directly, never the global one, which other test
//! threads use too.

use super::*;
use proptest::prelude::*;
use std::alloc::System;

const MB: usize = 1 << 20;

fn layout(size: usize) -> Layout {
    Layout::from_size_align(size, 8).unwrap()
}

/// A block of `size` bytes from `heap`.
fn alloc(heap: &CountingAlloc<System>, size: usize) -> *mut u8 {
    // SAFETY: the size is above zero (every caller's), the layout valid.
    let ptr = unsafe { heap.alloc(layout(size)) };
    assert!(!ptr.is_null());
    ptr
}

/// Frees `ptr`, a block of `size` bytes from `heap`.
fn free(heap: &CountingAlloc<System>, ptr: *mut u8, size: usize) {
    // SAFETY: `ptr` came from `heap` with this layout.
    unsafe { heap.dealloc(ptr, layout(size)) };
}

/// `ptr`, a block of `size` bytes from `heap`, moved to `new_size`.
fn realloc(heap: &CountingAlloc<System>, ptr: *mut u8, size: usize, new_size: usize) -> *mut u8 {
    // SAFETY: `ptr` came from `heap` with this layout, and `new_size` is
    // above zero and fits an `isize` once aligned.
    let moved = unsafe { heap.realloc(ptr, layout(size), new_size) };
    assert!(!moved.is_null());
    moved
}

#[test]
fn a_new_counter_has_counted_nothing() {
    let heap = CountingAlloc::new(System);
    assert_eq!(heap.counts(), Counts::new(0, 0, 0));
}

#[test]
fn a_block_is_live_while_it_lives_and_its_peak_stays_until_reset() {
    let heap = CountingAlloc::new(System);
    let block = alloc(&heap, MB);
    assert_eq!((heap.live_bytes(), heap.peak_bytes()), (MB, MB));
    free(&heap, block, MB);
    assert_eq!(heap.live_bytes(), 0);
    assert_eq!(heap.peak_bytes(), MB);
    heap.reset_peak();
    assert_eq!(heap.peak_bytes(), 0);
}

#[test]
fn the_calls_are_taken_once() {
    let heap = CountingAlloc::new(System);
    let block = alloc(&heap, MB);
    assert!(heap.take_allocs() >= 1);
    assert_eq!(heap.take_allocs(), 0);
    free(&heap, block, MB);
    // Freeing is no call counted.
    assert_eq!(heap.take_allocs(), 0);
}

#[test]
fn zeroed_blocks_count_as_blocks() {
    let heap = CountingAlloc::new(System);
    // SAFETY: a valid layout above zero.
    let block = unsafe { heap.alloc_zeroed(layout(64)) };
    assert!(!block.is_null());
    assert_eq!(heap.counts(), Counts::new(64, 64, 1));
    free(&heap, block, 64);
}

#[test]
fn a_realloc_grows_and_shrinks_the_live_bytes() {
    let heap = CountingAlloc::new(System);
    let block = alloc(&heap, 100);
    let block = realloc(&heap, block, 100, 300);
    assert_eq!((heap.live_bytes(), heap.peak_bytes()), (300, 300));
    let block = realloc(&heap, block, 300, 50);
    assert_eq!((heap.live_bytes(), heap.peak_bytes()), (50, 300));
    assert_eq!(heap.take_allocs(), 3);
    free(&heap, block, 50);
    assert_eq!(heap.live_bytes(), 0);
}

#[test]
fn reset_peak_starts_from_whats_live() {
    let heap = CountingAlloc::new(System);
    let big = alloc(&heap, 1000);
    let small = alloc(&heap, 10);
    free(&heap, big, 1000);
    heap.reset_peak();
    assert_eq!(heap.peak_bytes(), 10);
    free(&heap, small, 10);
}

#[test]
fn counts_reads_all_three_and_takes_the_calls() {
    let heap = CountingAlloc::new(System);
    let a = alloc(&heap, 40);
    let b = alloc(&heap, 60);
    free(&heap, a, 40);
    assert_eq!(heap.counts(), Counts::new(60, 100, 2));
    assert_eq!(heap.counts(), Counts::new(60, 100, 0));
    free(&heap, b, 60);
}

/// The same changes on `$word`, and what each read or gave.
macro_rules! changes {
    ($word:expr) => {{
        let word = $word;
        let mut seen = vec![word.add(5), word.add(7)];
        word.sub(2);
        seen.push(word.get());
        word.max(4);
        seen.push(word.get());
        word.max(30);
        seen.push(word.get());
        seen.extend([word.take(), word.take()]);
        word.set(9);
        seen.push(word.get());
        seen
    }};
}

/// The plain cells a target without atomics counts in count as the atomic
/// words do.
#[test]
fn a_single_threaded_word_counts_as_an_atomic_one() {
    let counted = [5, 12, 10, 10, 30, 30, 0, 9];
    assert_eq!(changes!(Word::new()), counted);
    assert_eq!(changes!(single::Word::new()), counted);
}

/// One step of a random run on a heap.
#[derive(Debug, Clone)]
enum Step {
    Alloc(usize),
    /// Moves the block at this index (modulo the blocks) to a new size.
    Realloc(usize, usize),
    /// Frees the block at this index (modulo the blocks).
    Free(usize),
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        (1usize..4096).prop_map(Step::Alloc),
        (any::<usize>(), 1usize..4096).prop_map(|(i, n)| Step::Realloc(i, n)),
        any::<usize>().prop_map(Step::Free),
    ]
}

proptest! {
    #[test]
    fn live_bytes_are_the_models_sum_and_the_peak_its_most(
        steps in proptest::collection::vec(step(), 0..64),
    ) {
        let heap = CountingAlloc::new(System);
        let mut blocks: Vec<(*mut u8, usize)> = Vec::new();
        let (mut sum, mut most) = (0usize, 0usize);
        for step in steps {
            match step {
                Step::Alloc(size) => blocks.push((alloc(&heap, size), size)),
                Step::Realloc(i, new_size) if !blocks.is_empty() => {
                    let i = i % blocks.len();
                    let (ptr, size) = blocks[i];
                    blocks[i] = (realloc(&heap, ptr, size, new_size), new_size);
                }
                Step::Free(i) if !blocks.is_empty() => {
                    let (ptr, size) = blocks.swap_remove(i % blocks.len());
                    free(&heap, ptr, size);
                }
                Step::Realloc(..) | Step::Free(_) => {}
            }
            sum = blocks.iter().map(|&(_, size)| size).sum();
            most = most.max(sum);
            prop_assert_eq!(heap.live_bytes(), sum);
            prop_assert_eq!(heap.peak_bytes(), most);
        }
        for (ptr, size) in blocks {
            free(&heap, ptr, size);
        }
        prop_assert_eq!(heap.live_bytes(), 0);
        prop_assert_eq!(heap.peak_bytes(), most.max(sum));
    }
}

mod tagged {
    //! A [`CountingAlloc::with_tags`] called directly. The tags entered
    //! are numbers, never asked of the process's table: only this
    //! instance counts them, so the counts are exact.

    use super::super::tagged::header_bytes;
    use super::super::tags::{self, OVERFLOW};
    use super::*;
    use crate::TagCounts;

    type Heap = CountingAlloc<System, super::super::Tagged>;

    /// A block of `size` bytes aligned to `align`, allocated under `tag`.
    fn alloc_in(heap: &Heap, tag: u16, size: usize, align: usize, zeroed: bool) -> *mut u8 {
        let layout = Layout::from_size_align(size, align).unwrap();
        let before = tags::enter(tag);
        // SAFETY: a valid layout above zero.
        let block = unsafe {
            if zeroed {
                heap.alloc_zeroed(layout)
            } else {
                heap.alloc(layout)
            }
        };
        tags::leave(before);
        // `System` refuses a small block only out of memory. An `if`, not an
        // `assert!`: CodeQL, which reads the code unbuilt, then sees that
        // the block given is never null.
        if block.is_null() {
            unreachable!("the heap gave no block");
        }
        block
    }

    /// `block` (`size` bytes aligned to `align`) moved to `new_size` under
    /// `tag`.
    fn realloc_in(
        heap: &Heap,
        tag: u16,
        block: *mut u8,
        (size, align): (usize, usize),
        new_size: usize,
    ) -> *mut u8 {
        let layout = Layout::from_size_align(size, align).unwrap();
        let before = tags::enter(tag);
        // SAFETY: `block` came from `heap` with this layout; `new_size` is
        // above zero and small.
        let moved = unsafe { heap.realloc(block, layout, new_size) };
        tags::leave(before);
        // An `if`, as `alloc_in`'s.
        if moved.is_null() {
            unreachable!("the heap moved no block");
        }
        moved
    }

    fn free_in(heap: &Heap, block: *mut u8, size: usize, align: usize) {
        let layout = Layout::from_size_align(size, align).unwrap();
        // SAFETY: `block` came from `heap` with this layout.
        unsafe { heap.dealloc(block, layout) };
    }

    /// `heap`'s row of `tag` (of `overflow` past it).
    fn row(heap: &Heap, tag: u16) -> Option<TagCounts> {
        let slot = tags::slot(tag);
        heap.rows().into_iter().find(|r| usize::from(r.tag) == slot)
    }

    /// `heap`'s live bytes and blocks under `tag`.
    fn live(heap: &Heap, tag: u16) -> (u64, u32) {
        row(heap, tag).map_or((0, 0), |r| (r.live_bytes, r.live_blocks))
    }

    #[test]
    fn the_header_keeps_each_alignment_and_holds_the_tag_just_before_the_block() {
        let heap = Heap::with_tags(System);
        for shift in 0..=12 {
            let align = 1usize << shift;
            assert_eq!(header_bytes(align), align.max(8));
            let block = alloc_in(&heap, 7, 3, align, false);
            assert_eq!(block.addr() % align, 0, "align {align}");
            // SAFETY: `block` is a block of `heap`'s, not null, and the two
            // bytes before it are its header's last.
            let tag = unsafe { block.sub(2).cast::<u16>().read_unaligned() };
            assert_eq!(tag, 7);
            free_in(&heap, block, 3, align);
        }
        assert_eq!(heap.live_bytes(), 0, "the headers are never counted");
        assert_eq!(heap.peak_bytes(), 3);
    }

    #[test]
    fn a_block_counts_under_the_tag_current_when_it_was_made() {
        let heap = Heap::with_tags(System);
        let a = alloc_in(&heap, 3, 100, 8, false);
        let b = alloc_in(&heap, 3, 50, 16, true);
        let c = alloc_in(&heap, 4, 10, 1, false);
        assert_eq!(live(&heap, 3), (150, 2));
        assert_eq!(live(&heap, 4), (10, 1));
        assert_eq!(heap.counts(), Counts::new(160, 160, 3));
        free_in(&heap, a, 100, 8);
        assert_eq!(live(&heap, 3), (50, 1));
        let three = row(&heap, 3).unwrap();
        assert_eq!((three.allocated_bytes, three.peak_bytes), (150, 150));
        free_in(&heap, b, 50, 16);
        free_in(&heap, c, 10, 1);
        assert_eq!(heap.live_bytes(), 0);
    }

    #[test]
    fn a_realloc_moves_the_bytes_to_the_current_tag() {
        let heap = Heap::with_tags(System);
        let block = alloc_in(&heap, 1, 100, 32, false);
        // SAFETY: the block has 100 bytes.
        unsafe { block.write_bytes(0xA5, 100) };
        let block = realloc_in(&heap, 2, block, (100, 32), 400);
        assert_eq!(live(&heap, 1), (0, 0));
        assert_eq!(live(&heap, 2), (400, 1));
        // SAFETY: the block has 400 bytes, the first 100 kept.
        let kept = unsafe { core::slice::from_raw_parts(block, 100) };
        assert!(kept.iter().all(|&b| b == 0xA5));
        let one = row(&heap, 1).unwrap();
        assert_eq!((one.allocated_bytes, one.peak_bytes), (100, 100));
        let block = realloc_in(&heap, 2, block, (400, 32), 40);
        assert_eq!(live(&heap, 2), (40, 1));
        assert_eq!(row(&heap, 2).unwrap().peak_bytes, 400);
        assert_eq!((heap.live_bytes(), heap.peak_bytes()), (40, 400));
        assert_eq!(heap.take_allocs(), 3);
        free_in(&heap, block, 40, 32);
        assert_eq!(live(&heap, 2), (0, 0));
    }

    /// [`System`], unless it refuses: then each call gives nothing.
    #[derive(Default)]
    struct Refusing {
        refuse: core::cell::Cell<bool>,
    }

    // SAFETY: `System`'s blocks, or none.
    unsafe impl GlobalAlloc for Refusing {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            if self.refuse.get() {
                return core::ptr::null_mut();
            }
            // SAFETY: the caller's layout, unchanged.
            unsafe { System.alloc(layout) }
        }

        unsafe fn dealloc(&self, block: *mut u8, layout: Layout) {
            // SAFETY: `block` is `System`'s, with this layout.
            unsafe { System.dealloc(block, layout) };
        }

        unsafe fn realloc(&self, block: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            if self.refuse.get() {
                return core::ptr::null_mut();
            }
            // SAFETY: the caller's block, layout and size, unchanged.
            unsafe { System.realloc(block, layout, new_size) }
        }
    }

    #[test]
    fn a_call_the_inner_allocator_refuses_counts_only_as_a_call() {
        let heap = CountingAlloc::with_tags(Refusing::default());
        let layout = Layout::from_size_align(100, 8).unwrap();
        heap.inner.refuse.set(true);
        // SAFETY: a valid layout above zero.
        assert!(unsafe { heap.alloc(layout) }.is_null());
        // SAFETY: as above.
        assert!(unsafe { heap.alloc_zeroed(layout) }.is_null());
        assert_eq!(heap.counts(), Counts::new(0, 0, 2));
        assert_eq!(heap.rows(), []);
        heap.inner.refuse.set(false);
        let before = tags::enter(5);
        // SAFETY: as above.
        let block = unsafe { heap.alloc(layout) };
        tags::enter(6);
        heap.inner.refuse.set(true);
        // SAFETY: `block` is `heap`'s, with `layout`.
        let moved = unsafe { heap.realloc(block, layout, 300) };
        tags::leave(before);
        assert!(moved.is_null());
        assert_eq!(heap.take_allocs(), 2);
        let tags: Vec<_> = heap.rows().iter().map(|r| (r.tag, r.live_bytes)).collect();
        assert_eq!(tags, [(5, 100)], "the block is as it was, under its tag");
        heap.inner.refuse.set(false);
        // SAFETY: `block` is still `heap`'s, with `layout`.
        unsafe { heap.dealloc(block, layout) };
        assert_eq!(heap.live_bytes(), 0);
    }

    #[test]
    fn a_size_too_big_with_its_header_is_refused() {
        let heap = Heap::with_tags(System);
        let most = usize::try_from(isize::MAX).unwrap() - 7;
        let huge = Layout::from_size_align(most, 8).unwrap();
        // SAFETY: a valid layout above zero.
        assert!(unsafe { heap.alloc(huge) }.is_null());
        let block = alloc_in(&heap, 1, 16, 8, false);
        let small = Layout::from_size_align(16, 8).unwrap();
        for new_size in [most, usize::MAX - 2] {
            // SAFETY: `block` is `heap`'s, with `small`. A size too big is
            // refused before the inner allocator sees it.
            assert!(unsafe { heap.realloc(block, small, new_size) }.is_null());
        }
        assert_eq!(heap.counts(), Counts::new(16, 16, 4));
        assert_eq!(live(&heap, 1), (16, 1));
        free_in(&heap, block, 16, 8);
    }

    #[test]
    fn a_tagged_counter_shows_its_tags_as_a_word() {
        let heap = Heap::with_tags(System);
        let shown = format!("{heap:?}");
        assert!(shown.contains("tags: Tagged { .. }"), "{shown}");
    }

    #[test]
    fn rows_leave_out_the_tags_never_used() {
        let heap = Heap::with_tags(System);
        assert_eq!(heap.rows(), []);
        let block = alloc_in(&heap, 9, 8, 8, false);
        free_in(&heap, block, 8, 8);
        let tags: Vec<u16> = heap.rows().iter().map(|r| r.tag).collect();
        assert_eq!(tags, [9], "a tag used once stays, with nothing live");
    }

    #[test]
    fn a_tag_past_overflow_counts_under_overflow() {
        let heap = Heap::with_tags(System);
        let block = alloc_in(&heap, 4000, 8, 8, false);
        let overflow = row(&heap, OVERFLOW).unwrap();
        assert_eq!(overflow.name, "overflow");
        assert_eq!(overflow.class, tags::OTHER_CLASS);
        assert_eq!(overflow.live_bytes, 8);
        free_in(&heap, block, 8, 8);
    }

    #[test]
    fn other_is_tag_zero() {
        let heap = Heap::with_tags(System);
        let block = alloc_in(&heap, tags::OTHER, 8, 8, false);
        let other = row(&heap, tags::OTHER).unwrap();
        assert_eq!((other.class, other.name), (tags::OTHER_CLASS, "other"));
        free_in(&heap, block, 8, 8);
    }

    /// One step of a random run on a tagged heap, under a current tag.
    #[derive(Debug, Clone)]
    enum Step {
        /// A block of this size, at 2 to this power alignment, zeroed or
        /// not.
        Alloc(usize, u32, bool),
        /// Moves the block at this index (modulo the blocks) to a size.
        Realloc(usize, usize),
        /// Frees the block at this index (modulo the blocks).
        Free(usize),
    }

    /// The tags a run enters: `other`, three more, and one past
    /// `overflow`.
    const RUN_TAGS: [u16; 5] = [0, 1, 2, 3, 300];

    fn tagged_step() -> impl Strategy<Value = (u16, Step)> {
        let step = prop_oneof![
            (1usize..=4096, 0u32..=6, any::<bool>()).prop_map(|(n, a, z)| Step::Alloc(n, a, z)),
            (any::<usize>(), 1usize..=4096).prop_map(|(i, n)| Step::Realloc(i, n)),
            any::<usize>().prop_map(Step::Free),
        ];
        (prop::sample::select(&RUN_TAGS[..]), step)
    }

    /// A block of the run: where it is, its size and alignment, and the
    /// byte it is filled with.
    #[derive(Debug, Clone, Copy)]
    struct Block {
        at: *mut u8,
        size: usize,
        align: usize,
        fill: u8,
    }

    impl Block {
        fn fill(&self) {
            // SAFETY: the block has `size` bytes.
            unsafe { self.at.write_bytes(self.fill, self.size) };
        }

        /// Whether its first `n` bytes are its fill.
        fn holds(&self, n: usize) -> bool {
            // SAFETY: the block has at least `n` bytes (`n` ≤ `size`).
            let bytes = unsafe { core::slice::from_raw_parts(self.at, n) };
            bytes.iter().all(|&b| b == self.fill)
        }
    }

    /// Each of [`RUN_TAGS`]' live bytes and blocks in the model `blocks`.
    fn model(blocks: &[(usize, Block)], tag: u16) -> (u64, u32) {
        let slot = tags::slot(tag);
        let mine: Vec<&Block> = blocks
            .iter()
            .filter(|(s, _)| *s == slot)
            .map(|(_, b)| b)
            .collect();
        let bytes: usize = mine.iter().map(|b| b.size).sum();
        (
            u64::try_from(bytes).unwrap(),
            u32::try_from(mine.len()).unwrap(),
        )
    }

    proptest! {
        #[test]
        fn each_tags_live_bytes_and_blocks_are_the_models(
            steps in proptest::collection::vec(tagged_step(), 0..64),
        ) {
            let heap = Heap::with_tags(System);
            // The model: each block with the tag slot it counts under.
            let mut blocks: Vec<(usize, Block)> = Vec::new();
            for (n, (tag, step)) in steps.into_iter().enumerate() {
                let fill = u8::try_from(n % 251).unwrap();
                let slot = tags::slot(tag);
                match step {
                    Step::Alloc(size, shift, zeroed) => {
                        let align = 1 << shift;
                        let at = alloc_in(&heap, tag, size, align, zeroed);
                        let block = Block { at, size, align, fill: 0 };
                        if zeroed {
                            prop_assert!(block.holds(size));
                        }
                        let block = Block { fill, ..block };
                        block.fill();
                        blocks.push((slot, block));
                    }
                    Step::Realloc(i, new_size) if !blocks.is_empty() => {
                        let i = i % blocks.len();
                        let old = blocks[i].1;
                        let at = realloc_in(&heap, tag, old.at, (old.size, old.align), new_size);
                        let moved = Block { at, size: new_size, ..old };
                        prop_assert!(moved.holds(old.size.min(new_size)));
                        let moved = Block { fill, ..moved };
                        moved.fill();
                        blocks[i] = (slot, moved);
                    }
                    Step::Free(i) if !blocks.is_empty() => {
                        let (_, block) = blocks.swap_remove(i % blocks.len());
                        prop_assert!(block.holds(block.size));
                        free_in(&heap, block.at, block.size, block.align);
                    }
                    Step::Realloc(..) | Step::Free(_) => {}
                }
                for &tag in &RUN_TAGS {
                    prop_assert_eq!(live(&heap, tag), model(&blocks, tag), "tag {}", tag);
                }
                let total: usize = blocks.iter().map(|(_, b)| b.size).sum();
                prop_assert_eq!(heap.live_bytes(), total);
            }
            for (_, block) in blocks {
                free_in(&heap, block.at, block.size, block.align);
            }
            prop_assert_eq!(heap.live_bytes(), 0);
        }
    }
}
