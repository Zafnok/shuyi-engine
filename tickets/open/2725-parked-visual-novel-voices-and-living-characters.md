---
id: "2725"
title: "Parked: Visual novel voices and living characters: a voice per line id, mouths that move with it, and eyes that blink"
type: feature
milestone: "Engine: visual novels"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2722", "1123"]
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
  backlog's voice hook, and `VnSettings`;
- 1123: `AnimationClip` and `frame_at(clip, elapsed_ms)`, a pure
  function of time.

Playing sound stays in the platform backend, `shuyi-app` (`AGENTS.md`
rule 5). The story and the scene only say which voice plays, at what
volume, and when it stops.

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
  - a shown line's voice is found by its line id, per language, by the
    convention game ADR-0045 sets. If it sets none, use
    `voices/<language>/<line id>` with any audio extension the formats
    know, falling back to the base language;
  - the `voice <line id>` direction (2721) overrides it, to reuse
    another line's voice;
  - a line with no voice is fine and plays silent;
  - `CharacterDef` gains `voiced: bool` (default false), for the check
    below.
- **The voice envelope**, `VoiceEnvelope`, asset kind `VoiceEnvelope`,
  format `voice-ron` (`*.voice.ron`):
  - `source_hash` (of the audio file's bytes), `duration_ms`, `step_ms`
    (default 50), and `levels`: one integer from 0 to 3 per step;
  - written by a tool, `shuyi bake` (or `shuyi voice-envelope <file>`
    if bake isn't there yet), never computed at run time. The tool
    decodes the audio, takes the loudness of each step, and quantizes it
    against the file's own peak. Floats are allowed in the tool only;
    the result is integers checked into the game;
  - decoding audio in the tool: use the decoder `shuyi-assets` already
    has for WAV and OGG if it gives samples. If not, add to `shuyi-cli`
    only `hound` (WAV, believed Apache-2.0) and `lewton` (OGG Vorbis,
    believed MIT OR Apache-2.0). Check each license from its
    `Cargo.toml` and repository on the day, and that `cargo deny check`
    passes. Neither goes into the runtime crates;
  - a fuzz-style test and a determinism fixture (ADR-0007 §5), with a
    short fixture voice we make (CC0-1.0, ours, listed in
    `THIRD_PARTY_ASSETS.md` as our own, created if it doesn't exist).
- **Playing voices** through the engine's sound path: the `Audio` trait
  if 3003 is done, otherwise `shuyi-app`'s sound calls.
  - It needs `play(sound, volume) -> handle` and `stop(handle)`, with
    volume an integer from 0 to 100. Add them if missing. `shuyi-app`
    turns the integer into macroquad's volume;
  - the story's screen plays the voice when a line is shown, and stops
    it on `Advance`, on skip, on going back, and on load;
  - **volume**: `VnSettings` (2723) gains `voice_volume` and a volume
    per voiced character, each 0–100. The volume sent is their product
    divided by 100. The quick menu's settings get a row each.
- **Auto waits for the voice**: 2723's auto sends `Advance` only after
  the reveal, the pause, and the voice's `duration_ms`. With no
  envelope, it doesn't wait for the voice, and the README says to bake
  envelopes.
- **The backlog's voice hook** (2723): replaying an entry's voice plays
  it at the current volume, and stops any voice playing.
- **Mouths** in `CharacterDef`: `mouth: Option<Mouth>` with:
  - `frames`: per expression, a list of mouth pictures from closed to
    open (2 to 4), drawn as a layer over the face;
  - `mode`: `Envelope` (the default) or `Flap { clip }` (a 1123 clip).
  - `mouth_frame(envelope, elapsed_ms, frame_count) -> usize`, pure:
    level × (frame count − 1) / 3. Past the end, the mouth is closed;
  - only the character speaking the voiced line moves their mouth. With
    no voice playing, every mouth is closed.
- **Blinking** in `CharacterDef`: `blink: Option<Blink>` with:
  - a clip per expression (1123), drawn as a layer over the eyes;
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
3. Add `play` and `stop` with an integer volume where they are missing,
   in the platform side and `shuyi-app`. Use a fake audio that records
   calls in tests.
4. Play and stop voices from the story's screen; add the volume
   settings; make auto wait; fill the backlog hook.
5. Add `mouth` and `blink` to `CharacterDef` (update its fixtures and
   determinism hash), then `mouth_frame` and the blink schedule in the
   presenter, then the layers in `build_scene`.
6. Add the check rules, or 2724's scope.

## Acceptance criteria

- [ ] A shown line with a voice plays it once at the right volume; a
      line without one plays nothing (fake audio test).
- [ ] `Advance`, skip, going back and loading each stop the voice
      playing (fake audio test).
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

- Unit: the voice lookup and its fallback; `mouth_frame`; the volume
  product; the check rules.
- Property: the blink schedule's bounds and determinism; voices never
  change the replay hash.
- Fuzz-style: random bytes into `voice-ron` never panic.
- Fixture and determinism: the envelope of the fixture voice.
- Snapshot: the scene dump while a voiced line plays, at three ms
  values.
- Integration: a Harness test with a fake audio, through a voiced line,
  an `Advance` that stops it, and the backlog replaying it.

## Completion notes
