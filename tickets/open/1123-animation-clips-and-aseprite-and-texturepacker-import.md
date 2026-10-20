---
id: "1123"
title: "Parked: Animation clips drawn from the time alone, with Aseprite and TexturePacker sheets imported"
type: feature
milestone: "Engine: real time"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["1119"]
nick_input: none
completed:
---

# 1123 — Animation clips; Aseprite and TexturePacker import

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0060 says the army walks around freely (then 1119–1125 become game work). Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0004 §2–§3. Today the only animation is the 3×4 walk sheet
(game ADR-0049). An `AnimationClip` is a list of frames with a direction and
a repeat rule. The frame shown is a pure function of the clip and the
time since it started (as game ADR-0060 does for cinematics), so a tick sim
stores only `(clip, start_step)`.

Aseprite's CLI exports a sheet plus JSON with frame tags. That is the
common route and needs no extra crate. Reading `.aseprite` directly goes
through `asefile` (MIT), only if it stays small.

## Nick input

None.

## Scope

**In:**
- `shuyi_assets::anim`: `Atlas` (named rectangles in an image),
  `AnimationClip`, `AnimationSet`, `frame_at(clip, elapsed_ms)`. `Atlas`
  and `AnimationSet` are asset kinds (`AssetKind`, ADR-0019 §1).
- The importers are `Format`s of those kinds (ADR-0007, ADR-0019;
  `docs/formats.md` says how to write one), registered with
  `Formats::register_standard`, each with its id, extension, limit and a
  property test that random bytes never panic it. They report errors with
  the file and the JSON path:
  - Aseprite JSON (`json-array` and `json-hash`, `frameTags` with
    direction and repeat, per-frame `duration`);
  - TexturePacker JSON (hash and array).

  Both read `.json`, and two formats of one kind can't claim one
  extension (ADR-0019 §2): one claims `.json`, and the other is found by
  sniffing its JSON (the tool named in its `meta.app`).
- Our RON sidecar form of the same data, as a third format of each kind,
  and `shuyi-cli import --print` to turn an import into it (its `Encode`).
- Measure `asefile` behind an `aseprite` feature. Keep it only if it adds
  under 300 KB to the WASM build (ADR-0004 §2). Write down the number
  either way.
- Fixtures: a small sheet made for the tests (CC0, ours).

**Out (do not do):** skeletal animation; tweens on sprites (the cinematic
easing covers them).

## Implementation steps

1. Write the types and `frame_at` with property tests (the frame index
   is periodic for loops; ping-pong never repeats the end frame twice).
2. Write the two importers with fixture files and error tests.
3. Do the `asefile` measurement.

## Acceptance criteria

- [ ] `frame_at` property tests pass.
- [ ] Both importers read their fixtures, and the bad fixtures give errors
      naming the JSON path.
- [ ] The `asefile` size result is recorded in the Completion notes.
- [ ] `cargo deny check` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, property, and fixture tests, as above.

## Completion notes
