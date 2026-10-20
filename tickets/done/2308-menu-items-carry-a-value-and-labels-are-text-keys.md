---
id: "2308"
title: "Menu items carry a value of the game's type, and their labels are text keys translated when the menu is painted"
type: feature
milestone: After MVP
model: opus-5.5
effort: medium
status: done
blocked_by: ["2307"]
nick_input: none
completed: 2026-10-19
---

# 2308 — Menu items carry a value, and labels are text keys

## Context

- **The game ticket:** game#1162
  ([`1162-the-battle-menus-carry-their-values-and-translate-when-painted.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1162-the-battle-menus-carry-their-values-and-translate-when-painted.md))
  waits for this one. Found by the game's first engine-friction sweep,
  game#1153.
- **The job:** "show a list of things the player can pick, in the
  player's language, and know which thing was picked".
- **Friction:** the game's input state machine for its main play
  screen, 1243 lines against an estimate of 925 (1.3×), outcome 1.
- **What the game does today.** `MenuItem.label` is a `String` baked
  when the menu is built (`crates/ui/src/widgets/menu.rs`). The game's
  input state is pure and doesn't know the player's language, so it
  builds its menus with English labels and rebuilds them, translated,
  every time it paints (a `told()` function and an `IN_PLAY` constant
  used 17 times, about 45 lines). The rebuild clones the whole input
  state each frame. And because an item doesn't say what it stands for,
  seven menus keep a parallel list beside the `Menu` and look the
  chosen index up in it.
- **The generic shape:** `Menu<T>` whose items carry a `T`, chosen as
  `MenuEvent::Chosen(T)` (or the index and a `value(i) -> &T`), and an
  item label that is either a text key with its arguments, translated
  by the skin when it paints, or literal text.
- **Outside tactics:** a farming game's seed list (each row is a seed
  id, its name in the player's language); a platformer's level select.

## Nick input

None.

## Scope

**In:**
- `MenuItem<T>` and `Menu<T>`, with `T = ()` keeping today's use
  working where it can (a type default), or a rename the session finds
  cleaner; the engine's own menus updated.
- `Label` = `Key { key, args }` or `Text(String)`; `MenuItem::new`
  keeps taking text; `MenuItem::keyed(key)` takes a key. The menu's
  view holds the label as is, and the glyph skin translates it with
  `Ctx::text_with` when it paints.
- Docs with an example: a list of values chosen by value.

**Out (do not do):**
- The list cursor and scrolling (2307, needed first: this ticket
  builds on the menu's new focus).
- Any game's menus.

**For the game:** breaking: `MenuEvent::Chosen` and `MenuItem` change
shape. The game's PR that moves its pin follows this one and fixes its
uses (game#1162).

## Implementation steps

1. Read `widgets/menu.rs`, its view and skin, and every use in the
   engine (`grep -rn "MenuItem::\|Menu::new" crates/`).
2. Add `T` and `Label`; translate in the skin.
3. Update the engine's own uses and examples.

## Acceptance criteria

- [x] A test builds a `Menu<Seed>`-style menu of an example enum and
      gets the enum value back on Confirm.
- [x] A snapshot shows a keyed label painted in a test language.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: chosen values, disabled items, keyed and literal labels.
- Snapshot: one keyed menu in two languages.

## Completion notes

- `shuyi_ui::text::Label` (`Key { key, args }` or `Text`) and
  `Ctx::label`, which words one through `Ctx::text_with`.
- `MenuItem<T = ()>` with `value` and `label: Label`; `MenuItem::new`
  and `disabled` take text, `keyed` takes a key, plus `with_arg`,
  `with_value` and `with_enabled`. `Menu<T = ()>` gives
  `MenuEvent::Chosen(T)` and `Menu::value(i)`.
- The view keeps the label; `widgets::menu::glyph::{size, paint}` and
  `dialogue::text_box::glyph::paint` now take `&Ctx<G>` and word labels
  when they paint.
- Engine uses: the debug menu (`Menu<usize>`), the layout picker
  (`Menu<Layout>`; `focused()` lost its unused `ctx`), the toy's lobby
  (`LobbyChoice`), and hello-grid's title and end menus (keyed, with
  `TitleChoice` and `EndChoice`; no rebuild with translated text).
- Module docs carry a seed list chosen by value. Tests: chosen values,
  disabled items, keyed and literal labels, `Ctx::label`, and snapshots
  of one keyed menu in English and the toy's debug language.
- Small addition: `check-text` treats the arguments of `MenuItem::keyed`
  and `Label::key` as keys, so a keyed menu isn't counted as literal text.
- [ADR-0020](../../docs/adr/0020-menu-items-carry-a-value-and-views-hold-labels-the-skin-translates.md)
  records the shape.
- **For the game:** breaking (`MenuEvent::Chosen`, `MenuItem`'s fields,
  the two skins' signatures, `LayoutPickerScreen::focused`). The game's PR
  that moves the pin is game#1162's; it doesn't exist yet. Not built
  against the game here.
- No follow-up tickets. Nick input: none.
