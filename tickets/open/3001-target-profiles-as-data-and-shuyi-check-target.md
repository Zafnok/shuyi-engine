---
id: "3001"
title: "Target profiles as data: targets/*.ron, the TargetProfile asset kind, and shuyi check --target for screen size and text size"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["2201", "1129", "game#1131"]
nick_input: none
completed:
---

# 3001 — Target profiles as data

## Context

Nick, 2026-10-15 (`docs/decisions.md`, *export targets*): Visions of
Shuyi should run on the Playdate and the Steam Deck, and later on
Nintendo, mobile and consoles, with expected performance numbers per
target. [ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§1 makes each target a data file. Every other export ticket reads these
profiles, so this ticket comes first:

- the controls check (3002);
- `shuyi export` (3004);
- the 1-bit output (3006);
- the budgets (3008);
- the backends (3010, 3012).

The game's frame is 800 × 512 logical pixels today (100 × 32 cells of
8 × 16). That is too big for the Playdate (400 × 240) and doesn't scale
evenly to the Deck (1280 × 800). This ticket makes that visible as a
check.

## Nick input

None.

## Scope

**In:**
- **`TargetProfile`** in `shuyi_assets::target`. Fields as in ADR-0008
  §1: `id`, `status`, `display`, `frame_rate`, `input`, `budgets` and
  `backend`.
  - Its `Format` (`target-ron`, extension `target.ron`) is registered by
    default (2201).
  - Its unit types are integers: pixels as `u16`, milliseconds and bytes
    as `u32`/`u64`.
  - It takes `deny_unknown_fields`.
- **The engine's profiles** in `targets/`: `desktop`, `web`,
  `steam-deck`, `playdate`, `switch`, `android` and `ios`, with the
  values and the provisional budgets from ADR-0008's two tables.
  - `switch`, `android` and `ios` are `status: planned`.
  - The profiles are embedded in `shuyi-cli`, and a game may override
    one or add one in its bundle.
- **The game's own per-target settings.** The game manifest that `shuyi
  new` writes (1129) gets a `targets` table: for each target id the game
  wants, its frame size in logical pixels and its scale (`integer` or
  `fit`). A game lists only the targets it ships on.
- **`shuyi check --target <id>`**, an operation of the tool contract
  (ADR-0005), so the CLI, MCP and skills all get it. Two checks for now:
  - **Fits.** The frame times its scale fits the display inside the
    safe area.
  - **Text.** The smallest text on screen (the font's cell height × the
    scale) is at least `min_text_px`, which is an error, and at least
    `good_text_px`, otherwise a warning.

  Each problem names the target, the game's setting and the numbers.
  `--target all` checks every target in the game's manifest.
- **The `run-gates` skill** lists `shuyi check --target all` for
  examples that declare targets.

**Out (do not do):**
- the controls check (3002);
- colour checks (3006);
- the budget checks (3008);
- any backend;
- what each target's hardware can do (`capabilities`, the parked 3017,
  ADR-0012 §1). It later moves `display.color` into
  `capabilities.colour`, so keep the colour field easy to move.

## Implementation steps

1. Add `target.rs` to `shuyi-assets` with the types, the `Format` and a
   fuzz-style property test (ADR-0007 §5).
2. Write the seven profile files from ADR-0008, with a comment on every
   provisional number saying which ticket measures it (3011, 3013).
3. Add the `targets` table to the manifest format and to the `shuyi new`
   templates. The default lists `desktop` and `web`.
4. Add the `check --target` operation with its JSON output, and the
   text table for the CLI.
5. Give each example a `targets` table, and make it pass, or record why
   in the example's README.

## Acceptance criteria

- [ ] Every file in `targets/` decodes, and a test checks the
      `playdate` and `steam-deck` values against ADR-0008's tables.
- [ ] `shuyi check --target playdate` on a fixture game with an
      800 × 512 frame fails with "frame 800 × 512 at scale 1 is larger
      than the display 400 × 240".
- [ ] `shuyi check --target steam-deck` warns when the text is 9–11 px
      and fails below 9.
- [ ] The same operation answers through the JSON contract, and a test
      checks the JSON.
- [ ] Truncated or random bytes give errors, never a panic.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the fit and text maths at integer and `fit` scale, with safe
  areas.
- Property: decoding random bytes.
- Integration: `shuyi check --target` on fixture games, passing and
  failing.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
