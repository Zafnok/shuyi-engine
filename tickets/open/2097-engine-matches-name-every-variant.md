---
id: "2097"
title: "The engine's own matches on its non-exhaustive enums name every variant, checked on nightly"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2097 — The engine's own matches on its non-exhaustive enums name every variant, checked on nightly

## Context

2082 to 2086 made the engine's growing enums `#[non_exhaustive]`
([ADR-0039](../../docs/adr/0039-public-types-that-can-grow-non-exhaustive-config-and-views-with-constructors-plain-values-left-open.md)).
That keeps a game compiling when a variant is added, as it should. But
an engine crate that matches another engine crate's enum must now have
a `_` arm too, and an arm that names some variants and folds the rest
into `_` swallows the next one without a word. The retro of PRs
#379–#484 (2026-11-17, problem 7) found it: #438 (2082) folded
`Paint::Image` into the `_` arm of `crates/app/src/render.rs`, line
379, so a new paint would draw as an image.

rustc's `non_exhaustive_omitted_patterns` lint finds exactly these: a
match on another crate's non-exhaustive enum whose `_` arm covers a
variant that exists today. It is unstable (issue 89554). Tried on
2026-11-17 with the nightly the `api` job already installs
(`api::NIGHTLY` in `crates/xtask/src/api.rs`, `nightly-2026-07-06`):

```bash
RUSTFLAGS='-Zcrate-attr=feature(non_exhaustive_omitted_patterns_lint) -W non_exhaustive_omitted_patterns' \
  cargo +nightly-2026-07-06 check --workspace --all-targets
```

Without the `-Zcrate-attr`, rustc only warns that the lint is unknown.
With it, the check took 1 m 28 s and found 69 matches:

- 64 in engine crates: `shuyi-cli` 20 (9 on `link::Answer` in
  `contract/attach.rs`), `shuyi-ui` 23 (6 in tests; 12 on `UiAction`:
  `widgets/ask.rs`, `menu.rs`, `name_entry.rs`, `toy/grid.rs`,
  `debug/camera.rs`, `dialogue/preview.rs`, `screens/key_bindings.rs`,
  `options.rs`, `profiles.rs`), `shuyi-platform` 7 (the
  `AudioRequest` filters in `harness.rs` and `tests/it/sounds.rs`),
  `shuyi-app` 6 (`render.rs` on `Paint`, `window.rs` on `Show`),
  `shuyi-sound-out` 3, `shuyi-script` 3, `shuyi-sound` 1 (a test),
  `shuyi-playdate` 1.
- 6 of those 64 are outside crates' enums, whose `_` is right: rhai's
  `EvalAltResult` and `ParseErrorType` (`crates/script/src/error.rs`)
  and cpal's `ErrorKind` and `SampleFormat`
  (`crates/sound-out/src/windows.rs`). 3 more are deliberate:
  `crates/ui/tests/it/growing_types.rs` shows that a game needs its `_`.
- 5 in a game's code: `examples/hello-grid` (3) and the book's
  `site/snippets` (2).

The retro also named `crates/platform/src/audio/player.rs:755` and
`crates/cli/src/contract/session.rs:718`. Both already name every
`MusicCommand` and keep `_` for the next one. The lint passes them
today and fails them when a variant is added. That is the point.

Three open tickets add variants to these enums and call it additive,
which stays true for games: 2316 (`UiAction::DebugPause`,
`DebugStep`), 2357 (`UiAction::DebugBack`) and 2405 (`RawInputEvent`'s
pointer events).

## Nick input

None.

## Scope

**In:**
- `cargo xtask omitted-patterns` (`crates/xtask/src/omitted.rs`). It
  runs `cargo +<api::NIGHTLY> check --all-targets` with `-p` for each
  engine crate (`layers::is_engine`), each with the features `api.rs`'s
  catalog checks it with. Its `RUSTFLAGS` are
  `-Zcrate-attr=feature(non_exhaustive_omitted_patterns_lint) -D
  non_exhaustive_omitted_patterns --cfg shuyi_omitted_patterns`, and it
  builds into `target/omitted-patterns`, away from the pinned build. It
  says what to do if the nightly is missing, as `cargo xtask api` does.
- CI's `api` job runs it after *Show the public API change*. `cargo
  xtask gates` runs it when that nightly is installed, and otherwise
  says it skipped it.
- Every one of the 55 engine matches the lint finds today names each
  variant. A `_` arm stays only for the variants to come.
- The 9 that must keep their `_` (the outside crates' enums, and
  `growing_types.rs`) take
  `#[cfg_attr(shuyi_omitted_patterns, allow(non_exhaustive_omitted_patterns))]`
  on the enclosing item, with a comment why. Attributes on an expression
  are unstable. `cfg(shuyi_omitted_patterns)` joins the workspace's
  `unexpected_cfgs` `check-cfg` list in `Cargo.toml`, and the copies of
  it in the `Cargo.toml`s of `crates/alloc-count`, `app`, `playdate` and
  `sound-out`.
- ADR-0039 says the rule under *How it is held*: an engine crate's
  match on another engine crate's non-exhaustive enum names every
  variant, and its `_` arm is for later ones only. The `input-actions`
  skill's line on adding a `UiAction` says the check then names each
  match to change.
- **Either order with 2316:** if 2316 is done, `DebugPause` and
  `DebugStep` are named in each match on `UiAction` here; if not,
  nothing to do (2316 names them in each match the check finds).
- **Either order with 2357:** if 2357 is done, `DebugBack` is named in
  each match on `UiAction` here; if not, nothing to do (2357 names it).
- **Either order with 2405:** if 2405 is done, its pointer variants are
  named in each engine match on `RawInputEvent` outside `shuyi-ui`; if
  not, nothing to do (2405 names them). None is there on 2026-11-17.

**Out (do not do):**
- `examples/`, `templates/`, `site/snippets` and the skill pack's
  examples. They are a game's code, and a game keeps its `_` arm
  (ADR-0039).
- Matches inside the enum's own crate, where `#[non_exhaustive]` does
  nothing and the lint can't see. Clippy's `wildcard_enum_match_arm` is
  a restriction lint that would also flag every `Option` and outside
  enum.
- Making an enum exhaustive again, or a variant non-exhaustive.

**Estimated size:** about 1,100 lines (code 850, tests 150, docs 100).

**Split:** past 1,000 lines, split off the `shuyi-cli` matches into a
new ticket, and leave `shuyi-cli` out of the check's crates until it
lands.

## Implementation steps

1. Run the check from *Context* with `-D` and the `-p` list, and
   save its findings. Check that the lint still has its name and
   feature gate on the nightly `api::NIGHTLY` names then. If
   `bump-rust` has moved it, use the new one.
2. `crates/xtask/src/omitted.rs` with its `Runner`-faked tests, the
   subcommand in `main.rs`, the step in `gates.rs`, and the step in
   `.github/workflows/ci.yml`'s `api` job.
3. The `check-cfg` entry, and the 9 `allow`s with their comments.
4. Name the variants, crate by crate: `shuyi-ui`, `shuyi-platform`,
   `shuyi-app`, `shuyi-cli`, then the rest. Keep each arm's behaviour.
   Where an arm did nothing for a variant, the new arm does nothing too.
   `render.rs`'s `Paint::Image` gets its own arm.
5. ADR-0039's paragraph and the `input-actions` skill's line, with
   `pwsh -File scripts/sync-agent-skills.ps1`. Then run the gates and
   `cargo xtask omitted-patterns`.

## Acceptance criteria

- [ ] `cargo xtask omitted-patterns` passes on the branch, and fails
      when a variant is added to `UiAction` (tried by hand, then
      reverted).
- [ ] Every one of the 55 matches names each variant, and no arm
      changed what it does: the existing tests pass unchanged.
- [ ] Only the 9 items in Scope carry the `allow`, each with a comment.
- [ ] CI's `api` job runs the check, and a PR that folds a variant into
      `_` fails it.
- [ ] Stable `cargo clippy --workspace --all-targets -- -D warnings`
      gives no `unexpected_cfgs` or `unknown_lints` warning.
- [ ] ADR-0039 states the rule and names the check.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`omitted.rs`, faked `Runner`): the command, the crates, the
  flags and the target folder it runs; a lint finding fails the run
  and names the file and line; a missing nightly gives the install
  line.
- The existing tests of every crate touched pass unchanged.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
