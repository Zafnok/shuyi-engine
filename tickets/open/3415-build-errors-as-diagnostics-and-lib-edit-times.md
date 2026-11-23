---
id: "3415"
title: Build failures come back as a list of the compiler's diagnostics, and the compile-time gate times edits to the game's library and rules crate
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3415 — Build failures come back as a list of the compiler's diagnostics, and the compile-time gate times edits to the game's library and rules crate

## Context

Most of an agent's turns on a Rust game go to compile errors. Two things
make those turns slower than they need to be. Both came from a
blind-spot audit of the engine (2026-11-23, its proposed tickets 66 and
67), checked here against the code.

**A build error reaches the agent as the end of a text.** The MCP server
runs the game's tool program with `cargo run --quiet … mcp-host`
(`crates/cli/src/mcp/child.rs`, `Cargo::launch`, through
`Program::cargo_args` in `crates/cli/src/project.rs`). Cargo's stderr is
copied on and its last bytes are kept: `const TAIL: usize = 6000`
(`child.rs:20`, `keep_tail`). When the program doesn't build, the answer
is `SH3005` (`codes::GAME_PROGRAM`) with "the last of its output"
(`Program::stopped`; `crates/cli/errors/SH3005.md`). Cargo prints errors
in order, and the first one is usually the cause, so a long cascade
pushes the error that matters out of the tail. There is no file, line,
error code or rustc suggestion in the answer, though every other answer
of the contract is structured (ADR-0022). The engine already reads
cargo's JSON elsewhere: `lint` runs clippy with `--message-format=json`
(`crates/cli/src/contract/lint.rs`, `messages`, `place`,
`compile_error`), and `bench` builds with
`--message-format=json-render-diagnostics` (`crates/cli/src/bench.rs`).
The other builds of the tool program don't: `shuyi mcp --ready`
(`crates/cli/src/mcp.rs`, `ready`) prints cargo's text, and
`bake --content` (`crates/cli/src/contract/bake.rs`, `bake_content`)
answers `SH3005` with only the program's stdout. And no operation
compiles the game without starting a session: an agent that wants to
know whether its edit compiles calls `start`.

**The compile-time gate times the cheapest edit.** `cargo xtask
compile-times` (`crates/xtask/src/compile_times.rs`, tickets 2009 and
2070, ADR-0049, ADR-0120) appends a comment to
`const MAIN: &str = "crates/game/src/main.rs"` (line 46), the five-line
bin. Only the bin rebuilds and links. An agent edits the game's library
(`crates/game/src/lib.rs`, a screen) or the rules crate
(`crates/rules/src/lib.rs`, in the `grid` template), then runs `cargo
shuyi …`: the template's alias is `run -q -p <crate> --example tools
--features tools --` (`templates/shuyi-default/base/.cargo/config.toml`),
which builds the library again with `shuyi-cli` and links the `tools`
example. `cargo test` links more binaries. None of that is timed, and
`tools/compile-budget.ron` holds only the `main.rs` ratio and the
dependency count, so it can slow down unseen.

Related: 2510 adds `#[diagnostic::on_unimplemented]` text to the traits a
game implements; that text arrives in this ticket's diagnostics as any
other message does. 2905 researches hot reload of code.

## Nick input

None.

## Scope

**In:**
- A shared reader of cargo's JSON messages in `shuyi-cli`: each
  compiler message as a `Diagnostic` (level, rustc code, message, file,
  line, column, the rendered text, and rustc's suggestions).
- The MCP server builds the game's tool program with
  `--message-format=json` before it runs it, and answers a failed build
  with `SH3005` and the build's errors in order, the first first, in a new
  answer field `diagnostics`. The error's text names the first error with
  its place. The stderr tail stays only for a program that built and then
  stopped (a panic).
- `shuyi mcp --ready`, `bake --content` and `bench` use the same build
  step, so their `SH3005` carry the same list.
- A new contract operation `build`: compiles the game's tool program (or,
  with `workspace: true`, the whole workspace with every target, tests
  included) and answers its diagnostics, `ok` when there is no error. It
  starts no session and runs no game. CLI `shuyi build`, MCP tool `build`.
- `cargo xtask compile-times` also times a change in the game's library
  and one in the rules crate, each through `cargo build`, the `tools`
  build that `cargo shuyi` makes, and `cargo test --no-run`; the tools
  and test times are budgeted as ratios of the reference build, by OS,
  as ADR-0120 does.
- Public types (ADR-0039): `Diagnostic`, `Suggestion`, `BuildInput` and
  `BuildOutput` may grow, so each is `#[non_exhaustive]` with a
  constructor or `Default`. `Answer` gains a field: `Answer` is not
  `#[non_exhaustive]` and has `deny_unknown_fields`, so this breaks the
  API and the answer's shape. Raise the contract's `VERSION` (54 today)
  and write a `Breaking:` line and a `migration` block. A search of the
  game (`gh search code --repo Zafnok/visions-of-shuyi 'Answer::failed'`,
  at its pin 50472bbc) found no code that builds an `Answer`; its copies of
  the skill pack name `SH3005` and are refreshed at its next pin bump.
- A `Behaviour:` line: an `SH3005` from a failed build reads differently
  (the first error and its place, not the tail).

**Out (do not do):**
- The forwarded server's own build (`crates/cli/src/mcp/forward.rs`, a
  project's own server built from another workspace): it keeps its tail.
  Its compile errors are in the engine of that workspace, and the game's
  build it answers for carries this ticket's diagnostics.
- Saying that a running session's build is older than the sources, and
  answering on first contact in a new worktree: 3414 does it.
- Messages on the traits themselves: 2510 does them.
- Applying rustc's suggestions to the files: the answer lists them; the
  agent applies them.
- A lighter `tools` feature or a separate tools crate so a library edit
  doesn't build `shuyi-cli` twice: this ticket measures it first. If the
  numbers call for it, write that ticket.
- Hot reload of code: 2905.
- A timeout on the `compile-times` job: 3508 sets CI's timeouts.

**Estimated size:** about 1,400 lines (code 650, tests 550, docs 200).

**Split:** past 900 lines, split off steps 9–12 (the compile-time gate's
new edits and budgets) into a new ticket.

## Implementation steps

1. `crates/cli/src/build.rs` (new, with `build/tests.rs`): `pub struct
   Diagnostic { level: String, code: Option<String>, message: String,
   file: Option<String>, line: Option<u32>, column: Option<u32>,
   rendered: String, suggestions: Vec<Suggestion> }` and `pub struct
   Suggestion { file, line, column, end_line, end_column, replacement:
   String, applicability: String }`, with `Serialize`, `Deserialize` and
   `JsonSchema`. `pub fn diagnostics(root: &Path, stdout: &str) ->
   Vec<Diagnostic>` reads each line whose `reason` is `compiler-message`,
   in order; the place is the primary span's, the file shown from `root`;
   suggestions come from the children's spans that carry
   `suggested_replacement`. Move `messages` and `place` from
   `contract/lint.rs` here and make `lint` use them.
2. In `build.rs`, `pub fn build_tool_program(project: &Project, program:
   &GameProgram, extra: &[String]) -> Result<(), BuildFailed>`: runs
   `cargo` with `program.cargo_build_args()` plus
   `--message-format=json` and `extra`, copies each message's `rendered`
   text to this process's stderr (a person watching still sees cargo's
   errors), and on failure returns `BuildFailed { diagnostics, tail }`.
   `diagnostics` keeps the errors only, at most 50, in cargo's order;
   `tail` is cargo's last stderr lines, for a failure with no error
   message (a missing linker). Name the program type as `project.rs`
   names it.
3. `crates/cli/src/contract.rs`: `Answer` gets `pub diagnostics:
   Vec<Diagnostic>`, `#[serde(default, skip_serializing_if =
   "Vec::is_empty")]`, and `Answer::with_diagnostics`. Raise `VERSION`,
   regenerate the schemas and add the fingerprint to
   `crates/cli/schemas/versions.json`. When more errors were found than
   kept, add a line to `limits` with the count; add one with the count of
   warnings left out.
4. `crates/cli/src/mcp/child.rs`: `Cargo::launch` calls
   `build_tool_program` first. A failure returns `SH3005` with the error
   text "the game's tool program didn't build: N errors; the first:
   `<file>:<line>:<column>: error[<code>]: <message>`" and the
   diagnostics. `Launch::launch`'s error becomes a struct carrying the
   code, the text and the diagnostics (`mcp/server.rs`), and the
   server's `failed` answers keep the diagnostics, in `structuredContent`
   and in the tool result's text (the first error's `rendered`). The
   `cargo run --quiet` that follows has nothing left to compile. `TAIL`
   and `keep_tail` stay for a program that built and then stopped.
5. `crates/cli/src/mcp.rs`, `ready`: build through `build_tool_program`
   and print the first error's rendered text and the count.
   `crates/cli/src/contract/bake.rs`, `bake_content`: build with the
   content profile's arguments through `build_tool_program` before it
   runs, so a compile error answers `SH3005` with diagnostics and not
   "gave no answer of a bake". `crates/cli/src/bench.rs`: its build uses
   the same reader (it needs the `compiler-artifact` lines too: keep
   `executable` reading them from the same stdout).
6. The `build` operation: `Op::Build` (`#[serde]` name `build`) in
   `Op::ALL` and `Op::name`; `contract/build.rs` with `BuildInput {
   workspace: bool }` and `BuildOutput { errors: u32, warnings: u32,
   diagnostics: Vec<Diagnostic> }`. Without `workspace`, it builds the
   tool program as step 2 does, so a `start` after it compiles nothing.
   With it, `cargo check --workspace --all-targets
   --message-format=json`, and its diagnostics keep warnings too (at
   most 50 in all). A project with no `game` part answers `SH1002` as
   `start` does. It does not run the game: `runs_the_game` stays false
   for it, so the server answers it itself (`one_off`).
7. Wire `build` into the CLI (`contract/cli.rs`, the usage, `shuyi
   build [--workspace]`, a text output of each error's rendered text then
   the counts) and the MCP tool list (`mcp.rs`, `tool` and `tools`), with
   its input schema.
8. Docs: `crates/cli/errors/SH3005.md` (a failed build answers
   `diagnostics`, the first error is in the message; the tail is for a
   program that stopped), `docs/contract.md` (the `build` row, the
   `diagnostics` field of an answer, the lines at about 565 and 610 that
   say "the last of its output"), and each skill of the pack that names
   `SH3005` (`skills/shuyi-*/SKILL.md`: balance-bots, cinematic,
   debug-replay, dialogue, gates, new-game, screen, test): "read the
   answer's `diagnostics`, the first one first; call `build` to check a
   change compiles". Write an ADR (the `write-adr` skill) for an answer
   that carries the compiler's diagnostics and the `build` operation,
   amending ADR-0022.
9. `crates/xtask/src/compile_times.rs`: beside `MAIN`, `const LIB: &str =
   "crates/game/src/lib.rs"` and `const RULES: &str =
   "crates/rules/src/lib.rs"` (both in a `grid` game,
   `crates/cli/src/template/plans.golden`). The `n`th change to each
   appends a documented public function, `/// compile-times' change
   <n>.` then `pub fn compile_times_<n>() -> u32 { <n> }`, so the crates
   that depend on it really recompile; a comment would let them off.
10. Measure, per change (`lib`, `rules`): `cargo build`; `cargo build
    -p timed --example tools --features tools` (what `cargo shuyi`
    builds; the game is made with the name `timed`); and `cargo test
    --no-run`. Build the `tools` example and the tests once, untimed,
    before the first change, so the first timed one is a rebuild. Each of
    the `REBUILDS` rounds times, after its reference build, the `main.rs`
    rebuild as today, then the six new ones; each new ratio is its time
    over that round's reference, and the gate holds the median of each.
    Add the fields to `Times`, its `json` and its `table` (one row each,
    seconds and ratio).
11. `tools/compile-budget.ron` and `Budget`: `edits_over_reference: {
    "lib tools": { "linux": …, "windows": … }, "lib test": …, "rules
    tools": …, "rules test": … }`, and `check` fails a measure over its
    budget, or a measure with no budget for the OS, as it does for the
    `main.rs` ratio. The plain `cargo build` times are reported, not
    budgeted. Calibrate as ADR-0120 did: `gh workflow run calibrate.yml
    --ref <branch> -f job=compile-times -f count=20`, each budget the
    highest ratio measured plus 20 %, and the run's id, the commit and the
    measured spread in the file's comment. Then a second calibration run
    of 20 on the same commit, every run passing, named there too.
12. Write the job's time before and after, from the calibration runs, in
    the completion notes (`.github/workflows/compile-times.yml` has no
    `timeout-minutes` today; 3508 sets CI's timeouts and reads them).
    Amend ADR-0120 (the `write-adr` skill's
    way of changing an ADR) with the new measures and why they are
    ratios of the same round's reference. Update the module's doc
    comment (lines 11–14).

## Acceptance criteria

- [ ] A game whose library has one error and twenty more that follow from
      it, so that their text passes 6,000 bytes, answers `start` with
      `SH3005`, the first error's file, line and code in `error`, and
      every error in `diagnostics`, the first first.
- [ ] An error with a rustc suggestion (a misspelt name rustc offers to
      replace) carries it in `suggestions`, with its place and
      replacement.
- [ ] More than 50 errors keep 50, and `limits` says how many there were.
- [ ] A game that builds and panics before it answers still answers
      `SH3005` with the tail of its output and no `diagnostics`.
- [ ] `build` on a game that compiles is `ok` with zero errors, starts no
      session, and a `start` after it compiles nothing (cargo's
      `Finished` with no `Compiling` line).
- [ ] `build` with `workspace: true` finds an error that is only in a
      test file.
- [ ] `build` on a project with no `game` part answers `SH1002`.
- [ ] `shuyi mcp --ready` and `bake --content` on a game that doesn't
      compile name the first error with its place.
- [ ] `lint`'s answers are unchanged by the move of `messages` and
      `place` (its tests pass unchanged).
- [ ] `cargo xtask compile-times --json` reports the six new measures
      and their ratios; `--budget` fails a ratio over its budget and a
      measure with no budget for the OS.
- [ ] `tools/compile-budget.ron` names the two calibration runs, and every
      run of the second passed.
- [ ] The contract's `VERSION` is raised, the schemas test passes, and the
      completion notes have a `Breaking:` line with a `migration` block
      and a `Behaviour:` line.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `build::diagnostics` over recorded cargo JSON (an error with a
  code, one without, a warning, a suggestion, a message with no span, a
  `compiler-artifact` line it skips); the 50 cap and the limits lines;
  `compile_times`' new edits and ratios with the faked `Runner`, as its
  tests do today; `Budget` parsing with and without the new map, and
  `check` failing each new measure.
- Integration: the MCP server with a faked `Launch` whose build fails
  answers `SH3005` with the diagnostics in `structuredContent`; the CLI
  binary test (`crates/cli/tests/it/binary.rs`) runs `shuyi build` on a
  game from `shuyi new blank` with an error written into its library,
  and on the same game fixed.
- Snapshot: the `build` answer's schema, and the tool list with `build`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
