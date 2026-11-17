---
id: "2091"
title: "Giving up the sound replacement after the first release keeps the in-house-sound feature, doing nothing"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-15
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

- [x] After `cargo xtask sound-parity give-up --apply` in a scratch
      checkout, `crates/app/Cargo.toml` has `in-house-sound = []`, and
      `cargo check -p shuyi-app --features in-house-sound` builds.
- [x] CI's `sound-give-up` job passes on the PR.
- [x] ADR-0068 and the skill say the feature is kept.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- The footprint's existing test (every edit still matches) passes.
- CI's `sound-give-up` job, which really gives up and builds what is left.

## Completion notes

- `tools/sound-parity/footprint.ron`: the `Replace` of the feature in
  `crates/app/Cargo.toml` now leaves `in-house-sound = []` under a comment
  that it does nothing and is kept so a game that turns it on still builds.
  The comment itself doesn't name the feature, so the kept line is the only
  one that does.
- CI's `sound-give-up` job: its last step greps lines, not files, and lets
  through exactly the lines `give_up::KEPT` names.
- The give-up skill and ADR-0068 §8 say the name is kept, with no
  `Breaking:` line for it.
- Deviation: the ticket's steps missed that `cargo xtask sound-parity
  give-up` refuses an edit that leaves any word of the chain behind (its
  `uncovered`), so the dry check failed on the kept line. `give_up::KEPT`
  names the kept lines (file and text), `uncovered` skips them, and a test
  (`only_the_kept_lines_may_stay`) checks that only such a line, in its
  file, passes.
- Deviation: the ticket missed the API check's feature lists too. The
  footprint took the feature out of them, so after giving up `cargo test -p
  xtask` failed (its manifest still has the feature; CI's first run of
  `sound-give-up` on this PR). Now giving up leaves the feature in
  `shuyi-app`'s `Features::Only` list in `crates/xtask/src/api.rs` and its
  test's copy: empty, it builds anywhere, and the semver check then holds
  its name. So three lines stay, not the ticket's one, and CI's grep allows
  those three exactly.
- Checked in a scratch worktree: `give-up --apply`, then `cargo check
  --workspace --all-targets`, `cargo check -p shuyi-app --features
  in-house-sound` and `cargo test -p shuyi-app -p xtask` passed, and CI's
  new grep found nothing.
- No break: the feature's name and the API are unchanged; only what giving
  up would do changes. The game names `in-house-sound` only in its ticket
  1241 and its docs (`gh search code --repo Zafnok/visions-of-shuyi
  'in-house-sound'`), and that ticket plans to turn it on, which this keeps
  safe.

Game: nothing to adopt, nothing in the engine's build changes until Nick gives up.
Book: none, the book doesn't describe giving up the sound replacement.
