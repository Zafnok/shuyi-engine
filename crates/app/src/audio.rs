//! Plays each frame's audio (ADR-0026): the sound and voice requests and
//! the music commands `shuyi_ui::Game` returns in its `FrameOutput`.
//!
//! Sounds are embedded and decoded once at start-up. Music is loaded from
//! the game's music folder when a track is about to play and freed when it
//! stops, so at most two tracks (the one fading out and the next) are in
//! memory. Voice clips ([`voice`], ADR-0046) are loaded the same way from
//! the game's voice folder. The device calls sit behind [`Backend`], so
//! everything else here is tested without a sound card.
//!
//! It also knows when each track really started ([`Audio::music_playing`],
//! ADR-0037): a track asked for only sounds once its file has loaded, and
//! screens that keep time with the music need the real moment.

#[cfg(not(target_arch = "wasm32"))]
mod native_music;
pub(crate) mod voice;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use shuyi_assets::audio::{AudioManifest, Files};
use shuyi_assets::voice::ClipVariant;
use shuyi_ui::audio::{AudioRequest, MusicCommand};

use self::voice::Voices;

/// What playing audio needs from the platform.
pub(crate) trait Backend {
    /// A decoded sound or track.
    type Sound;
    /// A music track or voice clip being loaded.
    type Loading;

    /// Decodes an embedded sound file.
    async fn load_sound(&mut self, bytes: &[u8]) -> Result<Self::Sound, String>;
    /// Starts loading and decoding the music or voice file at `path`.
    fn load_music(&mut self, path: &str) -> Self::Loading;
    /// The track, once `loading` has finished (called once per frame).
    fn poll_music(&mut self, loading: &mut Self::Loading) -> Option<Result<Self::Sound, String>>;
    /// How long `sound` plays, in seconds, if that is known: it is for an
    /// OGG file loaded with [`load_music`](Self::load_music).
    fn length(&self, sound: &Self::Sound) -> Option<f64>;
    /// Plays `sound` from the start at `volume` (0–1).
    fn play(&mut self, sound: &Self::Sound, volume: f32, looped: bool);
    /// Changes the volume of `sound` wherever it plays.
    fn set_volume(&mut self, sound: &Self::Sound, volume: f32);
    /// Stops `sound`.
    fn stop(&mut self, sound: &Self::Sound);
}

/// The audio player: owns the decoded sounds and the music tracks.
pub(crate) struct Audio<B: Backend> {
    manifest: AudioManifest,
    /// Every sound cue's decoded variants.
    sounds: BTreeMap<String, Vec<B::Sound>>,
    /// Music tracks loading or loaded, by cue.
    tracks: BTreeMap<String, Track<B>>,
    /// Loads stopped before they finished: polled until they do, then
    /// dropped, so a finished decode doesn't stay in memory.
    abandoned: Vec<B::Loading>,
    /// Where music files are, e.g. `music` or `C:/Games/some-game/music`.
    music_dir: String,
    /// The voice clips (ADR-0046).
    voices: Voices<B>,
    rng: VariantRng,
    /// Problems to log (missing files, decode errors).
    warnings: Vec<String>,
    /// The player's music and sound volumes (the Options screen's), 0–1:
    /// multipliers on every cue's own volume.
    music_volume: f32,
    sound_volume: f32,
}

struct Track<B: Backend> {
    state: TrackState<B>,
    /// Asked to play (it starts as soon as it's loaded).
    playing: bool,
    /// When the backend was told to play it, in the seconds of the clock
    /// given to [`Audio::play`]; `None` until then.
    started_at: Option<f64>,
    /// Fade gain, 0–1.
    gain: f32,
    /// The cue's own volume, 0–1.
    volume: f32,
    looped: bool,
}

enum TrackState<B: Backend> {
    Loading(B::Loading),
    Ready(B::Sound),
    Failed,
}

impl<B: Backend> Audio<B> {
    /// Decodes every sound in `manifest` (reading its files through
    /// `files`, the game's embedded sounds). A sound that fails is left out
    /// with a warning. Music tracks are looked for in `music_dir` and voice
    /// clips in `voice_dir`. `seed` seeds the variant picker.
    pub(crate) async fn load(
        backend: &mut B,
        manifest: AudioManifest,
        files: &Files<'_>,
        music_dir: String,
        voice_dir: String,
        seed: u64,
    ) -> Self {
        let mut warnings = Vec::new();
        let mut sounds = BTreeMap::new();
        for (cue, sound) in &manifest.sounds {
            let mut variants = Vec::new();
            for file in &sound.files {
                let decoded = match (files.sound)(file) {
                    Some(data) => backend.load_sound(&data).await,
                    None => Err("not in the asset bundle".to_owned()),
                };
                match decoded {
                    Ok(s) => variants.push(s),
                    Err(e) => {
                        let shown = (files.sound_shown)(file);
                        warnings.push(format!("sound \"{cue}\": {shown}: {e}"));
                    }
                }
            }
            sounds.insert(cue.clone(), variants);
        }
        Self {
            manifest,
            sounds,
            tracks: BTreeMap::new(),
            abandoned: Vec::new(),
            music_dir,
            voices: Voices::new(voice_dir),
            rng: VariantRng::new(seed),
            warnings,
            music_volume: 1.0,
            sound_volume: 1.0,
        }
    }

    /// Sets the player's volumes (the Options screen's, 0–1): every sound
    /// and track plays at its own volume times these. A change to the
    /// music's reaches the tracks already playing.
    pub(crate) fn set_volumes(&mut self, backend: &mut B, music: f32, sound: f32) {
        self.sound_volume = sound;
        if (music - self.music_volume).abs() <= f32::EPSILON {
            return;
        }
        self.music_volume = music;
        for track in self.tracks.values() {
            if let (true, TrackState::Ready(sound)) = (track.playing, &track.state) {
                backend.set_volume(sound, track.volume * track.gain * music);
            }
        }
    }

    /// Plays one frame's sound and voice `requests` (music requests are
    /// skipped: `music` already holds what they mean), applies the `music`
    /// commands and checks on tracks and clips still loading. `now` is the
    /// time in seconds on a clock that keeps running while the game isn't
    /// drawn (ADR-0037): a track that starts in this call started at `now`.
    pub(crate) fn play<V: ClipVariant>(
        &mut self,
        backend: &mut B,
        requests: &[AudioRequest<V>],
        music: &[MusicCommand],
        now: f64,
    ) {
        for request in requests {
            if let AudioRequest::PlaySound { cue, volume } = request {
                self.play_sound(backend, cue, *volume);
            }
            self.voices
                .request(backend, request, now, &mut self.abandoned);
        }
        for command in music {
            self.music(backend, command, now);
        }
        self.poll(backend, now);
    }

    /// The music sounding at `now` (on [`play`](Self::play)'s clock): its
    /// cue and the seconds since it started. `None` in silence, which
    /// includes a track still loading or whose file failed. During a fade
    /// this is the track fading out: the next one starts when the fade
    /// ends. A track played once is still reported after it has ended
    /// (quad-snd doesn't say when it does; `ui` knows its length).
    pub(crate) fn music_playing(&self, now: f64) -> Option<(&str, f64)> {
        self.tracks
            .iter()
            .filter_map(|(cue, track)| Some((cue.as_str(), track.started_at?)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(cue, started_at)| (cue, (now - started_at).max(0.0)))
    }

    fn play_sound(&mut self, backend: &mut B, cue: &str, volume: f32) {
        let (Some(def), Some(variants)) = (self.manifest.sounds.get(cue), self.sounds.get(cue))
        else {
            self.warnings.push(format!("unknown sound cue \"{cue}\""));
            return;
        };
        if variants.is_empty() {
            return; // Its files failed to load (warned then).
        }
        let sound = &variants[self.rng.below(variants.len())];
        let volume = percent(def.volume) * volume * self.sound_volume;
        backend.play(sound, volume, false);
    }

    fn music(&mut self, backend: &mut B, command: &MusicCommand, now: f64) {
        match command {
            MusicCommand::Load { cue } => {
                self.stop_track(backend, cue);
                let Some(def) = self.manifest.music.get(cue) else {
                    self.warnings.push(format!("unknown music cue \"{cue}\""));
                    return;
                };
                let path = format!("{}/{}", self.music_dir, def.file);
                let track = Track {
                    state: TrackState::Loading(backend.load_music(&path)),
                    playing: false,
                    started_at: None,
                    gain: 1.0,
                    volume: percent(def.volume),
                    looped: def.looped,
                };
                self.tracks.insert(cue.clone(), track);
            }
            MusicCommand::Start { cue } => {
                if let Some(track) = self.tracks.get_mut(cue) {
                    track.playing = true;
                    track.gain = 1.0;
                    if let TrackState::Ready(sound) = &track.state {
                        backend.play(sound, track.volume * self.music_volume, track.looped);
                        track.started_at = Some(now);
                    }
                }
            }
            MusicCommand::Gain { cue, gain } => {
                if let Some(track) = self.tracks.get_mut(cue) {
                    track.gain = *gain;
                    if let (true, TrackState::Ready(sound)) = (track.playing, &track.state) {
                        let volume = track.volume * track.gain * self.music_volume;
                        backend.set_volume(sound, volume);
                    }
                }
            }
            MusicCommand::Stop { cue } => self.stop_track(backend, cue),
        }
    }

    /// Stops and frees `cue`'s track, if it has one.
    fn stop_track(&mut self, backend: &mut B, cue: &str) {
        let Some(track) = self.tracks.remove(cue) else {
            return;
        };
        match track.state {
            TrackState::Ready(sound) if track.playing => backend.stop(&sound),
            TrackState::Loading(loading) => self.abandoned.push(loading),
            TrackState::Ready(_) | TrackState::Failed => {}
        }
    }

    /// Stops all music and the voice (the game quit, e.g. on the web page
    /// that stays open).
    pub(crate) fn stop_all(&mut self, backend: &mut B) {
        let cues: Vec<String> = self.tracks.keys().cloned().collect();
        for cue in cues {
            self.stop_track(backend, &cue);
        }
        self.voices.stop_all(backend, &mut self.abandoned);
    }

    /// Sets how loud voice clips play (0–1; `FrameOutput::voice_volume`),
    /// the one playing included.
    pub(crate) fn set_voice_volume(&mut self, backend: &mut B, volume: f32) {
        self.voices.set_volume(backend, volume);
    }

    /// Finishes loads: a track asked to play starts now.
    fn poll(&mut self, backend: &mut B, now: f64) {
        for (cue, track) in &mut self.tracks {
            let TrackState::Loading(loading) = &mut track.state else {
                continue;
            };
            match backend.poll_music(loading) {
                None => {}
                Some(Ok(sound)) => {
                    if track.playing {
                        let volume = track.volume * track.gain * self.music_volume;
                        backend.play(&sound, volume, track.looped);
                        track.started_at = Some(now);
                    }
                    track.state = TrackState::Ready(sound);
                }
                Some(Err(e)) => {
                    let file = self
                        .manifest
                        .music
                        .get(cue)
                        .map_or("?", |m| m.file.as_str());
                    let dir = &self.music_dir;
                    self.warnings
                        .push(format!("music \"{cue}\": {dir}/{file}: {e}"));
                    track.state = TrackState::Failed;
                }
            }
        }
        self.voices
            .poll(backend, now, &mut self.warnings, &mut self.abandoned);
        self.abandoned
            .retain_mut(|loading| backend.poll_music(loading).is_none());
    }

    /// Problems found since the last call, for the log.
    pub(crate) fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }
}

/// A manifest volume (percent) as a 0–1 factor.
fn percent(volume: u8) -> f32 {
    f32::from(volume) / 100.0
}

/// Where a `folder` that ships beside the game is: next to the executable
/// when it's there (a shipped build), else relative to the working
/// directory (`cargo run` from the repo root, or the web build, which
/// fetches `music/…` from beside the page).
pub(crate) fn folder_beside(
    folder: &str,
    exe: Option<&Path>,
    is_dir: impl Fn(&Path) -> bool,
) -> String {
    let beside_exe = exe
        .and_then(Path::parent)
        .map(|dir| dir.join(folder))
        .filter(|dir| is_dir(dir));
    beside_exe
        .as_deref()
        .and_then(Path::to_str)
        .map_or_else(|| folder.to_owned(), |dir| dir.replace('\\', "/"))
}

/// Where a `folder` that ships beside the game is on this platform: beside
/// the executable when it's there ([`folder_beside`]); on the web, beside
/// the page.
pub(crate) fn platform_folder(folder: &str) -> String {
    if cfg!(target_arch = "wasm32") {
        folder.to_owned()
    } else {
        let exe: Option<PathBuf> = std::env::current_exe().ok();
        folder_beside(folder, exe.as_deref(), Path::is_dir)
    }
}

/// Picks sound variants (`audio.md`: footsteps vary each step). A small
/// `SplitMix64`, separate from core's simulation RNG (ADR-0019): what
/// plays never affects the game.
pub(crate) struct VariantRng(u64);

impl VariantRng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number below `n` (`n` ≥ 1).
    pub(crate) fn below(&mut self, n: usize) -> usize {
        let n = u64::try_from(n.max(1)).unwrap_or(u64::MAX);
        usize::try_from(self.next_u64() % n).unwrap_or(0)
    }
}

/// The real backend: macroquad's audio (quad-snd). On native, music is
/// decoded on a worker thread into its own quad-snd context
/// ([`native_music`], ADR-0028); on the web the browser decodes it
/// asynchronously, so it goes through macroquad like the sounds.
pub(crate) mod device {
    use macroquad::audio::{
        PlaySoundParams, Sound, load_sound_from_bytes, play_sound, set_sound_volume, stop_sound,
    };
    #[cfg(target_arch = "wasm32")]
    use macroquad::experimental::coroutines::{Coroutine, start_coroutine};
    #[cfg(target_arch = "wasm32")]
    use macroquad::file::load_file;

    #[cfg(not(target_arch = "wasm32"))]
    use super::native_music::{Contexts, Music, Pending};

    /// Plays through macroquad. Only usable inside its main loop.
    #[derive(Default)]
    pub(crate) struct Macroquad {
        /// Spare music contexts.
        #[cfg(not(target_arch = "wasm32"))]
        contexts: Contexts,
    }

    /// A decoded sound, track or voice clip.
    pub(crate) enum Clip {
        /// In macroquad's own context: every sound, and (with its length
        /// in milliseconds, if it is an OGG file) music and voices on the
        /// web.
        Shared(Sound, Option<u32>),
        /// A native music track or voice clip in a context of its own.
        #[cfg(not(target_arch = "wasm32"))]
        Music(Music),
    }

    impl super::Backend for Macroquad {
        type Sound = Clip;
        #[cfg(target_arch = "wasm32")]
        type Loading = Coroutine<Result<(Sound, Option<u32>), String>>;
        #[cfg(not(target_arch = "wasm32"))]
        type Loading = Pending<Result<Music, String>>;

        async fn load_sound(&mut self, bytes: &[u8]) -> Result<Clip, String> {
            load_sound_from_bytes(bytes)
                .await
                .map(|sound| Clip::Shared(sound, None))
                .map_err(|e| e.to_string())
        }

        #[cfg(target_arch = "wasm32")]
        fn load_music(&mut self, path: &str) -> Self::Loading {
            let path = path.to_owned();
            start_coroutine(async move {
                let bytes = load_file(&path).await.map_err(|e| e.to_string())?;
                let length = shuyi_assets::audio::ogg_length_ms(&bytes);
                load_sound_from_bytes(&bytes)
                    .await
                    .map(|sound| (sound, length))
                    .map_err(|e| e.to_string())
            })
        }

        #[cfg(not(target_arch = "wasm32"))]
        fn load_music(&mut self, path: &str) -> Self::Loading {
            let path = path.to_owned();
            let contexts = self.contexts.clone();
            Pending::spawn("music-load", move || Music::load(&path, contexts))
        }

        #[cfg(target_arch = "wasm32")]
        fn poll_music(&mut self, loading: &mut Self::Loading) -> Option<Result<Clip, String>> {
            loading
                .retrieve()
                .map(|r| r.map(|(sound, length)| Clip::Shared(sound, length)))
        }

        #[cfg(not(target_arch = "wasm32"))]
        fn poll_music(&mut self, loading: &mut Self::Loading) -> Option<Result<Clip, String>> {
            loading.poll().map(|r| r.flatten().map(Clip::Music))
        }

        fn length(&self, sound: &Clip) -> Option<f64> {
            let ms = match sound {
                Clip::Shared(_, length) => *length,
                #[cfg(not(target_arch = "wasm32"))]
                Clip::Music(m) => m.length_ms(),
            };
            ms.map(|ms| f64::from(ms) / 1000.0)
        }

        fn play(&mut self, sound: &Clip, volume: f32, looped: bool) {
            match sound {
                Clip::Shared(s, _) => play_sound(s, PlaySoundParams { looped, volume }),
                #[cfg(not(target_arch = "wasm32"))]
                Clip::Music(m) => m.play(volume, looped),
            }
        }

        fn set_volume(&mut self, sound: &Clip, volume: f32) {
            match sound {
                Clip::Shared(s, _) => set_sound_volume(s, volume),
                #[cfg(not(target_arch = "wasm32"))]
                Clip::Music(m) => m.set_volume(volume),
            }
        }

        fn stop(&mut self, sound: &Clip) {
            match sound {
                Clip::Shared(s, _) => stop_sound(s),
                #[cfg(not(target_arch = "wasm32"))]
                Clip::Music(m) => m.stop(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::future::Future;
    use std::pin::pin;
    use std::rc::Rc;
    use std::task::{Context, Poll, Waker};

    use std::borrow::Cow;

    use proptest::prelude::*;
    use serde::Deserialize;
    use shuyi_assets::audio::{Bytes, CreditRef, MusicCue, SoundCue};

    use super::*;

    /// Which recording of a line a test clip is: its only one, or a
    /// second take.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize)]
    pub(crate) enum Take {
        /// The line's only clip.
        #[default]
        Only,
        /// A second take.
        Second,
    }

    impl ClipVariant for Take {
        fn suffix(self) -> &'static str {
            match self {
                Self::Only => "",
                Self::Second => ".2",
            }
        }

        fn split() -> &'static [Self] {
            &[Self::Second]
        }
    }

    /// The audio requests of the tests.
    pub(crate) type Request = AudioRequest<Take>;

    /// Runs a future that never waits on anything external.
    pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(v) = future.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }

    /// A mono 16-bit WAV of `samples` of a square wave, made here so no
    /// test file is shipped.
    fn wav(samples: u16) -> Vec<u8> {
        let data_len = u32::from(samples) * 2;
        let mut w = Vec::new();
        w.extend_from_slice(b"RIFF");
        w.extend_from_slice(&(36 + data_len).to_le_bytes());
        w.extend_from_slice(b"WAVEfmt ");
        w.extend_from_slice(&16u32.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes()); // PCM
        w.extend_from_slice(&1u16.to_le_bytes()); // mono
        w.extend_from_slice(&22_050u32.to_le_bytes());
        w.extend_from_slice(&44_100u32.to_le_bytes());
        w.extend_from_slice(&2u16.to_le_bytes());
        w.extend_from_slice(&16u16.to_le_bytes());
        w.extend_from_slice(b"data");
        w.extend_from_slice(&data_len.to_le_bytes());
        for i in 0..samples {
            let s: i16 = if i % 50 < 25 { 8_000 } else { -8_000 };
            w.extend_from_slice(&s.to_le_bytes());
        }
        w
    }

    /// Reads a PCM WAV's sample count, the way a decoder would refuse
    /// anything else.
    fn decode_wav(bytes: &[u8]) -> Result<usize, String> {
        let field = |at: usize| {
            bytes
                .get(at..at + 4)
                .and_then(|b| <[u8; 4]>::try_from(b).ok())
        };
        let ok =
            field(0) == Some(*b"RIFF") && field(8) == Some(*b"WAVE") && field(36) == Some(*b"data");
        let len = field(40).map(u32::from_le_bytes).unwrap_or_default() as usize;
        if ok && bytes.len() == 44 + len {
            Ok(len / 2)
        } else {
            Err("not a PCM WAV".to_owned())
        }
    }

    pub(crate) type Log = Rc<RefCell<Vec<String>>>;

    /// Records every call. Sounds are names; music and voice loads finish
    /// when their path is put in `ready` (or fail if in `broken`). A file
    /// plays for `length` seconds, if set.
    #[derive(Default)]
    pub(crate) struct Fake {
        pub(crate) log: Log,
        pub(crate) ready: Vec<String>,
        pub(crate) broken: Vec<String>,
        pub(crate) length: Option<f64>,
    }

    impl Backend for Fake {
        type Sound = String;
        type Loading = String;

        fn load_sound(&mut self, bytes: &[u8]) -> impl Future<Output = Result<String, String>> {
            std::future::ready(decode_wav(bytes).map(|samples| format!("wav{samples}")))
        }

        fn load_music(&mut self, path: &str) -> String {
            self.log.borrow_mut().push(format!("load {path}"));
            path.to_owned()
        }

        fn poll_music(&mut self, loading: &mut String) -> Option<Result<String, String>> {
            if self.broken.contains(loading) {
                Some(Err("no such file".into()))
            } else if self.ready.contains(loading) {
                Some(Ok(loading.clone()))
            } else {
                None
            }
        }

        fn length(&self, _sound: &String) -> Option<f64> {
            self.length
        }

        fn play(&mut self, sound: &String, volume: f32, looped: bool) {
            let looped = if looped { " looped" } else { "" };
            self.log
                .borrow_mut()
                .push(format!("play {sound} {volume:.2}{looped}"));
        }

        fn set_volume(&mut self, sound: &String, volume: f32) {
            self.log
                .borrow_mut()
                .push(format!("volume {sound} {volume:.2}"));
        }

        fn stop(&mut self, sound: &String) {
            self.log.borrow_mut().push(format!("stop {sound}"));
        }
    }

    impl Fake {
        pub(crate) fn calls(&self) -> Vec<String> {
            std::mem::take(&mut self.log.borrow_mut())
        }
    }

    fn manifest() -> AudioManifest {
        let mut m = AudioManifest::default();
        let sound = |files: &[&str], volume| SoundCue {
            files: files.iter().map(|f| (*f).to_owned()).collect(),
            volume,
            credit: CreditRef::Own,
        };
        m.sounds.insert("beep".into(), sound(&["sfx/beep.wav"], 50));
        let steps = ["sfx/step_1.wav", "sfx/step_2.wav", "sfx/step_3.wav"];
        m.sounds.insert("step".into(), sound(&steps, 100));
        m.sounds
            .insert("broken".into(), sound(&["sfx/broken.wav"], 100));
        let track = |file: &str, volume, looped| MusicCue {
            file: file.into(),
            volume,
            looped,
            length_ms: 60_000,
            credit: CreditRef::Own,
        };
        m.music.insert("title".into(), track("title.ogg", 80, true));
        m.music
            .insert("sting".into(), track("sting.ogg", 100, false));
        m.music.insert("lost".into(), track("lost.ogg", 100, true));
        m
    }

    /// The generated WAV for every sound but `broken`, which isn't a WAV.
    fn bytes(file: &str) -> Bytes {
        thread_local! {
            static WAVS: (&'static [u8], &'static [u8], &'static [u8], &'static [u8]) = (
                Vec::leak(wav(100)),
                Vec::leak(wav(200)),
                Vec::leak(wav(300)),
                Vec::leak(wav(400)),
            );
        }
        let wav = WAVS.with(|w| match file {
            "sfx/beep.wav" => Some(w.0),
            "sfx/step_1.wav" => Some(w.1),
            "sfx/step_2.wav" => Some(w.2),
            "sfx/step_3.wav" => Some(w.3),
            "sfx/broken.wav" => Some(&w.0[..20]),
            _ => None,
        });
        wav.map(Cow::Borrowed)
    }

    /// How the warnings name a sound file.
    fn shown(file: &str) -> String {
        format!("bundle/{file}")
    }

    /// The sounds of [`bytes`], named by [`shown`].
    pub(crate) fn files() -> Files<'static> {
        Files {
            sound: &bytes,
            sound_shown: &shown,
            sound_left_out: &|_: &str| false,
            music_dir: "music",
            music: None,
        }
    }

    pub(crate) fn audio(backend: &mut Fake) -> Audio<Fake> {
        let (music, voice) = ("music".into(), "voice/en".into());
        block_on(Audio::load(backend, manifest(), &files(), music, voice, 7))
    }

    fn sound(cue: &str, volume: f32) -> Request {
        Request::PlaySound {
            cue: cue.into(),
            volume,
        }
    }

    fn cmd(f: fn(String) -> MusicCommand, cue: &str) -> MusicCommand {
        f(cue.into())
    }

    fn load(cue: String) -> MusicCommand {
        MusicCommand::Load { cue }
    }

    fn start(cue: String) -> MusicCommand {
        MusicCommand::Start { cue }
    }

    fn stop(cue: String) -> MusicCommand {
        MusicCommand::Stop { cue }
    }

    fn gain(cue: &str, gain: f32) -> MusicCommand {
        MusicCommand::Gain {
            cue: cue.into(),
            gain,
        }
    }

    #[test]
    fn a_generated_wav_plays_through_the_app_path() {
        let mut fake = Fake::default();
        let mut audio = audio(&mut fake);
        assert_eq!(
            audio.take_warnings(),
            ["sound \"broken\": bundle/sfx/broken.wav: not a PCM WAV"]
        );
        let music_request = Request::PlayMusic {
            cue: "title".into(),
        };
        audio.play(
            &mut fake,
            &[sound("beep", 1.0), music_request, sound("beep", 0.5)],
            &[],
            0.0,
        );
        assert_eq!(fake.calls(), ["play wav100 0.50", "play wav100 0.25"]);
        // A sound whose file failed to decode is silent, not a crash.
        audio.play(&mut fake, &[sound("broken", 1.0)], &[], 0.0);
        assert!(fake.calls().is_empty());
        assert!(audio.take_warnings().is_empty());
    }

    #[test]
    fn unknown_or_missing_files_are_warnings() {
        let mut fake = Fake::default();
        let mut m = manifest();
        if let Some(beep) = m.sounds.get_mut("beep") {
            beep.files.push("sfx/gone.wav".into());
        }
        let dirs = ("music".into(), "voice/en".into());
        let mut audio = block_on(Audio::load(&mut fake, m, &files(), dirs.0, dirs.1, 1));
        assert_eq!(
            audio.take_warnings()[0],
            "sound \"beep\": bundle/sfx/gone.wav: not in the asset bundle"
        );
        audio.play(&mut fake, &[sound("nope", 1.0)], &[cmd(load, "nope")], 0.0);
        assert_eq!(
            audio.take_warnings(),
            ["unknown sound cue \"nope\"", "unknown music cue \"nope\""]
        );
        assert!(fake.calls().is_empty());
    }

    #[test]
    fn variants_are_picked_at_random() {
        let mut fake = Fake::default();
        let mut audio = audio(&mut fake);
        let steps: Vec<_> = (0..30).map(|_| sound("step", 1.0)).collect();
        audio.play(&mut fake, &steps, &[], 0.0);
        let calls = fake.calls();
        assert_eq!(calls.len(), 30);
        for variant in ["wav200", "wav300", "wav400"] {
            assert!(
                calls.iter().any(|c| c.contains(variant)),
                "{variant} never played: {calls:?}"
            );
        }
    }

    #[test]
    fn music_loads_then_starts_when_ready() {
        let mut fake = Fake::default();
        let mut audio = audio(&mut fake);
        audio.play::<Take>(
            &mut fake,
            &[],
            &[cmd(load, "title"), cmd(start, "title")],
            0.0,
        );
        assert_eq!(fake.calls(), ["load music/title.ogg"]);
        audio.play::<Take>(&mut fake, &[], &[], 0.0);
        assert!(fake.calls().is_empty(), "still loading");
        fake.ready.push("music/title.ogg".into());
        audio.play::<Take>(&mut fake, &[], &[], 0.0);
        assert_eq!(fake.calls(), ["play music/title.ogg 0.80 looped"]);
        audio.play::<Take>(&mut fake, &[], &[], 0.0);
        assert!(fake.calls().is_empty(), "started once");
        // A fade scales the cue's own volume; then stop frees it.
        audio.play::<Take>(&mut fake, &[], &[gain("title", 0.5)], 0.0);
        assert_eq!(fake.calls(), ["volume music/title.ogg 0.40"]);
        audio.play::<Take>(&mut fake, &[], &[cmd(stop, "title")], 0.0);
        assert_eq!(fake.calls(), ["stop music/title.ogg"]);
        assert!(audio.tracks.is_empty());
    }

    /// Ticket 0805: the player's volumes multiply every cue's own.
    #[test]
    fn the_players_volumes_scale_sounds_and_music() {
        let mut fake = Fake::default();
        fake.ready.push("music/title.ogg".into());
        let mut audio = audio(&mut fake);
        audio.set_volumes(&mut fake, 0.5, 0.2);
        assert!(fake.calls().is_empty(), "nothing is playing yet");
        // Sounds: the cue's 50 %, the request's volume, the player's 20 %.
        audio.play(
            &mut fake,
            &[sound("beep", 1.0), sound("beep", 0.5)],
            &[],
            0.0,
        );
        assert_eq!(fake.calls(), ["play wav100 0.10", "play wav100 0.05"]);
        // Music starts at the cue's 80 % times the player's 50 %.
        let begin = [cmd(load, "title"), cmd(start, "title")];
        audio.play::<Take>(&mut fake, &[], &begin, 0.0);
        assert_eq!(
            fake.calls(),
            ["load music/title.ogg", "play music/title.ogg 0.40 looped"]
        );
        // The same volumes again change nothing; a new music volume
        // reaches the track playing, and a fade multiplies with it.
        audio.set_volumes(&mut fake, 0.5, 0.2);
        assert!(fake.calls().is_empty());
        audio.set_volumes(&mut fake, 1.0, 0.2);
        assert_eq!(fake.calls(), ["volume music/title.ogg 0.80"]);
        audio.play::<Take>(&mut fake, &[], &[gain("title", 0.5)], 0.0);
        assert_eq!(fake.calls(), ["volume music/title.ogg 0.40"]);
        audio.set_volumes(&mut fake, 0.5, 0.2);
        assert_eq!(fake.calls(), ["volume music/title.ogg 0.20"]);
        // Silent: sounds and music at 0.
        audio.set_volumes(&mut fake, 0.0, 0.0);
        assert_eq!(fake.calls(), ["volume music/title.ogg 0.00"]);
        audio.play(&mut fake, &[sound("beep", 1.0)], &[], 0.0);
        assert_eq!(fake.calls(), ["play wav100 0.00"]);
    }

    /// A track already loaded and started later, or still loading when the
    /// volume is set, starts at the player's volume.
    #[test]
    fn a_track_that_starts_later_uses_the_players_volume() {
        let mut fake = Fake::default();
        let mut audio = audio(&mut fake);
        let begin = [cmd(load, "title"), cmd(start, "title")];
        audio.play::<Take>(&mut fake, &[], &begin, 0.0);
        fake.calls();
        // Still loading: nothing to change yet.
        audio.set_volumes(&mut fake, 0.25, 1.0);
        assert!(fake.calls().is_empty());
        fake.ready.push("music/title.ogg".into());
        audio.play::<Take>(&mut fake, &[], &[], 0.0);
        assert_eq!(fake.calls(), ["play music/title.ogg 0.20 looped"]);
        // Started again once loaded (ADR-0026: `Start` on a ready track).
        audio.play::<Take>(&mut fake, &[], &[cmd(start, "title")], 0.0);
        assert_eq!(fake.calls(), ["play music/title.ogg 0.20 looped"]);
    }

    #[test]
    fn a_loaded_track_starts_at_once_and_plays_once_if_not_looped() {
        let mut fake = Fake::default();
        fake.ready.push("music/sting.ogg".into());
        let mut audio = audio(&mut fake);
        audio.play::<Take>(&mut fake, &[], &[cmd(load, "sting")], 0.0);
        assert_eq!(fake.calls(), ["load music/sting.ogg"]);
        audio.play::<Take>(
            &mut fake,
            &[],
            &[gain("sting", 0.5), cmd(start, "sting")],
            0.0,
        );
        assert_eq!(fake.calls(), ["play music/sting.ogg 1.00"]);
    }

    #[test]
    fn a_gain_before_the_track_is_ready_applies_when_it_starts() {
        let mut fake = Fake::default();
        let mut audio = audio(&mut fake);
        audio.play::<Take>(
            &mut fake,
            &[],
            &[cmd(load, "title"), cmd(start, "title")],
            0.0,
        );
        audio.play::<Take>(&mut fake, &[], &[gain("title", 0.5)], 0.0);
        fake.ready.push("music/title.ogg".into());
        audio.play::<Take>(&mut fake, &[], &[], 0.0);
        assert_eq!(
            fake.calls(),
            ["load music/title.ogg", "play music/title.ogg 0.40 looped"]
        );
    }

    #[test]
    fn a_stopped_load_is_polled_until_done_then_dropped() {
        let mut fake = Fake::default();
        let mut audio = audio(&mut fake);
        audio.play::<Take>(
            &mut fake,
            &[],
            &[cmd(load, "title"), cmd(stop, "title")],
            0.0,
        );
        assert_eq!(audio.abandoned.len(), 1);
        audio.play::<Take>(&mut fake, &[], &[], 0.0);
        assert_eq!(audio.abandoned.len(), 1);
        fake.ready.push("music/title.ogg".into());
        audio.play::<Take>(&mut fake, &[], &[], 0.0);
        assert!(audio.abandoned.is_empty());
        assert_eq!(fake.calls(), ["load music/title.ogg"], "never played");
    }

    #[test]
    fn a_missing_track_is_a_warning_and_silence() {
        let mut fake = Fake::default();
        fake.broken.push("music/lost.ogg".into());
        let mut audio = audio(&mut fake);
        audio.take_warnings();
        audio.play::<Take>(
            &mut fake,
            &[],
            &[cmd(load, "lost"), cmd(start, "lost")],
            0.0,
        );
        assert_eq!(
            audio.take_warnings(),
            ["music \"lost\": music/lost.ogg: no such file"]
        );
        audio.play::<Take>(&mut fake, &[], &[gain("lost", 0.5), cmd(stop, "lost")], 0.0);
        assert_eq!(fake.calls(), ["load music/lost.ogg"]);
    }

    #[test]
    fn loading_again_replaces_the_track() {
        let mut fake = Fake::default();
        fake.ready.push("music/title.ogg".into());
        let mut audio = audio(&mut fake);
        audio.play::<Take>(
            &mut fake,
            &[],
            &[cmd(load, "title"), cmd(start, "title")],
            0.0,
        );
        audio.play::<Take>(&mut fake, &[], &[], 0.0);
        audio.play::<Take>(&mut fake, &[], &[cmd(load, "title")], 0.0);
        assert_eq!(
            fake.calls(),
            [
                "load music/title.ogg",
                "play music/title.ogg 0.80 looped",
                "stop music/title.ogg",
                "load music/title.ogg",
            ]
        );
    }

    #[test]
    fn a_ready_track_not_started_is_freed_silently() {
        let mut fake = Fake::default();
        fake.ready.push("music/title.ogg".into());
        let mut audio = audio(&mut fake);
        audio.play::<Take>(&mut fake, &[], &[cmd(load, "title")], 0.0);
        audio.play::<Take>(
            &mut fake,
            &[],
            &[gain("title", 0.5), cmd(stop, "title")],
            0.0,
        );
        assert_eq!(fake.calls(), ["load music/title.ogg"]);
        assert!(audio.tracks.is_empty());
        // Commands for tracks that aren't there do nothing.
        audio.play::<Take>(
            &mut fake,
            &[],
            &[cmd(start, "title"), gain("title", 1.0)],
            0.0,
        );
        assert!(fake.calls().is_empty());
    }

    #[test]
    fn stop_all_silences_every_track() {
        let mut fake = Fake::default();
        fake.ready.push("music/title.ogg".into());
        let mut audio = audio(&mut fake);
        let cmds = [cmd(load, "title"), cmd(start, "title"), cmd(load, "sting")];
        audio.play::<Take>(&mut fake, &[], &cmds, 0.0);
        audio.stop_all(&mut fake);
        assert!(audio.tracks.is_empty());
        assert_eq!(
            fake.calls(),
            [
                "load music/title.ogg",
                "load music/sting.ogg",
                "play music/title.ogg 0.80 looped",
                "stop music/title.ogg",
            ]
        );
        assert_eq!(audio.abandoned.len(), 1);
    }

    /// Ticket 0227: nothing is reported while the track loads; from the
    /// moment it starts, the seconds since; nothing after its stop.
    #[test]
    fn the_music_playing_is_reported_from_its_real_start() {
        let mut fake = Fake::default();
        let mut audio = audio(&mut fake);
        assert_eq!(audio.music_playing(10.0), None);
        let cmds = [cmd(load, "title"), cmd(start, "title")];
        audio.play::<Take>(&mut fake, &[], &cmds, 10.0);
        audio.play::<Take>(&mut fake, &[], &[], 11.0);
        assert_eq!(audio.music_playing(11.5), None, "still loading");
        fake.ready.push("music/title.ogg".into());
        assert_eq!(audio.music_playing(12.0), None, "loaded, not yet started");
        audio.play::<Take>(&mut fake, &[], &[], 12.5);
        assert_eq!(audio.music_playing(12.5), Some(("title", 0.0)));
        assert_eq!(audio.music_playing(14.0), Some(("title", 1.5)));
        // A fade doesn't move it, and it keeps counting past the loop
        // (`ui` wraps it at the track's length).
        audio.play::<Take>(&mut fake, &[], &[gain("title", 0.5)], 15.0);
        assert_eq!(audio.music_playing(100.0), Some(("title", 87.5)));
        // A clock that stepped back gives 0, not a negative time.
        assert_eq!(audio.music_playing(12.0), Some(("title", 0.0)));
        audio.play::<Take>(&mut fake, &[], &[cmd(stop, "title")], 101.0);
        assert_eq!(audio.music_playing(101.0), None);
    }

    #[test]
    fn a_loaded_track_is_reported_from_its_start_command() {
        let mut fake = Fake::default();
        fake.ready.push("music/sting.ogg".into());
        let mut audio = audio(&mut fake);
        audio.play::<Take>(&mut fake, &[], &[cmd(load, "sting")], 1.0);
        assert_eq!(audio.music_playing(2.0), None, "ready, not asked to play");
        audio.play::<Take>(&mut fake, &[], &[cmd(start, "sting")], 3.0);
        assert_eq!(audio.music_playing(3.25), Some(("sting", 0.25)));
        // Loading it again replaces the track: silence until it restarts.
        audio.play::<Take>(&mut fake, &[], &[cmd(load, "sting")], 4.0);
        assert_eq!(audio.music_playing(4.5), None);
    }

    #[test]
    fn a_track_that_fails_to_load_is_never_reported() {
        let mut fake = Fake::default();
        fake.broken.push("music/lost.ogg".into());
        let mut audio = audio(&mut fake);
        let cmds = [cmd(load, "lost"), cmd(start, "lost")];
        audio.play::<Take>(&mut fake, &[], &cmds, 1.0);
        audio.play::<Take>(&mut fake, &[], &[], 2.0);
        assert_eq!(audio.music_playing(3.0), None);
    }

    /// As `MusicState` switches tracks: the new one loads during the fade,
    /// then the old one stops and the new one starts.
    #[test]
    fn after_a_switch_the_report_follows_the_new_track_once_it_starts() {
        let mut fake = Fake::default();
        fake.ready.push("music/title.ogg".into());
        let mut audio = audio(&mut fake);
        let cmds = [cmd(load, "title"), cmd(start, "title")];
        audio.play::<Take>(&mut fake, &[], &cmds, 1.0);
        let fading = [cmd(load, "sting"), gain("title", 0.5)];
        audio.play::<Take>(&mut fake, &[], &fading, 5.0);
        assert_eq!(audio.music_playing(5.25), Some(("title", 4.25)));
        // The fade ends before the new track has loaded: silence.
        let switch = [cmd(stop, "title"), cmd(start, "sting")];
        audio.play::<Take>(&mut fake, &[], &switch, 5.5);
        assert_eq!(audio.music_playing(6.0), None);
        fake.ready.push("music/sting.ogg".into());
        audio.play::<Take>(&mut fake, &[], &[], 7.0);
        assert_eq!(audio.music_playing(7.5), Some(("sting", 0.5)));
    }

    /// Should two tracks ever sound at once, the one that started last is
    /// reported, whichever its cue.
    #[test]
    fn of_two_tracks_the_one_started_last_is_reported() {
        for (first, second) in [("sting", "title"), ("title", "sting")] {
            let mut fake = Fake::default();
            fake.ready.push("music/title.ogg".into());
            fake.ready.push("music/sting.ogg".into());
            let mut audio = audio(&mut fake);
            let cmds = [cmd(load, first), cmd(load, second)];
            audio.play::<Take>(&mut fake, &[], &cmds, 0.0);
            audio.play::<Take>(&mut fake, &[], &[cmd(start, first)], 1.0);
            audio.play::<Take>(&mut fake, &[], &[cmd(start, second)], 2.0);
            assert_eq!(audio.music_playing(2.5), Some((second, 0.5)));
        }
    }

    #[test]
    fn the_music_folder_is_beside_the_exe_when_there() {
        let exe = Path::new("/games/some-game/some-game.exe");
        let music = |exe, is_dir: fn(&Path) -> bool| folder_beside("music", exe, is_dir);
        assert_eq!(music(Some(exe), |_| true), "/games/some-game/music");
        assert_eq!(music(Some(exe), |_| false), "music");
        assert_eq!(music(None, |_| true), "music");
        let windows = Path::new(r"C:\games\some-game\some-game.exe");
        let found = music(Some(windows), |d| d.ends_with("music"));
        assert!(!found.contains('\\'), "{found}");
        assert!(found.ends_with("music"), "{found}");
        let exe = std::env::current_exe().ok();
        assert_eq!(
            platform_folder("music"),
            folder_beside("music", exe.as_deref(), Path::is_dir)
        );
    }

    #[test]
    fn a_folder_beside_the_game_is_found_by_its_name() {
        let exe = Path::new("/games/some-game/some-game.exe");
        assert_eq!(
            folder_beside("voice", Some(exe), |_| true),
            "/games/some-game/voice"
        );
        assert_eq!(folder_beside("voice", Some(exe), |_| false), "voice");
        assert_eq!(folder_beside("voice", None, |_| true), "voice");
        let only_voice = |d: &Path| d.ends_with("voice");
        assert_eq!(folder_beside("music", Some(exe), only_voice), "music");
    }

    #[test]
    fn percent_is_a_factor() {
        assert!((percent(100) - 1.0).abs() < f32::EPSILON);
        assert!((percent(60) - 0.6).abs() < f32::EPSILON);
        assert!(percent(0).abs() < f32::EPSILON);
    }

    #[test]
    fn the_rng_is_deterministic_per_seed() {
        let draws = |seed| {
            let mut rng = VariantRng::new(seed);
            (0..8).map(|_| rng.next_u64()).collect::<Vec<_>>()
        };
        assert_eq!(draws(1), draws(1));
        assert_ne!(draws(1), draws(2));
        // SplitMix64's published first output for seed 0.
        assert_eq!(VariantRng::new(0).next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(VariantRng::new(5).below(0), 0);
        assert_eq!(VariantRng::new(5).below(1), 0);
    }

    proptest! {
        /// Every variant gets its chance, whatever the seed.
        #[test]
        fn every_variant_gets_picked(seed: u64, n in 1usize..=6) {
            let mut rng = VariantRng::new(seed);
            let mut seen = vec![false; n];
            for _ in 0..200 {
                let i = rng.below(n);
                prop_assert!(i < n);
                seen[i] = true;
            }
            prop_assert!(seen.iter().all(|&s| s), "{seen:?}");
        }
    }
}
