//! Ducking (ADR-0099): a sound group lowered while a voice plays, as the
//! audio manifest's `ducks` rules say ([`DuckRule`]), so the words are
//! heard over the music. Each rule's multiplier moves in a straight line,
//! in whole milliseconds and integers only, down to its rule's
//! [`loudness`] while a voice sounds and back to [`Gain::FULL`] once it
//! doesn't; [`GroupGains`](super::GroupGains) multiplies the lowered
//! group's own gain by it, so its children go down too.

use crate::prelude::*;

use shuyi_assets::audio::DuckRule;

use super::loudness::{Gain, loudness};

/// One rule's multiplier, and where it is going.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Duck {
    /// The group it lowers.
    lower: String,
    /// The multiplier while a voice sounds: the rule's `loudness(to)`.
    floor: Gain,
    attack_ms: u32,
    release_ms: u32,
    /// Where it set off from, where it is going, and the whole
    /// milliseconds since it set off.
    from: Gain,
    target: Gain,
    elapsed_ms: u64,
    /// The multiplier now.
    now: Gain,
}

impl Duck {
    /// Where it is `elapsed_ms` after it set off: a pure function of
    /// `from`, `target`, the speed and the time, so the frames' sizes add
    /// no rounding. The speed is the full distance (full minus the floor)
    /// over the attack going down and over the release coming back, so a
    /// turn halfway down is back in half the release.
    fn at(&self) -> Gain {
        // It goes only to the floor or back to full.
        let going_down = self.target != Gain::FULL;
        let length = if going_down {
            self.attack_ms
        } else {
            self.release_ms
        };
        let distance = u64::from(Gain::FULL.get() - self.floor.get());
        if length == 0 || distance == 0 {
            return self.target;
        }
        let length = u64::from(length);
        let moved = distance
            .saturating_mul(self.elapsed_ms)
            .saturating_add(length / 2)
            / length;
        let from = u64::from(self.from.get());
        let at = if going_down {
            from.saturating_sub(moved).max(u64::from(self.target.get()))
        } else {
            from.saturating_add(moved).min(u64::from(self.target.get()))
        };
        // Between from and target, so at most a full gain.
        Gain::new(u16::try_from(at).unwrap_or(Gain::UNIT))
    }
}

/// Every ducking rule's multiplier now: what [`Game`](crate::Game) steps
/// each frame and folds into the groups' gains. With no rule, nothing is
/// lowered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DuckState {
    ducks: Vec<Duck>,
}

impl DuckState {
    /// The state of `rules`, each at [`Gain::FULL`]: nothing lowered yet.
    pub fn new(rules: &[DuckRule]) -> Self {
        let ducks = rules
            .iter()
            .map(|rule| Duck {
                lower: rule.lower.clone(),
                floor: loudness(rule.to),
                attack_ms: rule.attack_ms,
                release_ms: rule.release_ms,
                from: Gain::FULL,
                target: Gain::FULL,
                elapsed_ms: 0,
                now: Gain::FULL,
            })
            .collect();
        Self { ducks }
    }

    /// Moves every multiplier on by `dt_ms` whole milliseconds: down to its
    /// floor while a voice is `sounding`, back up to full while not. A
    /// change of direction sets off from where it is.
    pub fn update(&mut self, dt_ms: u64, sounding: bool) {
        for duck in &mut self.ducks {
            let target = if sounding { duck.floor } else { Gain::FULL };
            if target != duck.target {
                duck.from = duck.now;
                duck.target = target;
                duck.elapsed_ms = 0;
            }
            duck.elapsed_ms = duck.elapsed_ms.saturating_add(dt_ms);
            duck.now = duck.at();
        }
    }

    /// The multiplier of group `group` now: [`Gain::FULL`] for a group no
    /// rule lowers.
    pub fn multiplier(&self, group: &str) -> Gain {
        self.ducks
            .iter()
            .find(|duck| duck.lower == group)
            .map_or(Gain::FULL, |duck| duck.now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(lower: &str, to: u8, attack_ms: u32, release_ms: u32) -> DuckRule {
        let mut rule = DuckRule::default();
        rule.when = "voice".into();
        rule.lower = lower.into();
        rule.to = to;
        rule.attack_ms = attack_ms;
        rule.release_ms = release_ms;
        rule
    }

    /// The music's multiplier after each of `frames` (in milliseconds),
    /// a voice sounding or not.
    fn run(state: &mut DuckState, frames: &[u64], sounding: bool) -> Vec<u16> {
        frames
            .iter()
            .map(|&dt| {
                state.update(dt, sounding);
                state.multiplier("music").get()
            })
            .collect()
    }

    #[test]
    fn with_no_rule_nothing_is_lowered() {
        let mut state = DuckState::new(&[]);
        state.update(16, true);
        assert_eq!(state.multiplier("music"), Gain::FULL);
    }

    #[test]
    fn it_goes_down_in_a_straight_line_to_the_floor_and_no_further() {
        let mut state = DuckState::new(&[rule("music", 80, 100, 500)]);
        assert_eq!(loudness(80).get(), 1029);
        // Halfway down at 50 ms: 4096 − 3067 / 2.
        let half = run(&mut state, &[50], true);
        assert!(half[0].abs_diff(4096 - 3067 / 2) <= 1, "{half:?}");
        let mut state = DuckState::new(&[rule("music", 80, 100, 500)]);
        // 16, 17, 16, 17, 16, 17 ms: 99 ms, then 116.
        let frames = run(&mut state, &[16, 17, 16, 17, 16, 17, 16, 17, 16], true);
        assert!(
            frames.windows(2).all(|w| w[0] > w[1] || w[1] == 1029),
            "{frames:?}"
        );
        assert!(frames[5] > 1029, "not there at 99 ms: {frames:?}");
        assert_eq!(frames[6], 1029, "there once 100 ms have passed");
        assert!(frames.iter().all(|&g| g >= 1029), "{frames:?}");
        assert_eq!(
            state.multiplier("sfx"),
            Gain::FULL,
            "a group no rule lowers"
        );
    }

    #[test]
    fn the_frames_sizes_add_no_rounding() {
        let mut big = DuckState::new(&[rule("music", 80, 100, 500)]);
        let mut small = big.clone();
        run(&mut big, &[33, 33], true);
        run(&mut small, &[1; 66], true);
        assert_eq!(big, small);
    }

    #[test]
    fn it_comes_back_over_the_release() {
        let mut state = DuckState::new(&[rule("music", 80, 100, 500)]);
        run(&mut state, &[200], true);
        let up = run(&mut state, &[250, 249, 1, 100], false);
        assert!(up[0].abs_diff(1029 + 3067 / 2) <= 1, "{up:?}");
        assert_eq!(up[1..], [4090, 4096, 4096], "499 ms is 3061 of the way");
    }

    #[test]
    fn a_voice_that_stops_halfway_down_is_back_in_half_the_release() {
        let mut state = DuckState::new(&[rule("music", 80, 100, 500)]);
        run(&mut state, &[50], true);
        let up = run(&mut state, &[249, 1], false);
        assert!(up[0] < 4096, "{up:?}");
        assert_eq!(up[1], 4096);
        // And a voice again halfway up turns it down from there.
        let mut state = DuckState::new(&[rule("music", 80, 100, 500)]);
        run(&mut state, &[100], true);
        let halfway_up = run(&mut state, &[250], false)[0];
        let down = run(&mut state, &[25], true)[0];
        assert!(
            down.abs_diff(halfway_up - 3067 / 4) <= 1,
            "{halfway_up} {down}"
        );
    }

    #[test]
    fn zero_lengths_are_at_once_and_to_100_moves_nothing() {
        let mut state = DuckState::new(&[rule("music", 80, 0, 0)]);
        assert_eq!(run(&mut state, &[16], true), [1029]);
        assert_eq!(run(&mut state, &[0], false), [4096]);
        let mut state = DuckState::new(&[rule("music", 100, 100, 500)]);
        assert_eq!(run(&mut state, &[16, 1000], true), [4096, 4096]);
        assert_eq!(run(&mut state, &[16], false), [4096]);
        let mut state = DuckState::new(&[rule("music", 0, 10, 10)]);
        assert_eq!(run(&mut state, &[10], true), [0]);
    }
}
