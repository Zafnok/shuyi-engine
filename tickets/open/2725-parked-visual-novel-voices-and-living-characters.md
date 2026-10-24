---
id: "2725"
title: "Parked: Visual novel voices and living characters: a voice per line id, mouths that move with it, and eyes that blink"
type: feature
milestone: "Engine: visual novels"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2722", "2723", "2231", "2342", "2226", "2344"]
nick_input: none
completed:
---

# 2725 — Voices, moving mouths and blinking eyes

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0016](../../docs/adr/0016-visual-novels-and-detective-games-a-story-stage-a-case-file-and-a-fair-play-check.md)
asks for voice lines tied to lines (*Context*, §2's `voice` command,
§3's backlog "with each line's voice replayable"). Lines already have
ids for voice and translation (game ADR-0045). Players also expect
characters to look alive: a mouth that moves while they speak, and eyes
that blink.

What it builds on:

- 2721: `VnEvent::Line` and `VnEvent::Voice`, and `CharacterDef`'s
  layers;
- 2722: `VnPresenter` (time on the screen, outside `VnState`) and
  `VnScene`'s character layers;
- 2723: auto mode (it must wait for the voice), skip, going back, the
  backlog's voice hook, and the quick menu;
- 2231: `AnimationClip` and `frame_at(clip, elapsed_ms)`, a pure
  function of time;
- 2342: sound groups, with the voice volume as the `voice` group's;
- 2226: `Lang::chain`, a language's fallback chain.

Playing sound stays in the platform backend, `shuyi-app` (`AGENTS.md`
rule 5). The story and the scene only say which voice plays, in which
sound group, and when it stops.

**The mouth: an envelope, with a flap clip as the fallback.** A voice's
loudness over time is measured once by a tool, at bake time, and
stored as small integers. At run time the mouth frame is a pure
function of the envelope and the ms since the voice started. This
follows the speech: the mouth closes in pauses and opens wider on loud
syllables. It costs about 20 bytes per second of voice, and nothing at
run time but a table lookup. A simple flap clip (open and close at a
fixed rate while the voice plays) can't close in pauses, so it is only
the fallback: for a character set to `Flap`, or a voice with no
envelope yet (while a writer is still working). Lip sync from phonemes
is out: it needs per-language models and floats at run time.

## Nick input

None.

## Scope

**In:**
- **A voice per line id** in `shuyi_vn::voice`:
  - a shown line's voice is found by its line id, per language, where
    the engine finds voice clips today (game ADR-0046): the folder
    `VoiceFiles` names (`voice/<language>/`, shipped beside the game),
    whose manifest (`shuyi_assets::voice::VoiceManifest`, read with
    `read_voice_manifest`) lists each clip by line id. The language is
    the one in use at run time, not `VoiceFiles::lang` fixed at build
    time, falling back along its chain (2226's `Lang::chain`: `pt-BR`,
    then `pt`, then the game's source language). The clips stay in
    that folder beside the game, not in a pack's `files/` (2349's
    `localized_path`);
  - **Either order with 2349:** if it is done, the language in use is
    the player's pick (`Settings::language`, through `Ctx::lang`), so
    a voice follows the options row; if not, it is `Ctx::lang` as the
    game sets it, and 2349 has nothing to do here (its Out leaves
    voices per language to this ticket);
  - the `voice <line id>` direction (2721) overrides it, to reuse
    another line's voice;
  - a line with no voice is fine and plays silent;
  - `CharacterDef` gains `voiced: bool` (default false), for the check
    below.
- **The voice envelope**, `VoiceEnvelope`, asset kind `VoiceEnvelope`,
  format `voice-ron` (`*.voice.ron`):
  - `source_hash` (of the audio file's bytes), `duration_ms`, `step_ms`
    (default 50), and `levels`: one integer from 0 to 3 per step;
  - written by a tool, `shuyi voice-envelope <file>` in `shuyi-cli`,
    never computed at run time. The tool decodes the audio, takes the
    loudness of each step, and quantizes it against the file's own
    peak. Floats are allowed in the tool only; the result is integers
    checked into the game. **From 2204 (done):** `shuyi bake` packs a
    game's sources as they are, so a `*.voice.ron` goes into the pack
    like any other source, and bake computes nothing from the audio;
  - decoding audio in the tool: use the decoder `shuyi-assets` already
    has for WAV and OGG if it gives samples. If not, add to `shuyi-cli`
    only `hound` (WAV, believed Apache-2.0) and `lewton` (OGG Vorbis,
    believed MIT OR Apache-2.0). Check each license from its
    `Cargo.toml` and repository on the day, and that `cargo deny check`
    passes. Neither goes into the runtime crates;
  - a fuzz-style test and a determinism fixture (ADR-0007 §5), with a
    short fixture voice we make (CC0-1.0, ours, listed in
    `THIRD_PARTY_ASSETS.md` as our own, created if it doesn't exist).
- **Playing voices** through the engine's sound path: `AudioRequest`
  and the platform's `Audio` trait (3003).
  - A voice clip plays with `PlayVoice` and stops with `StopVoice`, as
    today. Any other sound it must stop uses 2344's `start_sound` and
    `stop_sound`, never a second kind of handle. Its volume is its
    group's gain (below), which `shuyi-app` turns into its own volume.
    Add another handle only if a voice clip needs one these don't give.
    **Either order with 3029:** if 3029 is done, add whatever is missing
    to its shared backend trait, not to `shuyi-app` alone; if not, add
    it to `shuyi-app`'s player, and 3029 moves it with the rest;
  - the story's screen plays the voice when a line is shown, and stops
    it on `Advance`, on skip and on going back;
  - **Either order with 2724:** if 2724 is done, loading a save stops
    the voice too; if not, nothing to do (2724 stops it when it lands);
  - **volume**: the voice volume is the `voice` sound group's (2342),
    not a `VnSettings` field. A volume per voiced character is a group
    under `voice`, declared in the audio manifest. 2342 plays every
    clip in `voice`, so this ticket adds the clip's group to
    `AudioRequest::PlayVoice` and `SoundRequest::Voice`, and the player
    plays it at that group's gain (`Gains::group`). The quick menu gets
    a row for `voice` and one for each character's group.
  - **Either order with 2343** (ducking under a voice): if 2343 is
    done, the `sounding` it hands `DuckState::update` also needs the
    gain of the playing clip's own group above 0, not only `voice`'s,
    so a character turned down to 0 lowers nothing, with a test; if
    not, nothing to do (2343 does it).
- **Auto waits for the voice**: 2723's auto sends `Advance` only after
  the reveal, the pause, and the voice's `duration_ms`. With no
  envelope, it doesn't wait for the voice, and the README says to bake
  envelopes.
- **The backlog's voice hook** (2723): replaying an entry's voice plays
  it at the current volume, and stops any voice playing.
- **Mouths** in `CharacterDef`: `mouth: Option<Mouth>` with:
  - `frames`: per expression, a list of mouth pictures from closed to
    open (2 to 4), drawn as a layer over the face;
  - `mode`: `Envelope` (the default) or `Flap { clip }` (a 2231 clip).
  - `mouth_frame(envelope, elapsed_ms, frame_count) -> usize`, pure:
    level × (frame count − 1) / 3. Past the end, the mouth is closed;
  - only the character speaking the voiced line moves their mouth. With
    no voice playing, every mouth is closed.
- **Blinking** in `CharacterDef`: `blink: Option<Blink>` with:
  - a clip per expression (2231), drawn as a layer over the eyes;
  - `every_ms: (min, max)`, in whole ms.
  - The gaps between blinks come from the core's seeded random numbers,
    seeded from the story's seed and the character's id, so two runs
    with the same seed blink at the same ms. The presenter keeps the
    next blink time; the scene shows the clip's frame at that time.
- **The scene**: `VnScene`'s character layers include the current mouth
  and eye frames, so tests read them from the scene.
- **The story check**: a voiced character's line with no voice, a voice
  with no envelope, and an envelope whose `source_hash` doesn't match
  its audio (stale) are warnings with file and line. If 2724 is done,
  add these rules to `shuyi_vn::check`. If not, add them to 2724's
  scope in this PR.
- **Either order with 2730** (testimony, in the mystery module): if
  2730 is done, its `TestimonyScreen` plays the shown statement's voice
  by its line id, moves the witness's mouth, and stops the voice when
  the statement changes, on press and on present, as the story's screen
  does (fake audio test); if not, nothing to do (2730 does it).

**Out (do not do):**
- lip sync from phonemes, and any analysis at run time;
- recording or generating voices;
- voices for the example's story (2726);
- music and sound effects, which `.dlg` already plays;
- anything in `VnState`: voices, mouths and blinks are presentation.

## Implementation steps

1. Write the voice lookup and its unit tests, with a fake asset list.
2. Write `VoiceEnvelope` and `voice-ron`, then the tool that writes it,
   with the fixture voice.
3. Add the clip's group to `PlayVoice` and `SoundRequest::Voice`, and
   play the clip at that group's gain in `shuyi-app` (in 3029's shared
   player if it is done). Use a fake audio that records calls in tests.
4. Play and stop voices from the story's screen; add the quick menu's
   volume rows; make auto wait; fill the backlog hook.
5. Add `mouth` and `blink` to `CharacterDef` (update its fixtures and
   determinism hash), then `mouth_frame` and the blink schedule in the
   presenter, then the layers in `build_scene`.
6. Add the check rules, or 2724's scope.

## Acceptance criteria

- [ ] A shown line with a voice plays it once, in its speaker's group;
      a line without one plays nothing (fake audio test).
- [ ] `Advance`, skip and going back each stop the voice playing, and
      so does loading a save if 2724 is done (fake audio test).
- [ ] The fixture voice's envelope is the same file on every run, and
      `voice-ron` passes its fuzz-style test and determinism fixture.
- [ ] With the fixture envelope, the mouth frame at each step matches a
      checked-in list; it is closed in the fixture's silent part and
      after the end.
- [ ] Blink times for a seed and a character are the same on every run,
      every gap is within `every_ms`, and a different seed gives a
      different schedule (property test).
- [ ] Auto with a voice playing waits for `duration_ms`.
- [ ] The `VnState` hash is the same with voices, mouths and blinks on
      or off (property test over random command records).
- [ ] No `f32` or `f64` in `shuyi-vn` outside the envelope tool, which
      lives in `shuyi-cli`.
- [ ] `cargo deny check` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the voice lookup and its fallback along the chain;
  `mouth_frame`; the clip's group on `PlayVoice`; the check rules.
- Property: the blink schedule's bounds and determinism; voices never
  change the replay hash.
- Fuzz-style: random bytes into `voice-ron` never panic.
- Fixture and determinism: the envelope of the fixture voice.
- Snapshot: the scene dump while a voiced line plays, at three ms
  values.
- Integration: a Harness test with a fake audio, through a voiced line,
  an `Advance` that stops it, and the backlog replaying it.

## Completion notes
