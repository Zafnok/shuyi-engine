---
id: "2090"
title: "cargo xtask api's outside-crate check reads the crates an outside dependency's types come from"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-21
---

# 2090 — `cargo xtask api`'s outside-crate check reads the crates an outside dependency's types come from

## Context

Ticket 2089 made `cargo xtask api` fail a line of the head's public API
listing that names an outside crate
[ADR-0119](../../docs/adr/0119-the-outside-crates-the-public-api-may-name.md)
doesn't allow (`OUTSIDE_CRATES` in `crates/xtask/src/api.rs`). It looks
for the paths of each engine crate's **direct** normal dependencies
(`cargo metadata --no-deps`).

`cargo-public-api` names a type by the crate that defines it, not the one
that re-exports it. ADR-0119's own table found `miniquad`'s `Conf` and
`KeyCode` in `shuyi-app`'s listing, through `macroquad`: `miniquad` is no
direct dependency of `shuyi-app`, so a new public item naming
`miniquad::conf::Conf` again would pass the check. The same holds for any
crate an outside dependency re-exports (`image`'s `png`, `serde`'s
`serde_core`, which is allowed).

## Nick input

None.

## Scope

**In:**
- Each engine crate's outside paths are its normal dependencies and,
  through them, every crate of the resolved graph they reach (from
  `cargo metadata` without `--no-deps`, its `resolve`), but no engine
  crate and nothing reached only through one.
- A faked run naming `miniquad::conf::Conf` in `shuyi-app` fails, as 2089's
  `gilrs` case does.
- **Either order with 2096:** if 2096 is done, its exhaustive-type check
  reads the same head listing in `compare_crates`: keep the two checks
  and their errors apart; if not, nothing to do (2096 works around
  this one).

**Out (do not do):**
- Changing `OUTSIDE_CRATES` or ADR-0119: `serde_core` is allowed already.
- Checking the base's listing.

**Estimated size:** about 200 lines (code 70, tests 110, docs 20).

## Implementation steps

1. In `crates/xtask/src/api.rs`, read the head's `cargo metadata
   --format-version 1` (with dependencies) for `resolve.nodes`, and give
   each engine crate's `Package` the paths (`-` as `_`) of the crates its
   normal dependencies reach, not through an engine crate.
2. Keep `unlisted_outside` as it is; feed it the wider list.
3. The fake `cargo metadata` in `crates/xtask/src/api/tests.rs` gains a
   `resolve`; a test of `shuyi-app` reaching `miniquad` through
   `macroquad`.
4. One clause in the `run-gates` skill's `api` section: the check reads
   the crates the dependencies reach.

## Acceptance criteria

- [x] A faked listing with `pub fn shuyi_app::f() -> miniquad::conf::Conf`,
      `shuyi-app` depending on `macroquad` and `macroquad` on `miniquad`,
      fails the run naming `miniquad` and ADR-0119.
- [x] A crate reached only through an engine crate is not looked for in
      the crate that depends on the engine crate.
- [x] `cargo xtask api` passes on `main`.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the reached crates of a faked `resolve`.
- Integration: a faked run through `Runner`, as
  `crates/xtask/src/api/tests.rs` does.

## Completion notes

- `crates/xtask/src/api.rs`: `compare_crates` reads the head's `cargo
  metadata --format-version 1 --locked` (with dependencies) once, and
  `reached_outside` walks its `resolve.nodes` from each workspace member's
  normal dependencies, through their normal dependencies, stopping at an
  engine crate. Each reached crate is named by its library target's name
  (`-` as `_`). The paths join the direct dependencies' in `Package`'s
  `outside`, and `unlisted_outside` is unchanged. The base's metadata is
  still read with `--no-deps` only: its listing isn't checked.
- Tests: three unit tests of `reached_outside` (dependencies'
  dependencies, never through an engine crate, dev-dependencies left
  out; a crate reached both ways counts; a library named by its target),
  a faked run where `shuyi-app` names `miniquad::conf::Conf` through
  `macroquad` and fails naming `miniquad` and ADR-0119 while `gilrs`,
  reached only through `shuyi-ui`, isn't looked for, and an unreadable
  metadata with dependencies failing the run. The fake's
  `with_head_packages` answers the new call with no graph.
- Docs: the `run-gates` skill's `api` section and the module's doc say
  the check reads the crates the dependencies reach.
- `cargo xtask api --base origin/main` passes on `main`'s tip
  (`50472bbc`) with the wider check: no head listing names a reached
  crate outside ADR-0119's list.
- 2096 was not done: its clause there now says what holds (both checks
  read the head's listing in `compare_crates`, kept apart).
- No deviation from the steps.

No break: only `xtask` changed, which is no published crate.

Book: none, no public API changed (only `xtask`).

Nick input: none.
