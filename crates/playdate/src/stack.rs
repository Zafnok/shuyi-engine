//! The arithmetic of the game's own stack (`crate::device`).
//!
//! The device gives the C stack about 10 KB, and a game's load or search can
//! go deeper. So the backend runs the game on a block from the heap, filled
//! with [`PAINT`] first. A stack grows down from its top, so the painted bytes
//! left at the bottom say how deep it ever went ([`high_water`]).

use core::sync::atomic::{AtomicBool, Ordering};

/// The game's stack unless its `main!` says otherwise, in bytes. The first
/// game on the backend measured 15,256 bytes at its deepest, in its load.
pub const DEFAULT_BYTES: usize = 64 * 1024;

/// The byte the stack is filled with before the game first runs on it.
pub const PAINT: u8 = 0xA5;

/// The bottom bytes of the stack that count as an overflow once used: the
/// game has gone, or is about to go, past its stack.
pub const RED_ZONE: usize = 256;

/// The bytes ever used of a stack of `size` bytes, given its `bytes` lowest
/// address first: everything above the painted bytes at its bottom.
pub fn high_water(size: usize, bytes: impl IntoIterator<Item = u8>) -> usize {
    let untouched = bytes.into_iter().take_while(|&byte| byte == PAINT).count();
    size.saturating_sub(untouched)
}

/// Whether a stack, given its `bytes` lowest address first, has been used
/// into its [`RED_ZONE`]. It reads that zone alone, so it can run every frame.
pub fn overflowed(bytes: impl IntoIterator<Item = u8>) -> bool {
    bytes.into_iter().take(RED_ZONE).any(|byte| byte != PAINT)
}

/// The starting stack pointer for a stack of `size` bytes at `base`: its top,
/// rounded down to 8 bytes as the ARM procedure call standard asks.
pub fn top(base: usize, size: usize) -> usize {
    (base + size) & !7
}

/// Runs `f` through `switch`, which moves to the stack, makes the call it is
/// handed and comes back. A call made while `inside` says the game is on the
/// stack already is a plain one, so it doesn't start again from the top.
pub fn run<R>(
    inside: &AtomicBool,
    f: impl FnOnce() -> R,
    switch: impl FnOnce(&mut dyn FnMut()),
) -> R {
    if inside.swap(true, Ordering::Relaxed) {
        return f();
    }
    let mut f = Some(f);
    let mut out = None;
    switch(&mut || out = f.take().map(|f| f()));
    inside.store(false, Ordering::Relaxed);
    match out {
        Some(out) => out,
        // check-text: not player text (a backend's bug, for the console)
        None => panic!("the game's stack came back without making the call"),
    }
}

#[cfg(test)]
mod tests {
    use core::cell::Cell;

    use super::*;

    /// The bytes of `stack`, as the device reads them.
    fn bytes(stack: &[u8]) -> impl Iterator<Item = u8> {
        stack.iter().copied()
    }

    #[test]
    fn an_untouched_stack_has_used_nothing() {
        assert_eq!(high_water(64, bytes(&[PAINT; 64])), 0);
    }

    #[test]
    fn a_use_of_3_kb_reads_3_kb() {
        let mut stack = alloc::vec![PAINT; DEFAULT_BYTES];
        for byte in &mut stack[DEFAULT_BYTES - 3 * 1024..] {
            *byte = 0;
        }
        assert_eq!(high_water(DEFAULT_BYTES, bytes(&stack)), 3 * 1024);
    }

    #[test]
    fn the_used_bytes_are_those_above_the_deepest_write() {
        let mut stack = [PAINT; 64];
        stack[40] = 0;
        stack[63] = 1;
        // Bytes 41 to 62 kept the paint's value, but byte 40 was written.
        assert_eq!(high_water(64, bytes(&stack)), 24);
        stack[0] = 7;
        assert_eq!(high_water(64, bytes(&stack)), 64);
    }

    #[test]
    fn the_red_zone_is_the_bottom_of_the_stack() {
        let mut stack = [PAINT; 1024];
        assert!(!overflowed(bytes(&stack)));
        stack[RED_ZONE] = 0;
        assert!(!overflowed(bytes(&stack)));
        stack[RED_ZONE - 1] = 0;
        assert!(overflowed(bytes(&stack)));
        stack[RED_ZONE - 1] = PAINT;
        stack[0] = 0;
        assert!(overflowed(bytes(&stack)));
    }

    #[test]
    fn the_top_is_eight_byte_aligned_and_inside_the_block() {
        assert_eq!(top(0x2000_0000, DEFAULT_BYTES), 0x2001_0000);
        assert_eq!(top(0x2000_0004, 100), 0x2000_0068);
        assert_eq!(top(0x2000_0003, 16), 0x2000_0010);
    }

    /// A switch that counts how often it was asked to move.
    fn counted(switches: &Cell<u32>) -> impl Fn(&mut dyn FnMut()) {
        |call| {
            switches.set(switches.get() + 1);
            call();
        }
    }

    #[test]
    fn a_call_moves_to_the_stack_and_hands_back_what_it_returned() {
        let inside = AtomicBool::new(false);
        let switches = Cell::new(0);
        let seen = run(
            &inside,
            || inside.load(Ordering::Relaxed),
            counted(&switches),
        );
        assert!(seen, "the guard is up during the call");
        assert_eq!(switches.get(), 1);
        assert!(!inside.load(Ordering::Relaxed), "and down after it");
        run(&inside, || (), counted(&switches));
        assert_eq!(switches.get(), 2, "the next call moves again");
    }

    #[test]
    fn a_nested_call_does_not_start_again_from_the_top() {
        let inside = AtomicBool::new(false);
        let switches = Cell::new(0);
        let (outer, still_inside) = run(
            &inside,
            || {
                let inner = run(&inside, || 7, counted(&switches));
                (inner + 1, inside.load(Ordering::Relaxed))
            },
            counted(&switches),
        );
        assert_eq!(outer, 8);
        assert_eq!(switches.get(), 1);
        assert!(still_inside, "the inner call leaves the guard up");
        assert!(!inside.load(Ordering::Relaxed));
    }
}
