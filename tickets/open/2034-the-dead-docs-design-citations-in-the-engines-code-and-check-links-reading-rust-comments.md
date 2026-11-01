---
id: "2034"
title: "The dead docs/design citations in the engine's code go, and check-links reads the docs paths Rust comments name"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2034 — The dead `docs/design` citations, and `check-links` on Rust comments

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-28). The engine's code was carved from the game
(`docs/carve.md`), and its comments still cite the game's design docs:
`docs/design/controls.md`, `docs/design/playdate.md` and
`docs/design/title-screen.md`. There is no `docs/design/` in this
repository, and there must never be one with the game's design in it
(`AGENTS.md`, rule 4). A reader who follows the citation finds nothing.

On 2026-10-28, 22 citations in `crates/` (no other folder has any):

| File | Lines | Cites |
| ---- | ----- | ----- |
| `crates/assets/src/keymap.rs` | 165, 199 | `controls.md`, *Rebinding keys* |
| `crates/ui/src/input/fixed.rs` | 1, 23 | `controls.md`, *Rebinding keys* |
| `crates/ui/src/input/help_keys.rs` | 5, 90 | `controls.md`, *Controller*, *Rebinding keys* |
| `crates/ui/src/input/help_keys/tests.rs` | 62 | `controls.md`, *Button names on screen* |
| `crates/ui/src/input/bindings.rs` | 6, 14, 237, 291 | `controls.md`, *Rebinding keys*, *Rebinding buttons*, *Dialogue* |
| `crates/ui/src/input/state.rs` | 38 | `controls.md`, *Switching between keyboard and controller* |
| `crates/ui/src/input/pad.rs` | 5, 94 | `controls.md`, *Controller*, *Button names on screen* |
| `crates/ui/src/input/pad.rs` | 50 | `playdate.md`, *Buttons and the crank* |
| `crates/ui/src/input/pad/tests.rs` | 47 | `controls.md`, *Controller → Default buttons* |
| `crates/ui/src/input/pad/tests.rs` | 164 | `playdate.md`, *Buttons and the crank* |
| `crates/ui/src/input/chord_tests.rs` | 34 | `playdate.md` |
| `crates/ui/src/frame/onebit.rs` | 25, 1122, 1192 | `playdate.md` (one-bit pictures, dithering, the faded listener) |
| `crates/ui/src/frame/glyph_buffer/backdrop.rs` | 15 | `title-screen.md`, *Intro cinematic* |

Nothing catches this: `cargo xtask check-links` (`crates/xtask/src/links.rs`)
reads only the links of `*.md` files, and a path in a Rust comment is
plain text in backticks. Every other `docs/…` path a Rust comment names
today exists (12 distinct paths, the three above the only dead ones).

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
- **`check-links` reads Rust comments too.** In `links.rs`, beside
  `relative_links`: `comment_paths(text) -> Vec<(usize, String)>`, for
  each line of a `*.rs` file whose text after the indent starts with
  `//` (so `//`, `///` and `//!`), every span in backticks that starts
  with `docs/`, with a `#anchor` and a trailing `.` or `,` cut off.
  `run` walks `*.rs` files under `crates/` and `examples/` (with the
  same `SKIPPED_DIRS` and `SKIPPED_PATHS`) and reports each path that
  doesn't exist from the repository's root as
  `<file>:<line>: no such file `<path>``. The module's docs and the
  `run-gates` skill's line on `check-links` say it reads both.
- **For the game:** comments only, and an xtask check of this
  repository. No API change.

**Out (do not do):**
- Paths other than `docs/…` in comments (`crates/…`, `targets/…`,
  `skills/…`): many are module paths or examples, not files; a later
  ticket can widen the check.
- Rust comments in `templates/`: a made game has its own `docs/`, and
  the template's comments cite none of the engine's today.
- Changing any code, test or rule the comments describe.
- Writing a `docs/design/` or a page of controls rules: the
  `shuyi-input` skill and the ADRs are their homes.

## Implementation steps

1. `grep -rn "docs/design/" crates examples templates skills` to find
   every citation (the table above, or more if code moved).
2. Replace each as Scope says, reading the code around it so the
   sentence is true.
3. Add `comment_paths` and the `*.rs` walk to `links.rs`, with its
   tests, and update the docs Scope names.
4. Run `cargo xtask check-links` (clean), then the gates.

## Acceptance criteria

- [ ] `grep -rn "docs/design" crates examples templates skills` finds
      nothing.
- [ ] `cargo xtask check-links` reports a `docs/…` path in a `///`
      comment that doesn't exist, with its file and line (a unit test
      over a temporary tree, as `links.rs`' tests build one), and
      passes on the repository.
- [ ] A path in backticks outside a comment (a string literal) is not
      read (unit test).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/xtask/src/links.rs`): `comment_paths` reads `//`, `///`
  and `//!` lines, cuts an anchor and a trailing `.`, and skips code;
  `run` reports a missing path and passes an existing one.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
