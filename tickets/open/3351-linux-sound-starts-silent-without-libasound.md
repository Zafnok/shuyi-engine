---
id: "3351"
title: "A Linux build with the in-house sound starts silent where ALSA's library is missing, or the requirement is kept"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3032"]
nick_input: decision
completed:
---

# 3351 — Linux starts silent without `libasound`, or keeps needing it

## Context

3032 asked that the in-house sound keep a "planned silent launch without an audio
library", or that migration wait. What 3032 found, in CI's `sound-parity` job (the
step *What each native probe loads*) and its `sound-linux-floor` job:

- **Neither backend has it.** quad-snd's probe and the in-house one both name
  `libasound.so.2` in `DT_NEEDED`: the system's loader refuses to start either where
  that library is missing, before any of our code runs. CPAL's ALSA host links it
  through `alsa-sys`, which finds it with `pkg-config`; it opens nothing with
  `dlopen`. So making the in-house backend the default (3084) takes nothing away
  that players have today.
- **The packages say so.** [`docs/targets/desktop.md`](../../docs/targets/desktop.md)
  names ALSA among what a Linux package runs with, and `shuyi doctor` checks it.
- **The plan that assumed otherwise.** 3004's text once had 3032 open ALSA with
  `dlopen`; 3071 now says not to assume it. ADR-0136 plans winit and wgpu, which
  open their libraries with `dlopen`, so after 3333 ALSA may be the one library a
  Linux package can't start without.

[ADR-0142](../../docs/adr/0142-in-house-sound-on-linux-and-macos-quad-snds-alsa-bindings-are-alsa-syss.md)
records 3032's choice and this question.

## Nick input

**Decision:** must a Linux game start, without sound, on a system that has no ALSA
library at all (some minimal containers and servers; every desktop Linux and SteamOS
has it)? If not, the requirement stays as the packages say, and this ticket only
writes that down. If yes, the sound backend loads ALSA at run time.

## Scope

**In:**
- **If the requirement stays:** say so in `docs/targets/desktop.md` (what a package
  needs to start) and in ADR-0142's status line, and take this ticket off 3084's
  `blocked_by`.
- **If it must start silent:** load `libasound.so.2` at run time and fall back to
  `Output::null` with the backend's one warning line, as a machine with no device
  does. Prefer a released crate or CPAL feature that does it, checked against
  ADR-0002 §8; otherwise an adapter of our own over the documented ALSA calls CPAL
  needs, written from ALSA's documentation, in `shuyi-sound-out`'s `sys`. Then:
  - `cargo xtask sound-parity` and CI prove a probe with no `libasound.so.2` on the
    library path starts and plays nothing, and the `linux-in-house` cells still pass;
  - 3071's library lists drop `libasound.so.2` for builds with the in-house
    backend alone, and `shuyi doctor`'s `lib-alsa` row becomes a build-only check.

**Out (do not do):**
- quad-snd's own linking: it goes with 3079.
- PipeWire or JACK hosts.

**Estimated size:** about 600 lines (code 300, tests and CI 200, docs 100) if the
library is loaded at run time; about 30 if the requirement stays.

## Implementation steps

1. Ask Nick the decision (`ask-nick`).
2. Do the branch his answer picks.
3. Update ADR-0142's status, `docs/targets/desktop.md`, and 3084.

## Acceptance criteria

- [ ] Nick's answer is in `docs/decisions.md`.
- [ ] A Linux build with the in-house sound either starts silent without
      `libasound.so.2`, proven in CI, or the requirement is written where a game's
      maker reads it.
- [ ] 3084 no longer waits on this.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration (if loaded at run time): the probe started with no ALSA library on its
  path plays to the null output and says why once.

## Completion notes

*(Filled in by the session that completes the ticket.)*
