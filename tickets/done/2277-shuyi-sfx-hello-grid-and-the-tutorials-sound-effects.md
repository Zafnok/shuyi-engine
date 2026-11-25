---
id: "2277"
title: "shuyi sfx, and Hello, Grid's sounds as .sfx.ron files"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2225"]
nick_input: none
completed: 2026-11-25
---

# 2277 — `shuyi sfx`, and Hello, Grid's sounds as `.sfx.ron` files

## Context

Split from 2225 at its own split point (its steps 7 and 8), with the
two pieces of its step 9 that were written around `shuyi sfx`.

- **From 2225 (done):** `.sfx.ron` sound effects are a format of
  `shuyi-assets` behind the `sfx` feature (ADR-0144):
  `shuyi_assets::sfx` (`SfxDef`, `from_source`, `render`, `to_wav`,
  `to_text`, `problems`, the presets `PRESETS`, `preset(name, seed)` and
  `preset_source(name)`), the formats `SfxDefRon` (with `Encode`) and
  `SfxRon`, `audio::playable(name, bytes)` for a sound hook, and
  `audio::decode_to_wav` and `audio::format_problem` reading a
  `.sfx.ron`. `shuyi-cli` turns `sfx` on. The templates' menu sounds
  are four `.sfx.ron` files and an `audio.ron` in
  `templates/shuyi-default/base/crates/game/files/`, read by
  `BasicData::load` (`AUDIO_RON`) and played through `run_game`'s
  sound reader. Docs: `docs/formats.md`, *Sound effects (`.sfx.ron`)*.
- **What is left:**
  - Hello, Grid still writes its blips by hand: a WAV header and float
    sines with four `#[allow]` casts (`examples/hello-grid/src/sound.rs`).
  - An agent has no command to start a sound from a preset, nor to
    render one to a WAV a person can listen to.
  - The tutorial's box pusher (`site/tutorial/08-polish`, `09-test`,
    `10-ship`, ticket 2803) has no sound: its menus already play the
    widgets' cues (`Menu::handle_with_sound` in `levels.rs`), which a
    manifest without them leaves silent, and a push or a win plays
    nothing.
  - The engine's `shuyi-new-game` skill (`skills/shuyi-new-game/SKILL.md`,
    ticket 1134) says nothing of sounds.

## Nick input

None.

## Scope

**In:**

- **Hello, Grid** (`examples/hello-grid`): its four blips become
  `files/sfx/move.sfx.ron`, `select.sfx.ron`, `cancel.sfx.ron` and
  `cursor.sfx.ron` (near today's pitches and lengths: 660, 880, 440 and
  990 hz; 50, 80, 80 and 30 ms). `src/sound.rs` keeps `manifest()`
  (its names become the `.sfx.ron` ones, its cue rules unchanged) and
  `files()` over the bundle through `audio::playable`. The float `wav`
  function, its `#[allow]`s and its test go. Its `Cargo.toml` turns on
  `shuyi-assets`' `sfx` (its dev-dependency line already names the
  crate; the library needs a dependency line of its own, as the
  templates' game crate has, and a `cargo-machete` ignore if no code
  names it). `check --baked` still holds: the pack keeps a `.sfx.ron`
  as its own bytes.
- **`shuyi sfx`**, a contract op that needs no game (like `doctor`), in
  `crates/cli/src/contract/sfx.rs`:
  - `shuyi sfx <preset | file.sfx.ron> [--seed N] [--out <file.sfx.ron>]
    [--wav <file.wav>] [--json]`;
  - `SfxInput { source, seed, out, wav }` (ADR-0039 kind 4, `Default`)
    and `SfxOutput { presets, text, length_ms, samples, peak, problems,
    written }`: the preset's or the file's text (`sfx::to_text`), its
    length, its sample count, its loudest sample, and the files written.
    `--wav` writes the render (`sfx::to_wav(&sfx::render(..))`) so a
    person can listen;
  - a file with problems answers `ok: false` with them (file, line,
    message); an unknown name answers an error that lists the presets
    (`sfx::PRESETS`);
  - `Op::Sfx` in `crates/cli/src/contract.rs` (`Op::ALL`, `name`, the
    schema pair in `schemas()`); `without_project` in
    `crates/cli/src/contract/cli.rs` runs it as it runs `doctor`, and
    its tests name it among the ops without a project;
    `sfx.input.json` and `sfx.output.json` under `crates/cli/schemas/`,
    `contract::VERSION` raised by one and its line in `versions.json`
    (`docs/contract.md`, *The schemas*);
  - MCP: `shuyi mcp` (1133) makes its tools from `Op::ALL`: its arm in
    `mcp::host::one_off` and in `mcp::description` (the compiler asks
    for both); it runs no game, so it is not in
    `contract::cli::runs_the_game`;
  - one row in the template's `AGENTS.md` command table
    (`templates/shuyi-default/base/AGENTS.md`): `cargo shuyi sfx coin
    --out crates/game/files/sfx/coin.sfx.ron`;
  - `docs/contract.md`: the `sfx` row in the table of operations.
- **The tutorial** (from 2803): `site/tutorial/08-polish`, and `09-test`
  and `10-ship` after it, get the four menu sounds as the templates have
  them (`files/audio.ron`, `files/sfx/*.sfx.ron`, the `sfx` feature of
  `shuyi-assets` in the game crate's `Cargo.toml`), and two sounds made
  with `shuyi sfx`: one on a push (the rules' `Event::Pushed`) and one on
  a win, each a cue of `audio.ron` the board plays through `ctx.audio`.
  `site/content/learn/book/box-pusher/08-polish.md` gets its part on
  them: what a `.sfx.ron` is, `cargo shuyi sfx coin --out …`, `--wav`
  to listen, and which fields to change. `site/tutorial/tries.toml`'s
  pictures don't change (sound isn't drawn). The steps' `Cargo.lock`
  files follow. **Either order with 2850:** if 2850 is done, steps
  08–10 already have the four menu sounds: add only the push and win
  sounds; if not, add the menu sounds to 08–10 as above (2850 brings
  the other steps up to them).
- **The `shuyi-new-game` skill** (from 1134): a short section on sound
  effects: a cue of `files/audio.ron` names a `.sfx.ron` file, `cargo
  shuyi sfx <preset> --out <file>` writes one, `--wav` renders it to
  listen to, and which fields to change (`docs/formats.md`'s table).
  `cargo xtask check-skills` passes.

**Out (do not do):**

- anything of the format itself, its presets or its golden rows (2225);
- an editor or a player for sounds, and playing a sound from the CLI;
- music, sequences, a live synth (2205, 2206, 3030).

**Estimated size:** about 1,400 lines (code 450, tests 450, docs 200,
data 300 with the schemas, which `pr-size` leaves out).

**Split:** past 1,000 lines, split off the tutorial's sounds and its
chapter into a new ticket blocked by this one.

## Implementation steps

1. Read ADR-0144, `crates/assets/src/sfx.rs`, `docs/formats.md`'s
   *Sound effects*, `crates/cli/src/contract/doctor.rs` (an op without a
   project) and `docs/contract.md`, *The schemas*.
2. Move Hello, Grid onto `.sfx.ron` files: its files, `src/sound.rs`,
   its `Cargo.toml`, its tests. Run its web build and `check --baked`.
3. Write `shuyi sfx`: the op, its arguments, its schemas, the version,
   the MCP arms, the `AGENTS.md` row, and its contract test.
4. Give the tutorial's three steps their sounds, and write the chapter's
   part. Run each step's tests (`cargo xtask tutorial` or the step's own
   `cargo test`, as `site/tutorial`'s README says).
5. Write the skill's section.
6. Run the gates.

## Acceptance criteria

- [x] Hello, Grid's `src/sound.rs` has no float and no `#[allow]`; its
      tests, `check --baked` and its web build pass.
- [x] `shuyi sfx coin --out x.sfx.ron --wav x.wav --json` matches its
      schema and writes both files; `x.sfx.ron` reads back to the same
      `SfxDef`; `shuyi sfx nope` answers an error that lists the
      presets; a broken file answers `ok: false` with its line.
- [x] `shuyi mcp` lists an `sfx` tool.
- [x] ~~The tutorial's steps 08 to 10 play a sound on a push and on a win,
      and their menus' cues: each step's test checks the cues are in its
      manifest and `format_problem` accepts what `audio::playable` gives
      for each file.~~ (not done: split off to 2853 at the ticket's split point)
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] `cargo xtask check-skills` passes with the skill's section.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `parse_args` of `shuyi sfx`.
- Integration: a contract test of `shuyi sfx` (a preset, a seed, a file,
  a broken file, an unknown name, `--out` and `--wav`); the schemas
  test; Hello, Grid's tests (its manifest's files play); the tutorial's
  sound test in each step.

## Completion notes

- **Hello, Grid** plays four `.sfx.ron` files
  (`examples/hello-grid/files/sfx/`: sine blips at 660, 880, 440 and
  990 hz, 50, 80, 80 and 30 ms long, as before). `src/sound.rs` keeps
  `manifest()` (its cues name `sfx/*.sfx.ron`, listed in `SOUNDS`, which
  replaces `BLIPS`) and `files()`, now the game's own files through
  `audio::playable`, shown as `files/<name>`. The float `wav`, its four
  `#[allow]`s and its test are gone; a test checks each file's pitch and
  length. Its `Cargo.toml` names `shuyi-assets` with `sfx` (a
  `cargo-machete` ignore: no code names the crate). The pack now holds 15
  files (the four kept as their own bytes); its table's `fingerprint
  files` row and the golden session's content stamp moved with the
  example's own files, which `shuyi_assets::golden` says needs no line.
  `check --baked` passes; the web build ran in `cargo xtask gates`.
- **`shuyi sfx`** (`crates/cli/src/contract/sfx.rs`): `SfxInput { source,
  seed, out, wav }` and `SfxOutput { presets, text, length_ms, samples,
  peak, problems, written }`. A source ending in `.ron` is a file, from
  the working folder; anything else a preset. A preset's text starts
  with a comment line naming it (and its seed), so the file a game keeps
  says where it came from. `--out` and `--wav` make their folder. Choices
  the ticket left open: `--seed` with a file is refused (`SH0001`: a
  file's own `seed` field is its noise's); a file that can't be read or
  written is `SH3004`; a file with problems answers `ok: false` with no
  code, its own text, and writes nothing. Contract version 60. MCP's
  `sfx` tool runs in the session's folder and takes no `project`.
- **Docs**: `docs/contract.md`'s row, the crate's table, the CLI's usage,
  the root README's `shuyi-cli` row, the template's `AGENTS.md` row,
  `docs/formats.md`'s presets bullet, the `shuyi-new-game` skill's
  *Sounds*, and the book: chapter 10's list of operations and chapter
  3's passage on sound effects (how to start one with `sfx`).
- **Split:** at 757 counted lines before the tutorial, whose three steps
  and chapter part were estimated at about 450 more, past the ticket's
  split point (1,000). The tutorial's push and win sounds and chapter
  08's part are **2853** (blocked by this one), and 2850's clause now
  names 2853.
- **Deviations:** the ticket's `cargo xtask check-skills` is `cargo shuyi
  check-skills skills` (and the `it` test `skills`); no xtask of that
  name exists. 2850 is still open, so the *either order* clause about it
  moved to 2853 with the tutorial.
- No break. The game builds no contract type by literal and matches no
  `Op` (`gh search code --repo Zafnok/visions-of-shuyi 'Op::'` and
  `'shuyi_cli::contract'` at pin `b69585e`: imports of `autobalance`,
  `playtest`, `replay`, `session` and `Answer` only); `Op` is
  `#[non_exhaustive]`, so `Op::Sfx` is an addition.

Behaviour: shuyi-cli: `Op::ALL` has `sfx` between `doctor` and `explain`, `shuyi mcp` lists an `sfx` tool, and every answer says `"contract": 60`.
