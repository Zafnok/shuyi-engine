---
name: shuyi-gates
description: The checks a game on Shuyi Engine passes before every commit (fmt, clippy, the tests, check, check --target, check-keys and lint), what each finding means, and why none is ever weakened to pass. Use before committing or pushing a game's change, when its CI fails, or when `cargo shuyi lint` or the rules crate's fence reports a problem. Not for writing the tests themselves (shuyi-test) or for a bug found by playing (shuyi-debug-replay).
license: MIT OR Apache-2.0
metadata:
  version: "1.0"
---

# The gates

Tests are the review. A game made with `shuyi new` comes with its gates,
and its CI (`.github/workflows/ci.yml`) runs the same ones on every pull
request. Run them before every commit, fast to slow, and stop at the
first failure. A gate is never weakened to pass it: fix the code.

## When to use it

- Before every commit you will push.
- When the game's CI fails, to run the failing command here.
- When `lint`, clippy or a test names a problem you need to understand.

## When not to

- Writing a test or a snapshot: `shuyi-test`.
- A bug seen while playing, or a replay that differs: `shuyi-debug-replay`.

## Steps

1. **Start from `lint`.** One answer with every finding of the shape of
   the game's code, each with its problem code, file and line:

   ```bash
   cargo shuyi lint --json
   ```

   It runs `check-keys` (`SH2101`), `check-text` (`SH2102`) and the
   rules crate's fence: clippy's bans (`SH2103`, with the column) and
   the crates under the rules (`SH2104`, with the `cargo tree -i` that
   shows the way in). `cargo shuyi explain <code>` prints each code's
   page. A part `shuyi.ron` lacks (no `rules` package, say) is a limit,
   not a pass.

2. **Then the rest, in this order** (the game's `AGENTS.md` lists them):

   ```bash
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   cargo shuyi check
   cargo shuyi check --target all
   cargo shuyi check-keys
   ```

   `check` loads every file of the game through every validator: each
   error with its file, line and code. `check --target all` checks that
   the frame, its text and every screen's controls fit each target
   `shuyi.ron` lists.

3. **Fix each failure in the code**, then run that gate again, then the
   rest.

4. **Commit `Cargo.lock`**, and once it is there add `--locked` to the
   cargo commands in CI.

## The rules crate's fence

A game with rules (`shuyi new --template grid` makes one) keeps them in
`crates/rules`, pure and deterministic: the same seed and the same
commands always play the same game, which replays, bots and tests rest
on. Three guards turn that into errors, and `lint` reads them:

- `crates/rules/clippy.toml`: the types and functions the rules may not
  use, each with its reason;
- `#![cfg_attr(not(test), deny(clippy::float_arithmetic))]` at the top
  of `crates/rules/src/lib.rs`: no float sums;
- `crates/rules/tests/layers.rs`: no screen, device, tool or luck crate
  anywhere under the rules.

What each error means, and what to use instead:

| It says | Use instead |
| ------- | ----------- |
| `disallowed type std::collections::HashMap` (or `HashSet`) | `BTreeMap` (`BTreeSet`): their order is the same on every run |
| `disallowed type f32` (or `f64`), `floating-point arithmetic detected` | whole numbers: tiles, turns, hundredths |
| `disallowed method std::time::Instant::now` | count turns, or put the time in a `Command` |
| `rand::…`, `getrandom::fill` | the `SimRng` kept in the state |
| `std::fs::…`, `std::env::var` | the game crate reads the file and hands the rules its text; settings come in when the state is made |
| `std::thread::spawn` | one thread |
| `depends on shuyi-ui` (any screen, device or tool crate) | move the code that needs it into `crates/game` |

Fix a failure in the code, never by loosening `clippy.toml`, the
`deny`, or the test's list. A test of the rules that must use a float
writes `#[expect(clippy::disallowed_types, reason = "…")]` on that test.

## Snapshots

A test that compares text or a picture with a file kept in the repository
fails when they differ. Read the new one and decide whether the change
was meant. If it was, write the file again the way the test says; if not,
fix the code. Never write it again blindly.

## When CI fails and this machine passes

Most often an operating system's difference (paths, line endings), the
Windows toolchain (on Windows with the GNU toolchain the game links with
MSYS2's `gcc`: `cargo shuyi doctor` checks it), or a tool's version. Run
the exact command from the failing job's log.

## When it fails

Branch on each finding's `code`, never its message:

- `SH2101`: a key named in code or text: react to an action, and name
  keys through the player's keymap (`shuyi-input`).
- `SH2102`: a screen's text written in its code: put it in
  `files/en.ron` and ask for it by key with `ctx.text`.
- `SH2103`: the rules use what their fence bans (the table above).
- `SH2104`: a crate under the rules that may not be there.
- `SH1002`: `shuyi.ron` lacks the part the command needs.
- `SH3005`: the game's tool program didn't build: the answer ends with
  the compiler's output.

## Worked example

[`example.ron`](example.ron) runs on a new `grid` game: `lint` passes,
a `HashMap` in the rules fails it with `SH2103` at its file and line,
`explain` prints the code's page, a `BTreeMap` fixes it, and then every
gate passes.
