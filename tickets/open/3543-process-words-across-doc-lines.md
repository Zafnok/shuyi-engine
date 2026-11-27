---
id: "3543"
title: The rustdoc process check reads a reference wrapped across two doc lines, and the ten left outside shuyi-ui are rewritten
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["3538"]
nick_input: none
completed:
---

# 3543 — The rustdoc process check reads a reference wrapped across two doc lines, and the ten left outside shuyi-ui are rewritten

## Context

The docs map's rule 4 (`docs/README.md`) keeps this repository's
process (tickets, the game's tickets and ADRs, Nick, `AGENTS.md`) out of
rustdoc: a game's developer reads it on docs.rs, where a ticket number
means nothing. Ticket 3538 added the check: `process_words` in
`crates/xtask/src/links.rs` finds `ticket NNNN`, `tickets NNNN`,
`game ticket`, `game#`, `engine#`, `game ADR`, `Nick` and `AGENTS.md` in
the `//!` and `///` lines of every crate's `src/` but `crates/xtask`'s
and the tests', and `cargo xtask check-links` fails on one.

`process_words` reads one doc line at a time. A reference that rustfmt's
or a writer's wrapping broke across two lines, a line ending in `ticket`
and the next starting `2234`, is never found. 3538 rewrote the six such
references of `shuyi-ui` by hand, its own crate; it left the others,
outside its scope. These ten remain on `main` (d30ba62d):

| File | Line | Wrapped words |
| ---- | ---- | ------------- |
| `crates/app/src/render.rs` | 24 | `ticket` / `2234` |
| `crates/assets/src/font.rs` | 561 | `ticket` / `3215` |
| `crates/assets/src/image.rs` | 17 | `ticket` / `2234` |
| `crates/assets/src/message.rs` | 519 | `ticket` / `2268` |
| `crates/assets/src/pack.rs` | 98 | `ticket` / `2263, ADR-0128 §2` |
| `crates/cli/src/mcp/aside.rs` | 15 | `ticket` / `2561, ADR-0115` |
| `crates/cli/src/mcp/aside.rs` | 181 | `ticket` / `2564` |
| `crates/platform/src/audio/cues.rs` | 231 | `game` / `ADR-0019` |
| `crates/platform/src/audio/player.rs` | 120 | `ticket` / `2234` |
| `crates/sound-out/src/windows.rs` | 57 | `ticket` / `3089` |

Found by reading each doc line (`//!`, or `///` not followed by `/`)
that ends in `ticket`, `tickets` or `game` and whose next doc line
starts with four digits or with `ADR`/`ticket`, in every `*.rs` under
`crates/` but `crates/xtask/`, `crates/ui/` (3538's) and the tests. The
line is the first of the two. The lines may have moved by the time this
ticket is worked: search again rather than trusting the table.

## Nick input

None.

## Scope

**In:**
- `process_words` also finds a word that starts at the end of one doc
  line and ends at the start of the next, the two joined by one space
  (`ticket` + `2234` reads as `ticket 2234`, `game` + `ADR-0019` as
  `game ADR`). It reports the first line's number, as a word on one line
  reports its own.
- The ten references in the table, and any the new check finds that the
  table missed, rewritten as 3538 rewrote `shuyi-ui`'s: the fact the
  ticket number stood for, said in the engine's words, or the aside
  dropped when the sentence stands without it. An `ADR-NNNN` beside the
  ticket number stays (ADRs are this repository's design record, not its
  process). `game ADR-0019` in `cues.rs` becomes the reason in plain
  words (the sound's RNG is apart from the simulation's so what plays
  never changes the game).
- A unit test of the join in `links.rs`'s tests.

**Out (do not do):**
- Changing which words count as process, or which files are read (3538
  settled both).
- References on one line: 3538's check already fails them.
- `crates/xtask/` (its docs are about tickets by design) and tests.
- Plain `//` comments and non-doc text.
- Any change to code: only doc comments change outside `crates/xtask`.

No public type or function changes; the API listing stays the same.

## Implementation steps

1. In `crates/xtask/src/links.rs`, make `process_words` see across a
   line break between two doc lines. One way: after the per-line scan,
   for each pair of consecutive doc lines (both `//!` or `///`, the
   second immediately after the first), take the last word of the first
   line's doc text and the start of the second's, join them with a
   single space (`"{tail} {head}"`), and run the same matcher on that
   joined text, keeping only matches that start in the tail and end in
   the head (so a word wholly on one line isn't counted twice). Factor
   the matcher out of the loop so both scans share it. A non-doc line
   between them ends the pair; leading spaces of the second line's text
   are trimmed before joining.
2. Update `process_words`'s doc comment to say it reads a word split
   across two doc lines too.
3. Run `cargo xtask check-links`. It now fails each line of the table
   (and of anything the table missed). Rewrite each as Scope says, in
   the same PR, and rewrap the paragraph with the crate's line width.
4. Run `cargo xtask check-links` again: it passes.

## Acceptance criteria

- [ ] `process_words` on `/// before ticket\n/// 2234 after` returns
      `(1, "ticket 2234")`; on `//! the (game\n//! ADR-0019) said` returns
      `(1, "game ADR")`.
- [ ] `process_words` returns nothing for `/// a ticket\n// 2234` (the
      second line isn't a doc line), for `/// a ticket\n\n/// 2234`
      (a line between), and for `/// a ticket\n/// 223x`.
- [ ] A word wholly on one line is reported once, not again by the join
      (`/// ticket 2234\n/// more` returns one word).
- [ ] None of the ten lines of the table names a ticket, a game ticket
      or a game ADR, on one line or across two, and
      `cargo xtask check-links` passes on the whole workspace.
- [ ] No file outside `crates/xtask/src/links.rs` changes but in its
      doc comments.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: in `crates/xtask/src/links.rs`'s tests, the cases of the
  acceptance criteria: a `ticket`/`NNNN` pair and a `game`/`ADR` pair
  across two doc lines found with the first line's number; a plain
  comment, a blank line or a non-digit second line ending the pair; a
  one-line word not counted twice.
- Integration: `cargo xtask check-links` over the repository passes
  (the gate).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
