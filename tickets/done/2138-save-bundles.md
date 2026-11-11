---
id: "2138"
title: "Save bundles: a storage's keys written as one checked text file, read back with a verdict per entry, and applied all or nothing"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-11
---

# 2138 — Save bundles: export a storage's keys as one file, import them with a verdict per entry

## Context

1. **The game tickets:** game#0872 (export saves,
   [link](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0872-export-saves-and-show-where-they-are.md))
   and game#0873 (import saves with a version check,
   [link](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0873-import-saves-with-a-version-check.md)).
   Nick, 2026-11-10: he wants to export his saves to a file and import that
   file again, and on import be told whether the file's saves can be read
   by this build or come from an incompatible version.
2. **The job in one sentence:** "pack some of my saved data into one
   file, and later unpack it, being told before anything is written which
   parts this version of the game can read".
3. **Friction numbers:** no friction review: a missing feature.
4. **The generic shape:** a **bundle** is a set of `Storage` entries
   (key → text) with a header: the bundle format's own version and the
   `Made` stamp of the build that wrote it (2111, `shuyi_core::made`).
   Writing one takes a storage and a key filter. Reading one gives an
   **import plan**: each entry with a verdict the game's formats decide
   (`Ready`, `Incompatible { found, wants }`, `Unreadable`, `Unknown`),
   which keys it would replace, and the stamp's `Mismatch` against this
   build. Applying a plan writes the chosen entries all or nothing.
5. **Another game that would write the same code:** a farming game's
   backup of its farms and settings; a visual novel moving its save slots
   to another computer (2724); a roguelike's "export my unlocks". None of
   it is tactics.

What exists (read on 2026-11-10 at `edb6c888`):

- `shuyi_ui::storage::Storage` (`read`, `write`, `delete`, `list`; keys
  `^[a-z0-9_-]{1,64}$`), and the profile store in
  `crates/ui/src/profiles.rs`, which prefixes a profile's keys.
- `shuyi_core::save::{SaveFormat, SaveHeader}`: a save's version, read
  even when the rest doesn't parse; a build reads only its own version.
- `shuyi_core::made::{Made, Mismatch}` (2111): what made a record.
- 3045 (done): writes that survive a power cut, and the last good copy.

## Nick input

None.

## Scope

**In:**
- A new format, `shuyi_ui::storage::bundle` (or `shuyi_platform` if the
  stamp's type isn't reachable from `ui`; the session decides and says
  why): `SaveBundle { version, made, entries: BTreeMap<String, String> }`
  in RON, with `SAVE_BUNDLE_VERSION = 1`. It follows ADR-0007: a
  `Format` registered for `shuyi import` and `shuyi check`, a fuzz-style
  test, and a golden fixture row in its crate's table (2116).
- `fn write_bundle(storage: &dyn Storage, made: &Made, keep: impl Fn(&str) -> bool)
  -> Result<String, StorageError>`: the entries in key order, so the same
  storage gives the same text.
- `trait EntryCheck { fn check(&self, key: &str, text: &str) -> Verdict; }`
  with `Verdict` (`#[non_exhaustive]`): `Ready`, `Incompatible { found:
  u32, wants: u32 }`, `Unreadable`, `Unknown` (a key the game doesn't
  know). A ready-made checker for one key pattern and one `SaveFormat`
  (`FormatCheck::<F>::new(|key| …)`), built on `SaveHeader`.
- `fn plan_import(text: &str, storage: &dyn Storage, check: &dyn EntryCheck,
  now: &Made) -> Result<ImportPlan, BundleError>`: `BundleError` for text
  that isn't a bundle, a bundle of a newer bundle version, or a key that
  isn't a valid storage key. `ImportPlan` lists each entry's key, verdict
  and whether the storage holds that key already (`replaces`), and the
  stamp's `Mismatch` list.
- `fn apply(plan: &ImportPlan, storage: &mut dyn Storage, take: impl Fn(&PlanEntry) -> bool)
  -> Result<Applied, StorageError>`: writes only `Ready` entries the
  caller takes. All or nothing: it first copies each key it will replace
  to a backup key, writes, and on any failure puts the backups back; then
  deletes the backups. The power-cut suite (3045) gets a case for it.
- **Either order with 2110:** if 2110 is done, `FormatCheck` uses
  `read_save` and adds `Verdict::Migratable { from, to }` for an older
  version inside the chain, and `apply` writes the migrated text; if not,
  nothing to do here (2110 adds `Migratable` to `FormatCheck` and
  `apply`).

**Out (do not do):**
- Getting the file to and from the player (a download, a picker): 3309
  (done) does it: `Ctx::offer_file`, `Ctx::ask_for_file`, and the
  answers `SystemEvent::FileOffered` and `SystemEvent::FileChosen`.
- Which keys a game bundles, its screens and its words: the game's
  (game#0872, game#0873).
- Merging two saves into one: an entry is taken whole or not at all.
- Compression or binary bundles.

## Implementation steps

1. `bundle.rs` (module beside `storage.rs`): the types, `write_bundle`,
   `plan_import`, `apply`. Public types that may grow are
   `#[non_exhaustive]` with constructors (ADR-0039); say so in the
   completion notes.
2. `FormatCheck<F: SaveFormat>`: a key matcher and `SaveHeader<F>` read
   with `ron::from_str`; a header that doesn't parse is `Unreadable`, a
   version other than `F::VERSION` is `Incompatible`, and a text that
   gives the version but doesn't parse as `F` is `Unreadable` (read with a
   `F: DeserializeOwned` bound).
3. A `Checks` list combining several `EntryCheck`s (first match wins;
   none → `Unknown`).
4. Register the format; the fixture `bundle_v1.ron` with two entries; the
   fuzz-style test (any text: `plan_import` never panics).
5. The power-cut case in the suite: a cut at every write of `apply`
   leaves either the old values or the new, per key set, never a mix.
6. Book chapter on saving (2802): a section "Bundles".

Expected size: about 700–900 changed lines with tests.

## Acceptance criteria

- [x] `write_bundle` of the same storage twice gives the same text; the
      fixture round-trips byte for byte.
- [x] `plan_import` of a bundle holding one current save, one of another
      version, one that doesn't parse and one unknown key gives `Ready`,
      `Incompatible { found, wants }`, `Unreadable` and `Unknown`, and
      marks `replaces` for a key the storage holds.
- [x] A bundle written by another engine version or game stamp reports
      the `Mismatch` and still plans (the verdicts decide).
- [x] `apply` writes only taken `Ready` entries; a storage that fails the
      third write leaves every key as it was (a test with a failing
      storage).
- [x] The power-cut case passes.
- [x] ~~`shuyi check` reads a `.ron` bundle file and reports a broken one.~~
      (not done as written: `check` takes no file, it checks the game's
      own content; `shuyi import <file>` is the operation that reads one
      file, and it reads a bundle and reports a broken one with its line,
      as the notes say)
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each verdict, `BundleError` cases, determinism of the text.
- Property: any bundle `write_bundle` makes plans every entry `Ready`
  against a checker that accepts all, and `apply` then gives a storage
  equal to the source's chosen keys.
- Fuzz-style: `plan_import` on arbitrary text never panics.
- Golden fixture row (2116).

## Completion notes

**What was done.**

- `shuyi_ui::storage::bundle` (`crates/ui/src/storage/bundle.rs`). It is
  in `shuyi-ui`, not `shuyi-platform`: `shuyi-ui` already depends on
  `shuyi-core` (`Made`, `SaveFormat`, `SaveHeader`) and `shuyi-assets`
  (`Format`), and `Storage` is its own.
  - `SaveBundle { version, made, entries }`, `SAVE_BUNDLE_VERSION = 1`,
    and `to_text`: RON with its type's names, four spaces, `\n` line
    ends, entries in key order.
  - `write_bundle`, `read_bundle`, `plan_import`, `apply`.
  - `EntryCheck` (a closure `|key, text| …` is one), `Verdict`,
    `FormatCheck::<F>::new(|key| …)` over `SaveHeader`, and `Checks`
    (the first verdict that isn't `Unknown` wins).
  - `ImportPlan` (`made`, `mismatch: Option<Mismatch>`, `entries`,
    with `entry` and `ready`), `PlanEntry` (`key`, `verdict`,
    `replaces`, `text`), `Applied` (`written`, `replaced`),
    `BundleError`.
  - The format `SaveBundleRon` (`save-bundle`, `.ron`, 4 MiB, with
    `Encode`).
- **ADR-0039:** `SaveBundle` is `#[non_exhaustive]` with `SaveBundle::new`;
  `Verdict`, `BundleError`, `PlanEntry`, `ImportPlan` and `Applied` are
  `#[non_exhaustive]` and built by the module only (answers, like
  `Mismatch`), so `Verdict::Migratable` (2110) is no break.
- **All or nothing, and the power cut.** The ticket's steps (back up,
  write, put the backups back on failure, delete the backups) can't
  survive a power cut alone: nothing runs to put the backups back. So
  `apply` writes a journal key after the backups and before the first
  new value, and deletes it once every value is written: that delete is
  the moment the import is done. A new `pub fn recover(storage)` undoes
  an interrupted import from the journal and deletes leftover backups;
  `apply` calls it first, and a game calls it at start. Keys starting
  with `_bundle-` (`RESERVED_PREFIX`) are the bundle's own:
  `write_bundle` leaves them out, `read_bundle` refuses them as
  `BadKey`. A torn journal is harmless: the journal is written before
  any value, so a storage that writes in place keeps the promise too (a
  test says so).
- **The power-cut suite** gained promise 8 (`power_cut.rs`): an import
  cut after every number of operations of the medium reads, once
  `recover` has run after the restart, every old value or every new one,
  and the new ones if `apply` returned `Ok`, with no reserved key left.
  It runs in every backend's rig: the app's fake disk and the Playdate's
  pass.
- **A bundle of another version** is refused by its version alone
  (`SaveHeader<SaveBundle>`, read first), whatever the rest holds:
  `BundleError::Version { found, reads }`, for any version other than
  1, older or newer.
- **Registered:** `Importers::standard()` (`shuyi-cli`) registers
  `SaveBundleRon` beside `Formats::standard()`, as the kind `save
  bundle`, with its text for `--print`. `Formats::standard()` itself is
  `shuyi-assets`', which can't depend on `shuyi-ui`.
- **Tests:** unit tests of each verdict, each `BundleError`, the text's
  determinism and the fixture's byte-for-byte round trip
  (`crates/ui/tests/fixtures/bundle_v1.ron`); a test with a storage
  that fails each of the six writes in turn; `recover`; the format
  through the registry with lines and codes; the property test (any
  bundle plans every entry `Ready` against an accept-all check, and
  `apply` gives the source's chosen keys over what the target held);
  fuzz-style tests on any text, text shaped like a bundle, and any
  bytes; the golden row `save bundle: save-bundle` in the new table
  `crates/ui/tests/golden/bundle.ron` (a `golden` test, so it runs on
  wasm32 in the `determinism` job with the rest of `shuyi-ui`'s).
- **Docs:** the book's chapter 7 has a section *Bundles* with its code
  in `site/snippets/src/saves.rs` (anchors `bundle` and `test_bundle`);
  `docs/formats.md` has the format's row; the skill pack's
  `shuyi-screen` points at bundles beside its files the player keeps.

**Deviations.**

- `shuyi check` takes no file: it checks the game's own content through
  the game's validators. The file check is `shuyi import <file>`, which
  reads a bundle (`save bundle: 1 entry, made with …`) and reports a
  broken one with its file, line and `SH2001` (a test in
  `crates/cli/src/contract/import/tests.rs`). A game whose own content
  holds a bundle (a shipped starting save, say) registers
  `SaveBundleRon` in its `Formats`, and its `check` then reads it.
- `recover`, the journal and the reserved `_bundle-` keys are new (see
  above): the power-cut criterion needs them.
- 2110 is open, so no `Migratable` here. 2110's clause is rewritten as
  what holds: it adds the variant to `FormatCheck` and `apply`.

**For the game.** game#0872 and game#0873 use this after the game's
next pin bump: `write_bundle` with the keys it picks, handed to the
player with `Ctx::offer_file`; `plan_import` on the `FileChosen` text
with a `FormatCheck` per save format; `apply` on the player's yes; and
`bundle::recover` at start. The game PR that moves the pin is written
by those game tickets.

No break: everything is new. The golden table is new, so its row is
not a break.

Nick input: none.
