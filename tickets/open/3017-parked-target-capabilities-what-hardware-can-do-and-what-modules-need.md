---
id: "3017"
title: "Parked: Target capabilities: each profile says what its hardware can do, each module says what it needs, and shuyi check --target names what doesn't fit"
type: feature
milestone: "Parked: export targets"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["3001", "2003"]
nick_input: none
completed:
---

# 3017 — Target capabilities and module needs

> **Parked** (Nick, 2026-10-17: "not all hardware supports all features"): until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title. It is unparked with 3D too, because 3104 and 3107 need it.

## Context

Nick, 2026-10-17 (`docs/decisions.md`): "not all hardware supports all
features", and a game that uses something its target lacks doesn't
export there. [ADR-0012](../../docs/adr/0012-gba-and-ds-targets-capabilities-indexed-colour-two-screens-and-homebrew.md)
§1 turns that into data and a check:

- `TargetProfile` ([ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
  §1, built by 3001) gains `capabilities`;
- each module's metadata ([ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
  §4, built by 2003) gains `needs`;
- `shuyi check --target <id>` fails when a game turns on a module, or a
  stage uses a feature, that the target can't run. The error names the
  module or stage, what it needs, and what the target has;
- an `optional` part (an HD-2D effect, [ADR-0011](../../docs/adr/0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md)
  §7) is left out and listed, never failed;
- the engine never drops a required feature silently.

This is the same rule as the controls check (3002): what doesn't fit
doesn't ship. The GBA and DS tickets (3019–3026) and the 3D tickets
(3103, 3107) all read these capabilities, so this comes first.

## Nick input

None.

## Scope

**In:**
- **`Capabilities`** in `shuyi_assets::target`, a new field of
  `TargetProfile`:
  - `colour`: `Rgba8`, `OneBit`, or `Indexed { banks, colours_per_bank,
    bits_per_pixel }`;
  - `screens`: a list of `Surface { id, size, touch }`, where `size` is
    `Any` or `Fixed(w, h)` in device pixels;
  - `three_d`: `None`, `Software { max_triangles }`, `Hardware {
    max_triangles, max_texture_bytes, effects }` or `Full`;
  - `sprites`: `None` (no limit) or `Some(SpriteLimits { max,
    max_per_line })`, with `max_per_line` an `Option`;
  - `memory`: `ram_bytes` and `video_ram_bytes`, each an `Option<u64>`
    (`None` means no fixed limit; the heap budget still applies);
  - `audio`: `channels` (`Option<u8>`) and `music` (`Streamed` or
    `Mixed`).
- **`display.color` moves to `capabilities.colour`.** The profile
  format's version goes up. An older file is read with a migration, so a
  game's own profile keeps loading. If 3006 has landed, its 1-bit check
  reads the new field; if not, 3006 reads it when it lands.
- **Every profile gets its capabilities**, with a comment on each
  provisional number naming the ticket that checks it:

  | Profile | `colour` | `screens` | `three_d` | `sprites` | `memory` (RAM / video RAM) | `audio` |
  | ------- | -------- | --------- | --------- | --------- | -------------------------- | ------- |
  | `desktop` | `Rgba8` | one, `Any` | `Full` | none | none / none | none, `Streamed` |
  | `web` | `Rgba8` | one, `Any` | `Full` (WebGL 2) | none | none / none | none, `Streamed` |
  | `steam-deck` | `Rgba8` | one, 1280 × 800, `touch: false` | `Full` | none | 16 GB / none | none, `Streamed` |
  | `playdate` | `OneBit` | one, 400 × 240 | `Software { max_triangles: 300 }` (prov., ADR-0011 §8) | none | 16 MB / 0 | none, `Streamed` |
  | `switch` (planned) | `Rgba8` | one, 1280 × 720 | `Full` | none | to be filled by 3014 | to be filled by 3014 |
  | `android`, `ios` (planned) | `Rgba8` | one, `Any`, `touch: true` | `Full` (prov.) | none | none / none | none, `Streamed` |

  The Steam Deck's screen is a touchscreen, but Deck Verified wants
  nothing to need it (ADR-0008 §1), so it says `touch: false` with a
  comment saying why.
- **Three new profiles, `status: planned`**, from ADR-0012 §2 and §3, so
  that the bake (3019), the CI builds (3020) and the two-screen work
  (3023) have a real profile to read. The backend tickets (3021, 3024)
  check every value and complete them:
  - `targets/gba.ron`: 240 × 160, `Indexed { banks: 16,
    colours_per_bank: 16, bits_per_pixel: 4 }`, `three_d: None`,
    sprites `{ max: 128, max_per_line: Some(32) }` (prov., 3018), RAM
    294 912 (32 KB + 256 KB) and video RAM 98 304, audio 2 channels
    `Mixed`, text 8 / 8 px, 60 / 30 fps, inputs as in ADR-0012 §2,
    `backend: shuyi-gba`;
  - `targets/nds.ron`: two surfaces of 256 × 192, the lower one
    `touch: true`; `colour` is the 2D layers' `Indexed { 16, 16, 4 }`,
    and the 3D engine's 15-bit colour is part of `three_d`;
    `three_d: Hardware { max_triangles: 2048, max_texture_bytes: 524288
    (prov.), effects: [fog, edge_marking, toon, alpha] }`; sprites
    `{ max: 128 }` per screen; RAM 4 MiB, video RAM 671 744 (656 KB);
    audio 16 channels `Mixed` (prov., 3018); text 8 / 10 px; inputs as
    in ADR-0012 §3; `backend: shuyi-nds`;
  - `targets/dsi.ron`: as `nds`, with 16 MiB of RAM.
- **`Needs`**, in `shuyi_assets::target`, one type read from three
  places:
  - a module's `[package.metadata.shuyi.needs]` (2003's `ModuleInfo`
    gets the field);
  - the game manifest, for the whole game and per stage
    (`stages.<id>.needs`), for example `sprites = { at_once = 200 }`;
  - a skin's settings file, with an `optional` list for effects it can
    do without (3107's diorama skin uses it).

  Its fields: `colour` (`Any`, `Indexed { min_colours }` or `Rgba8`),
  `three_d` (a level, and optionally `triangles` and `effects`),
  `sprites` (`at_once`, `per_line`), `ram_bytes`, `touch`. Every field
  is optional, and an empty table means "runs on every target".
- **Every module states its needs.** `cargo xtask check-modules` (2003)
  fails when a module crate has no `needs` key at all, so "runs
  everywhere" is said on purpose, never by forgetting. The catalog
  `docs/modules.md` shows each module's needs.
- **The check**, a pure function `capabilities_fit(profile: &TargetProfile,
  needs: &[(NeedSource, Needs)]) -> FitReport`, wired into `shuyi check
  --target <id>` (an operation of the tool contract, 1129):
  - `NeedSource` is a module, a stage, the game, or a skin;
  - `FitReport` has `problems` (errors) and `left_out` (optional parts
    the target can't run);
  - `three_d` levels order as `None < Software < Hardware < Full`; a
    triangle count above the target's `max_triangles` fails; an effect
    not in the target's `effects` fails unless it is optional, and then
    it goes in `left_out`; `Full` has every effect;
  - `colour: Rgba8` fails on `Indexed` and `OneBit`; `Indexed {
    min_colours }` fails on `OneBit`, and on an `Indexed` target with
    fewer colours;
  - sprites, RAM and touch compare numbers and flags.
- **The error text** names the source, the need and what the target
  has, for example: "module shuyi-3d needs three_d: software; target
  gba has three_d: none", and "stage battle needs 200 sprites at once;
  target gba has 128".
- **`shuyi export <id>` refuses** a game with any problem, through the
  check it already runs (3004). The export report lists `left_out`.

**Out (do not do):**
- the screen-count and layout check (3023);
- the controls check (3002 has it);
- quantizing images to a palette (3019);
- any backend (3021, 3024);
- measuring or setting budgets (3008, 3020);
- making the engine drop or swap a feature by itself (ADR-0012,
  *Alternatives*).

## Implementation steps

1. Add `Capabilities`, `Surface`, `ThreeD`, `SpriteLimits`, `Memory` and
   `AudioCaps` to `crates/assets/src/target.rs`, with the version bump
   and the `display.color` migration. Extend the property test on random
   bytes (ADR-0007).
2. Add `capabilities` to the seven existing files in `targets/`, and
   write `gba.ron`, `nds.ron` and `dsi.ron` from ADR-0012.
3. Add `Needs` and its decoding from TOML (module metadata, the
   manifest) and RON (skin files).
4. Add `needs` to `ModuleInfo` and the rule to `check-modules`; add the
   needs column to `docs/modules.md`. Give every existing module crate a
   `needs` key.
5. Write `capabilities_fit` with a test per rule, then wire it into
   `check --target`, its JSON output and the CLI's text table.
6. Add fixture games: one with a 3D module on `gba`, one stage with 200
   sprites, one skin with an optional and a required effect on `nds`.

## Acceptance criteria

- [ ] Every file in `targets/` decodes, and a test checks the `gba` and
      `nds` values against ADR-0012 §2 and §3.
- [ ] A profile in the old format (with `display.color`) loads through
      the migration (fixture).
- [ ] A fixture game with a module that needs `three_d: software` fails
      `shuyi check --target gba` with the module, the need and
      "three_d: none" in the message, and passes on `desktop`.
- [ ] A stage that needs 200 sprites at once fails on `gba`, naming the
      stage.
- [ ] On `nds`, a skin with `bloom` optional and `fog` required passes,
      and `bloom` is listed in `left_out`. With `bloom` required, it
      fails.
- [ ] `shuyi export gba` refuses the first fixture, and prints the
      check's errors.
- [ ] `cargo xtask check-modules` fails on a fixture module with no
      `needs` key.
- [ ] The JSON output matches its schema (1129's schema test).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each rule of `capabilities_fit`, passing and failing; the
  `three_d` ordering; the migration.
- Property: random `Needs` against random profiles never panic, and an
  empty `Needs` fits every profile.
- Integration: `shuyi check --target` on the fixture games, through the
  CLI and the JSON contract.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
