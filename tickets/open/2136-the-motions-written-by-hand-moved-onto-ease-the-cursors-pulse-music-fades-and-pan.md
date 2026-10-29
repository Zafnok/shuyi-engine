---
id: "2136"
title: "The motions written by hand moved onto shuyi_core::ease: the grid cursor's pulse, music fades and Pan, with no snapshot moving, and why the rest stay"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["2109"]
nick_input: none
completed:
---

# 2136 — The motions written by hand moved onto ease

## Context

2109 adds integer ease curves, `Lerp` and `Tween` in
`shuyi_core::ease`, and `shuyi_ui::ease::ease_f32` for code that works
in floats. It changes none of the engine's own motions. Its Context
lists them as "motions written by hand", and its Scope promised the
follow-up that moves them. This is that follow-up. 2364 (parked)
leaves moving `Pan` onto 2109's ease to it, and 2339 leaves it its
private smoothstep if 2339 lands before 2109.

Two rules of 2109's ADR ("Easing and tweens: integer curves in
`shuyi-core`, drawn from the time alone") apply here:

- a tween's time is a plain `u32` whose unit the caller picks;
- a skin that works in floats eases through `ease_f32`, which goes
  through the integer curve, so the curve is the same on every machine.

What is written by hand today (read on 2026-10-24):

- **The grid cursor's pulse** (`crates/ui/src/grid/cursor.rs`):
  `Cursor::brightness` is `pulse(blink_t, BLINK_PERIOD)`, which is
  `BLINK_MIN + (1 - BLINK_MIN) * (1 + cos(TAU * t / period)) / 2`, with
  `blink_t` in `f32` seconds. The cosine is `std`'s, or
  `crate::float`'s without `std`, so its last bits may differ between
  targets. `draw_cursor` (`crates/ui/src/grid/glyph/cursor.rs`) fades
  the corner marks and the tile glow by it.
- **Music fades** (`MusicState::update`, `crates/ui/src/audio.rs`):
  since 2341 each track's gain is a `Ramp`, a straight line from the
  gain it had (`from`) toward 0 or 1 at `elapsed / secs`, where
  `elapsed` sums the frames' `dt` in `f32` seconds and `secs` is the
  switch's `out_ms` or `in_ms` over 1000. A whole fade-out is today's
  `1 - elapsed / secs`. `Game::with_stack` (`crates/ui/src/game.rs`)
  builds the state from `AudioManifest::music_switch()`, in whole
  milliseconds, with no float cast.
- **`Pan::centre`** (`crates/ui/src/cinematic.rs`): eased by
  `ease_in_out`, the `f32` smoothstep. 2109 keeps `ease_in_out` as it
  is, so the game's own calls of it draw as today.
- If 2339 landed before 2109: its
  `shuyi_ui::frame::transition::progress`, a smoothstep in thousandths
  written out by hand.

Three more are on 2109's list, and stay as they are. This ticket writes
down why, so nobody moves them later by mistake:

- **`Rgb::lerp(self, other, t: f32)`** (`crates/ui/src/frame/color.rs`)
  mixes every colour the engine draws: overlays, the cursor, fades.
  2109's `Lerp for Rgb` is its twin in integers. The two agree at every
  `t` that is a whole number of 1/256, where both are exact. They
  don't agree at every `t`. At `GLOW_MAX * 0.5` (0.15, the tile glow
  at its dimmest), a channel 10 apart mixes to 2 in `f32` and to 1
  through `Lerp`: `0.15_f32` is just above 0.15, and 0.15 in 1/65536
  rounds just below. Moving it would change colours in a game's
  snapshots.
- **`TextBox::type_on(dt, chars_per_s)`**
  (`crates/ui/src/dialogue/text_box.rs`) is a rate, not a curve. The
  player's text speed may change in the middle of a page, and holding
  the key speeds it up (`HurryClock::passing`,
  `crates/ui/src/hurry.rs`). A tween runs from one value to another in
  a set time, which doesn't fit.
- **Idle and walk frames** (`crates/ui/src/grid/walk.rs`) are frame
  cycles, which are clips: 2231's `frame_at` and 2359's `UnitClips`
  (both parked). 2359 keeps `gait`'s arithmetic so that its default
  clips give exactly today's frames. A move here would change what it
  matches.

**Why it matters.** The engine's own motions are the code an agent
reads before it writes its own (the end goal of 2026-10-19,
`docs/decisions.md`). With this ticket they use 2109's names. The
cursor's pulse and a pan become the same curve on every target, as
2109's ADR asks of code in floats. A music fade becomes a `Tween` over
whole milliseconds, the unit the audio manifest already uses, and
2341's fade-ins can be written the same way.

## Nick input

None.

## Scope

**In:**
- **The rule for every move:**
  - every public signature stays: `Cursor::blink_t`,
    `Cursor::brightness() -> f32`, `BLINK_PERIOD`, `BLINK_MIN`,
    `MusicState::new(fade_secs: f32)`, `Pan`'s `f32` fields and
    `Pan::centre(progress: f32)`;
  - time is summed as it is today (the frames' `dt` in `f32` seconds)
    and turned into the tween's unit only where the tween reads it.
    Rounding each frame's `dt` would drift;
  - each result stays within its bound (below) of today's value.
    Every existing test keeps its expected values. A test that checks
    a float more tightly than the curves can (`1e-6`, where the curves
    are within 2/65536) may widen to the bound below, and the PR lists
    each one it widened;
  - no snapshot in the repository moves: `insta` text and picture
    snapshots, `frame-png` ones and Harness tests that compare frames.
    If moving a motion would move one, that motion stays as it is, and
    the Completion notes name the snapshot.
- **The grid cursor's pulse** (`crates/ui/src/grid/cursor.rs`):
  - `pulse(t, period)` reads a looping, yoyo `Tween<i32>` from
    `shuyi_core::ease::ONE` down to `BLINK_MIN` in 1/65536, with
    `Ease::InOut(Family::Sine)`, over half a period. Its time is in
    1/65536 of a period: `t` is first brought into one period (`%`,
    plus a period if below 0; infinity and not a number count as 0),
    then scaled and rounded. The value goes back to `f32` as
    `value as f32 / ONE as f32`;
  - so `pulse(0.0, 1.0)` and `pulse(1.0, 1.0)` are exactly 1 and
    `pulse(0.5, 1.0)` exactly 0.5, as the tests have them today;
  - bound: within 4/65536 of today's `pulse` at any `t`;
  - with no cosine left, the `crate::float::Float` import for `no_std`
    goes if nothing else in the file uses it;
  - the doc comments of `brightness` and `pulse` name the tween.
- **Music fades** (`MusicState`, `crates/ui/src/audio.rs`):
  - `MusicSwitch` holds the fade lengths in whole milliseconds and
    `MusicState::with_switch` takes it (2341), so there is no
    `with_fade_ms`; `new(fade_secs)` keeps rounding to the millisecond;
  - while a track fades out, its gain is a `Tween<i32>` from `ONE` to
    0, `Linear`, over `fade_ms`, read at `millis(elapsed)` and sent as
    `value as f32 / ONE as f32`;
  - the fade ends on the frame it ends on today: `Stop` comes once
    `elapsed` reaches `fade_ms as f32 / 1000.0`, the same number
    `Game::with_stack` computes today from the manifest's whole
    milliseconds;
  - bound: each gain within half a millisecond of the fade, plus
    1/65536, of today's.
- **`Pan::centre`** (`crates/ui/src/cinematic.rs`): eases through
  `shuyi_ui::ease::ease_f32(&Ease::Smooth, progress)`. A progress that
  isn't finite first becomes 0, as `ease_in_out` has it (`ease_f32`
  would clamp positive infinity to 1). The positions still mix in
  `f32`. `ease_in_out` keeps its body, as 2109 decided. `Pan`'s doc
  comment names `Ease::Smooth` in place of `ease_in_out`. Bound: within
  3/65536 of the distance from `from` to `to`, plus `f32` rounding, of
  today's centre.
- **The three that stay, written down:**
  - `Rgb::lerp`'s doc comment names `Lerp for Rgb` as its twin in
    integers and says where the two agree. A test checks that they
    agree at every `t` that is a whole number of 1/256;
  - `type_on`'s doc comment says it is a rate, and that a game that
    wants a reveal eased over a set time tweens the count of
    characters itself;
  - the Completion notes say why each of the three stays (the Context
    above).
- **Either order with 2339:** if 2339 is done and its
  `transition::progress` still has its private formula
  (`p² × (3000 − 2p) / 1 000 000` in thousandths, which it has if 2339
  landed before 2109), move it onto `Ease::Smooth` through
  `Easing::ease`, scaled from and to thousandths, as 2339's clause on
  2109 says (if it uses `Ease::Smooth` already, leave it); if not,
  nothing to do: 2109 is done before this ticket, so 2339 takes
  `Ease::Smooth` when it is built (its clause on 2109 says so).
- **From 2341 (done):** `MusicState` has fade-ins, overlaps and a
  `MusicSwitch` in whole milliseconds, each fade a `Ramp`: move every
  gain it computes (the fade-out, the fade-in, an overlap's two, and a
  fade from the gain a track had reached) onto `Tween`s by the rule
  above, its tests unchanged.
- **Either order with 2364** (parked; keyframed tracks in the cinematic
  file): if 2364 is done, its tracks already ease through
  `Easing::ease`, and `Pan` is still today's (its Out leaves `Pan` to
  this ticket): move `Pan` as above and nothing of 2364's; if not,
  nothing to do: 2364 leaves `Pan` alone when it is built.
- **What changes for the game:** no API break, so no `Breaking:` line.
  Nothing public changes shape. The
  cursor's brightness and a pan's centre may differ from today's in
  their last bits, within the bounds above. Where that falls on a
  rounding edge, one channel of a colour or a tenth of a pixel in a
  snapshot can change, rarely. The game's PR that moves its pin runs
  its snapshots and says if one moved.

**Out (do not do):**
- `Sequence` and `Points` (2135, 2109's other follow-up).
- Moving `Rgb::lerp`, `type_on`, or the idle and walk frames: the
  Context says why. Frame cycles as clips are 2231's and 2359's (both
  parked).
- `ease_in_out`'s body: 2109 keeps it, so the game's own calls of it
  draw as today.
- Public time in whole milliseconds or in integers (`blink_t`,
  `BLINK_PERIOD`, `new(fade_secs)`, `Pan`'s fields): each would break
  the game (a follow-up if a game on a target with no floats asks).
- Blinks that are on or off, with no curve in them: the text box's
  marker (`marker_lit`, `BLINK_S`).
- `Rgb::scale`, `Rgb::dimmed` and `GlyphBuffer::fade`: a brightness
  the caller passes, not a motion over time.
- Music fades along a curve other than a straight line: 2341's Out
  leaves that to a follow-up if a game hears the difference.
- Motions other tickets add (2340's shake and flash, 2343's ducking):
  they are written in their own tickets' terms, and 2109's Out says
  they may ease with it.
- An ADR: 2109's ADR covers this.

## Implementation steps

1. Read 2109's ADR, `crates/core/src/ease.rs` (`ONE`, `Ease`,
   `Family`, `Easing`, `Lerp`, `Tween`, `Repeat`) and
   `crates/ui/src/ease.rs` (`ease_f32`). Then read `grid/cursor.rs`,
   `grid/glyph/cursor.rs`, `audio.rs` (`MusicState` and its tests),
   `game.rs` (`with_stack`), `cinematic.rs` and `cinematic/tests.rs`,
   `frame/color.rs`, `dialogue/text_box.rs` and `hurry.rs`, all under
   `crates/ui/src/`. Check whether 2339 and 2364 are done.
2. Run the workspace's tests before changing anything, so every
   snapshot is known to pass.
3. In each module's tests, copy today's formula as a private reference
   (`pulse_before`, `gain_before`, `centre_before`). Move the motion,
   then compare against the reference (Tests required).
4. The cursor's pulse.
5. Music fades: every fade 2341 built (each `Ramp`), onto the tween.
6. `Pan::centre`.
7. If 2339 is done and still has its private formula,
   `transition::progress`.
8. The doc comments of `Rgb::lerp` and `type_on`, and the `Rgb` twin
   test.
9. Run the gates, the `no_std` build of `shuyi-ui` included. Check
   that `git status` shows no changed or new `.snap` file. Expect about
   400 changed lines, with 2341's fades.

## Acceptance criteria

- [ ] `Cursor::brightness` reads a looping, yoyo `Tween` with
      `Ease::InOut(Family::Sine)`. It is within 4/65536 of today's
      `pulse` at 4096 points of a period, and at a `blink_t` below 0
      and past the period.
- [ ] `MusicState`'s fade-out gain comes from a `Linear` `Tween` over
      whole milliseconds. Over a 500 ms fade at a `dt` of 1/60 and a
      1 s fade at a `dt` of 1/30, every frame sends today's commands,
      but for gains, which are within half a millisecond of the fade
      plus 1/65536; `Stop` comes on the same frame as today.
- [ ] `MusicState::with_switch(MusicSwitch::after_fade(500))` and
      `MusicState::new(0.5)` send the same commands.
- [ ] `Pan::centre` eases through `ease_f32(&Ease::Smooth, …)`. At
      every progress in steps of 1/1000 it is within 3/65536 of the
      distance from `from` to `to` of today's centre, and it gives
      `from` for a progress that isn't finite.
- [ ] `Rgb::lerp(a, b, k / 256)` equals `Lerp::mix(a, b, k * 256)` for
      every `k` from 0 to 256 and every pair of channel values.
- [ ] `Rgb::lerp`'s and `type_on`'s doc comments say why they are not
      tweens, and the Completion notes say why the three stay.
- [ ] If 2339 is done and had its private formula, `progress` goes
      through `Ease::Smooth`. It is within one thousandth of the
      formula at every `elapsed_ms` from 0 to `ms`, for `ms` of 1, 7,
      400 and 65 535, and gives 1000 when `ms` is 0.
- [ ] Each of 2341's fades reads a `Tween`, and its tests pass
      unchanged.
- [ ] Every existing test keeps its expected values, and the PR lists
      each tolerance it widened.
- [ ] No snapshot in the repository changes, and no `.snap.new` file
      is left.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `pulse` at 0, a quarter, a half and a whole period, and at a
  `t` below 0, past the period, infinite and not a number;
  `with_switch(MusicSwitch::after_fade(ms))` against `new`; a fade of 0 ms; `Pan::centre` at 0,
  1, not a number and infinity.
- Comparison against today's formulas, kept in the tests: the cursor
  at 4096 points of a period, the two fades frame by frame, the pan at
  1001 progresses, and 2339's `progress` if it moved.
- Property: the cursor's brightness stays within `BLINK_MIN..=1` for
  any `blink_t`.
- Exhaustive: `Rgb::lerp` against `Lerp::mix` at every whole 1/256.
- Snapshot / integration: every existing snapshot and Harness test,
  unchanged.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
