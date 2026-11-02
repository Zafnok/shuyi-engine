//! The output that discards: a thread that calls the callback as often as a device
//! would, for tests and for a machine with no sound.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::{Callback, OutputConfig};

/// See the [module docs](self).
pub(crate) struct NullOutput {
    stop: Arc<AtomicBool>,
    held: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl NullOutput {
    pub(crate) fn open(config: OutputConfig, callback: Callback) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let held = Arc::new(AtomicBool::new(false));
        let (thread_stop, thread_held) = (Arc::clone(&stop), Arc::clone(&held));
        let thread = std::thread::Builder::new()
            .name("sound-null".to_owned())
            .spawn(move || run(config, &callback, &thread_stop, &thread_held))
            .ok();
        Self { stop, held, thread }
    }

    pub(crate) fn hold(&self, on: bool) {
        self.held.store(on, Ordering::Release);
    }
}

impl Drop for NullOutput {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(config: OutputConfig, callback: &Callback, stop: &AtomicBool, held: &AtomicBool) {
    let channels = usize::from(config.channels.count());
    let frames = usize::try_from(config.frames.max(1)).unwrap_or(1);
    let mut block = vec![0; frames * channels];
    let period =
        Duration::from_secs_f64(f64::from(config.frames.max(1)) / f64::from(config.rate.max(1)));
    let mut next = Instant::now() + period;
    while !stop.load(Ordering::Acquire) {
        let now = Instant::now();
        if next > now {
            std::thread::sleep((next - now).min(Duration::from_millis(50)));
            continue;
        }
        next += period;
        // A slow machine doesn't catch up in a rush: real devices don't either.
        if next < now {
            next = now + period;
        }
        if held.load(Ordering::Acquire) {
            continue;
        }
        if let Ok(mut callback) = callback.try_lock() {
            callback(&mut block);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::sync::atomic::AtomicUsize;

    use shuyi_sound::Channels;

    use super::*;

    fn counted() -> (Arc<AtomicUsize>, Callback) {
        let count = Arc::new(AtomicUsize::new(0));
        let inner = Arc::clone(&count);
        let callback: Callback = Arc::new(Mutex::new(move |out: &mut [i16]| {
            inner.fetch_add(out.len() / 2, Ordering::AcqRel);
        }));
        (count, callback)
    }

    fn wait_for(what: &str, done: impl Fn() -> bool) {
        let start = Instant::now();
        while !done() {
            assert!(start.elapsed() < Duration::from_secs(20), "{what}");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn it_asks_for_blocks_of_the_size_it_was_told_until_dropped() {
        let (count, callback) = counted();
        let config = OutputConfig::new(48_000, Channels::Stereo, 240);
        let output = NullOutput::open(config, callback);
        wait_for("it never asked", || count.load(Ordering::Acquire) >= 480);
        assert_eq!(count.load(Ordering::Acquire) % 240, 0);
        drop(output);
        let after = count.load(Ordering::Acquire);
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(count.load(Ordering::Acquire), after, "it stopped");
    }

    #[test]
    fn held_it_asks_for_nothing_and_it_goes_on_after() {
        let (count, callback) = counted();
        let output = NullOutput::open(OutputConfig::new(48_000, Channels::Stereo, 48), callback);
        wait_for("it never asked", || count.load(Ordering::Acquire) > 0);
        output.hold(true);
        // One call may have been on its way.
        std::thread::sleep(Duration::from_millis(20));
        let held = count.load(Ordering::Acquire);
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(count.load(Ordering::Acquire), held);
        output.hold(false);
        wait_for("it never went on", || count.load(Ordering::Acquire) > held);
    }
}
