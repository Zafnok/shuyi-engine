---
id: "2784"
title: "Parked: DOOM's music: its MUS lumps turned to MIDI and played through the engine's MIDI synth"
type: feature
milestone: "Engine: demos"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: blocked
blocked_by: ["2780", "2206"]
nick_input: none
completed:
---

# 2784 — DOOM's music

> **Parked** (Nick, 2026-11-03: the DOOM and Bad Apple tickets are open, after the maze chase; this one needs MIDI, 2205 and 2206, which Nick parked on 2026-10-17): it is parked with them. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md),
*Bad Apple and DOOM*). 2780 runs DOOM on the engine, from
`github.com/Zafnok/shuyi-doom` (GPL), with its sound effects but no
music. DOOM's music is in its WAD as **MUS** lumps (`D_*`), a compact
form of MIDI. doomgeneric converts MUS to a standard MIDI file
(`mus2mid.c`); playing it needs a MIDI synth, and the engine's is
2205 (the `shuyi-midi` module, `.mid` and `.sf2`) and 2206 (the synth,
cues that play a sequence, playback in `shuyi-app`), both parked.

Freedoom's music is its own, under its license, so the port can play it
on Pages.

## Nick input

None.

## Scope

**In** (in `Zafnok/shuyi-doom`, unless it says this repository):
- Each `D_*` lump converted with doomgeneric's `mus2mid` when DOOM
  starts the song, and played through 2206's sequence cues, looping
  when DOOM asks. DOOM's music volume maps to the engine's music
  volume.
- A General MIDI SoundFont under a license the port may ship (check it
  against the port's license choice, 2780), or the engine's default
  synth voices if 2206 ships them.
- Desktop and the web.
- **This repository**: the PR moves the ticket to `done/`.

**Out (do not do):**
- OPL (AdLib) emulation.
- Music on the Playdate: MIDI there is 3057 (parked), a follow-up once
  it's done.
- Writing a MIDI synth in the port: the engine's is 2206's.

## Implementation steps

1. Read 2205 and 2206 as done, and doomgeneric's `i_sound.c`,
   `mus2mid.c`.
2. Convert and play; the volume; the loop.
3. Play Freedoom's first map with music on the desktop and the web.

## Acceptance criteria

- [ ] Freedoom's music plays in the port on the desktop and the web,
      looping, at DOOM's music volume.
- [ ] The SoundFont's license is recorded in the port's credits.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass, here and the port's own.

## Tests required

- Unit (in the port): `mus2mid` of a hand-made MUS lump equals a known
  MIDI file.
- Snapshot / integration: the headless speaker hears the first song's
  first second equal to a golden.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
