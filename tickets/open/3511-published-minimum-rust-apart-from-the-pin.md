---
id: "3511"
title: "The published minimum Rust trails the pinned toolchain, and CI's msrv job builds on it"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3511 — The published minimum Rust trails the pinned toolchain, and CI's msrv job builds on it

## Context

The engine builds on one exact stable Rust, the pin in
`rust-toolchain.toml` (`channel = "1.100.0"`). The `rust-version` that
cargo shows to a game is the same number: `rust-version = "1.100"` in
`[workspace.package]` of the root `Cargo.toml` (line 8), and every
member takes it with `rust-version.workspace = true`.
`cargo xtask check-toolchain` (`crates/xtask/src/toolchain.rs`, ticket
2012) makes them equal. Its rule 3 fails a workspace `rust-version`
that is not the pin's `X.Y`, and the message for one below the pin
already says "a lower `rust-version` needs the `msrv` job first".
The `bump-rust` skill moves both on every new minor
(`.claude/skills/bump-rust/SKILL.md`, step 1 and *What `rust-version`
promises*).

So the minimum Rust a game needs is whatever Rust came out a few weeks
ago. Rust 1.100 came out around 2026-11-12, about ten days before this
ticket. A game whose Rust is a few weeks old, or comes from a Linux
distribution, or from another studio's CI image, gets "requires rustc
1.100" on its first build. After the first release, keeping the two
equal would raise the minimum Rust every six weeks. That is a lot of
churn for games, and Nick asked for few version bumps after the MVP
([`docs/decisions.md`](../../docs/decisions.md), 2026-11-14, "no run of
breaking versions after the MVP").

What the engine needs is far below the pin. The newest `rust-version`
any locked dependency declares is 1.89 (`nanomp3` 0.2.0; then `image`
0.25.10, `proptest` 1.12.0, `sha2` 0.11.1, `encoding_rs` 0.8.44 at
1.88), from `cargo metadata --locked`. Edition 2024 needs 1.85. Only the
engine's own code may need newer, and nothing checks that today: no CI
job builds on any Rust but the pin (no `msrv` in `.github/`).

The workspace uses `resolver = "3"`. With it, cargo prefers dependency
releases whose `rust-version` fits the Rust doing the build (Rust 1.84
and later). So a game on an older Rust gets an older engine release that
fits, instead of a failed build. A raise of the minimum Rust then costs
a game little.

Ticket 2005 (the first release) plans a *Rust version* section in its
version ADR. That section offers "the pin, or a stated lag", and it says
the PR that first lowers `rust-version` adds the `msrv` job and changes
check-toolchain to at-or-below. This ticket is that PR: it makes the
choice and builds it. The blind-spot audit of 2026-11-23 found the gap
(its item 83).

The game is not touched. It pins this repository by git revision
(`shuyi-engine.rev` = `50472bbcd3084555a61728a5d07518f720bd6514`) and
builds on its own `rust-toolchain.toml`, `channel = "1.100.0"` (read
with `gh api repos/Zafnok/visions-of-shuyi/contents/rust-toolchain.toml`).
Lowering the engine's `rust-version` only loosens what cargo asks of it.

## Nick input

None. Which Rust the engine promises is a technical choice, like the pin
itself (the `bump-rust` skill, *Who and when*).

## Scope

**In:**
- **The policy, in a short ADR** (the next free number in `docs/adr/`;
  the `write-adr` skill):
  1. the pin follows each stable, as now (Dependabot and `bump-rust`),
     for development and its lints;
  2. the root `rust-version` is the oldest Rust the published crates
     build on, checked by CI's `msrv` job. It is no further back than four
     releases (about six months) below the pin on the day it is set;
  3. a `bump-rust` PR leaves the root `rust-version` alone;
  4. it is raised only when a language feature, a std API or a
     dependency the engine needs requires it, to the lowest Rust that
     gives it. A raise never goes in a patch release, and it comes with a
     `Behaviour:` line (ADR-0108), not a `Breaking:` line. The ADR says
     why: resolver 3 picks an older engine release for an older Rust, so
     a raise refuses no game's build;
  5. the templates' `rust-version` stays equal to the pin, because a game
     from `shuyi new` builds on the pin its `rust-toolchain.toml` names.
- **The value.** Find the oldest Rust from pin minus 4 (today 1.96) up to
  the pin on which the `msrv` job's command passes. Set the root
  `rust-version` to it. Fix the engine's code where it uses something
  newer than that, as clippy's `incompatible_msrv` lint and the build
  report it (clippy reads `rust-version`). If going one release lower
  needs more than about 150 lines of changes, stop at the last release
  that didn't, and say so in the ADR.
- **The `msrv` CI job** in `.github/workflows/ci.yml`, run when crates
  change. It reads the Rust from the root `Cargo.toml`'s `rust-version`
  at run time, so there is no second copy of the number to drift.
  It installs that Rust with `rustup toolchain install <X.Y>
  --profile minimal` and runs `cargo +<X.Y> check --workspace --locked`
  with the clippy job's `--features` list. The job goes in `ci-result`'s
  `needs`, and gets a row in `crates/xtask/src/gates.rs`'s table:
  `ci_only`, saying it installs a second Rust.
- **check-toolchain's rules** (`crates/xtask/src/toolchain.rs`):
  - rule 3, for the root: `rust-version` at or below the pin's `X.Y`
    (above still fails, as now);
  - rule 3, for a template's `[workspace.package]`: still equal to the
    pin's `X.Y`;
  - a new rule: when the root's is below the pin, `ci.yml` has a job
    named `msrv`. Its message names the ADR;
  - the module's doc comment lists the rules as they now are.
- **The skills.** `bump-rust`: step 1 moves the templates'
  `rust-version` only. *Who and when* no longer calls a failing rule 3 the
  reminder. *What `rust-version` promises* points at the ADR. Step 3's
  note on the `api` job's nightly says it must be at least the root
  `rust-version`, not the new pin. `run-gates`: a short section on the
  `msrv` job (CI only), with the two commands that run it locally.
- **For games:** the oldest Rust the engine needs, written where the docs
  map (`docs/README.md`, the `document` skill) puts what a game needs to
  build.
- No public type changes (ADR-0039 does not apply). The API listing is
  unchanged, so CI's `api` job finds no break.
- **Either order with 2005:** if 2005 is done, its version ADR's *Rust
  version* section still says `rust-version` equals the pin. This
  ticket's ADR amends that section (an amends line naming it), and the
  completion notes carry a `Behaviour:` line for the lower
  `rust-version`. If not, nothing to do there: 2005's section links this
  ticket's ADR instead of offering the choice.

**Out (do not do):**
- Moving the pin, or changing how Dependabot proposes Rust releases.
- An MSRV build for wasm32, the no-std targets or the templates' games.
  The `msrv` job checks the host build. The other targets keep building
  on the pin in their own jobs.
- `cargo hack` or a check of every feature combination.
- Moving a game's Rust when it moves to a newer engine: 2523 does that
  (its step 5 reads the templates', which stay equal to the pin).
- The list of crates the release publishes, the exact pins between the
  engine's crates and the runbook for a half-finished publish: 3510.

**Estimated size:** about 800 lines (code 250, tests 200, docs 350),
counted from the steps below and doubled. The code includes up to about
150 lines of fixes for APIs newer than the chosen Rust.

## Implementation steps

1. Write the ADR (policy points 1 to 5 above). Link ADR-0108, the
   2026-11-14 decision, ticket 2012 and, if it is done, 2005's version ADR.
2. Find the value. For each Rust from the pin down to pin minus 4:
   `rustup toolchain install 1.NN --profile minimal`, then
   `cargo +1.NN check --workspace --locked --features
   shuyi-app/capture,hello-grid/capture`. Set
   `rust-version` in the root `Cargo.toml`'s `[workspace.package]` to the
   oldest that passes after the fixes. Run `cargo clippy --workspace
   --all-targets ... -- -D warnings` on the pin: `incompatible_msrv` now
   names each std API newer than the new `rust-version`. Fix each one.
3. `crates/xtask/src/toolchain.rs`: split `workspace_rust_version` so the
   root accepts at-or-below and a template still needs equal. Add the
   `msrv` job rule. It reads `.github/workflows/ci.yml` line by line for a
   `  msrv:` job key, as the file reads TOML now. Update the module docs.
4. `.github/workflows/ci.yml`: the `msrv` job (the `crates` output of
   `changes`, checkout with `persist-credentials: false`, the rust-cache
   and `scripts/setup-linux.sh` as the clippy job has them, then a step
   that reads `rust-version` from `Cargo.toml` into a variable through
   `env`, never through an inline `${{ }}`, so zizmor passes). Add it to
   `ci-result`'s `needs`.
5. `crates/xtask/src/gates.rs`: the `msrv` row, `ci_only`, filtered on
   crates as `api` is.
6. The skills: `bump-rust` and `run-gates`, as in Scope. Run `pwsh -File
   scripts/sync-agent-skills.ps1 -Check`.
7. The line for games, where the docs map says.

## Acceptance criteria

- [ ] The root `rust-version` is below the pin, or the ADR says which
      change stopped it at the pin. It is never more than four releases
      below the pin.
- [ ] CI's `msrv` job runs `cargo check --workspace --locked` on exactly
      the root `rust-version`, and passes.
- [ ] Editing the root `rust-version` changes the Rust the job installs,
      with no other edit.
- [ ] `cargo xtask check-toolchain` passes a root `rust-version` below the
      pin when `ci.yml` has an `msrv` job. It fails one when the job is
      missing, and it fails one above the pin.
- [ ] It still fails a template whose `rust-version` differs from the pin,
      above or below.
- [ ] `cargo clippy` on the pin reports no `incompatible_msrv` warning.
- [ ] The ADR states the five points of the policy, and `bump-rust` no
      longer moves the root `rust-version`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`toolchain.rs`): a root `rust-version` one below the pin with an
  `msrv` job passes; the same without the job fails with the message that
  names the ADR; one above the pin fails; a template one below the pin
  still fails (the existing `a_template_rust_version_below_the_pin_fails`
  stays). The test `the_root_rust_version_below_the_pin_names_the_msrv_job`
  becomes the two root cases. `the_real_repository_passes` passes on the
  new values.
- Unit (`gates.rs`): the existing test that ties `ci.yml`'s jobs to the
  gates table passes with the `msrv` row.
- Integration: CI's `msrv` job itself, on this PR.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
