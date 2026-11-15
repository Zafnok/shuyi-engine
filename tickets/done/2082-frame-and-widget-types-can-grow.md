---
id: "2082"
title: "Before the first release, the frame's and the widgets' types that open tickets grow move under ADR-0039, and the ADR says every such type moves before a release"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-14
---

# 2082 — The frame's and the widgets' types that open tickets grow move under ADR-0039 before the first release

## Context

Nick, 2026-11-14 (`docs/decisions.md`): "I want to ensure we're not gonna
have a thousand version bumps after mvp. We'll follow semver. So I want
all major feature plans to be surfaced to me now if they will require a
ver bump." An audit of every open ticket that day found about 120 that
would break a game's code as planned. Most of them only grow a public
type that is still exhaustive: a new field on a struct games build by
literal, or a new variant on an enum games match. Under
[ADR-0039](../../docs/adr/0039-public-types-that-can-grow-non-exhaustive-config-and-views-with-constructors-plain-values-left-open.md)
the type moves under its rule "in the PR that next adds to it". After
2005 that PR is a breaking release, one per type.

Before 2005 a break costs nothing: no version has been published, and
the game moves its pin anyway. So every type an open ticket grows moves
now, in five tickets by area. Each one is mechanical: the attribute, the
constructor the ADR's kind asks for, and the call sites in the engine,
the templates and the examples.
- 2082: the frame and the widgets (this ticket);
- 2083: input and controllers;
- 2084: sound, music and cinematics;
- 2085: the assets' and the platform's data;
- 2086: `shuyi-cli`'s contract.

The types this ticket moves, checked exhaustive on 2026-11-14, with the
open tickets that grow each:

| Type | Where | Kind (ADR-0039) | Grown by |
| ---- | ----- | --------------- | -------- |
| `Sprite` | `crates/ui/src/frame/glyph_buffer.rs:219` | 3 (it has `new` and `painted`) | 2334, 2335, 2327, 2222, 2703, 1120 |
| `Paint` | `glyph_buffer.rs:187` | 6 | 2337, 1120 |
| `Overlay` | `glyph_buffer.rs:168` | 3 | 2337 |
| `Layer` (the frame's) | `glyph_buffer.rs:149` | 6 | 1120 |
| `PanelLook` | `crates/ui/src/frame/draw.rs:96` | 2 | 2338 |
| `MenuView` | `crates/ui/src/widgets/menu/view.rs:11` (it has `new`) | 3 | 2355, 2405 |
| `AskView` | `crates/ui/src/widgets/ask/view.rs:9` | 3 | 2409 |
| `GridView` | `crates/ui/src/widgets/name_entry.rs:338` | 3 | 2409 |
| `GridCell` | `name_entry.rs:172` | 6 | 2346 |
| `SkipView` | `crates/ui/src/dialogue/text_box.rs:237` | 3 | 2347 |
| `GlyphStyle` | `crates/ui/src/grid/glyph/style.rs:26` | 2 | 3203 |
| `GlyphSkin` | `crates/ui/src/grid/glyph.rs:37` (it has `const fn new`) | 2 | 2740 |
| `Projection` | `crates/ui/src/grid/layout.rs:29` | 6 | 1126, 1127, 2740 |
| `Grid` (the view's) | `layout.rs:49` | 3 | 2740, 2769, 1126, 2405 |
| `FitRule` | `crates/ui/src/input/fit.rs:80` | 6 | 2367 |
| `ScreenActions` | `fit.rs:41` | 3 | 2367 |

`Item` (`glyph_buffer.rs:334`) is already `#[non_exhaustive]`, so 2336's
new `Item::Shape` breaks nothing. Its planned `Breaking:` line is out of
date, and this ticket's PR corrects it.

## Nick input

None.

## Scope

**In:**
- Each type in the table under its ADR-0039 kind:
  - kind 2: `#[non_exhaustive]`, a `new` with what every caller sets
    (`const fn` where it can be), `with_<field>` where callers build it
    in a `const`;
  - kind 3: `#[non_exhaustive]`, a `new` for what it can't be without;
  - kind 6: `#[non_exhaustive]` on the enum, and on each struct-like
    variant that may gain fields.
- Every engine, template, example and tutorial-step call site moved
  onto the constructors.
- ADR-0039's **When** paragraph gains a sentence: before a crate's
  first release, every exhaustive type an open ticket plans to grow
  moves under the ADR at once (2082–2086), not in the PR that next adds
  to it.
- The listed tickets' text: a planned `Breaking:` line for one of these
  types becomes additive, and 2336's line is corrected.
- A `Breaking:` line in the completion notes (ADR-0038), naming each
  type. Before it, search the game for literals of each type
  (`gh search code --repo Zafnok/visions-of-shuyi '<Type> {'`) and list
  what the game's pin move has to change.

**Out (do not do):**
- Any new field or variant the listed tickets add.
- Plain values (kind 1: `Rect`, `Pos`, `Rgb`, `Cell`): they stay
  exhaustive.
- The widget painters' signatures (`menu::glyph::paint`,
  `ask::glyph::paint`, the name entry's painters). Each ticket that
  changes one adds a new function beside the old.
- Input, sound, assets and contract types (2083–2086).

**Estimated size:** about 900 lines (code 500, tests 250, docs 150).

## Implementation steps

1. For each type, add the attribute and its constructor in its module,
   with a doc line naming the kind.
2. `cargo build --workspace --all-targets` and fix each struct literal
   or exhaustive match it reports. Do the same in
   `templates/shuyi-default` (the templates' job, the `run-gates`
   skill) and the tutorial steps.
3. ADR-0039's **When** sentence.
4. Edit the listed tickets' planned `Breaking:` lines (grep
   `tickets/open` for each type's name).
5. The `document` skill: the book's pages that show a literal of these
   types.

## Acceptance criteria

- [x] Every type in the table is `#[non_exhaustive]`, and a test outside
      its crate builds it through its constructor (a doc test does).
- [x] `cargo xtask api` lists each change, and the completion notes'
      `Breaking:` line names each type (`api` runs in CI only: the PR's
      `api` job is the check).
- [x] The templates' job, the tutorial steps and the examples pass.
- [x] ADR-0039's **When** says what moves before a first release.
- [x] No open ticket still plans a `Breaking:` line for one of these
      types.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: one test per kind-2 type that its `new` and `with_<field>`
  build the value a literal built before.
- The existing snapshots stay unchanged: no frame or skin output moves.

## Completion notes

What was done:

- The sixteen types of the table are `#[non_exhaustive]`, each with a doc
  line naming its kind of ADR-0039.
  - Kind 3, new constructors: `AskView::new(text, answers)` (no line of
    keys), `GridView::new(title, spelled, rows, focus)`,
    `SkipView::new(question, answers)` and
    `ScreenActions::undeclared(screen)`. `Sprite::new`, `Overlay::new`,
    `MenuView::new`, `Grid::top_down`, `ScreenActions::of` and
    `ScreenActions::declared` were there already.
  - Kind 2: `PanelLook::new` with `with_border` and `with_title`;
    `GlyphStyle::PLAIN` with `with_range`, `with_flash`, `with_cursor`,
    `with_threat` and `with_side_cells`, all `const fn`;
    `GlyphSkin::new` with `with_style`. The toy's `ONE_BIT` is built
    with them, as a game builds its own.
  - Kind 6: `Layer`, `Paint`, `GridCell`, `Projection` and `FitRule`.
    None has a struct-like variant.
- Call sites: `shuyi-app`'s sprite drawing has a `_` arm on `Paint` (a
  newer paint is drawn in the image's own colours), and `shuyi-cli`'s
  fake game builds its `ScreenActions` with `undeclared`. The templates,
  the tutorial steps, the snippets and the examples built none of these
  by literal and matched none of these enums.
- Tests: `crates/ui/tests/it/growing_types.rs` builds each type from
  outside the crate through its constructors, in a `const` for the kind-2
  ones, and matches each enum with a `_` arm. `draw/tests.rs` and
  `glyph/style/tests.rs` check that `PanelLook`'s and `GlyphStyle`'s
  constructors build what a literal did; `GlyphSkin`'s was checked there
  already. No snapshot changed.
- ADR-0039's **When** says that before a crate's first release every
  exhaustive type an open ticket plans to grow moves at once (2082 to
  2086). It is a sentence added to an accepted ADR, as this ticket
  asks, not a new ADR.
- The planned `Breaking:` lines of 2334, 2335, 2337, 2338, 2346, 2355,
  2367, 2405, 2409, 2740 and 3203 for these types are now additive, and
  2336's (`Item`, already `#[non_exhaustive]`) is corrected to a
  `Behaviour:` line. 2222, 2327, 2703, 1120, 1126, 1127 and 2769 planned
  none.

Deviations:

- `PanelLook`'s `border` and `title` are renamed `with_border` and
  `with_title`, and `GlyphSkin::styled` is `with_style`: ADR-0039's
  kind 2 names them `with_<field>`, and before the release the rename
  costs no more than the attribute does.
- `GlyphStyle` has no `new`: none of its fields is one every caller
  must choose, so `GlyphStyle::PLAIN` (its `Default`) is the start and
  the `with_` methods change it.
- `ScreenActions::undeclared` is new here, not in 2367, because the
  attribute stops `shuyi-cli`'s literal from compiling. 2367 now says
  so.
- `AskView::new` takes the answers too, where 2409 planned
  `AskView::new(text)`; 2409 now names this one.
- The test outside the crate is one integration test file, not a doc
  test per type: one file builds all sixteen and is easier to keep.
- 2347 changes the type of `SkipView::answers` (and of the other views'
  hint fields) from `String` to styled text. That is a break no
  attribute prevents, so 2347's plan stays as it is.
- 2338's and 2409's painters (`ask::glyph::paint`, the name boxes'):
  both tickets now say the new signature is a new function beside the
  old one, as this ticket's **Out** asks.

The game, searched on 2026-11-14 at its `main` (`ce04d6b`, pin
`4ce11a3b`) with a shallow clone and `grep`, after `gh search code
--repo Zafnok/visions-of-shuyi '<Type> {'` for the first nine types ran
out of search quota. Its pin move changes:

- `Sprite`: 12 literals (`debug/sprite_test.rs` 10, `map_view/sprite.rs`,
  `map_view/sprite/units.rs`), all `Sprite { .., ..s }` updates: a copy
  with the fields assigned.
- `Overlay`: 1 (`map_view/sprite.rs`, an `Overlay { rect, ..o }` update).
- `PanelLook`: `.border(` and `.title(` in `screens/battle/glyph.rs`,
  `glyph/fight_box.rs` and `glyph/forecast.rs` (5 calls).
- `MenuView`: 11 literals (title, game over, mode select, save, the
  Playdate's tests, world map).
- `AskView`: 4 (profile, save, class change, preparations).
- `GridView`: 2 (`widgets/name_entry/playdate.rs`, `screens/profile/playdate.rs`).
- `SkipView`: 5 (`screens/dialogue.rs` and four tests).
- `GlyphSkin` and `GlyphStyle`: 1 each, the Playdate's `ONE_BIT`
  (`map_view/playdate.rs`).
- `Layer`, `Paint`, `GridCell`, `Projection`, `FitRule`, `Grid` and
  `ScreenActions`: no literal and no exhaustive `match` found.

The game's next pin move past this change makes the edits above (about
40 call sites); no game ticket is needed before it.

Book: none, no page of the book or snippet builds one of these types by
literal or calls a renamed method.

Behaviour: shuyi-app: a sprite whose `Paint` the backend doesn't know is
drawn in its image's own colours.

Breaking: shuyi-ui: Sprite: #[non_exhaustive]; build it with Sprite::new and assign the rest

```migration
since: 0.1.0
what: A Sprite is built with Sprite::new and its fields assigned, not by a literal.
old: Sprite { image, src, dest, .. }
new: Sprite::new(image, src, dest, layer)
before: |
  let flipped = Sprite { flip_x: true, ..sprite };
after: |
  let mut flipped = sprite;
  flipped.flip_x = true;
find: ["Sprite {"]
```

Breaking: shuyi-ui: Overlay: #[non_exhaustive]; build it with Overlay::new

```migration
since: 0.1.0
what: An Overlay is built with Overlay::new, not by a literal.
old: Overlay { rect, color, layer }
new: Overlay::new(rect, color, layer)
before: |
  let wide = Overlay { rect: double(o.rect), ..o };
after: |
  let wide = Overlay::new(double(o.rect), o.color, o.layer);
find: ["Overlay {"]
```

Breaking: shuyi-ui: Layer: #[non_exhaustive]; a match on it has a `_` arm

```migration
since: 0.1.0
what: A match on Layer has a wildcard arm.
old: Layer (exhaustive)
new: Layer (#[non_exhaustive])
before: |
  match layer { Layer::Under => 0, Layer::Over => 1 }
after: |
  match layer { Layer::Under => 0, _ => 1 }
```

Breaking: shuyi-ui: Paint: #[non_exhaustive]; a match on it has a `_` arm

```migration
since: 0.1.0
what: A match on Paint has a wildcard arm.
old: Paint (exhaustive)
new: Paint (#[non_exhaustive])
before: |
  match paint { Paint::Image => a, Paint::Solid(c) => b(c), Paint::Dimmed => d, Paint::Shaded(l) => e(l) }
after: |
  match paint { Paint::Solid(c) => b(c), Paint::Dimmed => d, Paint::Shaded(l) => e(l), _ => a }
```

Breaking: shuyi-ui: PanelLook: #[non_exhaustive]; build it with PanelLook::new, and `border` and `title` are `with_border` and `with_title`

```migration
since: 0.1.0
what: PanelLook's border and title methods are renamed with_border and with_title.
old: PanelLook::border, PanelLook::title
new: PanelLook::with_border, PanelLook::with_title
before: |
  let look = PanelLook::new(BoxStyle::Single, text, bg).border(edge).title(hi);
after: |
  let look = PanelLook::new(BoxStyle::Single, text, bg).with_border(edge).with_title(hi);
find: [".border(", ".title("]
```

Breaking: shuyi-ui: MenuView: #[non_exhaustive]; build it with MenuView::new and assign the rest

```migration
since: 0.1.0
what: A MenuView is built with MenuView::new, not by a literal.
old: MenuView { items, first_shown, shown }
new: MenuView::new(items)
before: |
  let menu = MenuView { items, first_shown: 0, shown: 2 };
after: |
  let menu = MenuView::new(items);
find: ["MenuView {"]
```

Breaking: shuyi-ui: AskView: #[non_exhaustive]; build it with AskView::new and assign `keys`

```migration
since: 0.1.0
what: An AskView is built with AskView::new, not by a literal.
old: AskView { text, answers, keys }
new: AskView::new(text, answers)
before: |
  let ask = AskView { text: "Leave?".to_owned(), answers: Vec::new(), keys: Some(keys) };
after: |
  let mut ask = AskView::new("Leave?", Vec::new());
  ask.keys = Some(keys);
find: ["AskView {"]
```

Breaking: shuyi-ui: GridView: #[non_exhaustive]; build it with GridView::new

```migration
since: 0.1.0
what: A GridView is built with GridView::new, not by a literal.
old: GridView { title, spelled, rows, focus }
new: GridView::new(title, spelled, rows, focus)
before: |
  let grid = GridView { title: "Name".to_owned(), spelled: String::new(), rows, focus: (0, 0) };
after: |
  let grid = GridView::new("Name", "", rows, (0, 0));
find: ["GridView {"]
```

Breaking: shuyi-ui: GridCell: #[non_exhaustive]; a match on it has a `_` arm

```migration
since: 0.1.0
what: A match on GridCell has a wildcard arm.
old: GridCell (exhaustive)
new: GridCell (#[non_exhaustive])
before: |
  match cell { GridCell::Char(c) => a(c), GridCell::Space => b, GridCell::Delete => d, GridCell::Done => e }
after: |
  match cell { GridCell::Char(c) => a(c), GridCell::Space => b, GridCell::Delete => d, _ => e }
```

Breaking: shuyi-ui: SkipView: #[non_exhaustive]; build it with SkipView::new

```migration
since: 0.1.0
what: A SkipView is built with SkipView::new, not by a literal.
old: SkipView { question, answers }
new: SkipView::new(question, answers)
before: |
  let skip = SkipView { question: "Skip scene?".to_owned(), answers: keys };
after: |
  let skip = SkipView::new("Skip scene?", keys);
find: ["SkipView {"]
```

Breaking: shuyi-ui: GlyphStyle: #[non_exhaustive]; start from GlyphStyle::PLAIN and change it with its `with_` methods

```migration
since: 0.1.0
what: A GlyphStyle is GlyphStyle::PLAIN changed by its const with_ methods, not a literal.
old: GlyphStyle { range, flash, cursor, threat, side_cells }
new: GlyphStyle::PLAIN.with_range(..).with_flash(..).with_cursor(..).with_threat(..).with_side_cells(..)
before: |
  const STYLE: GlyphStyle = GlyphStyle { range, flash: FlashLook::Invert, cursor: CursorLook::Flat, threat, side_cells: 1 };
after: |
  const STYLE: GlyphStyle = GlyphStyle::PLAIN
      .with_range(range)
      .with_flash(FlashLook::Invert)
      .with_cursor(CursorLook::Flat)
      .with_threat(threat)
      .with_side_cells(1);
find: ["GlyphStyle {"]
```

Breaking: shuyi-ui: GlyphSkin: #[non_exhaustive]; build it with GlyphSkin::new, and `styled` is `with_style`

```migration
since: 0.1.0
what: A GlyphSkin is built with GlyphSkin::new and with_style, which replaces styled.
old: GlyphSkin { colors, walk, foot_px, style }, GlyphSkin::styled
new: GlyphSkin::new(colors, walk, foot_px).with_style(style)
before: |
  const ONE_BIT: GlyphSkin = GlyphSkin { colors, walk, foot_px: 2, style: STYLE };
  const FLAT: GlyphSkin = GLYPH.styled(STYLE);
after: |
  const ONE_BIT: GlyphSkin = GlyphSkin::new(colors, walk, 2).with_style(STYLE);
  const FLAT: GlyphSkin = GLYPH.with_style(STYLE);
find: ["GlyphSkin {", ".styled("]
```

Breaking: shuyi-ui: Projection: #[non_exhaustive]; a match on it has a `_` arm

```migration
since: 0.1.0
what: A match on Projection has a wildcard arm.
old: Projection (exhaustive)
new: Projection (#[non_exhaustive])
before: |
  match grid.projection { Projection::TopDown => a }
after: |
  match grid.projection { Projection::TopDown => a, _ => b }
```

Breaking: shuyi-ui: Grid: #[non_exhaustive]; build it with Grid::top_down

```migration
since: 0.1.0
what: A Grid is built with Grid::top_down, not by a literal.
old: Grid { origin, corner, tile, tiles, scale, projection }
new: Grid::top_down(origin, corner, tile, tiles, scale)
before: |
  let grid = Grid { origin, corner, tile, tiles, scale: 1, projection: Projection::TopDown };
after: |
  let grid = Grid::top_down(origin, corner, tile, tiles, 1);
find: ["Grid {"]
```

Breaking: shuyi-ui: FitRule: #[non_exhaustive]; a match on it has a `_` arm

```migration
since: 0.1.0
what: A match on FitRule has a wildcard arm.
old: FitRule (exhaustive)
new: FitRule (#[non_exhaustive])
before: |
  match rule { FitRule::Undeclared => warn, FitRule::NoLayout | FitRule::NotOnTarget | FitRule::MenuSlot | FitRule::Unreachable | FitRule::Shared => fail }
after: |
  match rule { FitRule::Undeclared => warn, _ => fail }
```

Breaking: shuyi-ui: ScreenActions: #[non_exhaustive]; build it with of, declared or undeclared

```migration
since: 0.1.0
what: ScreenActions is built with of, declared or the new undeclared, not by a literal.
old: "ScreenActions { screen, actions: None }"
new: ScreenActions::undeclared(screen)
before: |
  let title = ScreenActions::<Action> { screen: "title", actions: None };
after: |
  let title = ScreenActions::<Action>::undeclared("title");
find: ["actions: None }"]
```
