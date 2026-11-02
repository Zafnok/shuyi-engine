---
id: "3114"
title: A dialogue line carries a delivery direction, never shown, and a voice clip remembers it
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-02
---

# 3114 — A dialogue line carries a delivery direction, never shown, and a voice clip remembers it

## Context

- **The game ticket:** game#0752
  (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0752-voice-direction-cues-in-scripts.md`).
- **The job in one sentence:** a writer says how a line is spoken ("whispering,
  frightened") in the script, the player never reads it, and a voice tool or an
  actor does.
- **The friction numbers:** no friction review: a missing feature.
- **The generic shape:** a `.dlg` line has an optional free-text direction, a
  scene can set one for a stretch of lines, and a clip of the voice manifest
  records the direction it was made under, so a tool can tell it needs remaking.
- **An example from a game that isn't tactics:** a farming game's shopkeeper
  who says the same words "cheerfully" in spring and "worn out" in winter, read
  by a hired actor from a recording script.

## Nick input

None.

## Scope

**In:** `speaker (direction): text`, `speaker[expression] (direction): text`,
`> (direction): text`; `@delivery <direction>` and `@delivery clear`; the
direction on `Step::Say`, `Step::Narrate` and `Line`; `direction` in the voice
manifest and `VoiceManifest::needs_remake`.

**Out (do not do):** calling a voice tool; a fixed list of directions; showing
a direction anywhere.

## Implementation steps

1. Parse a line's own direction, and `@delivery`, into the line's `direction`.
2. Keep it out of the line id; print it back so a printed scene parses the same.
3. Add it to the manifest, and `needs_remake`.

## Acceptance criteria

- [x] A line's own direction, `@delivery` and `@delivery clear` parse, and a
      line's own direction wins (tests).
- [x] A `@delivery` inside an `@if` part or a reply's reaction ends with it (test).
- [x] A direction changes no line id (test).
- [x] An empty direction is an error (test).
- [x] A printed scene parses back equal, directions included (test).
- [x] A changed direction means a remake and the clip stays playable (test).

## Tests required

Unit tests in `crates/assets/src/dialogue/tests.rs` and
`crates/assets/src/voice/tests.rs`.

## Completion notes

Done as scoped. `@delivery` makes no step: the parser writes its direction on
each line it covers, so a player, a language pack and a tool read one field,
and `print_scene` writes each line's own.

A clip whose direction changed stays in `Playable`: it still says the words on
screen, and a silent line would be worse. `needs_remake` is what a clip-making
tool asks.

Breaking: shuyi-assets: `dialogue::Step::Say`, `dialogue::Step::Narrate`: new field `direction`; a game that builds or fully matches one adds it.
Breaking: shuyi-assets: `dialogue::Line`: new field `direction`; a game that builds one adds it.
Breaking: shuyi-assets: `voice::VoiceClip`: new field `direction`; a game that builds one adds it.
Breaking: `.dlg` format: a speech line's head may hold ` (direction)`, and `@delivery` is the engine's; a game with its own `@delivery` directive renames it.
