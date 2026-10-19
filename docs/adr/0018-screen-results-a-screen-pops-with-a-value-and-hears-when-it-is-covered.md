# ADR-0018: Screen results: a screen pops with a value, and hears when it is covered

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related tickets:** 2306 (game#1158, game#1159)

## Context

A game often asks the player something on a screen of its own (a yes or
no, a number, a pick from a list) and needs the answer back in the screen
that asked. Until now `Transition::Pop` carried nothing, and nothing told
a screen that the one above it had gone. The game's first engine-friction
sweep (game#1153) found about 280 lines built by hand to get around it:
a flow screen that holds its children itself and reads a `result()`
getter after each closes, pop-ups kept as `Option` fields so their answer
can change the screen below, and screens that mark themselves stale
before pushing to notice they are back.

Flutter's `Navigator.pop(context, result)` and Godot's signals do this
job in other engines.

## Decision

In `shuyi_ui::screen` (`crates/ui/src/screen.rs`):

- **`Transition::PopWith(ScreenResult)`**, made with
  `Transition::pop_with(value)`. `ScreenResult` wraps a `Box<dyn Any>`
  with the value's type name (for `Debug`); the screen below reads it
  with `take::<T>() -> Option<T>`, `get::<T>()`, `is::<T>()` or
  `downcast::<T>() -> Result<T, ScreenResult>`. The two screens agree on
  the type; nothing else checks it.
- **`Screen::covered(&mut self, ctx)`**: called on the top screen when
  another is pushed over it. Not called for `Replace`: the screen below
  the one replaced stays covered.
- **`Screen::uncovered(&mut self, ctx, result: Option<ScreenResult>) ->
  Transition<G>`**: called on the new top screen after a `Pop` (`None`)
  or a `PopWith` (`Some`). **It returns a transition**, which the stack
  applies at once, as if from `update`. So a screen acts on the answer in
  the same frame: a flow pushes its next screen, a stage that hears "yes,
  leave" pops itself, a question asked over a question passes its answer
  on down. Applying loops, not recurses.
- Both have defaults that do nothing (`Transition::None`), so existing
  screens are unchanged.
- **`ScreenStack::apply_in(ctx, transition)`** applies a transition and
  makes these calls; `ScreenStack::update` uses it, and so do `Game`'s
  hook screens (the first-key and debug screens cover the top screen like
  any push). `ScreenStack::apply(transition)` and `ScreenStack::push`
  keep their old meaning, the stack alone with no context: for building a
  stack and for tests. `apply` drops a `PopWith` value.
- A pop that empties the stack quits, with or without a value, and tells
  nobody; `Quit` tells nobody.

## Consequences

- A question is a screen of its own, and its answer comes back where it
  was asked: no getters, no `Option` pop-ups, no stale flags. Hello,
  Grid's "Leave the stage?" is the example (`screens::Leave`).
- The harness drives `Game`, so tests of a screen see the same calls.
- `PopWith` is a new variant of a public enum: a game that matches
  `Transition` with no wildcard arm adds one at its pin bump. Nothing
  else breaks.
- Every `uncovered` that does something ends with `Transition::None`:
  one line, the price of acting in the same frame.
- The result is typed only at run time. A screen that pops with a
  `u8` where the screen below expects a `u16` gives `None`, not a compile
  error; tests of the two together catch it.

## Alternatives considered

- **`uncovered` returns nothing** (the ticket's first shape). The screen
  below would keep the answer and act on its next update: a frame late,
  with the screen below drawn alone for that frame (a flicker between two
  screens of a flow), and one more field per screen.
- **A typed result** (`Screen<G, R>` or an associated type). The stack
  holds `Box<dyn Screen<G>>` of many kinds, so every screen would need
  the same result type, which would be a game-wide enum: more code in
  every game for little more safety.
- **A result type on `GameData`** (one enum per game). Same as above,
  and it ties the engine's own screens to each game's enum.
- **Changing `ScreenStack::apply` to take the context.** Cleaner to read,
  but it breaks every caller of `apply`; `apply_in` beside it is
  additive.
