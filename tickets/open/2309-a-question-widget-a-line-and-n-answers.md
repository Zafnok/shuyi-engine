---
id: "2309"
title: "A question widget: a line of text and N answers with a focus and an answer for Cancel, its view and its glyph skin"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2309 — A question widget

## Context

- **The game ticket:** game#1163
  ([`1163-the-games-questions-on-the-engines-question-widget.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1163-the-games-questions-on-the-engines-question-widget.md))
  waits for this one. Found by the game's first engine-friction sweep,
  game#1153.
- **The job:** "ask the player a yes/no (or one of a few answers) over
  the screen and act on the answer".
- **Friction:** six copies in five of the game's screens (ratios 1.5×
  to 1.8×, and the 8.3× screen 2310 moves here), outcome 1. Each copy
  costs about 15 lines of logic, 10 of view and 20 of skin. Ranked
  seventh of the sweep's findings.
- **What the engine has:** the same thing twice, private to its
  screens: `screens/options.rs` (`Question`, `LeaveChoice`) with
  `options/view.rs` (`QuestionView`, `LeaveView`, `ChoiceView`) and
  `options/glyph.rs` (`paint_question`, `choices_line`), and again in
  `screens/key_bindings`. One of the game's screens copies
  `paint_question` and `choices_line` almost line for line.
- **The generic shape:** `Ask<T>`: a text key, the answers (`T`, each
  with a text key), the focus, the answer Cancel gives (or none, when
  Cancel does nothing), `handle(action) -> Option<T>`, a `view()` of
  plain data and a glyph skin function that paints it in a box over
  whatever is under it.
- **Outside tactics:** "Quit without saving?" in any game; a farming
  game's "Sell all 12 turnips?"; a platformer's "Restart the level?".

## Nick input

None.

## Scope

**In:**
- `widgets/ask.rs` (name it as the session likes): `Ask<T>`, its view
  and its glyph skin, with docs.
- The answers step left and right (and up and down) with wrap, with the
  menu sounds, as the options screen's question does today.
- `screens/options` and `screens/key_bindings` use it for their own
  questions, and their private copies go.

**Out (do not do):**
- A question as a screen of its own on the stack (2306 makes that a
  few lines for a game; this is a widget a screen holds).
- Text input in a question (the options screen's number box stays).
- Any game's screens.

**For the game:** additive. The game's PR that moves its pin follows
this one (game#1163).

## Implementation steps

1. Read the two private copies and their tests and snapshots.
2. Write the widget from them; move options and Key bindings onto it.
3. Their snapshots must not change.

## Acceptance criteria

- [ ] Unit: two and three answers, wrap, Cancel with and without a
      Cancel answer, Confirm returns the focused answer.
- [ ] The options and Key bindings snapshots are unchanged.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, as above.
- Snapshot: the widget alone, two answers, focus on the second.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
