# ADR-0049: Fast compiles: a new game's rebuild timed in CI against a budget, no debug info in `dev`, and no dynamic engine

- **Status:** Accepted
- **Date:** 2026-10-27
- **Related tickets:** 2009; 2905 and 2907 (they time builds with its
  tool), 2512 (a second budget under `tools/`)

## Context

Nick wants every feature on Bevy's homepage, "fast compile" among them
(`docs/decisions.md`, 2026-10-20). Bevy's homepage says a change
rebuilds in 0.8 to 3.0 seconds with its setup: the engine as one dylib
(`dynamic_linking`), a fast linker, and on nightly Cranelift and shared
generics. What a game's developer waits for is the rebuild after a
one-line change in their own game, and nothing measured it.

Ticket 2009 measured a new `grid` game (`shuyi new`) on CI's runners
(GitHub's 4-core `ubuntu-latest`, and `windows-latest` with the MSVC
host) and on Nick's Windows machine (GNU host, 24 cores), on the pinned
stable Rust (1.99.1). Each figure is the median of three rebuilds after
a line is appended to `crates/game/src/main.rs`, in seconds. The
runners' speed varies by up to half between runs, so the options that
looked like they helped were timed three times each:

| Option | Linux | Windows (MSVC) | Windows (GNU) |
| ------ | ----- | -------------- | ------------- |
| The template as it was (`debug = "line-tables-only"`) | 1.09, 1.19, 1.47, 1.52, 1.36 | 2.75, 3.25, 2.63, 2.64, 2.61 | 1.32, 1.27 |
| `debug = false` in `[profile.dev]` | 1.04, 1.04, 0.72, 0.86 | 1.95, 1.89, 1.94, 1.79 | 1.12 |
| Dependencies at `opt-level = 1` | 1.28, 0.89, 1.17, 1.05 | 1.91, 3.01, 2.32, 1.37 | 1.14 |
| Both of the above | 0.76, 1.05, 1.11 | 1.31, 1.86, 1.94 | |
| The engine as one Rust `dylib` (`shuyi-dylib`, Bevy's way) | 1.15 | 2.84 | 1.70 |
| `mold` (`-fuse-ld=mold`) | 1.33 | | |
| ld.bfd, not rustup's `rust-lld` (`-Clinker-features=-lld`) | 1.80 | | |
| `rust-lld` | (the default) | 2.75 | fails to link |
| `split-debuginfo = "unpacked"` | 1.58 | | |
| `incremental = false` | 1.33 | 3.09 | 2.07 |

What they show:

- With `opt-level = 1` the rebuild only follows the runner's speed (its
  ratio to the clean build stays the same), and the game would run
  slower; it doesn't help.
- `debug = false` takes about a third off on both runners and a sixth
  on Windows GNU, at the same runner speed. Without debug info cargo
  also strips std's (`strip = "debuginfo"` is its default then), and on
  MSVC no PDB is written: the link does less.
- Rust has linked with its own `rust-lld` by default on
  `x86_64-unknown-linux-gnu` since 1.90; `mold` is no faster. On
  Windows GNU `rust-lld` can't read the import libraries of
  `winapi-x86_64-pc-windows-gnu` ("unknown file type"), and through
  MSYS2's `gcc` it needs `-B` to the toolchain's own folder, which no
  template can know. On MSVC it is no faster than `link.exe`.
- The dylib built and linked on both Windows toolchains, under the
  65 535-symbol export limit, but made no rebuild faster, and Windows
  GNU's slower: the engine's crates are few and already built, so
  linking them statically costs little, and linking against a large
  DLL costs ld.bfd more.

## Decision

1. **`cargo xtask compile-times`** (`crates/xtask/src/compile_times.rs`)
   makes a `grid` game and a `blank` game with `shuyi new` on the
   checkout, in `target/compile-times/` (or `--dir`), each with the
   engine's own `Cargo.lock`. It builds the `grid` game in its own
   target folder, with the template's own setup and none of the
   caller's `RUSTFLAGS`, and reads cargo's `Finished … in` times: the
   clean `cargo build`, three rebuilds after a line appended to
   `main.rs` and their median, and the same for `cargo check`. It counts
   the `blank` game's dependency tree as `cargo metadata`'s resolve
   without the workspace's own crates, for every platform. It prints a
   table, or JSON with `--json`, and appends the table to `--summary`.
2. **The gate.** CI's `compile-times` job runs it on `ubuntu-latest` and
   `windows-latest` with `--budget tools/compile-budget.ron`. It fails
   when the median rebuild is over the budget's seconds for the
   runner's OS (the slowest measured plus 50 % for noise), or when the
   count is over its `dependencies`. A count under the budget warns, so
   it can come down. A change that adds a crate to a new game raises the
   number in the same PR, so each new dependency is a choice.
3. **The template's setup.** `templates/shuyi-default/base/Cargo.toml`
   builds `dev` (and so `test`) with `debug = false`, and keeps
   dependencies at `opt-level = 2`, so the game runs at speed. Its
   comment says how to get lines in backtraces, or a debugger, back. A
   panic's own message keeps its `file:line:column`: that comes from
   `Location`, not from debug info. No linker is written into
   `.cargo/config.toml`, so no Linux package joins `shuyi doctor`'s
   table or `setup-linux.sh`.
4. **The engine's repository keeps line tables** in its own
   `[profile.dev]`: its test failures are read from CI's logs, where a
   backtrace's lines matter more than a second a rebuild.
5. **No `dynamic` feature** and no `shuyi-dylib` crate. Nightly's
   Cranelift and `-Zshare-generics` are named in `README.md`'s *Fast
   compiles* for a developer to try; the repository stays on stable.

## Consequences

- A game made with `shuyi new` rebuilds in about a second on Linux and
  two on Windows, inside Bevy's range, and CI keeps it there.
- A new dependency of a game shows in a PR as a change to the budget.
- The job builds two games from clean on two runners, a few minutes of
  CI per PR. The numbers are the runners', not a developer's machine.
- A game's backtraces name functions, not lines, until its developer
  turns line tables back on.
- The gate can flake on a much slower runner; the headroom is half
  again the slowest run measured, and a real slowdown is found with
  `cargo build --timings`, not budgeted.

## Alternatives considered

- **Time a copy of `examples/hello-grid`**, as the ticket first said —
  it lives in the engine's workspace, with the engine's profiles and
  `.cargo/config.toml`, so a copy would need its manifest rewritten into
  a workspace that is no game's, and its numbers wouldn't follow the
  templates' setup. A `shuyi new` game is what a developer has.
- **Wall-clock time around cargo** — includes cargo's start and
  resolve; cargo's own `Finished … in` is what the developer reads, and
  it parses the same on every OS.
- **A fresh resolve for the count** — a crate's new release would move
  the count, and fail the gate, with no change here.
- **`debug = "line-tables-only"` kept in the template** — a third of
  every rebuild, for lines in backtraces that a panic's message already
  gives for its own place.
- **The `dynamic` feature, off on Windows** — it helped on no platform
  measured, and a dylib needs the toolchain's `std` on `PATH` to run
  outside `cargo run`.
- **`mold` on Linux** — no faster than rustup's `rust-lld`, and a
  package outside rustup for `setup-linux.sh` and `doctor`.
