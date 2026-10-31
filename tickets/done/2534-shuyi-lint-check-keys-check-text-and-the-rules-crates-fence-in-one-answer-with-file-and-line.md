---
id: "2534"
title: "shuyi lint: check-keys, check-text and the rules crate's fence in one answer, with file and line"
type: feature
milestone: After MVP
model: implementation
effort: medium
status: done
blocked_by: ["2015"]
nick_input: none
completed: 2026-10-22
---

# 2534 — `shuyi lint`: the game's code checks in one answer

## Context

ADR-0005 §1 lists a `lint` operation in the tool contract: "`check-keys`,
`check-text`, the boundary gate", returning "findings with file and
line". Nothing builds it, and until ticket 2015 no ticket owned it.

What exists today:

- `shuyi check-keys` (`crates/cli/src/check_keys.rs`) and `shuyi
  check-text` (`crates/cli/src/check_text.rs`) are tools of their own,
  each printing its findings as text, outside the contract: no
  `--json`, no `limits`, no problem codes (ADR-0022, ADR-0028).
- 2015 fenced a new game's rules crate (`templates/grid/crates/rules`,
  ADR-0032 §4): its `clippy.toml` bans hash maps, floats, clocks, file
  I/O and outside luck, its `src/lib.rs` denies
  `clippy::float_arithmetic`, and `tests/layers.rs` fails on a screen or
  device crate under it. An agent sees these as clippy errors and a
  failing test, in two runs and two shapes.

An agent making a game ("make me a snake game", `docs/decisions.md`,
2026-10-19) wants one call that says everything wrong with its code's
shape, each finding with file, line and what to do, as `shuyi check`
does for the game's files.

## Nick input

None.

## Scope

**In:**
- A contract operation `lint` (`crates/cli/src/contract/lint.rs`), on
  the CLI as `shuyi lint [--json]` and on MCP as a tool like the others
  (ADR-0031). One answer holds:
  - `check-keys`' findings and `check-text`'s, each with file, line and
    message;
  - the rules crate's fence: `cargo clippy -p <rules package>
    --message-format=json -- -D warnings`, read for the lints
    `clippy::disallowed_types`, `clippy::disallowed_methods` and
    `clippy::float_arithmetic`, each with file, line, column and
    clippy's message (whose reason names what to use);
  - the layers test's finding: `cargo tree` of the rules package, as
    `tests/layers.rs` reads it, each banned crate with the
    `cargo tree -i` command.
- The rules package comes from `shuyi.ron` (a new optional `rules`
  field naming the package; the grid template sets it). Without it, the
  answer's `limits` say the fence wasn't checked.
- A problem code for a finding of each kind (`crates/cli/errors/`,
  ADR-0028), and the answer's JSON schema (`crates/cli/schemas/`).
- `contract.md`, *The operations*, and the grid template's `AGENTS.md`
  tools table gain `lint`.

**Out (do not do):**
- Changing what `check-keys` or `check-text` find, or removing their
  commands.
- The engine's own crates (2006 runs `check-keys` and `check-text` on
  them; `cargo xtask check-layers` keeps their layers).
- A fence in `templates/blank` (2511 owns the minimal template).

## Implementation steps

1. Read ADR-0005 §1, ADR-0022, ADR-0028, ADR-0031 and ADR-0032, and
   `crates/cli/src/contract/check.rs` as the pattern for an operation.
2. Add `rules` to `shuyi.ron` (`crates/cli/src/project.rs`) and to
   `templates/grid/shuyi.ron`.
3. Write `lint.rs`: run the two checks in process, run clippy and
   `cargo tree` as child processes from the project's root, and parse
   clippy's JSON messages with `serde_json` (pure functions over the
   text, tested without cargo).
4. Wire the CLI, MCP, the schema and the problem codes.
5. Write the docs (the `document` skill).

## Acceptance criteria

- [x] `shuyi lint --json` on a game made with `shuyi new grid` answers
      no finding and no limit.
- [x] With a `HashMap`, an `f32`, `Instant::now()` and `1.5 * 2.0`
      appended to the rules, it answers one finding each, with file and
      line, and exits 1.
- [x] With `shuyi-ui` added to the rules' dependencies, it answers a
      finding naming `shuyi-ui` and the `cargo tree -i` command.
- [x] A hard-coded key and a literal screen string are findings too.
- [x] Without `rules` in `shuyi.ron`, the answer's `limits` say so.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: clippy's JSON messages parsed into findings (each of the three
  lints, another lint left out, a message with no span); `cargo tree`
  lines into findings.
- Integration: `shuyi lint` on a made game, clean and broken as above,
  in CI's `templates` job for `grid`.

## Completion notes

- `shuyi lint [--json]` (`crates/cli/src/contract/lint.rs`) is a contract
  operation and an MCP tool that takes `project`. It runs `check-keys` and
  `check-text` in process, and `cargo clippy -p <rules> --message-format=json
  -- -D warnings` and `cargo tree` from the project's root through a
  `Cargo` trait (faked in the tests). Clippy's JSON is read for
  `disallowed_types`, `disallowed_methods` and `float_arithmetic`, each
  with file, line, column and clippy's reason; `cargo tree` for the crates
  of `NEVER`, each with its `cargo tree -i` command. A rules crate that
  doesn't compile, a cargo that fails, and a part `shuyi.ron` lacks are
  limits; a cargo that doesn't start answers `SH1003`.
- `lint` runs on the project, never in the game's tool program: in `shuyi
  mcp` it runs in the server, and `cargo shuyi lint` answers from the
  game's tool program without asking it for anything.
- New problem codes `SH2101` hard-coded-key, `SH2102` screen-text,
  `SH2103` rules-fence and `SH2104` rules-dependency, in the content class
  (exit 1): a finding is a problem in a file of the game, at a line. The
  `SH21nn` block keeps them apart from `shuyi_assets`' content codes.
- Contract version 10: the `lint` schemas, and `lint` in `Op`.
- `shuyi.ron` has `rules` (the rules package). The grid template sets it,
  and gains a `check_text` part with `max_literals: 0` over
  `crates/game/src`, skipping the tool program, the window, the settings
  file and the actions' keymap names; five ids in `lib.rs` and `stage.rs`
  got a `// check-text: not player text` comment with the reason. The
  template's CI and `AGENTS.md` gates run `cargo shuyi lint`.
- **Deviation:** `check-text` findings are every literal only when there
  are more than `max_literals` (as `check-text` itself fails), so a game
  with an allowance isn't flooded.
- `lint::NEVER` is a third copy of the template's layers-test list; a test
  keeps them equal, and the docs map's *Pairs* row says so.
- CI's `templates` job (grid) makes a game, breaks it as the acceptance
  criteria say, and checks each finding with `jq`; then removes `rules`
  and checks the limit. Verified locally on a made grid game: clean (no
  finding, no limit), broken (seven findings, exit 1), without `rules` (the
  limit).
- `cargo shuyi lint` builds the game's tool program first, so code that
  doesn't compile stops it before lint runs; `shuyi lint` from the engine's
  binary, or the MCP tool, still answers.
- Noticed, not fixed: `SH3005`'s help line in `codes.rs` has a run of
  spaces in the middle (a lost line continuation); a one-line fix for any
  ticket that touches the codes next.
- Nick: nothing to decide.
