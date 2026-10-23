---
name: bump-rust
description: Move the pinned Rust (rust-toolchain.toml in the root and the templates, and rust-version) to a new stable release, and fix the new lints it brings. Use on a Dependabot PR that moves a rust-toolchain.toml, when cargo xtask check-toolchain fails, or when asked to update Rust.
---

# Move the pinned Rust

The engine builds on one exact stable Rust, written in the root
`rust-toolchain.toml`. The templates' `rust-toolchain.toml` files and
every `rust-version` follow it, and `cargo xtask check-toolchain`
(`crates/xtask/src/toolchain.rs`, ticket 2012) fails when they don't.

## Who and when

- **Claude moves the pin.** It is a technical choice, so Nick isn't asked.
- **The pin follows each stable.** Dependabot's `rust-toolchain` update
  (`.github/dependabot.yml`) opens a PR once a stable is 14 days old, and
  a session finishes it before the next stable is out (six weeks). The
  new lints then come one release at a time.
- **A bump PR holds nothing else.**
- **The Dependabot PR is the ticket**
  ([ADR-0037](../../../docs/adr/0037-dependency-upkeep-dependabot-a-weekly-advisory-check-workflow-audits-and-bot-work-as-its-own-ticket.md)
  §5): no ticket file to write or move. Work it on the Dependabot branch
  with this skill, not `dependency-upkeep`.
- **On a new minor**, Dependabot's PR fails `check-toolchain` until
  `rust-version` moves (its rule 3). That is the reminder working, not a
  fault.

## The checklist

1. **Move every pin** on the Dependabot branch. Dependabot moved the
   channels. For a new minor (`1.98.x` to `1.99.0`), move `rust-version`
   in `[workspace.package]` of the root `Cargo.toml` and of every
   template's `Cargo.toml` that has that table. A point release (`1.98.1`
   to `1.98.2`) moves only the channels. `cargo xtask check-toolchain`
   passes.
2. **Fix the new lints.** Read the release's notes on
   `blog.rust-lang.org` and clippy's `CHANGELOG.md` for new and newly
   warn-by-default lints. Run clippy and `cargo fmt --all --check` on the
   workspace, and on a game made from each template, as CI's `templates`
   job does (`cargo fmt --all` at the root doesn't reach `templates/`):

   ```bash
   cargo run -p shuyi-cli --locked -- new <template> <folder> --engine-path .
   ```

   Fix every new warning in the code, in this PR. A lint that is wrong for
   one line gets a local `#[allow(clippy::…)]` with a comment saying why,
   as the code already does; never a crate-wide allow (`AGENTS.md`
   rule 6).
3. **Move the `api` job's nightly and tools** (ADR-0038 §3), the consts
   `NIGHTLY`, `SEMVER_CHECKS` and `PUBLIC_API` in
   `crates/xtask/src/api.rs` and their lines in `ci.yml` (a test keeps
   them equal):
   - **for a new minor**, `NIGHTLY` becomes the first nightly of that
     minor (an older one refuses the new `rust-version`) whose rustdoc
     JSON format the pinned `cargo-public-api` reads; if none does, move
     `PUBLIC_API` to the release that reads it. A nightly's version is
     in `https://static.rust-lang.org/dist/<date>/channel-rust-nightly.toml`
     (`[pkg.rustc]`), and "reads it" means `cargo xtask api` passes on
     it. Skipping this step leaves the `api` job failing on every PR
     with "rustc X.Y-nightly is not supported" (ticket 2026);
   - **when the `api` job fails to read the new stable's rustdoc JSON**,
     move `SEMVER_CHECKS` to the release that reads it.

   Run `cargo xtask api` once with the new pins.
4. **Run the `run-gates` skill on Windows with the GNU host and MSYS2**,
   as Nick's machine builds. CI's Windows leg builds with MSVC, so only
   this run checks that the GNU link still works. The no-std build and the
   web build are part of it. A session that can't run on the GNU host says
   so in the PR, and the PR waits for that run.
5. **Tell the game, for a new minor.** The PR body says: "The game needs
   Rust X.Y at its next pin bump." Write the game ticket that moves the
   game's `rust-toolchain.toml` (and its `rust-version`) in that pin bump,
   as the `work-ticket` skill's §6 says for any change the game must
   follow. Once a job builds the game against engine PRs (ticket 2014), it
   shows the game failing on its older Rust on this PR: that is expected,
   and the game ticket fixes it.
6. **One PR per folder?** If Dependabot opened one PR per folder rather
   than one grouped PR, finish the root's (move the templates' channels
   there too) and close the others with a comment naming it.
7. **Merge on today's main.** Right before merging, if main moved since
   the branch's last CI run, merge main into the branch and wait for CI
   again. A PR merged meanwhile brings code the new lints were never run
   on: main's ruleset doesn't require an up-to-date branch, and #83
   (Rust 1.99) merged 23 seconds after #84, whose tests clippy 1.99
   failed on main (ticket 2026).

## What `rust-version` promises

Until the first release's version ADR (ticket 2005) says otherwise,
`rust-version` equals the pin's `X.Y`, and `check-toolchain` enforces it.

If that ADR allows a lag, the PR that first sets `rust-version` below the
pin, in the same PR:

- adds an `msrv` CI job: a `cargo check --workspace --locked` on exactly
  that Rust, as Bevy's does;
- changes `check-toolchain`'s rule 3 from "equal" to "at or below".

## Don'ts

- No nightly for the workspace.
- No other work in the bump PR.
- No `[workspace.lints]` change to pass a new lint. Turning a lint off for
  the whole workspace is a ticket of its own that says why.
