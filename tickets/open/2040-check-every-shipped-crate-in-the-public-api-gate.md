---
id: "2040"
title: "Check every shipped crate in the public API gate"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2040 — Check every shipped crate in the public API gate

## Context

The review of the latest 60 PRs in each repository on 2026-11-01 found
that `crates/xtask/src/api.rs`'s `API_CRATES` still names the original
seven crates. It omits `shuyi-playdate`, added by
[PR 197](https://github.com/Zafnok/shuyi-engine/pull/197), and
`shuyi-alloc-count`. Both are engine crates in `tools/layers.ron` and
the README, and games depend on their public items directly.

[PR 214](https://github.com/Zafnok/shuyi-engine/pull/214) added
`shuyi_playdate::stack_high_water` and stack configuration, yet its
automated label is `api: none`. A later removal from an omitted crate
can likewise miss the break and book checks of
[ADR-0038](../../docs/adr/0038-the-public-api-gate-a-listing-diff-and-semver-checks-on-every-pr-and-a-breaking-line-for-each-break.md).

The release-baseline check is open ticket 2028, **needed after this
ticket**: it must consume the corrected catalog rather than preserve a
second, incomplete one. Its blocker records that order.

## Nick input

None.

## Scope

**In:**
- Include all nine current engine crates in the PR API check. Keep one
  catalog, whether `API_CRATES` is retained with a parity test or made
  from workspace metadata. An engine workspace member named `shuyi` or
  `shuyi-*` cannot silently be left out; examples, the book and `xtask`
  remain outside this catalog. Do not filter on `publish`: the workspace
  is deliberately unpublished before its first release.
- An explicit, tested configuration for each added crate:
  `shuyi-playdate`'s SDK-free host surface, including `stack_high_water`
  and its exported macro, and `shuyi-alloc-count` with `global`, so that
  its feature-gated public module is covered too. Feed the same feature
  choice to listing and semver comparison, on both head and base.
- Preserve the existing handling of a crate absent from the base: it is
  added, not a failed base build. A missing configuration or a failed
  tool remains an error, never `api: none`.
- Keep the existing added/breaking label, `Breaking:` and `Book:` rules
  effective for the added crates. Document the checked configurations
  and their limits in the API command's rustdoc and `run-gates`.
- About 500–850 changed lines, with tests, in one PR. No new dependency.

**Out (do not do):**
- The crates.io baseline and version-selection command (2028), which
  uses this catalog after this ticket.
- A matrix of every feature combination or a real SDK/device API run;
  this closes the omitted-crate hole with the supported host surfaces.
- Reclassifying historical PRs, moving the game's engine pin, or
  weakening a semver finding to make the newly checked crates pass.

## Implementation steps

1. Read `api.rs`, `api/tests.rs`, `tools/layers.ron`, and the manifests
   of the two omitted crates. Check the host rustdoc listings before
   choosing the added crates' configurations.
2. Extend the catalog and command construction in `compare_crates`,
   `listing` and `semver`; add catalog parity against the engine members
   from Cargo metadata, using the existing JSON support.
3. Extend the fake `Runner` tests for both added crates, a new crate
   absent from the base, and feature arguments matching on both sides.
4. Show a real added-item and removed-item check on an added crate,
   reverting the demonstration edits; record the commands and results.
5. Update the command's docs and the shared `run-gates` source; sync its
   discovery loader with `scripts/sync-agent-skills.ps1`.

## Acceptance criteria

- [ ] The catalog includes all nine engine crates; a fixture that adds
      an engine workspace member without a catalog entry fails naming it.
- [ ] A change to the public host surface of `shuyi-playdate` or
      `shuyi-alloc-count` appears in the API listing and verdict.
- [ ] The allocator's `global` public items appear in its checked
      listing; listing and semver commands use the same configuration.
- [ ] A fake break in either added crate yields a breaking verdict and
      requires its `Breaking:` line; a public addition still requires
      book coverage or a `Book:` explanation.
- [ ] An added crate missing from the base is reported as added; a
      failed listing or semver command fails the check.
- [ ] The completion notes record the real demonstration checks, and
      no demonstration source edit remains.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: catalog parity, new-crate handling, both added crates' command
  configurations, additions, removals and tool failures through `Runner`.
- Regression: breaking/book rules applied to either previously omitted
  crate, including an allocator item behind `global`.
- Integration: the API command on real host listings as described above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
