---
id: "2217"
title: "Parked: Spine import: the official Spine runtime in an opt-in module whose users bring their own Spine license, or a reader of our own (Nick picks when unparked)"
type: feature
milestone: "Engine: animation"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2105", "2336"]
nick_input: decision
completed:
---

# 2217 — Spine import

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): a follow-up of the parked skeletal module (2105), so it starts after 2105 is unparked and done. Its first step is Nick's pick between two paths (Nick, 2026-10-20: "park the ticket as TBD between essentials license and our own reader"). Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Spine is the most used 2D bone tool. 2105 builds rigs in our own RON,
and 2216 imports DragonBones. This ticket brings in Spine's exports.

`docs/decisions.md` has three 2026-10-20 entries on Spine; read all
three. In short:

- Esoteric's terms (the Spine Editor License Agreement, last updated
  2025-04-05, Section 2 and Exhibit A) allow its runtimes inside an
  engine. Each developer who builds with them needs their own Spine
  license, and with it may ship a game to players who have none.
  Esoteric's Nate confirms on its forum (thread 26587, 2024-07-25) that
  Essential or Professional is enough to put the runtimes into any
  number of one's own products, below the $500k revenue threshold.
- A reader we write ourselves from the public format docs is not the
  Spine Runtimes, so those terms don't bind it. The user still needs a
  Spine license to make the files.
- A free engine needs no license of Nick's to carry the runtime: the
  Spine Runtimes License (Exhibit A, second paragraph) lets a toolkit
  integrate it free of charge as long as each of its users has their own
  Spine license. Nick needs a license only to test the module himself.
- Nick leans to path A: "I have a feeling our own reader would cost
  more than $69 in tokens".
- **Visions of Shuyi never uses Spine** (Nick, 2026-10-20): the game
  never turns this module on, and its `cargo deny` (permissive licenses
  only, no exceptions) keeps it out.

| | A. The official runtime | B. Our own reader |
| --- | --- | --- |
| Spine features | all of them (meshes, IK, physics), matching the editor | only what 2105 supports (no scale, meshes, IK or physics until 2105 grows) |
| Cost | nothing for the engine; an Essential license, $69 (checked 2026-10-20), if Nick tests it himself | our time and tokens, now and at every Spine version |
| License | the Spine Runtimes License: an exception to ADR-0002 §8 for one opt-in crate | permissive, like the rest |
| Users | each developer who turns it on needs a Spine license (they need one to make the files anyway) | the same, for the editor |
| Determinism | poses in floats: fine for drawing, not for hitboxes | poses in `Fx`, like the rest of the engine |
| Targets | C code: desktop sure; the web and the Playdate to check | everywhere the engine runs |

## Nick input

**Decision**, with the `ask-nick` skill, before any code: path A or B.
Bring the table above, re-check the Essential price and the license
date, and say whether path A's runtime builds for the web (step A1).
Record the answer in `docs/decisions.md`. If it is A:

- **Setup:** if Nick wants to run the module's tests himself, he buys a
  Spine Essential license at `esotericsoftware.com/spine-purchase` in his
  own name (it is for one named user, and needs Enterprise once his
  revenue, investment or financing passes $500k in 12 months). The
  engine itself needs none.
- The decision records the exception to ADR-0002 §8 for the one crate.

## Scope

### Path A: the official runtime in `shuyi-spine`

**In:**
- **A1, first: what to bind.** Find the official C runtime's current
  Rust route: `rusty_spine` (it bundles `spine-c`; last released in 2024
  as checked 2026-10-20) if it supports the current Spine version, or
  bindings of our own to `spine-c` pulled at build time. Either way,
  **this repository holds no Spine code**: it comes from crates.io or
  Esoteric's repository when a game builds with the module. Check that
  it builds with the GNU toolchain on Windows, and for the web build
  (`wasm32`, which needs a C compiler for wasm). Write what you found in
  the Completion notes before going on.
- **The crate `shuyi-spine`** in `crates/spine`:
  - it takes Exhibit A's second paragraph: the runtime is integrated
    free of charge, and each developer who uses the module brings their
    own Spine license. Its `license` field and a `LICENSE` file say the
    Spine Runtimes License applies, include that license's text and
    copyright notice (Exhibit A requires it), and say the engine's MIT
    OR Apache-2.0 does not cover the runtime;
  - it is opt-in only: never a default feature, and in the `shuyi`
    crate (2003) only behind its own feature;
  - `deny.toml` allows the Spine license **for this crate only** (a
    `cargo deny` exception with a comment pointing to
    `docs/decisions.md`).
- **What it does:** loads a Spine export (JSON or binary, with its
  atlas) through the runtime, poses it at a time, mixes animations as
  the runtime does, and hands the skin a list of textured triangles and
  quads for a world layer (1120). The frame has solid polygons (2336)
  but no textured triangles: add textured triangles on 2336's pixel
  rule (pixel centres, half-open edges) through a `for_each_run` like
  `Shape`'s, with `frame-png` and the 1-bit renderers drawing them too.
- **Determinism:** poses are for drawing only. The docs say a game must
  never read bone positions from this module into its rules (use 2105's
  `Fx` rigs, or the game's own data, for hitboxes).
- **Docs:** the module's README says, at the top, that every developer
  who turns it on needs their own Spine license, that players don't, and
  that "Spine is a trademark of Esoteric Software. Shuyi Engine is not
  affiliated with or endorsed by Esoteric Software."
- **Test files:** made by Nick (or an artist with a license) in the
  editor, or the smallest hand-written ones. Not Esoteric's example
  characters unless their terms allow it in a public repository.

**Out:** the Playdate and other `no_std` targets; editing Spine files;
a Spine viewer tool; Visions of Shuyi (it never turns the module on).

### Path B: a reader of our own

**In:**
- **The clean-room rule.** Do not open, read, search or copy anything
  in `github.com/EsotericSoftware/spine-runtimes`, nor any crate that
  wraps it. Work only from Esoteric's public format pages
  (`esotericsoftware.com/spine-json-format` and
  `esotericsoftware.com/spine-atlas-format`), and record the date you
  read them and the Spine version they describe. Write fixtures by hand
  (ours, CC0).
- **`SpineJson`**, a format of 2105's `Rig` kind, in
  `shuyi_bones::import` (behind the `import` feature; either order with
  2216, whichever lands first adds it), with a property test that random
  bytes never panic it:
  - the current stable Spine version (`skeleton.spine`); another version
    is a clear error naming it;
  - bones (`name`, `parent`, `length`, `x`, `y`, `rotation` in degrees,
    to 2335's `Angle`), slots, skins of `region` attachments, events,
    and animations: bone `rotate` and `translate`, slot `attachment`,
    `events`; seconds to ms;
  - curves: `stepped` as `Steps(1)` and linear as `Linear` (2105's keys
    take a `shuyi_core::ease::Ease`, 2109), and a Bézier as `Linear`
    with a warning naming the JSON path, as 2216 does. `Ease` has no
    curve through points: keys that take the `Points` curve of 2135
    are a follow-up of 2105 first, so a Bézier stays `Linear` whether
    or not 2135 is done;
  - floats rounded to `Fx` once, at load, by one tested function;
  - everything 2105 doesn't support (scale, shear, meshes, clipping,
    constraints, physics, deform) is an error naming the JSON path.
- **`SpineAtlas`**, a format of the `Atlas` kind (2222's, which 1123 imports into) for the text
  `.atlas` file.
- **`shuyi-cli import --print`** gives the RON, and import, print and
  read back give the same `Rig` (ADR-0007 §4).
- **Docs** in `docs/formats.md`: the user needs their own Spine license
  to make the files, and the trademark line as in path A.

**Out:** the binary `.skel` format; scale, meshes, constraints and
physics (follow-ups of 2105 first).

### Either path

**Out (do not do):** DragonBones (2216); taking both paths.

## Implementation steps

1. Read the three 2026-10-20 Spine entries in `docs/decisions.md`,
   2105's ADR and `Rig`, 2109's `shuyi_core::ease`, 2336's pixel rule
   (`shuyi_ui::frame::shape`), and 2216 if done.
2. For path A, do step A1. Then ask Nick (above) and record the answer.
3. Path A: the crate, its license files and `deny.toml` exception, the
   loader and the triangle items, the docs, the tests.
   Path B: the serde types, the conversion, the atlas format, the
   fixtures and the round trip, the docs.
4. Run the gates.

## Acceptance criteria

- [ ] Nick's pick is in `docs/decisions.md`, with his words.
- [ ] Path A: `cargo deny check` passes with the exception for
      `shuyi-spine` only; a default build of every other crate pulls in
      no Spine code; a test export draws in `frame-png`, reviewed and
      committed; the README's license notice is there.
- [ ] Path B: the Completion notes say the reader was written from the
      public format pages only, with their dates; the fixture imports to
      the same `Rig` as 2105's RON arm; each unsupported feature gives an
      error naming its JSON path; random bytes never panic it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Path A: loading and posing a test export; the triangles drawn equal
  in `shuyi-app` and `frame-png` on a test frame; the fast 1-bit
  renderer equals the reference with triangles in its frames; two
  triangles that share an edge never both cover a pixel (property
  test); a build of the engine without the feature contains no Spine
  code (`cargo tree`).
- Path B: degrees to `Angle`; seconds to ms; float to `Fx` rounding;
  `stepped` to `Steps(1)`, linear to `Linear`, and a Bézier to
  `Linear` with its warning; random bytes; round trip.

## Completion notes

*(Filled in by the session that completes the ticket.)*
