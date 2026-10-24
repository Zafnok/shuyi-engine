---
id: "2216"
title: "Parked: DragonBones JSON import into the skeletal module's rigs"
type: feature
milestone: "Engine: animation"
model: sonnet-5
effort: medium
status: blocked
blocked_by: ["2105"]
nick_input: none
completed:
---

# 2216 — DragonBones JSON import

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): a follow-up of the parked skeletal module (2105), so it starts after 2105 is unparked and done. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0007](../../docs/adr/0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md):
standard formats always load, and our text formats stay the source of
truth. 2105 builds rigs in our RON. This ticket reads the files bone
tools write, so an artist's work goes straight in.

DragonBones writes a skeleton file (`*_ske.json`) and a texture atlas
(`*_tex.json` beside a PNG). What was checked on 2026-10-20 (GitHub, the
`DragonBones` organization):

- **The runtimes are MIT and maintained.** `DragonBonesJS`'s `LICENSE`
  is MIT ("Copyright (c) 2012-2025 The DragonBones team"), with a
  PixiJS 8 runtime added in May 2025. `DragonBonesCSharp`,
  `DragonBonesUnity` and `Godot-DragonBones` had commits in 2026.
- **The old desktop editor (DragonBones Pro) is gone; LoongBones
  replaced it.** LoongBones is the same team's online editor, with
  documentation updated in October 2026 (`DragonBones/Loongbones-doc`).
  Its export panel offers "DragonBones JSON" with a data version to
  pick, so the format is still what the tool writes. Its price could
  not be confirmed: one forum post calls it free, another says it no
  longer is.
- So the format is alive, and the files artists make today import here.

We write our own serde types from the documentation. No crate, and no
code copied from a runtime.

Spine is not here: 2217 imports Spine's files. If Nick picks its path
B (a reader of our own), either order with 2217: whichever lands first
adds `shuyi_bones::import` and its feature, and the other reuses them.

## Nick input

None.

## Scope

**In:**
- **Check the license first.** Re-read `LICENSE` in the DragonBonesJS
  repository (MIT on 2026-10-20) and the format document's own terms. Record both, with the
  date, in the Completion notes and in `docs/formats.md`. If either is
  not permissive (ADR-0002 §8), stop and ask Nick with the `ask-nick`
  skill.
- **`DragonBonesJson`**, a format of 2105's `Rig` kind, in
  `shuyi_bones::import` (behind an `import` feature), registered as a
  standard format, with its id, extension, size limit and a property
  test that random bytes never panic it. It reads format 5.x
  (`version` and `compatibleVersion`), and rejects older versions with
  a clear error. Export a small rig from LoongBones at the newest data
  version it offers and list in the Completion notes any field this
  reader doesn't know; fields it doesn't know are warnings, never
  silently dropped.
  - the first armature, or the one named by the asset path's `#name`;
  - bones: `parent`, `transform` (`x`, `y`, `skX` as the rotation, in
    degrees, to 2335's `Angle`), `length`;
  - slots: `parent` bone, `displayIndex`, draw order;
  - skins: displays of `type: "image"` become attachments with their
    transform;
  - animations: `duration` in frames at the armature's `frameRate`, to
    ms; bone `rotateFrame` and `translateFrame`; slot `displayFrame`;
    `frame` events. `tweenEasing: null` is `Steps(1)` and `0` is
    `Linear` (2105's keys take a `shuyi_core::ease::Ease`, 2109); any
    other `tweenEasing`, and a Bézier `curve`, becomes `Linear` with a
    warning naming the JSON path.
  - Everything 2105 doesn't support is an error naming the JSON path:
    `scX` or `scY` other than 1, a skew (`skX ≠ skY`), mesh, armature
    and bounding-box displays, IK, FFD and nested armatures.
- **`DragonBonesAtlas`**, a format of the `Atlas` kind (2222's, which 1123 imports into) for
  `*_tex.json` (`imagePath`, `SubTexture` with `name`, `x`, `y`,
  `width`, `height`, and `frameX`, `frameY`, `frameWidth`,
  `frameHeight`). 1123's Aseprite and TexturePacker formats already
  share `.json`; follow the sniffing rule 1123 settled (ADR-0019 §2):
  this one is found by its `SubTexture` list.
- **`shuyi-cli import --print`** prints an imported rig as 2105's RON
  (its `Encode`), and a test checks: import, print, read the RON back,
  equal rig (ADR-0007 §4).
- **Fixtures:** small files written by hand for the tests (ours, CC0),
  never copied from DragonBones's samples.

**Out (do not do):**
- Spine (2217);
- scale, skew, meshes, IK (2105 doesn't have them; a follow-up of 2105
  first);
- keeping a Bézier `curve` as a curve: 2105's keys take an `Ease`, which
  has no curve through points. Keys that take the `Points` curve of
  2135 are a follow-up of 2105 first, so a Bézier stays `Linear` here
  whether or not 2135 is done;
- the binary DragonBones format (`.dbbin`).

## Implementation steps

1. Check the licenses, as above.
2. Read 2105's ADR and `Rig`, 2109's `Ease`, and 1123's sniffing rule
   for `.json`.
3. Write the serde types for the 5.x format, from the documentation.
4. Write the conversion to `Rig`, with errors that name the JSON path.
5. Write the atlas format.
6. Add fixtures, the round trip and the property tests.
7. Run the gates.

## Acceptance criteria

- [ ] The licenses are recorded with the date.
- [ ] The fixture skeleton and atlas load into a `Rig` and an `Atlas`.
- [ ] Each unsupported feature in a bad fixture gives an error naming its
      JSON path.
- [ ] Import, print as RON, read back: the same `Rig`.
- [ ] Random bytes never panic either format.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: degrees to `Angle` rounding; frames to ms at 24 and 30 fps;
  easing mapping; armature choice by `#name`.
- Property: random bytes; random valid rigs printed and read back.
- Snapshot / integration: the fixture rig at one time, drawn with 2105's
  skin, equal to the same rig read from RON.

## Completion notes

*(Filled in by the session that completes the ticket.)*
