---
id: "2411"
title: "A key or button let go while the game is suspended stays held after the resume"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-04
---

# 2411 — A key or button let go while the game is suspended stays held after the resume

## Context

Found while working 2137 (fixed steps and the input latch). While the
system has suspended the game (`SystemEvent::Suspend`), `App::run`
(`crates/platform/src/app.rs`) drains each frame's input and returns
before the game's frame (`drain(platform, &mut self.lists.input)`), so
`InputState` (`crates/ui/src/input/state.rs`) never hears a key or
button come up. A key held when the game is suspended, and let go before
it resumes, stays held for the game after `SystemEvent::Resume`, until
the player presses and lets go of it again. On a device a suspended app
may get no event for that key at all, so keeping the drained events
wouldn't fix it.

What a player sees, on any target that suspends (the Playdate's lock,
the Steam Deck's sleep, a phone's app switch):

- a held repeatable key repeats on its own after the resume: a menu's
  cursor runs down the list;
- `FrameInput::is_held` and `FrameInput::held` keep saying the action
  is held: a hold-to-fast-forward goes on;
- a tick stage's steps see it held. 2137's `TickDriver` forgets the
  presses it kept on a resume (`FrameInput::resumed`), but `held` is the
  frame's, so the toy's dot (`shuyi_ui::toy::tick::DotStage`) keeps
  moving.

`InputState::set_keymap` already lets go of everything held (since
2137, its actions come out as `released`), and a key still down after it
is ignored until pressed again. The same rule fits a resume: whatever
was held before the suspend is let go, and a key really still down
counts once it is pressed again.

## Nick input

None.

## Scope

**In:**
- `InputState::release_all(&mut self)`: lets go of every held key and
  button, the chord buttons waiting and the running repeat, and reports
  the actions it held as `released` after the next `update`, as
  `set_keymap` does. A key or button still down is ignored until it is
  pressed again. The `device` stays. `set_keymap` calls it, so the two
  can't drift.
- `Game`: the first frame after `Game::set_resumed` (which `App` calls on
  `SystemEvent::Resume`, 2137) calls `release_all` before it applies the
  frame's events, so a key pressed in that frame still counts.
- The docs: `App`'s module docs (what a resume does), `Game::set_resumed`
  and `InputState`'s.

**Out (do not do):**
- asking the platform which keys are down after a resume (a backend
  that can read the keyboard's state could keep a key held across a
  suspend): a ticket of its own if a game asks;
- focus loss (`SystemEvent::FocusLost`): the game keeps running and its
  window may still get the key's release; nothing changes there;
- anything in `TickDriver`: its latch already empties on a resume.

## Implementation steps

1. Read `App::on_system_event` and `App::run` (`crates/platform/src/app.rs`),
   `Game::frame`, `Game::step` and `Game::set_resumed`
   (`crates/ui/src/game.rs`), and `InputState::set_keymap` and `release`
   (`crates/ui/src/input/state.rs`).
2. Write `InputState::release_all` and make `set_keymap` use it. Unit
   tests beside `set_keymap_lets_go_of_what_it_held`
   (`crates/ui/src/input/tests.rs`).
3. Call it from `Game::step` when the frame is the first after a resume,
   before the events.
4. Harness tests (`crates/ui/tests/it/`): a held repeatable key on a
   screen that records its actions (`Recorder`, `tests/it/main.rs`),
   then `h.system(SystemEvent::Suspend)`, the key's release while
   suspended, `h.system(SystemEvent::Resume)` and a second of frames: no
   more repeats, and the action isn't held. The same on the toy's dot
   stage: the dot stops once, with one `stop` sound, and never moves
   after.
5. The docs, and the `document` skill over the change.

## Acceptance criteria

- [x] A key held across a suspend and let go during it gives no repeat
      after the resume, and `is_held` is false.
- [x] The toy's dot stops at the resume and doesn't move again until a
      direction is pressed.
- [x] A key pressed in the first frame after the resume counts.
- [x] `set_keymap` lets go as before (its tests pass unchanged).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `release_all` reports what was held as released and ignores a
  key still down until it is pressed again; a waiting chord button gives
  nothing after it.
- Integration: the two Harness tests above; a press in the first frame
  after a resume.

## Completion notes

- `InputState::release_all` lets go of every held key and button,
  pending presses, waiting chord buttons and the running repeat; the
  actions it held come out as `released` after the next update, and the
  device and bindings stay. It names every field, so a new one can't be
  missed. `set_keymap` now swaps the bindings and calls it.
- `Game::step` calls it on the first frame after `Game::set_resumed`,
  after the keymap sync and before the frame's events.
- Tests: two unit tests beside `set_keymap_lets_go_of_what_it_held`, one
  in `chord_tests.rs` (a waiting button gives nothing after it); Harness
  tests in the new `tests/it/resume.rs` (the `Recorder` now records
  `held` too) and in `tests/it/tick.rs` (the dot). All three integration
  tests fail without the call in `Game::step`.
- Deviation: the Harness runs a system event in a frame of its own, so
  "a key pressed in the first frame after the resume" drives `Game`
  directly (`set_resumed`, then a frame with the press), as `App` does.
- Docs: `App`'s module docs, `Game::set_resumed`, `InputState`'s,
  `docs/platform-backends.md`'s `Resume` row, and the book's *Actions*
  chapter (`site/content/learn/book/actions.md`).

No break: `release_all` is an addition, and `set_keymap` behaves as
before (its tests pass unchanged).
