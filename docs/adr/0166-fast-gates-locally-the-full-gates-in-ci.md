# ADR-0166: Fast gates locally, the full gates in CI

- **Status:** Accepted
- **Date:** 2026-11-27
- **Related tickets:** 3544 (this), 2054 (`cargo xtask gates` and its
  `--fast`), 2076 (the trim), 3545 (the trim after `--fast`)
- **Amends:** ADR-0121 §4 (the trim runs by itself after a full local
  run, which is now rare)

## Context

The `run-gates` skill told every session to run the full `cargo xtask
gates` before each push, and `AGENTS.md` rule 6, `work-ticket` and
`sonar-fix` repeated it. On Nick's machine (Windows, the GNU host, no
MSVC) a full run takes about two hours for one change: the workspace's
clippy and tests, the no-std build, the web build, the templates' and the
skill examples' games, the tutorial's tries, the compile times. CI runs
the same gates as parallel jobs on hosted runners on every PR, on Linux,
macOS and Windows. On 2026-11-27 a full local run on a one-line Sonar fix
was stopped partway, and Nick said the rule was wrong: fast gates
locally, the full suite in CI.

`cargo xtask gates --fast` runs the gates that take seconds (fmt, typos,
the ticket lint, the engine lint when Rust changed) and prints the same
summary block, naming the gates it left to CI. The pre-push hook already
runs it. What it misses most often in practice is a clippy finding or a
failing test in the crate just edited.

## Decision

1. **Before a push**, locally: `cargo xtask gates --fast`, plus
   `cargo clippy -p <crate> --all-targets --locked -- -D warnings` and
   `cargo test -p <crate> --locked` for each crate with a changed file.
   A diff with no crate (Markdown, tickets, skills) needs `--fast` only.
2. **The full `cargo xtask gates` is CI's.** A slow gate runs locally,
   alone, only to reproduce a CI failure on it, or where `run-gates`
   says a change needs it before the push: the wasm32 golden run for
   number code, hashing, a decoder or the pack; the API check for a
   change thought to break; the game at the next pin for a `Breaking:`
   line; and the full run on the GNU host for a Rust bump (`bump-rust`),
   which CI's MSVC Windows leg can't stand in for.
3. **The PR's body** carries the summary block of `--fast` on the pushed
   commit and the crates checked (`work-ticket` §6).
4. **The trim** (ADR-0121 §4) still runs after a passing full local run
   only. Until 3545 makes `--fast` trim too, `cargo xtask trim-targets`
   is run alone when the disk runs low.

`run-gates` holds the commands (*Before a push*); `AGENTS.md` rule 6
says it in one line.

## Consequences

- A push costs minutes, not hours, on Nick's machine, and a session
  doesn't hold a change for a run CI repeats anyway.
- More failures are first seen in CI: a gate across crates (the no-std
  build, the templates, the web build) breaks on the PR instead of
  locally. CI's run is the review (rule 6), so this costs a round trip,
  not a bad merge.
- A crate's dependents are not tested locally; a change to a widely used
  crate can fail in another crate's tests in CI.
- The trim runs by itself rarely until 3545.

## Alternatives considered

- **Keep the full local run** — two hours a change, repeated in CI.
- **Nothing locally beyond the pre-push hook** — the clippy and test
  failures of the edited crate are cheap to catch in minutes and the
  commonest cause of a red PR.
- **Test the touched crates' dependents too** — for `shuyi-core` that
  is most of the workspace, close to the full run again.
