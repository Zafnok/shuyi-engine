//! Cinematic files: a list of shots and when each starts, over a length
//! of time (ticket 1113; game ADR-0060). The engine reads the file's
//! frame and checks its times; what a shot shows is the game's own (`S`),
//! checked by the game ([`ShotCheck`]) and played by
//! `shuyi_ui::cinematic`.
//!
//! ```ron
//! (
//!     id: "title",
//!     length: Music("title"),     // or Seconds(10.0)
//!     shots: [
//!         (at: 0.0, shot: Pan(from: (4, 9), to: (12, 3.5))),
//!         (at: 6.0, shot: Logo),
//!     ],
//! )
//! ```
//!
//! - `at` is seconds from the start. A shot lasts until the next one's
//!   `at`; the last until the end. Shots cut: there is no fade.
//! - `length` is `Music(cue)`, the cue's `length_ms` in the audio
//!   manifest (so the file never repeats a number the manifest owns), or
//!   `Seconds(n)` for a cinematic on no track.
//! - `actions` (optional) are lists by name that shots may share: what a
//!   game's actors act out, say. The game says what a list is (`P`) and
//!   checks them with its shots.
//! - Unknown fields are refused. Times are kept in whole milliseconds, so
//!   a checked cinematic holds no floats and can be compared with `Eq`.
use crate::prelude::*;

use alloc::collections::BTreeMap;

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::audio::AudioManifest;
use crate::error::ContentError;
use crate::ron::parse_ron;

/// The longest a cinematic that states its own length may be, in seconds.
pub const MAX_SECONDS: f32 = 3600.0;

/// One cinematic: its shots, in order, over a length of time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CinematicDef<S> {
    /// The cinematic's id (its file stem).
    pub id: String,
    /// The music cue whose length is the cinematic's, if it follows one.
    pub music: Option<String>,
    /// How long it is, in milliseconds (never 0): the music cue's
    /// `length_ms`, or the length the file states.
    pub length_ms: u32,
    /// The shots, by start time; never empty, the first at 0. A shot lasts
    /// until the next one starts, the last until the end.
    pub shots: Vec<ShotDef<S>>,
}

/// A shot and when it starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShotDef<S> {
    /// Milliseconds from the cinematic's start.
    pub at_ms: u32,
    /// What is shown: the game's shot.
    pub shot: S,
}

/// A cinematic file as written, before any check: `S` is a shot and `P`
/// a list of `actions` as the game writes them (`()` for a game whose
/// shots share none).
#[derive(Debug, Deserialize)]
#[serde(
    deny_unknown_fields,
    bound(deserialize = "S: Deserialize<'de>, P: Deserialize<'de>")
)]
pub struct CinematicFile<S, P = ()> {
    /// The cinematic's id; it must be the file's name.
    pub id: String,
    /// How long it is.
    pub length: Length,
    /// Lists by name, for shots to share.
    #[serde(default = "BTreeMap::new")]
    pub actions: BTreeMap<String, P>,
    /// The shots and when each starts.
    pub shots: Vec<TimedShot<S>>,
}

/// How long a cinematic is, as its file says.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Length {
    /// As long as this music cue.
    Music(String),
    /// This many seconds.
    Seconds(f32),
}

/// A shot as written, and when it starts.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, bound(deserialize = "S: Deserialize<'de>"))]
pub struct TimedShot<S> {
    /// Seconds from the start.
    pub at: f32,
    /// The game's shot.
    pub shot: S,
}

/// How a game checks its shots `S`, one file at a time; it may keep what
/// it learns from one shot for the next (a shared list it checked).
pub trait ShotCheck<S> {
    /// The shot as the game keeps it.
    type Shot;

    /// Checks shot `n` (counted from 1), after the frame's checks of its
    /// time, adding each problem to `problems` as a whole message: one of
    /// the shot alone starts `shot {n}: `. Returns the shot as kept, used
    /// only if the file has no problem at all.
    fn check(&mut self, n: usize, shot: &S, problems: &mut Vec<String>) -> Self::Shot;

    /// Adds the problems found once every shot is checked (say, a shared
    /// list that no shot uses). None by default.
    fn finish(&mut self, _problems: &mut Vec<String>) {}
}

/// Reads cinematic `source` (errors attributed to `file`) without
/// checking it: a game whose shots share `actions` makes its
/// [`ShotCheck`] from them, then calls [`check`].
///
/// # Errors
///
/// When `source` isn't such a file, with the place.
pub fn parse<S: DeserializeOwned, P: DeserializeOwned>(
    file: &str,
    source: &str,
) -> Result<CinematicFile<S, P>, ContentError> {
    parse_ron(file, source)
}

/// Reads and checks cinematic `source` ([`parse`], then [`check`]), for a
/// game whose shots share no `actions`.
///
/// # Errors
///
/// Every problem of the file, as [`check`] reports them.
pub fn from_source<S: DeserializeOwned, C: ShotCheck<S>>(
    file: &str,
    stem: &str,
    source: &str,
    audio: &AudioManifest,
    shots: C,
) -> Result<CinematicDef<C::Shot>, Vec<ContentError>> {
    let raw: CinematicFile<S> = parse(file, source).map_err(|e| vec![e])?;
    check(file, stem, &raw, audio, shots)
}

/// Checks cinematic `raw`, read from `file`, whose name without its
/// extension is `stem`, with the music cues of `audio` and the game's
/// checks of its shots. Reports every problem, in order:
///
/// - the id isn't the file name;
/// - the length: a music cue that doesn't exist, or seconds that aren't
///   above 0 and at most [`MAX_SECONDS`];
/// - no shots;
/// - for each shot, a time that isn't one, a first shot not at 0, a shot
///   that doesn't start after the one before it, or not before the end;
///   then the shot's own problems ([`ShotCheck::check`]);
/// - and last, [`ShotCheck::finish`]'s.
///
/// # Errors
///
/// Every problem, each attributed to `file`.
pub fn check<S, P, C: ShotCheck<S>>(
    file: &str,
    stem: &str,
    raw: &CinematicFile<S, P>,
    audio: &AudioManifest,
    mut shots: C,
) -> Result<CinematicDef<C::Shot>, Vec<ContentError>> {
    let mut problems = Vec::new();
    if raw.id != stem {
        problems.push(format!(
            "id \"{}\" must match the file name \"{stem}\"",
            raw.id
        ));
    }
    let (music, length_ms) = match &raw.length {
        Length::Music(cue) => {
            let length = audio.music.get(cue).map(|m| m.length_ms);
            if length.is_none() {
                problems.push(format!("length: no music cue \"{cue}\""));
            }
            (Some(cue.clone()), length)
        }
        Length::Seconds(seconds) => {
            let length = millis(*seconds).filter(|&ms| ms > 0 && *seconds <= MAX_SECONDS);
            if length.is_none() {
                problems.push(format!(
                    "length: {seconds} seconds; it must be above 0 and at most {MAX_SECONDS}"
                ));
            }
            (None, length)
        }
    };
    if raw.shots.is_empty() {
        problems.push("shots: a cinematic needs at least one shot".to_owned());
    }
    let mut kept: Vec<ShotDef<C::Shot>> = Vec::new();
    for (i, TimedShot { at, shot }) in raw.shots.iter().enumerate() {
        let n = i + 1;
        let at_ms = millis(*at);
        match (at_ms, kept.last().map(|s| s.at_ms)) {
            (None, _) => problems.push(format!("shot {n}: at {at} is not a time")),
            (Some(ms), None) if ms != 0 => {
                problems.push(format!("shot {n}: the first shot must be at 0.0, not {at}"));
            }
            (Some(ms), Some(before)) if ms <= before => problems.push(format!(
                "shot {n}: at {at} must be after the shot before it; shots are in order of time"
            )),
            _ => {}
        }
        if let (Some(ms), Some(length)) = (at_ms, length_ms)
            && ms >= length
        {
            problems.push(format!(
                "shot {n}: at {at} is not before the end ({} seconds)",
                seconds(length)
            ));
        }
        let shot = shots.check(n, shot, &mut problems);
        kept.push(ShotDef {
            at_ms: at_ms.unwrap_or(0),
            shot,
        });
    }
    shots.finish(&mut problems);
    match length_ms {
        Some(length_ms) if problems.is_empty() => Ok(CinematicDef {
            id: raw.id.clone(),
            music,
            length_ms,
            shots: kept,
        }),
        _ => Err(problems
            .into_iter()
            .map(|m| ContentError::new(file, m))
            .collect()),
    }
}

/// `seconds` in whole milliseconds, or `None` if it isn't a time: not a
/// number, below 0, or past what a `u32` holds.
pub fn millis(seconds: f32) -> Option<u32> {
    let ms = libm::round(f64::from(seconds) * 1000.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // checked
    (ms.is_finite() && (0.0..=f64::from(u32::MAX)).contains(&ms)).then_some(ms as u32)
}

/// `ms` in seconds, for a message.
pub fn seconds(ms: u32) -> f64 {
    f64::from(ms) / 1000.0
}

#[cfg(test)]
mod tests;
