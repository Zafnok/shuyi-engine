---
id: "2041"
title: "The API listing sees what the semver check sees, in every crate"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] Every entry in `API_CRATES` gives both tools the same features, or
      says why a feature is left out.
- [ ] A public item added behind a chosen non-default feature changes the
      verdict from `none` to `added`.
- [ ] The completion notes record the real run, and no demonstration edit
      remains.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each crate's listing and semver arguments, through the fake
  `Runner`.
- Integration: the real run in step 3.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
