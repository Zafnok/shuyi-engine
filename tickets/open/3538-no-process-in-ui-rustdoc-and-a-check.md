---
id: "3538"
title: "No ticket, game ADR or Nick in shuyi-ui's rustdoc, and check-links keeps every crate's rustdoc free of them"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3538 — No process in `shuyi-ui`'s rustdoc, and a check that keeps it so

## Context

Split from 2034 at its **Split:** line. The docs map's rule 4
(`docs/README.md`, *The rules for writing docs*) says nothing of the
repository's process (tickets, Nick, the game's pin) goes in rustdoc.
2034 rewrote the `//!` and `///` lines of every crate but `shuyi-ui`
that named a ticket, a game ADR, Nick or `AGENTS.md`, and made
`cargo xtask check-links` read the `docs/` paths in Rust comments
(`comment_paths`, `crates/xtask/src/links.rs`). Two pieces are left:

- On 2026-11-26, `crates/ui/src` (test files left out) still has 144
  such doc lines in 62 files, for example
  `crates/ui/src/debug/camera.rs:1` ("the game's ticket 0228, moved
  here in 1154", and "game ADR-0048" on line 3),
  `crates/ui/src/lib.rs:1` ("game ADR-0004") and
  `crates/ui/src/frame/color.rs:117` ("Nick, seeing it in the game").
  Count them with:

  ```bash
  grep -rnE '^\s*//[!/].*(tickets? [0-9]{4}|game ticket|game#|engine#|game ADR|\bNick\b|AGENTS\.md)' \
    crates/ui/src --include=*.rs | grep -vE '(/tests\.rs|_tests\.rs|/tests/)'
  ```
- No check stops a new one in any crate.

To a reader on docs.rs these point at a private backlog or another
repository. The engine's own ADRs (`ADR-NNNN`) are technical and stay.

## Nick input

None.

## Scope

**In:**
- **No process in `shuyi-ui`'s rustdoc.** Each line the grep above
  finds is rewritten to say the fact in engine words, or loses the
  aside, as 2034 did for the other crates: "(ticket NNNN)" goes;
  "(game ADR-NNNN)" goes, or becomes the engine ADR that holds the rule
  now if one does; "Nick, seeing it in the game" becomes what was
  chosen and why, in a sentence. History an engine developer needs goes
  in a plain `//` comment, which rustdoc doesn't publish.
- **And a check keeps it so.** In `links.rs`, beside `comment_paths`:
  `process_words(text) -> Vec<(usize, String)>`, for each `//!` and
  `///` line (not `//`), each match of `ticket NNNN`, `tickets NNNN`,
  `game ticket`, `game#`, `engine#`, `game ADR`, the word `Nick` and
  `AGENTS.md`. `run` runs it on `crates/*/src/**/*.rs`, leaving out
  `crates/xtask/` (its docs are about tickets by design), files named
  `tests.rs` or `*_tests.rs`, and `tests/` folders. Each match is
  `<file>:<line>: rustdoc names this repository's process (`<word>`):
  docs/README.md rule 4`.
- The module's docs and the `run-gates` skill's line on `check-links`
  say it also reads process words in rustdoc.
- **For the game:** comments and an xtask check of this repository. No
  API change.

**Estimated size:** about 600 lines (code 80, tests 120, docs 400: the
~144 rewritten doc lines counted removed and added).

**Out (do not do):**
- The other crates' rustdoc: 2034 did it, and the check finds any left.
- A bare ticket number with no word beside it ("moved here in 1154"):
  a bare four-digit check would flag years and sizes.
- Turning the engine's `ADR-NNNN` citations into links.
- Process in Markdown (the book, the skill pack, the error pages,
  `contract.md`): a later ticket can lint them the same way.
- Comments that aren't docs (`//`).

## Implementation steps

1. Add `process_words` and its call in `run` to
   `crates/xtask/src/links.rs`, with its tests; run `cargo xtask
   check-links` to list every line it finds.
2. Rewrite each in `crates/ui`, reading the code around it so the
   sentence is true; fix any it finds elsewhere the same way.
3. Update the module's docs and the `run-gates` skill's line, then run
   `cargo xtask check-links` (clean) and the gates.

## Acceptance criteria

- [ ] `cargo xtask check-links` reports `game ticket` in a `///` line of
      a crate's source, with its file and line (a unit test over a
      temporary tree), and passes on the repository.
- [ ] The same words in a `//` comment, in a `tests.rs` file or under
      `crates/xtask/` are not reported (unit test).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/xtask/src/links.rs`): `process_words` finds each listed
  word in `//!` and `///` lines, not in `//` lines or code, and not
  `Nicked` or `ticket` with no number; `run` reports one in a crate's
  source and skips a `tests.rs` file and `crates/xtask/`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
