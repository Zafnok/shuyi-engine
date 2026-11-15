---
id: "2089"
title: "cargo xtask api fails a public item that names an outside crate ADR-0119 doesn't list"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-15
---

# 2089 — `cargo xtask api` fails a public item that names an outside crate ADR-0119 doesn't list

## Context

[ADR-0119](../../docs/adr/0119-the-outside-crates-the-public-api-may-name.md)
(ticket 2087) lists the outside crates the engine's public API may name:
`serde` everywhere, `schemars` behind `schema`, and `serde_json` and
`semver` in `shuyi-cli` alone. Any other outside type stays behind an
engine type or a `#[doc(hidden)]` path, so a new major version of the
crate, or replacing it (macroquad by 3034's backend, `include_dir` by
2215), is never a breaking release of ours (Nick, 2026-11-14,
`docs/decisions.md`).

2087 checked it once, by grepping the listing `cargo xtask api` makes
(`cargo-public-api --simplified`, ADR-0038) for every dependency's crate
name. Nothing checks it on each PR: a new public signature that names
`gilrs::GamepadId` or `image::RgbaImage` would pass CI, and ship.

## Nick input

None.

## Scope

**In:**
- The allowed outside crates as data in `crates/xtask/src/api.rs`: each
  crate path (`serde_core`, `serde`, `schemars`, `serde_json`, `semver`)
  with the engine crates that may name it, and a comment naming
  ADR-0119.
- In `cargo xtask api`, on the head's listing of each crate of
  `API_CRATES`: every line that names a path of an outside crate (a
  normal dependency of that crate, from `cargo metadata`, `-` as `_`)
  not allowed for that crate is a finding. The run fails on one, naming
  the crate, the line and ADR-0119.
- A test of the check on a faked listing, through the `Runner` the
  other tests fake.

**Out (do not do):**
- Hiding any item: 2087 left none but `SoundProbeBackend`, which 2084
  hides. **Either order with 2084:** if 2084 is done, the check passes
  on `shuyi-app` as it is; if not, allow `quad_snd` in `shuyi-app` with
  a comment naming 2084, and 2084 takes the line out.
- Checking the base's listing: only the head's lines count.

**Estimated size:** about 300 lines (code 120, tests 150, docs 30).

## Implementation steps

1. In `crates/xtask/src/api.rs`, a `const OUTSIDE_CRATES: &[(&str,
   &[&str])]` (crate path, the engine crates that may name it; `&[]` for
   every one).
2. A function of a crate's listing and its dependencies' names giving
   the lines that name a crate not allowed for it: a path is `<name>::`
   with no `::` or word character before it (so `shuyi_assets::image::`
   isn't the `image` crate).
3. Call it where the head's listing is made, add its findings to the
   summary, and fail the run on one, with the line and "ADR-0119 lists
   the outside crates the public API may name".
4. The `api` gate's section of the `run-gates` skill: one sentence on
   the check. ADR-0038 and ADR-0119 are accepted and stay as they are
   (ADR-0119's *Consequences* already names this ticket).

## Acceptance criteria

- [x] A faked listing with `pub fn shuyi_app::x(gilrs::GamepadId)` fails
      the run, naming `shuyi-app`, the line and ADR-0119.
- [x] A faked listing with `serde_core::de::Deserialize` in `shuyi-core`
      passes, and `serde_json::value::Value` passes in `shuyi-cli` and
      fails in `shuyi-ui`.
- [x] `shuyi_assets::image::ImageTable` is not read as the `image` crate.
- [x] `cargo xtask api` passes on `main`.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the line filter on the cases above.
- Integration: a faked run through `Runner`, as `crates/xtask/src/api/tests.rs` does.

## Completion notes

- `OUTSIDE_CRATES` in `crates/xtask/src/api.rs`: `serde_core`, `serde` and
  `schemars` for every crate, `serde_json` and `semver` for `shuyi-cli`,
  and `quad_snd` for `shuyi-app` with a comment naming 2084 (2084 isn't
  done: `SoundProbeBackend`'s three methods name `quad_snd::mixer::Playback`
  in `main`'s listing). 2084's step now says to take that line out.
- `cargo metadata --no-deps`, already read for the features, now gives each
  package its normal dependencies that are no engine crate, as paths (`-` as
  `_`, a rename's name when renamed). `unlisted_outside` finds the head's
  listing lines naming one `OUTSIDE_CRATES` doesn't allow in that crate; a
  path is `<name>::` with no `::` or word character before it.
- Each crate's lines are in `CrateApi::outside`; the run fails with one
  error per crate, naming the crate, each line, its crate path and "ADR-0119
  lists the outside crates the public API may name", and the summary has a
  `**Outside crate, fail:**` line for each. The base's listing isn't read.
- Tests: the line filter (the ticket's cases, a path anywhere on a line, a
  `my_image::`, a crate that is no dependency), the dependencies read from
  `cargo metadata` (dev-dependencies and engine crates left out, a crate of
  two `cfg` targets once), and two faked runs through `Runner`.
- `cargo xtask api --base origin/main` on the branch: `api: ok`, no
  outside-crate line, every crate 0 removed, 0 added.
- The `run-gates` skill's `api` section says what the check fails.
- Deviation: none from the steps. The check reads only an engine crate's
  direct dependencies, as the ticket says, so a crate an outside dependency
  re-exports isn't looked for: `miniquad` through `macroquad`, which
  ADR-0119's table found in `shuyi-app`. Follow-up: 2090 reads the crates
  the dependencies reach.

No break: `xtask` is no engine crate, and no public item changed.

Book: none, the check is a gate of the engine's own CI, not a thing a game uses.

Game: nothing to adopt, the change is to the engine's `xtask` only.
