---
id: "3352"
title: "The web build ships a game's FLAC and MP3 music tracks and voice clips, not only its OGG ones"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3352 — The web build ships a game's FLAC and MP3 music tracks and voice clips, not only its OGG ones

## Context

Found by the docs audit of 2026-11-25 (the last 60 PRs read against the
docs).

2203 made FLAC and MP3 formats of `shuyi-assets` (features `flac` and
`mp3`), and 2234 plays them: a sound as a WAV in memory on every target,
a native track decoded on the music's worker thread, and on the web
"the browser decodes the file as it is (Web Audio's `decodeAudioData`)"
(`docs/formats.md`, the *Music* bullet under the FLAC and MP3 rows,
about line 441).

But the web build never puts such a track beside the page. Music tracks
are not embedded: `shuyi web` copies `web.music` into the page's folder,
and its filter takes only OGG:

- `crates/cli/src/web.rs`, `is_track`: `ext == "ogg"`, used by
  `copy_music` and by the size count beside it;
- `is_voice_file`: `ogg` or `ron`, used by `copy_voice`;
- the module doc (`web.rs`, lines 13–15) says `.ogg` for both.

So a game whose `audio.ron` names `title.flac` or `theme.mp3` plays it on
the desktop and is silent on the web, with no warning. The page's own
loader (`crates/cli/web/sound.js`) decodes whatever bytes it fetches, so
the filter is the only thing in the way.

## Nick input

None.

## Scope

**In:**

- `is_track` takes `ogg`, `flac` and `mp3` (case as the native reader
  takes it: read `shuyi_assets::audio`'s sniffing and the formats'
  extensions, and use the same list, from one place, not a second copy).
- Voice clips: if a native voice clip may be FLAC or MP3 (read how the
  voice reader in `shuyi-app` opens a clip), `is_voice_file` takes them
  too; if voices are OGG only by design, say so in `formats.md` and
  leave the filter.
- A track whose format the game's build can't play is a warning of
  `shuyi web`, not a silent drop: a file in `web.music` with another
  extension is named in the answer's warnings (`.wav` included, if the
  web loader doesn't stream it).
- Docs: `web.rs`'s module doc; `docs/formats.md`'s *Music* bullet if
  anything it says changes; `skills/shuyi-ship/SKILL.md` (about line
  128, "folders of `.ogg` tracks"); `crates/cli/web/README.md` if it names
  the extension.

**Out (do not do):**

- the in-house sound backend's streamed FLAC and MP3 tracks (2275);
- transcoding a track to OGG for the web;
- anything of the page's loader beyond what a test shows is needed.

## Implementation steps

1. Read `crates/cli/src/web.rs` (`is_track`, `is_voice_file`,
   `copy_music`, `copy_voice` and their tests, about line 1180 on), and
   `crates/cli/web/sound.js`'s track loading.
2. Widen the filters from one list of the music extensions.
3. Add the warning for a file in `web.music` that is skipped.
4. Run a web build of a game with a FLAC and an MP3 track and check in
   headless Chromium that each plays (`tools/web-check/check.py`, as
   `crates/cli/web/README.md`, *Checked in CI*, says), or say in the
   completion notes why it couldn't be run.
5. The docs above.

## Acceptance criteria

- [ ] `copy_music` copies `.ogg`, `.flac` and `.mp3` files, and its test
      shows each.
- [ ] A file in `web.music` with another extension is named in a warning.
- [ ] Voice clips: widened with a test, or `formats.md` says they are OGG
      on every target.
- [ ] `web.rs`'s module doc and `shuyi-ship` name the formats.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `copy_music` with an OGG, a FLAC, an MP3 and a WAV file; the
  warning's text.
- Integration: the web check plays a FLAC track, if it can run here.

## Completion notes

*(Filled in by the session that completes the ticket.)*
