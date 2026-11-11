---
id: "2256"
title: A dialogue line whose text starts with a bracket prints and parses back unchanged
type: bug
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2256 — A dialogue line whose text starts with a bracket prints and parses back unchanged

## Context

Game ticket game#0759
(`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0759-dialogue-text-starting-with-a-paren-round-trips.md`)
has been open since 2026-11-03. The game's property test
`print_parse_round_trip` failed on its `main` with a narration step
whose text is `():0`:

1. The engine's `print_scene` (`crates/assets/src/dialogue.rs`) writes
   the text as it is.
2. The parser (`crates/assets/src/dialogue/parse.rs`, `direction`) then
   reads the leading `( )` as the line's voice direction and reports "a
   line's ( ) has no direction in it".
3. A non-empty `(...)` at the start of a line's text would be read as a
   direction instead, and silently dropped from the text.

The engine's own round trip never puts `(` in text: its `arb_text`
(`crates/assets/src/dialogue/tests.rs`) is letters and `,.!?`.

The retro of 2026-11-08 found what it costs:

- the game's random-seed proptest can still flake on any PR (macOS-only
  reds in game #539, #544 and #557);
- pin PR game #557 narrowed the game's generator with `prop_filter` to
  get past it, which 0759's own *Out* forbids;
- no engine ticket existed.

No friction review: a bug. Any game with spoken lines that begin with a
bracketed aside ("(whispers) over here") hits it.

## Nick input

None.

## Scope

**In:**
- Text that starts with `(` survives print → parse. Either `print_scene`
  writes it so the parser reads it as text (an escape the `.dlg` format
  has, or a new one documented in its format page), or the parser rule
  is narrowed so that text can't be read as a direction. Pick whichever
  keeps written scripts unchanged.
- The engine's `arb_text` widened to printable ASCII, `(` and `)`
  included, and the game's shrunk case `():0` as a fixed unit test.
- If the format changes, a `Breaking:` line and a migration block. The game's PR moving the pin follows,
  and it removes the game's `prop_filter` (game#0759).

**Out (do not do):**
- Changing what a voice direction is or how it is written.
- The game's regression file (game#0759 adds it).

## Implementation steps

1. Add the unit cases first: text `()`, `():0` and `(soft) word` print
   and parse back as text. Watch them fail.
2. Fix `print_scene` or `direction` as chosen, and document it where the
   `.dlg` format is described (the module docs of
   `crates/assets/src/dialogue.rs`, and any page `grep -rln "\.dlg"
   docs site` finds).
3. Widen `arb_text` and run the round-trip property.

## Acceptance criteria

- [ ] `():0`, `()` and `(soft) word` round-trip as text.
- [ ] The engine's round-trip property generates `(` and `)`.
- [ ] Every `.dlg` in the repository (`git ls-files '*.dlg'`) parses to
      the same scenes as before (a test or a recorded comparison).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the three cases above.
- Property: the widened round trip.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
