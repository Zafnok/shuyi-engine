//! The rules the system's output keeps a stream by, apart from CPAL so that they are
//! tested alone (ticket 3032): how much buffer a stream asks for, which errors end one,
//! when one that went quiet counts as lost, and the audio thread's priority where it
//! isn't raised.

use std::time::{Duration, Instant};

use cpal::{BufferSize, ErrorKind, SupportedBufferSize};

/// Whether a stream asks for [`buffer_size`]'s frames rather than the system's own
/// choice: on Linux, where ALSA's own choice through a sound server's plugin can be
/// seconds of buffer (ticket 3032).
pub(super) const FIXED_BUFFER: bool = cfg!(target_os = "linux");

/// The frames a stream asks to be called for: about 20 ms when `fixed`, inside what
/// the device allows; otherwise the system's own choice, as on Windows and macOS.
pub(super) fn buffer_size(supported: &SupportedBufferSize, rate: u32, fixed: bool) -> BufferSize {
    if !fixed {
        return BufferSize::Default;
    }
    let want = (rate / 50).max(1);
    match *supported {
        SupportedBufferSize::Range { min, max } => BufferSize::Fixed(want.max(min).min(max)),
        SupportedBufferSize::Unknown => BufferSize::Fixed(want),
    }
}

/// Whether a system's errors with no kind of their own pass, as long as the stream
/// still calls back: on Linux, ALSA's plugin for `PulseAudio` says an I/O error early
/// in a stream that plays on (`snd_pcm_avail_delay`, 4 scenarios in 36 of CI run
/// 45205209022), and building it again would cut the sound (ticket 3032). A stream
/// that did end stops calling back, and [`Stall`] builds it again.
pub(super) const PASSING_BACKEND_ERRORS: bool = cfg!(target_os = "linux");

/// Whether an error of `kind` ends a stream, so that it is built again at once: the
/// device or the system is gone, the stream is no use, or the error is one the system
/// couldn't name (`backend_passes`: unless such errors pass, as on Linux). A glitch, a
/// reroute or a refused priority never reaches here.
pub(super) const fn ends_the_stream(kind: ErrorKind, backend_passes: bool) -> bool {
    !matches!(kind, ErrorKind::BackendError if backend_passes)
}

/// How long a stream that plays may go without a callback before it counts as lost: a
/// stream on ALSA's plugin for `PulseAudio` can stop calling back with no error at all (ticket
/// 3032), and every system calls back many times a second.
const STALLED: Duration = Duration::from_millis(500);

/// Tells a stream that stopped calling back, as a lost one doesn't always say so.
#[derive(Debug, Clone, Copy)]
pub(super) struct Stall {
    calls: u32,
    since: Instant,
}

impl Stall {
    pub(super) const fn new(calls: u32, now: Instant) -> Self {
        Self { calls, since: now }
    }

    /// Whether a stream whose callbacks number `calls` at `now` has made none for
    /// [`STALLED`]. A held stream is meant to be quiet, so it never has.
    pub(super) fn stalled(&mut self, calls: u32, held: bool, now: Instant) -> bool {
        if held || calls != self.calls {
            *self = Self::new(calls, now);
            return false;
        }
        now.duration_since(self.since) >= STALLED
    }
}

/// The audio thread's priority, raised on Windows alone (ticket 3089): on Linux and
/// macOS it is CPAL's own thread as CPAL makes it, whose priority ticket 3320 decides.
#[cfg(not(windows))]
#[derive(Debug)]
pub(super) enum Raised {
    Not,
}

#[cfg(not(windows))]
impl Raised {
    pub(super) const fn current() -> Self {
        Self::Not
    }

    pub(super) const fn is_raised(&self) -> bool {
        !matches!(self, Self::Not)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every error ends a stream on Windows and macOS; on Linux an unnamed one passes.
    #[test]
    fn an_unnamed_error_passes_only_where_it_should() {
        for kind in [
            ErrorKind::DeviceNotAvailable,
            ErrorKind::HostUnavailable,
            ErrorKind::StreamInvalidated,
            ErrorKind::Other,
        ] {
            assert!(ends_the_stream(kind, true), "{kind:?}");
            assert!(ends_the_stream(kind, false), "{kind:?}");
        }
        assert!(ends_the_stream(ErrorKind::BackendError, false));
        assert!(!ends_the_stream(ErrorKind::BackendError, true));
    }

    /// A stream counts as stalled after half a second with no callback while it
    /// plays, never while held, and a callback starts the wait again.
    #[test]
    fn a_stream_that_stops_calling_back_stalls() {
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        let mut stall = Stall::new(0, start);
        assert!(!stall.stalled(0, false, at(200)));
        assert!(!stall.stalled(0, false, at(499)));
        assert!(stall.stalled(0, false, at(500)), "half a second with none");
        assert!(!stall.stalled(1, false, at(600)), "a callback came");
        assert!(!stall.stalled(1, false, at(1_000)));
        assert!(stall.stalled(1, false, at(1_100)));
        assert!(!stall.stalled(1, true, at(5_000)), "held: quiet on purpose");
        assert!(
            !stall.stalled(1, false, at(5_400)),
            "the wait starts when let go"
        );
        assert!(stall.stalled(1, false, at(5_500)));
    }

    /// Linux asks for about 20 ms, inside what the device allows; elsewhere the
    /// system chooses.
    #[test]
    fn a_fixed_buffer_is_20_ms_inside_the_devices_range() {
        let range = |min, max| SupportedBufferSize::Range { min, max };
        assert_eq!(
            buffer_size(&range(64, 8192), 48_000, true),
            BufferSize::Fixed(960)
        );
        assert_eq!(
            buffer_size(&range(64, 8192), 44_100, true),
            BufferSize::Fixed(882)
        );
        assert_eq!(
            buffer_size(&range(2048, 8192), 48_000, true),
            BufferSize::Fixed(2048)
        );
        assert_eq!(
            buffer_size(&range(64, 512), 48_000, true),
            BufferSize::Fixed(512)
        );
        assert_eq!(
            buffer_size(&SupportedBufferSize::Unknown, 48_000, true),
            BufferSize::Fixed(960)
        );
        assert_eq!(
            buffer_size(&range(64, 8192), 48_000, false),
            BufferSize::Default
        );
        assert_eq!(
            buffer_size(&range(64, 8192), 0, true),
            BufferSize::Fixed(64)
        );
    }
}
