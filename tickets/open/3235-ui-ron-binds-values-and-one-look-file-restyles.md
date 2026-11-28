---
id: "3235"
title: ".ui.ron nodes bind values, visibility and repeated rows, and one look file restyles every engine screen"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2330", "2371", "3230"]
nick_input: none
completed:
---

# 3235 — .ui.ron nodes bind values, visibility and repeated rows, and one look file restyles every engine screen

## Context

Found by the blind-spot audit of 2026-11-23 (UI area, its proposals 43
and 44). Two gaps stop a `.ui.ron` screen (2330) at static menus, and
make each game with its own art redraw the engine's screens in code.

1. **A `.ui.ron` binds only text arguments.** 2330's `UiDoc` takes a
   value from the game only through `DocScreen::set_arg(id, name,
   value)`, which fills a text key's `{n}`. Everything else falls back to
   a `Slot(id)` closure. 2330 lists a `Meter` widget, but no field says
   where its value comes from. Nothing says whether a node is shown or
   enabled. Nothing repeats a row for each item of a list: a shop's
   wares, a party, the save slots. 2330's Out rules out "logic in the
   file (no conditions or scripts)", and that rule stays: a binding here
   is a lookup by name, never an expression. So every screen that shows
   live data goes back to Rust, and hot reload (2220, done) can't show
   those parts.
2. **No one file restyles the UI.** A look comes from scattered places:
   `PanelLook` (`crates/ui/src/frame/draw.rs`, line 99: a `BoxStyle` and
   four colours), 2371's `WidgetLook` (open: `GLYPH` and `ONE_BIT`, built
   in code), 2338's nine-slice frames (open), and the palette's roles
   (`EngineColor`, `frame/color.rs` line 131). To change more, a game
   replaces a screen's whole painter: `painted_by` on Options
   (`screens/options.rs` line 658), Key bindings (`key_bindings.rs` line
   346) and Profiles (`profiles.rs` line 264), each a
   `fn(&Ctx<G>, &View, &mut GlyphBuffer)`. The game shows the cost: 617
   production lines of one-bit painters for this engine's screens, and a
   copied menu skin because `widgets/menu/glyph.rs` hard-codes
   `BoxStyle::Single` and its roles (2371, Context item 3; game ADR-0091
   says the game's Options and Key bindings use `painted_by`, found with
   `gh search code --repo Zafnok/visions-of-shuyi 'painted_by'`, game
   pinned at engine `50472bbc`). 2371 fixes the one-bit case in code.
   The next game with its own look pays the same again.

**Why "look", not "theme".** In this engine a theme is already a
palette: `GameSettings::Theme` and `GameData::theme_def`
(`crates/ui/src/ctx.rs` line 110) let the player pick colours, and 2324
(parked) checks them for colour vision. The new file is a **look file**
(`UiLook`, `*.look.ron`). It names colour roles, never colours, so a
palette theme still recolours a look.

Other engines do both as data: Godot's Theme resource and exported
properties, Unity UI Toolkit's style sheets and runtime data binding,
Defold's and GameMaker's GUI skins.

**Examples from games that aren't tactics:** a farming game's shop
lists its wares as repeated rows with a price and a greyed-out row for
what the player can't afford; a platformer's HUD shows a health meter
and a key icon only once the key is held; a puzzle game restyles every
engine screen with a rounded frame and a different focus bar, with one
file and no painter.

Builds on: 2330 (the `.ui.ron` and `DocScreen`), 2371 (`WidgetLook`),
3230 (the `Widget` contract, through which bindings and the look reach
every widget),
2329 and 2355 (layout and focus, through 2330), 2220 (done, `Hot<T>`),
3218 (done, typed `Arg` values), 3001 and 3053 (done, the target
profile's default painter set, which 2371 reads).

## Nick input

None.

## Scope

**In — part A, bindings (`shuyi-ui`, the `UiDoc` format):**
- **`UiValues`**, a plain-data view the game fills: named values of a
  few kinds, and lists of rows that are themselves `UiValues`. Built
  with `UiValues::new()` and `with_number(name, i64)`,
  `with_flag(name, bool)`, `with_arg(name, impl ToArg)` (kept as an
  `Arg`, as `Label::with_arg` does), `with_list(name, Vec<UiValues>)`.
  `#[non_exhaustive]` with that constructor (ADR-0039): kinds may come.
  No floats.
- **A doc declares its bindings.** A `UiDoc` gets an optional
  `binds: { "hp": Number, "has_key": Flag, "party": List({ "name": Arg,
  "hp": Number }) }`. Every binding a node names must be declared, with
  the kind its field needs. So `shuyi check` checks a file with no game
  code, and a running `DocScreen` checks the game's `UiValues` against
  the declaration.
- **Fields that bind** (each optional, each a name in `binds`, never an
  expression):
  - `Meter(value: "hp", max: "hp_max")`;
  - `Text(key: "gold.count", args: { "n": "gold" })`: the arg comes from
    the binding; `set_arg` still works, and a node that has both for one
    arg is an error;
  - `shown: "has_key"` on any node: when false, the node takes no space
    and isn't in the `FocusMap`;
  - `enabled: "can_buy"` on a widget: when false, it paints in the
    look's disabled role and isn't in the `FocusMap`;
  - **`Repeat(over: "party", max: 6, template: <node>)`**: the template
    once for each row of the list, laid out as the `Repeat`'s own
    `Style` says (2329's direction and gap). Names inside the template
    are looked up in the row first, then in the doc's values. An `id`
    inside the template becomes `id[i]` (`slot[2]`), and a `focus`
    neighbour inside it names a node of the same row. Rows past `max`
    are not shown (scrolling is a scroll panel's job, Out);
  - **a `Menu` whose items come from a list:**
    `Menu(id: "saves", items_over: "slots", label: "slot.label", value:
    "id")`: one item per row, its label key's args from the row, its
    value (ADR-0020) from the row's `value` arg. `items` and
    `items_over` together are an error.
- **`DocScreen::with_values(fn(&Ctx<G>) -> UiValues)`**: read on each
  `update` and kept, so painting reads the kept values. A value whose
  name or kind doesn't match the declaration is an error naming the
  file, the node path and the binding; in a release build the node
  paints as if its value were 0, false or empty, and the error is logged
  once.
- **Focus across a change of rows:** when a list grows or shrinks, the
  focus stays on the item or row with the same value or the same index,
  clamped to the last one (`Focus::rebuild`, as 2330's reload does).
- **`shuyi check`** reports, for every `.ui.ron`: a binding a node uses
  that `binds` doesn't declare, a kind that doesn't fit the field, and a
  template `id` repeated inside one row. Its reachability check (2330)
  lays out each `Repeat` with `max` rows and with none.
- Format change: additive. A `.ui.ron` with no `binds` parses as before.

**In — part B, the look file (`shuyi-ui`):**
- **`UiLook`**, an asset kind, format `*.look.ron`: a `default` look and
  named looks. Each look starts from a `base` (`Glyph` or `OneBit`,
  2371's `WidgetLook::GLYPH` and `WidgetLook::ONE_BIT`) and overrides
  any of: the `BoxStyle`, the frame's border, title, text and background
  roles (`EngineColor` names), the focus bar's role, the disabled role,
  the note rule, the scroll marks (two characters, `▲`/`▼` by default),
  and padding and gap in whole cells. An unknown role or field is an
  error naming the file and the path. `#[non_exhaustive]` with a
  constructor; new fields are additive.
- **`UiLook::widget_look(name) -> WidgetLook`**: a look file becomes
  2371's `WidgetLook` (and so its `PanelLook`). There is still one look
  type in code; the file is its data form.
- **Every engine screen and widget reads it.** `GameData` gets
  `fn ui_look(&self) -> Option<&UiLook>`, default `None` (additive). The
  glyph and one-bit painters of Options, Key bindings, Profiles and the
  layout picker, and the menu, question and name entry widgets, take
  their `WidgetLook` from it. The order: the game's look file, then the
  target profile's default painter set (2371), then `GLYPH`. A game that
  passes its own painter with `painted_by` keeps it.
- **A `.ui.ron` node takes `look: "shop"`**, a named look for that
  widget and the nodes under it. An unknown name fails `shuyi check`.
- **Reload:** the engine screens and `DocScreen` hold a `Hot<UiLook>`
  (2220): a changed look file repaints them without a restart.
- **Docs:** the UI chapter of the book (2802) gets a section on
  bindings and one on the look file, and the `.ui.ron` reference lists
  the new fields.

**Either order with 2338:** if 2338 is done, a look in the file also
takes `frame: "name"`, a nine-slice resolved by `GameData::nine_slice`,
passed to `WidgetLook::framed`; if not, nothing to do (2338 adds the
field to `UiLook` when it lands).

**Either order with 2334:** if 2334 is done, a look also takes `font:
"id"` (an id in `GameData::fonts`), used by a `Text` node under it that
names no `font` of its own; if not, nothing to do (2334 adds the field
to `UiLook` when it lands).

**Through 3230's contract:** bindings and the look reach a widget
through 3230's `Widget`. The look is the `look` field of `PaintCtx`
(a `WidgetLook` from `UiLook::widget_look`), never a new painter
parameter. `shown` and `enabled` are handled once in `DocScreen`, for
every widget kind.

**Either order with 3234:** if 3234 is done, `Toggle`, `Choice` and
`Slider` nodes bind their value with this ticket's bindings (`Flag` for
a toggle, `Number` for a slider and a choice), its image node takes
`shown`, and the look file styles every widget 3234 added; if not,
nothing to do (3234 binds them and covers them in the look file).

**What changes for the game:** nothing breaks. `GameData::ui_look` has a
default, and the `.ui.ron` and the painters' looks are additive. The
game has no `.ui.ron` yet (2330 is open), and its one-bit painters stay
until it chooses to move them onto a look file (game#1184 follows 2371).
No `Behaviour:` line is expected: every existing snapshot is unchanged.

**Out (do not do):**
- Expressions, conditions or arithmetic in a `.ui.ron` (2330's rule
  stays; a game computes `can_buy` itself and binds a flag).
- Writing back from the UI to the game's state by binding: a widget
  still acts through actions and `ScreenResult` (ADR-0018).
- Scrolling a `Repeat` past `max` (3234's scroll panel).
- Eased transitions between a widget's states (normal, focused,
  disabled): 3270. Menu sounds in the look file.
- Colours in the look file: roles only; colours stay in the palette and
  its themes (`GameSettings::Theme`, 2324).
- A text size the player picks (2323, parked) and the accessibility tree
  (2322, parked).
- An editor for the look file (2507, parked).
- BasicGame loading a look file from its folder by itself (2269 reloads
  BasicGame's other files).
- Moving engine screens into 3231's window layer: a follow-up when a
  game asks (3231's Out says so too).

**Estimated size:** about 1,800 lines (code 850, tests 700, docs 250).

**Split:** past 900 lines, split off part B (steps 5–8, the look file)
into a new ticket blocked by this one, with the 2338 and 2334 clauses
moved to it.

## Implementation steps

1. **Read** 2330's `UiDoc`, `DocScreen` and its `shuyi check` pass,
   2371's `WidgetLook` and the screens' painters, `frame/draw.rs`
   (`PanelLook`), `frame/color.rs` (`EngineColor`), `crates/assets/src/hot.rs`
   (`Hot<T>`) and `crates/assets/src/golden.rs`.
2. **`UiValues`** and the `binds` declaration in the `UiDoc` format,
   with its errors (file, node path, binding).
3. **The binding fields**: `Meter`'s `value` and `max`, `Text`'s `args`,
   `shown`, `enabled`, `Repeat` with its `id[i]` ids and row-first
   lookup, and `Menu`'s `items_over`. `DocScreen::with_values`, the
   checks against the declaration, and the focus across a change of rows.
4. **`shuyi check`**: undeclared bindings, wrong kinds, repeated
   template ids, and the reachability check over `Repeat` at `max` and
   at none.
5. **`UiLook`** and its `*.look.ron` format (an ADR-0007 `Format`), with
   `widget_look`.
6. **`GameData::ui_look`** and the engine screens' and widgets' painters
   reading it, in the order above.
7. **`look: "name"`** on `.ui.ron` nodes, and `shuyi check` for an
   unknown name.
8. **Reload** of the look file through `Hot<UiLook>`.
9. **Do the either-order parts** whose tickets are done (2338's
   `frame`, 2334's `font`, 3234's nodes).
10. **Docs**, then the gates.

## Acceptance criteria

- [ ] A `.ui.ron` with a `Meter(value: "hp", max: "hp_max")` and
      `UiValues` of 3 and 10 paints the same meter as `meter(rect, 3,
      10, ..)` called in code (a test comparing cells).
- [ ] A node with `shown: "has_key"` takes no space and gets no focus
      when the flag is false, and appears when it turns true on the next
      `update` (a harness test).
- [ ] A widget with `enabled` false paints in the look's disabled role
      and the D-pad passes over it (a harness test).
- [ ] A `Repeat(max: 5)` over a list of 7 rows shows 5 rows; over a list
      of 0 shows none and the screen still has a reachable widget (a
      test each).
- [ ] With the focus on row 4 of 5, shrinking the list to 3 rows puts
      the focus on row 3 (the last), and growing it back keeps it there
      (a test).
- [ ] A `Menu(items_over: ..)` over 3 save slots returns the chosen
      row's value in its `ScreenResult` (a test).
- [ ] `shuyi check` fails a `.ui.ron` that uses an undeclared binding, a
      `Flag` where a `Number` is needed, both `items` and `items_over`,
      and a `Text` arg both bound and set by `set_arg`, each naming the
      file and the node path.
- [ ] A `.ui.ron` with no `binds` parses and paints as before: 2330's
      moved screen keeps its snapshot.
- [ ] With no look file, every existing glyph and one-bit snapshot is
      unchanged.
- [ ] A look file whose `default` is `base: Glyph` with `box_style:
      Double` and a different focus-bar role changes the Options, Key
      bindings, Profiles and layout picker screens and the three widgets
      (snapshots), with no `painted_by`.
- [ ] A screen given `painted_by` keeps its painter under a look file (a
      test).
- [ ] A `.ui.ron` node with `look: "shop"` paints its menu in that look
      while its siblings keep the default (a snapshot); an unknown look
      name fails `shuyi check`.
- [ ] A look file with an unknown role fails to load, naming the file
      and the path.
- [ ] Changing the look file in a running harness test (2220's script)
      repaints the screen in the new look.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `UiValues` lookups (row first, then the doc); each binding
  field's parse; each error; `UiLook` parse, `base` and overrides, and
  `widget_look`; the look order (file, profile, `GLYPH`).
- Fuzz-style: random bytes never panic the `UiLook` format, nor the
  `UiDoc` format with `binds` (ADR-0007).
- Golden: a `.look.ron` fixture's `state_hash` as a row
  `ui-look: ron` in `crates/ui/tests/golden/`, and a `.ui.ron` fixture
  with bindings beside 2330's row.
- Snapshot: the four engine screens and three widgets under a test look
  file; a `look: "shop"` node.
- Integration: the harness tests above (shown, enabled, rows changing,
  reload of the look).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
