---
id: "2250"
title: Drop the voice module's cast, which no game reads
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2250 — Drop the voice module's cast, which no game reads

## Context

1. **The game ticket:** `game#0760`
   ([github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0760-one-form-for-cast-ron.md](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0760-one-form-for-cast-ron.md)).
2. **The job in one sentence:** a game plays voice clips from its
   manifest alone; which voice says each speaker's lines matters only to
   the tool that makes the clips.
3. **Friction numbers:** no friction review: an API with no user left.
4. **The generic shape:** `shuyi_assets::voice` has the manifest
   (`VoiceManifest`, `from_source`, staleness) and a cast (`Cast`,
   `cast_from_source`: speaker to voice id). The cast is read by nothing
   at run time: a clip's `voice` is a free string in the manifest, and
   playback looks clips up by line id. Visions of Shuyi was its only
   user, and since `game#0760` it no longer calls it: its cast lives in
   its voice-making tool (`cargo xtask voice`), in a richer form of its
   own (each speaker's reference clip and the program that reads its
   lines), which the engine's speaker-to-id map can't hold.
5. **An example from a game that isn't tactics:** a visual novel with
   recorded voices plays its clips by line id from the manifest; its
   casting sheet (actor per character) is a studio document, never read
   by the game. Nothing in it would call `cast_from_source`.

## Nick input

None.

## Scope

**In:**
- Remove `voice::Cast`, `voice::cast_from_source`, `RawCast` and their
  tests from `crates/assets/src/voice.rs`.
- The module's doc and `docs/formats.md`'s mention of
  `voice::cast_from_source`; the book's voice chapter if it shows the
  cast.

**Out (do not do):**
- A generic "cast for a voice-making tool" format: each tool's needs
  differ (reference clips, model names, actors). If a second game asks
  for one, that is a new ticket.
- Any change to the manifest, its staleness rule or `ClipVariant`.

**API:** breaking (two public items removed). The game no longer uses
them, so the game's PR that moves the pin past this one needs no code
change.

## Implementation steps

1. Delete `Cast`, `RawCast` and `cast_from_source` from
   `crates/assets/src/voice.rs`, and the tests that call them.
2. Reword the module doc: the manifest only.
3. Remove the mention in `docs/formats.md` (and the book, if any).

## Acceptance criteria

- [ ] `grep -rn "cast_from_source\|voice::Cast" crates docs` finds nothing.
- [ ] The completion notes have a `Breaking:` line for each removed item.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: none new; the manifest's tests stay as they are.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
