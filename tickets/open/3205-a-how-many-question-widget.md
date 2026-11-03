---
id: "3205"
title: "A how-many question widget beside Ask: a number from 1 to a most, set by up and down, then Confirm or Cancel"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3205 — A how-many question widget

## Context

A request from the game.

1. **The game ticket:** game#1215, the game's shops ask how many with
   this widget
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1215-shops-ask-how-many-with-the-engines-question.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** ask the player "how many?" over the
   screen, a number from 1 to a most that up and down change, Confirm
   takes and Cancel backs out of.
3. **The friction numbers:** no friction review. The game/engine boundary
   audit of 2026-11-03 found the game's three copies of the stepping
   (about 20 lines each), its own view and a painter of 26 lines, none
   of them naming anything of the game; about 40 game lines go.
4. **The generic shape:** `widgets::Count`, next to `widgets::Ask` and
   built like it: a question by text key, a value in `1..=max` that
   doesn't wrap, a view of plain data, and a glyph painter.
5. **Examples from games that aren't tactics:** a farming game's "Sell
   how many turnips?", a survival game's "Drop how many arrows?", a
   crafting bench's "Make how many?".

## Nick input

None.

## Scope

**In:**
- `crates/ui/src/widgets/count.rs` with `count/view.rs`,
  `count/glyph.rs` and `count/tests.rs`, re-exported from `widgets.rs`
  (`Count`, `CountEvent`, `CountView`) and its module doc listing it.
- `Count` has private fields and constructors, as `Ask` does.
  `CountView` is `#[non_exhaustive]` with public fields and a `new`, so
  a game's own painter tests can build one (ADR-0039). `CountEvent` is a
  closed enum of two answers.
- Additive: no break, no `Breaking:` line. The game's PR that moves the
  pin follows this one (game#1215).
- The widget lists in `skills/shuyi-screen/SKILL.md` (*The engine's own
  screens and widgets*) and the book's
  `site/content/learn/book/games-and-screens.md`.

**Out (do not do):**
- Wrapping round, a step of more than one (left and right do nothing),
  typing the number in, and a lowest value other than 1.
- A line naming the keys inside the box (the screen's help line says
  them, as the game's does).
- A one-bit painter: this ticket adds only the glyph one.

## Implementation steps

1. **Read** `widgets/ask.rs`, `ask/view.rs`, `ask/glyph.rs` and
   `ask/tests.rs`; copy their shape.
2. **`Count`** (`count.rs`): `Count::new(key: &'static str, max: usize)`
   starts at 1, a `max` of 0 counting as 1; `at(n)` clamps to
   `1..=max`; `key()`, `value()`, `max()`; `ACTIONS =
   crate::input::NAVIGATION`.
   `handle<A: ActionSet, V>(&mut self, action: A, audio: &mut
   AudioQueue<V>) -> Option<CountEvent>`: `Up` adds 1 and `Down` takes 1,
   playing `MenuSound::Move` when the value changes and nothing at an
   end. `Confirm` gives `CountEvent::Chosen(value)`, `Cancel`
   `CountEvent::Cancelled`, and both leave their sound to the screen, as
   `Ask::handle` does. Anything else gives `None`.
3. **The view** (`count/view.rs`): `CountView { text, value, max, note:
   Option<String> }`, with `at_min()`, `at_max()` and `noted(note)`
   (the screen's line under the number: a total price, say). `view(ctx)`
   and `view_with_keys(ctx, keys, args)` fill the question as
   `Ask::view_with_keys` does.
4. **The glyph look** (`count/glyph.rs`): `rect(view, buf)` (the widest
   line plus a margin of two either side, centred), `paint(palette,
   view, buf)` in that box, and `paint_in(palette, view, rect, buf)` in
   a box the screen places. It fills `EngineColor::Panel`, frames it in
   `BoxStyle::Double` in `Focus`, and from the second row inside prints,
   each centred: the question in `Highlight`, `▲` (`TextDim` at the
   most, `Text` otherwise), `n / max` in `Text`, `▼` (`TextDim` at 1)
   and the note, if any, in `Text`. That is the game's look today, so its
   snapshots can stay the same.
5. **Expected size:** about 170 production lines (with docs), judged by
   `ask.rs` with its view and glyph (287); about 150 lines of tests.

## Acceptance criteria

- [ ] `Count` steps 1 to `max` without wrapping; Move sounds only when
      the value changes (a test reading `AudioQueue::pending`).
- [ ] Confirm gives `Chosen(value)` and Cancel `Cancelled`, neither with
      a sound; other actions give `None`.
- [ ] `new` with a `max` of 0, and `at` beyond either end, clamp (tests).
- [ ] Snapshots of the glyph look at 1, in the middle and at the most,
      with and without a note; the arrows dim at the ends (a test).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: stepping, the ends, the events, the sounds, the view's text
  with its placeholders filled.
- Snapshot: `paint` and `paint_in`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
