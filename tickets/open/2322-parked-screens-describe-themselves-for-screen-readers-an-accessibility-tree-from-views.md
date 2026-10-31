---
id: "2322"
title: "Parked: Screens describe themselves for screen readers: an accessibility tree built from the engine's views, the focus, and announcements"
type: feature
milestone: "Engine: accessibility"
model: reasoning
effort: high
status: blocked
blocked_by: ["2405"]
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

**From 2323 (open):** Parked: the guide `docs/accessibility.md` is created here when this ticket lands first, and 2323 adds its section on text sizes; if 2323 lands first it creates the guide.

## Nick input

None.

## Scope

**In:**
- **`shuyi_ui::access`**, plain data, `no_std` with `alloc`:
  - `AccessNode { id: NodeId, role: Role, name: String, value:
    Option<String>, state: NodeState, bounds: Option<PxRect>, children:
    Vec<AccessNode> }`. `bounds` is the widget's or item's hit area in
    the frame (2405's `GlyphBuffer::hits`, matched by widget key and
    item), for magnifiers and touch exploration. It is `None` where a
    widget records none (2405 keeps only the top screen's hits);
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
  - the text box (dialogue) as `Text` with the page shown;
  - **Either order with 3036 (parked):** if 3036 is done, its
    achievements screen too: a `List` of `ListItem`s, unlocked or not,
    progress as the value, a hidden one named as hidden; if not, nothing
    to do (3036 describes it when it lands);
  - **Either order with 3054 (parked):** if 3054 is done, its high-score
    screen too: a `List` named by the board, a `ListItem` per entry with
    its rank, name and score; if not, nothing to do (3054 describes it
    when it lands);
  - **Either order with 2355** (focus between widgets): if 2355 is
    done, on a screen whose widgets share the focus through its
    `Focus` (`Screen::widget_focus`), the tree's `focus` is the focused
    item inside the widget `Focus::at()` names, and a menu with
    `MenuView::active` false has no focused item; if not, nothing to do
    here (2355 does it, its clause says so).

  Text that a view holds as `StyledText` (2347: help lines, a question's
  keys, the text box's wait corner) gives its `plain()` as the node's
  name. A `Key` span is read as written, with no extra word (a follow-up
  may announce it as a key).
- **Announcements:** `Ctx::announce(key, args, Politeness)` for things
  said without the focus moving ("Saved", "Turn 3"). `Polite` waits,
  `Assertive` interrupts. They are collected per frame.
- **Either order with 2366 (parked):** if 2366 is done, `Game` announces
  each new sound caption line (not a repeat) `Polite` through
  `Ctx::announce` while sound captions are on; if not, 2366 does it.
- **`Game::frame`** builds the tree from the stack: the top screen and
  the overlays over it (screens under an opaque one are hidden). It
  hands an `AccessUpdate { tree: Option<AccessTree>, announcements }`
  out with the frame. The tree is `Some` only when it changed (compared
  by value). It goes to a
  `Platform::access(&AccessUpdate)` (3003's boundary) with a default
  that does nothing. **Either order with 2367** (overlays that let the screen below
  run): if 2367 is done, the focus is in the top-most running screen
  that takes actions, so a `Passes::Everything` notice never holds it,
  and the screens a pass-through overlay lets run are in the tree under
  it; if 2367 isn't done, every overlay is modal and the focus is in the
  top screen, and 2367 adds the pass-through case (its clause says so).
- **Seeing it:** `frame-png --access <file>` writes the tree as indented
  text, one node a line (`menu "Options"`, `  slider "Music volume" 7/10
  focused`), and `shuyi scene --access` (1129) prints it too.
- **A warning in debug builds:** a screen that returns `None` is logged
  once by name, so a game sees which screens a screen reader can't read.
- **A guide**, `docs/accessibility.md`: what a game's screens should
  describe (a map screen says the cursor's tile and what stands on it),
  and how to test it with `frame-png --access`.

**Out (do not do):**
- talking to the operating system (3035);
- speech of our own: speech is the screen reader's job;
- the game's own screens (the game's tickets);
- the web (3058, after 3035).

## Implementation steps

1. Read ADR-0020, the menu, question, options, name entry and profiles
   modules, `Game::frame`, and 2355's `focus` module if it is done.
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
- [ ] A menu row's node has its row's bounds.
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
