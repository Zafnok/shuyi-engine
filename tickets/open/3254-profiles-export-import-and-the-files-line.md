---
id: "3254"
title: "The profiles screen exports and imports a profile, says where the files are, and checks the engine's own keys in a file of saves"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3254 — Export and import on the profiles screen

## Context

A request from the game.

1. **The game ticket:** game#1415, which deletes the game's wrapper of
   this engine's profiles screen and its export plumbing once this
   lands
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1415-profile-export-and-import-from-the-engine.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** on the profiles screen, a player saves a
   profile as a file they keep, brings one back in as a new profile, opens
   the folder the saves are in, and reads where that folder is and where
   the last file went.
3. **The friction numbers:** sweep game#1397 (the game's third
   engine-friction sweep, 2026-11-27). The game would keep about 60 lines
   (its file name prefix, its slot word, its own save-slot checks and
   where it opens the import screen). It measures about 330 production
   lines on this job, at the game's pin 230080d: `screens/profile.rs`
   (175 code lines, a wrapper of `ProfilesScreen` that adds the keys, a
   notice and a place line), `screens/files.rs` and `files/glyph.rs` (57,
   the place line cut from the left), and the generic parts of
   `save_files.rs` and `save_files/import.rs` (about 100 of their 589:
   `export_name`/`safe_name`, `ExportNotice`, `file_name`, `exported_of`,
   `name_in`/`with_number`/`capitalized`, `ProfileKeys::entries`,
   `OtherProfile`, and `GameCheck`'s verdicts for `settings`,
   `keybindings`, `layout` and `tips_seen`). About 5.5×.
4. **The generic shape:** this engine has the plumbing
   (`storage::bundle::{write_bundle, plan_import, EntryCheck,
   FormatCheck, Checks, Verdict}`, `Ctx::offer_file`, `Ctx::ask_for_file`,
   `Ctx::open_storage_folder`, `player_files::Offered`, `ProfileStore`,
   `PROFILE_NUMBERED_TEXT`) but no screen uses it, so every game with
   profiles writes the same wrapper:
   - `ProfilesScreen` gains opt-in **export** and **import** actions on a
     profile row and on the "add" row, an **open folder** action where
     the platform has a folder, and a **place line** (the folder, cut
     from the left to fit) in its view.
   - An **export file name** from a game prefix and the profile's name
     made file-safe, with its number, and the reverse: the profile name a
     file name holds (unique among the profiles, numbered when taken).
   - An **export notice**: the file offered, then where the platform
     says it went (`Offered::Written` gives its last path part,
     `Downloaded` the name offered, anything else a failure), cleared
     when the cursor moves.
   - A borrowed **view of another profile's storage**:
     `ProfileStore::of(&mut dyn Storage, ProfileId)` (read, write,
     delete, list with the prefix stripped), and its entries
     (`entries(keep)`, the bundle's reserved keys left out).
   - Ready **bundle checks for the engine's own keys**: `settings`,
     `keybindings` (read against a keymap), `layout` and `tips_seen`, to
     chain with a game's own `EntryCheck`; and `FormatCheck` taking a
     range of versions it reads, not only the current one.
5. **An example from a game that isn't tactics:** a farming game with a
   profile per player in the house lets one of them copy their farm to a
   USB stick and a friend load it on theirs, with the same three keys and
   the same "saved to Documents/My Games/…" line.

## Nick input

None.

## Scope

**In:**
- `screens/profiles.rs`: builders `.exporting(prefix)`,
  `.importing(open)` (a function giving the screen to push, so the game
  keeps its own import screen) and `.with_place()`; the actions declared
  through `ui_actions` only when set; the help line naming them; a
  `place: Option<PlaceView>` in `ProfilesView`; a `heard(Offered)` the
  game's system hook calls (or the screen hears `SystemEvent::FileOffered`
  itself, if the screen stack already routes system events: check
  `Game::hear`).
- `storage/export.rs` (new): `export_name(prefix, name, id, suffix)`,
  `name_in(prefix, file)`, `unique_name(ctx, base)` and
  `ExportNotice`.
- `profiles.rs`: `ProfileStore::of` (a borrowed view) and
  `ProfileStore::entries(keep)`.
- `storage/bundle.rs`: `engine_checks(keymap)` (a `Checks` of the four
  engine keys) and `FormatCheck::reading(range)`.
- New public types are `#[non_exhaustive]` with constructors (ADR-0039);
  every builder is additive. No break to the public API.

**Out (do not do):**
- A save-slot picker or a slot's columns: a game's own (a slot suffix is
  only a string the game passes to `export_name`).
- The import screen's list of what a file holds and where each part goes
  (the game's destinations are its own).
- The Playdate: it has no files a player reaches; the actions stay off
  where `PlatformFacts::storage_place` is unknown.

**Estimated size:** about 900 lines (code 380, tests 420, docs 100).

## Implementation steps

1. `ProfileStore::of` and `entries`, with tests on `MemoryStorage`
   (keys of two profiles, the bundle's reserved keys left out).
2. `storage/export.rs`: the names and the notice, ported from the
   game's `save_files.rs` (`export_name`, `safe_name`, `file_name`,
   `exported_of`, `ExportNotice`) and `save_files/import.rs` (`name_in`,
   `with_number`, `capitalized`, `new_name`), with the game's prefix a
   parameter. Behavior unchanged: `"Odo Kai"`, id 2 gives
   `<prefix>-odo-kai-2.ron`, and back.
3. `engine_checks` and `FormatCheck::reading`, from the game's
   `GameCheck` arms for the four keys and its `versioned`.
4. The screen's builders, actions, help line and place line (cut from
   the left with `…`, as the game's `files/glyph.rs` does).
5. Docs: `skills/shuyi-screen/SKILL.md` (the profiles screen's builders)
   and the book's *Saves and profiles* page.
6. The PR says the game's PR that moves the pin (game#1415) follows it.

## Acceptance criteria

- [ ] `ProfilesScreen::new(ctx).exporting("my-game")` on a desktop
      `Ctx`: the export action on a profile row offers a file named
      `my-game-<name>-<id>.ron` holding that profile's entries and none of
      another's; on a platform with no storage place the action isn't
      declared.
- [ ] `name_in("my-game", "my-game-odo-kai-2 (3).ron") == "Odo Kai"`, and
      `unique_name` numbers it when a profile has that name.
- [ ] `ExportNotice` after `Offered::Written("C:/x/a (2).ron")` shows
      `a (2).ron`; after `Offered::Downloaded`, the name offered; moving
      the cursor clears it.
- [ ] `engine_checks(&keymap)` gives `Ready` for this build's settings,
      key bindings, layout and tips seen, `Incompatible` for a settings
      version past what it reads, `Unreadable` for broken text.
- [ ] The place line is in `ProfilesView` and painted; a snapshot on
      100 × 32.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the names and their reverse, the notice, `ProfileStore::of`,
  the four checks.
- Snapshot / integration: the screen with the place line and the help
  line naming export and import, on the headless harness.

## Completion notes

*(Filled in by the session that completes the ticket.)*
