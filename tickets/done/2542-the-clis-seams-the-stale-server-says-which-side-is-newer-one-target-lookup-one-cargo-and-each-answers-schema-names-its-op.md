---
id: "2542"
title: "The CLI's seams: the stale-server limit says which side is newer, no run of spaces in a help line, one target lookup, one way to find cargo, and each answer's schema names its own op"
type: feature
milestone: MVP
model: implementation
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-26
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
   `VERSION + 1` (`mcp/server/tests.rs`). Since 2546 a fresh server of
   the checkout answers first, and the limit is written only where
   Scope item 1 says.
2. **A run of spaces inside two strings that agents read.** A line
   continuation was lost in SH3005's help line (`contract/codes.rs`,
   1133) and in `lint`'s MCP tool description (`mcp.rs`, 2534), so each
   has about ten spaces mid-sentence. Both reach agents: every SH3005
   answer, and `tools/list`. 2534's completion notes saw the first and
   didn't fix it. 2546 rewrote SH3005's help line with single spaces;
   `lint`'s description still has the run, and the test below is still
   wanted.
3. **"No target profile" three times.** `lib.rs`' `target_profile` (for
   `frame-png --target`, 3006) exits 2 with no problem code. `check
   --target` (`contract/check.rs`) fails with `codes::TARGET` (SH3003).
   `bake` (`contract/bake.rs`, `unknown_target`) writes the message a
   third time. The same mistake gets different exit codes.
4. **`CARGO` found eight times.** `std::env::var("CARGO")` or
   `var_os("CARGO")` with a `"cargo"` fallback is in `game.rs`,
   `web.rs` (twice), `mcp.rs`, `mcp/child.rs` and `contract/lint.rs`,
   and since the retro in `mcp/forward.rs` (2546) and `catalog.rs`
   (2003).
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
1. **`stale` names which side is newer, and what fixes it where the
   limit is written now.** When the server is newer, it also says "if
   the MCP entry runs a `shuyi` installed for every project, register
   the project's own (`<command> mcp install`)". Change the messages
   and leave the restart alone. `docs/contract.md`'s *A stale server
   restarts* says each case in one sentence, and doesn't bring back the
   "run `--ready`, then reconnect" advice 2546 took out of it. Since
   2546 the limit is written in two kinds of place:
   - beside "a fresh server of this checkout didn't start: …", when a
     forwarding server's restart (a forwarded server of its own
     workspace) fails. Its next call tries the restart again, so the
     fix is what stopped the fresh server, not `--ready` and a
     reconnect;
   - where no restart runs: in a forwarded server
     (`SHUYI_MCP_FORWARDED` set), whose answer the session's server
     passes on as it is, or for a folder with no server of its own to
     start. There a reconnect (in Claude Code: /mcp) starts a fresh
     server.
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
   that cargo sets when it runs us, else `cargo`), used by every place
   `grep -rn '"CARGO"' crates/cli/src` finds (eight today).
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
5. `cargo()` and its callers (eight today).
6. The `op` const in the output schemas; a test that no
   `*.output.json` contains another op's name as an `op` value; then
   regenerate, raise the version and add the line.
7. `docs/contract.md`, then the `document` skill.

## Acceptance criteria

- [x] `stale` with a newer server says the server is newer, and its test
      covers both directions.
- [x] No help line, page title or MCP tool description holds two spaces
      in a row (the test).
- [x] `check --target moon`, `bake --target moon` and `frame-png
      --target moon` print the same message with SH3003 and exit with
      the same code.
- [x] `std::env::var("CARGO")` and `var_os("CARGO")` appear once in
      `crates/cli/src`.
- [x] `run.output.json` and `session.step.output.json` differ in `op`,
      and adding an op to `Op` changes no other op's output schema.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `stale` both ways; the strings test; `target::profile` (known,
  unknown, profiles that don't load).
- Integration: `frame-png --target` with an unknown id through the
  binary; the contract's schema test (regenerated files and fingerprint).

## Completion notes

All five seams closed.

- `mcp::server::stale` says which side is newer. An older server: reconnect
  (in Claude Code: /mcp). A newer one: register the project's own `shuyi`
  (`<command> mcp install`). When a forwarding server's restart fails, the
  limit keeps only the diagnosis, beside "a fresh server of this checkout
  didn't start: …". Tests cover `VERSION + 1` and `VERSION - 1`.
- `lint`'s MCP description lost its run of spaces; two tests fail on a run
  of spaces or a line break in a codes-table title, help or page title and
  in any tool description.
- `contract::target::profile` is the one lookup; `check`, `bake` and
  `frame-png --target` use it, all with SH3003 and the same exit code.
  Deviation: it returns `Profile` (the profile with its file), since
  `check` reports the file.
- `crate::cargo()` is the one place that reads `CARGO` (nine uses in
  eight files).
- Each output schema's `op` is a `const` of its own op; `Op` is no longer
  in any output schema. Contract version 25 -> 26.
- `frame-png --target moon` is tested in-process on the toy
  (`tests/it/toy.rs`), not through the binary, which has no game program
  to run it on.

Breaking: shuyi-cli: contract: output schemas' `op` is a const of the op, not a reference to `Op`; version 26

