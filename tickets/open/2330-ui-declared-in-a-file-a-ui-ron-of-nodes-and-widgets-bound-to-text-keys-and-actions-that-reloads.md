---
id: "2330"
title: "UI declared in a file: a .ui.ron of nodes and widgets bound to text keys and actions, built into a screen, and reloaded while the game runs"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["2329", "2220"]
nick_input: none
completed:
---

# 2330 — UI declared in a file

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): every feature
on Bevy's homepage. Bevy UI "can be composed in code or declared in the
Bevy Scene format". 2329 gives the layout in code. This ticket lets a
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
  - **text is always a text key**, translated when painted (ADR-0020);
    a literal string is an error;
  - **never a key:** a widget acts on the game's actions, and help text
    names keys through the keymap (`AGENTS.md` rule 8). The format has
    no field that names a key, and `shuyi check-keys` (2006) covers
    `.ui.ron` files;
  - errors name the file, the node path (`root.children[1]`) and the
    problem.
- **`DocScreen<G>`**, a screen built from a `UiDoc`: it lays out with
  2329, draws the widgets, hands focus between menus with the UI
  actions, and returns a `ScreenResult` (ADR-0018) with the chosen
  item's value. A game fills a `Slot` with a closure.
- **Reload:** a `DocScreen` holds a `Hot<UiDoc>` (2220); a changed file
  rebuilds it, keeping the focus on the item with the same value.
- One engine screen with a menu (2310's profiles screen, or another)
  moves to a `.ui.ron`, with its snapshot unchanged.

**Out (do not do):**
- logic in the file (no conditions or scripts; scripts are 2101's);
- an editor (2507).

## Implementation steps

1. Write the `UiDoc` types and format, with errors.
2. Write `DocScreen` over 2329's layout.
3. Add reload, and `check-keys` support if 2006 is done (else a line in
   2006).
4. Move that screen over.
5. Run the gates.

## Acceptance criteria

- [ ] The moved screen, built from `.ui.ron`, has the same snapshot as
      before.
- [ ] A literal string or a key name in a `.ui.ron` is an error naming
      the node path.
- [ ] Changing the file in a running harness test (2220's script)
      changes the screen and keeps the focus.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each widget's parse; each error.
- Fuzz-style: random bytes never panic the format (ADR-0007).
- Snapshot: the moved screen.

## Completion notes

*(Filled in by the session that completes the ticket.)*
