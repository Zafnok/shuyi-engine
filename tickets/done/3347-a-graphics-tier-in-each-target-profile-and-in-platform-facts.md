---
id: "3347"
title: "A graphics tier in each target profile and in PlatformFacts, previewed on the desktop with SHUYI_GRAPHICS"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-23
---

# 3347 — A graphics tier in each target profile and in PlatformFacts

## Context

- **The game ticket:** `game#1314`
  ([github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1314-each-target-has-a-graphics-tier-and-every-moving-look-a-still-fallback.md](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1314-each-target-has-a-graphics-tier-and-every-moving-look-a-still-fallback.md)).
  Nick (2026-11-22), with the game's tileset overhaul: the weaker
  targets (the Playdate, a GBA later) can't show many animations, so
  there should be "engine flags for how to deal w this", in place before
  the first moving looks are written. That request is his OK for this
  ticket.
- **The job in one sentence:** a game knows, on each target, how much it
  may move and draw beyond its stills, and can preview a weaker target's
  stills on the desktop.
- **Friction:** no friction review: a missing feature.
- **The generic shape:** a target profile says its **graphics tier**,
  `GraphicsTier::{Minimal, Reduced, Full}` (ordered, `Full` by default),
  and `PlatformFacts` carries it to the screens (ADR-0060). A moving
  look (a tile that loops through frames, particles, an effect over the
  screen) says the lowest tier it plays at, and a skin draws its still
  below that (`GraphicsTier::plays`). A debug build of `shuyi-app` shows
  another tier when `SHUYI_GRAPHICS` names one.
- **An example that isn't tactics:** a farming game's swaying crops and
  falling leaves play on a PC, and its Playdate port shows the crops
  still and no leaves; a platformer's waterfall loops on the Switch and
  is one frame on a GBA.

Today nothing says it. `TargetProfile` (`crates/assets/src/target.rs`)
has the display, frame rate, input, quit and budgets; `PlatformFacts`
(`crates/ui/src/platform_facts.rs`) has the target, windowing, input,
files and screen colour. `Ctx::idle_motion` turns looping motion off for
tests (On or Off), which is a different question: whether a screen left
alone may move, not how much a target can afford.

The parked tickets that will draw moving looks (2359 animated tiles,
2360 a tile-layer painter, 2103 particles, 2370 weather ambiences) are
all blocked on others, so none ships first to carry the tier: this
ticket does, and they read it. 3017's `capabilities` (parked) say what
a target's hardware can do; the tier is what a game chooses to spend
there, and stays beside them.

## Nick input

None.

## Scope

**In:**
- `shuyi_assets::target::GraphicsTier`: `Minimal < Reduced < Full`,
  `Full` the default, with `ALL`, `name`, `from_name` (any case),
  `is_full`, `plays(lowest)` and `Display`; what each tier allows in its
  doc comments, as a guide a game keeps to (the engine enforces none).
- `TargetProfile::graphics`, `#[serde(default)]`: every profile written
  before it still loads, as `Full`. `targets/playdate.ron` says
  `graphics: Reduced`; the others stay `Full`.
- `PlatformFacts::graphics` from the profile, `with_graphics(tier)`;
  serde default `Full`, and not written when `Full`, so a session record
  made before it reads the same. `shuyi_ui` re-exports `GraphicsTier`.
- `shuyi_platform::GRAPHICS_VAR` (`SHUYI_GRAPHICS`): `shuyi-app` reads
  it in a build with the debug tools (`shuyi_ui::DEBUG_TOOLS`) and shows
  that tier in place of its target's; a name it doesn't know is a
  `platform` warning, and the target's tier stays.
- Docs: `docs/targets/README.md` (a section and table),
  `docs/platform-backends.md` (the `facts` method), the book's
  *Where the game runs*, and the `shuyi-animate` skill.

**Out (do not do):**
- Any moving look itself (2359, 2360, 2103, 2370).
- A check that a game's moving looks name their stills: the game's own
  data says what a still is (game#1314 checks it there).
- A tier on the command line of `shuyi run` or `frame-png`: a game's
  own tool sets `with_graphics` on its harness.
- A GBA profile (3018 and later).

**Estimated size:** about 300 lines, with the tests, docs and this
ticket.

## Implementation steps

1. `GraphicsTier` and `TargetProfile::graphics` in `shuyi-assets`, with
   tests: every tier parses, a profile without the field is `Full`, an
   unknown name is refused, the order and `plays`.
2. `targets/playdate.ron`: `graphics: Reduced`.
3. `PlatformFacts::graphics`, `with_graphics`, the re-export; tests: each
   engine profile's tier, the round trip of a record with and without it.
4. `GRAPHICS_VAR` and `shuyi-app`'s preview (`facts::preview_graphics`,
   pure, tested; read in `run`).
5. The docs.

## Acceptance criteria

- [x] A profile's `graphics` parses; one without it is `Full`.
- [x] `PlatformFacts::from_profile` carries it: the Playdate's is
      `Reduced`, the web's and the Deck's `Full`.
- [x] A session record without `graphics` reads as `Full`, and one at
      `Full` writes no `graphics`.
- [x] `SHUYI_GRAPHICS=reduced` gives `Reduced`; an unknown name keeps the
      target's tier with a warning.
- [x] The book's chapter is updated (2802).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the profile field, the tier's order, names and `plays`; the
  facts per engine profile; the record round trip; the preview.

## Completion notes

- `shuyi_assets::target::GraphicsTier` (`Minimal < Reduced < Full`,
  `Full` the default) and `TargetProfile::graphics`; the engine's
  `targets/playdate.ron` is `Reduced`, every other profile `Full`.
- `PlatformFacts::graphics` (from the profile; not written in a record
  when `Full`) and `with_graphics`; `shuyi_ui` re-exports the tier.
- `shuyi_platform::GRAPHICS_VAR` (`SHUYI_GRAPHICS`): `shuyi-app` shows
  that tier in a build with the debug tools (`facts::preview_graphics`,
  read once in `run`, before the capture); an unknown name logs a
  `platform` warning and keeps the target's.
- Docs: `docs/targets/README.md` (*The graphics tier*),
  `docs/platform-backends.md`, the book's *Where the game runs*, and the
  `shuyi-animate` skill (a bullet and a rule).
- No deviation from the steps. No engine ADR: the tier is one more fact
  of a profile read into `PlatformFacts`, as ADR-0008 §1 and ADR-0060
  already decide; what each tier means is in the enum's docs and the
  targets page. The game's ADR-0131 records how its skins use it.
- No ticket: nothing is left over. The moving looks that read the tier
  are 2359, 2360, 2103 and 2370, parked; 3017's capabilities stay beside
  it.

Game: game#1314 moves the pin to this change.

Behaviour: shuyi-ui: `PlatformFacts::from_profile` of the engine's `playdate` profile gives `graphics: Reduced`, so its facts no longer equal a copy of that profile without the field (a game keeping its own copy of `targets/playdate.ron` adds `graphics: Reduced` to it).
Behaviour: shuyi-app: a build with the debug tools reads `SHUYI_GRAPHICS` at start and shows that tier in place of its target's.
