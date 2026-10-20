# ADR-0020: Menu items carry a value, and views hold labels the skin translates

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related tickets:** 2308 (game#1162)

## Context

The game's first engine-friction sweep (game#1153) found its main play
screen's input state machine 1.3× its estimate, for two reasons in the
engine's `Menu`:

- `MenuItem.label` was a `String` baked when the menu was built. The
  game's input state is pure and doesn't know the player's language, so
  it built English menus and rebuilt them, translated, on every paint,
  cloning the whole input state each frame.
- `MenuEvent::Chosen` gave an index, so seven of the game's menus kept a
  list beside the `Menu` to look up what the index stood for.

Both are general: any game whose logic builds menus away from the
player's language, and any list of things to pick (a farming game's
seeds, a platformer's levels), hits them.

## Decision

- `shuyi_ui::text::Label` is text a view holds before it is shown:
  `Label::Key { key, args }` (a text key and its placeholders' values,
  already text) or `Label::Text(String)` (shown as it is).
  `Ctx::label(&Label) -> String` words it: a key through
  `Ctx::text_with`, so placeholders that name actions get the player's
  keys too.
- `MenuItem<T = ()>` has a `value: T` and a `label: Label`.
  `MenuItem::new(text)` and `MenuItem::disabled(text)` keep taking text;
  `MenuItem::keyed(key)` takes a key, `with_arg` fills its placeholders,
  `with_value(v)` turns a `MenuItem<()>` into a `MenuItem<V>`, and
  `with_enabled(on)` sets whether it can be chosen.
- `Menu<T = ()>` holds `MenuItem<T>`; `handle` (for `T: Clone`) returns
  `MenuEvent<T>`, whose `Chosen(T)` is a copy of the chosen item's value.
  The index is still `Menu::focus()`, and `Menu::value(i)` reads one.
- `MenuItemView.label` is the `Label` as the menu holds it. The skins
  word it when they paint: `widgets::menu::glyph::{size, paint}` and
  `dialogue::text_box::glyph::paint` take the `&Ctx<G>` instead of a
  `&Palette`.
- `check-text` reads the arguments of `MenuItem::keyed` and `Label::key`
  as keys, like those of `text`.

## Consequences

- A menu is built once, by logic that doesn't know the language, and
  needs no rebuilding when the player changes it.
- A screen matches on what was chosen, not on an index.
- Breaking for the game: `MenuEvent::Chosen`, `MenuItem`'s fields, and
  the two skins' signatures change. The game's PR that moves its pin
  (game#1162) follows.
- `Menu::new(vec![])` with nothing else to say what `T` is needs
  `Menu::<()>::new`: a type default doesn't drive inference.
- Words are made on every paint (one `String` per item): cheap at a
  menu's size.
- Only labels are keys for now; a suffix and a note stay text.

## Alternatives considered

- **Index plus `value(i) -> &T`, `Chosen(usize)` unchanged** — keeps old
  matches compiling, but every screen still does the lookup the ticket is
  there to remove.
- **Translate in `Menu::view(ctx)`** — the view would be words again, and
  logic would need the `Ctx` to build it, which is what the game can't do
  in its pure input state.
- **A `Translate` trait the skins take instead of `Ctx`** — more general,
  but every skin in the engine already paints with a `Ctx`, and the trait
  would have one implementor.
