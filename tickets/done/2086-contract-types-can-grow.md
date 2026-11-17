---
id: "2086"
title: "Before the first release, shuyi-cli's contract types that open tickets grow move under ADR-0039, Op::ALL becomes a slice, and check takes what it reads in its input"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-16
---

# 2086 — shuyi-cli's contract types that open tickets grow move under ADR-0039 before the first release

## Context

The last of five tickets that move, before 2005, every exhaustive public
type an open ticket plans to grow (2082 has the reasons and Nick's words
of 2026-11-14). A game depends on `shuyi-cli` for its tool program (the
template's `tools` feature, `templates/shuyi-default/base/crates/game/Cargo.toml:28,44`),
so its public API is a game's too. 2025 moved the contract's inputs onto
constructors. The audit found that the operations and the answers are
still exhaustive, and that about half the breaking tickets in the
tools' block come from `Op` alone.

| Type or item | Where | Grown by |
| ------------ | ----- | -------- |
| `Op` and `Op::ALL: [Op; 16]` | `crates/cli/src/contract.rs:81,133` | 2225, 2505, 2515, 2517, 2520, 2523, 2527, 2552, 2232, 2529 |
| `CheckOutput` | `crates/cli/src/contract/check.rs:56` | 2516, 2517 |
| `check(game, input)` | `check.rs:67` | 2223 and 2517 each add an argument |
| `ReplayStep`, `ReplayOutput` | `crates/cli/src/contract/replay.rs:89,116` | 2520 |
| `AddOutput` | `crates/cli/src/contract/add.rs:59` | 2526 |
| `ModuleEntry` (the `modules` answer's) | `crates/cli/src/contract/modules.rs:44` | 2526 |
| `LintCheck` | `crates/cli/src/contract/lint.rs:82` | 2579 |
| `Concern`, `Status` (the catalog's) | `crates/cli/src/catalog.rs:61,94` | 2502; seven module tickets add `Status::Building` |
| `Part` (`lang-status`'s) | `crates/cli/src/lang_status.rs:28` | 2351 |
| `Event` (the session's) | `crates/cli/src/contract/session.rs:165` | 2356 |
| `font_atlas::run` | `crates/cli/src/font_atlas.rs:666` | 2224 adds a parameter |
| `Project::rules: Option<String>` | `crates/cli/src/project.rs:100` | 2771 wants one or a list |

2552 says it breaks no API, but it adds an `Op`. This ticket's PR
corrects that line.

## Nick input

None.

## Scope

**In:**
- `Op` `#[non_exhaustive]`, and `Op::ALL` as `&'static [Op]`.
- The answers and enums in the table `#[non_exhaustive]` (ADR-0039
  kinds 3 and 6), and their call sites in the CLI, the MCP server and
  the tests.
- `check` reads the project root and the project from `CheckInput`
  (already `#[non_exhaustive]`), so later inputs are fields, not
  arguments.
- `font_atlas::run` takes an options struct of kind 4.
- `Project::rules` becomes a list (`Vec<String>`) that still reads a
  lone string from `shuyi.ron` (a serde `untagged` of one or many), so
  every `shuyi.ron` keeps loading.
- The listed tickets' planned `Breaking:` lines for these items become
  additive, and 2552's line is corrected.
- A `Breaking:` line in the completion notes; the template's tools
  example and the game's tools program are the callers to search
  (`gh search code --repo Zafnok/visions-of-shuyi 'Op::'`).

**Out (do not do):**
- The contract's JSON and `contract::VERSION`: what the version
  promises is 2088's.
- Any operation or field the listed tickets add.
- `check_keys::run`, `check_text::run_names` and `codec::decode_png`
  (2543 and 2565 change them).

**Estimated size:** about 700 lines (code 350, tests 250, docs 100).

## Implementation steps

1. The attributes; `Op::ALL` as a slice, and its users (the MCP tool
   list, the usage text, the schemas' writer).
2. `CheckInput` gains the root and the project; `check`'s callers move.
3. `font_atlas::run`'s options struct.
4. `Project::rules` as a list read from one or many; the lint's fence
   reads each.
5. Build the workspace, the templates and the tools examples; fix each
   literal and exhaustive match.
6. Edit the listed tickets' planned `Breaking:` lines, and 2552's.

## Acceptance criteria

- [x] Every item in the table is changed as above, and the contract's
      schemas and JSON answers are byte for byte the same (the schema
      tests pass with no file regenerated).
- [x] A `shuyi.ron` with `rules: "rules"` and one with a list both load.
- [ ] `cargo xtask api` lists each change, and the `Breaking:` line names
      each item.
- [x] No open ticket still plans a `Breaking:` line for one of these
      items.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Project::rules` from a string and from a list.
- The existing contract, schema and MCP tests pass unchanged.

## Completion notes

What was done:

- `contract::Op` is `#[non_exhaustive]`, and `Op::ALL` and `Op::SESSION`
  are `&'static [Op]`. `Op::SESSION` became a slice too, since 2356 and
  2357 add session tools. Their users (the MCP tool list and `tool`, the
  schemas' writer, `skills`' command list and the tests) iterate the
  slices.
- `#[non_exhaustive]` with a comment naming its ADR-0039 kind: kind 3,
  `CheckOutput`, `ReplayStep`, `ReplayOutput`, `AddOutput` and
  `ModuleEntry`; kind 5, the catalog's `Concern` (a field it gains is
  `#[serde(default)]`); kind 6, `LintCheck`, the catalog's `Status`,
  `lang_status::Part` and the session's `Event`. The notes are `//`
  comments, not doc lines: a doc line goes into the JSON schemas, and
  they stay byte for byte the same (`cargo shuyi schema` into a scratch
  folder and `diff -r` show none changed; the schema tests pass with no
  file regenerated).
- `CheckInput` gains `project: Option<Project>` (the root and the
  `shuyi.ron`), `#[serde(skip)]`, so not in the JSON. The CLI and the
  MCP server set it. `check` with a `target` and a project checks the
  targets (`check::targets`); without a project it fails as before, with
  a text naming the field. `check`'s signature is unchanged: 2223's and
  2517's checks read the project from the input.
- `font_atlas::run(&RunOptions)`: `RunOptions` is kind 4
  (`#[non_exhaustive]`, `Default`, fields `rules`, `command`, `fonts`,
  `wides`, `out_dir`, `chars`), with a doc example. The module's tests
  call it through a local `run` with the old arguments, so their call
  sites are unchanged.
- `Config::rules` is a `Vec<String>`, read from one name, a list, or
  either in `Some(…)`: a visitor, not a serde `untagged`, so the game's
  `rules: Some("trpg-core")` keeps loading (an untagged enum refuses
  `Some`). `lint` fences each package; none named is the limit it was.
  Test: `the_rules_packages_are_read_from_one_name_or_a_list`.
- The tickets that grow these items now grow them additively, each with
  a **From 2086 (done)** clause: `Op` in 2515, 2517, 2520, 2523, 2527,
  2232 and 2529 (both parked), and 2552's "no API break" line now gives
  the reason; `CheckOutput` in 2516 and 2517; `check`'s argument in 2223
  and 2517 (the project from `CheckInput::project`); `ReplayStep` and
  `ReplayOutput` in 2520 (and its stale `ReplayInput` and `CrashReport`
  lines: both were `#[non_exhaustive]` already); `LintCheck` in 2579;
  `Concern` in 2502; `Status::Building` in 2118, 2701, 2707, 2721, 2727,
  2741, 2745, 2756, 3101 and 3107; `Part` in 2351; `Event` in 2356;
  `font_atlas::run` in 2224; `rules` in 2771 (parked: its step now says
  the list is done). 2225, 2505 and 2526 planned no `Breaking:` line for
  these items.
- `docs/contract.md`'s `lint` row says `rules` names one package or a
  list.

Deviations:

- `CheckInput` loses `Eq` (it keeps `PartialEq`): `Config` has no `Eq`.
- Some existing tests changed, against "pass unchanged": the ones that
  iterated `Op::ALL` by value (`.iter().copied()` now), and the ones that
  set or read `config.rules` as an `Option`. Their assertions are
  unchanged.
- `cargo xtask api` needs a nightly and two tools (CI's `api` job); it
  was not run locally. CI's job checks the `Breaking:` lines below.

The game, searched on 2026-11-16 at its `main` (`3855377`, pin
`1ccb1acc`) with a shallow clone and `grep` for each item (`gh search
code` hit its rate limit). Its pin move changes:

- `crates/xtask/src/game_tools/tests.rs:990`: `font_atlas::run(rules,
  command, &fonts, &[], &dir, &chars)` builds a `RunOptions`.
- `crates/xtask/src/game_tools/tests.rs:641`:
  `project.config.rules.as_deref() == Some("trpg-core")` becomes
  `project.config.rules == ["trpg-core"]`.
- Nothing else: it names `Part::Dialogue` as a value, reads
  `ReplayRun::steps`' fields, and calls `check::check` and
  `check::targets` with `CheckInput::default()`; it matches no `Op`,
  `Event`, `LintCheck` or `Status`, and builds none of the answers. Its
  `shuyi.ron` (`rules: Some("trpg-core")`) loads unchanged.

Game: nothing to adopt beyond the pin move: the two call sites above
change with it.

Book: none, the book's tools chapter calls `check` with
`CheckInput::default()` and matches `Event::Quit { .. }`, both
unchanged; no page or snippet builds one of these types, matches `Op`,
or calls `font_atlas::run`.

Breaking: shuyi-cli: contract::Op: #[non_exhaustive], and Op::ALL and Op::SESSION are &'static [Op]; a match on Op has a `_` arm, and a use of the arrays iterates the slices

```migration
since: 0.1.0
what: Op is non-exhaustive and its lists are slices.
old: "Op (exhaustive), Op::ALL: [Op; 16], Op::SESSION: [Op; 8]"
new: "Op (#[non_exhaustive]), Op::ALL: &'static [Op], Op::SESSION: &'static [Op]"
before: |
  for op in Op::ALL.into_iter() { println!("{}", op.name()); }
after: |
  for op in Op::ALL.iter().copied() { println!("{}", op.name()); }
find: ["Op::ALL.into_iter()", "Op::SESSION.into_iter()"]
```

Breaking: shuyi-cli: CheckOutput, ReplayStep, ReplayOutput, AddOutput, ModuleEntry, catalog::Concern: #[non_exhaustive]; read their fields, never build them by literal or match them without `..`

```migration
since: 0.1.0
what: The contract's answers are read, not built; a pattern on one ends with `..`.
old: "CheckOutput { errors, targets }"
new: "CheckOutput { errors, targets, .. }"
before: |
  let CheckOutput { errors, targets } = output;
after: |
  let CheckOutput { errors, targets, .. } = output;
```

Breaking: shuyi-cli: LintCheck, catalog::Status, lang_status::Part, session::Event: #[non_exhaustive]; a match on one has a `_` arm

```migration
since: 0.1.0
what: A match on LintCheck, Status, Part or Event has a wildcard arm.
old: LintCheck, Status, Part, Event (exhaustive)
new: LintCheck, Status, Part, Event (#[non_exhaustive])
before: |
  match status { Status::Ready => "ready", Status::Planned => "planned", Status::Parked => "parked" }
after: |
  match status { Status::Ready => "ready", Status::Planned => "planned", _ => "parked" }
```

Breaking: shuyi-cli: contract::check::CheckInput: a field `project` (not in the JSON), and no `Eq`; `check` with a target reads the project from it

```migration
since: 0.1.0
what: CheckInput holds the project; check with a target needs it, or call check::targets as before.
old: "CheckInput: Eq"
new: "CheckInput: PartialEq, with project: Option<Project>"
before: |
  let input = CheckInput::default();
  let answer = check(&tools, &input);
after: |
  let mut input = CheckInput::default();
  input.project = Some(Project::load(root)?);
  let answer = check(&tools, &input);
```

Breaking: shuyi-cli: font_atlas::run: takes &RunOptions instead of six arguments

```migration
since: 0.1.0
what: font_atlas::run takes its options in a RunOptions built from default().
old: font_atlas::run(rules, command, fonts, wides, out_dir, chars)
new: font_atlas::run(&RunOptions)
before: |
  let said = font_atlas::run(rules, "shuyi", &fonts, &[], &dir, &chars)?;
after: |
  let mut options = font_atlas::RunOptions::default();
  options.rules = rules.clone();
  options.command = "shuyi".to_owned();
  options.fonts = fonts;
  options.out_dir = dir;
  options.chars = chars;
  let said = font_atlas::run(&options)?;
find: ["font_atlas::run(rules"]
```

Breaking: shuyi-cli: project::Config::rules: a Vec<String>, not an Option<String>; every shuyi.ron still loads

```migration
since: 0.1.0
what: Config::rules is a list of package names; shuyi.ron may write one name or a list.
old: "rules: Option<String>"
new: "rules: Vec<String>"
before: |
  assert_eq!(project.config.rules.as_deref(), Some("my-game-rules"));
after: |
  assert_eq!(project.config.rules, ["my-game-rules"]);
find: ["config.rules.as_deref()"]
```

Behaviour: shuyi-cli: `check` with a `target` and `CheckInput::project` set checks the targets, as `check::targets` does; without a project, its error text now names the input's project

Behaviour: shuyi-cli: `shuyi.ron`'s `rules` may name a list of packages, and `lint` fences each
