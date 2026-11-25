---
id: "2279"
title: "A sound effect's text has LF line ends on every platform: shuyi sfx --out writes no CRLF on Windows"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
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
contract's `text` field carries the same mix: the MCP `sfx` tool gave
`"// The engine's preset `hit` (`shuyi sfx hit`).\n(\r\n    wave: Noise,\r\n…`
on Nick's machine. The repository keeps text files LF (`.gitattributes`:
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

- [ ] `sfx::to_text` writes no `\r`, on Windows too (the test runs in
      CI's Windows job and on Nick's machine).
- [ ] `shuyi sfx <preset> --out <file>` writes a file with LF line ends
      only.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `to_text` of each preset in `sfx::PRESETS` holds no `\r` and
  reads back to the same `SfxDef`.
- Integration: the contract's `sfx` with `out` writes a file with no
  `\r`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
