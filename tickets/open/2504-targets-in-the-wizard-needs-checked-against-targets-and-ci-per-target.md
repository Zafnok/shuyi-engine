---
id: "2504"
title: "Targets in the wizard: pick several export targets, check the picked modules' needs against them, and give the new game a CI job per target"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2502", "3001", "3004"]
nick_input: none
completed:
---

# 2504 — Targets in the wizard

## Context

Nick, 2026-10-17 (`docs/decisions.md`): the wizard asks "what export
targets they want to support".
[ADR-0013](../../docs/adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)
§5 sets the rules:

- the picked targets fill the manifest's `targets` table (3001);
- the wizard runs the capabilities check (`capabilities_fit`, ADR-0012
  §1, the parked 3017) on the picked modules' `needs` against each
  picked target, and refuses a game that doesn't fit;
- the new game's CI checks and exports each picked target (3004);
- a target with its own duties brings a part (the Playdate's 1-bit
  patterns and controls check).

**From 3001 (done):** 2502 asks the targets question from the profiles
in `targets/`, each answer with its status, and refuses a planned one.
This ticket gives a planned profile its ticket, fills the manifest, and
adds the checks, the per-target CI and the target parts.

**Either order with 3017** (parked): if 3017 is done, call its
`capabilities_fit` as below. If not, skip that check, add a `limits`
entry ("modules not checked against targets: 3017 isn't built"), and
3017 wires it in when it lands (its scope says so). Never add a second
per-module list of targets: `needs` is the one place a module says what
hardware it wants.

## Nick input

None.

## Scope

**In:**
- **A planned target's ticket.** 2502's targets question shows a
  `planned` profile with its ticket and refuses it by flag (ADR-0013
  §6), but `Status::Planned` holds no ticket (3001). It gains the
  tickets that add the target, as `Planned(tickets: ["3014"])`: 3014
  for `switch`, 3015 for `android` and `ios`, and the `gba`, `nds` and
  `dsi` profiles' too if 3017 is done (if not, 3017 writes them, its
  clause says so). A bare `Planned` still reads, so a game's own
  profile keeps loading.
- **The check**, in the composer, if 3017 is done: for each picked
  target, `capabilities_fit(profile, needs)` over the picked modules'
  `needs` (and the template parts' stage needs, if a part declares any).
  Any problem is `ComposeError::DoesNotFit { target, problems }`, which
  prints 3017's own error text. It runs before anything is written.
  `left_out` entries are printed as warnings.
- **The manifest:** for each picked target, a `targets` entry `(frame:
  (w, h), scale: integer | fit, text_px)`. **From 3001 (done):** a
  profile has no default frame or scale (`display.sizes` is empty for
  `desktop` and `web`), so the entry takes the template's frame and
  font cell, as the templates' `desktop` and `web` entries do (384 ×
  384, `integer`, 16). That frame is taller than the Playdate's 240 px,
  so `target-playdate` brings its own entry (below).
- **The new game's CI** (`.github/workflows/ci.yml` in the base part)
  gets a matrix over the picked targets (through 3050's `export.yml`
  and plan if 3050 is done), each running `shuyi check --target <id>`
  and `shuyi export <id>` (3004). For `desktop`, the export entries are
  3004's desktop exporters, each on the runner the engine's own CI
  uses: `windows`, `linux` inside the Steam Linux Runtime ("sniper")
  SDK container, and `macos` (the universal app) on `macos-latest`.
  Copy the container line from the engine's `.github/workflows/ci.yml`
  (its `export` job), so a game's Linux package has the engine's glibc
  floor. **Either order with 3071:** if 3071 is done, the matrix also
  has a `linux-arm64` entry on `ubuntu-24.04-arm`, in the container
  3071's leg uses, as a commented line the developer turns on; if not,
  nothing to do (3071 adds that line when it lands). A target whose
  exporter needs a tool CI can't have (the Playdate SDK) runs the check
  only, and the workflow says why in a comment.
- **Either order with 3050** (release a game from a tag): if 3050 is
  done, the new game's per-target CI job calls the template's
  `.github/workflows/export.yml` (3050). The matrix comes from `cargo
  shuyi export --plan --json`, so the game's targets are named once, in
  `shuyi.ron`. The Playdate's check-only comes from the plan's `ci:
  check-only` (or from its `limits` entry while 3010 has no Playdate
  exporter), and no matrix is written into `ci.yml`. If the plan lists
  `linux-arm64`, a `jq` filter on it drops that entry, and a commented
  line keeps it for a developer who turns it on. If 3050 isn't done,
  3050's scope says it does this when it lands.
- **Target parts** in the built-in template:
  - **From 3002 and 3038 (done):** the templates' tools offer their
    controls (`GameTools::controls`), and their keymaps have an empty
    `targets: {}`. `playdate` and `steam-deck` have no keyboard, so
    `check --target` fails a game that lists one without a table of
    buttons for it, and so does the template's
    `every_screen_declares_its_actions_and_they_fit_every_target`
    test. Each part below adds its target's table to the keymap's
    `targets`: `"playdate"` on the d-pad, A (`East`) and B (`West`),
    and `"steam-deck"` on the pad, with every screen's actions on it;
  - `target-playdate`, when `playdate` is picked: its `shuyi.ron`
    entry, with a frame that fits 400 × 240 (or `scale: fit` and a
    `text_px` that passes), its keymap table, and a pattern map with
    the template's roles, `files/palette@onebit.ron`. **From 3006
    (done):** `check --target` checks the map only through
    `GameTools::one_bit`, which neither template's tools offer yet
    (grid's `Tools`, `BasicTools`), so the check only says the one-bit
    patterns aren't checked. The part's tools offer the map, and
    `frame_one_bit` for `frame-png --target playdate`, as
    `examples/hello-grid/src/tools.rs` does (`BasicTools` could offer
    the map when the game's files hold one);
  - `target-steam-deck`, when `steam-deck` is picked: if 3012 is done,
    its export in the matrix; if not, 3012 adds it (its scope says so).
- **The JSON** of `--questions` (2502) gives each module's `needs` and
  each target's capabilities, if 3017 is done, so an agent can explain a
  refusal before it happens. If not, 3017 adds them (its clause says
  so).

**Out (do not do):**
- new exporters or backends (3004, 3010, 3012);
- choosing frame sizes for the developer beyond the template's frame
  and the Playdate part's entry;
- parked targets (3014–3016, 3021, 3024) beyond showing them as planned;
- the capabilities types and the check themselves (3017).

## Implementation steps

1. Give a planned profile its tickets (`Status::Planned`).
2. Add the check to the composer, before writing, or the `limits` entry
   if 3017 isn't done.
3. Write the manifest entries and the CI matrix in the base part (the
   calls to 3050's `export.yml`, from its plan, if 3050 is done).
4. Write the Playdate part (its `shuyi.ron` entry, keymap table and
   pattern map, and tools that offer the map) and the Steam Deck part
   (its keymap table, and its export if 3012 is done).

## Acceptance criteria

- [ ] `shuyi new demo --no-input --targets desktop,web,playdate` makes a
      game whose manifest lists three targets. Its CI checks each of
      them and has an entry per exporter of each picked target: it
      exports `windows`, `linux` (in the sniper container), `macos` and
      `web` (from 3050's plan if 3050 is done, where `desktop` gives the
      first three). The Playdate entry is a check only.
- [ ] If 3017 is done: a fixture module with `needs.three_d =
      "software"` picked with `--targets playdate,web` passes, and with
      a fixture profile of `three_d: none` fails before writing, with
      3017's error text. If 3017 isn't done: the `limits` entry is
      there.
- [ ] A planned target by flag (`--targets switch`) fails with its
      status and ticket 3014.
- [ ] The new game made with `desktop,web` passes `shuyi check --target
      all` in CI.
- [ ] The game made with `--targets desktop,web,playdate,steam-deck`
      passes `shuyi check --target all`, with no controls problem and
      no limit saying the controls or the one-bit patterns aren't
      checked.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the needs × target check (or its `limits` entry); manifest
  entries from the template and the target parts; a planned profile's
  tickets, and a bare `Planned` still read.
- Integration: the three-target game above; the refusal; the CI job
  builds the `desktop,web` game and runs its checks.

## Completion notes
