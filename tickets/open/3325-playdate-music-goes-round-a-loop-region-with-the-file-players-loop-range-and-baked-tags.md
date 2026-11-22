---
id: "3325"
title: "Playdate music goes round a loop region: the file player's loop range, and a baked track keeps its file's tags"
type: feature
milestone: "Engine: audio"
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: ["2397"]
nick_input: none
completed:
---

# 3325 — Playdate music goes round a loop region

## Context

**From 2397 (done):** a looped `MusicCue` may name a loop region
(`loop_start_ms`, `loop_end_ms`, `LoopRegion`), or its Ogg file may (the
`LOOPSTART` and `LOOPLENGTH` comments, read by
`shuyi_sound::tags::loop_region` and `VorbisSource::loop_region`), the cue's
winning ([ADR-0136](../../docs/adr/0136-a-tracks-loop-region-is-the-cues-or-its-files-played-by-the-backend-and-the-clock-told-where-it-is.md)).
The shared player calls `Backend::set_loop_region(sound, region)` once a
looped track loads and reports the clock from the `Looping` it answers; the
default is `Err(Unsupported)`, which plays the whole track and warns once for
a cue that names a region. 2397 split this off: the Playdate backend
(`crates/playdate/src/audio.rs`, `Speaker<S: Sdk>`) still has the default, so
a region plays whole there, truthfully.

The Playdate plays music with the SDK's file player, from a track `shuyi
export playdate` decoded and baked to ADPCM (`crates/cli/src/export/playdate/audio.rs`,
`decode_ogg`, `adpcm_wav`). Baking drops the Ogg comments, so a file's own
region is lost on the way.

## Nick input

None.

## Scope

**In:**
- `Sdk::file_set_loop_range(player, start_secs, end_secs)` over
  `fileplayer->setLoopRange`, in the fake (`crates/playdate/src/fake.rs`)
  with its calls logged, and on the device.
- `Speaker::set_loop_region`: the cue's region, converted at the baked
  track's rate as `LoopRegion::frames` says (frames rounded down), or the
  baked file's region; answers the `Looping` it set, in seconds from those
  frames. Units tested: a region's seconds against the frames they come
  from, at the baked rate.
- The baked track keeps its source's region: the exporter reads the Ogg
  tags with `shuyi_sound::tags::loop_region` over the comments it decodes
  (lewton's comment header, as `decode_ogg` reads), refuses tags that
  don't read naming the file and the tag, and writes the region where the
  device finds it beside the track (a `smpl` chunk's loop in the baked
  WAV, or a field of the baked manifest: decide, and say why). A region
  in frames of the source is the same frames in the baked file, which
  keeps the rate.
- No desktop decoder on the handheld: the device reads only what was baked.
- If the CLI comes to depend on `shuyi-sound` for the tags, that line goes in
  `tools/sound-parity/footprint.ron` with what a give-up leaves in its place
  (ADR-0068), or the exporter reads the two comments itself and checks them
  as `shuyi_sound::tags` does.
- Docs: `docs/platform-backends.md` (the Playdate plays regions),
  `docs/formats.md` if the baked form changes, `docs/targets/` for the
  Playdate.

**Out (do not do):**
- A tracker's restart point on the GBA (3027 maps it).
- Seeking in the file player other than its loop range.

## Implementation steps

1. The SDK call, its fake, and `set_loop_region` for the cue's region.
2. The exporter reads and checks the tags, and bakes the region.
3. The device reads the baked region; docs.

## Acceptance criteria

- [ ] A cue's region sets the file player's loop range, in seconds from the
      baked frames, and the clock follows it (a test through the fake).
- [ ] A file's own region survives baking and is the one played when the cue
      names none; bad tags are refused at export, naming the file and tag.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: units of the loop range at the baked rate; the exporter's tags.
- Integration: the fake's file player gets the range; a baked file keeps
  the corpus's `intro_loop.ogg` region.

## Completion notes

*(Filled in by the session that completes the ticket.)*
