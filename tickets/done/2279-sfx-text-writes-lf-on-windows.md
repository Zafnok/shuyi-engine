---
id: "2279"
title: "A sound effect's text has LF line ends on every platform: shuyi sfx --out writes no CRLF on Windows"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-25
---

# 2279 — A sound effect's text has LF line ends on every platform: `shuyi sfx --out` writes no CRLF on Windows

## Context

Found while working 2853 (the book's box pusher's push and win sounds).
`shuyi_assets::sfx::to_text` (`crates/assets/src/sfx.rs`) writes a
`SfxDef` with `PrettyConfig::new()` (`ron`), whose `new_line` is
`"\r\n"` when the crate is built for Windows. So on Windows,
`shuyi sfx coin --out crates/game/files/sfx/coin.sfx.ron` (2277,
`crates/cli/src/contract/sfx.rs`) writes a file whose first line, the
one naming the preset, ends in `\n` and every other in `\r\n`, and the
contract's `text` field carries the same mix: on Nick's machine the
MCP `sfx` tool's `text` for `hit` had `\n` after its first line and
`\r\n` after each other. The repository keeps text files LF (`.gitattributes`:
`* text=auto eol=lf`), the engine's other RON writers set
`.new_line("\n")` (`crates/ui/src/storage/bundle.rs`,
`crates/platform/src/session.rs`, `crates/cli/src/font_atlas.rs`), and a
game made on Windows then commits a file whose bytes differ from the same
command's on Linux. RON reads either, so the sound is the same: the bug
is the bytes.

## Nick input

None.

## Scope

**In:**

- `sfx::to_text` sets `.new_line("\n")`, so its text is the same bytes
  on every platform.
- A test in `crates/assets` that `to_text` of a preset's `SfxDef` holds
  no `\r`, and one in `crates/cli/src/contract/sfx.rs` that `--out`'s
  file holds none.

**Out (do not do):**

- The format, its presets or the render (2225, 2277).
- Other RON writers of the engine: a search finds those that matter set
  `new_line` already.

## Implementation steps

1. `crates/assets/src/sfx.rs`, `to_text`: `PrettyConfig::new()
   .new_line("\n")…`.
2. The two tests.
3. Run the gates.

## Acceptance criteria

- [x] `sfx::to_text` writes no `\r`, on Windows too (the test runs in
      CI's Windows job and on Nick's machine).
- [x] `shuyi sfx <preset> --out <file>` writes a file with LF line ends
      only.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `to_text` of each preset in `sfx::PRESETS` holds no `\r` and
  reads back to the same `SfxDef`.
- Integration: the contract's `sfx` with `out` writes a file with no
  `\r`.

## Completion notes

- `sfx::to_text` (`crates/assets/src/sfx.rs`) sets
  `PrettyConfig::new().new_line("\n")`, and its doc says its lines end in
  `\n` on every platform.
- `the_text_of_each_preset_ends_its_lines_in_lf_alone_on_every_platform`
  (`crates/assets/src/sfx/tests.rs`): each preset of `PRESETS`, through
  `to_text`, holds no `\r`, has more than one line and reads back to the
  same `SfxDef`. The contract's
  `out_and_wav_write_the_text_and_the_sound_making_their_folder`
  (`crates/cli/src/contract/sfx/tests.rs`) now also holds that the
  `--out` file has no `\r`, rather than a new test repeating its setup.
  On this Windows machine both failed with the fix taken out, and pass
  with it.
- The roadmap's row for 2279 is marked done.
- Deviations: none.

Behaviour: shuyi-assets, shuyi-cli: on Windows, `sfx::to_text`, the
contract's `sfx` `text` and the file `shuyi sfx --out` writes end every
line in `\n`, where they ended all but a preset's first line in `\r\n`.
Elsewhere nothing changes. RON reads both, so the sound is the same.
Searched the game (`gh search code --repo Zafnok/visions-of-shuyi
'sfx.ron'`, `'to_text sfx'`): it names no `.sfx.ron` and doesn't call
`to_text`.

Book: none, the public API keeps its shape, and chapter 08's `shuyi sfx`
steps read the same: only the line ends on Windows change.

Skills: none, the `.sfx.ron` format, its schema and the problem codes
are the same; only the bytes of a line end written on Windows change.

Game: nothing to adopt, the game makes no `.sfx.ron` and calls no `sfx`
writer.
