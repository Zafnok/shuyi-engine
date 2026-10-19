---
id: "2306"
title: "A screen hands a result to the one below when it pops, and the one below hears that it is uncovered"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2306 — A screen hands a result to the one below when it pops

## Context

- **The game tickets:** game#1158
  ([`1158-the-flow-and-the-title-on-screen-results.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1158-the-flow-and-the-title-on-screen-results.md))
  and game#1159
  ([`1159-the-battles-rewind-and-guides-as-screens-on-the-stack.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1159-the-battles-rewind-and-guides-as-screens-on-the-stack.md))
  wait for this one. Found by the game's first engine-friction sweep,
  game#1153.
- **The job:** "ask the player something on a screen of its own, and
  get the answer back when it closes".
- **Friction:** the game's flow screen, 539 lines against an estimate
  of 390 (1.4×), its title (1.1×) and its main play screen (2009
  against 1700, 1.2×), outcome 1. Ranked third of the sweep's findings:
  about 10 screens shrink.
- **What the game does today.** `Transition::Pop` carries nothing
  (`crates/ui/src/screen.rs`, `enum Transition`), and nothing tells a
  screen that the one above it has gone. So:
  - the game's flow screen holds its child screens itself, in an
    11-variant enum, forwards `update` and `draw` to each by hand, and
    after each one pops, takes it back and reads a `result()` getter
    (about 120 lines); eight of its screens keep a result field and a
    getter only for that;
  - two pop-ups of the play screen are `Option` fields instead of
    screens on the stack, with key priority written out by hand, because
    their answer must change the screen below (about 120 lines);
  - the title screen marks itself `stale` before it pushes and
    refreshes on its next update, and another screen calls
    `set_under_scene(false)` at the start of every update, both to
    rebuild "I was covered, now I'm back" (about 40 lines).
- **The generic shape:** a screen pops with a value, and the screen
  below receives it; a screen hears when it is covered and uncovered.
  Flutter's `Navigator.pop(context, result)` and Godot's signals do the
  same job.
- **Outside tactics:** a farming game's shop asks "how many?" on a
  number screen and gets the number back; a platformer's pause menu
  hears that the map screen above it closed and redraws its list.

## Nick input

None.

## Scope

**In:**
- `Transition::PopWith(Box<dyn Any>)` (or a typed equivalent the
  session finds cleaner): the stack pops the screen and calls the new
  screen below's `Screen::uncovered(&mut self, ctx, result: Option<Box<dyn
  Any>>)`; a plain `Pop` calls it with `None`.
- `Screen::covered(&mut self, ctx)`, called when a screen is pushed on
  top of it (not for `Replace` of the top screen itself).
- Both have empty defaults, so no existing screen changes.
- A helper to read a result as a type: `result.downcast::<T>()` is
  enough if it reads well; otherwise a small `ScreenResult` wrapper.
- The harness (`crates/ui/src/harness.rs`) sees the same calls, so
  tests of a screen can drive a pushed screen to its result.
- Docs: the screen module's docs and one example screen (the smallest
  example game asks a yes/no on a pushed screen and reads the answer).

**Out (do not do):**
- A generic "pick one" screen; a game writes one in a few lines on top
  of this.
- Changing how the engine's own screens (options, Key bindings, the
  layout picker) report what happened, beyond using the new calls where
  they remove code.
- Any game's screens.

**For the game:** additive, nothing breaks. The game's tickets 1158 and
1159 move its pin past this and delete the hand-made versions.

## Implementation steps

1. Read `crates/ui/src/screen.rs` (`Screen`, `Transition`,
   `ScreenStack::update`) and the harness's driver.
2. Add the variant and the two methods with defaults; call them from
   the stack at push, pop and pop-with.
3. Make sure a pop that empties the stack (quit) and a `Replace` keep
   today's behavior.
4. The example and the docs.

## Acceptance criteria

- [ ] A test pushes a screen that pops with `7u8`; the screen below
      gets `Some(7)` in `uncovered`.
- [ ] A test checks `covered` is called once on push and `uncovered`
      once on pop, and neither on `Replace` of the top screen.
- [ ] Every existing test passes unchanged.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the stack's calls on push, pop, pop with a value, replace and
  quit.
- Integration: the example's yes/no through the harness.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
