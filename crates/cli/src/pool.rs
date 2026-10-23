//! A pool of threads for the tools that play many slow tries (`playtest`,
//! and later `autobalance`): every item of a work list run on one of `n`
//! threads and timed, the answers given back in the list's order, so what
//! is made of them never depends on the thread count. The clock is the
//! tools'; the core never reads one.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// An item's place in the work list, and its timed answer.
type Done<A, E> = (usize, Result<(A, Duration), E>);

/// The threads to use when not told: one a core, or 1 if that's unknown.
pub fn cores() -> usize {
    thread::available_parallelism().map_or(1, std::num::NonZero::get)
}

/// Runs `run` on every item of `work` on `threads` threads (at least one,
/// and no more than there are items): thread `w` takes items `w`,
/// `w + threads`, `w + 2 × threads`, …, and times each. The answers come
/// back in `work`'s order with their times, or the first error in that
/// order, whatever the thread count: once an item fails, no thread starts
/// a later one, but the earlier ones all run. A panic in `run` is resumed
/// here, on the calling thread.
pub fn run<T, A, E, F>(work: &[T], threads: usize, run: &F) -> Result<Vec<(A, Duration)>, E>
where
    T: Sync,
    A: Send,
    E: Send,
    F: Fn(&T) -> Result<A, E> + Sync,
{
    let threads = threads.clamp(1, work.len().max(1));
    // The earliest item that failed yet.
    let failed = AtomicUsize::new(usize::MAX);
    let failed = &failed;
    let mut done: Vec<Done<A, E>> = thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|worker| {
                scope.spawn(move || {
                    let mut done = Vec::new();
                    for (i, item) in work.iter().enumerate().skip(worker).step_by(threads) {
                        if i > failed.load(Ordering::Relaxed) {
                            break;
                        }
                        let started = Instant::now();
                        let answer = run(item).map(|a| (a, started.elapsed()));
                        if answer.is_err() {
                            failed.fetch_min(i, Ordering::Relaxed);
                        }
                        done.push((i, answer));
                    }
                    done
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| {
                worker
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            })
            .collect()
    });
    done.sort_by_key(|(i, _)| *i);
    done.into_iter().map(|(_, answer)| answer).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Item `i` of `n` sleeps longer the earlier it is, so the threads
    /// finish them out of order.
    fn slow(n: u64) -> impl Fn(&u64) -> Result<u64, String> + Sync {
        move |&i| {
            thread::sleep(Duration::from_millis(n - i));
            Ok(i * 10)
        }
    }

    #[test]
    fn the_answers_come_back_in_the_works_order_whatever_the_thread_count() {
        let work: Vec<u64> = (0..12).collect();
        let want: Vec<u64> = work.iter().map(|i| i * 10).collect();
        for threads in [0, 1, 3, 4, 12, 50] {
            let done = run(&work, threads, &slow(12)).unwrap();
            let answers: Vec<u64> = done.iter().map(|(a, _)| *a).collect();
            assert_eq!(answers, want, "{threads} thread(s)");
            // Each item slept its time, and was timed doing it.
            assert!(done[0].1 >= Duration::from_millis(12), "{:?}", done[0].1);
        }
        let found = run(&[] as &[u64], 4, &slow(0)).unwrap();
        assert!(found.is_empty(), "{found:?}");
        assert_eq!(
            cores(),
            thread::available_parallelism().map_or(1, std::num::NonZero::get)
        );
    }

    #[test]
    fn the_first_error_in_the_works_order_stops_it() {
        let work: Vec<u64> = (0..40).collect();
        let fails = |&i: &u64| {
            if i % 7 == 3 {
                Err(format!("item {i}"))
            } else {
                Ok(i)
            }
        };
        assert_eq!(run(&work, 1, &fails).unwrap_err(), "item 3");
        // On one thread, nothing after the failing item runs.
        let started = AtomicUsize::new(0);
        let counted = |i: &u64| {
            started.fetch_add(1, Ordering::Relaxed);
            fails(i)
        };
        assert!(run(&work, 1, &counted).is_err());
        assert_eq!(started.load(Ordering::Relaxed), 4);
        // On several threads too, though a later item may fail first.
        for threads in [2, 4, 8] {
            assert_eq!(run(&work, threads, &fails).unwrap_err(), "item 3");
        }
    }

    #[test]
    #[should_panic(expected = "a bug in a try")]
    fn a_panic_on_a_thread_is_resumed_here() {
        let work = [1, 2, 3];
        let _ = run(&work, 3, &|&i: &i32| -> Result<i32, String> {
            assert!(i != 2, "a bug in a try");
            Ok(i)
        });
    }
}
