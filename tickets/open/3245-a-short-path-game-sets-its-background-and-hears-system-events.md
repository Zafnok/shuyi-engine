---
id: "3245"
title: "A game on the short path sets its own background policy and hears the system's events: BasicGame::background and BasicGame::on_system"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2739"]
nick_input: none
completed:
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

- [ ] A game on the short path that sets `background(paused())` pauses
      while unfocused, in the Harness and in `App`.
- [ ] Its `on_system` hook hears `FocusLost` from `run_game`'s start and
      from the tools' Harness.
- [ ] Snake opens its pause menu on `FocusLost`, and its snake doesn't
      move while unfocused.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Start::background` over `BACKGROUND`; the builders carry their
  values to `start`.
- Integration: snake's focus tests in `examples/snake/tests/it/`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
