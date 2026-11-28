---
id: "3255"
title: "Every screen can be found on the stack: as_any and as_any_mut come from a blanket supertrait, not a copy in each screen"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3255 — Every screen can be found on the stack

## Context

A request from the game.

1. **The game ticket:** game#1416, which deletes the game's copies once
   this lands
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1416-screens-found-without-as-any.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** a test or a system hook finds a screen of
   a given type on the stack (`ScreenStack::find::<T>()`,
   `Game::screen_mut::<T>()`) without that screen writing anything for
   it.
3. **The friction numbers:** sweep game#1397 (2026-11-27). The game would
   keep 0 lines. It has 31 copies of
   `fn as_any(&self) -> Option<&dyn Any> { Some(self) }` in 28 files and 7
   of `as_any_mut`, about 150 lines with their blank lines, at its pin
   230080d (`grep -rn "fn as_any(&self)" crates`). This engine has 22
   copies of its own (`grep -rn "fn as_any(&self)" crates examples
   templates`). A screen that forgets them is silently not found:
   `find` returns `None`.
4. **The generic shape:** `Screen::as_any` and `as_any_mut`
   (`crates/ui/src/screen.rs`, about line 125) default to `None`, so each
   screen opts in by hand. Instead, a supertrait
   `pub trait AsAny: Any { fn as_any(&self) -> &dyn Any; fn as_any_mut(&mut self) -> &mut dyn Any; }`
   with `impl<T: Any> AsAny for T`, and `pub trait Screen<G: GameData>:
   AsAny`. `ScreenStack::find` and `find_mut` then work for every screen.
   `Box<dyn Screen<G>>` is already `'static`, so no screen that can be
   pushed loses anything.
5. **An example from a game that isn't tactics:** a platformer's test
   finds its pause menu on the stack to read its focused row; a farming
   game's file-dropped hook finds its import screen. Neither should have
   to write two lines into every screen for that.

## Nick input

None.

## Scope

**In:**
- `crates/ui/src/screen.rs`: `AsAny` with its blanket impl, `Screen`'s
  supertrait, `find`/`find_mut` (and `Game::screen`/`screen_mut` if they
  exist) on `AsAny`; `Screen::as_any`/`as_any_mut` removed.
- Every engine screen's copy deleted (examples and templates too).

**Breaking:** yes. A game's `fn as_any` in an `impl Screen` no longer
compiles (no such method in the trait); the fix is to delete it. A screen
type holding a non-`'static` borrow can't implement `Screen` any more:
search the engine, its examples and the game for one first, and name
the result in the completion notes (the game's pin 230080d has none:
every screen is boxed into the stack). The ticket's `Breaking:` line
goes in the completion notes (ADR-0038 §6).

**Out (do not do):**
- Any other change to the `Screen` trait.
- A derive macro: a blanket impl needs none.

## Implementation steps

1. Add `AsAny`, make it `Screen`'s supertrait, route `find`/`find_mut`
   through it, and drop the two trait methods.
2. Delete the 22 copies; build every crate, example and template.
3. Update `skills/shuyi-screen/SKILL.md` and the book where they tell a
   screen to write `as_any` for tests (`grep -rn as_any skills site`).
4. The PR says the game's PR that moves the pin (game#1416) follows it.

## Acceptance criteria

- [ ] `grep -rn "fn as_any" crates examples templates` finds only
      `AsAny`'s own.
- [ ] A test pushes a screen that writes nothing for it and finds it
      with `ScreenStack::find::<ThatScreen>()` and `find_mut`.
- [ ] CI's `api` job reports the break, and the completion notes have its
      `Breaking:` line.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `find`/`find_mut` on a screen with no `as_any` of its own, and
  on a stack of several.

## Completion notes

*(Filled in by the session that completes the ticket.)*
