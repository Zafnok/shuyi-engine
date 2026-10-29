//! Plays each frame's audio (game ADR-0026): the sound and voice requests and
//! the music commands `shuyi_platform::App` hands the sound device
//! ([`AudioFrame`](shuyi_platform::AudioFrame)).
//!
//! Sounds are embedded and decoded once at start-up. Music is loaded from
//! the game's music folder when a track is about to play and freed when it
//! stops, so at most two tracks (the one fading out and the next) are in
//! memory. In a crossfade the two play at once, their gains moving
//! opposite ways (ADR-0054). Voice clips ([`voice`], game ADR-0046) are loaded the same way from
//! the game's voice folder. The device calls sit behind [`Backend`], so
//! everything else here is tested without a sound card.
//!
//! A sound started with a handle (`SoundRequest::Start`) is one playback
//! the player keeps by its id, so that one copy stops alone and its volume
//! follows the requests and the player's sound volume.
//!
//! quad-snd can't pause (ADR-0056): a sound paused under a menu
//! (`SoundRequest::PauseSounds`) is muted, and so is everything sounding
//! while the system suspends the game ([`Audio::suspend`]: the sounds
//! started with a handle, the voice and the music). A muted sound runs on
//! silently, so a track's clock runs too, and comes back at the volume it
//! would have had.
//!
//! It also knows when each track really started ([`Audio::music_playing`],
//! game ADR-0037): a track asked for only sounds once its file has loaded, and
//! screens that keep time with the music need the real moment.

#[cfg(not(target_arch = "wasm32"))]
mod native_music;
pub(crate) mod voice;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use shuyi_assets::audio::{AudioManifest, Files};
use shuyi_platform::SoundRequest;
use shuyi_ui::audio::{MusicCommand, SoundId};

use self::voice::Voices;

/// What playing audio needs from the platform.
pub(crate) trait Backend {
    /// A decoded sound or track.
    type Sound;
    /// A music track or voice clip being loaded.
    type Loading;
    /// One playback of a sound, which stops alone.
    type Playing;

    /// Decodes an embedded sound file.
    async fn load_sound(&mut self, bytes: &[u8]) -> Result<Self::Sound, String>;
    /// Starts loading and decoding the music or voice file at `path`.
    fn load_music(&mut self, path: &str) -> Self::Loading;
    /// The track, once `loading` has finished (called once per frame).
    fn poll_music(&mut self, loading: &mut Self::Loading) -> Option<Result<Self::Sound, String>>;
    /// How long `sound` plays, in seconds, if that is known: it is for an
    /// OGG file loaded with [`load_music`](Self::load_music).
    fn length(&self, sound: &Self::Sound) -> Option<f64>;
    /// Plays `sound` from the start at `volume` (0–1): this playback.
    fn play(&mut self, sound: &Self::Sound, volume: f32, looped: bool) -> Self::Playing;
    /// Changes the volume of `sound` wherever it plays.
    fn set_volume(&mut self, sound: &Self::Sound, volume: f32);
    /// Stops `sound`.
    fn stop(&mut self, sound: &Self::Sound);
    /// Changes the volume of `playing` alone.
    fn set_playing_volume(&mut self, playing: &Self::Playing, volume: f32);
    /// Stops `playing` alone: other copies of its sound play on.
    fn stop_playing(&mut self, playing: Self::Playing);
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
    /// The voice clips (game ADR-0046).
    voices: Voices<B>,
    rng: VariantRng,
    /// Problems to log (missing files, decode errors).
    warnings: Vec<String>,
    /// The player's music and sound volumes (the Options screen's), 0–1:
    /// multipliers on every cue's own volume.
    music_volume: f32,
    sound_volume: f32,
    /// The sounds started with a handle and not stopped.
    handled: BTreeMap<SoundId, Handled<B>>,
    /// The voice volume asked for, 0–1, kept while suspended.
    voice_volume: f32,
    /// Whether the system suspended the game: everything is muted.
    suspended: bool,
}

/// A sound started with a handle: its playback and what its volume is
/// made of.
struct Handled<B: Backend> {
    playing: B::Playing,
    /// The cue's own volume, 0–1.
    cue_volume: f32,
    /// The volume its requests gave, 0–1.
    volume: f32,
    /// Whether `PauseSounds` paused it: it is muted until `ResumeSounds`.
    paused: bool,
}

impl<B: Backend> Handled<B> {
    /// What it plays at, with the player's sound volume `sound`: nothing
    /// while it is paused or the game is `suspended`.
    fn level(&self, sound: f32, suspended: bool) -> f32 {
        if self.paused || suspended {
            0.0
        } else {
            self.cue_volume * self.volume * sound
        }
    }
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
            handled: BTreeMap::new(),
            voice_volume: 1.0,
            suspended: false,
        }
    }

    /// Sets the player's volumes (the Options screen's, 0–1): every sound
    /// and track plays at its own volume times these. A change reaches the
    /// tracks and the sounds started with a handle already playing.
    pub(crate) fn set_volumes(&mut self, backend: &mut B, music: f32, sound: f32) {
        if (sound - self.sound_volume).abs() > f32::EPSILON {
            self.sound_volume = sound;
            self.set_handled_volumes(backend);
        }
        if (music - self.music_volume).abs() <= f32::EPSILON {
            return;
        }
        self.music_volume = music;
        self.set_track_volumes(backend);
    }

    /// The player's music volume as the tracks hear it: nothing while
    /// suspended.
    fn music_level(&self) -> f32 {
        if self.suspended {
            0.0
        } else {
            self.music_volume
        }
    }

    /// Sets every track playing to its volume times its gain and the
    /// music level.
    fn set_track_volumes(&self, backend: &mut B) {
        let music = self.music_level();
        for track in self.tracks.values() {
            if let (true, TrackState::Ready(sound)) = (track.playing, &track.state) {
                backend.set_volume(sound, track.volume * track.gain * music);
            }
        }
    }

    /// Sets every sound started with a handle to its level.
    fn set_handled_volumes(&self, backend: &mut B) {
        for handled in self.handled.values() {
            let level = handled.level(self.sound_volume, self.suspended);
            backend.set_playing_volume(&handled.playing, level);
        }
    }

    /// Mutes everything sounding while the system suspends the game: the
    /// sounds started with a handle, the voice and the music (quad-snd
    /// can't pause). Sounds played once finish. A second call does
    /// nothing.
    pub(crate) fn suspend(&mut self, backend: &mut B) {
        if self.suspended {
            return;
        }
        self.suspended = true;
        self.set_handled_volumes(backend);
        self.set_track_volumes(backend);
        self.voices.set_volume(backend, 0.0);
    }

    /// Brings back what [`suspend`](Self::suspend) muted, at the volumes
    /// asked for meanwhile; a sound paused under a menu stays muted. A
    /// call while not suspended does nothing.
    pub(crate) fn resume(&mut self, backend: &mut B) {
        if !self.suspended {
            return;
        }
        self.suspended = false;
        self.set_handled_volumes(backend);
        self.set_track_volumes(backend);
        self.voices.set_volume(backend, self.voice_volume);
    }

    /// Plays one frame's sound and voice `requests`, applies the `music`
    /// commands and checks on tracks and clips still loading. `now` is the
    /// time in seconds on a clock that keeps running while the game isn't
    /// drawn (game ADR-0037): a track that starts in this call started at `now`.
    pub(crate) fn play(
        &mut self,
        backend: &mut B,
        requests: &[SoundRequest],
        music: &[MusicCommand],
        now: f64,
    ) {
        for request in requests {
            self.sound(backend, request);
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
    /// includes a track still loading or whose file failed. Of two tracks
    /// (a crossfade) it is the newest that sounds: the old one until the
    /// new one has loaded. A track played once is still reported after it has ended
    /// (quad-snd doesn't say when it does; `ui` knows its length).
    pub(crate) fn music_playing(&self, now: f64) -> Option<(&str, f64)> {
        self.tracks
            .iter()
            .filter_map(|(cue, track)| Some((cue.as_str(), track.started_at?)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(cue, started_at)| (cue, (now - started_at).max(0.0)))
    }

    /// Applies one sound request (the voice's are [`Voices`]').
    fn sound(&mut self, backend: &mut B, request: &SoundRequest) {
        match request {
            SoundRequest::Sound { cue, volume } => {
                // A looped cue played this way (a bug the debug build
                // catches) plays once: nothing could stop it.
                self.play_sound(backend, cue, *volume, false);
            }
            SoundRequest::Start {
                id, cue, volume, ..
            } => {
                self.start_sound(backend, *id, cue, *volume);
            }
            SoundRequest::Stop { id } => {
                if let Some(handled) = self.handled.remove(id) {
                    backend.stop_playing(handled.playing);
                }
            }
            SoundRequest::Volume { id, volume } => {
                if let Some(handled) = self.handled.get_mut(id) {
                    handled.volume = *volume;
                    let level = handled.level(self.sound_volume, self.suspended);
                    backend.set_playing_volume(&handled.playing, level);
                }
            }
            SoundRequest::PauseSounds => self.pause_handled(backend, true),
            SoundRequest::ResumeSounds => self.pause_handled(backend, false),
            _ => {}
        }
    }

    /// Plays a variant of sound `cue` at `volume` times its own and the
    /// player's sound volume, looped or not: the playback, unless the cue
    /// is unknown or none of its files loaded.
    fn play_sound(
        &mut self,
        backend: &mut B,
        cue: &str,
        volume: f32,
        looped: bool,
    ) -> Option<B::Playing> {
        let (Some(def), Some(variants)) = (self.manifest.sounds.get(cue), self.sounds.get(cue))
        else {
            self.warnings.push(format!("unknown sound cue \"{cue}\""));
            return None;
        };
        if variants.is_empty() {
            return None; // Its files failed to load (warned then).
        }
        let sound = &variants[self.rng.below(variants.len())];
        let volume = percent(def.volume) * volume * self.sound_volume;
        Some(backend.play(sound, volume, looped))
    }

    /// Pauses (mutes) every sound started with a handle, or resumes the
    /// ones paused.
    fn pause_handled(&mut self, backend: &mut B, paused: bool) {
        for handled in self.handled.values_mut() {
            if handled.paused != paused {
                handled.paused = paused;
                let level = handled.level(self.sound_volume, self.suspended);
                backend.set_playing_volume(&handled.playing, level);
            }
        }
    }

    /// Starts sound `cue` as `id`, looped if its cue says so.
    fn start_sound(&mut self, backend: &mut B, id: SoundId, cue: &str, volume: f32) {
        let def = self.manifest.sounds.get(cue);
        let (looped, cue_volume) = def.map_or((false, 1.0), |d| (d.looped, percent(d.volume)));
        if let Some(playing) = self.play_sound(backend, cue, volume, looped) {
            let handled = Handled {
                playing,
                cue_volume,
                volume,
                paused: false,
            };
            if self.suspended {
                backend.set_playing_volume(&handled.playing, 0.0);
            }
            if let Some(old) = self.handled.insert(id, handled) {
                backend.stop_playing(old.playing);
            }
        }
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
            MusicCommand::Start { cue, gain, .. } => {
                let music = self.music_level();
                if let Some(track) = self.tracks.get_mut(cue) {
                    track.playing = true;
                    track.gain = *gain;
                    if let TrackState::Ready(sound) = &track.state {
                        let volume = track.volume * track.gain * music;
                        backend.play(sound, volume, track.looped);
                        track.started_at = Some(now);
                    }
                }
            }
            MusicCommand::Gain { cue, gain } => {
                let music = self.music_level();
                if let Some(track) = self.tracks.get_mut(cue) {
                    track.gain = *gain;
                    if let (true, TrackState::Ready(sound)) = (track.playing, &track.state) {
                        let volume = track.volume * track.gain * music;
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

    /// Stops all music, the voice and the sounds started with a handle
    /// (the game quit, e.g. on the web page that stays open).
    pub(crate) fn stop_all(&mut self, backend: &mut B) {
        let cues: Vec<String> = self.tracks.keys().cloned().collect();
        for cue in cues {
            self.stop_track(backend, &cue);
        }
        for handled in std::mem::take(&mut self.handled).into_values() {
            backend.stop_playing(handled.playing);
        }
        self.voices.stop_all(backend, &mut self.abandoned);
    }

    /// Sets how loud voice clips play (0–1; `FrameOutput::voice_volume`),
    /// the one playing included.
    pub(crate) fn set_voice_volume(&mut self, backend: &mut B, volume: f32) {
        self.voice_volume = volume;
        if !self.suspended {
            self.voices.set_volume(backend, volume);
        }
    }

    /// Finishes loads: a track asked to play starts now.
    fn poll(&mut self, backend: &mut B, now: f64) {
        let music = self.music_level();
        for (cue, track) in &mut self.tracks {
            let TrackState::Loading(loading) = &mut track.state else {
                continue;
            };
            match backend.poll_music(loading) {
                None => {}
                Some(Ok(sound)) => {
                    if track.playing {
                        let volume = track.volume * track.gain * music;
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
/// `SplitMix64`, separate from core's simulation RNG (game ADR-0019): what
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

/// The real backend: quad-snd, which macroquad's audio is. Sound effects
/// load into a quad-snd context of the player's own, where each playback
/// stops and changes its volume alone. On native, music is decoded on a
/// worker thread into contexts of its own ([`native_music`], game
/// ADR-0028); on the web the browser decodes it asynchronously, so it goes
/// through macroquad's.
pub(crate) mod device {
    #[cfg(target_arch = "wasm32")]
    use macroquad::audio::{
        PlaySoundParams, Sound, load_sound_from_bytes, play_sound, set_sound_volume, stop_sound,
    };
    #[cfg(target_arch = "wasm32")]
    use macroquad::experimental::coroutines::{Coroutine, start_coroutine};
    #[cfg(target_arch = "wasm32")]
    use macroquad::file::load_file;
    #[cfg(not(target_arch = "wasm32"))]
    use quad_snd::PlaySoundParams;
    use quad_snd::{AudioContext, Playback};

    #[cfg(not(target_arch = "wasm32"))]
    use super::native_music::{Contexts, Music, Pending};

    /// Plays through quad-snd. Only usable inside macroquad's main loop.
    #[derive(Default)]
    pub(crate) struct Macroquad {
        /// Spare music contexts.
        #[cfg(not(target_arch = "wasm32"))]
        contexts: Contexts,
        /// The sound effects' context, opened with the first sound.
        effects: Option<AudioContext>,
    }

    /// A decoded sound, track or voice clip.
    pub(crate) enum Clip {
        /// A sound effect, in the effects' context.
        Effect(quad_snd::Sound),
        /// A music track or voice clip on the web, in macroquad's context,
        /// with its length in milliseconds if it is an OGG file.
        #[cfg(target_arch = "wasm32")]
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
        /// A sound effect's playback; music and voices have none (they are
        /// stopped by their clip).
        type Playing = Option<Playback>;

        #[cfg(not(target_arch = "wasm32"))]
        fn load_sound(
            &mut self,
            bytes: &[u8],
        ) -> impl std::future::Future<Output = Result<Clip, String>> {
            use std::panic::{AssertUnwindSafe, catch_unwind};
            let ctx = self.effects.get_or_insert_with(AudioContext::new);
            // quad-snd `unwrap`s the decode, at once on native.
            let loaded = catch_unwind(AssertUnwindSafe(|| quad_snd::Sound::load(ctx, bytes)))
                .map(Clip::Effect)
                .map_err(|_| "not a readable OGG or WAV file".to_owned());
            std::future::ready(loaded)
        }

        #[cfg(target_arch = "wasm32")]
        async fn load_sound(&mut self, bytes: &[u8]) -> Result<Clip, String> {
            let ctx = self.effects.get_or_insert_with(AudioContext::new);
            let sound = quad_snd::Sound::load(ctx, bytes);
            // The browser decodes it in the background, as for
            // macroquad's `load_sound_from_bytes`.
            while !sound.is_loaded() {
                macroquad::window::next_frame().await;
            }
            Ok(Clip::Effect(sound))
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
                Clip::Effect(_) => None,
                #[cfg(target_arch = "wasm32")]
                Clip::Shared(_, length) => *length,
                #[cfg(not(target_arch = "wasm32"))]
                Clip::Music(m) => m.length_ms(),
            };
            ms.map(|ms| f64::from(ms) / 1000.0)
        }

        fn play(&mut self, sound: &Clip, volume: f32, looped: bool) -> Option<Playback> {
            let params = PlaySoundParams { looped, volume };
            match sound {
                Clip::Effect(s) => return Some(s.play(self.effects.as_ref()?, params)),
                #[cfg(target_arch = "wasm32")]
                Clip::Shared(s, _) => play_sound(s, params),
                #[cfg(not(target_arch = "wasm32"))]
                Clip::Music(m) => m.play(volume, looped),
            }
            None
        }

        fn set_volume(&mut self, sound: &Clip, volume: f32) {
            match sound {
                Clip::Effect(s) => {
                    if let Some(ctx) = &self.effects {
                        s.set_volume(ctx, volume);
                    }
                }
                #[cfg(target_arch = "wasm32")]
                Clip::Shared(s, _) => set_sound_volume(s, volume),
                #[cfg(not(target_arch = "wasm32"))]
                Clip::Music(m) => m.set_volume(volume),
            }
        }

        fn stop(&mut self, sound: &Clip) {
            match sound {
                Clip::Effect(s) => {
                    if let Some(ctx) = &self.effects {
                        s.stop(ctx);
                    }
                }
                #[cfg(target_arch = "wasm32")]
                Clip::Shared(s, _) => stop_sound(s),
                #[cfg(not(target_arch = "wasm32"))]
                Clip::Music(m) => m.stop(),
            }
        }

        fn set_playing_volume(&mut self, playing: &Option<Playback>, volume: f32) {
            if let (Some(playback), Some(ctx)) = (playing, &self.effects) {
                playback.set_volume(ctx, volume);
            }
        }

        fn stop_playing(&mut self, playing: Option<Playback>) {
            if let (Some(playback), Some(ctx)) = (playing, &self.effects) {
                playback.stop(ctx);
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
    use shuyi_assets::voice::ClipVariant;
    use shuyi_ui::audio::AudioRequest;

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

    impl<B: Backend> Audio<B> {
        /// [`play`](Audio::play) with the screens' own requests, as `App`
        /// hands them to the device.
        pub(crate) fn hear(
            &mut self,
            backend: &mut B,
            requests: &[Request],
            music: &[MusicCommand],
            now: f64,
        ) {
            let requests: Vec<SoundRequest> = requests
                .iter()
                .filter_map(SoundRequest::from_request)
                .collect();
            self.play(backend, &requests, music, now);
        }
    }

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
    /// plays for `length` seconds, if set. Each play is a playback,
    /// numbered from 1 (`#1`).
    #[derive(Default)]
    pub(crate) struct Fake {
        pub(crate) log: Log,
        pub(crate) ready: Vec<String>,
        pub(crate) broken: Vec<String>,
        pub(crate) length: Option<f64>,
        pub(crate) plays: u32,
    }

    impl Backend for Fake {
        type Sound = String;
        type Loading = String;
        type Playing = u32;

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

        fn play(&mut self, sound: &String, volume: f32, looped: bool) -> u32 {
            let looped = if looped { " looped" } else { "" };
            self.log
                .borrow_mut()
                .push(format!("play {sound} {volume:.2}{looped}"));
            self.plays += 1;
            self.plays
        }

        fn set_volume(&mut self, sound: &String, volume: f32) {
            self.log
                .borrow_mut()
                .push(format!("volume {sound} {volume:.2}"));
        }

        fn stop(&mut self, sound: &String) {
            self.log.borrow_mut().push(format!("stop {sound}"));
        }

        fn set_playing_volume(&mut self, playing: &u32, volume: f32) {
            self.log
                .borrow_mut()
                .push(format!("volume #{playing} {volume:.2}"));
        }

        fn stop_playing(&mut self, playing: u32) {
            self.log.borrow_mut().push(format!("stop #{playing}"));
        }
    }

    impl Fake {
        pub(crate) fn calls(&self) -> Vec<String> {
            std::mem::take(&mut self.log.borrow_mut())
        }
    }

    fn manifest() -> AudioManifest {
        let mut m = AudioManifest::default();
        let sound = |files: &[&str], volume| {
            SoundCue::new(
                files.iter().map(|f| (*f).to_owned()).collect(),
                volume,
                CreditRef::Own,
            )
        };
        m.sounds.insert("beep".into(), sound(&["sfx/beep.wav"], 50));
        let rain = sound(&["sfx/beep.wav"], 50).with_looped(true);
        m.sounds.insert("rain".into(), rain);
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
        Files::new(&bytes, &shown, &|_: &str| false, "music")
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
        MusicCommand::start(cue, 1.0)
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
    fn a_started_sound_is_one_playback_its_requests_and_the_sound_volume_reach() {
        let mut fake = Fake::default();
        let mut audio = audio(&mut fake);
        audio.take_warnings();
        let mut queue = shuyi_ui::audio::AudioQueue::<Take>::default();
        let [first, second] = [queue.start_sound("rain"), queue.start_sound("rain")];
        let once = queue.start_sound("beep");
        audio.hear(&mut fake, &queue.take(), &[], 0.0);
        assert_eq!(
            fake.calls(),
            [
                "play wav100 0.50 looped",
                "play wav100 0.50 looped",
                "play wav100 0.50"
            ]
        );
        queue.set_sound_volume(first, 0.5);
        audio.hear(&mut fake, &queue.take(), &[], 0.0);
        assert_eq!(fake.calls(), ["volume #1 0.25"]);
        // The player's sound volume reaches every playback, at its own.
        audio.set_volumes(&mut fake, 1.0, 0.4);
        assert_eq!(
            fake.calls(),
            ["volume #1 0.10", "volume #2 0.20", "volume #3 0.20"]
        );
        // A stop stops that copy alone; the other plays on.
        queue.stop_sound(first, 0);
        queue.stop_sound(once, 0);
        audio.hear(&mut fake, &queue.take(), &[], 0.0);
        assert_eq!(fake.calls(), ["stop #1", "stop #3"]);
        audio.hear(&mut fake, &[], &[], 0.0);
        let got = fake.calls();
        assert!(got.is_empty(), "{got:?}");
        audio.stop_all(&mut fake);
        assert_eq!(fake.calls(), ["stop #2"]);
        queue.set_sound_volume(second, 1.0);
        queue.stop_sound(second, 0);
        audio.hear(&mut fake, &queue.take(), &[], 0.0);
        assert!(fake.calls().is_empty(), "a stopped id is gone");
    }

    #[test]
    fn a_pause_mutes_the_sounds_playing_and_its_resume_restores_them() {
        let mut fake = Fake::default();
        let mut audio = audio(&mut fake);
        let mut queue = shuyi_ui::audio::AudioQueue::<Take>::default();
        let below = queue.start_sound("rain");
        queue.pause_sounds();
        let menu = queue.start_sound_at("rain", 0.5);
        audio.hear(&mut fake, &queue.take(), &[], 0.0);
        assert_eq!(
            fake.calls(),
            [
                "play wav100 0.50 looped",
                "volume #1 0.00",
                "play wav100 0.25 looped"
            ]
        );
        // A paused sound keeps the volumes it is given, silently.
        queue.set_sound_volume(below, 0.5);
        audio.hear(&mut fake, &queue.take(), &[], 0.0);
        audio.set_volumes(&mut fake, 1.0, 0.5);
        assert_eq!(
            fake.calls(),
            ["volume #1 0.00", "volume #1 0.00", "volume #2 0.12"]
        );
        queue.resume_sounds();
        audio.hear(&mut fake, &queue.take(), &[], 0.0);
        assert_eq!(fake.calls(), ["volume #1 0.12"]);
        assert!(queue.sound_is_live(menu));
    }

    #[test]
    fn a_suspend_mutes_the_sounds_and_the_music_and_the_resume_restores_them() {
        let mut fake = Fake {
            ready: vec!["music/title.ogg".into()],
            ..Fake::default()
        };
        let mut audio = audio(&mut fake);
        let mut queue = shuyi_ui::audio::AudioQueue::<Take>::default();
        let [below, other] = [queue.start_sound("rain"), queue.start_sound("rain")];
        queue.pause_sounds();
        let music = [cmd(load, "title"), cmd(start, "title")];
        audio.hear(&mut fake, &queue.take(), &music, 0.0);
        fake.calls();
        audio.suspend(&mut fake);
        assert_eq!(
            fake.calls(),
            [
                "volume #1 0.00",
                "volume #2 0.00",
                "volume music/title.ogg 0.00"
            ]
        );
        audio.suspend(&mut fake);
        assert!(fake.calls().is_empty(), "a second suspend does nothing");
        // It mutes: the track's clock runs on.
        assert_eq!(audio.music_playing(2.0), Some(("title", 2.0)));
        audio.resume(&mut fake);
        // The pause under a menu holds; the music comes back.
        assert_eq!(
            fake.calls(),
            [
                "volume #1 0.00",
                "volume #2 0.00",
                "volume music/title.ogg 0.80"
            ]
        );
        audio.resume(&mut fake);
        assert!(fake.calls().is_empty(), "a second resume does nothing");
        queue.resume_sounds();
        audio.hear(&mut fake, &queue.take(), &[], 3.0);
        assert_eq!(fake.calls(), ["volume #1 0.50", "volume #2 0.50"]);
        assert!(queue.sound_is_live(below) && queue.sound_is_live(other));
    }

    #[test]
    fn a_looped_cue_played_without_a_handle_plays_once() {
        let mut fake = Fake::default();
        let mut audio = audio(&mut fake);
        audio.hear(&mut fake, &[sound("rain", 1.0)], &[], 0.0);
        assert_eq!(fake.calls(), ["play wav100 0.50"]);
    }

    #[test]
    fn a_generated_wav_plays_through_the_app_path() {
        let mut fake = Fake::default();
        let mut audio = audio(&mut fake);
        assert_eq!(
            audio.take_warnings(),
            ["sound \"broken\": bundle/sfx/broken.wav: not a PCM WAV"]
        );
        let music_request = Request::play_music("title", None);
        audio.hear(
            &mut fake,
            &[sound("beep", 1.0), music_request, sound("beep", 0.5)],
            &[],
            0.0,
        );
        assert_eq!(fake.calls(), ["play wav100 0.50", "play wav100 0.25"]);
        // A sound whose file failed to decode is silent, not a crash.
        audio.hear(&mut fake, &[sound("broken", 1.0)], &[], 0.0);
        let got = fake.calls();
        assert!(got.is_empty(), "{got:?}");
        let got = audio.take_warnings();
        assert!(got.is_empty(), "{got:?}");
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
        audio.hear(&mut fake, &[sound("nope", 1.0)], &[cmd(load, "nope")], 0.0);
        assert_eq!(
            audio.take_warnings(),
            ["unknown sound cue \"nope\"", "unknown music cue \"nope\""]
        );
        let got = fake.calls();
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn variants_are_picked_at_random() {
        let mut fake = Fake::default();
        let mut audio = audio(&mut fake);
        let steps: Vec<_> = (0..30).map(|_| sound("step", 1.0)).collect();
        audio.hear(&mut fake, &steps, &[], 0.0);
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
        audio.play(
            &mut fake,
            &[],
            &[cmd(load, "title"), cmd(start, "title")],
            0.0,
        );
        assert_eq!(fake.calls(), ["load music/title.ogg"]);
        audio.play(&mut fake, &[], &[], 0.0);
        assert!(fake.calls().is_empty(), "still loading");
        fake.ready.push("music/title.ogg".into());
        audio.play(&mut fake, &[], &[], 0.0);
        assert_eq!(fake.calls(), ["play music/title.ogg 0.80 looped"]);
        audio.play(&mut fake, &[], &[], 0.0);
        assert!(fake.calls().is_empty(), "started once");
        // A fade scales the cue's own volume; then stop frees it.
        audio.play(&mut fake, &[], &[gain("title", 0.5)], 0.0);
        assert_eq!(fake.calls(), ["volume music/title.ogg 0.40"]);
        audio.play(&mut fake, &[], &[cmd(stop, "title")], 0.0);
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
        audio.hear(
            &mut fake,
            &[sound("beep", 1.0), sound("beep", 0.5)],
            &[],
            0.0,
        );
        assert_eq!(fake.calls(), ["play wav100 0.10", "play wav100 0.05"]);
        // Music starts at the cue's 80 % times the player's 50 %.
        let begin = [cmd(load, "title"), cmd(start, "title")];
        audio.play(&mut fake, &[], &begin, 0.0);
        assert_eq!(
            fake.calls(),
            ["load music/title.ogg", "play music/title.ogg 0.40 looped"]
        );
        // The same volumes again change nothing; a new music volume
        // reaches the track playing, and a fade multiplies with it.
        audio.set_volumes(&mut fake, 0.5, 0.2);
        let got = fake.calls();
        assert!(got.is_empty(), "{got:?}");
        audio.set_volumes(&mut fake, 1.0, 0.2);
        assert_eq!(fake.calls(), ["volume music/title.ogg 0.80"]);
        audio.play(&mut fake, &[], &[gain("title", 0.5)], 0.0);
        assert_eq!(fake.calls(), ["volume music/title.ogg 0.40"]);
        audio.set_volumes(&mut fake, 0.5, 0.2);
        assert_eq!(fake.calls(), ["volume music/title.ogg 0.20"]);
        // Silent: sounds and music at 0.
        audio.set_volumes(&mut fake, 0.0, 0.0);
        assert_eq!(fake.calls(), ["volume music/title.ogg 0.00"]);
        audio.hear(&mut fake, &[sound("beep", 1.0)], &[], 0.0);
        assert_eq!(fake.calls(), ["play wav100 0.00"]);
    }

    /// A track already loaded and started later, or still loading when the
    /// volume is set, starts at the player's volume.
    #[test]
    fn a_track_that_starts_later_uses_the_players_volume() {
        let mut fake = Fake::default();
        let mut audio = audio(&mut fake);
        let begin = [cmd(load, "title"), cmd(start, "title")];
        audio.play(&mut fake, &[], &begin, 0.0);
        fake.calls();
        // Still loading: nothing to change yet.
        audio.set_volumes(&mut fake, 0.25, 1.0);
        let got = fake.calls();
        assert!(got.is_empty(), "{got:?}");
        fake.ready.push("music/title.ogg".into());
        audio.play(&mut fake, &[], &[], 0.0);
        assert_eq!(fake.calls(), ["play music/title.ogg 0.20 looped"]);
        // Started again once loaded (game ADR-0026: `Start` on a ready track).
        audio.play(&mut fake, &[], &[cmd(start, "title")], 0.0);
        assert_eq!(fake.calls(), ["play music/title.ogg 0.20 looped"]);
    }

    #[test]
    fn a_loaded_track_starts_at_once_and_plays_once_if_not_looped() {
        let mut fake = Fake::default();
        fake.ready.push("music/sting.ogg".into());
        let mut audio = audio(&mut fake);
        audio.play(&mut fake, &[], &[cmd(load, "sting")], 0.0);
        assert_eq!(fake.calls(), ["load music/sting.ogg"]);
        audio.play(
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
        audio.play(
            &mut fake,
            &[],
            &[cmd(load, "title"), cmd(start, "title")],
            0.0,
        );
        audio.play(&mut fake, &[], &[gain("title", 0.5)], 0.0);
        fake.ready.push("music/title.ogg".into());
        audio.play(&mut fake, &[], &[], 0.0);
        assert_eq!(
            fake.calls(),
            ["load music/title.ogg", "play music/title.ogg 0.40 looped"]
        );
    }

    #[test]
    fn a_stopped_load_is_polled_until_done_then_dropped() {
        let mut fake = Fake::default();
        let mut audio = audio(&mut fake);
        audio.play(
            &mut fake,
            &[],
            &[cmd(load, "title"), cmd(stop, "title")],
            0.0,
        );
        assert_eq!(audio.abandoned.len(), 1);
        audio.play(&mut fake, &[], &[], 0.0);
        assert_eq!(audio.abandoned.len(), 1);
        fake.ready.push("music/title.ogg".into());
        audio.play(&mut fake, &[], &[], 0.0);
        assert!(audio.abandoned.is_empty(), "{:?}", audio.abandoned);
        assert_eq!(fake.calls(), ["load music/title.ogg"], "never played");
    }

    #[test]
    fn a_missing_track_is_a_warning_and_silence() {
        let mut fake = Fake::default();
        fake.broken.push("music/lost.ogg".into());
        let mut audio = audio(&mut fake);
        audio.take_warnings();
        audio.play(
            &mut fake,
            &[],
            &[cmd(load, "lost"), cmd(start, "lost")],
            0.0,
        );
        assert_eq!(
            audio.take_warnings(),
            ["music \"lost\": music/lost.ogg: no such file"]
        );
        audio.play(&mut fake, &[], &[gain("lost", 0.5), cmd(stop, "lost")], 0.0);
        assert_eq!(fake.calls(), ["load music/lost.ogg"]);
    }

    #[test]
    fn loading_again_replaces_the_track() {
        let mut fake = Fake::default();
        fake.ready.push("music/title.ogg".into());
        let mut audio = audio(&mut fake);
        audio.play(
            &mut fake,
            &[],
            &[cmd(load, "title"), cmd(start, "title")],
            0.0,
        );
        audio.play(&mut fake, &[], &[], 0.0);
        audio.play(&mut fake, &[], &[cmd(load, "title")], 0.0);
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
        audio.play(&mut fake, &[], &[cmd(load, "title")], 0.0);
        audio.play(
            &mut fake,
            &[],
            &[gain("title", 0.5), cmd(stop, "title")],
            0.0,
        );
        assert_eq!(fake.calls(), ["load music/title.ogg"]);
        assert!(audio.tracks.is_empty());
        // Commands for tracks that aren't there do nothing.
        audio.play(
            &mut fake,
            &[],
            &[cmd(start, "title"), gain("title", 1.0)],
            0.0,
        );
        let got = fake.calls();
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn stop_all_silences_every_track() {
        let mut fake = Fake::default();
        fake.ready.push("music/title.ogg".into());
        let mut audio = audio(&mut fake);
        let cmds = [cmd(load, "title"), cmd(start, "title"), cmd(load, "sting")];
        audio.play(&mut fake, &[], &cmds, 0.0);
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
        audio.play(&mut fake, &[], &cmds, 10.0);
        audio.play(&mut fake, &[], &[], 11.0);
        assert_eq!(audio.music_playing(11.5), None, "still loading");
        fake.ready.push("music/title.ogg".into());
        assert_eq!(audio.music_playing(12.0), None, "loaded, not yet started");
        audio.play(&mut fake, &[], &[], 12.5);
        assert_eq!(audio.music_playing(12.5), Some(("title", 0.0)));
        assert_eq!(audio.music_playing(14.0), Some(("title", 1.5)));
        // A fade doesn't move it, and it keeps counting past the loop
        // (`ui` wraps it at the track's length).
        audio.play(&mut fake, &[], &[gain("title", 0.5)], 15.0);
        assert_eq!(audio.music_playing(100.0), Some(("title", 87.5)));
        // A clock that stepped back gives 0, not a negative time.
        assert_eq!(audio.music_playing(12.0), Some(("title", 0.0)));
        audio.play(&mut fake, &[], &[cmd(stop, "title")], 101.0);
        assert_eq!(audio.music_playing(101.0), None);
    }

    #[test]
    fn a_loaded_track_is_reported_from_its_start_command() {
        let mut fake = Fake::default();
        fake.ready.push("music/sting.ogg".into());
        let mut audio = audio(&mut fake);
        audio.play(&mut fake, &[], &[cmd(load, "sting")], 1.0);
        assert_eq!(audio.music_playing(2.0), None, "ready, not asked to play");
        audio.play(&mut fake, &[], &[cmd(start, "sting")], 3.0);
        assert_eq!(audio.music_playing(3.25), Some(("sting", 0.25)));
        // Loading it again replaces the track: silence until it restarts.
        audio.play(&mut fake, &[], &[cmd(load, "sting")], 4.0);
        assert_eq!(audio.music_playing(4.5), None);
    }

    #[test]
    fn a_track_that_fails_to_load_is_never_reported() {
        let mut fake = Fake::default();
        fake.broken.push("music/lost.ogg".into());
        let mut audio = audio(&mut fake);
        let cmds = [cmd(load, "lost"), cmd(start, "lost")];
        audio.play(&mut fake, &[], &cmds, 1.0);
        audio.play(&mut fake, &[], &[], 2.0);
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
        audio.play(&mut fake, &[], &cmds, 1.0);
        let fading = [cmd(load, "sting"), gain("title", 0.5)];
        audio.play(&mut fake, &[], &fading, 5.0);
        assert_eq!(audio.music_playing(5.25), Some(("title", 4.25)));
        // The fade ends before the new track has loaded: silence.
        let switch = [cmd(stop, "title"), cmd(start, "sting")];
        audio.play(&mut fake, &[], &switch, 5.5);
        assert_eq!(audio.music_playing(6.0), None);
        fake.ready.push("music/sting.ogg".into());
        audio.play(&mut fake, &[], &[], 7.0);
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
            audio.play(&mut fake, &[], &cmds, 0.0);
            audio.play(&mut fake, &[], &[cmd(start, first)], 1.0);
            audio.play(&mut fake, &[], &[cmd(start, second)], 2.0);
            assert_eq!(audio.music_playing(2.5), Some((second, 0.5)));
        }
    }

    /// A track that fades in starts at its `Start`'s gain, loaded or not.
    #[test]
    fn a_start_at_gain_zero_plays_at_volume_zero() {
        let mut fake = Fake::default();
        fake.ready.push("music/sting.ogg".into());
        let mut audio = audio(&mut fake);
        let silent = MusicCommand::start("sting", 0.0);
        audio.play(&mut fake, &[], &[cmd(load, "sting"), silent], 0.0);
        audio.play(&mut fake, &[], &[gain("sting", 0.5)], 0.0);
        assert_eq!(
            fake.calls(),
            [
                "load music/sting.ogg",
                "play music/sting.ogg 0.00",
                "volume music/sting.ogg 0.50"
            ]
        );
        // Still loading: it starts at that gain once loaded.
        let silent = MusicCommand::start("title", 0.0);
        audio.play(&mut fake, &[], &[cmd(load, "title"), silent], 0.0);
        fake.ready.push("music/title.ogg".into());
        audio.play(&mut fake, &[], &[], 0.0);
        assert_eq!(
            fake.calls(),
            ["load music/title.ogg", "play music/title.ogg 0.00 looped"]
        );
    }

    /// A crossfade, as `MusicState` sends it: the two tracks play at once,
    /// their gains moving opposite ways, and the report follows the new
    /// one once it sounds.
    #[test]
    fn two_tracks_play_at_once_in_a_crossfade() {
        let mut fake = Fake::default();
        fake.ready.push("music/title.ogg".into());
        let mut audio = audio(&mut fake);
        let cmds = [cmd(load, "title"), cmd(start, "title")];
        audio.play(&mut fake, &[], &cmds, 1.0);
        let crossfade = [cmd(load, "sting"), MusicCommand::start("sting", 0.0)];
        audio.play(&mut fake, &[], &crossfade, 2.0);
        assert_eq!(audio.music_playing(2.5), Some(("title", 1.5)));
        fake.ready.push("music/sting.ogg".into());
        audio.play(&mut fake, &[], &[], 3.0);
        assert_eq!(audio.music_playing(3.5), Some(("sting", 0.5)));
        let halfway = [gain("title", 0.5), gain("sting", 0.5)];
        audio.play(&mut fake, &[], &halfway, 3.5);
        audio.play(
            &mut fake,
            &[],
            &[cmd(stop, "title"), gain("sting", 1.0)],
            4.0,
        );
        assert_eq!(
            fake.calls(),
            [
                "load music/title.ogg",
                "play music/title.ogg 0.80 looped",
                "load music/sting.ogg",
                "play music/sting.ogg 0.00",
                "volume music/title.ogg 0.40",
                "volume music/sting.ogg 0.50",
                "stop music/title.ogg",
                "volume music/sting.ogg 1.00",
            ]
        );
        assert_eq!(audio.music_playing(4.5), Some(("sting", 1.5)));
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
