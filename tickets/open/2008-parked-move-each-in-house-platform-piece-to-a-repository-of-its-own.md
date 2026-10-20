---
id: "2008"
title: "Parked: Move each in-house platform piece (sound, sound output, pads, embedding, and macroquad's replacement) to a repository of its own"
type: infra
milestone: "Engine: in-house platform"
model: sonnet-5
effort: medium
status: blocked
blocked_by: ["2902", "2903", "2215", "3034"]
nick_input: setup
completed:
---

# 2008 — Move each in-house platform piece to a repository of its own

> **Parked** (Nick, 2026-10-19: "super down the line - carve each of these new things into their own repos which we pull into engine … I want them to be full fledged replacements first"): after every piece is a full replacement and Nick says it's time. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): each in-house piece moves to its
own repository, which the engine depends on, "super down the line", once
it's a full replacement.
[ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§4 made each one a crate with no `shuyi-*` dependency, so the move is
mechanical:

- `shuyi-sound` (2210–2214, 3030) and `shuyi-sound-out` (3031, 3032), if
  the sound research (2902) kept our own code;
- `shuyi-pads` (2402, 2403), if the controller research (2903) kept our
  own code;
- `shuyi-embed` (2215);
- the 2D renderer crate that 3034's tickets make. If those are
  not all done when this is unparked, move the others, and leave a
  ticket for those.

The engine moved out of the game the same way (game ticket 1131,
`docs/carve.md`): with its history.

## Nick input

**Setup:** for each piece, when this ticket starts:

1. Decide the repository's name (the ticket proposes `shuyi-sound` and
   so on, under `github.com/Zafnok`; `ask-nick` if there's a better one).
2. Create an empty public repository with that name on GitHub
   (*New repository*, no README, no license, no `.gitignore`).
3. Say so in the session, and the session pushes the history.

Also a **decision**, asked with `ask-nick` when the ticket starts: publish
these crates on crates.io (like 2005 for the engine), or keep them as git
dependencies.

## Scope

**In:**
- For each crate: its history split out (`git subtree split` or
  `git filter-repo`, whichever keeps the commits that touched it), pushed
  to its repository with the license files, README, CI (fmt, clippy,
  tests, docs, `cargo deny`, the no-std build where it applies), and
  `typos`.
- The engine depends on each by a pinned git revision (or crates.io
  version, per Nick's answer), and the crate's folder leaves `crates/`.
- `docs/carve.md` gets a section saying what moved where.
- The `run-gates` skill and `AGENTS.md` say where these crates live and
  how a change to one is made (a PR there first, then a pin move here,
  as the game does with the engine).

**Out (do not do):**
- Changing any crate's API in the move.
- Moving engine crates (`shuyi-core`, `shuyi-ui`, `shuyi-assets`,
  `shuyi-app`).

## Implementation steps

1. For one crate first (`shuyi-embed`, the smallest): split, push, set up
   CI, depend on it from here, remove the folder. Check the game still
   builds at the next pin.
2. The others the same way, one PR here each, or one PR if Nick prefers.
3. Docs (scope above).

## Acceptance criteria

- [ ] Each moved crate builds and passes its CI in its own repository.
- [ ] This repository builds with each as an outside dependency, and its
      gates pass.
- [ ] The game builds at the next pin.
- [ ] `docs/carve.md` and `AGENTS.md` say where each piece lives.

## Tests required

- The crates' own tests, run in their new CI.
- This repository's gates.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
