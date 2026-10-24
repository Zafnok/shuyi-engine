---
id: "1123"
title: "Parked: Aseprite and TexturePacker sheets imported into atlases and animation clips"
type: feature
milestone: "Engine: real time"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["2231"]
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
- The importers fill `shuyi_assets::atlas::Atlas`, 2222's asset kind
  (`AssetKind`, ADR-0019 §1) of pages and a table of name to page and
  rectangle. An imported sheet is an atlas of one page, the sheet's
  image, with each frame a named rectangle; there is one `Atlas` type for
  sheets and for packed images, so the sprite skin draws both the same
  way. What an import needs that packing doesn't (a trimmed frame's
  offset and its size before trimming, as TexturePacker's
  `spriteSourceSize` and `sourceSize` give) is added here as optional
  fields of an entry. **Either order with 2222:** if 2222 is done, add
  those fields to its `Atlas`; if not, write `Atlas` in
  `shuyi_assets::atlas` with the shape 2222's Scope names, and 2222
  adds the packer to it.
- The clips stay 2231's (`AnimationClip<ImageRect>`, `AnimationSet`,
  `frame_at`): an importer fills a clip's frames with the rectangles of
  the atlas's entries.
- The importers are `Format`s of `Atlas` and of 2231's `AnimationSet`
  (ADR-0007, ADR-0019; `docs/formats.md` says how to write one),
  registered with `Formats::register_standard`, each with its id,
  extension, limit and a property test that random bytes never panic
  it. They report errors with the file and the JSON path:
  - Aseprite JSON (`json-array` and `json-hash`, `frameTags` with
    direction and repeat, per-frame `duration`);
    - `meta.slices`: a slice with a `center` (Aseprite's 9-patch)
      becomes an entry of the nine-slice kind (`NineSliceDefs`, 2235,
      on 2338's `NineSlice`), named as the slice. Its rect is the
      slice's bounds placed in the sheet by its frame, and its borders
      are the centre's distance from the bounds' edges. So frame
      borders come from a standard file too (`AGENTS.md` rule 10).
      **Either order with 2235:** if 2235 is done, read the slices
      here; if not, leave this out (2235 reads Aseprite's slice
      centres when it lands after this one);
  - TexturePacker JSON (hash and array).

  Both read `.json`, and two formats of one kind can't claim one
  extension (ADR-0019 §2): one claims `.json`, and the other is found by
  sniffing its JSON (the tool named in its `meta.app`).
- Our RON sidecar form of `Atlas`, as a third format of it (2231 has
  `AnimationSet`'s), and `shuyi import --print` to turn an import into
  it (its `Encode`). It is the atlas's table, not 2222's `atlas.ron`
  (`AtlasSpec`, which says what to pack). Both are RON formats of the
  `Atlas` kind, and two formats of one kind can't claim one extension
  (ADR-0019 §2): tell them apart as ADR-0019 allows (by file name, as
  `atlas.ron` is, or by sniffing), and say how in the completion notes.
- **From 1129 (done):** `shuyi import` tries only the kinds
  `Importers::standard()` lists (`crates/cli/src/contract/import.rs`),
  so add `kind_with_text::<Atlas>` there, summed up with the file's
  `count` helper (`1 page, 12 entries`), with `--print` writing the
  RON sidecar above through its `Encode`. Fix that file's module doc,
  which says `import` needs no change for the Aseprite importer.
- Measure `asefile` behind an `aseprite` feature. Keep it only if it adds
  under 300 KB to the WASM build (ADR-0004 §2). Write down the number
  either way.
- Fixtures: a small sheet made for the tests (CC0, ours).
- **The skill pack. From 1134 (done):** its `shuyi-import-art` skill
  (`skills/shuyi-import-art/SKILL.md`) says this ticket's import is to
  come and lists 1123 in its `waits_for`, which `check-skills` fails on
  once this ticket is in `tickets/done/`. Drop 1123 from it, and teach
  the Aseprite and TexturePacker import as built. If that empties
  `waits_for` (1124 landed first), give the skill its worked example,
  `skills/shuyi-import-art/example.ron`, importing a sheet with the
  contract's `import`, which CI's `skills` job runs as it runs the
  others; otherwise 1124's PR writes it, as the last of the two.

**Out (do not do):** the clip types, `frame_at` and the `AnimationSet`
RON format (2231); animated tiles and unit clips in the grid kit (2359);
skeletal animation (2105); which clip plays when (state machines, 2104);
particles (2103); tweens on sprites (`shuyi_core::ease::Tween`, 2109).
Code over a store: `Atlas` and what the importers read are plain data
that name neither `Arena` (1119) nor `shuyi-ecs`'s `World` (2904's ADR),
and a tick game keeps `(clip, start_step)` in whichever store it uses.

## Implementation steps

1. Read 2231's `anim` module and 2222's `atlas` module (if 2222 is
   done), then add the import fields to `Atlas` (or write it, as Scope
   says) and its RON form.
2. Write the two importers with fixture files and error tests.
3. Do the `asefile` measurement.

## Acceptance criteria

- [ ] Both importers read their fixtures, and the bad fixtures give errors
      naming the JSON path.
- [ ] `shuyi import` reads the Aseprite fixture as an `Atlas`, and
      `--print` writes RON that decodes to the same atlas.
- [ ] If the nine-slice kind is done, a slice with a centre in the
      Aseprite fixture reads into it, with its rect and borders checked
      by hand.
- [ ] The `asefile` size result is recorded in the Completion notes.
- [ ] `shuyi-import-art`'s `waits_for` no longer holds 1123, and
      `check-skills` passes.
- [ ] `cargo deny check` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, property, and fixture tests, as above.

## Completion notes
