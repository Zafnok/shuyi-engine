---
id: "2305"
title: "The text box paints narration from the left like speech when the game asks, or centred as today"
type: feature
milestone: MVP
model: sonnet-5
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2305 — The text box paints narration from the left when the game asks

## Context

- **The game ticket:** game#0740
  ([`0740-narration-text-left-aligned-like-speech.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0740-narration-text-left-aligned-like-speech.md)).
  Nick asked for narration and speech to line up the same way, from
  the left.
- **The job:** let a game choose whether lines nobody says start at the
  same column as speech or sit in the middle of the box.
- **Friction:** no friction review: a missing option, found when the
  game's ticket was split between the two repositories.
- **The generic shape:** the dialogue text box's glyph skin
  (`crates/ui/src/dialogue/text_box/glyph.rs`, `paint_lines`) always
  centres a narration box's lines across the box, and a single line
  vertically when nothing follows it; speech starts at
  `BoxPlace::text_x`. A game picks where its box goes with `BoxPlace`;
  it should pick this too.
- **Outside tactics:** a visual novel whose narrator's lines read like
  everyone else's; an adventure game that centres them like a caption.

## Nick input

None.

## Scope

**In:**
- An alignment for narration in `BoxPlace` (e.g. `narration:
  NarrationAlign` with `Centred`, today's look, and `Left`, from
  `text_x` like speech), read by `paint_lines`.
- With `Left`: narration lines start at `text_x` and a single line stays
  on the first text row (as speech does); narration keeps its dim color.

**Out (do not do):**
- The color of narration, the caption line, or the speaker's name.
- Any game's box or text.

**For the game:** `BoxPlace` gains a field, so a game that builds it
with a struct literal adds it (`Centred` keeps its look). The game's PR
that moves its pin follows this one (game#0740).

## Implementation steps

1. Add the enum (documented) and the field.
2. `paint_lines`: branch on it for the column and the vertical place.
3. Update every `BoxPlace` the engine builds (the examples' and the
   tests') with `Centred`, so nothing changes.
4. Tests and the docs of the skin.

## Acceptance criteria

- [ ] With `Centred`, every existing snapshot is unchanged.
- [ ] With `Left`, a narration line starts at `text_x` and a single line
      sits on the first text row (tests of a look, reading cells).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (look): both alignments, one line and three lines.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
