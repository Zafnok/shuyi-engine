---
id: "3506"
title: The api check lists a proc-macro crate without a semver check, and a line it loses is a break
type: bug
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-22
---

# 3506 — The api check lists a proc-macro crate without a semver check, and a line it loses is a break

## Context

CI's `api` job (`cargo xtask api`, `crates/xtask/src/api.rs`, ADR-0038)
lists each crate of `API_CRATES` with `cargo-public-api` on the head and
the base, and runs `cargo-semver-checks` against the base on each crate
the base has. Ticket 2215 (#527, ADR-0138) added `shuyi-embed-macros`, a
proc-macro crate, to `API_CRATES`. `cargo-semver-checks` 0.51.0 refuses
it:

```text
error: no crates with library targets selected, nothing to semver-check
note: only library targets contain an API surface that can be checked for semver
```

`parse_semver` turns that into "`cargo semver-checks` failed on
shuyi-embed-macros without naming a break", rightly: a failed run that
names no break is never a pass. #527 passed only because the crate was
new there (the base had none, so no semver check ran). Since it merged,
every PR's `api` job fails: #528 (ticket 2397, job 137384529632) and
#529.

## Nick input

None.

## Scope

**In:**
- A list beside `API_CRATES` of the crates the semver check can't read,
  each with its reason. `shuyi-embed-macros`: "a proc-macro crate:
  cargo-semver-checks reads library targets only; its listing is what
  holds it".
- Such a crate is listed on both sides, as every crate is, and
  `cargo-semver-checks` is never run on it.
- The listing holds it: each line its listing loses is a finding of its
  own lint (`listing_line_removed`), so the label is `breaking` and the
  rule asks for a `Breaking:` line naming the crate, as a semver finding
  would. A removed or renamed macro is a break the semver tool would have
  called one. An added line is an addition, as for every crate.
- The summary's table says the crate's semver check was not run, and
  why.
- A test that reads the workspace's manifests: a crate is in the list if
  and only if its `[lib]` is `proc-macro = true`.
- The `run-gates` skill's api section says so.

**Out (do not do):**
- Loosening `parse_semver`: a failed semver run that names no break stays
  an error.
- Dropping the crate from `API_CRATES` or from the listing.
- A semver check of the proc-macro's expansion (what code a macro writes):
  `shuyi-embed`'s own listing and tests hold what a game can name.

**Estimated size:** about 150 lines (code 60, tests 70, docs 20).

## Implementation steps

1. In `crates/xtask/src/api.rs`, add `pub const UNCHECKED: &[(&str,
   &str)]`: a crate's name and why no semver check runs on it, with
   `shuyi-embed-macros` and the reason above.
2. In `compare_crates`, for a crate in `UNCHECKED` that the base has,
   list both sides and skip `semver`; its findings are one
   `Finding { lint: "listing_line_removed", items: <the removed lines> }`
   when it lost lines, none otherwise.
3. Give `CrateApi` an `unchecked: Option<&'static str>` (the reason), and
   make `summary`'s semver column read `not run: <reason>` for it.
4. Update the module docs and the `run-gates` skill's api section.

## Acceptance criteria

- [x] `cargo xtask api` on a branch off `main` runs no
      `cargo semver-checks` on `shuyi-embed-macros`, lists it on both
      sides, and passes.
- [x] A removed line of an unchecked crate is a `breaking` label and
      needs its `Breaking:` line; an added line is `added`.
- [x] A test fails if a proc-macro crate is missing from `UNCHECKED`, or
      if it names a crate that is no proc-macro crate.
- [x] CI's `api` job passes on this PR.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a run over a fake whose base has the whole catalog runs no
  semver check on an unchecked crate and lists it in the head and the
  base; a lost line of such a crate is a `listing_line_removed` finding,
  and the summary's row says "not run".
- Unit: `UNCHECKED` is exactly the workspace's proc-macro crates.

## Completion notes

What was done:

- `crates/xtask/src/api.rs`: `UNCHECKED` lists the crates
  `cargo-semver-checks` can't read, each with its reason, today
  `shuyi-embed-macros`. `compare_crates` lists such a crate on both sides
  and never runs the semver check on it; `unchecked_findings` makes each
  line its listing lost one `listing_line_removed` finding, so the label
  is `breaking` and the rule asks for a `Breaking:` line naming it. The
  summary's semver column reads `<n>, not run: <reason>` for it.
  `parse_semver` is unchanged: a failed run that names no break is still
  an error.
- Tests in `crates/xtask/src/api/tests.rs`: `UNCHECKED` is exactly the
  workspace's `proc-macro = true` crates; a run lists the unchecked crate
  in the head and the base, runs no `check-release` on it, and makes its
  lost line a break whose `Breaking:` line the rule asks for; an added
  line is an addition. The test that every crate has one configuration
  now asserts no semver command for an unchecked crate (it failed before
  the fix).
- The module's docs and the `run-gates` skill's api section say so.

This blocked every PR's `api` job since #527 (ticket 2215): #528 (ticket
2397, job 137384529632) and #529 failed with "`cargo semver-checks`
failed on shuyi-embed-macros without naming a break".

Deviations: none from this ticket. Against the brief that started it
(no findings for such a crate, only the lost-lines warning), a lost line
is a break: a warning would let a removed or renamed macro reach the
game with no `Breaking:` line, where every other crate's removal is held
by the semver check. No ADR: this is how ADR-0038's gate treats a target
its semver tool can't read, written in `api.rs`'s docs and the skill.

Book: none, no public API of an engine crate changes (only `xtask`).

No break, no `Behaviour:` line: `xtask` is not shipped, and the game
runs its own api check, not this one.
