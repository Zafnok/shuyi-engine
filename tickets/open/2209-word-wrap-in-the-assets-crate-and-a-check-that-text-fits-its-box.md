---
id: "2209"
title: "Word wrap in the assets crate, and a check that a line of text fits its box once its tokens are at their longest"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2209 — Word wrap in the assets crate, and a fits-its-box check

## Context

- **The game ticket:** game#1169
  ([`1169-content-loaders-on-the-engines-folder-loader-and-wrap.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1169-content-loaders-on-the-engines-folder-loader-and-wrap.md))
  waits for this one. Found by the game's first engine-friction sweep,
  game#1153.
- **The job:** "check, when the files load, that a line of text will
  fit the box it is shown in".
- **Friction:** the game's cinematics, 1614 lines against 1050 (1.5×),
  outcome 1, with a signal: **engine code copied into the game**. The
  game's content crate checks that a timed text shot fits its box with
  its own `page_lines` (about 30 lines), which re-writes the engine's
  `word_wrap` line count (`crates/ui/src/widgets/wrap.rs`) and its
  token expansion to the longest value (the dialogue check in
  `crates/assets/src/dialogue/check.rs`). It copies because
  `word_wrap` is in `shuyi-ui`, which a content crate doesn't depend
  on. When 2302 changes the break rules, the copy will drift: the check
  will pass text that the box then breaks differently.
  `DialogueLimits` already carries `box_width` and `box_lines`
  (`crates/assets/src/dialogue.rs`), but no engine check uses them.
- **The generic shape:** `shuyi_assets::text::word_wrap` (moved, with
  `shuyi_ui::widgets::word_wrap` re-exporting it), and
  `lines_at_longest(text, tokens, width) -> usize`, which wraps the
  text with every token at its longest; the dialogue check uses it with
  `box_width` and `box_lines` to report a line longer than a page.
- **Outside tactics:** a visual novel's or a farming game's dialogue,
  checked when it loads rather than seen cut off on screen.

## Nick input

None.

## Scope

**In:**
- Move `word_wrap` to `shuyi-assets`; `shuyi-ui` re-exports it so
  nothing breaks.
- `lines_at_longest`, and a check in the dialogue checks that a line
  fits `box_lines` pages of `box_width` (a warning or an error: follow
  what the check does for `max_text`).

**Out (do not do):**
- New break rules (2302). **Either order works:** if 2302 is done
  first, move its rules with the function; if this is done first, 2302
  changes the function in its new place.
- Wide glyphs (2301).
- Any game's checks.

**For the game:** additive. The game's PR that moves its pin follows
this one and deletes its copy (game#1169).

## Implementation steps

1. Move the function and its tests; re-export.
2. Write `lines_at_longest` from the dialogue check's token expansion.
3. Add the fits-its-box check with a test script.

## Acceptance criteria

- [ ] `word_wrap`'s tests pass in the new place, and `shuyi_ui`'s
      callers build unchanged.
- [ ] Unit: a line with a token whose longest value pushes it to a
      second page is reported; the same line with a short token isn't.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, as above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
