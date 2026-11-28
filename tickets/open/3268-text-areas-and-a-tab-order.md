---
id: "3268"
title: "Forms for the keyboard: multi-line text fields on the editing core, and next and previous focus over a screen's widgets with groups that hold the focus"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3232", "2355"]
nick_input: none
completed:
---

# 3268 — Text areas and a tab order

## Context

3232 gives text fields one editing core (caret, selection, undo,
clipboard). Its **Out** leaves "multi-line fields (a sign's text, chat
history): a follow-up ticket if a game asks". 2355's `FocusMap` moves
the focus by direction only, and its **Out** leaves jumping between
groups. The Bevy coverage audit of 2026-11-28 (`docs/ROADMAP.md`,
*Bevy's game coverage*) found the screens that need both: detective and
journaling games with a notebook, letters and signs in sandbox
builders, chat, and keyboard-first forms such as a character creator or
a settings screen with several text fields, where Tab moves to the next
field. Bevy has multi-line inputs and tab navigation with tab groups
(`examples/ui/text/multiline_text_input.rs`,
`multiple_text_inputs.rs`; `crates/bevy_input_focus/src/tab_navigation.rs`,
`autofocus.rs`).

## Nick input

None.

## Scope

**In:**
- **A text area:** a `TextArea` widget on 3232's core. Text wraps by
  2302's rules in cells, or by 2334's runs when a skin uses runs
  (**Either order with 2334:** if 2334 is done, wrap runs through its
  line breaker; if not, cells only, and 2334 adds runs to the text
  area). Up and down move the caret by visual line, keeping its column
  where they can; a limit of lines and characters; a scroll window
  that keeps the caret visible. The text stays a `String` in the
  game's state, changed only by the screen's commands.
- **Next and previous:** two new `UiAction`s, `FocusNext` and
  `FocusPrevious`, bound by default to Tab and Shift-Tab in the keymap
  (rule 8; the `input-actions` skill), with no pad default (the d-pad
  already moves). A screen's `FocusMap` gains an order: tree order, or a
  widget's own index.
- **Groups:** a group of widgets the order walks as one unit; a modal
  group holds the focus inside it (a dialog). A widget marked autofocus
  takes the focus when its screen opens.
- **Tab inside a text area** moves the focus, unless the area asks to
  insert a tab; Enter adds a line unless the area is one line.
- The reachability check of 2355 covers the order: every focusable
  widget is reached by `FocusNext` from any start.
- New types follow ADR-0039.

**Out (do not do):**
- rich text in a text area (3267 styles runs that are drawn, not
  edited);
- the IME's composing text (3329, parked);
- pointer selection by drag in a text area (3232 leaves it too).

**Estimated size:** about 1,300 lines (code 600, tests 550, docs 150).

**Split:** past 1,000 lines, split off next and previous focus (the
second and third bullets) into a new ticket.

## Implementation steps

1. Read 3232's core, 2355's `FocusMap`, 2302's wrap and the
   `input-actions` skill.
2. Write `TextArea`: wrap, visual-line moves, limits and scroll.
3. Add the two actions and their default bindings, with help text.
4. Add the order, groups, modal groups and autofocus to `FocusMap`.
5. Extend the reachability check.
6. Write the docs page: a detective's notebook and a two-field form.

## Acceptance criteria

- [ ] Down from column 7 of a line onto a shorter line puts the caret
  at its end, and the next Down returns to column 7 when it can.
- [ ] A text area at its line limit refuses Enter and says so.
- [ ] `FocusNext` from the last widget of a modal group goes to its
  first, never out of it.
- [ ] A screen with an autofocus field opens with that field focused.
- [ ] The key-bindings screen lists both new actions with their keys.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: visual-line moves, limits, the order, groups.
- Property: any edit sequence keeps the caret inside the text; the order
  reaches every focusable widget.
- Snapshot / integration: a Harness run filling a form with Tab.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
