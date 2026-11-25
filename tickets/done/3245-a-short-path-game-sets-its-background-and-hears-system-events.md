---
id: "3245"
title: "A game on the short path sets its own background policy and hears the system's events: BasicGame::background and BasicGame::on_system"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2739"]
nick_input: none
completed: 2026-11-25
---

# 3245 — A short-path game sets its background and hears system events

## Context

2739 wrote `examples/snake` on the short path (`BasicGame`, `BasicData`
and `run_game`, ADR-0036). Its ticket asked two things the short path
can't give:

- **a background policy of its own.** 3093 made the policy a const,
  `GameSettings::BACKGROUND` (`crates/ui/src/settings.rs`), which `App`
  copies into its private `background` field when it starts
  (`crates/platform/src/app.rs`, `background: <G::Settings as
  GameSettings>::BACKGROUND`). `BasicData`'s settings are `()`
  (`crates/ui/src/basic.rs`), so every game on the short path keeps
  `()`'s policy: a real-time game keeps running, and dies, while its
  window has no focus. Snake wants `Background::KEEP_RUNNING.paused()`.
- **a hook for the system's events.** `Start::on_system` takes one
  (`SystemHook<G>`), and 3042 sends `FocusLost`; but `BasicGame::start`
  builds its `Start` itself and `run_game` calls it, so a game on the
  short path can't add one. Snake wants to push its pause menu on
  `FocusLost`.

For now snake pauses when it is resumed after a suspend (it reads
`FrameInput::resumed`), and keeps running while unfocused. 3043 sets
`()`'s policy to slow the frames in the background for every short-path
game; this ticket lets a game choose more.

## Nick input

None.

## Scope

**In:**
- **`Start::background(policy)`**: the same start with `policy` in place
  of `G::Settings::BACKGROUND`, which `App` keeps from then on (its
  `background` field). Without it, nothing changes.
- **`BasicGame::background(policy)`**, a builder as `console` is, which
  `BasicGame::start` hands to `Start::background`, and the Harness of
  `BasicGame::harness` and `BasicTools::harness` applies too.
- **`BasicGame::on_system(hook)`**, a builder taking a
  `SystemHook<BasicData<A, T>>`, which `start` hands to `Start::on_system`
  and the Harnesses install (`Harness::with_system_hook`).
- **Snake adopts both:** `.background(Background::KEEP_RUNNING.paused())`,
  and a hook that pushes its pause menu over the stage on `FocusLost`;
  a Harness test of each (`h.system(SystemEvent::FocusLost)`: the menu is
  open, and no step ran while unfocused). Its README says so.
- The docs map's page of the short path, and `docs/targets/README.md`'s
  background section if it names the short path.

**Out (do not do):**
- A settings type of the game's own on `BasicData` (a `GameSettings`
  with options rows): a game that needs that leaves the short path.
- 3043's frame cap in the background.

## Implementation steps

1. Read 3093's and 3042's completion notes, `Start` and `App::new` in
   `crates/platform/src/app.rs`, `BasicGame` in
   `crates/platform/src/basic.rs`, and `BasicTools::harness`.
2. Add `Start::background` and the field `App` reads it from; a test that
   a start with `paused()` pauses an unfocused game whose settings say
   `KEEP_RUNNING`.
3. Add `BasicGame::background` and `BasicGame::on_system`, carried to
   `start` and to both Harnesses; tests of each.
4. Snake: both, its tests and README.
5. The `Behaviour:` line if any default changes (none should), and the
   gates.

## Acceptance criteria

- [x] A game on the short path that sets `background(paused())` pauses
      while unfocused, in the Harness and in `App`.
- [x] Its `on_system` hook hears `FocusLost` from `run_game`'s start and
      from the tools' Harness.
- [x] Snake opens its pause menu on `FocusLost`, and its snake doesn't
      move while unfocused.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Start::background` over `BACKGROUND`; the builders carry their
  values to `start`.
- Integration: snake's focus tests in `examples/snake/tests/it/`.

## Completion notes

Done as the steps say:

- `Start::background(policy)` and its field `Start::background`
  (`Option<Background>`; `Start` is `#[non_exhaustive]`, so the field is
  no break): `App::new` keeps it in place of `G::Settings::BACKGROUND`.
  `App::set_background` and `App::background` set and read the policy,
  and `Harness::with_background` sets it on a Harness, as
  `with_system_hook` sets the hook.
- `BasicGame::background` and `BasicGame::on_system`, builders as
  `console` is: `start` hands them to its `Start`, and the new
  `BasicGame::harness_on(game)` puts both on a Harness of one of its
  games. `BasicGame::harness`, `BasicTools::harness` and snake's
  `Tools::harness` (which seeds the first run, so builds its own game)
  all go through it.
- Snake sets `.background(Background::KEEP_RUNNING.paused())` and
  `.on_system(snake::ON_SYSTEM)`, which hands the stage `Cancel` on
  `FocusLost` (`Game::act`), the action that already opens its pause
  menu; on the title it does nothing. Tests in
  `examples/snake/tests/it/focus.rs`: the menu opens and the snake makes
  no step while unfocused, nor after the focus is back until Resume; the
  title opens no menu; the real start (`Harness::start`) pauses and opens
  the menu too. Its README says so, and its line count is the one
  `cargo xtask template-lines` measures now (937).
- Tests: `Start::background` over the toy's `KEEP_RUNNING`
  (`app/tests.rs`); the builders reach `start`, `App` and the Harness
  (`basic/tests.rs`); the tools' Harness (`cli/src/basic/tests.rs`).

Deviations:

- The platform prelude now also exports `Background` and `SystemEvent`,
  which a game on the short path names to use the two builders. A game
  that glob-imports the prelude and another module with an item of the
  same name sees the two as ambiguous where it uses the name; a name of
  its own still wins.
- `docs/targets/README.md` has no background section: nothing there to
  change. The book's *Saves and settings* chapter says how a short-path
  game sets its policy and hook, its *Targets and export* chapter names
  `BasicGame::on_system`, and `docs/platform-backends.md` names
  `Start::background` in the `FocusLost` row.
- No ADR: this carries ADR-0077's policy through the start the way
  `Start::on_system` already carries the hook.

Ticket 2787 names `BasicGame::background` and snake's hook as the
pattern its arcade part follows.

Breaking: none. `Start` gains a field but is `#[non_exhaustive]`; the
rest are additions.

Behaviour: none for a game that doesn't call the new builders: no
default changed.

Searched the game for the items touched (`gh search code --repo
Zafnok/visions-of-shuyi 'BasicGame'`, `'Start {'`, `'prelude::*'`): it
doesn't use `BasicGame`, builds no `Start` literal, and glob-imports only
its own preludes.

Game: nothing to adopt, the game isn't on the short path and builds no
`Start` literal.
