---
name: run-gates
description: Run the local quality gates before pushing (fmt, clippy pedantic, tests, docs, cargo-deny, machete, the examples' web build, the crates' layers, the modules, the no-std build, the golden tables on wasm32, the skill pack's worked examples and a game's compile times; the ticket lint, typos, links, the docs map and the pinned Rust). Mutation testing runs in CI only. Use before every commit you intend to push, and whenever CI fails and you need to reproduce it.
---

# Run the local gates

Verify shared skill discovery before the other gates:

```powershell
pwsh -File scripts/sync-agent-skills.ps1 -Check
```

These mirror CI. Run them fast to slow, and stop at the first failure.

What holds for every game on the engine is the shipped skill
[`shuyi-gates`](../../../skills/shuyi-gates/SKILL.md): the gates a game
runs, what each finding of `lint` and the rules crate's fence means,
snapshots, a CI failure this machine doesn't reproduce, and why a gate is
never weakened to pass it. This skill holds only what is this
repository's: the engine's own crates and CI's jobs, the game at the next
pin, and Nick's machine.

## Environment facts

- Nick's machine uses the **GNU** Rust host toolchain
  (`x86_64-pc-windows-gnu`). **MSVC is not installed**; never add
  anything that needs it locally.
- Put MSYS2's `C:\msys64\mingw64\bin` on `PATH` before building.
- Install tools with `cargo install --locked <tool>`.

## The code gates (CI's `ci.yml`)

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --features shuyi-app/capture,hello-grid/capture --locked -- -D warnings   # pedantic: the workspace's lints
cargo test --workspace --locked --no-fail-fast   # without --no-fail-fast cargo stops at the first failing test binary
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo deny check
cargo machete
```

`cargo test` also checks the skill pack (`crates/cli/tests/it/skills.rs`).
To see its problems alone:

```bash
cargo shuyi check-skills skills --tickets tickets/open
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
cargo build -p shuyi-core -p shuyi-ui -p shuyi-platform -p shuyi-alloc-count --no-default-features --target thumbv7em-none-eabihf --locked
cargo build -p shuyi-assets --no-default-features --target thumbv7em-none-eabihf --locked
cargo build -p shuyi-ui --no-default-features --features raster --target thumbv7em-none-eabihf --locked
cargo build -p shuyi-platform --no-default-features --features capture --target thumbv7em-none-eabihf --locked
RUSTFLAGS="--cfg shuyi_playdate_sdk" cargo build -p shuyi-playdate --target thumbv7em-none-eabihf --locked
cargo build -p hello-grid --lib --no-default-features --features playdate --target thumbv7em-none-eabihf --locked
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

## The real window (CI's `render` job, CI only)

The `render` job (ADR-0052) runs the render card (`shuyi-app`'s
`render_card` example: every kind of thing a frame draws) and Hello,
Grid's plan in their windows under Xvfb with Mesa's llvmpipe, and
compares each capture with the raster. It needs Xvfb, so it runs in CI
only. On Linux (a dev container, a cloud
agent's), after `bash scripts/setup-linux.sh`:

```bash
sudo apt-get install -y --no-install-recommends xvfb libgl1-mesa-dri mesa-utils
cargo build -p shuyi-app --example render_card --features capture --locked
SHUYI_CAPTURE_OUT=target/capture/card LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a -s "-screen 0 1280x1024x24" target/debug/examples/render_card
cargo build -p hello-grid --features capture --locked
SHUYI_CAPTURE=examples/hello-grid/capture/frames.ron LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a -s "-screen 0 1280x1024x24" target/debug/hello-grid
```

It exits 0 when every capture passes, 1 when one fails, 2 when the plan
can't run; the pictures and `capture.txt` are under `target/capture/`.
On any desktop the card and the plan run in a real window, without
Xvfb: `cargo run -p shuyi-app --example render_card --features capture`,
and `SHUYI_CAPTURE=examples/hello-grid/capture/frames.ron cargo run -p
hello-grid --features capture`.

## The web build in a browser (CI's `web-render` job, CI only)

The `web-render` job (ticket 2019) opens every example's web build, Hello,
Grid's capture build and the render card in headless Chromium with
`tools/web-check/check.py`, and fails on a console error, a page error, a
failed request or a canvas of one colour. It needs Python 3 and
Playwright, so it isn't one of these gates. Its wasm32 clippy step is in
the `wasm` job:

```bash
cargo clippy --target wasm32-unknown-unknown -p shuyi-app --features capture --lib --examples --locked -- -D warnings
```

Running `check.py` by hand: `crates/cli/web/README.md`, *Checked in CI*.

## The sound parity gates (CI's `sound-parity` job, CI only)

On a pull request or a push that changes a sound path, the `sound-parity` job
records what the backend really emits, natively and in Chromium, and holds it to expected
signals and to its speed budget. It needs PulseAudio on Linux and takes minutes,
so it isn't one of these gates. Its own tests are `cargo test -p xtask
sound_parity`. Running it by hand, and what its cells mean:
[`tools/sound-parity/README.md`](../../../tools/sound-parity/README.md).

## Golden tables (CI's `determinism` job)

Golden tables (`shuyi_assets::golden`, ticket 2116) pin values a build
gives: a decoded asset's `state_hash` (`crates/assets/tests/golden/formats.ron`),
a record's state hash at each step (`examples/hello-grid/tests/golden/hello-grid.ron`).
Their tests have `golden` in their names. They run:

- natively in the `test` job, on x64 (Windows, Linux) and ARM64 (macOS);
- on wasm32 under wasmtime in the `determinism` job, with every unit and
  integration test of `shuyi-core`, `shuyi-assets`, `shuyi-ui` and
  `shuyi-platform`, and Hello, Grid's `golden` test.

A table's test that fails prints the rows that moved and the new table.
Bless it, which runs the golden tests, writes each table they print and
runs them again:

```bash
cargo xtask golden --bless
```

New rows are not a break. A changed or removed row is: add a `Breaking:`
line naming the table (`Breaking: shuyi-assets: formats.ron "lights:
lights": <what a game sees>`) to the ticket's completion notes. If you
didn't mean to move it, find what did.

The wasm32 run is not a local gate. It needs the target and wasmtime at
the version CI pins:

```bash
rustup target add wasm32-wasip1
cargo install --locked wasmtime-cli@49.0.3
```

Run it with the job's runner and two commands before pushing a change to
number code, hashing, a decoder or the pack (in Git Bash, from the
repository root):

```bash
export CARGO_TARGET_WASM32_WASIP1_RUNNER="wasmtime run --dir=$PWD::$PWD --env=CI=true --env=INSTA_UPDATE=no --env=INSTA_WORKSPACE_ROOT=$PWD"
cargo test --target wasm32-wasip1 --locked --no-fail-fast --lib --tests -p shuyi-core -p shuyi-assets -p shuyi-ui -p shuyi-platform -- --nocapture
cargo test --target wasm32-wasip1 --locked --no-fail-fast -p hello-grid --no-default-features --test golden -- --nocapture
```

On Windows only `-- golden` passes in the first: the snapshot tests open
the Windows paths compiled into them, which WASI can't. On Linux (or in
a Linux container) both pass whole. A test that needs threads or
unwinding, which WASI preview 1 lacks, gets `#[cfg_attr(target_os =
"wasi", ignore = "…")]` saying so; any other test that fails on wasm32 is
a determinism bug, never ignored.

A new example with a golden table adds its own step to the job: `cargo
test --target wasm32-wasip1 -p <example> --no-default-features --test
golden`, its window feature off, as Hello, Grid's is.

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

## The skill pack's worked examples (CI's `skills` job)

Each skill in `skills/` has a worked example, `example.ron`, that runs on
a game fresh from `shuyi new` (ADR-0043). CI runs them all, in three
shards. Locally, run the examples of the skills a change touches, or of
every skill when a change touches the contract, a problem code or a
template:

```bash
cargo xtask skill-examples shuyi-gates shuyi-test   # or no names for every one
```

The games are made in `target/skill-examples/<skill>`, and build in this
repository's target folder.

## A game's compile times (CI's `compile-times` job)

CI times a new `grid` game on Linux and Windows (ADR-0049): its clean
build, its rebuild after a one-line change in `main.rs` and the same for
`cargo check`, and counts a new `blank` game's crates. It fails when the
median rebuild is over `tools/compile-budget.ron`'s number for the
runner's OS, or the count is over its `dependencies`. A change that adds
a crate to a new game raises that number in the same PR, so each new
dependency is a choice. Locally, on a change to the templates' setup, the
profiles or the engine's dependencies (the numbers are this machine's,
not the budget's):

```bash
cargo xtask compile-times   # --json for JSON; the games are made in target/compile-times
```

A slow rebuild is never fixed by raising the budget: find what grew
(`cargo build --timings` in `target/compile-times/timed`) first.

## The repository's own checks (CI's `tickets` job)

They run on every PR, docs-only ones too:

```bash
cargo xtask ticket-lint --pr-branch t<NNNN>-<slug>   # the branch, as CI does; without it on main
cargo xtask changelog > /dev/null   # every done ticket's Breaking: block parses (ticket 2804)
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
It also warns on a clause in an open ticket, or in a doc under `docs/`
(not `docs/adr/` or `docs/ROADMAP.md`), that waits on a done one ("if
NNNN is done", "once NNNN lands", "either order with NNNN", "waits for
NNNN"; the forms are in `find_clauses`' doc comment); the PR closing a
ticket settles the clauses that name it (the `work-ticket` skill, §5).
On CI the warnings show on the PR. It fails a done ticket with a `- [ ]`
box under *Acceptance criteria*: tick it, or write `- [x] ~~…~~ (not
done: why)`. It fails a clause in an
open ticket that names another open ticket, not in its `blocked_by`,
whose text never names it back: write the other side there (the
`write-ticket` skill, *Check the dependency chain*).

## The engine's own input and text rules (CI's `test` job)

`check-keys` and `check-text` over the engine's code, as the root
`shuyi.ron` sets them (ticket 2006): no key named outside the key
pipeline, no engine screen's text written as a literal. Each finding has
a problem code, a file and a line. The `rules` limit in the answer is
expected: the engine's own layers are `cargo xtask check-layers`'.

```bash
cargo run -q -p shuyi-cli -- lint
```

A new file that names keys by design goes in `key_files` with the reason
beside it; never loosen the checks to pass.

## The public API (CI's `api` job)

Not a local gate: it needs a nightly and two tools. CI runs it on every
PR that changes a crate (ADR-0038): it shows what merging adds to the
public API of the seven crates a game uses, and fails a break that the
branch's ticket has no `Breaking:` line for (the `work-ticket` skill,
§5). It also fails a change to the public API that changes no file of
the book (`site/content/learn/`, `site/snippets/`) when the ticket's
notes have no `Book:` line saying why (ticket 2815). Before pushing a
change you think breaks, run it from the repository root:

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

The rule is `shuyi-gates`': read the new snapshot and decide whether the
change is meant. Here a failing snapshot test writes `*.snap.new`, and
`cargo insta accept` takes it once you have; `cargo insta review` is
interactive and won't work here.

## When CI fails but local passes

`shuyi-gates` has the general case. Here, also: CI's Windows runners
build with MSVC and Nick's machine with GNU, and a red `main` may be
`main`'s own break: check the latest `main` run before blaming your
branch.
