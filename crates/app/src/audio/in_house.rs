//! The sound device behind the `in-house-sound` feature (ADR-0066, ADR-0074): the
//! shared player's rules over `shuyi-sound`'s mixer and `shuyi-sound-out`'s output.
//!
//! The mixer stays on the game's thread until the first play, so that sounds loading
//! at the start go straight into its bank, and then moves to the callback, which only
//! ever applies what comes and mixes. From there:
//!
//! - **Commands** go as words through the mixer's own queue ([`Remote`]).
//! - **Sounds and streams** go through a hand-over of boxes, which the callback fills
//!   in and sends back to be freed here: it allocates and frees nothing. This side
//!   names their places ([`Remote::reserve_sound`]), so nothing waits on the audio
//!   thread, and a play sent after its sound always finds it (`Inbox::apply_with`).
//! - **Music and voices** stream. A [`Feeding`] on a worker decodes ahead into a ring
//!   a track's length doesn't change. It loops by starting its decoder again, so a
//!   loop's way back is in the ring before the mix gets there. In the browser there
//!   are no threads, and the feeding is done as the page asks for frames.
//!
//! **What the web hands the adapters.** A library's panic stops the program there, as
//! `wasm32-unknown-unknown` doesn't unwind. So each file goes to the browser's own
//! decoder first, which runs off the page's thread and refuses a corrupt file with an
//! error, and only a file it read goes on. A file that it reads and a library panics
//! on would still stop the program; the game's own files are all that get here.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use shuyi_platform::audio::player::Backend;
use shuyi_sound::mix::queue::{Inbox, Remote};
use shuyi_sound::mix::stream::{self, Feeder, LoopedRegion};
use shuyi_sound::mix::{
    Config, Mixer, Params, Region, SharedPcm, SoundId, StreamId, UNITY, VoiceId,
};
use shuyi_sound::vorbis::VorbisSource;
use shuyi_sound::wav::WavSource;
use shuyi_sound::{Channels, Error, Limits, Source, read_all};
use shuyi_sound_out::{HandOut, HandTo, Output, OutputConfig, handover};

#[cfg(not(target_arch = "wasm32"))]
use super::native_music::Pending;

/// The most voices at once: what the cue rules let sound, while no manifest says.
const VOICES: usize = 32;
/// The most decoded sounds held, variants each counting.
const SOUNDS: usize = 1024;
/// The most tracks at once: a crossfade's two, a voice clip, and those on their way out.
const STREAMS: usize = 6;
/// The most commands waiting: more than a frame sends, by far.
const COMMANDS: usize = 1024;
/// How many hand-overs may be on their way at once.
const JOBS: usize = 64;
/// The frames a ring of decoded sound holds, ahead of the mix, however long the track
/// is: `mixer_memory` in `tools/sound-parity/budget.ron` holds the backend to it.
const FEED_FRAMES: usize = 4096;
/// The frames the callback is asked for at a time, natively and on the web.
const BLOCK_FRAMES: u32 = if cfg!(target_arch = "wasm32") {
    512
} else {
    256
};
/// A volume change's ramp, in milliseconds: a step in a mix's level clicks.
const VOLUME_RAMP_MS: u64 = 8;
/// The mix's rate when the device doesn't say: the corpus's own.
const DEFAULT_RATE: u32 = 44_100;

/// A file's kind, from what it starts with: an asset kind's formats, not its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Wav,
    Ogg,
}

impl Kind {
    fn of(bytes: &[u8]) -> Result<Self, String> {
        if bytes.starts_with(b"RIFF") {
            Ok(Self::Wav)
        } else if bytes.starts_with(b"OggS") {
            Ok(Self::Ogg)
        } else {
            Err("not a readable OGG or WAV file".to_owned())
        }
    }
}

/// `bytes` decoded whole, for the bank.
fn decode(bytes: &[u8]) -> Result<SharedPcm, String> {
    let limits = Limits::default();
    let pcm = match Kind::of(bytes)? {
        Kind::Wav => WavSource::new(bytes, &limits).and_then(|mut s| read_all(&mut s, &limits)),
        Kind::Ogg => VorbisSource::new(bytes, &limits).and_then(|mut s| read_all(&mut s, &limits)),
    };
    pcm.map(SharedPcm::new).map_err(|e| e.to_string())
}

/// `first`, and when `looped` every next pass from `open`, gapless and in the ring
/// before the mix gets there.
fn looping<S, F>(first: S, open: F, looped: bool) -> Box<dyn Source + Send>
where
    S: Source + Send + 'static,
    F: FnMut() -> Result<S, Error> + Send + 'static,
{
    match Region::new(0, u64::MAX).filter(|_| looped) {
        Some(whole) => Box::new(LoopedRegion::new(first, whole, open)),
        None => Box::new(first),
    }
}

/// A source of a track's bytes, decoding as it is asked.
fn track_source(track: &Track, looped: bool) -> Result<Box<dyn Source + Send>, String> {
    // A long track's length is no limit here: it is never all in memory.
    let limits = Limits::new(Limits::DEFAULT_MAX_RATE, u64::MAX);
    let bytes = Arc::clone(&track.bytes);
    let failed = |e: Error| e.to_string();
    Ok(match track.kind {
        Kind::Wav => {
            let first = WavSource::new(Arc::clone(&bytes), &limits).map_err(failed)?;
            looping(
                first,
                move || WavSource::new(Arc::clone(&bytes), &limits),
                looped,
            )
        }
        Kind::Ogg => {
            let first = VorbisSource::new(Arc::clone(&bytes), &limits).map_err(failed)?;
            looping(
                first,
                move || VorbisSource::new(Arc::clone(&bytes), &limits),
                looped,
            )
        }
    })
}

/// A track read from its file, not playing. Its bytes are what it holds.
pub struct Track {
    bytes: Arc<[u8]>,
    kind: Kind,
    /// How long it plays, if it is an OGG file.
    length_ms: Option<u32>,
}

impl Track {
    /// `bytes` as a track: a file whose headers don't read is an error now.
    fn new(bytes: Vec<u8>) -> Result<Self, String> {
        let kind = Kind::of(&bytes)?;
        let length_ms = shuyi_assets::audio::ogg_length_ms(&bytes);
        let track = Self {
            bytes: bytes.into(),
            kind,
            length_ms,
        };
        track_source(&track, false)?;
        Ok(track)
    }
}

/// A decoded sound in the bank, or a track.
pub enum Clip {
    /// A sound of the bank, with what is playing it.
    Effect {
        id: SoundId,
        rate: Cell<u32>,
        voices: RefCell<Vec<VoiceId>>,
    },
    /// A music track or a voice clip, with its playback.
    Stream {
        track: Track,
        rate: Cell<u32>,
        voice: Cell<Option<VoiceId>>,
    },
}

impl Clip {
    fn rate(&self) -> &Cell<u32> {
        match self {
            Self::Effect { rate, .. } | Self::Stream { rate, .. } => rate,
        }
    }
}

/// What goes to the audio thread in a box, and comes back holding what is to be freed.
enum Job {
    /// The mixer itself, at the first play; back comes an empty box.
    Start(Option<Box<(Mixer, Inbox)>>),
    /// A sound for its place; back comes what was there.
    Sound(SoundId, Option<SharedPcm>),
    /// A stream for its slot; back comes what was there.
    Stream(StreamId, Option<Box<dyn Source + Send>>),
    /// Takes a stream out of its slot: back it comes.
    Take(StreamId, Option<Box<dyn Source + Send>>),
}

impl Job {
    /// Does it, with no allocation and nothing freed.
    fn run(&mut self, mixer: &mut Mixer) {
        match self {
            Self::Start(_) => {}
            Self::Sound(id, pcm) => {
                if let Some(new) = pcm.take() {
                    *pcm = mixer.put_sound(*id, new);
                }
            }
            Self::Stream(id, source) => {
                if let Some(new) = source.take() {
                    *source = mixer.put_stream(*id, new);
                }
            }
            Self::Take(id, source) => *source = mixer.take_stream(*id),
        }
    }
}

/// What the callback owns. Nothing here allocates, frees, waits or reads a file.
struct AudioSide {
    seated: Option<Box<(Mixer, Inbox)>>,
    jobs: HandOut<Job>,
    back: HandTo<Job>,
    /// On the web the page's one thread does the decoders' work too.
    #[cfg(target_arch = "wasm32")]
    feeding: Feedings,
    /// When a command last reached the mix, for the probe's measure of latency.
    #[cfg(all(feature = "sound-probe", not(target_arch = "wasm32")))]
    reached: Arc<Reached>,
}

/// Runs what came, and sends each box back to be freed where that may be done.
fn run_jobs(jobs: &mut HandOut<Job>, back: &mut HandTo<Job>, mixer: &mut Mixer) {
    while let Some(mut job) = jobs.take() {
        job.run(mixer);
        // Room for everything on its way, by `JOBS`. If it ever weren't, a leak is
        // what the audio thread can afford and a free isn't.
        if let Err(job) = back.send(job) {
            std::mem::forget(job);
        }
    }
}

impl AudioSide {
    fn fill(&mut self, out: &mut [i16]) {
        #[cfg(target_arch = "wasm32")]
        self.feeding.step();
        // The mixer's own box comes first, and what came after it runs below.
        while self.seated.is_none() {
            let Some(mut job) = self.jobs.take() else {
                break;
            };
            if let Job::Start(seated) = &mut *job {
                self.seated = seated.take();
            }
            if let Err(job) = self.back.send(job) {
                std::mem::forget(job);
            }
        }
        let Some(seated) = &mut self.seated else {
            out.fill(0);
            return;
        };
        let (mixer, inbox) = &mut **seated;
        let (jobs, back) = (&mut self.jobs, &mut self.back);
        #[cfg(all(feature = "sound-probe", not(target_arch = "wasm32")))]
        let reached = &self.reached;
        inbox.apply_with(mixer, |mixer| {
            run_jobs(jobs, back, mixer);
            #[cfg(all(feature = "sound-probe", not(target_arch = "wasm32")))]
            reached.stamp();
        });
        run_jobs(jobs, back, mixer);
        mixer.render(out);
    }
}

/// A worker's decoding of one track into its ring.
struct Feeding {
    feeder: Feeder,
    source: Box<dyn Source + Send>,
    stop: Arc<AtomicBool>,
}

impl Feeding {
    /// Tops the ring up: whether there is more to do, later. A file that goes wrong
    /// partway ends there, and says so.
    fn step(&mut self, problems: &Mutex<Vec<String>>) -> bool {
        if self.stop.load(Ordering::Acquire) {
            return false;
        }
        match self.feeder.fill_from(&mut self.source) {
            Ok(_) => !self.feeder.is_closed(),
            Err(error) => {
                self.feeder.close();
                problems
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(format!("a track stopped partway: {error}"));
                false
            }
        }
    }
}

/// Every track being decoded, on the worker's side.
#[derive(Clone, Default)]
struct Feedings {
    list: Arc<Mutex<Vec<Feeding>>>,
    problems: Arc<Mutex<Vec<String>>>,
}

impl Feedings {
    fn add(&self, feeding: Feeding) {
        self.list
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(feeding);
    }

    /// Tops up each ring, and lets go of those done with.
    fn step(&self) {
        // Only the game's thread adds, and only briefly; a turn missed comes round.
        if let Ok(mut list) = self.list.try_lock() {
            list.retain_mut(|feeding| feeding.step(&self.problems));
        }
    }

    fn take_problems(&self) -> Vec<String> {
        std::mem::take(&mut *self.problems.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// The worker thread: it keeps every ring topped up until told to stop.
#[cfg(not(target_arch = "wasm32"))]
struct Worker {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Worker {
    fn start(feedings: Feedings) -> Self {
        // A tenth of a ring: the mix can't catch it up from there.
        let period = std::time::Duration::from_millis(10);
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("sound-feed".to_owned())
            .spawn(move || {
                while !stopping.load(Ordering::Acquire) {
                    feedings.step();
                    std::thread::sleep(period);
                }
            })
            .ok();
        Self { stop, thread }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// A stream playing: what ends its feeding, and takes it away, when its voice has ended.
struct Live {
    voice: VoiceId,
    stream: StreamId,
    stop: Arc<AtomicBool>,
}

/// The mixer, the way to it and what feeds it.
struct Engine {
    remote: Remote,
    /// The mixer, until the first play.
    here: Option<Box<(Mixer, Inbox)>>,
    jobs: HandTo<Job>,
    back: HandOut<Job>,
    /// How many boxes are away.
    away: usize,
    live: VecDeque<Live>,
    feedings: Feedings,
    #[cfg(not(target_arch = "wasm32"))]
    _worker: Worker,
    /// Dropped first: the callback stops before what it uses goes.
    output: Option<Output>,
    #[cfg(all(feature = "sound-probe", not(target_arch = "wasm32")))]
    reached: Arc<Reached>,
}

/// Makes the mixer and its two sides, with no output yet.
fn sides(rate: u32) -> (Engine, AudioSide) {
    let config = Config::new(rate, Channels::Stereo, VOICES)
        .with_sounds(SOUNDS)
        .with_streams(STREAMS);
    let mixer = Mixer::new(config);
    let (remote, inbox) = mixer.remote(COMMANDS);
    let (to_audio, from_game) = handover(JOBS);
    let (to_game, from_audio) = handover(JOBS);
    let feedings = Feedings::default();
    #[cfg(all(feature = "sound-probe", not(target_arch = "wasm32")))]
    let reached = Arc::new(Reached::default());
    let side = AudioSide {
        seated: None,
        jobs: from_game,
        back: to_game,
        #[cfg(target_arch = "wasm32")]
        feeding: feedings.clone(),
        #[cfg(all(feature = "sound-probe", not(target_arch = "wasm32")))]
        reached: Arc::clone(&reached),
    };
    let engine = Engine {
        remote,
        here: Some(Box::new((mixer, inbox))),
        jobs: to_audio,
        back: from_audio,
        away: 0,
        live: VecDeque::new(),
        #[cfg(not(target_arch = "wasm32"))]
        _worker: Worker::start(feedings.clone()),
        feedings,
        output: None,
        #[cfg(all(feature = "sound-probe", not(target_arch = "wasm32")))]
        reached,
    };
    (engine, side)
}

impl Engine {
    /// Hands `job` over, or does it here while the mixer is here. Whether that was
    /// done: with every box away it wasn't.
    fn give(&mut self, mut job: Job) -> bool {
        if let Some(here) = &mut self.here {
            job.run(&mut here.0);
            return true;
        }
        self.collect();
        if self.away >= JOBS {
            return false;
        }
        match self.jobs.send(Box::new(job)) {
            Ok(()) => {
                self.away += 1;
                true
            }
            Err(_) => false,
        }
    }

    /// Frees what has come back, and the places of the streams taken out.
    fn collect(&mut self) {
        while let Some(job) = self.back.take() {
            self.away = self.away.saturating_sub(1);
            if let Job::Take(stream, _) = *job {
                self.remote.release_stream(stream);
            }
        }
    }

    /// The mixer goes to the callback: from here commands reach it.
    fn seat(&mut self) {
        if let Some(here) = self.here.take()
            && self.jobs.send(Box::new(Job::Start(Some(here)))).is_ok()
        {
            self.away += 1;
        }
    }

    /// Ends the feeding of each stream whose voice has ended, and takes it away.
    fn sweep(&mut self) {
        for _ in 0..self.live.len() {
            let Some(live) = self.live.pop_front() else {
                break;
            };
            if self.remote.is_playing(live.voice) {
                self.live.push_back(live);
                continue;
            }
            live.stop.store(true, Ordering::Release);
            if !self.give(Job::Take(live.stream, None)) {
                self.live.push_back(live);
            }
        }
    }
}

/// `volume`, 0–1, as the mixer's.
fn level(volume: f32) -> u16 {
    let scaled = (volume.clamp(0.0, 1.0) * f32::from(UNITY)).round();
    // Between 0 and `UNITY`, which fits.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let level = scaled as u16;
    level
}

/// Plays through the mixer, which it opens with the first sound.
#[derive(Default)]
pub struct InHouse {
    engine: Option<Engine>,
    #[cfg(test)]
    said: Vec<String>,
}

impl InHouse {
    /// Says a problem in the log, as the player's are.
    #[cfg_attr(not(test), allow(clippy::unused_self))]
    fn say(&mut self, line: &str) {
        #[cfg(test)]
        self.said.push(line.to_owned());
        macroquad::logging::warn!("audio: {}", line);
    }

    /// The engine, opened on the system's output, or an output that discards: no
    /// sound, and one line to say so, rather than a crash.
    fn engine(&mut self) -> &mut Engine {
        if self.engine.is_none() {
            let rate = Output::preferred_rate().unwrap_or(DEFAULT_RATE);
            let (mut engine, mut side) = sides(rate);
            let config = OutputConfig::new(rate, Channels::Stereo, BLOCK_FRAMES);
            let (output, problem) = Output::open_or_null(config, move |out| side.fill(out));
            if let Some(problem) = problem {
                self.say(&format!("the game runs silent: {problem}"));
            }
            engine.output = Some(output);
            self.engine = Some(engine);
        }
        match &mut self.engine {
            Some(engine) => engine,
            None => unreachable!("made above"),
        }
    }

    /// Puts `bytes`, decoded, in the bank.
    fn load_effect(&mut self, bytes: &[u8]) -> Result<Clip, String> {
        let pcm = decode(bytes)?;
        let engine = self.engine();
        let id = engine
            .remote
            .reserve_sound()
            .map_err(|_| "the bank of sounds is full".to_owned())?;
        if !engine.give(Job::Sound(id, Some(pcm))) {
            engine.remote.release_sound(id);
            return Err("the sound queue is full".to_owned());
        }
        Ok(Clip::Effect {
            id,
            rate: Cell::new(shuyi_sound::mix::RATE_ONE),
            voices: RefCell::new(Vec::new()),
        })
    }

    /// Starts `track` from its first frame: its stream in the ring and the mixer, its
    /// first frames decoded, so that it starts when asked.
    fn start_stream(
        &mut self,
        track: &Track,
        params: Params,
        looped: bool,
    ) -> Result<VoiceId, String> {
        let source = track_source(track, looped)?;
        let engine = self.engine();
        engine.seat();
        let (feeder, feed) = stream::feed(source.format(), FEED_FRAMES);
        let stream = engine
            .remote
            .reserve_stream()
            .map_err(|_| "no stream is free".to_owned())?;
        if !engine.give(Job::Stream(stream, Some(Box::new(feed)))) {
            engine.remote.release_stream(stream);
            return Err("the sound queue is full".to_owned());
        }
        let stop = Arc::new(AtomicBool::new(false));
        let mut feeding = Feeding {
            feeder,
            source,
            stop: Arc::clone(&stop),
        };
        // Its first frames now, and the worker takes it from there.
        if feeding.step(&engine.feedings.problems) {
            engine.feedings.add(feeding);
        }
        let Ok(voice) = engine.remote.play_stream(stream, params) else {
            stop.store(true, Ordering::Release);
            let _ = engine.give(Job::Take(stream, None));
            return Err("no voice is free".to_owned());
        };
        engine.live.push_back(Live {
            voice,
            stream,
            stop,
        });
        Ok(voice)
    }

    /// What is left to do at any call: the streams that ended, and what the worker
    /// has to say.
    fn tidy(&mut self) {
        let Some(engine) = &mut self.engine else {
            return;
        };
        engine.collect();
        engine.sweep();
        let problems = engine.feedings.take_problems();
        for problem in problems {
            self.say(&problem);
        }
    }

    /// How long a play takes to reach the mix going to the device's buffer, in
    /// milliseconds: the time to the callback that has it, at no volume so that
    /// nothing is heard. For the probe's `latency` order.
    #[cfg(all(feature = "sound-probe", not(target_arch = "wasm32")))]
    pub fn play_latency_ms(&mut self, sound: &Clip) -> Option<f64> {
        let started = std::time::Instant::now();
        let engine = self.engine.as_ref()?;
        let reached = Arc::clone(&engine.reached);
        let playing = self.play(sound, 0.0, false);
        let waited = reached.wait_after(started)?;
        self.stop_playing(playing);
        Some(waited * 1000.0)
    }
}

impl Backend for InHouse {
    type Sound = Clip;
    #[cfg(not(target_arch = "wasm32"))]
    type Loading = Pending<Result<Track, String>>;
    #[cfg(target_arch = "wasm32")]
    type Loading = macroquad::experimental::coroutines::Coroutine<Result<Track, String>>;
    /// A voice, unless none was free.
    type Playing = Option<VoiceId>;

    #[cfg(not(target_arch = "wasm32"))]
    fn load_sound(
        &mut self,
        bytes: &[u8],
    ) -> impl std::future::Future<Output = Result<Clip, String>> {
        std::future::ready(self.load_effect(bytes))
    }

    #[cfg(target_arch = "wasm32")]
    async fn load_sound(&mut self, bytes: &[u8]) -> Result<Clip, String> {
        web::vetted(bytes).await?;
        self.load_effect(bytes)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_music(&mut self, path: &str) -> Self::Loading {
        let path = path.to_owned();
        Pending::spawn("music-load", move || {
            Track::new(std::fs::read(&path).map_err(|e| e.to_string())?)
        })
    }

    #[cfg(target_arch = "wasm32")]
    fn load_music(&mut self, path: &str) -> Self::Loading {
        let path = path.to_owned();
        macroquad::experimental::coroutines::start_coroutine(async move {
            let bytes = macroquad::file::load_file(&path)
                .await
                .map_err(|e| e.to_string())?;
            web::vetted(&bytes).await?;
            Track::new(bytes)
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn poll_music(&mut self, loading: &mut Self::Loading) -> Option<Result<Clip, String>> {
        self.tidy();
        loading.poll().map(|r| r.flatten().map(clip_of))
    }

    #[cfg(target_arch = "wasm32")]
    fn poll_music(&mut self, loading: &mut Self::Loading) -> Option<Result<Clip, String>> {
        self.tidy();
        loading.retrieve().map(|r| r.map(clip_of))
    }

    fn length(&self, sound: &Clip) -> Option<f64> {
        match sound {
            Clip::Effect { .. } => None,
            Clip::Stream { track, .. } => track.length_ms.map(|ms| f64::from(ms) / 1000.0),
        }
    }

    fn play(&mut self, sound: &Clip, volume: f32, looped: bool) -> Option<VoiceId> {
        self.tidy();
        let params = Params::new()
            .with_volume(level(volume))
            .with_rate(sound.rate().get());
        let played = match sound {
            Clip::Effect { id, voices, .. } => {
                let engine = self.engine();
                engine.seat();
                let params = if looped { params.looped() } else { params };
                let played = engine
                    .remote
                    .play(*id, params)
                    .map_err(|_| "no voice is free: a sound was skipped".to_owned());
                if let Ok(voice) = played {
                    let remote = &mut engine.remote;
                    let mut voices = voices.borrow_mut();
                    voices.retain(|voice| remote.is_playing(*voice));
                    voices.push(voice);
                }
                played
            }
            Clip::Stream { track, voice, .. } => {
                if let Some(old) = voice.take() {
                    self.stop_playing(Some(old));
                }
                let played = self.start_stream(track, params, looped);
                voice.set(played.as_ref().ok().copied());
                played
            }
        };
        match played {
            Ok(voice) => Some(voice),
            Err(problem) => {
                self.say(&problem);
                None
            }
        }
    }

    fn set_volume(&mut self, sound: &Clip, volume: f32) {
        match sound {
            Clip::Effect { voices, .. } => {
                let voices = voices.borrow().clone();
                for voice in voices {
                    self.set_playing_volume(&Some(voice), volume);
                }
            }
            Clip::Stream { voice, .. } => self.set_playing_volume(&voice.get(), volume),
        }
    }

    fn stop(&mut self, sound: &Clip) {
        match sound {
            Clip::Effect { voices, .. } => {
                let voices = voices.take();
                for voice in voices {
                    self.stop_playing(Some(voice));
                }
            }
            Clip::Stream { voice, .. } => self.stop_playing(voice.take()),
        }
        self.tidy();
    }

    fn set_playing_volume(&mut self, playing: &Option<VoiceId>, volume: f32) {
        if let (Some(voice), Some(engine)) = (playing, &mut self.engine) {
            let _ = engine.remote.fade_to(*voice, level(volume), VOLUME_RAMP_MS);
        }
    }

    fn stop_playing(&mut self, playing: Option<VoiceId>) {
        if let (Some(voice), Some(engine)) = (playing, &mut self.engine) {
            let _ = engine.remote.stop(voice);
        }
    }

    fn pause(
        &mut self,
        playing: &Option<VoiceId>,
    ) -> Result<(), shuyi_platform::audio::player::Unsupported> {
        if let (Some(voice), Some(engine)) = (playing, &mut self.engine) {
            let _ = engine.remote.pause(*voice);
        }
        Ok(())
    }

    fn resume(
        &mut self,
        playing: &Option<VoiceId>,
    ) -> Result<(), shuyi_platform::audio::player::Unsupported> {
        if let (Some(voice), Some(engine)) = (playing, &mut self.engine) {
            let _ = engine.remote.resume(*voice);
        }
        Ok(())
    }

    fn set_rate(
        &mut self,
        sound: &Clip,
        rate: u32,
    ) -> Result<(), shuyi_platform::audio::player::Unsupported> {
        sound.rate().set(rate);
        Ok(())
    }

    fn hold_output(&mut self, on: bool) {
        if let Some(output) = self.engine.as_ref().and_then(|e| e.output.as_ref()) {
            if on {
                output.suspend();
            } else {
                output.resume();
            }
        }
    }

    fn warn(&mut self, line: &str) {
        macroquad::logging::warn!("audio: {}", line);
    }
}

/// A track read, as a clip.
fn clip_of(track: Track) -> Clip {
    Clip::Stream {
        track,
        rate: Cell::new(shuyi_sound::mix::RATE_ONE),
        voice: Cell::new(None),
    }
}

/// When a command last reached the mix, for the probe's measure: natively, as the web
/// build has no clock to read it by.
#[cfg(all(feature = "sound-probe", not(target_arch = "wasm32")))]
#[derive(Default)]
pub(crate) struct Reached {
    anchor: std::sync::OnceLock<std::time::Instant>,
    micros: std::sync::atomic::AtomicU64,
}

#[cfg(all(feature = "sound-probe", not(target_arch = "wasm32")))]
impl Reached {
    fn stamp(&self) {
        let anchor = self.anchor.get_or_init(std::time::Instant::now);
        let micros = u64::try_from(anchor.elapsed().as_micros()).unwrap_or(u64::MAX);
        self.micros.store(micros.max(1), Ordering::Release);
    }

    /// The seconds from `started` to the next stamp after it, within a second.
    fn wait_after(&self, started: std::time::Instant) -> Option<f64> {
        loop {
            let micros = self.micros.load(Ordering::Acquire);
            if micros > 0 {
                let anchor = *self.anchor.get()?;
                let at = anchor + std::time::Duration::from_micros(micros);
                if at >= started {
                    return Some((at - started).as_secs_f64());
                }
            }
            if started.elapsed().as_secs() >= 1 {
                return None;
            }
            std::thread::yield_now();
        }
    }
}

/// What goes through the browser's decoder before a library sees it.
#[cfg(target_arch = "wasm32")]
mod web {
    /// Waits for the browser's own decoder's word on `bytes`.
    pub(super) async fn vetted(bytes: &[u8]) -> Result<(), String> {
        let number = shuyi_sound_out::web::vet(bytes);
        loop {
            match shuyi_sound_out::web::vetted(number) {
                Some(true) => return Ok(()),
                Some(false) => return Err("not a readable OGG or WAV file".to_owned()),
                None => macroquad::window::next_frame().await,
            }
        }
    }
}

#[cfg(test)]
mod tests;
