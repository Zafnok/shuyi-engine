---
id: "2842"
title: Every entry-point type has a tested example, and docs.rs shows every public feature
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2842 — Every entry-point type has a tested example, and docs.rs shows every public feature

## Context

A Rust developer learns a crate from its crate root and its main types
on docs.rs. Bevy, macroquad, tokio and std put a runnable example on
each of them. Ours have exact prose and no usage. A blind-spot audit of
the engine (2026-11-23) found this, and this ticket checked it again:

- **No example on the types a game starts from.** `TurnRules`
  (`crates/core/src/turn.rs:27`), `BasicGame`
  (`crates/platform/src/basic.rs:46`) and the `shuyi` crate root
  (`crates/shuyi/src/lib.rs`, a bullet list) have no code fence at all.
  `TickSim` (`crates/core/src/tick.rs:187`) and `Harness`
  (`crates/platform/src/harness.rs:105`) have one in their module's `//!`
  doc, none on the item. `Screen` (`crates/ui/src/screen.rs:46`) has one
  on a method (line 125), none on the trait. `Ctx`
  (`crates/ui/src/ctx.rs:267`), the platform's `Frame`
  (`crates/platform/src/platform.rs:192`), `ActionSet`
  (`crates/assets/src/keymap/action_set.rs:41`), `run_game`
  (`crates/app/src/lib.rs:792`) and `Bundle`
  (`crates/assets/src/bundle.rs:527`) have none on the item either.
- **docs.rs would hide most of the agent-facing API.** docs.rs builds
  default features on `x86_64-unknown-linux-gnu`, offline. No crate has
  a `[package.metadata.docs.rs]` (`grep -rn "docs.rs\|docsrs" crates
  --include=Cargo.toml` finds nothing). So these would be missing:
  shuyi-platform's `harness`, `link`, `heap`, `capture` and `schema`;
  shuyi-ui's `raster`, `schema` and `toy`; shuyi-assets' `png`, `golden`
  and the format features (`jpeg` to `mp3`); shuyi-alloc-count's `global`; and the
  `shuyi` front door's `script` and `schema`. Nothing labels an item with
  the feature it needs.
- **`all-features = true` would fail.** shuyi-app's `in-house-sound`
  brings CPAL's `links = "alsa"` beside quad-snd's. That builds only
  through the root `Cargo.toml`'s `[patch.crates-io] alsa-sys`
  (ADR-0075), and a published crate carries no patch. 3079 (parked)
  removes quad-snd and with it the patch; until then the feature stays
  out of any docs.rs list.
- **The feature lists already exist.** `API_CRATES` in
  `crates/xtask/src/api.rs:110` holds, for each engine crate, the
  features the API gate checks (tickets 2040, 2041), with
  `in-house-sound` left out for this very reason. docs.rs should show
  what the API gate judges, so this ticket reads that one list.
- **Nothing imitates docs.rs.** The `docs` gate
  (`crates/xtask/src/gates.rs:350`) and CI's `docs` job run `cargo doc
  --workspace --no-deps` with default features on the pinned stable.
  `pages.yml:92` builds `/api/` the same way. So a broken intra-doc link
  in a feature-gated item is never seen.
- **The release leaves it out.** 2005's Out says "`docs.rs` beyond what
  `cargo publish` gives", and its book step adds each crate's docs.rs
  link beside its `/api/` link (2802, done). The links would point at
  pages missing half the API.

The milestone is After MVP, as the audit's tickets are. Nick decides in
3528 which of them join the MVP; if this one does, it becomes a blocker
of 2005 there.

## Nick input

None.

## Scope

**In:**
- A tested example on each item of a list of entry points, kept as data
  in `tools/doc-entry-points.ron`: `shuyi` (the crate root),
  `shuyi::prelude`, `run_game`, `BasicGame`, `Screen`, `Ctx`,
  `TurnRules`, `TickSim`, `Harness`, the platform's `Frame`, `ActionSet`
  and `Bundle`.
- A short whole game at the `shuyi` crate root (about 20 lines), adapted
  from `site/snippets/src/quick_start.rs`, with its files in a
  `Layer::table` so it needs no folder.
- `cargo xtask check-doc-examples`: it fails when an item of the list has
  no Rust code fence in its own doc comment.
- `[package.metadata.docs.rs]` in every published engine crate, with the
  features of its `API_CRATES` entry, `rustdoc-args = ["--cfg",
  "docsrs"]` and `targets = ["x86_64-unknown-linux-gnu"]`. A test holds
  each manifest's list equal to its entry.
- `#![cfg_attr(docsrs, feature(doc_cfg))]` in each crate root, so docs.rs
  labels each item with the feature it needs. `cfg(docsrs)` joins the
  `check-cfg` lists.
- `cargo xtask docs-rs`, and a CI job `docs-rs` that runs it: each crate
  documented as docs.rs would, offline, on the pinned nightly, with no
  `[patch.crates-io]`.
- `/api/` on the website built with the same feature lists (stable, so
  no labels).
- No public type is added. Only doc comments and manifests change in the
  engine crates.

**Out (do not do):**
- The README's code sample: 2843 makes the README a front door. It may
  reuse the crate root's game.
- Small standalone task programs (`examples/cookbook/`): 2786 writes
  them. This ticket writes rustdoc examples on the entry-point types.
- `#[doc(alias)]` and a plain-text API per crate: 3413 (done); domain
  preludes: 3438.
- Examples on items outside the list. The list grows in the ticket that
  adds a new entry point.
- Removing ticket numbers, game tickets and other repository process
  from rustdoc. 2034 removed the dead `docs/design` citations and the process
  outside `shuyi-ui`, and 3538 did `shuyi-ui` and the check; the wider
  cleanup is not this ticket's.
- Publishing. 2005 publishes; docs.rs builds on publish.
- Any change to the stable docs gate's own command, beyond adding the
  check to its steps.

**Either order with 2005:** if 2005 is done, the metadata reaches
docs.rs at the next release; after it, open each crate's docs.rs page,
check that the build passed and that `Harness` shows a `harness` label,
and write what you saw in the completion notes. If not, nothing to do:
2005's first publish carries the metadata.

**Either order with 3520:** if 3520 is done, its kit features (`bots`
in shuyi-core; `grid`, `dialogue`, `cinematic` in shuyi-ui; `dialogue`
in shuyi-platform; the four in `shuyi`) are already in `API_CRATES`, so
each `[package.metadata.docs.rs]` list reads them; if not, nothing to
do (3520 adds them to each docs.rs list with `API_CRATES`).

**Estimated size:** about 1,400 lines (code 450, tests 400, docs 550).
The docs are the examples themselves, about 25 lines each.

**Split:** past 900 lines, split off steps 6–9 (the docs.rs metadata,
the `docs-rs` command and job, and `/api/`) into a new ticket, and keep
the examples and their check here.

## Implementation steps

1. **The list.** Write `tools/doc-entry-points.ron`: for each entry, the
   crate, the item's path and the source file it is declared in. Start
   with the twelve in the Scope. A comment at its head says a new entry
   point joins the list in its own ticket.
2. **The check.** Add `crates/xtask/src/doc_examples.rs` and the command
   `cargo xtask check-doc-examples`. For each entry it finds the item's
   declaration in its file (`pub trait Name`, `pub struct Name`, `pub fn
   name`, `pub mod name`; the crate root's `//!` block for a crate). It
   reads the doc comment just above (`///` lines, or `#[doc = …]`). It
   passes a fence that is Rust: ```` ``` ````, ```` ```rust ````,
   ```` ```no_run ```` or ```` ```should_panic ````. It fails
   ```` ```ignore ````, ```` ```text ```` and no fence, each with the
   file and line. It also fails an entry whose declaration it can't
   find, so a rename can't drop an item silently. Add it to the `docs`
   gate's steps in `crates/xtask/src/gates.rs`.
3. **The crate root.** In `crates/shuyi/src/lib.rs`, add a `no_run`
   example: a `BasicGame` of one screen that draws a line of text and
   leaves on `UiAction::Cancel`, run by `run_game` from `main`. Its files
   are a `Layer::table` of the few files it needs. Under it, a second,
   tested example drives the same game one frame through `Harness`
   (shuyi's dev-dependency turns on `shuyi-platform/harness`). Keep both
   short; the rest of the crate doc stays as it is.
4. **The other examples.** One on each remaining entry, tested (not
   `no_run`) where it can run headless:
   - `TurnRules`: a counter game whose command adds one, applied and
     replayed, its state hash compared.
   - `TickSim`: the same counter, stepped with inputs.
   - `Screen` and `Ctx`: a screen that reads an action from `Ctx` and
     returns a `ScreenResult`.
   - `Harness`: start a game, press an action, read the scene.
   - `BasicGame`, `run_game`, `prelude`: the crate root's game in the
     shape each item shows (the builder calls; `main`; the one `use`).
   - The platform's `Frame`, `ActionSet` and `Bundle`: making one and
     the call a game makes on it (`Bundle::new` over a `Layer::table`
     and `Bundle::file`).
   An example on an item behind a feature compiles only with it: write
   it so it builds in `cargo test --workspace`, whose feature
   unification turns those features on today (check that it does; if a
   crate's doctests miss a feature, turn it on in that crate's
   dev-dependencies).
5. **The tests of the check.** In `doc_examples.rs`: an item with a Rust
   fence passes; `ignore`, `text` and no fence fail with the line; an
   entry whose item is gone fails; and the real list passes on the
   repository.
6. **The metadata.** In each published crate's `Cargo.toml` (every
   `API_CRATES` entry), add:
   ```toml
   [package.metadata.docs.rs]
   features = [ …the entry's Features::Only list… ]
   rustdoc-args = ["--cfg", "docsrs"]
   targets = ["x86_64-unknown-linux-gnu"]
   ```
   Add a test in `crates/xtask/src/api.rs` that reads each manifest and
   fails when its docs.rs list differs from its `API_CRATES` entry, or
   when an entry has no table.
7. **The labels.** Add `#![cfg_attr(docsrs, feature(doc_cfg))]` to each
   crate root. Check on the pinned `NIGHTLY` (`api.rs:264`) that this
   feature labels items by their `cfg` on its own (since 2025 it does;
   the old `doc_auto_cfg` was folded into it). If that nightly still
   needs `doc_auto_cfg`, use that. Add `'cfg(docsrs)'` to the
   `unexpected_cfgs` `check-cfg` list in the root `Cargo.toml` and in
   the four crates that repeat it (`alloc-count`, `app`, `playdate`,
   `sound-out`; a grep finds any other).
8. **The imitation.** Add `cargo xtask docs-rs` (`crates/xtask/src/docs_rs.rs`).
   It copies the workspace's manifests and sources to a scratch folder
   under `target/`, drops the root's `[patch.crates-io]`, runs `cargo
   fetch` once, then for each crate runs, offline, `cargo
   +<NIGHTLY> rustdoc -p <crate> --lib --features <its list>` with
   `RUSTDOCFLAGS="--cfg docsrs -D warnings"`. `shuyi-playdate` builds
   without `--cfg shuyi_playdate_sdk`, as the API gate does. A failure
   names the crate and rustdoc's first error. Add the CI job `docs-rs`
   in `.github/workflows/ci.yml`, like the `api` job: `rustup toolchain
   install` of the same nightly, `scripts/setup-linux.sh`, then the
   command; on the same path filter as `docs`. In `gates.rs`, add
   `gate("docs.rs", …).ci_only("needs a nightly")`, as the `api` gate
   is. If a system library docs.rs's build image lacks stops shuyi-app
   or shuyi there, write what you found in the completion notes, and
   leave that crate's docs.rs list as it is (default features still
   fail the same way).
9. **`/api/`.** Add `cargo xtask docs-rs --features-flag`, which prints
   one `--features` value for the whole workspace (`shuyi-core/toy,…`).
   `pages.yml:92` passes it to its `cargo doc`, so `/api/` shows what
   docs.rs shows.
10. **Docs.** The run-gates skill names the new check and the CI-only
    `docs.rs` gate. The contributor docs (as the docs map,
    `docs/README.md`, places a gate) say that a new public feature joins
    its crate's `API_CRATES` entry, which also sets its docs.rs list, and
    that a new entry point joins `tools/doc-entry-points.ron`.

## Acceptance criteria

- [ ] Each of the twelve items in `tools/doc-entry-points.ron` has a Rust
      code fence in its own doc comment, and `cargo xtask
      check-doc-examples` passes.
- [ ] Taking the fence off `TurnRules`, or changing it to ```` ```ignore ````,
      makes the check fail and name `crates/core/src/turn.rs` and the line.
- [ ] The `shuyi` crate root shows a whole game, and one example drives it
      through `Harness` in `cargo test -p shuyi`.
- [ ] Every example that is not `no_run` runs in `cargo test --workspace`.
- [ ] Each published crate's `Cargo.toml` has a `[package.metadata.docs.rs]`
      whose features equal its `API_CRATES` entry, and a test fails when
      one differs. No list holds `in-house-sound`.
- [ ] CI's `docs-rs` job documents every crate of `API_CRATES` with no
      `[patch.crates-io]`, offline, on the pinned nightly, with
      `-D warnings`, and passes.
- [ ] In the `docs-rs` job's output for shuyi-platform, `Harness` carries
      a label naming the `harness` feature.
- [ ] `/api/` shows `shuyi_platform::harness` and `shuyi::script`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `doc_examples.rs`: a Rust fence passes; `ignore`, `text`, no
  fence and a missing item each fail with the file and line; the real
  list passes. `api.rs`: each manifest's docs.rs list equals its
  `API_CRATES` entry.
- Doctests: every example of step 3 and step 4, run by `cargo test
  --workspace`.
- Integration: CI's `docs-rs` job over every crate.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
