//! The cue rules at run time (ADR-0139): which file each play of a sound
//! cue plays, at what volume, and which plays are dropped, decided once in
//! [`App`](crate::App) before the device, so every backend plays the same.
//!
//! [`Cues`] turns a frame's [`AudioRequest`]s into [`SoundRequest`]s, in
//! order. Each play of a sound goes through its cue's [`CueRules`]: first
//! its gap, then its file and its volume. A play dropped draws nothing from
//! the variant picker, so the plays that sound are the same with it or
//! without it. Time is the frame's, in whole milliseconds, and the picker is
//! a `SplitMix64` of its own, apart from the simulation's RNG: what plays
//! never changes the game, and the same seed and requests give the same
//! plays on every port.

use alloc::collections::BTreeMap;

use crate::alloc_prelude::*;

use shuyi_assets::audio::{AudioManifest, CueRules, Pick};
use shuyi_assets::voice::ClipVariant;
use shuyi_ui::audio::{AudioRequest, splitmix64};

use crate::audio::SoundRequest;
use crate::platform::Millis;

/// Mixed into the seed, so the picker doesn't repeat the music pool's
/// picks from the same seed ("cues-rng" in ASCII).
const SALT: u64 = 0x6375_6573_2D72_6E67;

/// The cue rules' state: what each cue played last, and the picker.
#[derive(Debug)]
pub struct Cues {
    rng: VariantRng,
    /// Each cue that has played, by id.
    cues: BTreeMap<String, Played>,
}

/// A play the cue rules dropped: its cue and why. A view (ADR-0039, kind
/// 3): read its fields.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub struct DroppedSound {
    /// The sound cue.
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
}

impl DropWhy {
    /// Its name in the tool contract's answers: `gap`.
    pub const fn word(self) -> &'static str {
        match self {
            Self::Gap => "gap",
        }
    }
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

impl Cues {
    /// No cue played yet, the picker seeded from `seed` (the game's
    /// `ctx.music_seed`).
    pub fn new(seed: u64) -> Self {
        Self {
            rng: VariantRng::new(splitmix64(seed ^ SALT)),
            cues: BTreeMap::new(),
        }
    }

    /// Turns `asked`, one frame's requests at `now`, into what the device
    /// gets, in order, added to `out`: each sound played or started with
    /// its file and volume picked by its cue's rules in `manifest`, every
    /// other request as [`SoundRequest::from_request`] makes it. A play its
    /// rules drop goes to `dropped` instead.
    ///
    /// A cue the manifest lacks, or one with no files, goes through with
    /// file 0 at volume 0 (the device warns of it, or plays a sound the
    /// game added as it is). A sound started with a handle is never
    /// dropped and starts no gap: the game stops it itself.
    pub fn apply<V: ClipVariant>(
        &mut self,
        manifest: &AudioManifest,
        now: Millis,
        asked: &[AudioRequest<V>],
        out: &mut Vec<SoundRequest>,
        dropped: &mut Vec<DroppedSound>,
    ) {
        for request in asked {
            match request {
                AudioRequest::PlaySound { cue, volume, .. } => {
                    match self.pick(manifest, now, cue, true) {
                        Ok((file, level)) => {
                            out.push(
                                SoundRequest::sound(cue.clone(), *volume).with_pick(file, level),
                            );
                        }
                        Err(why) => dropped.push(DroppedSound {
                            cue: cue.clone(),
                            why,
                        }),
                    }
                }
                AudioRequest::StartSound { id, cue, volume } => {
                    let (file, level) = self.pick(manifest, now, cue, false).unwrap_or((0, 0));
                    out.push(SoundRequest::start(*id, cue.clone(), *volume).with_pick(file, level));
                }
                other => out.extend(SoundRequest::from_request(other)),
            }
        }
    }

    /// The file and volume of a play of `cue` at `now`, or why it is
    /// dropped; a play `gapped` is held to the cue's gap and starts it.
    fn pick(
        &mut self,
        manifest: &AudioManifest,
        now: Millis,
        cue: &str,
        gapped: bool,
    ) -> Result<(usize, u8), DropWhy> {
        let Some(def) = manifest.sounds.get(cue).filter(|def| !def.files.is_empty()) else {
            return Ok((0, 0));
        };
        let rules = def.rules;
        if !self.cues.contains_key(cue) {
            self.cues.insert(cue.to_owned(), Played::default());
        }
        let Some(played) = self.cues.get_mut(cue) else {
            return Ok((0, 0));
        };
        if gapped && too_soon(rules, played.sounded, now) {
            return Err(DropWhy::Gap);
        }
        let file = played.pick(rules.pick, def.files.len(), &mut self.rng);
        let level = volume(def.volume, rules.volume_min, &mut self.rng);
        if gapped {
            played.sounded = Some(now);
        }
        Ok((file, level))
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
