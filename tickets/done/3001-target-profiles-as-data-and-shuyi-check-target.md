---
id: "3001"
title: "Target profiles as data: targets/*.ron, the TargetProfile asset kind, and shuyi check --target for screen size and text size"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: done
blocked_by: ["2201", "1129", "game#1131"]
nick_input: none
completed: 2026-10-20
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

- [x] Every file in `targets/` decodes, and a test checks the
      `playdate` and `steam-deck` values against ADR-0008's tables.
- [x] `shuyi check --target playdate` on a fixture game with an
      800 × 512 frame fails with "frame 800 × 512 at scale 1 is larger
      than the display 400 × 240".
- [x] `shuyi check --target steam-deck` warns when the text is 9–11 px
      and fails below 9.
- [x] The same operation answers through the JSON contract, and a test
      checks the JSON.
- [x] Truncated or random bytes give errors, never a panic.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the fit and text maths at integer and `fit` scale, with safe
  areas.
- Property: decoding random bytes.
- Integration: `shuyi check --target` on fixture games, passing and
  failing.

## Completion notes

**What was done.**
- **`shuyi_assets::target`**: `TargetProfile` (`id`, `status`,
  `display`, `frame_rate`, `input`, `budgets`, `backend`), all integers
  and `deny_unknown_fields`, read by the `TargetRon` format (`target-ron`),
  which is registered by default. The decoder reports every problem of a
  profile:
  - an id that isn't the file's name;
  - text sizes or frame rates out of order;
  - an empty size, or a safe area that leaves nothing;
  - a chord of buttons the target doesn't have.

  A property test feeds it random, truncated and changed bytes.
- **`targets/`**: the seven profiles from ADR-0008's two tables. Each
  provisional number says which ticket measures it (3008, 3011, 3013,
  3015). `switch`, `android` and `ios` are `Planned`; the others are
  `Supported`, as the ticket says.
- **`shuyi.ron`** gets `targets`, a table from target id to `(frame: (w,
  h), scale: integer | fit, text_px)`, and `target_profiles`, an optional
  folder of the game's own profiles. A game's profile adds a target, or
  replaces the engine's with the same id. Both templates list `desktop`
  and `web`, and their CI runs `cargo shuyi check --target all`.
- **`check --target <id | all>`** is part of the contract's `check`.
  - `CheckInput.target` and `CheckOutput.targets` are new, each report
    holding its problems with level, check, setting and message.
  - The contract version goes to 2, with the schemas regenerated.
  - It runs on the project file alone, so no game tool program is built.
  - `--json`, the text table and the exit code all follow the other
    operations.
- **Hello, Grid** lists `desktop`, `web` and `steam-deck`, and passes all
  three. Its README says why the Playdate isn't listed: the 384 × 384
  frame is taller than 240 px. CI's `wasm` job and the `run-gates` skill
  run `check --target all` there.

**Deviations.**
- **The extension is `ron`, not `target.ron`.** The format registry
  refuses an extension with a dot in it (ADR-0019), and ADR-0008 names
  the files `targets/<id>.ron` anyway. Like every other RON kind, the kind
  asked for says which format reads the file.
- **`display.sizes` is a list.** It is empty for a window or a browser
  canvas, which takes any size; their text is checked at scale 1, and the
  answer says so in its limits. It has one entry per mode for the
  Switch's handheld and docked. ADR-0008 says "width and height".
- **Buttons are written as strings** (`"East"`), as keymap files write
  them. `Button` gained a serde `Deserialize` by its name.
- **The text check takes `text_px` from the game's `targets` entry**, not
  from its font. The CLI doesn't load the game's font, and a game may use
  a smaller font on one target.
- **The fit scale.** `integer` is the largest whole scale that fits.
  When even 1 doesn't fit, the error is the ticket's "frame W × H at
  scale 1 is larger than the display …", or "… the safe area … of the
  display …" when there are insets. `fit` is the largest factor, smaller
  than 1 too, and the text shrinks with it.

**API changes for the game** (none of its code is known to use them):
- `CheckOutput` gained `targets`;
- `CheckInput` gained `target`;
- `Config` gained `targets` and `target_profiles`;
- the contract is version 2.

No game PR is needed for this ticket. The game's next pin bump carries
it, and the game may then add its own `targets` table.

**Follow-up tickets.** None.

**For Nick.** The engine now knows each export target's screen, inputs
and budgets, and `shuyi check --target playdate` says whether a game's
screen fits. The game's 800 × 512 frame fails on the Playdate as
expected.
