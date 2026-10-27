---
id: "2330"
title: "UI declared in a file: a .ui.ron of nodes and widgets bound to text keys and actions, built into a screen, and reloaded while the game runs"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["2329", "2220", "2355"]
nick_input: none
completed:
---

# 2330 — UI declared in a file

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): every feature
on Bevy's homepage. Bevy UI "can be composed in code or declared in the
Bevy Scene format". 2329 gives the layout in code, and 2355 the focus
between laid-out widgets. This ticket lets a
screen's layout and widgets live in a text file, so a change shows in
the running game with hot reload (2220), and a future editor (2507,
parked) has a file to edit. ADR-0009 already asks for settings as data.

## Nick input

None.

## Scope

**In:**
- **`UiDoc`**, an asset kind in `shuyi-ui`, and its format `*.ui.ron`:
  ```ron
  UiDoc(
    root: Column(gap: 1, padding: 2, children: [
      Text(key: "title.main", align: Center),
      Menu(id: "main", items: [
        (label: "menu.start", value: "start"),
        (label: "menu.options", value: "options"),
      ]),
      Fill,
      Text(key: "footer.help"),
    ]),
  )
  ```
  - every `Style` field of 2329 as a field on any node;
  - widgets: `Text`, `Menu` (items carry values, ADR-0020), `Meter`,
    `TitledBox`, `Question`, `TextBox`, and `Slot(id)` for a part the
    game's code draws;
  - any widget may have an `id`, unique in the file (a repeat is an
    error): the game's code and the `focus` fields name a widget by it;
  - `Menu`, `TitledBox`, `Question` and `TextBox` take an optional
    `frame: "name"`: a nine-slice that the game's `GameData::nine_slice`
    resolves (2338), drawn in place of the `"panel"` or `"focus"` frame
    (`Ctx::panel_frame`) the widget wears with no `frame`. It names a
    picture, so it is not text and not a text key. **Either order with
    2338:** if 2338 isn't done, leave the field out, and 2235
    (nine-slices in a data file, after 2338) adds it;
  - **Either order with 2334:** if 2334 is done, `Text` takes an
    optional `font` (an id in `GameData::fonts`) and is drawn as 2334's
    `TextLabel` in a layout in pixels; if not, 2334 adds the field;
  - **Either order with 2347:** if 2347 is done, a `Text` node paints
    its text with `Ctx::label_styled` and `GlyphBuffer::print_styled`,
    so a `{Confirm}` in it gets the key role; if not, 2347 does it for
    `Text` when it lands;
  - a widget node's `focus` field, mapped onto 2355's `FocusNode`:
    `keeps` (directions the widget never gives away) and a neighbour
    per direction by `id`, as in `focus: (right: "buy")`; a `Menu`
    node's `wrap` (default true, as `Menu`; false is
    `Menu::without_wrap`), and a wrapping menu keeps Up and Down;
    `wrap_focus: true` on the doc (`FocusMap::wrapping`). A
    `FocusError` becomes an error naming the file and the node path;
  - **text is always a text key**, translated when painted (ADR-0020);
    a literal string is an error; a key whose text takes values (`{n}`,
    or 2348's plural and select) gets them from the game's code
    (`DocScreen::set_arg(id, name, value)`, on the node with that
    `id`), never from the file. The values are typed as `Label`'s args
    are: 2348's `Arg` (`impl ToArg`) if 2348 is done, else text (an
    `impl Display`, as `Label::with_arg` takes it today), and 2348
    retypes `set_arg` when it lands (its clause says so);
    - a `.ui.ron` may name the engine's own keys. **Either order with
      2351:** if 2351 is done, they resolve through the engine's
      English (`Lang::with_engine_english`), so the moved engine screen
      needs no English of the game's, and this PR makes 2351's
      `engine_english` test (`crates/ui/tests/it/engine_english.rs`)
      also collect the keys of the engine's `.ui.ron` files, so
      `crates/ui/lang/en/ui.ron` keeps them; if not, 2351 does it;
  - **never a key:** a widget acts on the game's actions, and help text
    names keys through the keymap (`AGENTS.md` rule 8). The format has
    no field that names a key, and `shuyi check-keys` already reads a
    `.ui.ron` in a game's asset folders as it reads any `.ron`
    (`crates/cli/src/check_keys.rs`); a test here shows it;
    - **From 2006 (done):** add the folder of the moved engine
      screen's `.ui.ron` to the root `shuyi.ron`'s `check_keys`
      `assets`, unless a folder there covers it already;
  - errors name the file, the node path (`root.children[1]`) and the
    problem.
- **`DocScreen<G>`**, a screen built from a `UiDoc`: it lays out with
  2329, draws the widgets (menus without the focus with
  `MenuView::active` false), moves the focus between them with 2355's
  `Focus` over a `FocusMap` built from the layout (each widget gets a
  direction first, and only a direction it leaves unused moves the
  focus), implements `Screen::widget_focus`, and returns a
  `ScreenResult` (ADR-0018) with the chosen item's value. A game fills
  a `Slot` with a closure.
- **Reload:** a `DocScreen` holds a `Hot<UiDoc>` (2220); a changed file
  rebuilds it, keeping the focus on the item with the same value and on
  the same widget (`Focus::rebuild`).
- **`shuyi check`** lays out every `.ui.ron` at the game's console size
  (and at each target's with `--target`) and fails when
  `FocusMap::unreachable` lists a widget, naming the file and the node
  path.
- One engine screen with a menu (2310's profiles screen, or another)
  moves to a `.ui.ron`, with its snapshot unchanged.

**Out (do not do):**
- logic in the file (no conditions or scripts; scripts are 2101's);
- an editor (2507).

## Implementation steps

1. Write the `UiDoc` types and format, with errors.
2. Write `DocScreen` over 2329's layout and 2355's `Focus`.
3. Add reload, the reachability check in `shuyi check`, and a
   `check-keys` test over a `.ui.ron` in a game's asset folder.
4. Move that screen over.
5. Do the either-order parts whose tickets are done: 2334's `font`,
   2338's `frame`, 2347's styled text, 2348's typed values for
   `set_arg`, 2351's test over the engine's `.ui.ron` keys, and the
   moved screen's folder in the root `shuyi.ron` (2006, done).
6. Run the gates.

## Acceptance criteria

- [ ] The moved screen, built from `.ui.ron`, has the same snapshot as
      before.
- [ ] A literal string or a key name in a `.ui.ron` is an error naming
      the node path.
- [ ] Changing the file in a running harness test (2220's script)
      changes the screen and keeps the focus.
- [ ] A `.ui.ron` with a list and a menu side by side moves the focus
      Right and Left between them in a harness test.
- [ ] A `.ui.ron` with a widget under a wrapping menu fails
      `shuyi check`, naming its node path; the moved screen's file
      passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each widget's parse; each error.
- Fuzz-style: random bytes never panic the format (ADR-0007).
- Snapshot: the moved screen.
- Integration: the harness test of the focus between two widgets;
  `shuyi check` on a `.ui.ron` with a widget no direction reaches.

## Completion notes

*(Filled in by the session that completes the ticket.)*
