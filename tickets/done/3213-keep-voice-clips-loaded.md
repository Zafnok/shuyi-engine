---
id: "3213"
title: A screen can keep a set of voice clips loaded, beside the clips coming next
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-12
---

# 3213 — A screen can keep a set of voice clips loaded, beside the clips coming next

## Context

1. **The game ticket:** game#0494
   ([`tickets/done/0494-vixs-tips-speak-in-the-chapter-1-demo.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0494-vixs-tips-speak-in-the-chapter-1-demo.md)).
   The game's tutorial guide says a line when a tip or a lesson box comes
   on screen. On the web build, served from itch.io, several of her lines
   stayed silent: a clip is fetched when it is asked for, and one not
   loaded `LATE_SECS` (0.3 s) after it was asked for is dropped
   (`crates/platform/src/audio/player/voice.rs`).
2. **The job in one sentence:** keep the clips of a screen's lines that
   may be said at any moment loaded, so each plays at once when asked for.
3. **No friction review:** a missing feature. game#0494 preloads the
   guide's lines with `Ctx::preload_voices` (likeliest first), which
   keeps only the next `AHEAD` (3) loaded. A box the player triggers at a
   moment the game can't predict (picking a unit up the first time, the
   cursor resting on a new kind of ground) is often not among those 3:
   in the game's web demo, served with each clip held 0.5 s, the tip
   shown on first picking a unit up was fetched only as it showed, and
   was silent.
4. **The generic shape:** besides the clips coming next in order
   (`preload_voices`, a script read line by line), a screen can name a
   set of clips to **keep** loaded until it names another set (an empty
   one to let them go). Clips in the set are never freed by the
   coming-next window moving on, and play at once when asked for.
5. **An example from a game that isn't tactics:** a farming game's
   shopkeeper with a dozen barks, one said when the player picks up an
   item, opens a drawer or walks off; a platformer's companion who
   remarks on whichever of several hazards the player meets first. The
   lines are few and short, and which comes next depends on the player.

The game pins `234de697` (`shuyi-engine.rev`); it calls
`preload_voices` from its dialogue screen, its debug menu's voice test,
and (game#0494) its battle screen for the guide's lines
(`gh search code --repo Zafnok/visions-of-shuyi preload_voices`).

## Nick input

None.

## Scope

**In:**
- `Ctx::keep_voices(&mut self, lines: &[LineId])`: the clips to keep
  loaded, replacing the set named last; lines without a playable clip
  are left out, as `preload_voices` does. Unlike `preload_voices`, an
  empty or all-unplayable list **is** sent (it lets the set go), and
  nothing is sent while voices are off.
- A request carrying the set (`AudioRequest::KeepVoices { lines }`, a
  `SoundRequest::KeepVoices { clips }` beside `PreloadVoices`), passed
  to the platform like `PreloadVoices`.
- The voice player keeps every file of the set wanted (`wanted`) and
  loads those missing (`keep_wanted`), beside the asked-for clip, the
  playing one and the next `AHEAD` coming. A file in both the set and
  the coming list is loaded once.
- Additive API: no break (`AudioRequest` is `#[non_exhaustive]` if it
  already is; say so in the completion notes either way).
- The doc of `Ctx::preload_voices` and the voice player's module doc
  name `keep_voices` and say when to use which.

**Out (do not do):**
- Changing `AHEAD` or `LATE_SECS`, or the late rule.
- A limit on the set's size or memory (a game names a handful of short
  clips; record it as a follow-up if a measure shows a need).
- The game's switch to it (game#0494's follow-up, after this merges and
  the pin moves).

## Implementation steps

1. `crates/ui/src/audio.rs`: `AudioRequest::KeepVoices { lines:
   Vec<(LineId, V)> }`, `AudioQueue::keep_voices`, and the variant in
   every `match` that lists `PreloadVoices` (search `PreloadVoices`).
2. `crates/ui/src/ctx.rs`: `Ctx::keep_voices`, as in Scope.
3. `crates/ui/src/game.rs`: the variant where `PreloadVoices` is
   handled.
4. `crates/platform/src/audio.rs`: `SoundRequest::KeepVoices { clips:
   Vec<String> }`, mapped from `AudioRequest::KeepVoices` like
   `PreloadVoices` (clip files by `clip_file`).
5. `crates/platform/src/audio/player/voice.rs`: a `kept: Vec<String>`
   field, set by the request; `wanted` adds `self.kept.contains(file)`;
   `keep_wanted` loads the kept files that aren't loaded.
6. Docs as in Scope.

## Acceptance criteria

- [x] A test of the voice player: with a set kept and a coming list
      whose window has moved past every file of the set, every kept file
      stays loaded; asking for one plays it on the next poll, with no
      load. An empty set frees them (unless coming next, asked for or
      playing).
- [x] A test of `Ctx::keep_voices`: unplayable lines left out; an empty
      list sent; nothing sent with voices off.
- [x] The request reaches the player through the platform's mapping (a
      test like `PreloadVoices`'s in `crates/platform/src/audio.rs`).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the voice player's kept set (above); `Ctx::keep_voices`; the
  request's mapping.

## Completion notes

- `Ctx::keep_voices(&[LineId])` (`crates/ui/src/ctx.rs`): the playable
  clips of the lines, sent even when none is left (it lets the set go);
  nothing while voices are off. `preload_voices` and it share a private
  `playable` helper. `AudioQueue::keep_voices` and
  `AudioRequest::KeepVoices { lines }` beside `PreloadVoices`, in each
  `match` that lists it (`cue`, the music player, `is_known`).
- `SoundRequest::KeepVoices { clips }` (`crates/platform/src/audio.rs`),
  mapped by `clip_file` like `PreloadVoices`.
- The voice player keeps a `kept` set: `wanted` counts it, `keep_wanted`
  loads its missing files (a file both kept and coming is loaded once),
  and `stop_all` clears it. The headless backend ignores it, as it does
  `PreloadVoices`.
- Tests: `a_kept_set_stays_loaded_until_another_is_named` (voice player),
  `keep_voices_sends_the_playable_set_even_empty_but_not_with_voices_off`
  (`Ctx`), the mapping in `SoundRequest::from_request`'s test, and the
  request beside `PreloadVoices` in the music player's and `is_known`'s
  tests.
- Docs: the doc of `Ctx::preload_voices` and the voice player's module
  doc name `keep_voices` and when to use which; the backend guide's list
  of requests (`docs/platform-backends.md`).
- No deviation.

No break: `AudioRequest` and `SoundRequest` were already
`#[non_exhaustive]`, so the new variants are additive. The game matches
`AudioRequest` with a wildcard (`gh search code --repo
Zafnok/visions-of-shuyi PreloadVoices`: `crates/ui/src/debug.rs`, its
dialogue tests and `crates/ui/tests/it/voice.rs` build or match
`PreloadVoices`, nothing exhaustive over the enum).

Book: the chapter *Games and screens* (`site/content/learn/book/games-and-screens.md`)
says when to use `preload_voices` and when `keep_voices`.

Game: game#0494 (its PR Zafnok/visions-of-shuyi#664, or its follow-up
after this merges, moves the pin and switches the guide's lines to
`keep_voices`).
