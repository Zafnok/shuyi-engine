---
id: "2785"
title: "Key names stand out in the template's and Hello, Grid's hints: their help lines and questions in key spans, and a key colour in their palettes"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2347"]
nick_input: none
completed:
---

# 2785 — Key names stand out in the template's and Hello, Grid's hints

## Context

2347 (ADR-0122) gave the engine `StyledText`: a hint's key names are
`TextRole::Key` spans, which `GlyphBuffer::print_styled` paints with the
palette's optional `key` and `key_bg` (`Palette::text_colors_on`). It
switched every engine hint (the text box's corner and skip question, the
question widget's keys, the four engine screens' help lines) and
`examples/hello-grid`'s text box `WaitView`, which the workspace needed
to build. Its size valve (past 600 lines) moved its step 7 here: a new
game's hints, and the rest of Hello, Grid's, still print their keys as
plain text, and no palette of ours names `key`, so nothing a new game
shows stands out yet. The end goal (2347, 2026-10-19): an agent writes
`"{Confirm} start"` and the key stands out, with no extra code, on the
web and on the Playdate.

What holds today (read 2026-11-16, on 2347's branch):

- `templates/shuyi-default/base/crates/game/src/title.rs`: `TitleView`
  holds `play: Option<String>` and `help: String`, built with
  `ctx.text_with("title.play")` and `ctx.text_with("title.help")`, and
  printed with `screens::print_centred`.
- `templates/shuyi-default/stages/top-down-grid/crates/game/src/stages/top_down_grid.rs`:
  its view's `help: String` (line 237), built with `ctx.text_with(help)`
  (line 300) and printed with `buf.print` (line 374).
- `examples/hello-grid/src/skin.rs`: `TitleView`, `StageView` and
  `EndView` hold `help: String`, painted by `help` (line 81) through
  `line`, which uses `buf.print`; `QuestionView::answers: String`
  (line 44), built at `src/screens.rs:327` with
  `ctx.text_with("stage.leave.answers")` and painted by
  `paint_question` (line 150) in one string with the question.
- The palettes: `templates/shuyi-default/base/crates/game/files/palette.ron`,
  `templates/shuyi-default/stages/top-down-grid/crates/game/files/palette.ron`
  and `examples/hello-grid/files/palette.ron` have no `key`;
  `examples/hello-grid/files/palette@onebit.ron` has no role for it.

## Nick input

None. The engine's own examples and template pick their key colour; the
game's look stays game#0250's.

## Scope

**In:**

- The template's title (`help`, and `play`, which names `{Confirm}`) and
  its `top-down-grid` stage's help line hold `StyledText` built with
  `ctx.text_styled`, and print with `screens::print_centred_styled` or
  `buf.print_styled` and `ctx.palette.text_colors_on(fg, bg)`.
- Hello, Grid's `TitleView::help`, `StageView::help`, `EndView::help`
  and `QuestionView::answers` the same; `paint_question` prints the
  question and then its answers with `print_styled`, as one centred line
  as today.
- A `key` colour in the three palettes (`files/palette.ron`), distinct
  from every other name in each, so a new game's keys stand out; no
  `key_bg`, so the cells behind them stay the panel's.
- 1-bit (ADR-0026): Hello, Grid's `palette@onebit.ron` gives `key` a
  pattern, lists it among its `text` roles, and puts it in a `distinct`
  group with `text`; its one-bit check test still passes. The template
  has no pattern map, so nothing there.
- The template job's checks (`cargo fmt --all` doesn't format
  `templates/`).

**Out (do not do):**

- The engine's screens and widgets: 2347 did them.
- Key caps, bold, boxes (3215).
- The game's palette or look (game#0250, game#0254).

**Estimated size:** about 450 lines (code 200, tests 150, docs 100),
snapshots apart.

## Implementation steps

1. Read ADR-0122 and 2347's completion notes.
2. Switch the template's base title and its stage's help line; run the
   template job's checks.
3. Switch Hello, Grid's help lines and its leave question.
4. Add `key` to the three palettes and Hello, Grid's pattern map.
5. Review every snapshot change: only key cells move.

## Acceptance criteria

- [ ] Hello, Grid's snapshots change only in key cells: its help lines,
      its text box's wait corner and its leave question (reviewed).
- [ ] A new game from the template (`shuyi new`, both stages) builds,
      and its title's and stage's help lines paint their keys in `key`.
- [ ] Hello, Grid's one-bit check test passes with `key` in its pattern
      map.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass, the template job's
      included.

## Tests required

- Snapshot / integration: Hello, Grid's snapshots; a test of the
  template's game that its help line's key cells are in `key`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
