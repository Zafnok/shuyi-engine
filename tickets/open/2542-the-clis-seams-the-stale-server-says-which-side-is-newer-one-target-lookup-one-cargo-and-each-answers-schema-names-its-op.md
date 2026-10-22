---
id: "2542"
title: "The CLI's seams: the stale-server limit says which side is newer, no run of spaces in a help line, one target lookup, one way to find cargo, and each answer's schema names its own op"
type: feature
milestone: MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2542 — The CLI's seams

## Context

Eight PRs changed the tool contract in two days (3006, 2126, 2509, 2530,
3040, 1133, 2534, 2535). A retro of them (2026-10-22) found they built on
each other on purpose: no logic was copied, and the contract's version
went 6 → 10, one per PR. It found five small seams where one PR's code
repeated or misread another's:

1. **The stale-server limit always blames the server.** 2535's
   `mcp::server::stale` (`crates/cli/src/mcp/server.rs`) fires when a
   host's answer speaks another contract than the server's `VERSION`, and
   says "the server was built before the engine changed; run
   "<command> mcp --ready", then reconnect it". That is right when the
   game's engine is newer. When the server is newer (a game's branch
   with an older pin checked out under a running server, or, once 2005
   releases `shuyi`, a `shuyi` installed globally over an old pin, as
   ADR-0031's Consequences name), the sentence is wrong. `--ready` and a
   reconnect still fix the first case, because the project's entry
   builds from its own pin, but not the second. Its test covers only
   `VERSION + 1` (`mcp/server/tests.rs`).
2. **A run of spaces inside two strings that agents read.** A line
   continuation was lost in SH3005's help line (`contract/codes.rs`,
   1133) and in `lint`'s MCP tool description (`mcp.rs`, 2534), so each
   has about ten spaces mid-sentence. Both reach agents: every SH3005
   answer, and `tools/list`. 2534's completion notes saw the first and
   didn't fix it.
3. **"No target profile" three times.** `lib.rs`' `target_profile` (for
   `frame-png --target`, 3006) exits 2 with no problem code. `check
   --target` (`contract/check.rs`) fails with `codes::TARGET` (SH3003).
   `bake` (`contract/bake.rs`, `unknown_target`) writes the message a
   third time. The same mistake gets different exit codes.
4. **`CARGO` found six times.** `std::env::var("CARGO")` or
   `var_os("CARGO")` with a `"cargo"` fallback is in `game.rs`,
   `web.rs` (twice), `mcp.rs`, `mcp/child.rs` and `contract/lint.rs`.
5. **Every output schema lists every op.** `Answer<T>.op` is an `Op`
   (`contract.rs`), so each `*.output.json` holds the whole `Op` enum, and
   each new op rewrites all 19 output schemas (2126, 1133 and 2534 each
   did). Two contract PRs in flight conflict on all of them. It also
   makes `run.output.json` and `session.step.output.json` the same file,
   though their answers' `op` differ.

## Nick input

None.

## Scope

**In:**
1. **`stale` names which side is newer.** When the game's engine is
   newer: today's sentence. When the server is newer: "the game's engine
   speaks contract X and this server Y: the server is newer than the
   game's engine; if the MCP entry runs a `shuyi` installed for every
   project, register the project's own (`<command> mcp install`), else
   run "<command> mcp --ready" and reconnect it". `docs/contract.md`,
   *The MCP server*, says both in one sentence each.
2. **The two strings** are one sentence each with single spaces, and a
   test over every row of the codes table (help line and page title) and
   every MCP tool description fails on two spaces in a row or a line
   break.
3. **One lookup of a target's profile:** `contract::target::profile(
   config, root, id) -> Result<TargetProfile, (codes::Code, String)>`,
   with the message `check` writes today and `codes::TARGET` for an
   unknown id (the profiles' load error keeps its own code). `check`,
   `bake` and `frame-png --target` call it. `frame-png` exits with the
   code's class's exit code (`codes::exit_code`) and prints the code, as
   the contract's commands do.
4. **One `cargo`:** `crate::cargo() -> std::ffi::OsString` (the `CARGO`
   that cargo sets when it runs us, else `cargo`), used by the six
   places.
5. **Each answer's schema names its op:** `contract::pair` (or
   `op_schemas`) writes the output schema's `op` property as a `const` of
   that op's name, as it serializes, instead of a `$ref` to `Op`. No
   output schema then holds `Op`'s other names. The answers themselves
   don't change. Regenerate the schemas, raise the contract's version and
   add the `versions.json` line, as *The schemas* in `docs/contract.md`
   says.

**Out (do not do):**
- a warning from `lint` when the server is stale: `lint` runs in the
  server and never asks the game's program (`docs/contract.md` says so);
- `session.frame_png` taking a `target`: it would change the `Session`
  trait, nobody has asked for the Playdate's picture over MCP, and
  `frame-png --target playdate` gives it;
- merging `bots` into `playtest` (2544).

## Implementation steps

1. Read `crates/cli/src/mcp/server.rs` (`stale`, `answer_of`) and its
   tests, `contract/codes.rs`, `mcp.rs` (the tool descriptions),
   `contract/target.rs`, `contract/check.rs`, `contract/bake.rs`,
   `lib.rs` (`target_profile`, the `frame-png` arm), and `contract.rs`
   (`pair`, `op_schemas`, `schemas`).
2. `stale`: test first, with `VERSION + 1` and `VERSION - 1`.
3. The strings and their test (in `contract/codes/tests.rs` and
   `mcp/tests.rs`).
4. `target::profile` and its three callers; test `frame-png --target
   moon` through the binary (`tests/it/binary.rs`) for the code and the
   exit code.
5. `cargo()` and its six callers.
6. The `op` const in the output schemas; a test that no
   `*.output.json` contains another op's name as an `op` value; then
   regenerate, raise the version and add the line.
7. `docs/contract.md`, then the `document` skill.

## Acceptance criteria

- [ ] `stale` with a newer server says the server is newer, and its test
      covers both directions.
- [ ] No help line, page title or MCP tool description holds two spaces
      in a row (the test).
- [ ] `check --target moon`, `bake --target moon` and `frame-png
      --target moon` print the same message with SH3003 and exit with
      the same code.
- [ ] `std::env::var("CARGO")` and `var_os("CARGO")` appear once in
      `crates/cli/src`.
- [ ] `run.output.json` and `session.step.output.json` differ in `op`,
      and adding an op to `Op` changes no other op's output schema.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `stale` both ways; the strings test; `target::profile` (known,
  unknown, profiles that don't load).
- Integration: `frame-png --target` with an unknown id through the
  binary; the contract's schema test (regenerated files and fingerprint).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
