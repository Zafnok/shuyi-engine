---
id: "2379"
title: "Parked: Cue pitch: a pitch range in the cue rules, picked for each play like the volume and played as the sound's rate"
type: feature
milestone: "Engine: in-house platform"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["2344", "2345", "2378", "3029", "3030", "3031", "3032"]
nick_input: none
completed:
---

# 2379 — Cue pitch: a pitch range in the cue rules

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): a follow-up of parked work is parked. A pitch plays through the in-house mixer (3030) and its sound output (3031, 3032), so this starts after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`), once those three are unparked and done. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title. It also waits for the sound research (2902), which may keep, change or drop the mixer it plays through.

## Context

3030 promised this ticket: "A pitch range in the audio manifest's cue
rules (2345) is 2379's (parked), once the desktop and the web play
through this mixer (3031, 3032)". 2345's **Out** and 3029's **Out**
point here too.

- **Why.** The 2026-10-20 comparison (2345's Context) found that
  Godot's `AudioStreamRandomizer` and Unity's Audio Random Container
  vary the pitch of each play as well as its volume, so twenty
  footsteps from three files don't sound like a machine. 2345 gave
  Shuyi a file and a volume picked for each play, and left pitch out:
  quad-snd 0.2.8's `PlaySoundParams` is `looped` and `volume` only, so
  no backend could play it, and a field that does nothing on the
  desktop and the web would mislead.
- **What changes that** (all of it in `blocked_by`):
  - 3029 adds `set_rate(sound, rate)` to the shared backend trait as a
    provided method: 16.16 fixed point, 1.0 plays the sound as
    recorded, from the sound's next `play`. Its default returns
    `Err(Unsupported)`. The player's helper turns the first one into
    one warning, and the sound plays at 1.0;
  - 3030's mixer plays a voice at `Params::rate`, the resampler's step;
  - 3031 fills in `set_rate` over the mixer on Windows and the web, and
    3032 makes that the only path, on Linux and macOS too, and removes
    quad-snd. After 3032 every desktop and web build plays a rate;
  - the Playdate backend (3029) fills in `set_rate` if the SDK's sample
    player has a rate.
- **Where the pick goes.** 2345's rules run in `shuyi-platform`'s
  `App` (`Cues`, `crates/platform/src/audio/cues.rs`), with a variant
  RNG of their own, and a `SoundRequest::Sound` reaches the device with
  its file (`variant`) and its volume (`cue_volume`) already picked.
  The pitch is one more pick there, so the Harness, the headless
  backend and every port play the same rate for the same seed.
- **Copies.** 2378 (2345's part two) ends a copy at its start plus its
  file's length, for the copy caps and the voice limit, and the player
  and the Playdate backend keep each copy's playback until its length
  has passed. A copy played at half the rate lasts twice as long, so
  each of those must use the rate.
- **Handled sounds.** 2344's `SoundRequest::Start` gets its file and
  its cue volume from `Cues` as `Sound` does (2344's clause with 2345),
  so it gets the rate too.
- **Outside tactics:** a platformer's footsteps and coins, a shooter's
  shots, a farming game's hoe, a puzzle game's clicks.

## Nick input

None.

## Scope

**In:**
- **The data** (`crates/assets/src/audio.rs`), optional, so every
  manifest written before this loads unchanged:
  - `CueRules` gains `pitch_min: u16` and `pitch_max: u16`, each a
    whole percent of the rate the file was recorded at, 100 when left
    out. Each play picks a whole percent from `pitch_min` to
    `pitch_max`. Both at 100 is today's sound. One field alone is a
    range on one side: `pitch_min: 90` alone plays from 90 to 100. Two
    equal values other than 100 play every time at that percent;
  - `CueRules` follows ADR-0039 (2345 made it so), and the new fields
    are additive. Its hand-written `Default` (2378 wrote it for
    `priority`) and `CueRules::DEFAULT` set both to 100, and the test
    that they are equal stays;
  - the RON, flat on the sound entry, as 2345's fields are:
    ```ron
    (id: "step", files: ["sfx/step_1.ogg", "sfx/step_2.ogg"],
     volume: 60, credit: Own, pitch_min: 94, pitch_max: 106),
    ```
    `RawSound` reads each with `#[serde(default = "…")]`, a function
    that returns 100;
  - `pub const PITCH_PERCENT: RangeInclusive<u16> = 25..=400`: from a
    quarter of the rate to four times it, two octaves each way. Below
    it a short sound turns into a rumble; above it 3030's linear
    resampler skips most samples and aliases;
  - `Check::sound`: `pitch_min` over `pitch_max`, or either outside
    `PITCH_PERCENT`, is an error naming the cue, at its line;
  - the manifest's binary form changes, so `pack::VERSION`
    (`crates/assets/src/pack.rs`) becomes the next number and the
    pinned snapshot in `crates/assets/src/pack/tests.rs` is accepted.
- **The rate** (`crates/platform/src/audio.rs`):
  - `pub const RATE_ONE: u32 = 0x1_0000` (1.0 in 16.16) and
    `pub const fn rate_from_percent(percent: u16) -> u32`, which is
    `(u32::from(percent) << 16) / 100`, so 100 gives exactly
    `RATE_ONE`;
  - `pub const fn length_at_rate(length_ms: u32, rate: u32) -> u32`:
    how long a sound of `length_ms` plays at `rate`,
    `length_ms * 0x1_0000 / rate` rounded up, in `u64`, saturated to
    `u32::MAX`. A rate of 0 counts as `RATE_ONE`;
  - `SoundRequest::Sound` and `SoundRequest::Start` (2344) gain
    `rate: u32`, 16.16, as 3029's `set_rate` takes it. If 3029 gave
    the rate a type of its own, use that type here and in
    `rate_from_percent`. The variants' docs say a backend plays it
    through `set_rate`, or at 1.0 if it can't.
    `SoundRequest::from_request` sets `RATE_ONE`.
- **The pick** (`Cues`, `crates/platform/src/audio/cues.rs`):
  - a play that sounds draws its file, then its volume, then its
    percent, `pitch_min + rng.below(pitch_max - pitch_min + 1)`, and
    sends `rate_from_percent` of it. A cue whose `pitch_min` equals its
    `pitch_max` draws nothing for its rate, so a game without ranges
    gets the same files and volumes from the same seed as before this
    ticket. A dropped play still draws nothing;
  - `Start` gets its rate the same way, after its file and its volume;
  - a manifest built in code skips the check: `Cues` clamps both values
    into `PITCH_PERCENT`, and plays `pitch_min` over `pitch_max` at
    `pitch_max` with no draw, as 2345 plays `volume_min` over `volume`
    at `volume`;
  - the copies (2378): a copy's `ends` is `started` plus
    `length_at_rate` of its file's length and its rate. At 50 percent a
    copy holds its copy and its voice twice as long, at 200 percent half
    as long. A copy of unknown length still ends at once. The model in
    2378's property test works its ends out the same way, from each
    `Sound` request's `rate`.
- **The player** (the shared player 3029 moved out of `shuyi-app`; its
  completion notes say where):
  - it keeps, for each file's slot, the rate it last set (`RATE_ONE`
    at load). Before it plays a `Sound` or a `Start` whose rate
    differs, it calls `set_rate(sound, rate)` through 3029's helper,
    then `play`. On `Ok` the slot keeps the new rate;
  - on `Unsupported` the helper gives its one warning for the rate,
    which names the feature, the slot keeps `RATE_ONE`, and the sound
    plays at 1.0 (3029's fallback). Later plays give no second warning;
  - a game without pitch ranges never calls `set_rate`, so a backend
    without it never warns;
  - it keeps a copy's playback (2378) until `StopCopy` names it or
    `length_at_rate` of its length has passed, in place of its length.
    So does the Playdate backend for each copy's sample player (2378's
    clause with 3029), with a test against its fake SDK. Where
    `set_rate` gave `Unsupported`, the copy plays at 1.0 and is kept
    for its length.
- **Either order with 2218** (writers of the text formats): if 2218 is
  done, its `AudioManifestRon` writer writes `pitch_min` and
  `pitch_max`, each only when it isn't 100, and its round trip covers a
  cue with a pitch range; if not, nothing to do (2218 writes them, its
  clause says so).
- **Docs:** `docs/formats.md` (the two fields in the audio manifest's
  cue rules, `PITCH_PERCENT`, and the percents for semitones: 89 is
  about two down, 94 one down, 106 one up, 112 two up);
  `docs/platform-backends.md` (the `audio` section: `rate` on `Sound`
  and `Start`, and that a backend plays it through `set_rate` or at
  the normal rate).
- **An ADR** (the `write-adr` skill, the next free number). 2345's ADR
  says pitch waits for a backend that can play it, and an accepted
  ADR's decision is never rewritten, so this one extends it and
  3029's: the pitch is a whole-percent range in the cue rules, picked
  in `Cues` after the file and the volume, and only when the range is
  wider than one value; it reaches the device as a 16.16 rate; a
  backend plays it with `set_rate`, or at 1.0 with one warning; and a
  copy lasts its length over its rate.
- **For the game:** the API changes. `SoundRequest::Sound` and `Start`
  have a new field (match them with `..`), and the pack's version goes
  up, so a pack baked before this is refused and rebaked. The game's
  manifest loads as it is. A backend of the game's own plays `rate` or
  leaves it out. Each break gets a `Breaking:` line in the completion
  notes, and the game's PR that moves the pin follows this one.

**Out (do not do):**
- Filling in `set_rate` in a backend. 3029 (the Playdate) and 3031
  (the in-house backend, the only one after 3032) do it, and 3029's
  default plays at 1.0.
- A rate set on a sound already playing, slides and vibrato:
  `set_rate` acts from the sound's next play. Write a ticket when a
  game asks.
- A rate a game asks for on one play (an engine's hum that rises with
  speed): only the manifest's range here. Write a ticket when a game
  asks.
- Pitch for music tracks and voice lines: only sound cues.
- Semitones or cents in the manifest: whole percents only. The docs
  give the percents for semitones.
- The rate in `run`'s answer: `sounds` lists the cues asked for, before
  the rules (2345). Write a ticket if an agent needs the rates.
- Saying that a game needs `Rate`, and checking it against a target:
  3017 (parked). Its needs already cover a game that changes a sound's
  rate on a target without `Rate`.
- Choosing the sound stack: 2902 (parked). If its report replaces
  3030's mixer, whatever fills in `set_rate` then plays this rate.

## Implementation steps

1. Read the completion notes of 2344, 2345, 2378, 3029, 3031 and 3032,
   then `crates/assets/src/audio.rs` (`CueRules`, `RawSound`,
   `Check::sound`, the tests' `FULL`), `crates/platform/src/audio.rs`,
   `crates/platform/src/audio/cues.rs` (`VariantRng`, the copies), and
   the shared player with its helper and its fake backend. Check
   whether 2218 is done (the either-order clause).
2. The data: the two fields, their defaults, `PITCH_PERCENT`, the
   check, `CueRules::DEFAULT`, the pack version and its snapshot.
3. `RATE_ONE`, `rate_from_percent`, and `rate` on `Sound` and `Start`.
   Fix every literal of them (`grep -rn "SoundRequest::Sound {"` and
   `"SoundRequest::Start {"` over `crates/` and `examples/`).
4. The pick in `Cues` and the copies' ends, with their tests.
5. The player: the rate per slot and `set_rate`, and each copy kept
   for `length_at_rate` of its length, with tests against its fake
   backend; then the same keeping in the Playdate backend, with a test
   against its fake SDK.
6. The Harness test (below).
7. Check by ear on the desktop and in the web build (the `run` skill):
   give hello-grid's move cue `pitch_min: 80, pitch_max: 125` in a
   local change, not committed, and hear its steps go higher and lower.
8. The docs and the ADR. Run the gates.

## Acceptance criteria

- [ ] Every manifest in the repository's tests loads with `pitch_min`
      and `pitch_max` at 100; a test loads `FULL` and finds them so on
      each cue.
- [ ] `pitch_min: 94, pitch_max: 106` on a sound entry loads to those
      values; `pitch_min` over `pitch_max`, `24` or `401` is an error
      naming the cue, at its line.
- [ ] `rate_from_percent(100) == RATE_ONE`, `rate_from_percent(50) ==
      0x8000` and `rate_from_percent(200) == 0x2_0000`;
      `length_at_rate(1000, 0x8000) == 2000`,
      `length_at_rate(1000, 0x2_0000) == 500`, and
      `length_at_rate(1000, rate_from_percent(94))` is 1 064.
- [ ] The percent picked is always within `pitch_min..=pitch_max`
      (property test), and the same seed and the same requests give
      the same rates.
- [ ] With every cue's `pitch_min` equal to its `pitch_max`, `Cues`
      gives the same files and volumes as with the default rules, for
      random requests and seeds (property test): an equal range draws
      nothing.
- [ ] A dropped play draws nothing with pitch ranges on the cues
      (2345's property test, given random ranges).
- [ ] In `Cues`, a copy of a 1 000 ms file holds its copy for
      2 000 ms at 50 percent and for 500 ms at 200 percent (unit
      tests), and 2378's property test, given random pitch ranges,
      still finds no more than `max_copies` of a cue, or `max_voices`
      in all, sounding.
- [ ] The player against its fake backend, and the Playdate backend
      against its fake SDK: a copy of a 1 000 ms file at 50 percent is
      still kept at 1 500 ms and gone at 2 000 ms.
- [ ] Against the player's fake backend: a `Sound` at `0x1_8000` calls
      `set_rate` with it before `play`; a second play at the same rate
      doesn't call it again; a play at `RATE_ONE` after it calls it
      with `RATE_ONE`; a game without ranges never calls it.
- [ ] On a test backend with only the required methods, a cue with a
      range plays ten times with exactly one warning, which names the
      rate.
- [ ] In the Harness, with the toy game's `click` given `pitch_min: 50,
      pitch_max: 200` (`Toy::audio` is public), Confirm on the menu
      overlay (`crates/ui/src/toy.rs`) gives, in
      `harness.platform().speaker().requests()` after that step, a
      `SoundRequest::Sound` whose rate is within `0x8000..=0x2_0000`,
      and a second run with the same seed gives the same rate.
- [ ] `pack::VERSION` is one higher, with the pack snapshot updated.
- [ ] `shuyi-platform` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the RON fields and their defaults; the check's three errors;
  `rate_from_percent` and `length_at_rate`; a cue of two equal values
  plays at that percent; a manifest built in code with values out of
  range is clamped; the copies' ends at 50 and 200 percent; the
  player's `set_rate` calls and its one warning; a copy kept for its
  length at its rate, by the player and by the Playdate backend.
- Property: the percent within the range; an equal range draws
  nothing; a dropped play draws nothing; 2378's copy and voice limits
  with random ranges.
- Integration: the Harness test above.
- Snapshot: the pack's pinned binary forms.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
