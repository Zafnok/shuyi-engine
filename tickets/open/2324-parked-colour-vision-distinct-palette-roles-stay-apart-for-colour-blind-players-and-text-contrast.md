---
id: "2324"
title: "Parked: Colour vision: a check that distinct palette roles stay apart for protanopia, deuteranopia and tritanopia, text contrast, and frame-png through each"
type: feature
milestone: "Engine: accessibility"
model: implementation
effort: medium
status: blocked
blocked_by: ["3006"]
nick_input: none
completed:
---

# 2324 — Colour vision: distinct roles stay apart

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): the game doesn't ask for it, so this waits until after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after 3006. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20 and Godot 4.6, under
accessibility: games offer colour-blind palettes. Shuyi can already
ship them: a game's colour themes are palettes
(`GameSettings::Theme`, `GameData::theme_def`), and the player picks one
on the options screen. What's missing is a way for the developer to know
a theme works.

[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§4 already has the idea: skins name colours by role, and groups of
roles must stay distinct (`team.player`, `team.enemy`, `team.ally`).
This ticket checks those groups as players with colour vision
deficiency see them, so a game knows that red and green sides look the
same to about one man in twelve.

**From 3006 (done, ADR-0026):** the groups don't live in the palette
file. A palette (`PaletteDef`) is only names and colours. The
`distinct` groups and the `text` roles are in the one-bit pattern map,
`palette@onebit.ron` beside the palette (`PatternMapDef`), which only
a game with a one-bit target writes. 3006's check,
`shuyi_assets::onebit::check(palette, map)`, runs in `shuyi check
--target <id>` on each `color: OneBit` target, through the game's
optional `GameTools::one_bit`. The game's tools offer no themes yet.

It is a tool (`shuyi-cli`), not the simulation, so it may use floats.

## Nick input

None.

## Scope

**In:**
- **`shuyi_cli::vision`:**
  - `Vision::{Protan, Deutan, Tritan, Achromat}` and
    `simulate(vision, rgb) -> rgb`, in linear RGB, with the matrices of
    Machado, Oliveira and Fernandes (2009) at full severity, written
    from the paper, with a citation in the docs;
  - distance in OKLab (Björn Ottosson, 2020, from the published
    formulas).
- **The check**, in `shuyi check` for every game, not per target
  (palettes and themes don't change with the target), beside 3006's
  `shuyi_assets::onebit::check` in the code:
  - **where the groups live**, decided here in an ADR that amends
    ADR-0026: either `distinct` moves to a file every game has, and
    both checks read it there (the palette file, past names and
    colours, with a new format version; or a new file beside it), or
    this check reads the pattern map's groups. Either way a game with
    no groups gets a line in the check's limits, never a silent pass;
  - the default palette and every theme reach the CLI through a new
    optional `GameTools` method, as `GameTools::one_bit` gives 3006's
    check its palette and map;
  - for the default palette and every theme, for each vision, every
    pair of roles in a `distinct` group is at least a set distance apart
    after simulation. The worker picks the distance on the fixtures,
    and writes the number and why in the module docs;
  - a vision that **no** theme passes is an error; a theme that fails a
    vision another theme passes is a note ("theme `deutan` covers
    deuteranopia"). So a game can ship one theme per vision;
  - **text contrast:** the file that holds the groups may list pairs,
    `contrast: [(text, panel), …]`. Each pair must reach a contrast
    ratio of 4.5:1 (WCAG 2.x relative luminance). Below is a warning.
    When a palette names `key` (2347), the pair `(key, key_bg)` is
    checked by default, or `(key, panel)` when the palette has no
    `key_bg`.
- **`frame-png --vision <protan|deutan|tritan|achromat>`** writes the
  frame as that vision sees it, after painting, so a person or an agent
  can look.
- **Fixtures:** a palette that fails deuteranopia, and a theme that
  fixes it.
- **The guide**, `docs/accessibility.md` (created here, or by 2322 or
  2323 if one is done first): one theme per vision, and never colour
  alone: a side also gets a shape or a glyph.

**Out (do not do):**
- recolouring a theme automatically ("daltonization");
- checking images and sprites (the check reads palettes only);
- the 1-bit check (3006) and indexed colour (3019).

## Implementation steps

1. Read ADR-0008 §4, ADR-0026 and `shuyi_assets::onebit` (the pattern
   map's groups and `check`), then write the ADR on where the groups
   live.
2. Write `simulate` and the OKLab distance, with tests against values
   worked by hand from the papers.
3. Write the check and the contrast rule, with the fixtures.
4. Add `frame-png --vision`.
5. Write the guide section.
6. Run the gates.

## Acceptance criteria

- [ ] The failing fixture fails for deuteranopia, naming the group, the
      two roles and the distance; with the fixing theme it passes, with
      a note.
- [ ] A game with no `distinct` groups gets a line in the check's
      limits, not a pass.
- [ ] A contrast pair below 4.5:1 warns, naming both roles and the
      ratio.
- [ ] `frame-png --vision deutan` on the toy game is committed and
      reviewed.
- [ ] `simulate` on white and black gives white and black for every
      vision.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `simulate` on a handful of colours against hand-worked values;
  OKLab distance of equal colours is 0; the contrast ratio of black on
  white is 21.
- Property: distances are symmetric and never negative.
- Snapshot / integration: the fixtures through the check; one
  `frame-png --vision` picture.

## Completion notes

*(Filled in by the session that completes the ticket.)*
