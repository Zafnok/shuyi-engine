//! The cue rules at run time (ADR-0139): which file each play of a sound
//! cue plays, at what volume, and which plays are dropped or cut, decided
//! once in [`App`](crate::App) before the device, so every backend plays the
//! same.
//!
//! [`Cues`] turns a frame's [`AudioRequest`]s into [`SoundRequest`]s, in
//! order. Each play of a sound goes through its cue's [`CueRules`]: first
//! its gap, then its cue's copy cap, then the manifest's voices; then it
//! picks its file and its volume, and gets its [`CopyId`]. A play dropped
//! draws nothing from the variant picker, so the plays that sound are the
//! same with it or without it. Time is the frame's, in whole milliseconds,
//! and the picker is a `SplitMix64` of its own, apart from the simulation's
//! RNG: what plays never changes the game, and the same seed and requests
//! give the same plays on every port.
//!
//! **Copies.** A play that sounds is a copy of its cue until its start plus
//! its file's length ([`SoundCue::length_ms`]); one whose length is unknown
//! ends at once, so it never holds a copy or a voice. A copy cut to make
//! room is stopped alone ([`SoundRequest::StopCopy`], before the new play)
//! and reported as [`DropWhy::Cut`]. Music, voice clips and sounds started
//! with a handle don't count.

use alloc::collections::BTreeMap;

use crate::alloc_prelude::*;

use shuyi_assets::audio::{AudioManifest, CueRules, Pick, SoundCue, WhenFull};
use shuyi_assets::voice::ClipVariant;
use shuyi_ui::audio::{AudioRequest, splitmix64};

use crate::audio::SoundRequest;
use crate::platform::Millis;

/// Mixed into the seed, so the picker doesn't repeat the music pool's
/// picks from the same seed ("cues-rng" in ASCII).
const SALT: u64 = 0x6375_6573_2D72_6E67;

/// The cue rules' state: what each cue played last, the copies sounding,
/// and the picker.
#[derive(Debug)]
pub struct Cues {
    rng: VariantRng,
    /// Each cue that has played, by id.
    cues: BTreeMap<String, Played>,
    /// The copies sounding, oldest first.
    sounding: Vec<Sounding>,
    /// The last copy's id.
    last_copy: u64,
    /// The latest frame's time: a clock that goes backwards counts as no
    /// time passing for the copies.
    clock: Millis,
}

/// One play of a sound cue that sounded, as the cue rules count it
/// ([`SoundRequest::Sound`]'s `copy`): a backend stops it alone with
/// [`SoundRequest::StopCopy`]. Ids rise from 1 in the order the plays
/// sounded, so a lower one is older. A plain value (ADR-0039).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, serde::Serialize)]
pub struct CopyId(u64);

impl CopyId {
    /// No copy: a sound no [`Cues`] counted, such as one built with
    /// [`SoundRequest::sound`] alone. A backend keeps nothing for it.
    pub const NONE: Self = Self(0);

    /// The copy numbered `n` (0 is [`NONE`](Self::NONE)).
    pub const fn new(n: u64) -> Self {
        Self(n)
    }

    /// Its number.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A play the cue rules dropped or cut: its cue and why. A view (ADR-0039,
/// kind 3): read its fields.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub struct DroppedSound {
    /// The sound cue: for [`DropWhy::Cut`], the cut copy's.
    pub cue: String,
    /// Why it was dropped.
    pub why: DropWhy,
}

/// Why a play was dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub enum DropWhy {
    /// Less than its cue's `min_gap_ms` after the cue's last play that
    /// sounded.
    Gap,
    /// Its cue's `max_copies` copies sounded, and its `when_full` is
    /// `Skip`.
    Copies,
    /// The manifest's `max_voices` copies sounded, each of a higher
    /// priority than its own.
    Voices,
    /// A copy that was sounding, cut to make room for a new play: by its
    /// cue's `StopOldest`, or by the voice rule.
    Cut,
}

impl DropWhy {
    /// Its name in the tool contract's answers: `gap`, `copies`, `voices`
    /// or `cut`.
    pub const fn word(self) -> &'static str {
        match self {
            Self::Gap => "gap",
            Self::Copies => "copies",
            Self::Voices => "voices",
            Self::Cut => "cut",
        }
    }
}

/// A copy sounding.
#[derive(Debug)]
struct Sounding {
    copy: CopyId,
    cue: String,
    priority: u8,
    started: Millis,
    /// When its file's length has passed.
    ends: Millis,
}

/// What one cue has played.
#[derive(Debug, Default)]
struct Played {
    /// The file it played last.
    last: Option<usize>,
    /// The shuffle's order, and how far through it the plays are.
    order: Vec<usize>,
    at: usize,
    /// When it last played and sounded, for its gap.
    sounded: Option<Millis>,
}

impl Played {
    /// The file a play of a cue of `n` files (`n` ≥ 1) picks by `pick`.
    fn pick(&mut self, pick: Pick, n: usize, rng: &mut VariantRng) -> usize {
        let last = self.last.filter(|&last| last < n);
        let file = if n == 1 {
            0
        } else {
            match pick {
                Pick::NoRepeat => match last {
                    // One of the others, each as likely.
                    Some(last) => {
                        let other = rng.below(n - 1);
                        if other >= last { other + 1 } else { other }
                    }
                    None => rng.below(n),
                },
                Pick::Shuffle => {
                    if self.at >= self.order.len() || self.order.len() != n {
                        self.deal(n, last, rng);
                    }
                    let file = self.order.get(self.at).copied().unwrap_or(0);
                    self.at += 1;
                    file
                }
                Pick::InOrder => last.map_or(0, |last| (last + 1) % n),
                // `Random`, and any pick a later engine adds.
                _ => rng.below(n),
            }
        };
        self.last = Some(file);
        file
    }

    /// A new shuffle of `n` files (`n` ≥ 2), never starting with `last`.
    fn deal(&mut self, n: usize, last: Option<usize>, rng: &mut VariantRng) {
        self.order.clear();
        self.order.extend(0..n);
        for i in (1..n).rev() {
            let j = rng.below(i + 1);
            self.order.swap(i, j);
        }
        if self.order.first().is_some_and(|&first| Some(first) == last) {
            let other = 1 + rng.below(n - 1);
            self.order.swap(0, other);
        }
        self.at = 0;
    }
}

/// Where a frame's requests go: the device's, and the drops.
struct Out<'a> {
    requests: &'a mut Vec<SoundRequest>,
    dropped: &'a mut Vec<DroppedSound>,
}

impl Cues {
    /// No cue played yet, the picker seeded from `seed` (the game's
    /// `ctx.music_seed`).
    pub fn new(seed: u64) -> Self {
        Self {
            rng: VariantRng::new(splitmix64(seed ^ SALT)),
            cues: BTreeMap::new(),
            sounding: Vec::new(),
            last_copy: 0,
            clock: Millis(0),
        }
    }

    /// Turns `asked`, one frame's requests at `now`, into what the device
    /// gets, in order, added to `out`: each sound played or started with
    /// its file and volume picked by its cue's rules in `manifest`, every
    /// other request as [`SoundRequest::from_request`] makes it. A play its
    /// rules drop goes to `dropped` instead, and so does each copy cut to
    /// make room for one, after its [`SoundRequest::StopCopy`] in `out`.
    ///
    /// A cue the manifest lacks, or one with no files, goes through with
    /// file 0 at volume 0 (the device warns of it, or plays a sound the
    /// game added as it is), and holds no copy. A sound started with a
    /// handle is never dropped, starts no gap and holds no copy or voice:
    /// the game stops it itself.
    pub fn apply<V: ClipVariant>(
        &mut self,
        manifest: &AudioManifest,
        now: Millis,
        asked: &[AudioRequest<V>],
        out: &mut Vec<SoundRequest>,
        dropped: &mut Vec<DroppedSound>,
    ) {
        self.clock = self.clock.max(now);
        let clock = self.clock;
        self.sounding.retain(|copy| copy.ends > clock);
        let mut out = Out {
            requests: out,
            dropped,
        };
        for request in asked {
            match request {
                AudioRequest::PlaySound { cue, volume, .. } => {
                    self.play(manifest, now, cue, *volume, &mut out);
                }
                AudioRequest::StartSound { id, cue, volume } => {
                    let def = manifest.sounds.get(cue.as_str());
                    let (file, level) = self.draw(def, cue);
                    let start = SoundRequest::start(*id, cue.clone(), *volume);
                    out.requests.push(start.with_pick(file, level));
                }
                other => out.requests.extend(SoundRequest::from_request(other)),
            }
        }
    }

    /// One play of `cue` at `now`: dropped, or sounding (after the copies
    /// cut to make room for it).
    fn play(
        &mut self,
        manifest: &AudioManifest,
        now: Millis,
        cue: &str,
        volume: f32,
        out: &mut Out<'_>,
    ) {
        let def = manifest.sounds.get(cue).filter(|def| !def.files.is_empty());
        let drop = |out: &mut Out<'_>, why| {
            out.dropped.push(DroppedSound {
                cue: cue.to_owned(),
                why,
            });
        };
        if let Some(def) = def {
            let sounded = self.cues.get(cue).and_then(|played| played.sounded);
            if too_soon(def.rules, sounded, now) {
                return drop(out, DropWhy::Gap);
            }
            if let Err(why) = self.make_room(cue, def.rules, manifest.max_voices, out) {
                return drop(out, why);
            }
        }
        let (file, level) = self.draw(def, cue);
        self.last_copy += 1;
        let copy = CopyId(self.last_copy);
        if let Some(def) = def {
            if let Some(played) = self.cues.get_mut(cue) {
                played.sounded = Some(now);
            }
            if let Some(length) = def.length_ms(file).filter(|&ms| ms > 0) {
                self.sounding.push(Sounding {
                    copy,
                    cue: cue.to_owned(),
                    priority: def.rules.priority,
                    started: self.clock,
                    ends: Millis(self.clock.0.saturating_add(u64::from(length))),
                });
            }
        }
        let sound = SoundRequest::sound(cue, volume).with_pick(file, level);
        out.requests.push(sound.with_copy(copy));
    }

    /// Makes room for a play of `cue` by `rules` under `max_voices`, cutting
    /// a copy if its rules say so, or says why it is dropped.
    fn make_room(
        &mut self,
        cue: &str,
        rules: CueRules,
        max_voices: Option<u16>,
        out: &mut Out<'_>,
    ) -> Result<(), DropWhy> {
        if let Some(max) = rules.max_copies {
            let copies = self.sounding.iter().filter(|copy| copy.cue == cue).count();
            if copies >= usize::from(max) {
                let oldest = self
                    .sounding
                    .iter()
                    .position(|copy| copy.cue == cue)
                    .filter(|_| rules.when_full == WhenFull::StopOldest);
                // `Skip`, a rule a later engine adds, and a cap of 0
                // (refused in a file) drop it.
                let oldest = oldest.ok_or(DropWhy::Copies)?;
                self.cut(oldest, out);
            }
        }
        if let Some(max) = max_voices
            && self.sounding.len() >= usize::from(max)
        {
            // The lowest priority, the oldest of those, ties to the lower id.
            let lowest = self
                .sounding
                .iter()
                .enumerate()
                .min_by_key(|(_, copy)| (copy.priority, copy.started, copy.copy))
                .filter(|(_, copy)| copy.priority <= rules.priority)
                .map(|(at, _)| at);
            self.cut(lowest.ok_or(DropWhy::Voices)?, out);
        }
        Ok(())
    }

    /// Cuts the copy sounding at `at`: stopped alone, and reported.
    fn cut(&mut self, at: usize, out: &mut Out<'_>) {
        let copy = self.sounding.remove(at);
        out.requests
            .push(SoundRequest::StopCopy { copy: copy.copy });
        out.dropped.push(DroppedSound {
            cue: copy.cue,
            why: DropWhy::Cut,
        });
    }

    /// The file and volume a play of `cue`, `def` in the manifest, picks:
    /// file 0 at volume 0 for a cue the manifest lacks or with no files.
    fn draw(&mut self, def: Option<&SoundCue>, cue: &str) -> (usize, u8) {
        let Some(def) = def.filter(|def| !def.files.is_empty()) else {
            return (0, 0);
        };
        let rules = def.rules;
        if !self.cues.contains_key(cue) {
            self.cues.insert(cue.to_owned(), Played::default());
        }
        let Some(played) = self.cues.get_mut(cue) else {
            return (0, 0);
        };
        let file = played.pick(rules.pick, def.files.len(), &mut self.rng);
        let level = volume(def.volume, rules.volume_min, &mut self.rng);
        (file, level)
    }
}

/// Whether a play at `now` is inside the gap `rules` leave after the play
/// `sounded` last (none with a gap of 0): a clock that goes backwards
/// counts as no time passing.
fn too_soon(rules: CueRules, sounded: Option<Millis>, now: Millis) -> bool {
    let gap = u64::from(rules.min_gap_ms);
    sounded.is_some_and(|at| now.0.saturating_sub(at.0) < gap)
}

/// A play's volume in percent: a whole percent from `min` to `volume`, or
/// `volume` with no `min` or one not under it (a manifest built in code).
fn volume(volume: u8, min: Option<u8>, rng: &mut VariantRng) -> u8 {
    match min {
        Some(min) if min < volume => {
            let above = rng.below(usize::from(volume - min) + 1);
            min.saturating_add(u8::try_from(above).unwrap_or(0))
        }
        _ => volume,
    }
}

/// Picks sound variants and volumes (`audio.md`: footsteps vary each step).
/// A small `SplitMix64`, separate from core's simulation RNG (game
/// ADR-0019): what plays never affects the game.
#[derive(Debug)]
struct VariantRng(u64);

impl VariantRng {
    const fn new(seed: u64) -> Self {
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
    fn below(&mut self, n: usize) -> usize {
        let n = u64::try_from(n.max(1)).unwrap_or(u64::MAX);
        usize::try_from(self.next_u64() % n).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests;
