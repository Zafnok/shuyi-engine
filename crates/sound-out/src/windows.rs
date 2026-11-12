//! Windows' output: CPAL on the default device, written from its documentation
//! (ADR-0066). A stream lives on a thread of this module's, which also watches for
//! its device going away: CPAL's documentation leaves open whether a stream follows
//! the default device here, so it covers both ways, a stream rerouted and one lost,
//! and a default that changed under a stream still running.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, ErrorKind, SampleFormat, SizedSample, Stream, StreamConfig};

use crate::convert_impl::{Boundary, DeviceSample};
use crate::sys::raised::Raised;
use crate::{Callback, OutputConfig, OutputError};

/// How often the thread looks at the default device: well under the second a player
/// has to be heard on the speakers again.
const WATCH: Duration = Duration::from_millis(200);
/// The most a stream may take to open.
const OPEN: Duration = Duration::from_secs(5);
/// How many looks in a row must see another default device before the stream moves:
/// one look that finds none, as the system's list changes, is no reason to drop a
/// stream that plays.
const MOVED_LOOKS: u32 = 2;

/// What the output's owner asks of its thread.
enum Order {
    Hold(bool),
    Close,
}

/// See the [module docs](self).
pub(crate) struct WindowsOutput {
    orders: Sender<Order>,
    thread: Option<JoinHandle<()>>,
    stats: Arc<Stats>,
}

/// What a session of the output has been through, for a probe to read.
#[derive(Debug, Default)]
pub(crate) struct Stats {
    /// How many times a stream was built after the first.
    pub(crate) rebuilds: AtomicU32,
    /// How many callbacks were silence, the mix's lock held elsewhere.
    pub(crate) silent: AtomicU32,
    /// How many callbacks found the device's buffer empty ([`Dry`]).
    pub(crate) dry: AtomicU32,
    /// Whether the latest stream's thread runs above normal (`sys::raised`).
    pub(crate) raised: AtomicBool,
}

/// Tells when a callback finds the device's buffer empty: the device has played all
/// of it, and nothing more, since the callback before, so a gap is heard (ticket
/// 3089). CPAL asks each time for the buffer's room, which is all of it only at the
/// first callback and after it ran dry: at its pace a callback finds about one
/// period's room.
#[derive(Debug, Default)]
struct Dry {
    /// The most room a callback was given, in samples: the whole buffer.
    whole: usize,
}

impl Dry {
    /// Whether a callback given `room` samples found the buffer empty.
    fn ran_dry(&mut self, room: usize) -> bool {
        let dry = self.whole > 0 && room >= self.whole;
        self.whole = self.whole.max(room);
        dry
    }
}

fn device_error(error: &cpal::Error) -> OutputError {
    match error.kind() {
        ErrorKind::DeviceNotAvailable | ErrorKind::HostUnavailable => OutputError::NoDevice,
        _ => OutputError::Device(error.to_string()),
    }
}

/// The rate the default device runs at.
pub(crate) fn preferred_rate() -> Option<u32> {
    let device = cpal::default_host().default_output_device()?;
    Some(device.default_output_config().ok()?.sample_rate())
}

/// A stream on `device`: paused, as they start. `lost` is set when it stops being any
/// use. Samples convert on the audio thread, and never wait: only one stream runs at
/// a time, so `try_lock` finds the callback free, and otherwise this block is silent.
fn build<T: SizedSample + DeviceSample + Send + 'static>(
    device: &cpal::Device,
    stream: StreamConfig,
    config: OutputConfig,
    callback: Callback,
    lost: Arc<AtomicBool>,
    stats: Arc<Stats>,
) -> Result<Stream, OutputError> {
    let from = usize::from(config.channels.count());
    let mut boundary = Boundary::new(
        from,
        config.rate,
        usize::try_from(config.frames).unwrap_or(1),
        usize::from(stream.channels),
        stream.sample_rate,
    );
    let silence = T::from_mix(0);
    let on_error = move |error: cpal::Error| match error.kind() {
        // Rerouted, or a glitch or a refused priority that it plays on through.
        ErrorKind::DeviceChanged | ErrorKind::Xrun | ErrorKind::RealtimeDenied => {}
        _ => lost.store(true, Ordering::Release),
    };
    // Raised at the first callback, the only way onto CPAL's thread, and let go with
    // the callback, which CPAL drops on that thread.
    let mut raised: Option<Raised> = None;
    let mut dry = Dry::default();
    device
        .build_output_stream::<T, _, _>(
            stream,
            move |out: &mut [T], _| {
                if raised.is_none() {
                    let now = Raised::current();
                    stats.raised.store(now.is_raised(), Ordering::Relaxed);
                    raised = Some(now);
                }
                if dry.ran_dry(out.len()) {
                    stats.dry.fetch_add(1, Ordering::Relaxed);
                }
                if let Ok(mut callback) = callback.try_lock() {
                    boundary.fill(out, &mut *callback);
                } else {
                    stats.silent.fetch_add(1, Ordering::Relaxed);
                    out.fill(silence);
                }
            },
            on_error,
            Some(OPEN),
        )
        .map_err(|e| device_error(&e))
}

/// A stream on the default device, in the device's own format, playing unless held.
fn open_stream(
    host: &cpal::Host,
    config: OutputConfig,
    callback: &Callback,
    lost: &Arc<AtomicBool>,
    stats: &Arc<Stats>,
    held: bool,
) -> Result<(Stream, cpal::Device), OutputError> {
    let device = host.default_output_device().ok_or(OutputError::NoDevice)?;
    let supported = device
        .default_output_config()
        .map_err(|e| device_error(&e))?;
    let format = supported.sample_format();
    let mut stream_config: StreamConfig = supported.config();
    stream_config.buffer_size = BufferSize::Default;
    let (shared, flag, counted) = (Arc::clone(callback), Arc::clone(lost), Arc::clone(stats));
    let stream = match format {
        SampleFormat::F32 => build::<f32>(&device, stream_config, config, shared, flag, counted),
        SampleFormat::F64 => build::<f64>(&device, stream_config, config, shared, flag, counted),
        SampleFormat::I16 => build::<i16>(&device, stream_config, config, shared, flag, counted),
        SampleFormat::I32 => build::<i32>(&device, stream_config, config, shared, flag, counted),
        SampleFormat::U16 => build::<u16>(&device, stream_config, config, shared, flag, counted),
        SampleFormat::U8 => build::<u8>(&device, stream_config, config, shared, flag, counted),
        other => Err(OutputError::Device(format!("a device format of {other:?}"))),
    }?;
    if !held {
        stream.play().map_err(|e| device_error(&e))?;
    }
    Ok((stream, device))
}

impl WindowsOutput {
    pub(crate) fn open(config: OutputConfig, callback: Callback) -> Result<Self, OutputError> {
        let (orders, inbox) = mpsc::channel();
        let (opened, answer) = mpsc::channel();
        let stats = Arc::new(Stats::default());
        let counted = Arc::clone(&stats);
        let thread = std::thread::Builder::new()
            .name("sound-out".to_owned())
            .spawn(move || run(config, &callback, &counted, &inbox, &opened))
            .map_err(|e| OutputError::Device(e.to_string()))?;
        match answer.recv_timeout(OPEN * 2) {
            Ok(Ok(())) => Ok(Self {
                orders,
                thread: Some(thread),
                stats,
            }),
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(error)
            }
            Err(_) => Err(OutputError::Device(
                "the device didn't open in time".to_owned(),
            )),
        }
    }

    pub(crate) fn hold(&self, on: bool) {
        let _ = self.orders.send(Order::Hold(on));
    }

    pub(crate) fn stats(&self) -> (u32, u32) {
        (
            self.stats.rebuilds.load(Ordering::Relaxed),
            self.stats.silent.load(Ordering::Relaxed),
        )
    }

    pub(crate) fn dry_callbacks(&self) -> u32 {
        self.stats.dry.load(Ordering::Relaxed)
    }

    pub(crate) fn raised_priority(&self) -> bool {
        self.stats.raised.load(Ordering::Relaxed)
    }
}

impl Drop for WindowsOutput {
    fn drop(&mut self) {
        let _ = self.orders.send(Order::Close);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The thread: opens the stream, says how that went, then keeps a stream going until
/// told otherwise.
fn run(
    config: OutputConfig,
    callback: &Callback,
    stats: &Arc<Stats>,
    inbox: &mpsc::Receiver<Order>,
    opened: &Sender<Result<(), OutputError>>,
) {
    let host = cpal::default_host();
    let lost = Arc::new(AtomicBool::new(false));
    let mut held = false;
    let mut current = match open_stream(&host, config, callback, &lost, stats, held) {
        Ok(stream) => {
            let _ = opened.send(Ok(()));
            Some(stream)
        }
        Err(error) => {
            let _ = opened.send(Err(error));
            return;
        }
    };
    let mut quiet = Quiet::new(config);
    let mut moved_looks = 0;
    loop {
        match inbox.recv_timeout(if current.is_some() {
            WATCH
        } else {
            quiet.period
        }) {
            Ok(Order::Close) | Err(RecvTimeoutError::Disconnected) => return,
            Ok(Order::Hold(on)) => {
                held = on;
                if let Some((stream, _)) = &current {
                    // A device that can't pause is one whose voices the player holds.
                    let _ = if on { stream.pause() } else { stream.play() };
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
        let moved = current
            .as_ref()
            .is_some_and(|(_, device)| host.default_output_device().as_ref() != Some(device));
        moved_looks = if moved { moved_looks + 1 } else { 0 };
        if moved_looks >= MOVED_LOOKS || lost.swap(false, Ordering::AcqRel) || current.is_none() {
            // The old stream goes first: only one may hold the callback.
            moved_looks = 0;
            drop(current.take());
            current = open_stream(&host, config, callback, &lost, stats, held).ok();
            stats.rebuilds.fetch_add(1, Ordering::Relaxed);
        }
        if current.is_none() && !held {
            quiet.run(callback);
        }
    }
}

/// With no device for now the mix still goes on, discarded, so that what plays ends
/// when it should and the game's commands don't pile up.
struct Quiet {
    block: Vec<i16>,
    frames: u32,
    rate: u32,
    period: Duration,
    last: Instant,
}

impl Quiet {
    fn new(config: OutputConfig) -> Self {
        let channels = usize::from(config.channels.count());
        let frames = config.frames.max(1);
        Self {
            block: vec![0; usize::try_from(frames).unwrap_or(1) * channels],
            frames,
            rate: config.rate.max(1),
            period: Duration::from_millis(20),
            last: Instant::now(),
        }
    }

    fn run(&mut self, callback: &Callback) {
        let frames = self.last.elapsed().as_secs_f64() * f64::from(self.rate);
        // Whole blocks of what has gone by, and the most a pause in here could owe.
        let blocks = (frames / f64::from(self.frames)).min(64.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let blocks = blocks as u32;
        if blocks == 0 {
            return;
        }
        self.last = Instant::now();
        if let Ok(mut callback) = callback.try_lock() {
            for _ in 0..blocks {
                callback(&mut self.block);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// As CPAL asks on a buffer of 1,000 samples played 400 at a time.
    #[test]
    fn only_a_callback_given_the_whole_buffer_after_the_first_ran_dry() {
        let mut dry = Dry::default();
        assert!(!dry.ran_dry(1_000), "the first fills an empty buffer");
        for room in [400, 400, 600, 400] {
            assert!(!dry.ran_dry(room), "{room} is a callback at its pace");
        }
        assert!(dry.ran_dry(1_000), "late: the device played it all");
        assert!(!dry.ran_dry(400));
    }

    /// A first callback given less than the buffer: the empty buffer later is larger.
    #[test]
    fn more_room_than_ever_is_dry_too() {
        let mut dry = Dry::default();
        assert!(!dry.ran_dry(400));
        assert!(dry.ran_dry(1_000));
        assert!(dry.ran_dry(1_000));
        assert!(!dry.ran_dry(999));
    }
}
