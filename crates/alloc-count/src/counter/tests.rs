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
