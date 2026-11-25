---
id: "3428"
title: "The skill pack catches up: the picture and sound formats, the audio manifest, the log file, a target's scale and the bots' seed"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3428 — The skill pack catches up: the picture and sound formats, the audio manifest, the log file, a target's scale and the bots' seed

## Context

Found by the docs audit of 2026-11-25. The skill pack (`skills/`) is
what an agent making a game is taught, and the docs map's row says it
changes with "an operation, a field, a problem code, a template, a
format, a rule a game keeps". Most skills kept up with the last 60 PRs
(`shuyi-new-game`, `shuyi-stage-grid`'s *Luck*, `shuyi-animate`,
`shuyi-input`). These didn't:

- **`shuyi-import-art`'s table of kinds** (`skills/shuyi-import-art/SKILL.md`,
  about lines 40 and 44) lists a picture as `.png` and sounds and music
  as `.wav` and `.ogg`. 2203 and 2234 added JPEG, WebP, QOI and JPEG XL
  pictures and FLAC and MP3 sounds and tracks, each behind a
  `shuyi-assets` feature; 2225 added `.sfx.ron` sound effects (the `sfx`
  feature, which the template turns on). Its step 3 (about line 69) says
  a short-path game reads "its text, keys, palette and font itself";
  since 2225 `BasicData::load` reads `files/audio.ron` too.
- **The audio manifest's newer fields** (`docs/formats.md`): cue rules
  (2345: `pick`, `volume_min`, `min_gap_ms`), music's intro and loop
  regions (2397: `loop_start_ms`, `loop_end_ms`), and the plays the cue
  rules dropped in `run`'s answer and `Harness::dropped_sounds` (3419).
  No skill names them; they are in `formats.md` and the book only.
- **The log file**: 3041 made shipped builds write a log file (the last
  three runs kept, the panic's message in it), and 3324 the `SHUYI_LOG`
  filter on a desktop. `shuyi-debug-replay` (about lines 120–136)
  teaches crash reports and "the log's last lines", but not where the
  log file is nor how to ask a player for it, nor `SHUYI_LOG`.
- **A target's scale** (2333): `shuyi-ship` (about line 178) says a
  target has "its scale (`integer` or `fit`)", but not that the window
  draws the game's `GameSettings::DEFAULT_SCALE` and the target's scale
  should match it, as the template's `shuyi.ron` says.
- **`shuyi-balance-bots`** (2133): it doesn't name
  `shuyi_core::bots::playtest::bot_seed`, the seed of an entrant's bots
  that the template uses, and its personas paragraph (about line 211)
  names only `UnknownFeature`, though the settings
  (`RolloutSettings`, `PlanSettings`, so a persona's `settings`) now
  refuse unknown fields.
- **The template's `AGENTS.md`** (`templates/shuyi-default/base/AGENTS.md`,
  *The short path*, about lines 116–117) says `BasicData` loads "the
  text, the keys, the palette and the font from `files/`"; it loads the
  sounds from `files/audio.ron` too.

## Nick input

None.

## Scope

**In:**

- `shuyi-import-art`: the picture and sound rows name every format, and
  which feature of `shuyi-assets` each needs (one link to
  `docs/formats.md`'s table, not a copy of the sizes); a `.sfx.ron` row
  or line; step 3 names `audio.ron`.
- The audio manifest: a short section in the skill that teaches adding a
  sound (`shuyi-import-art`, unless the session finds a better home in
  the pack, said in the notes): a cue, its rules, a track's loop region,
  and how a test sees a dropped play (`Harness::dropped_sounds`, `run`'s
  `dropped`). Its worked example (`example.ron`) gets a step if the
  skill's example can show one; `cargo xtask check-skills` runs it.
- `shuyi-debug-replay`: where a shipped build's log file is (one link
  to the doc that says it per OS: `docs/crash-reports.md` or
  `docs/platform-backends.md`), that it keeps the last three runs, and
  `SHUYI_LOG` to filter it while debugging on a desktop.
- `shuyi-ship`: the target's scale matches `DEFAULT_SCALE`, in a line.
- `shuyi-balance-bots`: `bot_seed`, and the unknown-field error.
- The template's `AGENTS.md` line.

**Out (do not do):**

- the `shuyi-new-game` skill's sound section and the template's `sfx`
  row (2277);
- `shuyi-ship`'s line on the web's music formats (3352);
- the progress card (`Progress::Of`, `Progress::Wait`) and range tints,
  which no skill covered before: a ticket if the session thinks one
  should.

## Implementation steps

1. Read `skills/README.md` and each skill above, `docs/formats.md`'s
   audio sections, `docs/crash-reports.md`, and the rustdoc of
   `BasicData::load`, `bot_seed` and `Harness::dropped_sounds`.
2. Edit each skill, and the template's line.
3. `cargo xtask check-skills` (the worked examples on a new game).
4. Run the gates.

## Acceptance criteria

- [ ] `shuyi-import-art` names every picture and sound format the engine
      reads, with its feature, and `.sfx.ron`.
- [ ] A skill teaches the audio manifest's cue rules, loop regions and
      dropped plays.
- [ ] `shuyi-debug-replay` names the log file and `SHUYI_LOG`.
- [ ] `shuyi-ship` and `shuyi-balance-bots` say the facts above.
- [ ] The template's `AGENTS.md` names `audio.ron`.
- [ ] `cargo xtask check-skills` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `cargo xtask check-skills`; the template's tests (`cargo xtask
  template-lines --check` if the template's text counts).

## Completion notes

*(Filled in by the session that completes the ticket.)*
