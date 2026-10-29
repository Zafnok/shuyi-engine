//! Voice clips (game ADR-0046): the files of the game's voice folder for the
//! script's language (`voice/<lang>/`, say), loaded when asked for and a
//! few ahead, played one at a time and freed when they end.
//!
//! A clip is loaded like a music track (on native, on game ADR-0028's worker
//! thread). A scene says which clips are coming
//! ([`SoundRequest::PreloadVoices`]); the next [`AHEAD`] of them are kept
//! loaded, so a line's clip is ready when the line is shown. A clip that
//! still isn't loaded [`LATE_SECS`] after it was asked for is not played:
//! a voice starting in the middle of a line is worse than none.

use std::collections::BTreeMap;
use std::path::Path;

use shuyi_platform::SoundRequest;

use super::{Backend, folder_beside};

/// How many of the clips that are coming are kept loaded (*tunable*).
pub(crate) const AHEAD: usize = 3;
/// A clip not loaded this many seconds after it was asked for is dropped
/// rather than played late (*tunable*).
pub(crate) const LATE_SECS: f64 = 0.3;

/// The voice player.
pub(crate) struct Voices<B: Backend> {
    /// Where the clips are, e.g. `voice/en`.
    dir: String,
    /// How loud clips play, 0–1.
    volume: f32,
    /// Clips loading or loaded, by file (relative to `dir`).
    clips: BTreeMap<String, Clip<B>>,
    /// The clips a scene said are coming, in order.
    coming: Vec<String>,
    /// How many of `coming` are behind us: the clip asked for last, and
    /// those before it.
    passed: usize,
    /// The clip asked for and not yet started, and when it was asked for.
    asked: Option<(String, f64)>,
    /// The clip that is playing, and when it ends (if its length is known).
    playing: Option<(String, Option<f64>)>,
}

enum Clip<B: Backend> {
    Loading(B::Loading),
    Ready(B::Sound),
}

impl<B: Backend> Voices<B> {
    /// A player of the clips in `dir`, at full volume, with nothing loaded.
    pub(crate) fn new(dir: String) -> Self {
        Self {
            dir,
            volume: 1.0,
            clips: BTreeMap::new(),
            coming: Vec::new(),
            passed: 0,
            asked: None,
            playing: None,
        }
    }

    /// Applies one request (those that aren't about voices do nothing).
    /// Loads dropped before they finished go to `abandoned`.
    pub(crate) fn request(
        &mut self,
        backend: &mut B,
        request: &SoundRequest,
        now: f64,
        abandoned: &mut Vec<B::Loading>,
    ) {
        match request {
            SoundRequest::Voice { clip } => {
                let file = clip.clone();
                self.stop(backend);
                if let Some(at) = self.coming.iter().position(|f| *f == file) {
                    self.passed = at + 1;
                }
                self.asked = Some((file, now));
            }
            SoundRequest::StopVoice => self.stop(backend),
            SoundRequest::PreloadVoices { clips } => {
                self.coming.clone_from(clips);
                self.passed = 0;
            }
            SoundRequest::Sound { .. } => return,
        }
        self.keep_wanted(backend, abandoned);
    }

    /// Stops the voice that is playing or about to, and frees it.
    fn stop(&mut self, backend: &mut B) {
        self.asked = None;
        if let Some((file, _)) = self.playing.take()
            && let Some(Clip::Ready(sound)) = self.clips.remove(&file)
        {
            backend.stop(&sound);
        }
    }

    /// Whether `file` is wanted in memory: asked for, playing, or one of
    /// the next [`AHEAD`] coming.
    fn wanted(&self, file: &str) -> bool {
        let is = |f: &String| f == file;
        self.asked.as_ref().is_some_and(|(f, _)| is(f))
            || self.playing.as_ref().is_some_and(|(f, _)| is(f))
            || self.coming.iter().skip(self.passed).take(AHEAD).any(is)
    }

    /// Frees the clips no longer wanted and starts loading the wanted ones
    /// that aren't loaded.
    fn keep_wanted(&mut self, backend: &mut B, abandoned: &mut Vec<B::Loading>) {
        let unwanted: Vec<String> = self
            .clips
            .keys()
            .filter(|file| !self.wanted(file))
            .cloned()
            .collect();
        for file in unwanted {
            if let Some(Clip::Loading(loading)) = self.clips.remove(&file) {
                abandoned.push(loading);
            }
        }
        let ahead = self.coming.iter().skip(self.passed).take(AHEAD);
        let asked = self.asked.iter().map(|(file, _)| file);
        let missing: Vec<String> = asked
            .chain(ahead)
            .filter(|file| !self.clips.contains_key(*file))
            .cloned()
            .collect();
        for file in missing {
            let loading = backend.load_music(&format!("{}/{file}", self.dir));
            self.clips.insert(file, Clip::Loading(loading));
        }
    }

    /// Once per frame: finishes loads, starts the clip asked for if it is
    /// ready (or gives up on it if it is late), and frees the clip that
    /// has ended. Failed loads are added to `warnings`.
    pub(crate) fn poll(
        &mut self,
        backend: &mut B,
        now: f64,
        warnings: &mut Vec<String>,
        abandoned: &mut Vec<B::Loading>,
    ) {
        let mut failed = Vec::new();
        for (file, clip) in &mut self.clips {
            let Clip::Loading(loading) = clip else {
                continue;
            };
            match backend.poll_music(loading) {
                None => {}
                Some(Ok(sound)) => *clip = Clip::Ready(sound),
                Some(Err(e)) => {
                    warnings.push(format!("voice clip {}/{file}: {e}", self.dir));
                    failed.push(file.clone());
                }
            }
        }
        for file in failed {
            self.clips.remove(&file);
            self.asked.take_if(|(asked, _)| *asked == file);
        }
        if let Some((file, at)) = self.asked.take() {
            if let Some(Clip::Ready(sound)) = self.clips.get(&file) {
                backend.play(sound, self.volume, false);
                let ends = backend.length(sound).map(|length| now + length);
                self.playing = Some((file, ends));
            } else if now - at <= LATE_SECS {
                self.asked = Some((file, at));
            }
        }
        if self
            .playing
            .as_ref()
            .is_some_and(|(_, ends)| ends.is_some_and(|ends| ends <= now))
        {
            self.playing = None;
        }
        self.keep_wanted(backend, abandoned);
    }

    /// Sets how loud clips play (0–1), the one playing included.
    pub(crate) fn set_volume(&mut self, backend: &mut B, volume: f32) {
        // Volumes are steps of a tenth: exact equality is the point.
        #[allow(clippy::float_cmp)]
        if volume == self.volume {
            return;
        }
        self.volume = volume;
        if let Some((file, _)) = &self.playing
            && let Some(Clip::Ready(sound)) = self.clips.get(file)
        {
            backend.set_volume(sound, volume);
        }
    }

    /// Stops the voice and frees every clip (the game quit).
    pub(crate) fn stop_all(&mut self, backend: &mut B, abandoned: &mut Vec<B::Loading>) {
        self.stop(backend);
        self.coming.clear();
        self.passed = 0;
        self.keep_wanted(backend, abandoned);
    }

    /// The files in memory or on their way there, in name order.
    #[cfg(test)]
    pub(crate) fn held(&self) -> Vec<&str> {
        self.clips.keys().map(String::as_str).collect()
    }
}

/// Where the voice clips of the script's language `lang` are: `<lang>` in
/// the voice `folder` next to the executable when it's there (a shipped
/// build), else relative to the working directory (`cargo run` from the
/// repo root, or the web build, which fetches from beside the page).
pub(crate) fn platform_voice_dir(folder: &str, lang: &str) -> String {
    let voice = if cfg!(target_arch = "wasm32") {
        folder.to_owned()
    } else {
        let exe = std::env::current_exe().ok();
        folder_beside(folder, exe.as_deref(), Path::is_dir)
    };
    format!("{voice}/{lang}")
}

/// The text of the voice manifest `file` in `dir`, or `None` if there is
/// none (no voices: not a problem). `Err` if the file is there and can't
/// be read.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::unused_async)] // The web build fetches it.
pub async fn read_manifest(dir: &str, file: &str) -> Result<Option<String>, String> {
    let path = format!("{dir}/{file}");
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{path}: {e}")),
    }
}

/// The text of the voice manifest `file` in `dir`, fetched from beside the
/// page, or `None` if it can't be had: a build without voices has no such
/// file, and a failed fetch doesn't say why it failed.
#[cfg(target_arch = "wasm32")]
pub async fn read_manifest(dir: &str, file: &str) -> Result<Option<String>, String> {
    let path = format!("{dir}/{file}");
    Ok(macroquad::file::load_string(&path).await.ok())
}

#[cfg(test)]
mod tests {
    use shuyi_assets::dialogue::LineId;

    use super::super::tests::{Fake, Request, Take, audio, block_on};
    use super::*;
    use crate::audio::Audio;

    /// Five lines of a scene called `test`.
    const LINES: [&str; 5] = [
        "test_21833604",
        "test_7de7b254",
        "test_01439969",
        "test_5da5d147",
        "test_d7fb4eb3",
    ];

    fn line(i: usize) -> LineId {
        LineId::new(LINES[i])
    }

    /// The path the fake backend sees for clip `i`.
    fn path(i: usize) -> String {
        format!("voice/en/test/{}.ogg", LINES[i])
    }

    fn play(i: usize) -> Request {
        Request::PlayVoice {
            line: line(i),
            variant: Take::Only,
        }
    }

    fn preload(lines: &[usize]) -> Request {
        Request::PreloadVoices {
            lines: lines.iter().map(|&i| (line(i), Take::Only)).collect(),
        }
    }

    /// The line ids of the files held, in name order.
    fn held(audio: &Audio<Fake>) -> Vec<String> {
        let stem = |file: &&str| {
            let stem = file.trim_start_matches("test/").trim_end_matches(".ogg");
            stem.to_owned()
        };
        audio.voices.held().iter().map(stem).collect()
    }

    /// `LINES[i]` for each `i`, in name order, as [`held`] lists them.
    fn sorted(lines: &[usize]) -> Vec<&'static str> {
        let mut ids: Vec<&str> = lines.iter().map(|&i| LINES[i]).collect();
        ids.sort_unstable();
        ids
    }

    /// A backend where every clip of [`LINES`] loads at once.
    fn all_ready() -> Fake {
        Fake {
            ready: (0..LINES.len()).map(path).collect(),
            ..Fake::default()
        }
    }

    /// Acceptance: a second `PlayVoice` stops the first; `StopVoice` stops
    /// the voice.
    #[test]
    fn one_voice_at_a_time_and_stop_stops_it() {
        let mut fake = all_ready();
        let mut audio = audio(&mut fake);
        audio.hear(&mut fake, &[play(0)], &[], 0.0);
        assert_eq!(
            fake.calls(),
            [
                format!("load {}", path(0)),
                format!("play {} 1.00", path(0))
            ]
        );
        audio.hear(&mut fake, &[play(1)], &[], 1.0);
        assert_eq!(
            fake.calls(),
            [
                format!("stop {}", path(0)),
                format!("load {}", path(1)),
                format!("play {} 1.00", path(1)),
            ]
        );
        assert_eq!(held(&audio), [LINES[1]], "the first clip is freed");
        audio.hear(&mut fake, &[Request::StopVoice], &[], 2.0);
        assert_eq!(fake.calls(), [format!("stop {}", path(1))]);
        let found = held(&audio);
        assert!(found.is_empty(), "{found:?}");
        // Stopping silence does nothing.
        audio.hear(&mut fake, &[Request::StopVoice], &[], 3.0);
        let found = fake.calls();
        assert!(found.is_empty(), "{found:?}");
        assert!(
            audio.take_warnings().len() <= 1,
            "only the broken test sound"
        );
    }

    /// Acceptance: clips play at the voice volume, and a change reaches
    /// the clip that is playing.
    #[test]
    fn the_volume_follows_the_voice_volume() {
        let mut fake = all_ready();
        let mut audio = audio(&mut fake);
        audio.set_voice_volume(&mut fake, 0.8);
        assert!(fake.calls().is_empty(), "nothing is playing yet");
        audio.hear(&mut fake, &[play(0)], &[], 0.0);
        assert_eq!(fake.calls()[1], format!("play {} 0.80", path(0)));
        audio.set_voice_volume(&mut fake, 0.3);
        assert_eq!(fake.calls(), [format!("volume {} 0.30", path(0))]);
        // The same volume again (every frame sends it) does nothing.
        audio.set_voice_volume(&mut fake, 0.3);
        let found = fake.calls();
        assert!(found.is_empty(), "{found:?}");
        audio.hear(&mut fake, &[play(1)], &[], 1.0);
        assert_eq!(fake.calls()[2], format!("play {} 0.30", path(1)));
    }

    #[test]
    fn a_clip_plays_once_it_has_loaded_if_that_is_soon() {
        let mut fake = Fake::default();
        let mut audio = audio(&mut fake);
        audio.hear(&mut fake, &[play(0)], &[], 10.0);
        assert_eq!(fake.calls(), [format!("load {}", path(0))]);
        audio.play(&mut fake, &[], &[], 10.2);
        assert!(fake.calls().is_empty(), "still loading");
        fake.ready.push(path(0));
        audio.play(&mut fake, &[], &[], 10.0 + LATE_SECS);
        assert_eq!(fake.calls(), [format!("play {} 1.00", path(0))]);
        audio.play(&mut fake, &[], &[], 11.0);
        assert!(fake.calls().is_empty(), "started once");
    }

    #[test]
    fn a_clip_that_loads_late_is_dropped_not_played() {
        let mut fake = Fake::default();
        let mut audio = audio(&mut fake);
        audio.take_warnings();
        audio.hear(&mut fake, &[play(0)], &[], 10.0);
        audio.play(&mut fake, &[], &[], 10.31);
        assert!(held(&audio).is_empty(), "given up on");
        assert_eq!(audio.abandoned.len(), 1);
        fake.ready.push(path(0));
        audio.play(&mut fake, &[], &[], 10.4);
        assert_eq!(fake.calls(), [format!("load {}", path(0))], "never played");
        assert!(audio.abandoned.is_empty(), "{:?}", audio.abandoned);
        let found = audio.take_warnings();
        assert!(found.is_empty(), "{found:?}");
        // A clock that stepped back doesn't count as late.
        fake.ready.clear();
        audio.hear(&mut fake, &[play(1)], &[], 20.0);
        audio.play(&mut fake, &[], &[], 5.0);
        assert_eq!(held(&audio), [LINES[1]]);
    }

    /// A scene's clips are loaded three ahead; those passed are freed.
    #[test]
    fn preloading_keeps_the_next_three_clips_and_frees_those_passed() {
        let mut fake = all_ready();
        let mut audio = audio(&mut fake);
        audio.hear(&mut fake, &[preload(&[0, 1, 2, 3, 4])], &[], 0.0);
        let loads = |files: &[usize]| -> Vec<String> {
            files.iter().map(|&i| format!("load {}", path(i))).collect()
        };
        assert_eq!(fake.calls(), loads(&[0, 1, 2]));
        assert_eq!(AHEAD, 3);
        // Saying the first line loads the fourth clip; the first is kept
        // while it plays.
        audio.hear(&mut fake, &[play(0)], &[], 1.0);
        let mut expected = loads(&[3]);
        expected.push(format!("play {} 1.00", path(0)));
        assert_eq!(fake.calls(), expected);
        assert_eq!(held(&audio), sorted(&[0, 1, 2, 3]));
        // Skipping to the third line frees the first two.
        audio.hear(&mut fake, &[play(2)], &[], 2.0);
        let mut expected = vec![format!("stop {}", path(0))];
        expected.extend(loads(&[4]));
        expected.push(format!("play {} 1.00", path(2)));
        assert_eq!(fake.calls(), expected);
        assert_eq!(held(&audio), sorted(&[2, 3, 4]));
        // A new scene's list replaces the old one; the voice plays on.
        audio.hear(&mut fake, &[preload(&[1])], &[], 3.0);
        assert_eq!(fake.calls(), loads(&[1]));
        assert_eq!(held(&audio), sorted(&[1, 2]));
    }

    #[test]
    fn a_clip_outside_the_list_is_loaded_when_asked_for() {
        let mut fake = all_ready();
        let mut audio = audio(&mut fake);
        audio.hear(&mut fake, &[preload(&[0, 1]), play(4)], &[], 0.0);
        let calls = fake.calls();
        assert!(
            calls.contains(&format!("play {} 1.00", path(4))),
            "{calls:?}"
        );
        // The list's place is unchanged: its clips stay loaded.
        assert_eq!(held(&audio), sorted(&[0, 1, 4]));
    }

    #[test]
    fn a_clip_is_freed_when_it_ends() {
        let mut fake = all_ready();
        fake.length = Some(1.5);
        let mut audio = audio(&mut fake);
        audio.hear(&mut fake, &[play(0)], &[], 10.0);
        audio.play(&mut fake, &[], &[], 11.49);
        assert_eq!(held(&audio), [LINES[0]]);
        audio.play(&mut fake, &[], &[], 11.5);
        let found = held(&audio);
        assert!(found.is_empty(), "{found:?}");
        // It ended by itself: nothing to stop.
        fake.calls();
        audio.hear(&mut fake, &[Request::StopVoice], &[], 12.0);
        let found = fake.calls();
        assert!(found.is_empty(), "{found:?}");
        // A clip of unknown length is kept until it is stopped.
        fake.length = None;
        audio.hear(&mut fake, &[play(1)], &[], 20.0);
        audio.play(&mut fake, &[], &[], 9_999.0);
        assert_eq!(held(&audio), [LINES[1]]);
    }

    #[test]
    fn a_clip_that_fails_to_load_is_a_warning_and_silence() {
        let mut fake = Fake::default();
        fake.broken.push(path(0));
        let mut audio = audio(&mut fake);
        audio.take_warnings();
        audio.hear(&mut fake, &[play(0)], &[], 0.0);
        assert_eq!(
            audio.take_warnings(),
            [format!("voice clip {}: no such file", path(0))]
        );
        let found = held(&audio);
        assert!(found.is_empty(), "{found:?}");
        audio.play(&mut fake, &[], &[], 0.1);
        assert_eq!(fake.calls(), [format!("load {}", path(0))], "not retried");
        let found = audio.take_warnings();
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn stop_all_stops_the_voice_and_frees_every_clip() {
        let mut fake = all_ready();
        let mut audio = audio(&mut fake);
        audio.hear(&mut fake, &[preload(&[0, 1, 2]), play(0)], &[], 0.0);
        fake.calls();
        audio.stop_all(&mut fake);
        assert_eq!(fake.calls(), [format!("stop {}", path(0))]);
        let found = held(&audio);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn sound_and_music_requests_are_not_voices() {
        let mut fake = all_ready();
        let mut audio = audio(&mut fake);
        audio.hear(&mut fake, &[play(0)], &[], 0.0);
        fake.calls();
        let others = [
            Request::PlaySound {
                cue: "beep".into(),
                volume: 1.0,
            },
            Request::play_music("title", None),
            Request::StopMusic,
        ];
        audio.hear(&mut fake, &others, &[], 0.1);
        assert_eq!(fake.calls(), ["play wav100 0.50"], "the voice plays on");
        assert_eq!(held(&audio), [LINES[0]]);
    }

    /// Acceptance: with no voice folder the game starts and logs nothing
    /// about voices.
    #[test]
    fn without_a_voice_folder_there_is_no_manifest_and_no_warnings() {
        let nowhere = tempfile::tempdir().unwrap();
        let dir = format!("{}/voice/en", nowhere.path().display());
        assert_eq!(block_on(read_manifest(&dir, "voice.ron")), Ok(None));
        // A frame of ordinary audio warns of nothing either.
        let mut fake = Fake::default();
        let mut audio = audio(&mut fake);
        audio.take_warnings();
        audio.set_voice_volume(&mut fake, 1.0);
        audio.play(&mut fake, &[], &[], 0.0);
        let found = audio.take_warnings();
        assert!(found.is_empty(), "{found:?}");
        let found = fake.calls();
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_manifest_that_cannot_be_read_is_an_error_naming_it() {
        // A folder where the file should be.
        let odd = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(odd.path().join("voice.ron")).unwrap();
        let dir = odd.path().display().to_string();
        let error = block_on(read_manifest(&dir, "voice.ron")).unwrap_err();
        let start = format!("{dir}/voice.ron: ");
        assert!(error.starts_with(&start), "{error}");
    }

    #[test]
    fn a_manifest_is_read_whole() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("voices.ron"), "(clips: [])").unwrap();
        let dir = dir.path().display().to_string();
        let text = block_on(read_manifest(&dir, "voices.ron"));
        assert_eq!(text, Ok(Some("(clips: [])".to_owned())));
    }

    #[test]
    fn the_voice_folder_is_the_scripts_language_under_the_voice_folder() {
        let dir = platform_voice_dir("voice", "en");
        assert!(dir.ends_with("voice/en"), "{dir}");
        assert!(!dir.contains('\\'), "{dir}");
    }
}
