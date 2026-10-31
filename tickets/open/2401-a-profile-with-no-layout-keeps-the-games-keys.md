---
id: "2401"
title: "A profile with no layout of its own keeps the game's keys: a game with one layout never loses them on a switch"
type: bug
milestone: After MVP
model: implementation
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2401 — A profile with no layout of its own keeps the game's keys

## Context

- Found while working 2310 (the player profiles screen). No friction
  review: a bug.
- `Ctx::switch_profile` (`crates/ui/src/ctx.rs`, through
  `load_profile`) sets the layout in use to the profile's saved one
  (`settings.layout()`). A profile just added, or one that never chose a
  layout, has none, so `refresh_keymap` maps nothing: every action shows
  `! not mapped` and Cancel no longer works.
- A game with a layout picker opens it for such a profile (the game,
  game#1164, opens its first-launch picker after adding). A game with one
  layout and no picker, like `examples/hello-grid` (`LAYOUT`, set with
  `ctx.use_layout` at start), is left with no keys. Hello, Grid works
  around it in `Title::update` (`examples/hello-grid/src/screens.rs`):
  it calls `ctx.use_layout(LAYOUT)` when `ctx.layout()` is `None`.
- Outside tactics: any one-layout game (a platformer, a puzzle game)
  that lets a family share one copy.

## Nick input

None.

## Scope

**In:**
- A game can say its default layout, used whenever the profile in use
  has none: an optional `GameData` method (say
  `fn default_layout(&self) -> Option<Layout> { None }`), read by
  `Ctx::load_profile` and wherever else the layout in use is set from
  the settings.
- Hello, Grid gives its `LAYOUT` that way and drops its workaround in
  `Title::update`.
- **From 2511 (done):** a game on the short path has no `GameData` of
  its own: `BasicData` (`crates/ui/src/basic.rs`) is the engine's, so
  the game can't add the method. `BasicData` implements it as its
  keymap file's first layout (`keymap_def().first_layout()`), the one
  `BasicGame::ctx` (`crates/platform/src/basic.rs`) sets at start. If
  `Ctx::new` reads the default too, that `use_layout` call goes.

**Out (do not do):**
- What a profile holds or how it is saved (`crates/ui/src/profiles.rs`).
- The profiles screen (`crates/ui/src/screens/profiles.rs`).
- Saving the default layout into a profile's settings: it is used, not
  chosen.

## Implementation steps

1. Read `Ctx::new`, `Ctx::load_profile`, `use_layout`, `choose_layout`,
   `saved_layout` and `shown_layout` in `crates/ui/src/ctx.rs`, and the
   `first_key` hook in `crates/ui/src/game.rs` (it opens a picker while
   no layout is chosen: it must still do so for a game without a
   default).
2. Add the `GameData` method with a `None` default and doc comment.
3. In `load_profile` (and `Ctx::new` if it sets the layout), use the
   profile's saved layout, else the game's default.
4. Hello, Grid: implement the method; remove the workaround from
   `Title::update`; keep `tests/it/profiles.rs` passing.

## Acceptance criteria

- [ ] With a default layout, switching to and adding a profile with none
      keeps every action mapped (a toy-game unit test in
      `crates/ui/src/ctx/tests.rs`).
- [ ] A game on the short path keeps every action mapped after adding
      and switching to a profile with no layout (a test on
      `BasicData`).
- [ ] Without one, nothing changes (the existing first-launch and
      picker tests pass unchanged).
- [ ] Hello, Grid has no `use_layout` call in `Title::update`, and
      `tests/it/profiles.rs` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a switch to a new profile with and without a default layout,
  and on `BasicData`.
- Integration: Hello, Grid's `tests/it/profiles.rs`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
