---
id: "2005"
title: "The first release: the engine's crates on crates.io, a version, a changelog, and the game on versions instead of a git revision"
type: infra
milestone: MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2002", "1129", "1133", "1134", "1132"]
nick_input: sign-off
completed:
---

# 2005 — The first release on crates.io

## Context

Plan B ([`docs/decisions.md`](../../docs/decisions.md), 2026-10-14):
carve, reach parity with the game, and publish with the AI tooling
([`docs/ROADMAP.md`](../../docs/ROADMAP.md), phase 2). The code came here
with the game's ticket 1131 (game#1131), which left publishing out. Until
this ticket, every crate is `publish = false` at version 0.1.0, and games
(the game first) depend on a git revision. Game ADR-0063 §8 says the game
moves to crates.io versions "later": this is that ticket.

A crate name on crates.io is first come, first served, and `shuyi-*` are
the names Nick chose (`docs/decisions.md`, *The name*).

## Nick input

**Sign-off:** before `cargo publish` runs, Nick says go. Publishing can't
be undone (a version can be yanked, never deleted), and it is the
engine's first public release. Nick's contribution policy (the game's
0059, *Still open* in `docs/decisions.md`) should be settled first,
because `CONTRIBUTING.md` says it is decided "before the engine's first
public release".

## Scope

**In:**
- Each crate's metadata for crates.io: `description`, `keywords`,
  `categories`, `readme`, `documentation`, and the two license files in
  each crate's package (`cargo package --list` shows them).
- A version policy as an ADR: semver, 0.x while the API moves, one
  version for all `shuyi-*` crates, and what a breaking change needs (a
  note in the changelog, a pin bump in the game).
- `CHANGELOG.md`, starting at the first release.
- A release workflow: on a tag `vX.Y.Z`, the gates, then `cargo publish`
  of each crate in dependency order (`shuyi-core`, `shuyi-assets`,
  `shuyi-ui`, `shuyi-app`, `shuyi-cli`), with a crates.io token as a
  secret.
- Reserving the names early is allowed (an empty 0.0.0 publish) if Nick
  wants them held before the MVP; ask in the sign-off.
- In the game repository: a ticket that moves the game from the git
  revision to the released versions (`shuyi-engine.rev` and `cargo xtask
  engine-pin` retire, or stay for the local checkout only).

**Out (do not do):**
- The MVP tickets themselves.
- `docs.rs` beyond what `cargo publish` gives.

## Implementation steps

1. Write the ADR and the changelog.
2. Fill in the metadata; `cargo package` for each crate must pass.
3. Write the release workflow; dry-run it with `cargo publish --dry-run`.
4. Ask Nick for the sign-off, then tag and publish.
5. Write the game's ticket for moving to versions.

## Acceptance criteria

- [ ] Every `shuyi-*` crate is on crates.io at the first version, with its
      README, licenses and docs.
- [ ] A tag publishes a release through CI.
- [ ] The game's follow-up ticket exists.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `cargo package` and `cargo publish --dry-run` for each crate, in CI.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
