---
id: "2091"
title: "Giving up the sound replacement after the first release keeps the in-house-sound feature, doing nothing"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: in-progress
blocked_by: []
nick_input: none
completed:
---

# 2091 — Giving up the sound replacement keeps the `in-house-sound` feature, doing nothing

## Context

[ADR-0068](../../docs/adr/0068-the-sound-replacement-stays-reversible-at-every-stage.md)
§8 (ticket 2084) says `shuyi-app`'s `in-house-sound` feature stays for good:
a feature a game names that later goes away is a breaking release, and Nick
decided on 2026-11-14 that no run of breaking versions follows the MVP
([decision](../../docs/decisions.md#2026-11-14-no-run-of-breaking-versions-after-the-mvp)).

But giving up the replacement (§0: `cargo xtask sound-parity give-up --apply`,
the `give-up-sound-replacement` skill) takes the feature out with everything
else: `tools/sound-parity/footprint.ron` replaces the feature's lines in
`crates/app/Cargo.toml` with nothing, and CI's `sound-give-up` job
(`.github/workflows/ci.yml`) fails if any file outside the record still names
`in-house-sound`. Before the first release (2005) that costs nothing; after
it, a game that turned the feature on stops building.

## Nick input

None.

## Scope

**In:**
- The footprint's `Replace` of the `in-house-sound` feature in
  `crates/app/Cargo.toml` leaves the feature in place as
  `in-house-sound = []`, with a comment that it does nothing and is kept so a
  game that names it still builds (ADR-0068 §8).
- CI's `sound-give-up` job's last step allows that one line and its
  comment in `crates/app/Cargo.toml` (and nothing else naming
  `in-house-sound`).
- The `give-up-sound-replacement` skill says the feature is kept, and the
  completion notes of that session write no `Breaking:` line for it.
- ADR-0068's §8 gains a sentence that giving up keeps the name too.

**Out (do not do):**
- Running the give-up for real; it stays Nick's call.
- Any change to which backend is the default (3084) or to removing quad-snd
  (3079).

## Implementation steps

1. Change the footprint's `Replace` for the feature (its `with:`).
2. Change the CI step's `git grep` so the kept line passes.
3. The skill's line and ADR-0068's sentence.
4. Run `cargo xtask sound-parity give-up` (the dry check) and
   `cargo test -p xtask`.

## Acceptance criteria

- [ ] After `cargo xtask sound-parity give-up --apply` in a scratch
      checkout, `crates/app/Cargo.toml` has `in-house-sound = []`, and
      `cargo check -p shuyi-app --features in-house-sound` builds.
- [ ] CI's `sound-give-up` job passes on the PR.
- [ ] ADR-0068 and the skill say the feature is kept.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The footprint's existing test (every edit still matches) passes.
- CI's `sound-give-up` job, which really gives up and builds what is left.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
