---
id: "2307"
title: "A list cursor: focus that wraps, skips disabled rows and stays valid after a removal, with a scroll window, under the menu"
type: feature
milestone: After MVP
model: opus-5.5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2307 — A list cursor with a scroll window

## Context

- **The game ticket:** game#1160
  ([`1160-the-games-lists-on-the-engines-list-cursor.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1160-the-games-lists-on-the-engines-list-cursor.md))
  waits for this one. Found by the game's first engine-friction sweep,
  game#1153.
- **The job:** "move a highlight up and down a list longer than its
  box".
- **Friction:** found in 15 of the game's screens and skins (ratios
  1.2× to 1.8×, outcome 1). Ranked first of the sweep's findings.
- **What the game does today:**
  - 13 hand-written cursors, 8 to 12 lines each, all built around
    `(i + n - 1) % n` and a move sound;
  - 6 scroll windows, 6 to 10 lines each, with four different rules:
    keep the focus in view; the focus at the bottom; the focus centred;
    free scroll. So the same list scrolls differently on two skins.
  - One screen copies the engine's `options::stepped` (the no-wrap
    case).
- **What the engine has:** the stepping twice, private:
  `widgets/menu.rs` (inside `Menu::handle`) and `screens/options.rs`
  (`stepped_index`). `Menu` has no scrolling, and `MenuView` paints
  every row.
- **The generic shape:** `ListCursor`, a focus over `n` rows that
  steps with or without wrap, skips rows a predicate rejects, clamps
  when rows are removed, and keeps a window of `rows_shown` rows that
  contains the focus by one named rule. `Menu` keeps its API and uses it
  inside.
- **Outside tactics:** a farming game's seed shop, a platformer's level
  select, a visual novel's save list: each is a list longer than its
  box.

## Nick input

None.

## Scope

**In:**
- `widgets/list.rs` (or `widgets/cursor.rs`): `ListCursor` with
  - `step(forward, wrap, enabled: impl Fn(usize) -> bool) -> bool`
    (whether it moved, so the caller plays the sound);
  - `set_len(n)` (clamps the focus after a removal);
  - `window(rows_shown) -> Range<usize>` with a `Scroll` rule:
    `KeepInView` (move only when the focus would leave the window) and
    `Centred`; the window state lives in the cursor, so it doesn't jump;
  - `window_by_heights(heights, room)` for rows of different heights
    (a backlog of wrapped lines).
- `Menu` uses it for its focus, and `MenuView` carries the window
  (`first_shown`, and whether there is more above or below) so a skin
  can paint arrows. The menu's glyph skin paints only the window.
- `screens/options.rs`'s `stepped_index` and `stepped` use it.

**Out (do not do):**
- Items that carry a value, or labels as text keys (2308).
- Pointer or wheel scrolling (1121).
- Any game's screens.

**For the game:** additive. `MenuView` gains fields, so a game that
builds one by hand adds them. The game's PR that moves its pin follows
this one (game#1160).

## Implementation steps

1. Write `ListCursor` with its doc and unit tests.
2. Move `Menu`'s focus onto it; keep `Menu`'s tests green unchanged.
3. Add the window to `MenuView` and the glyph skin (a long menu in a
   short box scrolls, with a mark above or below).
4. `options.rs` on it.

## Acceptance criteria

- [ ] Unit tests: wrap and no wrap, every row disabled but one, a
      removal at the end clamps, `KeepInView` moves the window by one
      at its edge, `Centred` keeps the focus in the middle, rows of
      different heights.
- [ ] Property test: the focus is always inside the window and always
      on an enabled row when one exists.
- [ ] `Menu`'s existing tests pass unchanged; a new snapshot shows a
      scrolled menu.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit and property, as above.
- Snapshot: a menu of 20 items in a box of 6 rows, focus on item 12.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
