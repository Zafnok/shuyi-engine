# ADR-0068: The sound replacement stays reversible at every stage

- **Status:** Accepted
- **Date:** 2026-10-31
- **Related tickets:** 2573, 2210, 3030, 3031, 3032, 2397, 3084, 3079, 3034

## Context

[ADR-0066](0066-sound-uses-library-codecs-and-output-with-an-owned-mixer-and-parity-gates.md)
keeps quad-snd beside each new stage, but said nothing of going back, and 3079
made the new backend the default and deleted the old one in one step. Nick asked
that the whole chain be easily reversible if a hurdle can't be climbed
([decision](../decisions.md)): he may decide, at any stage and for any reason, to
scrap all of it in one session, until quad-snd is removed. Reverting each stage's
PR wouldn't do: other work lands between them, so the reverts would conflict, and
a session would spend itself untangling them.

## Decision

0. **Giving up is one command, Nick's to call.**
   `cargo xtask sound-parity give-up --apply` takes out everything listed in
   `tools/sound-parity/footprint.ron`: whole paths, and exact edits to shared
   files. The `give-up-sound-replacement` skill does the rest of that session:
   tickets, the record, the game's handoff. A chain ticket adds its own lines to
   the footprint in its PR. A test fails on any file that names the chain's words
   and isn't covered, or an edit that no longer matches. CI's `sound-give-up` job
   really does it and builds and checks what is left. No agent gives up on its
   own judgement. A ticket outside the chain that would use `shuyi-sound` waits
   for 3079, so that giving up never takes another feature with it.
1. **New code goes beside the old.** 2210, 3030, 3031 and 3032 add crates and a
   path in `shuyi-app`. They don't change what the quad-snd path does.
2. **One switch.** `shuyi-app`'s cargo feature `in-house-sound` chooses the
   backend, behind the same `Backend` trait. No game code or file names a backend.
3. **A game can't come to need the new backend.** New manifest fields are
   optional. What quad-snd can't do goes through `Backend`'s provided methods,
   whose default is `Unsupported` with a truthful fallback, so a game using true
   pause, rate or a loop region still runs on quad-snd, as it does there today.
   A break only one backend could honour stops the stage until it needn't.
4. **The old path stays exercised.** Until 3079, CI's `sound-parity` job records
   quad-snd with every cell mandatory, whichever backend is the default.
5. **The default moves alone.** 3084 flips the feature on by default, with quad-snd
   still selectable, and undoing it is one line.
6. **Removal is last, late and rehearsed.** 3079 starts only after the game has
   moved its pin to 3084's default and run on it. Its removal is one commit
   holding nothing else, and a scratch branch proves reverting that commit brings
   quad-snd back with its matrix passing, before it merges.
7. **A test holds it.** `cargo test -p xtask` fails if `shuyi-app` loses
   `quad-snd`, the workspace's macroquad loses `audio`, or `in-house-sound`
   joins the default features. 3084 takes out the last of these; 3079 the rest.

## Consequences

Until 3079 merges, giving up is one session, and a game not past 3084's pin
changes nothing. Every chain ticket pays a few lines of footprint, and sound PRs
a CI job, to keep it so. After it,
going back is a revert that conflicts more as later work builds on the removal.
Replacing macroquad and its web loader (3034) closes that door in practice, since
quad-snd's web path runs on that loader, so 3034 must not start until the chain is
finished or abandoned. Keeping two paths costs CI time and some discipline until 3079.

## Alternatives considered

- **Reverting each stage's PR when the time comes:** other work lands between
  them, so it gets harder with every merge, and nothing proves it still works
  until the day it is needed.
- **Keeping the chain on a long-lived branch until the end:** nothing would be
  tested against the rest of the engine, and it would drift for months.
- **Keeping quad-snd forever as the fallback:** it leaves the quiet dependency
  Nick chose to leave, and ADR-0066 removes it.
- **A runtime switch between backends:** it ships both in every game and adds
  a setting no player needs; a build-time feature keeps one in any package.
- **Vendoring quad-snd for after the removal:** a disguised dependency, which 3079
  rules out. `Cargo.lock` and git hold the exact version a revert returns to.
