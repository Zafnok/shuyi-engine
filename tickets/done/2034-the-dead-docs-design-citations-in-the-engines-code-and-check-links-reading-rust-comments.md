---
id: "2034"
title: "The engine's rustdoc cites no game document, ticket or Nick, and check-links reads the docs paths Rust comments name"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-26
---

# 2034 — No game documents, tickets or Nick in rustdoc, and `check-links` on Rust comments

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-28). The engine's code was carved from the game
(`docs/carve.md`), and its comments still cite the game's design docs:
`docs/design/controls.md`, `docs/design/playdate.md` and
`docs/design/title-screen.md`. There is no `docs/design/` in this
repository, and there must never be one with the game's design in it
(`AGENTS.md`, rule 4). A reader who follows the citation finds nothing.

On 2026-11-23, 19 citations in `crates/` (no other folder has any; 2410
removed the three Playdate button ones):

| File | Lines | Cites |
| ---- | ----- | ----- |
| `crates/assets/src/keymap.rs` | 169, 203 | `controls.md`, *Rebinding keys* |
| `crates/ui/src/input/fixed.rs` | 1, 23 | `controls.md`, *Rebinding keys* |
| `crates/ui/src/input/help_keys.rs` | 5, 91 | `controls.md`, *Controller*, *Rebinding keys* |
| `crates/ui/src/input/help_keys/tests.rs` | 62 | `controls.md`, *Button names on screen* |
| `crates/ui/src/input/bindings.rs` | 6, 14, 237, 291 | `controls.md`, *Rebinding keys*, *Rebinding buttons*, *Dialogue* |
| `crates/ui/src/input/state.rs` | 38 | `controls.md`, *Switching between keyboard and controller* |
| `crates/ui/src/input/pad.rs` | 5, 113 | `controls.md`, *Controller*, *Button names on screen* |
| `crates/ui/src/input/pad/tests.rs` | 55 | `controls.md`, *Controller → Default buttons* |
| `crates/ui/src/frame/onebit.rs` | 22, 1086, 1539 | `playdate.md` (one-bit pictures, dithering, the faded listener) |
| `crates/ui/src/frame/glyph_buffer/backdrop.rs` | 15 | `title-screen.md`, *Intro cinematic* |

Nothing catches this: `cargo xtask check-links` (`crates/xtask/src/links.rs`)
reads only the links of `*.md` files, and a path in a Rust comment is
plain text in backticks. One other `docs/…` path a Rust comment names is
dead: `examples/hello-grid/examples/asset_memory.rs:2` cites
`docs/research/asset-memory.md`, and the report is
`docs/research/2906-asset-memory.md`. The other 12 distinct paths exist.
One comment names a glob, not a path: `crates/xtask/src/tickets.rs:16`'s
`docs/**/*.md`.

**The same rustdoc names this repository's process** (found by an
audit of the engine's blind spots, 2026-11-23). The docs map's
rule 4 (`docs/README.md`, *The rules for writing docs*) says nothing of
the repository's process (tickets, Nick, the game's pin) goes in
rustdoc. Yet on 2026-11-23 the `//!` and `///` lines of the crates'
sources (test files and `crates/xtask` left out) name a ticket, a game
ticket, a game ADR, Nick or `AGENTS.md` on about 310 lines in about 120
files:

| Pattern | Lines |
| ------- | ----- |
| `//!` with `ticket NNNN`, `game ticket`, `game#`, `engine#`, `game ADR` or `Nick` | 167 |
| `///` with the same (some on private items) | 139 |
| `AGENTS.md`, in either | 4 |

By crate: `ui` 151, `app` 43, `assets` 41, `cli` 35, `platform` 25,
`core` 5, `sound` 4, `sound-out` 1, `playdate` 1; `shuyi`, `script`,
`embed`, `embed-macros` and `alloc-count` none. Examples:

- `crates/ui/src/debug/camera.rs:1`: "The scene camera (the game's
  ticket 0228, moved here in 1154)", and "(game ADR-0048)" on line 3;
- `crates/ui/src/lib.rs:1`: "independent of any window or platform
  (game ADR-0004)";
- `crates/ui/src/frame/color.rs:117`: "Nick, seeing it in the game";
- `crates/ui/src/grid/sprite.rs:189`: "(Nick, 2026-10-13, game ticket
  0444)";
- `crates/platform/src/platform.rs:326`: "(`AGENTS.md` rule 5)".

To a reader on docs.rs these point at a private backlog or another
repository. No check stops a new one: xtask's other "Rule 4" checks
(`modules.rs`, `pins.rs`) are other rules, and `shuyi lint` lints games.
The engine's own ADRs (`ADR-NNNN`, about 600 doc lines) are technical,
not process, and stay; none is a link today.

## Nick input

None.

## Scope

**In:**
- **Each citation goes,** replaced by the engine's own source of the
  rule when it has one, or by the rule itself in one sentence in the
  comment when it hasn't. Never a game document, and never the game's
  words for its own screens (rule 4). The engine's homes for these
  rules:
  - the fixed `Escape` for `Cancel`: ADR-0036 (*Decision*), and rule 5
    of `skills/shuyi-input/SKILL.md`'s *The rules*;
  - rebinding, layouts and the bindings screen: the `shuyi-input`
    skill's *The player's keys*;
  - controllers (buttons by place, the Nintendo-style Confirm and
    Cancel swap, a stick as a direction): rule 7 of its *The rules*;
  - targets' buttons: ADR-0025, and `targets/<id>.ron`;
  - one-bit pictures and their shades: ADR-0026;
  - *Button names on screen*, *Switching between keyboard and
    controller*, *Dialogue* groups and the title backdrop: no engine
    page says them, so the comment states what the code does in a
    sentence (the backdrop's in engine words: a backdrop behind a
    title, not the game's intro).
- **From 2410 (done):** the three Playdate button citations (in `pad.rs`,
  `pad/tests.rs` and `chord_tests.rs`) are gone, citing
  `targets/playdate.ron` and ADR-0008 §1.
- **`asset_memory.rs` cites its report by its real name,**
  `docs/research/2906-asset-memory.md`.
- **`check-links` reads Rust comments too.** In `links.rs`, beside
  `relative_links`: `comment_paths(text) -> Vec<(usize, String)>`, for
  each line of a `*.rs` file whose text after the indent starts with
  `//` (so `//`, `///` and `//!`), every span in backticks that starts
  with `docs/`, with a `#anchor` and a trailing `.` or `,` cut off. A
  span with a `*` in it is a glob and is skipped. `run` walks `*.rs`
  files under `crates/` and `examples/` (with the same `SKIPPED_DIRS`
  and `SKIPPED_PATHS`) and reports each path that doesn't exist from
  the repository's root as `<file>:<line>: no such file `<path>``.
- **No process in the engine's rustdoc.** Each line the second table
  counts is rewritten to say the fact in engine words, or loses the
  aside. "(the game's ticket 0228, moved here in 1154)" goes;
  "(game ADR-0048)" goes, or becomes the engine ADR that holds the rule
  now if one does; "Nick, seeing it in the game" becomes what was
  chosen and why, in a sentence. Where the history matters to an
  engine developer, it goes in a plain `//` comment, which rustdoc
  doesn't publish. `AGENTS.md` rule 5 becomes the rule itself (only a
  backend touches a device), with ADR-0008 or ADR-0024 if one says it.
- **And a check keeps it so.** In `links.rs`, beside `comment_paths`:
  `process_words(text) -> Vec<(usize, String)>`, for each `//!` and
  `///` line (not `//`), each match of `ticket NNNN`, `tickets NNNN`,
  `game ticket`, `game#`, `engine#`, `game ADR`, the word `Nick` and
  `AGENTS.md`. The same walk runs it on `crates/*/src/**/*.rs`, leaving
  out `crates/xtask/` (its docs are about tickets by design), files
  named `tests.rs` or `*_tests.rs`, and `tests/` folders. Each match is
  `<file>:<line>: rustdoc names this repository's process (`<word>`):
  docs/README.md rule 4`. The module's docs and the `run-gates` skill's
  line on `check-links` say it reads `*.md` links, `docs/` paths in
  Rust comments and process words in rustdoc.
- **For the game:** comments and docs only, and an xtask check of this
  repository. No API change.

**Estimated size:** about 1,300 lines (code 150, tests 200, docs 950:
the ~330 rewritten doc lines counted removed and added).

**Split:** past 900 lines, split off the rewrite of `crates/ui` (about
150 lines) and the `process_words` check into a new ticket; the rest
lands first.

**Out (do not do):**
- Paths other than `docs/…` in comments (`crates/…`, `targets/…`,
  `skills/…`): many are module paths or examples, not files; a later
  ticket can widen the check.
- Rust comments in `templates/`: a made game has its own `docs/`, and
  the template's comments cite none of the engine's today.
- Changing any code, test or rule the comments describe.
- Writing a `docs/design/` or a page of controls rules: the
  `shuyi-input` skill and the ADRs are their homes.
- Turning the engine's `ADR-NNNN` citations into links (about 600 doc
  lines): they are technical, so rule 4 allows them, and a link needs a
  home on docs.rs that doesn't exist yet. A later ticket.
- A bare ticket number with no word beside it ("moved here in 1154"):
  every one found sits on a line the check already flags, and a bare
  four-digit check would flag years and sizes.
- Process in the book, the skill pack, the error pages and
  `contract.md` (rule 4's other homes): Markdown, not rustdoc; a later
  ticket can lint them the same way.
- Comments that aren't docs (`//`): rustdoc doesn't publish them.

## Implementation steps

1. `grep -rn "docs/design/" crates examples templates skills` to find
   every citation (the table above, or more if code moved).
2. Replace each as Scope says, reading the code around it so the
   sentence is true.
3. Fix the `asset_memory.rs` citation.
4. Add `comment_paths` and the `*.rs` walk to `links.rs`, with its
   tests.
5. Add `process_words` with its tests, and run `cargo xtask
   check-links` to list every process line (the second table, or more
   if code moved).
6. Rewrite each, crate by crate, as Scope says, reading the code
   around it so the sentence is true.
7. Update the docs Scope names, run `cargo xtask check-links` (clean),
   then the gates.

## Acceptance criteria

- [x] `grep -rn "docs/design" crates examples templates skills` finds
      nothing.
- [x] `cargo xtask check-links` reports a `docs/…` path in a `///`
      comment that doesn't exist, with its file and line (a unit test
      over a temporary tree, as `links.rs`' tests build one), and
      passes on the repository.
- [x] A path in backticks outside a comment (a string literal) is not
      read, and a glob is skipped (unit tests).
- [x] ~~`cargo xtask check-links` reports `game ticket` in a `///` line
      of a crate's source, with its file and line, and passes on the
      repository.~~ (not done: split to 3538 at the **Split:** line)
- [x] ~~The same words in a `//` comment, in a `tests.rs` file or under
      `crates/xtask/` are not reported (unit test).~~ (not done: split
      to 3538)
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/xtask/src/links.rs`): `comment_paths` reads `//`, `///`
  and `//!` lines, cuts an anchor and a trailing `.`, skips a glob, and
  skips code; `run` reports a missing path and passes an existing one.
- Unit (same file): `process_words` finds each listed word in `//!`
  and `///` lines, not in `//` lines or code, and not `Nicked` or
  `ticket` with no number; `run` reports one in a crate's source and
  skips a `tests.rs` file and `crates/xtask/`.

## Completion notes

- Every `docs/design/` citation in `crates/` is gone: each now cites
  the engine's home of the rule (ADR-0036, ADR-0026, or the
  `shuyi-input` skill's *The rules* and *The player's keys*), or says
  the rule in a sentence (button names on screen, the last device
  pressed, the backdrop's whole-step zoom). Process asides on the same
  lines went with them.
- `asset_memory.rs` and `examples/hello-grid/Cargo.toml` cite
  `docs/research/2906-asset-memory.md`.
- `check-links` reads `docs/` paths in Rust comments: `comment_paths` in
  `crates/xtask/src/links.rs`, run on every `*.rs` under `crates/` and
  `examples/`, with unit tests (each kind of comment, an anchor and a
  trailing `.` or `,` cut, a glob, a string, code before `//` and a span
  its line doesn't close skipped; `run` over a temporary tree, which
  skips `templates/` and `target/`). It passes on the repository.
- Every `//!` and `///` line of the crates other than `shuyi-ui` (and
  `xtask`, and tests) that named a ticket, a game ADR, Nick or
  `AGENTS.md` is rewritten: about 180 lines in about 85 files. Most lost
  the aside; `AGENTS.md` rule 5 became ADR-0008 §2, rule 9 ADR-0002 §4,
  game ADR-0013's license rule ADR-0002 §8.
- Deviation, the ticket's **Split:**: the PR would have passed 900
  lines, so the rewrite of `shuyi-ui`'s rustdoc (144 lines in 62 files)
  and the `process_words` check moved to ticket 3538. The acceptance
  criteria on `process_words` (the fourth and fifth) are 3538's.
- Docs: the module's docs, `cargo xtask`'s usage and the `run-gates`
  skill's line on `check-links` say it reads Rust comments' `docs/`
  paths.
- Game: nothing to adopt, comments and an xtask check of this
  repository only.
- No break. Book: none, no public API changed.
