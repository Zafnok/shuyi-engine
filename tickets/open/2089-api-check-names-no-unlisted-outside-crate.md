---
id: "2089"
title: "cargo xtask api fails a public item that names an outside crate ADR-0119 doesn't list"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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
- Hiding any item: 2087 left none but `SoundProbeBackend`, which
  **2084 (done)** hid with `#[doc(hidden)]`, so the check passes on
  `shuyi-app` as it is, with no outside crate allowed for it.
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

- [ ] A faked listing with `pub fn shuyi_app::x(gilrs::GamepadId)` fails
      the run, naming `shuyi-app`, the line and ADR-0119.
- [ ] A faked listing with `serde_core::de::Deserialize` in `shuyi-core`
      passes, and `serde_json::value::Value` passes in `shuyi-cli` and
      fails in `shuyi-ui`.
- [ ] `shuyi_assets::image::ImageTable` is not read as the `image` crate.
- [ ] `cargo xtask api` passes on `main`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the line filter on the cases above.
- Integration: a faked run through `Runner`, as `crates/xtask/src/api/tests.rs` does.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
