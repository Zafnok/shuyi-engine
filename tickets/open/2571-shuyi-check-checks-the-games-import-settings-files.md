---
id: "2571"
title: "shuyi check checks the game's import settings files, through the bundle and formats its bake names"
type: feature
milestone: After MVP
model: implementation
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2571 — `shuyi check` checks the game's import settings files

## Context

Ticket 2229 (done) added import settings
([ADR-0057](../../docs/adr/0057-import-settings-a-meta-file-beside-a-file-and-folder-defaults-typed-sections-the-nearest-wins.md)):
a `<file>.meta` beside a file and a `_folder.meta` per folder, RON
structs of typed sections a game registers in its `Formats`
(`Formats::register_import_settings`). `Formats::check_import_settings`
(`crates/assets/src/format.rs`) finds every problem of every settings
file of a bundle: bad RON, an unknown section, an unknown field or a
bad value, a `<name>.meta` whose file is missing, a settings file of a
settings file.

**From 3007 (done):** `Formats::register_standard` registers the
engine's first section, `onebit` (`shuyi_assets::bitmap::dither::OneBitSettings`),
so `Formats::standard().check_import_settings` already finds a typo in
it with its settings file and line, with no Playdate bake running. `shuyi
check` running that check on a game's bundle is this ticket's.

`Bake::bake` (`crates/assets/src/pack/bake.rs`) already runs it, so
`shuyi bake` and `shuyi check --baked` refuse a bad settings file. Plain
`shuyi check` doesn't: it runs only `GameTools::check`. ADR-0057 §7 and
the end goal (`docs/decisions.md`, 2026-10-19: "`shuyi check` catches a
typo in a settings file even when no Playdate bake runs") want it there.
2229's scope had this part, and its **Size** rule moved it here: that PR
passed 600 changed lines without it.

The contract's `check` is `check` in `crates/cli/src/contract/check.rs`.
`GameTools::bake(target)` (`crates/cli/src/game.rs`) gives a
`GameBake { bake, bundle, formats }`: the bundle and the formats the
game reads its sources with. Its default method's error is
`unsupported("bake and check --baked", "bake")`. The fake game
(`crates/cli/src/contract/fake.rs`) offers `bake` over its `sources`
(`SOURCES` by default), with `Formats::standard()`, which registers no
section.

## Nick input

None.

## Scope

**In:**

- **`check`** (`crates/cli/src/contract/check.rs`), without `--target`:
  - after `GameTools::check`, call `GameTools::bake(None)`; when it
    gives a `GameBake`, add `formats.check_import_settings(&bundle)`'s
    errors to the answer's;
  - when it gives an error, go on without them, and push to the answer's
    `limits` the line `"settings files weren't checked: "` followed by
    that error;
  - with `--baked`, `bake::check_baked`'s errors come after, but an error
    equal to one already listed is dropped: `Bake::bake` runs the same
    check, so each settings error is reported once.
- **`docs/contract.md`:** the row for `bake` in the `GameTools` table
  says `check` uses it too, for the settings files.
- No public type changes: `CheckOutput` and `CheckInput` stay as they
  are.

**Out (do not do):**

- Any change to `Formats::check_import_settings` or `Bake::bake`.
- The templates offering `GameTools::bake`, so `shuyi check` covers
  their settings files: a follow-up when a template bakes.
- Sections and their schemas in `shuyi kinds` (2515 leaves them out), and
  `shuyi import` showing the settings a file is read with.
- `check --target`: unchanged.

## Implementation steps

1. Read ADR-0057 §7, `check` and `bake::check_baked` in
   `crates/cli/src/contract/`, `GameTools::bake` and `GameBake` in
   `crates/cli/src/game.rs`, and the fake's `bake` in `fake.rs`.
2. In `check`, after `game.check()`, run the settings check as Scope
   says, then `--baked`'s, skipping an error already listed.
3. Write the tests below in `check.rs`'s `tests`, with copies of
   `fake::SOURCES` that add a settings file.
4. Update `docs/contract.md`'s `bake` row (the `document` skill).
5. Run the gates.

## Acceptance criteria

- [ ] `check` reports a bad settings file of the fake game's bundle (a
      `data/names.ron.meta` holding `(nope: ())`: an unknown section), at
      its line.
- [ ] `check --baked` reports that error once.
- [ ] `check` on a fake without `bake` (`sources: None`) passes, and its
      `limits` hold a line starting `settings files weren't checked: `.
- [ ] A fake whose sources have no settings file answers as before.
- [ ] `docs/contract.md`'s `bake` row says `check` uses it.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/cli/src/contract/check.rs`): the three cases of the
  acceptance criteria through `fake::Fake`, and one whose sources'
  `.meta` names a missing file (`gone.png.meta`).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
