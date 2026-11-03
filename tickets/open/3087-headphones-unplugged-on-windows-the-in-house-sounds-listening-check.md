---
id: "3087"
title: "Headphones unplugged on Windows: the in-house sound's listening check"
type: feature
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: ["3031"]
nick_input: sign-off
completed:
---

# 3087 — Headphones unplugged on Windows: the in-house sound's listening check

## Context

3031 put the in-house sound behind `shuyi-app`'s `in-house-sound` feature, and one of
its criteria is a player's: "Unplugging headphones on Windows keeps sound going on the
speakers within a second." CI's `device-loss` cell holds the backend to it on a virtual
endpoint, taken away and given back while it plays (heard again 15 to 31 ms after the
endpoint is back), and Nick listened to the game with the new sound on Windows and in
the browser and signed it off. He had no headphones at hand, so that one check moved
here, apart from the work that goes on (his words, 2026-11-02: "we can do that in a
decoupled followup").

What it adds that CI can't: a real device going away while another stays, where CPAL's
stream may follow the new default itself or be lost and rebuilt
(`crates/sound-out/src/windows.rs`, [ADR-0075](../../docs/adr/0075-in-house-sound-output-cpal-beside-quad-snd-a-worklet-fed-by-messages-and-a-virtual-endpoint-for-windows-evidence.md)).

## Nick input

**Sign-off:** with headphones plugged into the Windows machine, play the game built with
the new sound, unplug them while music plays, and say whether the music carries on
from the speakers within about a second, and whether plugging them back moves it there
again.

## Scope

**In:**
- A build of the game with `in-house-sound` on, at the engine's revision of the time,
  as 3031's completion notes say (the root `[patch.crates-io]` of `alsa-sys`, the
  feature on `shuyi-app`, the game started from its own folder so it finds `music\`).
- Nick's check, and his words in the completion notes.
- If it fails: what the output's own counts say (`Output::stats`: rebuilds and silent
  callbacks), and a ticket for the fix.

**Out (do not do):**
- Changing the default backend (3084).
- A device picker or exclusive mode (3031's **Out**).

## Implementation steps

1. Build the game as **In** says, and give Nick the commands, in PowerShell.
2. Record what he says.

## Acceptance criteria

- [ ] Nick unplugged headphones while the game played with the new sound, and says
      whether it went on from the speakers within a second.
- [ ] His words are in the completion notes.

## Tests required

- Manual: the sign-off.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
