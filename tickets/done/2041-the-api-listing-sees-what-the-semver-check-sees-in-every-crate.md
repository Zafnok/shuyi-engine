---
id: "2041"
title: "The API listing sees what the semver check sees, in every crate"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-06
---

# 2041 — The API listing sees what the semver check sees, in every crate

## Context

Ticket 2040 gave `shuyi-playdate` and `shuyi-alloc-count` one explicit
feature choice (`Features::Only`). The other seven entries of `API_CRATES`
in `crates/xtask/src/api.rs` stayed on `Features::ToolDefaults`. There the
two tools differ: `cargo-public-api` lists default features, while
`cargo-semver-checks` turns on every feature but those its heuristic skips.

So a new public item behind a non-default feature, such as a module of
`shuyi` or `shuyi-core`'s `schema`, adds no listing line. The PR is
labelled `api: none`, and the `Book:` rule never runs
([ADR-0038](../../docs/adr/0038-the-public-api-gate-a-listing-diff-and-semver-checks-on-every-pr-and-a-breaking-line-for-each-break.md)).

## Nick input

None.

## Scope

**In:**
- Choose `Features::Only` for each of the seven, from each crate's
  manifest and real host listing, and remove `ToolDefaults` if nothing
  uses it. A feature that can't build on the host's CI runner stays out,
  and the choice says why.
- Update `api.rs`'s module docs, `run-gates` and ADR-0038's *Every engine
  crate* note. Sync the skill loaders.

**Out (do not do):**
- A matrix of every feature combination.
- Reclassifying historical PRs, or weakening a semver finding.
- The release-baseline command (2028): it reads `Features` from the
  catalog, so it needs no change here.

## Implementation steps

1. Read each of the seven manifests' `[features]`, and list each crate
   with the features chosen. Check that they build together.
2. Set the entries in `API_CRATES`, and extend the fake `Runner` test
   `each_added_crate_has_one_configuration_on_both_sides_and_in_both_tools`.
3. Run a real addition behind a non-default feature through `cargo xtask
   api`, revert it, and record the result.
4. Update the docs named in the Scope.

## Acceptance criteria

- [x] Every entry in `API_CRATES` gives both tools the same features, or
      says why a feature is left out.
- [x] A public item added behind a chosen non-default feature changes the
      verdict from `none` to `added`.
- [x] The completion notes record the real run, and no demonstration edit
      remains.
- [x] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass. Run locally: those this
      change reaches (`xtask`'s fmt, clippy, tests and docs; the ticket
      lint, links, docs map, typos, layers, modules, toolchain, machete,
      skill sync, sound-parity give-up). The workspace-wide builds are CI's.

## Tests required

- Unit: each crate's listing and semver arguments, through the fake
  `Runner`.
- Integration: the real run in step 3.

## Completion notes

Every entry of `API_CRATES` is now `Features::Only`, and `ToolDefaults` is
gone. Each crate is checked with every feature its manifest has, which is
what the semver check's heuristic already turned on (no feature name it
skips), so the listing now sees the same items:

| Crate | Features |
| ----- | -------- |
| `shuyi-core` | `std`, `schema`, `toy` |
| `shuyi-assets` | `std`, `png`, `sample-actions`, `golden` |
| `shuyi-ui` | `std`, `debug-tools`, `storage-conformance`, `onebit-reference`, `raster`, `toy`, `schema` |
| `shuyi-platform` | `std`, `harness`, `link`, `heap`, `capture`, `schema` |
| `shuyi-app` | `link`, `capture`, `sound-probe` (not `in-house-sound`, ADR-0075, as 2040 chose) |
| `shuyi-cli` | `toy`, `jis` |
| `shuyi` | `std`, `app` (`app` builds `shuyi-app` without `in-house-sound`) |
| `shuyi-sound` | `std`, `wav`, `vorbis` |
| `shuyi-sound-out` | none (it has none) |
| `shuyi-playdate`, `shuyi-alloc-count` | as 2040 chose |

They build together: the real run below built and documented every one on
both sides.

Tests: `every_crate_has_one_configuration_on_both_sides_and_in_both_tools`
(the old `each_added_crate_…`, now over all eleven crates, with their
features spelled out in the test) checks both tools and both sides through
the fake `Runner`. `every_feature_of_each_crates_manifest_is_chosen_or_left_out`
reads each real manifest: a feature added to an engine crate later fails it
until its entry chooses it or the test's `LEFT_OUT` names it.

Deviations:

- The ticket said seven crates; `shuyi-sound-out` had joined since, so
  there were eight.
- Explicit features would have failed a PR that adds or removes a feature:
  `cargo public-api --features <new>` on a base without it. So `cargo
  metadata` now gives each side's declared features. A side builds with the
  chosen features its manifest has, and the semver check (one feature list
  for both sides) takes only those both sides have. An item behind a feature
  new to the head shows in the listing as an addition. Tested by
  `a_feature_new_in_the_head_is_listed_there_and_its_items_are_added`.
- `tools/sound-parity/footprint.ron`: the give-up edits of `api.rs` and its
  tests now match the new text. After a give-up, `shuyi-app` keeps `link`
  and `capture`, and `LEFT_OUT` empties. `cargo xtask sound-parity give-up`
  (the check, without `--apply`) passes.

The real run: `pub fn api_probe_2041()` behind `#[cfg(feature = "schema")]`
at the end of `crates/core/src/lib.rs`, then `cargo xtask api --verdict
<file>` against `6555a04d`. Exit 0, verdict `added`, one added line
(`pub fn shuyi_core::api_probe_2041()`) in `shuyi-core`, 0 elsewhere and no
semver finding. Under the old configuration, `cargo public-api -p shuyi-core
--simplified` (default features) printed no line for it, so the verdict was
`none`. With this ticket's flags it prints the line. The edit is reverted.

On this machine, two of the four runs failed before that one. In both,
`cargo-semver-checks` crashed (a panic in its repro hint, after the
baseline build of `shuyi-sound-out` failed). After the third, a
half-removed `target/api-base` was left behind. By hand, the same semver
check passed, and the fourth run, started from a clean tree, passed. The
cause wasn't found, and CI runs on Linux. If it comes back, it is worth a
ticket.

Docs: `api.rs`'s module docs, `run-gates` (loaders synced, unchanged) and
ADR-0038's *Every engine crate* note. 2028 has a **From 2041 (done)** note:
`semver_args` now takes both sides' features.

No break: only `xtask` changed, and it is never shipped.

Book: none, the gate is the repository's own CI tooling and no game calls it.

Nick input: none.
