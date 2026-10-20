---
id: "2217"
title: "Parked: Spine JSON and atlas import into the skeletal module's rigs, with a reader of our own written from Spine's public format documentation"
type: feature
milestone: "Engine: animation"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2105"]
nick_input: none
completed:
---

# 2217 — Spine JSON and atlas import

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): a follow-up of the parked skeletal module (2105), so it starts after 2105 is unparked and done. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-20 (`docs/decisions.md`, *the engine may read Spine's
files, with a reader of our own*): the engine may read Spine's files
with a reader we write ourselves, "written from the public format docs
with no Spine code", and the user brings their own Spine license.

Spine is the most used 2D bone tool. 2105 builds rigs in our own RON
(the source of truth, ADR-0007 §4), and 2216 imports DragonBones. This
ticket imports Spine's exports into the same `Rig`, so an artist's
Spine work goes straight in.

**The clean-room rule. Read this before anything else.**

- The Spine Editor License Agreement (Section 2) and the Spine Runtimes
  License restrict Esoteric's runtime libraries and works derived from
  them. This reader must be neither.
- **Do not open, read, search or copy anything in
  `github.com/EsotericSoftware/spine-runtimes`**, nor any crate that
  wraps it (`rusty_spine`, `bevy_spine`). Do not translate code from
  them.
- Work only from Esoteric's **public format documentation**: the JSON
  export format page (`esotericsoftware.com/spine-json-format`) and the
  texture atlas format page (`esotericsoftware.com/spine-atlas-format`).
  Record the date you read them and the Spine version they describe.
- Do not use Esoteric's example projects or their exports (spineboy,
  raptor and the rest) as fixtures. Write the fixtures by hand from the
  documentation.
- This is ADR-0021 §2's rule (written from specifications, never
  copied), applied to a file format.

## Nick input

None.

## Scope

**In:**
- **`SpineJson`**, a format of 2105's `Rig` kind, in
  `shuyi_bones::import` (behind the `import` feature 2216 adds; if 2216
  isn't done, add the feature here), registered as a standard format,
  with its id, extension, size limit and a property test that random
  bytes never panic it.
  - **Versions:** the current stable Spine version the documentation
    describes, and the one before it if the documentation shows the
    differences. `skeleton.spine` gives the version; another version is
    a clear error naming it.
  - **What it reads**, mapped to 2105's `Rig`:
    - `bones`: `name`, `parent`, `length`, `x`, `y`, `rotation`
      (degrees, to 1119's `Angle`);
    - `slots`: `name`, `bone`, `attachment` (at rest), `color` (to the
      nearest palette role only if the game gives a role map; otherwise
      ignored with a warning), draw order;
    - `skins`: `region` attachments with `x`, `y`, `rotation`, `width`,
      `height`, `path`;
    - `events`, and `animations`: bone `rotate` and `translate`
      timelines, slot `attachment` timelines, `events`, and
      `drawOrder` only if 2105 supports it (otherwise an error naming
      the JSON path). Times are seconds, converted to ms.
    - **Curves:** `stepped` is stepped; linear stays linear; a Bézier
      curve becomes linear with a warning naming the JSON path, unless
      2105 has gained curves by then.
  - **Numbers** are floats in JSON. Each is rounded to `Fx` once, at
    load, by one function with a test, so a rig loads the same on every
    machine (import is asset loading, not the simulation).
  - **Everything 2105 doesn't support is an error naming the JSON path:**
    `scaleX` or `scaleY` other than 1, shear, `inherit` other than
    normal, `mesh`, `linkedmesh`, `boundingbox`, `path`, `point` and
    `clipping` attachments, IK, transform, path and **physics**
    constraints, and deform timelines. Physics in particular is never
    written from anything but the documentation (Esoteric's staff have
    said a runtime that uses their physics code falls under the runtimes
    license).
- **`SpineAtlas`**, a format of 1123's `Atlas` kind for Spine's text
  `.atlas` files: pages, and per region its name, bounds, offsets and
  rotation, as the atlas format page describes.
- **`shuyi-cli import --print`** prints an imported rig as 2105's RON,
  and a test checks: import, print, read the RON back, equal rig
  (ADR-0007 §4).
- **Docs** (`docs/formats.md`, and the module's README):
  - the engine reads Spine's exported JSON and atlas files with its own
    reader;
  - **the user needs their own Spine license** to make those files, and
    Spine's own terms (no commercial use of Trial or Education work,
    Enterprise above the revenue threshold) are between the user and
    Esoteric;
  - "Spine is a trademark of Esoteric Software. Shuyi Engine is not
    affiliated with or endorsed by Esoteric Software.";
  - which Spine version and features are read, and the export settings
    an artist should use (JSON, not binary; region attachments only).
- **Fixtures**, written by hand from the documentation (ours, CC0): a
  three-bone arm with one animation, matching 2105's arm, and bad files
  for each unsupported feature.

**Out (do not do):**
- Spine's binary `.skel` format (a follow-up if a game asks);
- scale, shear, meshes, constraints and physics (each needs 2105 to
  support it first, as a follow-up of 2105);
- the official Spine Runtimes, or any crate built on them
  (`docs/decisions.md`: every game built with the engine would then need
  a Spine license);
- DragonBones (2216).

## Implementation steps

1. Read the clean-room rule above, `docs/decisions.md`'s 2026-10-20
   Spine entry, 2105's ADR and `Rig`, and 2216 if done.
2. Read Esoteric's JSON and atlas format pages, and only those. Record
   the dates and the version in the Completion notes.
3. Write the serde types for the JSON, from the documentation.
4. Write the conversion to `Rig`, with errors that name the JSON path.
5. Write the `.atlas` format.
6. Write the fixtures by hand; add the round trip and the property
   tests.
7. Write the docs.
8. Run the gates.

## Acceptance criteria

- [ ] The Completion notes say the reader was written from the public
      format pages only, with their dates and the Spine version.
- [ ] The fixture arm imports to the same `Rig` as 2105's RON arm.
- [ ] Each unsupported feature in a bad fixture gives an error naming its
      JSON path.
- [ ] Import, print as RON, read back: the same `Rig`.
- [ ] Random bytes never panic either format.
- [ ] `docs/formats.md` says the user needs their own Spine license, and
      carries the trademark line.
- [ ] No new dependency (`cargo deny check` passes unchanged).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: degrees to `Angle`; seconds to ms; float to `Fx` rounding at
  the edges; curve mapping; the version check.
- Property: random bytes; random valid rigs printed and read back.
- Snapshot / integration: the fixture arm at one time, drawn with
  2105's skin, equal to the same arm read from RON.

## Completion notes

*(Filled in by the session that completes the ticket.)*
