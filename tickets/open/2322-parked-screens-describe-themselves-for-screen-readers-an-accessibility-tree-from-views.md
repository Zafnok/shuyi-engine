---
id: "2322"
title: "Parked: Screens describe themselves for screen readers: an accessibility tree built from the engine's views, the focus, and announcements"
type: feature
milestone: "Engine: accessibility"
model: opus-5.5
effort: high
status: blocked
blocked_by: []
nick_input: none
completed:
---

# 2322 — An accessibility tree from the engine's views

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): the game doesn't ask for it, so this waits until after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20 and Godot 4.6. Both
talk to screen readers (NVDA, Narrator, VoiceOver, Orca) through
AccessKit; Shuyi doesn't. A screen reader can't read pixels or glyph
cells. It needs a tree of nodes, each with a role, a name, a value and
a state, and to know which one has the focus.

Shuyi is well placed for this. Views are plain data (ADR-0002 §4), menu
items carry values and labels are text keys translated when painted
([ADR-0020](../../docs/adr/0020-menu-items-carry-a-value-and-views-hold-labels-the-skin-translates.md)),
and the engine's own screens (options, profiles, key bindings, the
layout picker, the debug menu) and widgets (the menu, the question, the
name entry) know what they show.

This ticket builds the tree, headless and testable, in `shuyi-ui`. 3035
sends it to the system's screen reader. The game's own screens (its map
screen, its battle) are the game's work, through the same method.

## Nick input

None.

## Scope

**In:**
- **`shuyi_ui::access`**, plain data, `no_std` with `alloc`:
  - `AccessNode { id: NodeId, role: Role, name: String, value:
    Option<String>, state: NodeState, children: Vec<AccessNode> }`;
  - `Role`: `Window`, `Dialog`, `Menu`, `MenuItem`, `List`, `ListItem`,
    `CheckBox`, `Slider`, `TextInput`, `Button`, `Text`, `Group`.
    `#[non_exhaustive]`;
  - `NodeState`: disabled, checked, selected, and a slider's
    `(now, min, max)`;
  - `NodeId`: a `u64` from the screen's name and the node's path, stable
    from frame to frame;
  - `AccessTree { root: AccessNode, focus: Option<NodeId> }`.
- **`Screen::describe(&self, ctx: &Ctx<G>) -> Option<AccessNode>`**,
  defaulted to `None`. A defaulted method breaks no game, so the game's
  PR that moves the pin needs no change.
- **The engine's screens and widgets describe themselves**, from the
  views they already build, with names translated through the game's
  language pack:
  - the menu widget and its `ListCursor` (disabled rows, the focus);
  - the question widget (2309) as a `Dialog` of `Button`s;
  - `OptionsScreen` rows: a slider for a volume ("Music volume", 7 of
    10), a check box for on and off, a list for enums;
  - the name entry (2310) as a `TextInput` with its text;
  - the profiles, key bindings and layout picker screens, and the
    debug menu;
  - the text box (dialogue) as `Text` with the page shown.
- **Announcements:** `Ctx::announce(key, args, Politeness)` for things
  said without the focus moving ("Saved", "Turn 3"). `Polite` waits,
  `Assertive` interrupts. They are collected per frame.
- **`Game::frame`** builds the tree from the stack: the top screen and
  the overlays over it (screens under an opaque one are hidden). It
  hands an `AccessUpdate { tree: Option<AccessTree>, announcements }`
  out with the frame. The tree is `Some` only when it changed (compared
  by value). **Either order with 3003:** if 3003 is done, it goes to a
  `Platform::access(&AccessUpdate)` with a default that does nothing;
  if not, it goes out with what `Game::frame` returns, and 3003 moves
  it.
- **Seeing it:** `frame-png --access <file>` writes the tree as indented
  text, one node a line (`menu "Options"`, `  slider "Music volume" 7/10
  focused`). If 1129 is done, `shuyi scene --access` prints it too.
- **A warning in debug builds:** a screen that returns `None` is logged
  once by name, so a game sees which screens a screen reader can't read.
- **A guide**, `docs/accessibility.md`: what a game's screens should
  describe (a map screen says the cursor's tile and what stands on it),
  and how to test it with `frame-png --access`.

**Out (do not do):**
- talking to the operating system (3035);
- speech of our own: speech is the screen reader's job;
- the game's own screens (the game's tickets);
- the web (3035 writes a follow-up for it).

## Implementation steps

1. Read ADR-0020, the menu, question, options, name entry and profiles
   modules, and `Game::frame`.
2. Write the `access` types and the text printer.
3. Add `Screen::describe`, and implement it on each engine screen and
   widget, with a snapshot test each.
4. Add announcements and the `AccessUpdate` hand-out.
5. Add `frame-png --access`, the debug-build warning and the guide.
6. Run the gates, including the `no_std` build of `shuyi-ui`.

## Acceptance criteria

- [ ] Each engine screen's tree has a committed text snapshot.
- [ ] Moving the menu cursor moves `focus` to that item's node, and
      nothing else in the tree changes.
- [ ] A frame where nothing changed gives `tree: None`.
- [ ] An announcement is handed out once, on the frame it was made.
- [ ] Node ids are unique within a tree, and stable across frames.
- [ ] The toy game's options screen, printed with `frame-png --access`,
      is reviewed and committed.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each role's printing; disabled rows; a slider's values; a
  translated name in a second language.
- Property: ids unique for random menus; the tree equals itself built
  twice.
- Snapshot / integration: every engine screen's tree; the toy game's
  options screen through `frame-png --access`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
