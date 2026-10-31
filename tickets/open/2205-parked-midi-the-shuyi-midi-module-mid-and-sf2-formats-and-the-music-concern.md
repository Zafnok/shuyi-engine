---
id: "2205"
title: "Parked: MIDI: the shuyi-midi module, the .mid and .sf2 formats, Sequence as plain data, and music as a concern in the catalog"
type: feature
milestone: "Engine: MIDI music"
model: implementation
effort: high
status: blocked
blocked_by: ["2201", "2003", "2116"]
nick_input: none
completed:
---

# 2205 — MIDI: the module, the formats, and music as a concern

> **Parked** (Nick, 2026-10-17, asked whether MIDI should be parked because the game doesn't need it, chose "Parked"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-17 (`docs/decisions.md`): the new-game wizard asks "if
they want MIDI sound".
[ADR-0014](../../docs/adr/0014-midi-music-sequences-and-a-sound-bank-in-a-module.md)
sets the design: music is a concern in the catalog, and MIDI is a module,
`shuyi-midi`, with two formats (`mid`, `sf2`) and a `Sequence` that is
plain data. This ticket is ADR-0014 §1, §2 and the format tests of §4.
Playing the music is 2206.

## Nick input

None.

## Scope

**In:**
- **The crate** `crates/midi` (`shuyi-midi`), with the module duties of
  ADR-0009 §4:
  - metadata: concern `music`, answer `midi`, serves every direction,
    alternatives none (recorded music is the default and can share a
    game);
  - the README in its fixed order;
  - the `midi` feature of the `shuyi` crate (2003) and the catalog row;
  - the `shuyi-midi` skill. **From 1134 (done):**
    `skills/shuyi-midi/SKILL.md` in the pack's form, with a worked
    example, `example.ron`, that runs on a fresh game after `shuyi add
    midi` and that CI's `skills` job runs; or, if nothing of the module
    runs yet, a `waits_for` naming the ticket that writes the example,
    which this PR then tells (it names this skill and drops its id). It
    is a module's skill: `shuyi add midi` copies it into a game;
  - its `needs` (ADR-0012 §1), if 3017 is done: `ram_bytes` for the
    largest fixture bank plus the synth, provisional until 2206 measures
    it. If 3017 isn't done, 3017 gives it a `needs` key with the other
    modules.
- **The music concern** in `docs/modules.ron`: answers `recorded`
  (default, ready) and `midi` (this module), `alternatives: false`.
  **Either order with 2502:** if 2502 is done, give it `pick: Many` and
  the question "What plays the music?", so the wizard asks it with no
  change to the wizard, and add `--music` to its flags list in
  `docs/templates.md`; if not, give it only the fields the catalog has
  today, and 2502 adds `pick` and `question` (its clause says so).
- **`Sequence`** in `shuyi_midi::sequence`: tracks of events (note on and
  off with key and velocity, program change, control change, tempo) at
  integer ticks, the ticks per quarter note, the tempo map, and the
  length in ticks. No floats. A `Sequence` can be built in code with a
  small builder, and it has a text form, `*.seq.ron`.
- **Either order with 2365** (parked: a beat and bar clock for recorded
  music): if 2365 is done, add `Sequence::beat_at(tick) ->
  shuyi_core::beat::BeatPos`, with beats from the ticks per quarter note
  and bars from the time signature (4/4 if the `Sequence` keeps none),
  so a screen keeps time with MIDI and recorded music the same way. If
  not, 2365 adds it when it is worked.
- **`SoundBank`** in `shuyi_midi::bank`: the `.sf2` file's bytes, checked
  to parse, with its preset names listed.
- **Formats** (ADR-0007), registered by the module's `register` call:
  - `mid` (`.mid`, `.midi`) into `Sequence`, with `midly`;
  - `seq-ron` (`.seq.ron`) into `Sequence`;
  - `sf2` (`.sf2`) into `SoundBank`, parsed with the synth crate 2206
    uses (`rustysynth`), so the bank is parsed by one library only.
  - Check each crate's license from its `Cargo.toml` and repository on
    the day (`midly` is believed Unlicense, `rustysynth` MIT; both are on
    ADR-0002 §8's list), that `cargo deny` passes, and that each builds
    for `wasm32-unknown-unknown` without `wasm-bindgen`. If one fails,
    pick another or write the reader, and record why in the README.
- **Fixtures**, made by us and released CC0-1.0, listed in
  `THIRD_PARTY_ASSETS.md` as our own: two short `.mid` files (one track,
  several tracks with a tempo change) and the smallest `.sf2` that holds
  one instrument.
- **A golden table**, `crates/midi/tests/golden/formats.ron`, through
  2116's `shuyi_assets::golden` (a dev-dependency on `shuyi-assets`
  with its `golden` feature), checked by a test with `golden` in its
  name: one row per fixture, the `state_hash` of what its format
  decodes. `-p shuyi-midi` joins the wasm32 `cargo test` of CI's
  `determinism` job.
- **A sound bank for games.** Look for a General MIDI bank under
  CC0-1.0, CC-BY-3.0 or CC-BY-4.0. Record what you found, with links
  and dates, in the README. If none passes, the engine ships none, and
  the README says each game brings its own.

**Out (do not do):**
- the synth and playback (2206);
- MIDI input from devices, and MIDI output to devices (ADR-0014,
  alternatives);
- tracker formats (`.mod`, `.xm`).

## Implementation steps

1. Check the crates' licenses and web builds; record the results.
2. Write the crate, its metadata, README and catalog row.
3. Write `Sequence`, its builder and `seq-ron`.
4. Write the `mid` and `sf2` formats, with fixtures, tests and the
   golden table.
5. Add the music concern to the catalog.

## Acceptance criteria

- [ ] Each fixture `.mid` decodes to the `Sequence` hash in a golden
      table (2116's `shuyi_assets::golden`), and `-p shuyi-midi` joins
      the wasm32 `cargo test` of CI's `determinism` job.
- [ ] A `.mid` fixture and its `.seq.ron` give equal `Sequence`s.
- [ ] `cargo deny check` passes with the `midi` feature on.
- [ ] `check-modules` passes, with the music concern and the module's
      row.
- [ ] The README states the sound bank search's result.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the builder; tick and tempo maths with no floats;
  `Sequence::beat_at`, if 2365 is done.
- Golden: decode hashes per format in `formats.ron`, natively in CI's
  `test` job and on wasm32 in its `determinism` job.
- Fuzz-style: arbitrary bytes into `mid`, `seq-ron` and `sf2` never
  panic.

## Completion notes
