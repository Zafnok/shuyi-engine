//! Everything of this crate that isn't safe code (ADR-0021 §6): the hand-over that
//! moves a boxed value between threads, which no lock or allocation may touch, and
//! the web's functions, which `sound.js` provides.
#![allow(unsafe_code)]

use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

/// A fixed ring of pointers, for one writer and one reader, which owns what it holds.
struct Ring<T> {
    slots: Box<[AtomicPtr<T>]>,
    /// A power of two less one.
    mask: u32,
    /// How many were ever put in. Only the writer stores it.
    head: AtomicU32,
    /// How many were ever taken out. Only the reader stores it.
    tail: AtomicU32,
}

impl<T> Ring<T> {
    fn slot(&self, at: u32) -> &AtomicPtr<T> {
        &self.slots[usize::try_from(at & self.mask).unwrap_or(0)]
    }
}

impl<T> Drop for Ring<T> {
    fn drop(&mut self) {
        for slot in &mut *self.slots {
            let held = *slot.get_mut();
            if !held.is_null() {
                // SAFETY: a non-null slot holds a pointer `HandTo::send` got from
                // `Box::into_raw` and no reader took; with `&mut self` nobody else
                // can reach it, and this frees it once, as the slot is dropped next.
                drop(unsafe { Box::from_raw(held) });
            }
        }
    }
}

/// Makes the two ends of a hand-over with room for at least `slots` values waiting.
/// Nothing is allocated or freed by [`HandTo::send`] or [`HandOut::take`]: a value's
/// box goes from one thread to the other, and whoever ends up with it frees it, so
/// the end on an audio thread sends each box back rather than dropping it.
pub fn handover<T: Send>(slots: usize) -> (HandTo<T>, HandOut<T>) {
    let slots = u32::try_from(slots)
        .unwrap_or(1 << 31)
        .clamp(1, 1 << 31)
        .next_power_of_two();
    let ring = Arc::new(Ring {
        slots: (0..slots)
            .map(|_| AtomicPtr::new(ptr::null_mut()))
            .collect(),
        mask: slots - 1,
        head: AtomicU32::new(0),
        tail: AtomicU32::new(0),
    });
    (
        HandTo {
            ring: Arc::clone(&ring),
        },
        HandOut { ring },
    )
}

/// The end of a [`handover`] that puts values in.
pub struct HandTo<T> {
    ring: Arc<Ring<T>>,
}

impl<T> HandTo<T> {
    /// Puts `value` in, or gives it back if there is no room.
    pub fn send(&mut self, value: Box<T>) -> Result<(), Box<T>> {
        let ring = &*self.ring;
        let head = ring.head.load(Ordering::Relaxed);
        let tail = ring.tail.load(Ordering::Acquire);
        if head.wrapping_sub(tail) > ring.mask {
            return Err(value);
        }
        ring.slot(head)
            .store(Box::into_raw(value), Ordering::Release);
        ring.head.store(head.wrapping_add(1), Ordering::Release);
        Ok(())
    }
}

/// The end of a [`handover`] that takes values out.
pub struct HandOut<T> {
    ring: Arc<Ring<T>>,
}

impl<T> HandOut<T> {
    /// The next value, if one has come.
    pub fn take(&mut self) -> Option<Box<T>> {
        let ring = &*self.ring;
        let tail = ring.tail.load(Ordering::Relaxed);
        if ring.head.load(Ordering::Acquire) == tail {
            return None;
        }
        let held = ring.slot(tail).swap(ptr::null_mut(), Ordering::AcqRel);
        ring.tail.store(tail.wrapping_add(1), Ordering::Release);
        if held.is_null() {
            return None;
        }
        // SAFETY: `head` said this slot was written, with `Release`, by the one writer,
        // from `Box::into_raw`; the swap took the pointer out, so this is its only
        // owner, and the box's value was `Send` to come here.
        Some(unsafe { Box::from_raw(held) })
    }
}

// SAFETY: an end only ever moves whole `T`s, each owned by one side at a time, through
// atomics; the `Arc`'s contents are atomics and pointers it owns. So with `T: Send`
// an end can go to another thread. Neither end hands out a shared `&T`.
unsafe impl<T: Send> Send for HandTo<T> {}
// SAFETY: as for `HandTo`.
unsafe impl<T: Send> Send for HandOut<T> {}

/// The functions of `sound.js`, and what it calls here. Plain numbers, and pointers
/// into this module's own buffers that it reads before returning.
#[cfg(target_arch = "wasm32")]
pub(crate) mod web {
    /// The plugin's version, which the loader compares with the `version` in
    /// `crates/cli/web/sound.js` (a `shuyi-cli` test keeps them equal). Bump both when
    /// the functions below change.
    const PLUGIN_VERSION: u32 = 1;

    unsafe extern "C" {
        /// Opens the output at about `rate`, asking for `frames` at a time, and says
        /// the rate it runs at, or 0 if this page can't: `discard` never makes a sound.
        pub(crate) safe fn shuyi_sound_open(rate: u32, frames: u32, discard: u32) -> u32;
        /// Holds the output, or lets it go on if `on` is 0.
        pub(crate) safe fn shuyi_sound_hold(on: u32);
        /// Starts the browser's own decoder on `bytes` bytes at `pointer`, to learn
        /// whether it can read them: the number to ask about.
        safe fn shuyi_sound_vet(pointer: *const u8, bytes: u32) -> u32;
        /// 0 while the browser decodes, 1 if it read the file, 2 if it refused it.
        pub(crate) safe fn shuyi_sound_vetted(number: u32) -> u32;
    }

    /// Asks the browser about `bytes`. It copies them before this returns.
    pub(crate) fn vet(bytes: &[u8]) -> u32 {
        shuyi_sound_vet(bytes.as_ptr(), u32::try_from(bytes.len()).unwrap_or(0))
    }

    /// What the loader calls to check the plugin's version.
    #[unsafe(no_mangle)]
    pub extern "C" fn shuyi_sound_crate_version() -> u32 {
        PLUGIN_VERSION
    }

    /// What `sound.js` calls for `frames` frames: where they are, until the next call.
    #[unsafe(no_mangle)]
    pub extern "C" fn shuyi_sound_fill(frames: u32) -> *const i16 {
        crate::web::fill(frames)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use super::*;

    /// Says when it is freed.
    struct Tracked(Arc<AtomicUsize>);

    impl Drop for Tracked {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::AcqRel);
        }
    }

    #[test]
    fn values_come_out_in_order_and_a_full_ring_gives_one_back() {
        let (mut to, mut out) = handover::<u32>(3);
        for value in 0..4 {
            assert!(to.send(Box::new(value)).is_ok());
        }
        assert_eq!(to.send(Box::new(9)).map_err(|back| *back), Err(9));
        for value in 0..4 {
            assert_eq!(out.take().map(|got| *got), Some(value));
        }
        assert!(out.take().is_none());
        assert!(to.send(Box::new(5)).is_ok());
        assert_eq!(out.take().map(|got| *got), Some(5));
    }

    #[test]
    fn counters_wrap_past_their_width() {
        let (mut to, mut out) = handover::<u32>(4);
        to.ring.head.store(u32::MAX - 1, Ordering::Relaxed);
        to.ring.tail.store(u32::MAX - 1, Ordering::Relaxed);
        for value in 0..32 {
            assert!(to.send(Box::new(value)).is_ok());
            assert!(to.send(Box::new(value + 100)).is_ok());
            assert_eq!(out.take().map(|got| *got), Some(value));
            assert_eq!(out.take().map(|got| *got), Some(value + 100));
            assert!(out.take().is_none());
        }
    }

    #[test]
    fn what_was_never_taken_is_freed_with_the_ring_and_only_once() {
        let freed = Arc::new(AtomicUsize::new(0));
        let (mut to, mut out) = handover(4);
        for _ in 0..3 {
            assert!(to.send(Box::new(Tracked(Arc::clone(&freed)))).is_ok());
        }
        drop(out.take());
        assert_eq!(freed.load(Ordering::Acquire), 1);
        drop(to);
        assert_eq!(
            freed.load(Ordering::Acquire),
            1,
            "the other end still holds"
        );
        drop(out);
        assert_eq!(freed.load(Ordering::Acquire), 3);
    }

    #[test]
    fn nothing_is_lost_or_doubled_between_real_threads() {
        const COUNT: u32 = 200_000;
        let (mut to, mut out) = handover::<u32>(8);
        let writer = std::thread::spawn(move || {
            for value in 0..COUNT {
                let mut held = Box::new(value);
                loop {
                    match to.send(held) {
                        Ok(()) => break,
                        Err(back) => {
                            held = back;
                            std::thread::yield_now();
                        }
                    }
                }
            }
        });
        let mut next = 0;
        while next < COUNT {
            match out.take() {
                Some(value) => {
                    assert_eq!(*value, next);
                    next += 1;
                }
                None => std::thread::yield_now(),
            }
        }
        assert!(writer.join().is_ok());
        assert!(out.take().is_none());
    }
}
