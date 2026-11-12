//! Sound output: hands a mix to the system and nothing else. [`Output`] calls a
//! callback for samples, from the system's audio thread where there is one, and the
//! callback knows nothing of where they go.
//!
//! - On Windows it is CPAL (`windows`): it takes the device's format, converts at
//!   the boundary ([`convert`]) and comes back on the new default device when one
//!   goes away.
//! - In the browser it is a worklet of our own (`web`, and `sound.js` beside the
//!   page), which the page's one thread feeds with blocks.
//! - [`Output::null`] calls the callback on a timer and discards what it gives: for
//!   tests, and a machine with no sound, where a caller picks it over a crash.
//!
//! [`handover`] moves what can't cross as plain data to a thread that may neither
//! allocate nor free. Everything that isn't safe code is in `sys`.

mod convert_impl;
#[cfg(not(target_arch = "wasm32"))]
mod null;
mod sys;
#[cfg(target_arch = "wasm32")]
pub mod web;
#[cfg(windows)]
mod windows;

use std::sync::{Arc, Mutex};

use shuyi_sound::Channels;

/// How samples become what a device takes.
pub mod convert {
    pub use crate::convert_impl::{Boundary, DeviceSample};
}

pub use sys::{HandOut, HandTo, handover};

/// The callback, shared with whichever stream is running now. Only one stream runs
/// at a time, so its `try_lock` always gets it: it never waits.
type Callback = Arc<Mutex<dyn FnMut(&mut [i16]) + Send>>;

/// What an output is asked for: the format of what its callback writes. The device's
/// own may differ, and [`convert`] makes up the difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct OutputConfig {
    /// Frames a second.
    pub rate: u32,
    /// How many channels a frame has.
    pub channels: Channels,
    /// How many frames the callback is asked for at a time, at most.
    pub frames: u32,
}

impl OutputConfig {
    /// These.
    pub const fn new(rate: u32, channels: Channels, frames: u32) -> Self {
        Self {
            rate,
            channels,
            frames,
        }
    }
}

/// Why an output didn't open.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum OutputError {
    /// There is nothing to play on.
    #[error("no sound device")]
    NoDevice,
    /// This target has no way out yet, or this page can't use one.
    #[error("no sound output here: {0}")]
    Unavailable(String),
    /// The device or the system refused.
    #[error("the sound device refused: {0}")]
    Device(String),
}

/// An open output. The callback runs until it is dropped.
pub struct Output {
    inner: Inner,
    rate: u32,
}

enum Inner {
    #[cfg(not(target_arch = "wasm32"))]
    Null(null::NullOutput),
    #[cfg(windows)]
    Windows(windows::WindowsOutput),
    #[cfg(target_arch = "wasm32")]
    Web(web::WebOutput),
}

impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Output")
            .field("rate", &self.rate)
            .finish_non_exhaustive()
    }
}

impl Output {
    /// The rate the device runs at, if there is one to ask: a mix at that rate needs
    /// no conversion on its way out.
    pub fn preferred_rate() -> Option<u32> {
        #[cfg(windows)]
        return windows::preferred_rate();
        #[cfg(not(windows))]
        None
    }

    /// Opens the system's output, or says why it can't.
    pub fn open(
        config: OutputConfig,
        callback: impl FnMut(&mut [i16]) + Send + 'static,
    ) -> Result<Self, OutputError> {
        Self::open_shared(config, Arc::new(Mutex::new(callback)))
    }

    fn open_shared(config: OutputConfig, callback: Callback) -> Result<Self, OutputError> {
        #[cfg(windows)]
        return windows::WindowsOutput::open(config, callback).map(|output| Self {
            inner: Inner::Windows(output),
            rate: config.rate,
        });
        #[cfg(target_arch = "wasm32")]
        return web::WebOutput::open(config, callback, false).map(|output| Self {
            rate: output.rate(),
            inner: Inner::Web(output),
        });
        #[cfg(not(any(windows, target_arch = "wasm32")))]
        {
            let _ = (config, callback);
            Err(OutputError::Unavailable(
                "this system's output isn't written yet".to_owned(),
            ))
        }
    }

    /// An output that calls `callback` on a timer and discards what it gives.
    pub fn null(config: OutputConfig, callback: impl FnMut(&mut [i16]) + Send + 'static) -> Self {
        Self::null_shared(config, Arc::new(Mutex::new(callback)))
    }

    fn null_shared(config: OutputConfig, callback: Callback) -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            // Only the page's own timer can call it: the worklet's never runs.
            let output = web::WebOutput::open(config, callback, true);
            match output {
                Ok(output) => Self {
                    rate: output.rate(),
                    inner: Inner::Web(output),
                },
                Err(_) => Self {
                    rate: config.rate,
                    inner: Inner::Web(web::WebOutput::idle(config.rate)),
                },
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        Self {
            rate: config.rate,
            inner: Inner::Null(null::NullOutput::open(config, callback)),
        }
    }

    /// The system's output, or, if it won't open, [`null`](Self::null) and why: no
    /// sound rather than a crash, and the caller's to say once.
    pub fn open_or_null(
        config: OutputConfig,
        callback: impl FnMut(&mut [i16]) + Send + 'static,
    ) -> (Self, Option<OutputError>) {
        let callback: Callback = Arc::new(Mutex::new(callback));
        match Self::open_shared(config, Arc::clone(&callback)) {
            Ok(output) => (output, None),
            Err(error) => (Self::null_shared(config, callback), Some(error)),
        }
    }

    /// The rate its callback's samples play at.
    pub const fn rate(&self) -> u32 {
        self.rate
    }

    /// How many times the system's stream was built again, as when its device went
    /// away, and how many of its callbacks were silence instead of the mix. Nothing
    /// asks this but a probe, to say what a recording went through: 0 and 0 where
    /// there is no stream to build.
    #[cfg_attr(not(windows), allow(clippy::unused_self))]
    pub fn stats(&self) -> (u32, u32) {
        #[cfg(windows)]
        if let Inner::Windows(output) = &self.inner {
            return output.stats();
        }
        (0, 0)
    }

    /// How many times the system's stream found the device's buffer empty: its
    /// callback came late, and the device played silence until it did, a gap a
    /// player hears. Nothing asks this but a probe, to tell a gap in a recording
    /// that is the output's from one that isn't: 0 where there is no stream.
    #[cfg_attr(not(windows), allow(clippy::unused_self))]
    pub fn dry_callbacks(&self) -> u32 {
        #[cfg(windows)]
        if let Inner::Windows(output) = &self.inner {
            return output.dry_callbacks();
        }
        0
    }

    /// Whether the system's audio thread runs above the game's, as a buffer of a
    /// few tens of milliseconds needs on a busy machine: false where there is no
    /// such thread, or the system refused.
    #[cfg_attr(not(windows), allow(clippy::unused_self))]
    pub fn raised_priority(&self) -> bool {
        #[cfg(windows)]
        if let Inner::Windows(output) = &self.inner {
            return output.raised_priority();
        }
        false
    }

    /// Stops calling the callback, for as long as the system suspends the game.
    pub fn suspend(&self) {
        self.hold(true);
    }

    /// Goes on calling it after [`suspend`](Self::suspend).
    pub fn resume(&self) {
        self.hold(false);
    }

    fn hold(&self, on: bool) {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            Inner::Null(output) => output.hold(on),
            #[cfg(windows)]
            Inner::Windows(output) => output.hold(on),
            #[cfg(target_arch = "wasm32")]
            Inner::Web(output) => output.hold(on),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    use super::*;

    const CONFIG: OutputConfig = OutputConfig::new(48_000, Channels::Stereo, 48);

    fn counting() -> (Arc<AtomicUsize>, impl FnMut(&mut [i16]) + Send + 'static) {
        let count = Arc::new(AtomicUsize::new(0));
        let inner = Arc::clone(&count);
        (count, move |out: &mut [i16]| {
            inner.fetch_add(out.len(), Ordering::AcqRel);
        })
    }

    fn wait_for(count: &AtomicUsize, more_than: usize) {
        let start = Instant::now();
        while count.load(Ordering::Acquire) <= more_than {
            assert!(start.elapsed() < Duration::from_secs(20), "it never asked");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn the_null_output_runs_at_the_rate_it_was_asked_for_and_can_be_held() {
        let (count, callback) = counting();
        let output = Output::null(CONFIG, callback);
        assert_eq!(output.rate(), 48_000);
        assert_eq!(output.stats(), (0, 0), "it has no stream to build or lose");
        assert_eq!(output.dry_callbacks(), 0, "nor a buffer to run dry");
        assert!(!output.raised_priority(), "nor a system thread");
        assert!(format!("{output:?}").contains("48000"));
        wait_for(&count, 0);
        assert_eq!(
            count.load(Ordering::Acquire) % 96,
            0,
            "whole blocks of stereo"
        );
        output.suspend();
        std::thread::sleep(Duration::from_millis(20));
        let held = count.load(Ordering::Acquire);
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(count.load(Ordering::Acquire), held, "it stopped asking");
        output.resume();
        wait_for(&count, held);
    }

    /// Linux and macOS come with their own ticket: until then they say so, and the
    /// caller gets an output that discards rather than a crash.
    #[cfg(not(windows))]
    #[test]
    fn a_system_without_its_output_is_quiet_and_says_why() {
        assert_eq!(Output::preferred_rate(), None);
        let (_, callback) = counting();
        assert!(matches!(
            Output::open(CONFIG, callback),
            Err(OutputError::Unavailable(_))
        ));
        let (count, callback) = counting();
        let (output, problem) = Output::open_or_null(CONFIG, callback);
        assert!(matches!(problem, Some(OutputError::Unavailable(_))));
        assert_eq!(output.rate(), 48_000);
        wait_for(&count, 0);
    }

    #[test]
    fn errors_say_what_went_wrong() {
        assert_eq!(OutputError::NoDevice.to_string(), "no sound device");
        let refused = OutputError::Device("busy".to_owned());
        assert_eq!(refused.to_string(), "the sound device refused: busy");
    }

    /// The one lint that differs from the workspace's is `unsafe_code`, as ADR-0021
    /// §6 says, and the one added is what makes each block say why it holds.
    #[test]
    fn the_lints_are_the_workspace_lints_except_unsafe_code() {
        let manifest = include_str!("../Cargo.toml");
        let root = include_str!("../../../Cargo.toml");
        let section = |text: &'static str, name: &str| -> Vec<&'static str> {
            text.lines()
                .skip_while(|line| *line != name)
                .skip(1)
                .take_while(|line| !line.starts_with('['))
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .collect()
        };
        let rust = section(manifest, "[lints.rust]");
        assert!(rust.contains(&"unsafe_code = \"deny\""), "{rust:?}");
        let workspace_rust: Vec<String> = section(root, "[workspace.lints.rust]")
            .iter()
            .map(|line| line.replace("\"forbid\"", "\"deny\""))
            .collect();
        assert_eq!(rust, workspace_rust);
        let mut clippy = section(manifest, "[lints.clippy]");
        clippy.retain(|line| !line.starts_with("undocumented_unsafe_blocks"));
        assert_eq!(clippy, section(root, "[workspace.lints.clippy]"));
    }
}
