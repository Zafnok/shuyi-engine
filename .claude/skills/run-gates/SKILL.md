---
name: run-gates
description: Run the local quality gates before pushing (fmt, clippy pedantic, tests, docs, cargo-deny, machete, the examples' web build, the crates' layers, the modules and the no-std build; the ticket lint, typos, links, the docs map and the pinned Rust). Mutation testing runs in CI only. Use before every commit you intend to push, and whenever CI fails and you need to reproduce it.
---

# Run the local gates

These mirror CI. Run them fast to slow, and stop at the first failure.

## Environment facts

- Nick's machine uses the **GNU** Rust host toolchain
  (`x86_64-pc-windows-gnu`). **MSVC is not installed**; never add
  anything that needs it locally.
- Put MSYS2's `C:\msys64\mingw64\bin` on `PATH` before building.
- Install tools with `cargo install --locked <tool>`.

## The code gates (CI's `ci.yml`)

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings   # pedantic: the workspace's lints
cargo test --workspace --locked --no-fail-fast   # without --no-fail-fast cargo stops at the first failing test binary
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo deny check
cargo machete
```

The web build of the examples, from each example's folder (CI's `wasm`
job; the page goes to its `dist/web/`):

```bash
cd examples/hello-grid && cargo shuyi web --release
```

Each example that lists `targets` in its `shuyi.ron` fits them (ADR-0008
§1; CI's `wasm` job), from its folder:

```bash
cd examples/hello-grid && cargo shuyi check --target all
```

The crates' layers (CI's `layers` job, ADR-0032): which engine crate
depends on which, the outside crates of the pure ones, no device crate
below any crate but the backend's, and the float ban in the simulations,
as `tools/layers.ron` writes them. A new engine crate, engine edge or
outside crate of a pure crate adds its line there in the same PR.

```bash
cargo xtask check-layers
```

The modules (CI's `modules` job, ADR-0009): no default crate depends on
one; each has a README, its `[package.metadata.shuyi]` and a row in
`docs/modules.md`; its answers are in `docs/modules.ron`; alternatives
name each other; and each feature of the `shuyi` crate names a row. Then
the `shuyi` crate as the default engine, with each module's feature
alone (`cargo build -p shuyi --features <feature>`, for each), and with
every one:

```bash
cargo xtask check-modules
cargo build -p shuyi --locked
cargo test -p shuyi --all-features --locked
```

The crates a Playdate game builds without `std` (CI's `no-std` job,
ADR-0008 §5). Add the target once:

```bash
rustup target add thumbv7em-none-eabihf
```

It builds on Windows with the GNU host toolchain. A library build for
the target needs no linker for the target: only the build scripts and
proc macros are linked, for the host, by MSYS2's `gcc` as usual.

```bash
cargo build -p shuyi-core -p shuyi-ui -p shuyi-platform --no-default-features --target thumbv7em-none-eabihf --locked
cargo build -p shuyi-assets --no-default-features --target thumbv7em-none-eabihf --locked
cargo xtask check-no-std   # no `std::` path outside cfg(feature = "std") in those three
RUSTFLAGS="-D warnings -A unused-imports" cargo test -p shuyi-core -p shuyi-ui -p shuyi-platform --no-default-features --locked --no-fail-fast
```

The last runs their tests without `std` on the host, the headless
backend's included. The tests bring `std` in, and with it `std`'s float
methods, so the `Float` imports the `no_std` code needs are unused there
only; the target build denies every warning.

Code that needs `std` goes behind `cfg(feature = "std")`: on the line
with its `std::` path, or in a module whose file starts with
`#![cfg(feature = "std")]`. A test that needs `std` (an asset converter,
`catch_unwind`) gets `#[cfg(feature = "std")]` too.

## A new game from each template (CI's `templates` job)

`shuyi new` makes a game from each template, and the made game runs its
own gates. From the repository root, for `grid` and then `blank`:

```bash
cargo run -q -p shuyi-cli --locked -- new ../demo-grid --template grid --engine-path "$PWD"
cd ../demo-grid
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo shuyi check
cargo shuyi check --target all
cargo shuyi check-keys
cargo shuyi lint
```

`templates/` is not in the workspace, so `cargo fmt --all` here never
formats its files; the made game's `cargo fmt --all --check` is what
catches them (rustfmt's import order has failed CI this way).

Two more steps of the job run only in CI: that the rules crate's fence
holds, and that `shuyi lint` finds what breaks the game's code (both on the
`grid` game, with code written to break them).

## The repository's own checks (CI's `tickets` job)

They run on every PR, docs-only ones too:

```bash
cargo xtask ticket-lint --pr-branch t<NNNN>-<slug>   # the branch, as CI does; without it on main
cargo xtask check-links   # relative links in every *.md resolve (no network)
cargo xtask check-pins    # actions pinned by SHA, one SHA each, templates' workflows in dependabot.yml, templates' requirements = the root's
cargo xtask check-toolchain   # one pinned Rust: the templates' channels, rust-version, Dependabot (the bump-rust skill)
cargo xtask check-docs    # every *.md has a row in the docs map, docs/README.md
typos                     # install with `cargo install --locked typos-cli`; words to allow go in _typos.toml
```

The ticket lint checks `game#NNNN` ids in `blocked_by` against the game
repository's tickets with `gh api`, or against a local checkout with
`--game-repo <path>`. With neither it warns and skips them; CI has no
access to the game repository, so check them locally when you add one.
It also warns on a clause in an open ticket that waits on a done one
("if NNNN is done", "either order with NNNN", "waits for NNNN"); the PR
closing a ticket settles the clauses that name it (the `work-ticket`
skill, §5). On CI the warnings show on the PR. It fails a clause in an
open ticket that names another open ticket, not in its `blocked_by`,
whose text never names it back: write the other side there (the
`write-ticket` skill, *Check the dependency chain*).

Later tickets add their checks here, for example `check-keys` and
`check-text` on the engine's own code (2006) and `check-skills`.

## The public API (CI's `api` job)

Not a local gate: it needs a nightly and two tools. CI runs it on every
PR that changes a crate (ADR-0038): it shows what merging adds to the
public API of the seven crates a game uses, and fails a break that the
branch's ticket has no `Breaking:` line for (the `work-ticket` skill,
§5). Before pushing a change you think breaks, run it from the
repository root:

```bash
cargo xtask api --pr-branch t<NNNN>-<slug>   # against `git merge-base HEAD main`; without --pr-branch, no rule
```

Without a tool it prints the line that installs it (`cargo install
--locked …`, `rustup toolchain install nightly-… --profile minimal`).

## The game at the next pin

An API change here breaks the game when it moves its pin
(`shuyi-engine.rev` in the game repository). The ticket's `Breaking:`
lines tell the game's pin bump what changed and what to do. Before
pushing one, build the game against this working copy: in the game's
checkout, `cargo xtask engine` puts this repository in its
`shuyi-engine/` folder, and `cargo test --workspace` there builds the
game on it.

## Coverage and SonarCloud

Not a local gate. CI's `coverage` job runs `cargo llvm-cov` without
`shuyi-app` and the SonarCloud scan (ADR-0017). Findings come back as
`sonar` GitHub issues (the `sonar-fix` skill), not as a failing check.

## Mutation testing

Not a local gate: never run `cargo mutants` on Nick's machine. CI runs it
on every PR's diff, in shards, with nextest as the test runner. If CI
reports `MISSED` mutants in code you wrote, add an assertion that would
fail under each one. `#[mutants::skip]` is allowed only with a comment
explaining why the code is untestable, never to pass the gate.

## Snapshots (insta)

- A failing snapshot test writes `*.snap.new`. **Read the new snapshot**
  and decide whether the change is intended.
- If it is, run `cargo insta accept`. If not, fix the code. Never accept
  blindly; `cargo insta review` is interactive and won't work here.

## When CI fails but local passes

Most likely an OS difference (path separators, line endings), MSVC vs
GNU, or a tool version. Reproduce with the exact command from the
failing job's log. A red `main` may be `main`'s own break: check the
latest `main` run before blaming your branch.
