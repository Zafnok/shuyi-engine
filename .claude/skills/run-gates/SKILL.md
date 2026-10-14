---
name: run-gates
description: Run the local quality gates before pushing (ticket lint, typos and links now; fmt, clippy, tests, docs, the web build, cargo-deny and machete once the code arrives). Mutation testing runs in CI only. Use before every commit you intend to push, and whenever CI fails and you need to reproduce it.
---

# Run the local gates

These mirror CI. Run them fast to slow, and stop at the first failure.

## Environment facts

- Nick's machine uses the **GNU** Rust host toolchain
  (`x86_64-pc-windows-gnu`). **MSVC is not installed**; never add
  anything that needs it locally.
- Put MSYS2's `C:\msys64\mingw64\bin` on `PATH` before building.
- Install tools with `cargo install --locked <tool>`.

## Now (the repository holds Markdown only)

These work once ticket 2002 lands:

```bash
cargo xtask ticket-lint                 # add --pr-branch t<NNNN>-<slug> as CI does
cargo xtask check-links
typos
```

## Once the code arrives (game ticket 1131)

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast    # without --no-fail-fast cargo stops at the first failing test binary
cargo doc --workspace --no-deps          # with RUSTDOCFLAGS="-D warnings"
cargo build -p hello-grid --target wasm32-unknown-unknown   # the web build, through an example
cargo deny check
cargo machete
cargo xtask ticket-lint && cargo xtask check-links && typos
```

1131 and later tickets add their checks here, for example
`check-keys`, `check-text` and `check-skills`.

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
