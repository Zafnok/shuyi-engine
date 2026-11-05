---
id: "3094"
title: "The grid template's crash record: its stage's record in a crash report, given through BasicGame"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["3046"]
nick_input: none
completed:
---

# 3094 — The grid template's crash record

## Context

3046 gave every game on `shuyi-app` crash reports
([ADR-0079](../../docs/adr/0079-crash-reports-one-panic-hook-per-backend-writing-through-the-games-storage-offered-once-and-the-games-record-added-on-native.md),
`docs/crash-reports.md`). On native, `shuyi_app::run` catches a panic of
the game's frame and adds the game's record of its play to the report:
`App::crash_record` calls the hook the game gave `Start::crash_record`
(`type CrashRecord<G> = fn(&Game<G>) -> Option<String>`). hello-grid gives
one (`examples/hello-grid/src/screens/stage.rs`: `StageRecord` moved there
from `tools.rs`, `crash_record`, and `StageScreen::pending`, the command
the rules are applying, so a panic inside them is in the record).

3046's **Size** rule moved the grid template's half here: that PR passed
650 changed lines of code and tests. There is a second reason it needs its
own design: since 2511 the template's `main` is one `run_game` call, and
the `Start` is made inside `BasicGame::start`
(`crates/platform/src/basic.rs`), so the template has no `Start::new` to
add `.crash_record(..)` to.

What is there today, in `templates/shuyi-default/stages/top-down-grid/`:

- `crates/game/src/stages/top_down_grid/tools.rs` holds `StageRecord`
  (seed, commands, `made`), behind the `tools` feature, which the shipped
  binary doesn't build; `top_down_grid.rs` re-exports it with the tools;
- `top_down_grid.rs`'s `StageScreen` keeps its `seed` and its commands;
- `crates/game/tests/stage.rs` reads a `StageRecord` the tools wrote.

## Nick input

None.

## Scope

**In:**
- **`BasicGame` takes a crash record.** `BasicGame::crash_record(hook)`,
  a builder like `screens` and `modules`, with
  `hook: CrashRecord<BasicData<A, T>>`; `BasicGame::start` passes it to
  `Start::crash_record`. A new private field; `BasicGame`'s public shape
  grows additively (ADR-0039).
- **The template's stage:**
  - `StageRecord` moves out of `tools.rs` into `top_down_grid.rs`, which
    the shipped binary builds; `tools.rs` names it from there, and
    `top_down_grid.rs`'s `pub use` keeps `StageRecord` public without the
    `tools` feature;
  - `StageScreen::send` (or the template's equivalent) keeps the command
    it is applying in a public `pending: Option<Command>` field until the
    rules return, and the record ends with it;
  - `pub fn crash_record(game: &Game<Data>) -> Option<String>`: the open
    `StageScreen`'s `StageRecord`, `made: Some(crate::made())`, written
    with `ron`; `None` when no stage is open. `StageScreen` answers
    `as_any_mut` if a test needs it;
  - the game's `BasicGame` (in the template's `crates/game/src/lib.rs` or
    the part that builds it) adds `.crash_record(..)` when the stage part
    is in. The blank template, without a stage, adds nothing.
- **Either order with 2512:** if 2512 is done, `cargo xtask
  template-lines` still passes; if not, nothing to do.
- The templates are outside `cargo fmt --all`: format them as CI's
  template job checks them.
- `docs/crash-reports.md`'s *The game's record*: a line on
  `BasicGame::crash_record` for a game on the short path.

**Out (do not do):**
- Anything of 3046's that is done: the hook, `CrashReport`,
  `Setup::last_crash`.
- `replay --crash`: 2520.

## Implementation steps

1. Read `crates/platform/src/basic.rs`, `crates/platform/src/app.rs`
   (`Start::crash_record`, `App::crash_record`), hello-grid's
   `screens/stage.rs` and the template's stage part.
2. Add `BasicGame::crash_record` and pass it on in `start`, with a unit
   test that `start()` carries it.
3. Move the template's `StageRecord`, add `pending` and `crash_record`,
   and wire the game's `BasicGame`.
4. Add the test below, make a new game with `shuyi new` from the template
   and run its tests (CI's template job), and update the docs.

## Acceptance criteria

- [ ] `BasicGame::crash_record` reaches `App::crash_record` (a unit test
      through the headless backend).
- [ ] In a game made from the template with the stage, a stage with two
      commands taken and a third pending gives a record of the three,
      which its tools' `replay` reads; with no stage open, `None`.
- [ ] `StageRecord` still names the record type for the tools and tests.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-platform`): `BasicGame::crash_record` in `start()`.
- Integration (the template's `crates/game/tests/stage.rs`): the record
  with a pending command, read by its tools' `replay`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
