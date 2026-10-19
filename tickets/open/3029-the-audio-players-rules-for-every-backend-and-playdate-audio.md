---
id: "3029"
title: "The audio player's rules shared by every backend, and audio on the Playdate: sounds baked to ADPCM, music streamed through the SDK"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["3003", "3010"]
nick_input: none
completed:
---

# 3029 — The audio player for every backend, and Playdate audio

## Context

- **The game ticket:** game#1170
  ([`1170-the-playdate-front-end-on-the-engines-backend.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1170-the-playdate-front-end-on-the-engines-backend.md))
  waits for this one. Found by the game's first engine-friction sweep,
  game#1153. 3010 lists audio under **Out** and asks for this follow-up
  "when this lands"; it is written now because there is working code to
  lift.
- **The job:** "play a game's sounds and music on the Playdate the way
  they play on the PC".
- **Friction:** the game's Playdate front end, 1735 production lines
  against an estimate of 100 (17×), outcome 1, with a signal: **engine
  code copied into the game**. The engine's audio player, which keeps
  the rules (cue volumes, picking a variant, how many voices, fades,
  the music clock), is `pub(crate) struct Audio<B: Backend>` in a
  private module of `shuyi-app` (`crates/app/src/audio.rs`). So the
  game re-wrote those rules for the Playdate (`crates/playdate/src/audio.rs`,
  245 lines, with a fake speaker in its tests) over the SDK's sample
  and file players (`src/pd/sound.rs`, 89 lines), and converts its OGG
  and WAV files to IMA ADPCM at build time (`crates/xtask/src/playdate_audio.rs`,
  about 400 lines).
- **The generic shape:**
  - the player's rules move out of `shuyi-app` to a place every backend
    shares (`shuyi-ui`'s audio, or `shuyi-platform` from 3003), behind
    the small backend trait they already have (load, play, stop, volume,
    stream music, position), so a backend writes only that trait;
  - `shuyi-playdate` implements it over the SDK's sample players (sounds
    loaded at start) and file players (music streamed, at most two at
    once for a fade);
  - `shuyi export playdate` bakes sounds and music to the formats the
    SDK plays.
- **Outside tactics:** any game exported to the Playdate with sound.

## Nick input

None.

## Scope

**In:**
- Move the rules; `shuyi-app` keeps its behavior on desktop and the web
  (its tests unchanged).
- The Playdate backend's audio, from the game's two files (read them at
  the game's `main`).
- The conversion in the exporter, from the game's `playdate_audio.rs`;
  check the license of anything it uses (`cargo deny`).
- A track the game doesn't ship on the Playdate plays as silence, with
  a warning (the game's rule: some music is left out for the `.pdx`
  size).

**Out (do not do):**
- MIDI through the SDK's synth (2205, 2206, parked).
- Any game's sound list.

**For the game:** the game's audio on the Playdate moves onto this in
game#1170. If the rules move to a new place, the game's desktop build
may need an import path changed in that PR.

## Implementation steps

1. Read `shuyi-app`'s audio module and its tests; read the game's
   Playdate audio and conversion.
2. Move the rules behind the backend trait; desktop tests unchanged.
3. The Playdate backend's audio with a fake SDK.
4. The bake step in the exporter.

## Acceptance criteria

- [ ] `shuyi-app`'s audio tests pass unchanged.
- [ ] Host tests of the Playdate audio with a fake SDK: a cue plays at
      its volume; a fade between two tracks opens the second before
      closing the first; the music clock follows the file player.
- [ ] The example exported for the Playdate plays a sound in the
      Simulator, or the reason it can't is recorded.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (host) against a fake SDK; the existing desktop audio tests.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
