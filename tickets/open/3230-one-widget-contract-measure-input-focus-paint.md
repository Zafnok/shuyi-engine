---
id: "3230"
title: "Every widget implements one contract for measure, input, focus, paint and hits, and a game's own widget joins it"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3230 — One widget contract

## Context

Found by the engine blind-spot audit of 2026-11-23 (the in-game UI
area: "no shared widget contract", high).

**What Shuyi has today.**
- Four widgets in `crates/ui/src/widgets/`: `Menu` (`menu.rs`), `Ask`
  (`ask.rs`), `NameBox` and `NameEntry` (`name_entry.rs`), with
  `ListCursor` (`list.rs`) under the menu. No trait joins them:
  `grep 'pub trait' crates/ui/src` finds `Screen`, `GameData`,
  `Module`, painter traits and others, and no widget trait.
- Their methods all differ. `Menu::handle(action) -> Option<MenuEvent<T>>`
  and `handle_with_sound(action, audio)`; `Ask::handle(action, audio) ->
  Option<T>`; `NameEntry::handle(action, ctx) -> NameEvent`. Their
  painters differ too: `menu::glyph::size(ctx, view)` and
  `paint(ctx, view, buf, x, y)`; `ask::glyph::rect(view, buf)` and
  `paint(palette, view, buf)`, which centres the box on the whole
  buffer; `name_entry::glyph::paint_typing(palette, buf, rect, view)`
  and `paint_grid(palette, buf, rect, view)`.
- Hits are recorded by painters into the frame (2405, done;
  [ADR-0134](../../docs/adr/0134-pointing-in-turn-games-and-screens.md)),
  under a `WidgetKey` (`crates/ui/src/frame/hit.rs`). Only `Menu` has a
  key of its own (`Menu::keyed`); 2409 adds `Ask::keyed` and
  `NameEntry::keyed`.

**What is planned, one side protocol at a time.**
- 2329 adds a `Measure` trait for layout.
- 2355 adds `FocusNode` and the directions a widget **keeps**.
- 2371 adds `WidgetLook`, passed to each widget's painter.
- 2322 (parked) adds describe code to each widget for the
  accessibility tree.
- 2405 (done) added `WidgetKey` and `Hit`; 2409 adds the pointer to
  the other widgets.

Each of these patches the same four widgets by hand. The UI tickets
hold 36 "either order with" clauses that do this coupling (2329: 3,
2330: 3, 2334: 8, 2355: 5, 2371: 3, 2409: 4, 2322: 5, 2346: 5;
counted with `grep -o -i 'either order'`). Each new widget (sliders,
tabs, the inventory grid of 2759, the message log of 2766, the HUD of
2762) multiplies the work.

A game's own widget is worse off. 2330's `.ui.ron` has a closed list
(`Text`, `Menu`, `Meter`, `TitledBox`, `Question`, `TextBox`) and
`Slot(id)` "for a part the game's code draws": a closure that layout
can't measure, focus can't reach and `shuyi check`'s reachability check
can't see.

Godot's `Control`, Unity's `VisualElement`, Bevy's UI nodes and egui's
`Widget` trait are each one contract that every widget implements once.

**Example from a game that isn't tactics.** A farming game's seed ring
(a circle of seed bags, turned with Left and Right) sits beside the
shop's menu. It should be measured by the layout, take the focus with
the d-pad, take a click, and be named in the shop's `.ui.ron`, the
same as the engine's menu.

**This ticket** writes that contract before 2329, so 2329, 2355, 2371,
2409 and 2322 implement one trait instead of five, and a game's widget
is a first-class one.

## Nick input

None.

## Scope

**In:**
1. **`shuyi_ui::widgets::Widget<G: GameData>`**
   (`crates/ui/src/widgets/contract.rs`, re-exported from
   `widgets.rs`), `no_std` with `alloc`, integers only, and
   **object-safe** (a test builds a `Vec<Box<dyn Widget<G>>>`):
   - `fn kind(&self) -> &'static str`: a stable `snake_case` name
     (`"menu"`, `"question"`), for errors, tests and the editor, as
     `Screen::name`.
   - `fn key(&self) -> WidgetKey`: the key its hits carry. Defaulted
     to `WidgetKey::default()`; `Menu` returns the one `keyed` sets.
   - `fn measure(&self, ctx: &Ctx<G>, room: Size) -> Size`: the
     smallest size it needs, in the frame's units (cells for the glyph
     skin), never more than `room` unless it can't fit at all. `Size {
     w: i32, h: i32 }` is new here (kind 1 of ADR-0039, a plain value);
     2329's `layout` takes it.
   - `fn handle(&mut self, ctx: &mut Ctx<G>, input: &WidgetInput<'_,
     G::Action>) -> Handled`. `WidgetInput` is `#[non_exhaustive]`
     with a constructor (kind 3 of ADR-0039): this frame's `action`
     (an `Option`) and `pointer` (`Option<&PointerInput>`). `Handled`
     is `#[non_exhaustive]`: `Used` (it did something, sounds played
     through `ctx.audio` as `handle_with_sound` does), `Unused` (the
     input is the screen's: a direction it gives away goes to focus),
     and `Event(WidgetEvent)`. `WidgetEvent` is `#[non_exhaustive]`:
     `Chosen(u32)` (the item's index), `Cancelled`, `Changed`,
     `Done`. A widget's typed result (a menu item's `T`) stays on its
     own methods; a screen holding `dyn Widget` maps the index.
   - `fn keeps(&self) -> &'static [Dir]`: the directions it uses
     itself and never gives away (`shuyi_core::Dir`). Defaulted to
     none. A wrapping `Menu` keeps Up and Down; `Ask` keeps all four.
     This is what 2355's `FocusNode::keeps` reads.
   - `fn paint(&self, paint: &PaintCtx<'_, G>, buf: &mut GlyphBuffer)`:
     paints its view inside `paint.rect` and records its hits under
     `key()` there (ADR-0134). `PaintCtx` is `#[non_exhaustive]` with
     a constructor: the `ctx` and the `rect` (`frame::Rect`). It is
     built to grow: 2371's look and 2355's "has the focus" are fields
     added to it, never new parameters.
   - The rule, in the module docs and the ADR: **a widget's logic
     builds a plain-data view and its painter paints it** (`AGENTS.md`
     rule 9). `paint` is `view()` then the skin's painter; it changes
     nothing. Tests of what happened read the view; only tests of a
     look read cells.
2. **The engine's widgets implement it**, each a thin layer over the
   methods they have, so every existing test and snapshot stays as it
   is:
   - `Menu<T: Clone>`: `measure` is `menu::glyph::size`; `handle`
     calls `handle_with_sound` and `point_with_sound`, and gives
     `Unused` for an action that leaves the focus where it was and
     chooses nothing (Left and Right today); `keeps` is Up and Down;
     `paint` is `menu::glyph::paint` at the rect's corner.
   - `Ask<T: Copy + PartialEq>`: `measure` is the width and height
     `ask::glyph::rect` computes; a new `ask::glyph::paint_in(palette,
     view, buf, rect)` paints in a given rect, and `paint` stays and
     calls it with `rect`, so its snapshots don't change; `Chosen`
     carries the answer's index.
   - `NameBox` and `NameEntry`: over `paint_typing` and `paint_grid`.
     Where a view needs an argument the widget doesn't hold (their
     `title`), the impl is on a small wrapper (`NameEntry::titled(self,
     title: Label)`), named in the docs.
3. **A game's own widget.** `WidgetKinds<G>` (`widgets/contract.rs`):
   `register(kind: &'static str, build: fn(&ron::Value) ->
   Result<Box<dyn Widget<G>>, String>)`, `build(kind, args)`, and
   `kinds()` in name order. A kind named twice is an error. A game
   hands its kinds in through a new defaulted
   `GameData::widget_kinds(&self) -> Option<&WidgetKinds<Self>>`
   (`None`), as 2338's `GameData::nine_slice` names frames. `.ui.ron`
   reads it in 2330.
4. **An ADR** (`write-adr`): one contract for every widget; the
   methods and why each is there; what stays on a widget's own typed
   methods; that a cross-cutting feature is a defaulted method or a
   field of `WidgetInput` or `PaintCtx`, never a new trait; painters
   record hits (ADR-0134); views stay plain data. It names 2329, 2355,
   2371, 2409, 2322 and 2330 as the tickets that build on it.
5. **Either order with 2371:** if 2371 is done, `PaintCtx` carries its
   `WidgetLook` (`look`, default `WidgetLook::GLYPH`), and the menu,
   question and name entry impls pass it to their painters, with a
   one-bit snapshot of one widget painted through the contract; if
   not, nothing to do (2371 adds `look` to `PaintCtx`, its clause says
   so).
6. **Either order with 2409:** if 2409 is done, `Ask`'s and
   `NameEntry`'s `handle` pass `input.pointer` to `Ask::point` and
   `NameEntry::point`, and their `key` returns the key `keyed` sets,
   with a test each; if not, they read the action only, and 2409 routes
   their pointer through the contract (its clause says so).
7. **Either order with 2322 (parked):** if 2322 is done, add `fn
   describe(&self, ctx: &Ctx<G>) -> Option<AccessNode>` to `Widget`,
   defaulted to `None`, and move 2322's per-widget describe code for
   the menu, question and name entry into their impls; if not, nothing
   to do (2322 adds `describe` as a defaulted method of `Widget`, its
   clause says so).
8. **Either order with 3231:** if 3231 is done, the widget contract
   measures and paints in either 3231's window layer (`WindowLayer`,
   window pixels) or the frame: `measure`'s `Size` is in the units of
   the target it paints into, and a defaulted `fn paint_window(&self,
   paint: &PaintCtx<'_, G>, layer: &mut WindowLayer)` (nothing by
   default) paints there and records its hits in the layer's `Hits`,
   with `Menu`'s impl and a test of a menu painted in the layer; if
   not, nothing to do (3231 leaves widgets in the layer to this ticket,
   and its clause says so).
9. **Either order with 3235:** if 3235 is done, move its bindings
   (`UiValues`, `shown`, `enabled`) and its `WidgetLook` from `UiLook`
   onto the widget contract, so every widget kind reads its bound values
   and its look there and `shown` and `enabled` are handled once; if
   not, nothing to do (3235 builds on the contract).
10. **From 2802 (done):** the book's chapter *The frame and skins*
   (`site/content/learn/book/frame-and-skins.md`) gets a section,
   *Writing a widget*, its code in `site/snippets/`: the seed ring of
   the Context, implemented, registered and painted beside a menu.

**For the game:** additive. A new trait, new types, impls on the
engine's widgets, a new `ask::glyph::paint_in` and a defaulted
`GameData` method; no signature changes. The game pins engine
`50472bbc`. `gh search code --repo Zafnok/visions-of-shuyi 'trait
Widget'` finds nothing, and `'widget_kinds'` finds nothing. Its
`crates/ui/src/widgets/mod.rs` re-exports engine widgets by name
(`pub use shuyi_ui::widgets::{AnswerView, Ask, …}`), and its
`widgets/menu.rs` globs `shuyi_ui::widgets::menu::*`; neither picks up
anything new from `widgets::contract`. Its `impl GameData for Shuyi`
(`crates/ui/src/screen.rs`) gets the default.

**Out (do not do):**
- The layout solver: 2329, which waits for this ticket and lays out
  with `Widget::measure` and `Size`.
- `FocusMap` and `Focus`: 2355, which comes after 2329 and builds each
  `FocusNode` from `Widget::keeps` and `Handled::Unused`.
- The `.ui.ron` node that names a registered kind, and `DocScreen`
  holding its widgets as `dyn Widget`: 2330, which comes after 2329.
- `TextLabel`'s measure in pixels: 2334, through 2329's clause with it.
- New widgets (toggle, slider, choice, scroll panel, tab bar, tooltip,
  image node): 3234. Text editing: 3232, and drag and drop: 3233; both
  wait for this ticket and implement this contract (3232 puts
  `TextField` on it, 3233 adds its drag source and drop target marks as
  defaulted methods). Bindings and a theme file: 3235, through part 9.
- Moving engine screens onto `dyn Widget`: the screens keep their
  widgets as they are; 2329 and 2330 move the first ones.
- A second skin's painter (2334's text, 2371's one-bit) beyond the
  either-order parts above.

**Estimated size:** about 1,200 lines (code 550, tests 450, docs 200).

**Split:** past 1,000 lines, split off part 3 (`WidgetKinds` and
`GameData::widget_kinds`) into a new ticket blocked by this one, and
add its id to 2330's `blocked_by`.

## Implementation steps

1. Read `widgets.rs`, `widgets/menu.rs`, `menu/view.rs`,
   `menu/glyph.rs`, `ask.rs`, `ask/glyph.rs`, `name_entry.rs`,
   `name_entry/glyph.rs`, `frame/hit.rs`, `frame/glyph_buffer.rs`
   (`Rect`, `GlyphBuffer::hit`), `input/pointer.rs`, `ctx.rs`
   (`GameData`, `Ctx::widths`, `Ctx::audio`) and `screen.rs`. Read
   ADR-0134 and ADR-0039. Check whether 2371, 2409 and 2322 are done.
2. Write `Size`, `WidgetInput`, `Handled`, `WidgetEvent`, `PaintCtx`
   and `Widget` in `widgets/contract.rs`, with the module docs and a
   doc test of a small widget.
3. Implement `Widget` for `Menu`, `Ask` (with `ask::glyph::paint_in`),
   `NameBox` and `NameEntry`.
4. Write `WidgetKinds` and `GameData::widget_kinds`.
5. Do the either-order parts whose tickets are done (2371, 2409,
   2322, 3231, 3235).
6. Write the ADR and the book's section, check the size against the
   split, and run the gates.

## Acceptance criteria

- [ ] A `Vec<Box<dyn Widget<G>>>` holding a `Menu`, an `Ask` and a
      test widget compiles and is painted and handled through the trait
      (integration test).
- [ ] For each engine widget, `measure` gives the size its glyph
      painter uses today (`menu::glyph::size`, the `w` and `h` of
      `ask::glyph::rect`), and `paint` into a rect at that size gives
      the same cells as the existing snapshot (unit tests).
- [ ] Every existing widget and screen snapshot is unchanged.
- [ ] On a wrapping `Menu`, Up and Down give `Used`, Left and Right
      give `Unused`, Confirm on an item gives `Event(Chosen(i))` and
      Cancel gives `Event(Cancelled)` when the menu cancels (unit
      tests). `keeps()` is Up and Down.
- [ ] For a random sequence of actions, a `Menu` driven through
      `Widget::handle` ends with the same focus, the same events and
      the same sounds queued as one driven through `handle_with_sound`
      (property test).
- [ ] A click on a menu row through `Widget::handle` with a
      `PointerInput` chooses the row, as `point_with_sound` does (unit
      test).
- [ ] Each widget records its hits inside `PaintCtx::rect` under
      `key()` (unit test reading `GlyphBuffer::hit_at`).
- [ ] `WidgetKinds` builds a registered test kind from RON args,
      refuses an unknown kind and args its builder refuses with the
      builder's message, and refuses a kind registered twice (unit
      tests). A game with no kinds gets `None` from
      `GameData::widget_kinds`.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] The ADR is written and linked from `docs/adr/README.md`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each method on each engine widget; `Handled` for each kind of
  action; hits under the key; `WidgetKinds` errors; `paint_in` against
  `paint`.
- Property: `Widget::handle` agrees with `handle_with_sound` on random
  action sequences; `measure` never exceeds `room` for a menu that
  fits.
- Snapshot / integration: a test screen in `crates/ui/tests/it/`
  painting a menu, a question and a registered test widget side by
  side through `dyn Widget`, with its cells snapshot and its hits
  checked.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
