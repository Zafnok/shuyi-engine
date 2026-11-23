---
id: "3240"
title: "The player's pick of the picture: sharp pixels or fill the screen, an options row and a saved setting"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: ["2333"]
nick_input: none
completed:
---

# 3240 — The player's pick of the picture

## Context

Split off 2333 at its step 7 (its PR passed about 600 counted lines
without it). 2333 built the placement (`ScaleMode`, `place`,
ADR-0140), `GameSettings::DEFAULT_SCALE`, `Frame::scale` and the sharp
drawing in `shuyi-app`; a frame carries the game's default today. This
ticket lets the player choose, as `frame_cap` lets them choose the frame
rate (2321, done: the shape to copy).

The game that asked for 2333 (Visions of Shuyi, game#1320) sets its own
default; whether it offers the row is its own choice, through its
`engine-change` skill. A farming game or a platformer that fills the
window by default wants the same row for players who prefer pixels
square.

## Nick input

None. A game opts in to the row.

## Scope

**In:**
- `Settings::scale: ScaleMode` (`crates/ui/src/settings.rs`), beside
  `frame_cap`. `Settings::default()` takes the game's
  `DEFAULT_SCALE`, so "Restore defaults" (`Settings::restored`)
  returns to it.
- `present_game` (`crates/platform/src/app.rs`) sets `Frame::scale` from
  the settings in place of `DEFAULT_SCALE`; the loading card keeps the
  default (the settings aren't read yet).
- `ScaleMode::key`: the options screen's text key of each, as
  `TextSpeed::key` gives (`options.value.sharp`, `options.value.fill`).
- `OptionRow::scale()`: a choice row built with `OptionRow::choice` if
  2314 is done, else written like `OptionRow::text_speed`; label
  `options.row.scale` ("Picture"), values "Sharp pixels" and "Fill the
  screen". The English goes in the engine's `crates/ui/lang/en/ui.ron`
  if 2351 is done, else in the toy's and hello-grid's `en.ron`.
- `scale` in the toy's `File`, hello-grid's saved form
  (`examples/hello-grid/src/settings.rs`) and the `()` form's
  `EngineOnly` (`impl GameSettings for ()`), each `#[serde(default)]`
  so a file without it loads with the default; with 2110 done, a new
  `VERSION` of the `()` form and its fixture, as its guard asks.
- hello-grid's options screen gets the row after the map look; the
  templates don't.

**Out:** a different default per target; a remembered window size
(3044); the Fit capture (ticket 3241).

**For the game:** additive. `Settings` gains a public field but already
has a private one, so no game builds it field by field. The game's PR
that moves the pin follows this one.

**Estimated size:** about 500 lines (code 200, tests 200, docs 50, text
50).

## Implementation steps

1. Read `settings.rs`, `toy.rs`, `screens/options.rs`, 2321's
   `frame_cap` row and settings, and `examples/hello-grid/src/settings.rs`.
   Check which of 2110, 2314 and 2351 are done.
2. Add `Settings::scale`, its default and `restored`, and `ScaleMode::key`.
3. Set `Frame::scale` from the settings in `present_game`.
4. Add `OptionRow::scale` and its text; the row on hello-grid's options
   screen.
5. Save it in the toy's, hello-grid's and `()`'s forms.
6. Docs: `docs/platform-backends.md`'s `frame.scale` line says it is the
   player's setting; the ADR-0140 index row notes this ticket.

## Acceptance criteria

- [ ] `OptionRow::<Toy>::scale()` steps between the two values and sets
      `Settings::scale`; `Settings::restored()` gives back the toy's
      `Fit`.
- [ ] A `Harness` test: the toy's presented frame carries `Fit`; after
      `Settings::scale` is set to `Integer`, it carries `Integer`.
- [ ] A toy settings file without `scale` loads with `Fit`.
- [ ] hello-grid's options test (`tests/it/options.rs`) lists
      `options.row.scale` after `options.row.look`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Settings::default()` and `restored()` with the toy's default;
  the row; the saved forms with and without `scale`.
- Integration: the `Harness` test above; hello-grid's options test.

## Completion notes

*(Filled in by the session that completes the ticket.)*
