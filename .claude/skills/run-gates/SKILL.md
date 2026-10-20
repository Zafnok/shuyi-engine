---
name: run-gates
description: Run the local quality gates before pushing (fmt, clippy pedantic, tests, docs, cargo-deny, machete, the examples' web build, the platform crate's dependencies and the no-std build; the ticket lint, typos and links). Mutation testing runs in CI only. Use before every commit you intend to push, and whenever CI fails and you need to reproduce it.
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

The platform boundary depends on no device crate (CI's `platform-deps`
job): this prints nothing.

```bash
cargo tree -p shuyi-platform --edges normal,build --target all --all-features --prefix none --format '{p}' --locked \
  | grep -E '^(macroquad|miniquad|quad-[a-z0-9-]+|gilrs|gilrs-core|cpal|rodio|kira|alsa|alsa-sys|coreaudio-rs|coreaudio-sys|oboe|symphonia|lewton|hound) '
```

The crates a Playdate game builds without `std` (CI's `no-std` job; it
needs `rustup target add thumbv7em-none-eabihf`):

```bash
cargo build -p shuyi-core --no-default-features --target thumbv7em-none-eabihf
cargo build -p shuyi-assets --no-default-features --target thumbv7em-none-eabihf
cargo build -p shuyi-ui --no-default-features --target thumbv7em-none-eabihf
```

## The repository's own checks (CI's `tickets` job)

They run on every PR, docs-only ones too:

```bash
cargo xtask ticket-lint --pr-branch t<NNNN>-<slug>   # the branch, as CI does; without it on main
cargo xtask check-links   # relative links in every *.md resolve (no network)
typos                     # install with `cargo install --locked typos-cli`; words to allow go in _typos.toml
```

The ticket lint checks `game#NNNN` ids in `blocked_by` against the game
repository's tickets with `gh api`, or against a local checkout with
`--game-repo <path>`. With neither it warns and skips them; CI has no
access to the game repository, so check them locally when you add one.

Later tickets add their checks here, for example `check-keys` and
`check-text` on the engine's own code (2006) and `check-skills`.

## The game at the next pin

An API change here breaks the game when it moves its pin
(`shuyi-engine.rev` in the game repository). Before pushing one, build
the game against this working copy: in the game's checkout, `cargo
xtask engine` puts this repository in its `shuyi-engine/` folder, and
`cargo test --workspace` there builds the game on it.

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
