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
(`*_tex.json` beside a PNG). Its runtimes are MIT (DragonBonesJS,
`github.com/DragonBones/DragonBonesJS`, as third-party listings say;
checked 2026-10-20 in those listings only; its LICENSE file is still to be read). Its
format is documented in that repository. The original editor is reported
discontinued; the DragonBonesJS docs point to LoongBones for making the
files. Either way the format stays readable, and other engines' runtimes
read it.

We write our own serde types from the documentation. No crate, and no
code copied from a runtime.

Spine is not here: whether the engine reads Spine's files is a question
for Nick (its runtime license asks every user to own an editor
license).

## Nick input

None.

## Scope

**In:**
- **Check the license first.** Read `LICENSE` in the DragonBonesJS
  repository and the format document's own terms. Record both, with the
  date, in the Completion notes and in `docs/formats.md`. If either is
  not permissive (ADR-0002 §8), stop and ask Nick with the `ask-nick`
  skill.
- **`DragonBonesJson`**, a format of 2105's `Rig` kind, in
  `shuyi_bones::import` (behind an `import` feature), registered as a
  standard format, with its id, extension, size limit and a property
  test that random bytes never panic it. It reads format 5.x
  (`version` and `compatibleVersion`), and rejects older versions with
  a clear error.
  - the first armature, or the one named by the asset path's `#name`;
  - bones: `parent`, `transform` (`x`, `y`, `skX` as the rotation, in
    degrees, to 1119's `Angle`), `length`;
  - slots: `parent` bone, `displayIndex`, draw order;
  - skins: displays of `type: "image"` become attachments with their
    transform;
  - animations: `duration` in frames at the armature's `frameRate`, to
    ms; bone `rotateFrame` and `translateFrame`; slot `displayFrame`;
    `frame` events. `tweenEasing: null` is stepped, `0` is linear; any
    other easing or `curve` becomes linear with a warning naming the
    JSON path.
  - Everything 2105 doesn't support is an error naming the JSON path:
    `scX` or `scY` other than 1, a skew (`skX ≠ skY`), mesh, armature
    and bounding-box displays, IK, FFD and nested armatures.
- **`DragonBonesAtlas`**, a format of 1123's `Atlas` kind for
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
- Spine (a question for Nick, not a ticket);
- scale, skew, meshes, IK (2105 doesn't have them; a follow-up of 2105
  first);
- the binary DragonBones format (`.dbbin`).

## Implementation steps

1. Check the licenses, as above.
2. Read 2105's ADR and `Rig`, and 1123's sniffing rule for `.json`.
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
