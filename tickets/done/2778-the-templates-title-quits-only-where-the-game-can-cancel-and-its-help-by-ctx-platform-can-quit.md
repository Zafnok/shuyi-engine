---
id: "2778"
title: "The template's title quits only where the game can: Cancel and its help line by ctx.platform.can_quit(), and the rule in a new game's AGENTS.md"
type: feature
milestone: After MVP
model: sonnet-5
effort: low
status: done
blocked_by: ["3053"]
nick_input: none
completed: 2026-10-30
---

# 2778 — The template's title quits only where the game can

## Context

3053 gave a running game its platform's facts: `ctx.platform`
(`shuyi_ui::PlatformFacts`), with `can_quit()` false where the player
leaves through the system (the web's tab, a console's home button), and
`Harness::as_target(&TargetProfile)` to run a test as a target
([ADR-0060](../../docs/adr/0060-platform-facts-at-run-time-read-from-the-targets-profile-handed-through-setup-and-read-by-screens-never-by-rules.md)).
Hello, Grid's title hides its Quit there. 3053 grew past its size, so its
**Size** clause moved the template's part here.

What a new game has today (read 2026-10-30):

- `templates/shuyi-default/base/crates/game/src/title.rs`: Cancel
  returns `Transition::Quit` on every target, and the help line is
  `title.help` ("{Cancel} leave",
  `templates/shuyi-default/base/crates/game/files/en.ron`). The web
  build ignores a quit (`shuyi_app::run`, ADR-0024 §3), so on the web
  Cancel freezes the game on its last frame and the help names it.
- The grid stage (`stages/top-down-grid/.../top_down_grid.rs`) goes back
  to the title on Cancel (`Transition::Pop`); it never quits. 3053's
  plan for the grid template's stage (written when the grid template
  was its own folder) has nothing left to do, and no new text key is
  needed.
- `templates/shuyi-default/base/AGENTS.md`, *The layers*, says nothing
  of the facts.

## Nick input

None.

## Scope

**In:**
- `title.rs`: where `!ctx.platform.can_quit()`, Cancel does nothing and
  `TitleView::help` is empty (the title has nothing else to choose
  there; its `play` line stays). Its module and field docs say so.
- `base/AGENTS.md`, *The layers*, a bullet: "**Ask `ctx.platform`, never
  `cfg!`:** offer Quit only where `ctx.platform.can_quit()`. Screens read
  the facts; the rules crate never does." The docs map's pair *The layer,
  skin and key rules* holds with the root `AGENTS.md` rule 5's line
  3053 wrote.
- A test in `base/crates/game/tests/it/main.rs`: the title as `web`
  (`game::game().harness().as_target(&engine_profiles()?["web"].profile)`)
  doesn't quit on Cancel and its help is empty; as `desktop` Cancel
  quits.
- **Either order with 2803:** if 2803 is done, its steps under
  `site/tutorial/` get the same change (Cancel and any Quit only where
  `ctx.platform.can_quit()`). If not, nothing to do (2803 starts from
  the changed template).

**Out (do not do):**
- The grid stage: it goes back to the title, never quits.
- The book's snippets (`site/snippets/src/screens.rs`): 3053 changed
  them.

## Implementation steps

1. Read `templates/shuyi-default/base/crates/game/src/title.rs`, its
   test, and `docs/templates.md`.
2. Change `title.rs` and its test, and the `AGENTS.md` line.
3. Make a game from each short name (`cargo run -p shuyi-cli -- new
   <dir> --template blank --engine-path .`, then `grid`) and run CI's
   `templates` job's gates in it (`cargo fmt --all --check`, clippy,
   `cargo test --workspace`, `cargo shuyi check --target all`): the
   templates are outside `cargo fmt --all`.

## Acceptance criteria

- [x] A game made with `--template blank`, run as `web`: Cancel on the
      title doesn't quit and the help is empty; as `desktop`, Cancel
      quits.
- [x] The same for `--template grid`.
- [x] A new game's `AGENTS.md` has the line.
- [x] A game made from each short name passes CI's `templates` job's
      gates.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the template's `crates/game/tests/it/main.rs`, as above.

## Completion notes

The title (`title.rs`) now quits on Cancel only where `ctx.platform.can_quit()`,
and its `TitleView::help` is empty elsewhere (the web, a console). The
template's `AGENTS.md` has the `ctx.platform` bullet. A new test in the
template's `tests/it/main.rs` runs the title as `web` and `desktop`. A game made
from `blank` and from `grid` passed fmt, clippy, tests and `check --target all`.
`site/tutorial/` doesn't exist yet, so 2803 got a "From 2778 (done)" note
instead.

Book: none, no public API changed (template code and its AGENTS.md only).
