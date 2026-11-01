---
id: "3084"
title: "Make the in-house sound backend the default, with quad-snd still selectable"
type: infra
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: blocked
blocked_by: ["2573", "3032", "2397", "2234", "2378"]
nick_input: none
completed:
---

# 3084 — The in-house sound backend becomes the default

## Context

Nick asked that the sound replacement stay easily reversible
([decision](../../docs/decisions.md)), so the new default and the removal of
quad-snd are separate steps
([ADR-0068](../../docs/adr/0068-the-sound-replacement-stays-reversible-at-every-stage.md)).
This one is the switch, and undoing it is one line. 3079 waits for the game to
have run on it.

## Nick input

None.

## Scope

**In:**
- Run the complete 2573 matrix for every required platform, with the new backend
  against quad-snd on the same corpus. Any mandatory cell not passed stops here.
- Add `in-house-sound` to `shuyi-app`'s default features, with a way to build on
  quad-snd that a game's manifest can name. Every migrated consumer follows.
- CI keeps recording quad-snd with every cell mandatory, beside the new default.
- Take the default-features half of 2573's fence out of
  `crates/xtask/src/sound_parity/tests.rs`, and keep the half that holds quad-snd.
- Say in the completion notes how to go back, and hand the game's pin move to the
  game (`engine-change`), naming what to listen for.

**Out (do not do):**
- Removing anything of quad-snd or macroquad's `audio` (3079).
- A runtime switch between backends (ADR-0068).

## Implementation steps

1. Run the matrix and record its revision.
2. Flip the default, and prove both builds pass in CI.
3. Write the handoff to the game.

## Acceptance criteria

- [ ] The complete matrix passes on the new backend, and quad-snd's still does.
- [ ] A build selecting quad-snd plays through it, proven by `cargo tree` and a recording.
- [ ] Reverting this PR alone returns every game to quad-snd with nothing else to change.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the `sound-parity` job on both backends.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
