---
id: "3261"
title: "The tests copied when code moved: cli's hand-checked frame, the blank template's fixtures and the lint-copy test, each kept once"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3261 — The tests copied when code moved, each kept once

## Context

Split from 3258 (shared test doubles), whose PR passed 1,000 counted
lines on its storage double and `Toy` overrides alone. The redundancy
sweep of 2026-11-28 found about 175 lines of tests copied when code
moved from one crate to another. Checked against `main` at 5fed216f:

- cli's `crates/cli/src/frame_png/tests.rs` (`tiny_atlas` line 22,
  `painter` 49, `hand_checked_frame` 66, `matches_a_hand_checked_frame`
  130, and `a_missing_image_draws_magenta_where_the_sprite_shows`)
  repeats ui's `crates/ui/src/frame/raster/tests.rs` (`tiny_atlas` 21,
  `painter` 48, `hand_checked_frame` 119, `matches_a_hand_checked_frame`
  183). The painter moved into `shuyi_ui::frame::raster` (2332); the
  cli's copy tests the same pixels again.
- The blank template's five `include_bytes!` constants (`en.ron`,
  `keymap.ron`, `palette.ron`, `font/atlas.ron`, `font/atlas.png` under
  `templates/shuyi-default/base/crates/game/files/`) and its `GOOD`
  table are written in four test files:
  `crates/cli/src/basic/tests.rs`, `crates/platform/src/basic/tests.rs`,
  `crates/ui/src/basic/tests.rs` and
  `crates/platform/tests/it/basic_reload.rs`.
- The test `the_lints_are_the_workspace_lints_except_unsafe_code` is
  nearly the same in `crates/app/src/lib.rs` (about 1509) and
  `crates/alloc-count/src/lib.rs` (about 202): each crate copies the
  workspace's `[lints]` because it allows `unsafe_code`.

## Nick input

None.

## Scope

**In:**
- Drop cli's hand-checked frame and its missing-image test (ui's
  `frame/raster/tests.rs` keeps both), and the helpers only they use.
  Keep cli's PNG-atlas and bad-atlas tests, which read the cli's own
  loading of a PNG.
- One fixture file, `crates/ui/src/basic/template_files.rs` (or beside
  the template if a path from all four is shorter), holding the five
  constants and `GOOD`, pulled into each of the four tests with
  `include!`. A test that adds rows to `GOOD` (platform's
  `preview_en.ron`) builds its own table from the shared one.
- An xtask check, `cargo xtask lint-copy` (a step of `gates --fast`),
  over every workspace crate whose `[lints]` is not `workspace = true`:
  its `[lints.*]` tables equal `[workspace.lints]` apart from the
  entries it names in a comment-free allowlist in the check (today
  `unsafe_code` for `shuyi-app` and `shuyi-alloc-count`). Remove the two
  copied tests.
- **Either order with 2335:** if 2335 is done, its turned-sprite checks
  in `frame_png/tests.rs` move to ui's `frame/raster/tests.rs` with the
  hand-checked frame; if not, nothing to do here (2335 writes them in
  ui's `frame/raster/tests.rs`).

**Out (do not do):**
- `GlyphBuffer::row_text` and the tests' row helpers: 3204.
- The cli's scratch and argv helpers: 3546.
- Changing what a remaining test asserts.

**Estimated size:** about 450 lines (code 150, tests 300), most of them
removed.

## Implementation steps

1. Delete the cli's copy of the hand-checked frame and its helpers that
   nothing else uses; run `cargo test -p shuyi-cli`.
2. Write the fixture file, `include!` it in the four tests, delete their
   copies.
3. Add `lint_copy` to `crates/xtask` (reading each crate's `Cargo.toml`
   and the root's with the `toml` the xtask has), wire it into
   `gates --fast`, and delete the two tests.

## Acceptance criteria

- [ ] `grep -rn "fn hand_checked_frame" crates` finds one, in `crates/ui`.
- [ ] `grep -rln "font/atlas.png\")" crates` finds the fixture file alone.
- [ ] `the_lints_are_the_workspace_lints_except_unsafe_code` is gone
      from both crates, and `cargo xtask lint-copy` passes on `main`.
- [ ] A crate whose `[lints]` is not `workspace = true` and that lacks
      an entry of `[workspace.lints]` fails `cargo xtask lint-copy`
      (a unit test of the check).
- [ ] Every ui, platform, app, cli and alloc-count test passes with the
      same names and assertions, apart from the deleted copies.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the lint-copy check passes a crate with the copy, and fails
  one missing an entry, one with a changed level, and one with an entry
  the allowlist doesn't name.
- The existing tests are the regression suite.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
