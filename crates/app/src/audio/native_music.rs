//! Native music loading off the main thread.
//!
//! quad-snd decodes a whole OGG inside `Sound::load`, which took 80–580 ms
//! per track on the main thread. Music therefore gets its own quad-snd
//! `AudioContext`s (each is only a channel to its own mixer thread, so it
//! can move between threads): a worker thread reads the file, takes a
//! context from the pool (or opens one) and runs `Sound::load` there. The
//! main thread only polls for the result, then plays, fades and stops the
//! track by sending messages to that context's mixer.

use std::borrow::Cow;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex, PoisonError};

use quad_snd::{AudioContext, PlaySoundParams, Sound};

/// A value being made on a worker thread.
///
/// Dropping it before the worker is done detaches the thread: the worker
/// finishes and drops the value itself, so nothing leaks.
pub struct Pending<T> {
    rx: Receiver<T>,
    /// The value (or the worker's failure) has been handed out.
    taken: bool,
}

impl<T: Send + 'static> Pending<T> {
    /// Runs `work` on a new thread named `name`.
    pub(crate) fn spawn(name: &str, work: impl FnOnce() -> T + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                // No one waiting any more: the value is dropped here.
                let _ = tx.send(work());
            });
        // A thread that couldn't start drops `tx`: `poll` reports it.
        drop(spawned);
        Self { rx, taken: false }
    }

    /// The value once the worker is done: `None` before that, then the
    /// value exactly once, then `None` again. A worker that died without
    /// one (it panicked or never started) yields an `Err` instead.
    pub(crate) fn poll(&mut self) -> Option<Result<T, String>> {
        if self.taken {
            return None;
        }
        let result = match self.rx.try_recv() {
            Ok(value) => Ok(value),
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err("the loading thread stopped".to_owned()),
        };
        self.taken = true;
        Some(result)
    }
}

/// Music contexts not in use, kept for the next track: quad-snd's mixer
/// threads never exit, so each context is opened once and reused.
#[derive(Clone, Default)]
pub(crate) struct Contexts(Arc<Mutex<Vec<AudioContext>>>);

impl Contexts {
    fn take(&self) -> AudioContext {
        let spare = self.0.lock().unwrap_or_else(PoisonError::into_inner).pop();
        spare.unwrap_or_else(AudioContext::new)
    }

    fn put(&self, ctx: AudioContext) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(ctx);
    }
}

/// A decoded music track on its own context. Dropping it stops it, frees
/// its samples and returns the context to the pool.
pub struct Music {
    sound: Sound,
    /// How long it plays, if the file is an OGG, a FLAC or an MP3.
    length_ms: Option<u32>,
    /// Always `Some` until dropped.
    ctx: Option<AudioContext>,
    contexts: Contexts,
}

impl Music {
    /// Reads and decodes the music file at `path`. Runs on a worker thread.
    pub(crate) fn load(path: &str, contexts: Contexts) -> Result<Self, String> {
        let file = std::fs::read(path).map_err(|e| e.to_string())?;
        let (bytes, length_ms) = track_bytes(path, &file)?;
        let ctx = contexts.take();
        // quad-snd `unwrap`s the decode; a corrupt file must not cost the
        // context (its mixer thread would idle forever unused).
        // Caught on purpose, so no crash report (`crate::crash::quiet`).
        let decoded =
            crate::crash::quiet(|| catch_unwind(AssertUnwindSafe(|| Sound::load(&ctx, &bytes))));
        if let Ok(sound) = decoded {
            Ok(Self {
                sound,
                length_ms,
                ctx: Some(ctx),
                contexts,
            })
        } else {
            contexts.put(ctx);
            Err("not a readable OGG or WAV file".to_owned())
        }
    }

    /// How long it plays, in milliseconds, if the file is an OGG, a FLAC
    /// or an MP3.
    pub(crate) fn length_ms(&self) -> Option<u32> {
        self.length_ms
    }

    pub(crate) fn play(&self, volume: f32, looped: bool) {
        if let Some(ctx) = &self.ctx {
            self.sound.play(ctx, PlaySoundParams { looped, volume });
        }
    }

    pub(crate) fn set_volume(&self, volume: f32) {
        if let Some(ctx) = &self.ctx {
            self.sound.set_volume(ctx, volume);
        }
    }

    pub(crate) fn stop(&self) {
        if let Some(ctx) = &self.ctx {
            self.sound.stop(ctx);
        }
    }
}

/// What quad-snd decodes of the track file at `path`, whose bytes are
/// `file`, and how long it plays: a FLAC or MP3 file as the WAV
/// [`decode_to_wav`](shuyi_assets::audio::decode_to_wav) makes of it, any
/// other as it is. Its length is
/// [`track_length_ms`](shuyi_assets::audio::track_length_ms)'s.
pub(crate) fn track_bytes<'a>(
    path: &str,
    file: &'a [u8],
) -> Result<(Cow<'a, [u8]>, Option<u32>), String> {
    let length_ms = shuyi_assets::audio::track_length_ms(path, file);
    let bytes = match shuyi_assets::audio::decode_to_wav(path, file) {
        None => Cow::Borrowed(file),
        Some(wav) => Cow::Owned(wav?),
    };
    Ok((bytes, length_ms))
}

impl Drop for Music {
    fn drop(&mut self) {
        if let Some(ctx) = self.ctx.take() {
            // Stops it wherever it plays and frees the samples.
            self.sound.delete(&ctx);
            self.contexts.put(ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::RecvTimeoutError;
    use std::time::Duration;

    use super::*;

    const WAIT: Duration = Duration::from_secs(10);

    /// Polls `pending` until it yields, like `Audio` does once per frame.
    fn wait<T: Send + 'static>(pending: &mut Pending<T>) -> Result<T, String> {
        let start = std::time::Instant::now();
        loop {
            if let Some(result) = pending.poll() {
                return result;
            }
            assert!(start.elapsed() < WAIT, "the worker never finished");
            std::thread::yield_now();
        }
    }

    #[test]
    fn not_ready_until_the_worker_is_done_then_yields_once() {
        let (go_tx, go_rx) = mpsc::channel::<()>();
        let mut pending =
            Pending::spawn("test-load", move || go_rx.recv_timeout(WAIT).map(|()| 42));
        for _ in 0..5 {
            assert!(pending.poll().is_none(), "the worker is still waiting");
        }
        go_tx.send(()).ok();
        assert_eq!(wait(&mut pending), Ok(Ok(42)));
        assert!(pending.poll().is_none(), "yielded exactly once");
        assert!(pending.poll().is_none());
    }

    /// Records its own drop.
    struct Tracked(mpsc::Sender<()>);

    impl Drop for Tracked {
        fn drop(&mut self) {
            self.0.send(()).ok();
        }
    }

    #[test]
    fn a_dropped_pending_value_is_dropped_by_the_worker() {
        let (go_tx, go_rx) = mpsc::channel::<()>();
        let (dropped_tx, dropped_rx) = mpsc::channel();
        let pending = Pending::spawn("test-load", move || {
            go_rx.recv_timeout(WAIT).ok();
            Tracked(dropped_tx)
        });
        drop(pending);
        assert_eq!(
            dropped_rx.recv_timeout(Duration::from_millis(50)),
            Err(RecvTimeoutError::Timeout),
            "the value doesn't exist yet"
        );
        go_tx.send(()).ok();
        assert_eq!(dropped_rx.recv_timeout(WAIT), Ok(()), "freed, not leaked");
    }

    #[test]
    fn a_worker_that_panics_is_an_error_once() {
        let mut pending: Pending<u8> = Pending::spawn("test-load", || panic!("decode failed"));
        assert_eq!(wait(&mut pending), Err("the loading thread stopped".into()));
        assert!(pending.poll().is_none());
    }

    /// Before and after moving loading off the game's thread: how long the calling
    /// thread is blocked starting each track in the folder named by
    /// `MUSIC_DIR`, synchronously (quad-snd's own `Sound::load`, as before)
    /// and through `Pending`. Opens the real sound device, so it only runs
    /// when asked:
    /// `MUSIC_DIR=<folder> cargo test --release -p shuyi-app -- --ignored --nocapture music_load_timings`
    #[test]
    #[ignore = "needs a sound device and a folder of music"]
    fn music_load_timings() {
        use std::time::Instant;

        let root = std::env::var("MUSIC_DIR").unwrap_or_default();
        let mut files: Vec<_> = std::fs::read_dir(root)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "ogg"))
            .collect();
        files.sort();
        let contexts = Contexts::default();
        let sync_ctx = AudioContext::new();
        for file in files {
            let path = file.to_string_lossy().into_owned();
            let bytes = std::fs::read(&file).unwrap_or_default();
            let t = Instant::now();
            let sound = Sound::load(&sync_ctx, &bytes);
            let before = t.elapsed();
            sound.delete(&sync_ctx);

            let t = Instant::now();
            let (worst, mut polls) = {
                let c = contexts.clone();
                let p = path.clone();
                let mut pending = Pending::spawn("music-load", move || Music::load(&p, c));
                let mut worst = t.elapsed();
                let mut polls = 0;
                loop {
                    let poll = Instant::now();
                    let done = pending.poll();
                    worst = worst.max(poll.elapsed());
                    polls += 1;
                    if let Some(result) = done {
                        let music = result.and_then(|r| r).map_err(|e| format!("{path}: {e}"));
                        assert!(music.is_ok(), "{music:?}", music = music.err());
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(16));
                }
                (worst, polls)
            };
            polls -= 1;
            let name = file.file_name().unwrap_or_default().to_string_lossy();
            eprintln!(
                "{name:32} before {:6.1} ms | after: worst main-thread call {:6.3} ms, ready after {:6.1} ms ({polls} frames)",
                before.as_secs_f64() * 1e3,
                worst.as_secs_f64() * 1e3,
                t.elapsed().as_secs_f64() * 1e3,
            );
        }
    }

    /// This process's memory in use, in KiB, as Windows' `tasklist` reports
    /// it.
    fn working_set_kib() -> Option<u64> {
        let pid = format!("PID eq {}", std::process::id());
        let out = std::process::Command::new("tasklist")
            .args(["/FI", &pid, "/NH"])
            .output()
            .ok()?;
        let row = String::from_utf8_lossy(&out.stdout).into_owned();
        // The row ends `… 12,776 K`.
        let memory = row.split_whitespace().rev().nth(1)?;
        let digits: String = memory.chars().filter(char::is_ascii_digit).collect();
        digits.parse().ok()
    }

    /// The memory 20 loaded voice clips
    /// take. Loads the clip named by `VOICE_CLIP` 20 times, as the voice
    /// player would 20 different clips. Opens the real sound device and
    /// reads `tasklist`, so it only runs when asked, on Windows:
    /// `VOICE_CLIP=<file.ogg> cargo test --release -p shuyi-app -- --ignored --nocapture voice_memory`
    #[test]
    #[ignore = "needs a sound device, a voice clip and Windows' tasklist"]
    fn voice_memory_with_20_clips() {
        let path = std::env::var("VOICE_CLIP").unwrap_or_default();
        let contexts = Contexts::default();
        // The device and one clip first, so only the clips are counted.
        let first = Music::load(&path, contexts.clone());
        std::thread::sleep(Duration::from_millis(500));
        let before = working_set_kib();
        let clips: Vec<_> = (0..20)
            .map(|_| Music::load(&path, contexts.clone()))
            .collect();
        std::thread::sleep(Duration::from_millis(500));
        let after = working_set_kib();
        assert!(first.is_ok() && clips.iter().all(Result::is_ok));
        let length = first.ok().and_then(|m| m.length_ms());
        eprintln!("{path}: {length:?} ms; 20 clips: {before:?} KiB -> {after:?} KiB");
    }

    /// An OGG Vorbis file's framing, as `ogg_length_ms` reads it: a first
    /// page with a stereo 44.1 kHz identification header (30 bytes, as it
    /// declares), then a last page that puts the stream at `samples`.
    fn ogg_framing(samples: u64) -> Vec<u8> {
        let mut ogg = b"OggS".to_vec();
        ogg.extend_from_slice(&[0; 22]);
        ogg.extend_from_slice(&[1, 30]);
        ogg.extend_from_slice(b"vorbis    ");
        ogg.extend_from_slice(&44_100_u32.to_le_bytes());
        ogg.resize(28 + 30, 0);
        ogg.extend_from_slice(b"OggS ");
        ogg.extend_from_slice(&samples.to_le_bytes());
        ogg.extend_from_slice(&[0; 12]);
        ogg.extend_from_slice(&[1, 4]);
        ogg.extend_from_slice(&[0; 4]);
        ogg
    }

    #[test]
    fn a_flac_track_is_its_wav_and_an_ogg_track_its_own_bytes() {
        let flac = include_bytes!("../../../assets/tests/fixtures/formats/tone.flac");
        let (wav, length) = track_bytes("music/tone.flac", flac).unwrap();
        assert!(matches!(wav, Cow::Owned(_)));
        assert_eq!(&wav[..4], b"RIFF");
        // 50 ms: 2,205 stereo frames of 16 bits, after the 44-byte header.
        assert_eq!((wav.len(), length), (44 + 2205 * 4, Some(50)));
        let ogg = ogg_framing(88_200);
        let (bytes, length) = track_bytes("music/two.ogg", &ogg).unwrap();
        assert!(matches!(bytes, Cow::Borrowed(b) if *b == *ogg));
        assert_eq!(length, Some(2000));
        let broken = track_bytes("music/tone.flac", &flac[..40]);
        assert!(broken.is_err_and(|why| why.starts_with("not a readable FLAC file")));
    }

    #[test]
    fn a_broken_flac_track_fails_before_touching_the_device() {
        let path = std::env::temp_dir().join(format!("shuyi-2234-{}.flac", std::process::id()));
        std::fs::write(&path, b"fLaC and nothing more").unwrap();
        let contexts = Contexts::default();
        let result = Music::load(&path.to_string_lossy(), contexts.clone());
        std::fs::remove_file(&path).ok();
        assert!(result.is_err());
        let pooled = contexts.0.lock().map_or_default(|c| c.len());
        assert_eq!(pooled, 0, "no context was opened");
    }

    #[test]
    fn a_missing_file_fails_before_touching_the_device() {
        let contexts = Contexts::default();
        let result = Music::load("no/such/music/track.ogg", contexts.clone());
        assert!(result.is_err());
        let pooled = contexts.0.lock().map_or_default(|c| c.len());
        assert_eq!(pooled, 0, "no context was opened");
    }
}
