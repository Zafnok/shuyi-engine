---
id: "2414"
title: "Controllers on the Steam Deck: try gilrs and SDL3 on the Deck's controls and a pad, before SDL3 replaces gilrs"
type: research
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2903"]
nick_input: setup
completed:
---

# 2414 — Controllers on the Steam Deck

## Context

**From 2903 (done):** the controller research tried gilrs 0.11.2 and SDL
3.4.20 on Nick's pads on Windows
([`docs/research/2903-controllers-after-gilrs.md`](../../docs/research/2903-controllers-after-gilrs.md)),
and SDL3 was chosen
([ADR-0147](../../docs/adr/0147-desktop-pads-through-sdl3-with-our-own-start-up-for-switch-2-pads.md)).
The Steam Deck wasn't tried. Nick, 2026-11-22: "rn we leave untested but
have it as a followup". 2402 (parked) replaces gilrs with SDL3 and waits
for this, so the Deck's behaviour is known before the switch. 3012 wants
the Deck's controls reported as `PadKind::SteamDeck`.

2903's probe is a scratch program: a macroquad window that asks for each
button by position, the sticks, the triggers, both motors and an unplug,
and logs what the library reported. The report says what it measured.

## Nick input

**Setup:** copy one file to the Deck (USB stick or your own way), add it
to Steam as a non-Steam game, and run it in Game Mode; then once in
Desktop Mode with Steam closed; then with a pad plugged into the Deck.
About 2 minutes a run, six runs.

## Scope

**In:**
- Write the probe again in a scratch folder (outside the repository),
  with gilrs 0.11.2 (or the release of the day) and `sdl3-sys` with SDL's
  input parts (ADR-0147 §1), each as a cargo feature.
- Build it for `x86_64-unknown-linux-gnu` in Docker's `rust:1-bookworm`
  image (Nick allowed the pull on 2026-11-22), with `cmake` and
  `libudev-dev` installed in the container.
- Runs, each with both libraries: the Deck's controls in Game Mode
  (through Steam Input's virtual pad), the Deck's controls in Desktop
  Mode with Steam closed, and the Xbox or PS5 pad by USB-C.
- Record for each: the names and USB ids each library reports, every
  button by position, the back buttons if a library lists them, the
  sticks, triggers, both motors, unplugging the pad, and which ids would
  tell `PadKind::SteamDeck` apart (3012).
- Add a section to the 2903 report with the facts dated, and write what
  2402 and 3012 must change into their tickets.

**Out (do not do):**
- Changing the engine's code.
- Reading any library's source (ADR-0021 §2).

## Implementation steps

1. The probe, then the Linux build in Docker.
2. Hand Nick the file and the runs above.
3. The report's new section, and 2402's and 3012's tickets updated.

## Acceptance criteria

- [ ] The report has a dated Steam Deck section covering each run, for
      both libraries.
- [ ] 2402 and 3012 say what the Deck needs, or say nothing changes.
- [ ] The ticket lint passes, and all gates in the `run-gates` skill
      pass.

## Tests required

- None shipped (research).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
