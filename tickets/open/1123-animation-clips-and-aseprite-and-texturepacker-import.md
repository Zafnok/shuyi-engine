---
id: "1123"
title: "Parked: Aseprite and TexturePacker sheets imported into atlases and animation clips"
type: feature
milestone: "Engine: real time"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["1119", "2231"]
nick_input: none
completed:
---

# 1123 — Aseprite and TexturePacker import

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): after Visions of Shuyi launches, not only after the MVP (`docs/decisions.md`, 2026-10-20). Unparked early only if the game needs it: its ticket 0060 says the army walks around freely (then 1119–1125 become game work), or it asks for a piece of this through its `engine-change` skill, which is then split out and unparked alone. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0004 §2–§3. Today the only animation is the 3×4 walk sheet
(game ADR-0049). An `AnimationClip` is a list of frames with a direction and
a repeat rule. The frame shown is a pure function of the clip and the
time since it started (as game ADR-0060 does for cinematics), so a tick sim
stores only `(clip, start_step)`. The clip itself (`AnimationClip`,
`AnimationSet`, `frame_at` and the RON file of clips) is 2231's, split
out on 2026-10-20 because it needs no tick simulation. This ticket
imports into it.

Aseprite's CLI exports a sheet plus JSON with frame tags. That is the
common route and needs no extra crate. Reading `.aseprite` directly goes
through `asefile` (MIT), only if it stays small.

## Nick input

None.

## Scope

**In:**
- `shuyi_assets::anim::Atlas` (named rectangles in an image), an asset
  kind (`AssetKind`, ADR-0019 §1), beside 2231's `AnimationClip`,
  `AnimationSet` and `frame_at`.
- The importers are `Format`s of `Atlas` and of 2231's `AnimationSet`
  (ADR-0007, ADR-0019; `docs/formats.md` says how to write one),
  registered with `Formats::register_standard`, each with its id,
  extension, limit and a property test that random bytes never panic
  it. They report errors with the file and the JSON path:
  - Aseprite JSON (`json-array` and `json-hash`, `frameTags` with
    direction and repeat, per-frame `duration`);
    - `meta.slices`: a slice with a `center` (Aseprite's 9-patch)
      becomes an entry of the nine-slice kind (`NineSliceDefs`, the
      follow-up ticket that 2338's step 8 writes), named as the slice.
      Its rect is the slice's bounds placed in the sheet by its frame,
      and its borders are the centre's distance from the bounds' edges.
      So frame borders come from a standard file too (`AGENTS.md` rule
      10). **Either order with that ticket:** if it is done, read the
      slices here; if not, or 2338 hasn't written it yet, leave this
      out: 2338's step 8 has that ticket read Aseprite's slice centres
      once this one is done;
  - TexturePacker JSON (hash and array).

  Both read `.json`, and two formats of one kind can't claim one
  extension (ADR-0019 §2): one claims `.json`, and the other is found by
  sniffing its JSON (the tool named in its `meta.app`).
- Our RON sidecar form of `Atlas`, as a third format of it (2231 has
  `AnimationSet`'s), and `shuyi import --print` to turn an import into
  it (its `Encode`).
- Measure `asefile` behind an `aseprite` feature. Keep it only if it adds
  under 300 KB to the WASM build (ADR-0004 §2). Write down the number
  either way.
- Fixtures: a small sheet made for the tests (CC0, ours).

**Out (do not do):** the clip types, `frame_at` and the `AnimationSet`
RON format (2231); animated tiles and unit clips in the grid kit (2359);
skeletal animation (2105); which clip plays when (state machines, 2104);
particles (2103); tweens on sprites (`shuyi_core::ease::Tween`, 2109).
Code over a store: `Atlas` and what the importers read are plain data
that name neither `Arena` (1119) nor `shuyi-ecs`'s `World` (2904's ADR),
and a tick game keeps `(clip, start_step)` in whichever store it uses.

## Implementation steps

1. Read 2231's `anim` module, then write `Atlas` and its RON form.
2. Write the two importers with fixture files and error tests.
3. Do the `asefile` measurement.

## Acceptance criteria

- [ ] Both importers read their fixtures, and the bad fixtures give errors
      naming the JSON path.
- [ ] If the nine-slice kind is done, a slice with a centre in the
      Aseprite fixture reads into it, with its rect and borders checked
      by hand.
- [ ] The `asefile` size result is recorded in the Completion notes.
- [ ] `cargo deny check` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, property, and fixture tests, as above.

## Completion notes
