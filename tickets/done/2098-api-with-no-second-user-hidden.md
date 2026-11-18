---
id: "2098"
title: Before the first release, public API with no second user is hidden or moved out of the engine crates
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-18
---

# 2098 — Before the first release, public API with no second user is hidden or moved out of the engine crates

## Context

Nick's decision of 2026-11-14 (`docs/decisions.md`, "no run of breaking
versions after the MVP") wants every break that a release can't soften
done before 2005. A public item is a promise under semver from 0.1 on.
Several PRs since have made items public whose only caller is another
engine crate, or the book's site. Each would need a major version to
change after the release. The retro of PRs #379–#484 found these:

| Item | Where | Added by | Its callers |
| ---- | ----- | -------- | ----------- |
| `recipient` (`Code`, `CodeError`, `FieldRead`, `FIELD*`, `CODE_BYTES`, `field_of`, `field_code`, `slot_code`, `find_fields`, `read_fields`, `write_field`) | `crates/assets/src/lib.rs:68`, `crates/assets/src/recipient.rs` | #417 (3302) | `shuyi-app`'s `recipient.rs:12`, `shuyi-cli`'s `export/recipient.rs`, `pack.rs` |
| `image::mark` (`mark`, `read`, `capacity`, `palette_count`, `MarkError`, `PictureRead`, `BITS_PER_COLOR`, `MIN_CARRIERS`) | `crates/assets/src/image.rs:25`, `image/mark.rs` | #417 | `shuyi-cli`'s `export/recipient.rs` only |
| `pack::Raw` (8 methods), `pack::PackMark` | `crates/assets/src/pack.rs:949,1029` | #417 | `shuyi-cli`'s `export/recipient.rs` only |
| `export::recipient` (11 items) and `export::recipient::ledger` (15 items) | `crates/cli/src/export.rs:63`, `export/recipient.rs:27` | #417 | `contract::export`, in the same crate |
| `run_game_checked`, `PageCheck` | `crates/app/src/lib.rs:771,780` | #410 (2828) | `site/exercises/main.rs` and each `site/exercises/<step>.rs` |
| `page::page_check`, `page::CheckReport`, `page::report_check` | `crates/app/src/page.rs:178,203,228` | #410 | `run_game_checked` only |
| `contract::doctor::FILE_ATTRIBUTE_COMPRESSED` | `crates/cli/src/contract/doctor.rs:110` | #435 (3402) | `doctor.rs:146` only |
| `skills::WITHOUT_EXAMPLES` | `crates/cli/src/skills.rs:132` | #406 (2597) | `skills.rs` and `template/tests.rs` only |

That is 23 items and 14 methods in `shuyi-assets` from one PR.
`PageCheck` is a tuple alias, `(&'static str, fn(&BasicData<A, T>) ->
Result<String, String>)`, so it can't take `#[non_exhaustive]` and can't
grow (ADR-0039). [ADR-0117](../../docs/adr/0117-exercises-checks-spliced-into-a-copy-of-each-step-and-run-by-its-web-build.md)
made `run_game_checked` public on purpose ("any game on the short path
with a page can have checks"). No game has asked for that. A reader
never sees the exercises' `main.rs` (ADR-0117 §2). `pub mod page` itself
stays public: `examples/hello-grid/src/main.rs:10` uses `page_layer` and
`PageLimits` (3069).

`WITHOUT_EXAMPLES` is also a marker in games' files. `shuyi new
--no-examples` writes the sentence into each skill's `SKILL.md`, and
`Skill::left_out_examples` finds it with `contains`. Rewording it would
make every such game's `check` ask for examples again. No test pins its
text today: both tests read the const (`skills/tests.rs:512`,
`template/tests.rs:1135`).

#417 also made `pack::write` copy every raw file. Before it,
`write_first` kept a stored file's bytes as `Cow::Borrowed`. Now
`RawFile::kept` (`pack.rs:930`) does `bytes.to_vec()` for every stored
file, and the copies live until the pack is assembled. Stored files are
the ones lz4 can't shrink: pictures and sounds, most of a pack's bytes.

`#[doc(hidden)]` items are left out by both `cargo-public-api` and
`cargo-semver-checks`, and their doc says semver doesn't cover them
([ADR-0119](../../docs/adr/0119-the-outside-crates-the-public-api-may-name.md) §2;
`shuyi_assets::__embed` and `shuyi_app::open_window` are hidden this
way). The API gate is [ADR-0038](../../docs/adr/0038-the-public-api-gate-a-listing-diff-and-semver-checks-on-every-pr-and-a-breaking-line-for-each-break.md).

**The game uses none of them** (pin `e738ffc2`, searched 2026-11-17):
`gh search code --repo Zafnok/visions-of-shuyi` for `PackMark`,
`shuyi_assets::recipient`, `image::mark`, `Raw::read`,
`export::recipient`, `FILE_ATTRIBUTE_COMPRESSED` and `WITHOUT_EXAMPLES`
finds nothing. `run_game_checked` and `PageCheck` appear only in
`docs/engine/pins.md`, a pin review's notes. The game's
`crates/xtask/src/playdate_audio.rs` uses `shuyi_cli::export::playdate`,
which stays. Game ticket 1272 (copies marked per recipient) may call
`shuyi_app::recipient()`, which stays public.

Parked 3083 names `shuyi_assets::recipient::Code`. That path still works
once hidden, so 3083 needs no change. 2838 (After MVP) runs
`run_game_checked` from the site, which still works hidden.

## Nick input

None.

## Scope

**In:**
- **Hidden** (`#[doc(hidden)]`), because another engine crate calls
  them: `shuyi_assets::recipient` (the whole module: `shuyi-app` and
  `shuyi-cli` both read it), `pack::Raw` and `pack::PackMark` (they need
  the pack's private index). Each one's doc says it is the engine's own
  and semver doesn't cover it (ADR-0119 §2).
- **Hidden**, because only the book's site calls them:
  `run_game_checked`, `PageCheck`, `page::page_check`,
  `page::CheckReport` and `page::report_check`. A game that asks for
  checks gets a public shape then, in its own ticket, as a struct under
  ADR-0039 rather than the tuple alias. That is additive, a minor
  version. ADR-0117's line saying `run_game_checked` is public is
  amended to say it is hidden and why.
- **Moved to `shuyi-cli`**: `image::mark` goes to
  `crates/cli/src/export/recipient/mark.rs`, with its tests. It reads
  only `Image`'s public fields and `recipient::Code`. It becomes private
  to `shuyi-cli`.
- **`pub(crate)`**: `shuyi_cli::export::recipient` and its `ledger`
  module, and `contract::doctor::FILE_ATTRIBUTE_COMPRESSED`. The doctor
  constant becomes a private `const`.
- **`WITHOUT_EXAMPLES`**: `pub(crate)`, and a test that pins its text
  as a literal. Its doc says games' files hold it, so changing it needs
  `left_out_examples` to read the old sentence too, and a `Behaviour:`
  line.
- **`pack::write` borrows raw files again.** `RawFile` gains a lifetime,
  with `kept: Cow<'a, [u8]>`. A stored file from `Content::Raw` is
  borrowed; a baked file, a compressed one and every file `Raw::read`
  reads are owned. The bytes a pack holds don't change, and neither does
  `pack::VERSION` (19).
- One `Breaking:` line for each of `shuyi-assets`, `shuyi-app` and
  `shuyi-cli`. Each has a `migration` block saying a game changes
  nothing, since no game uses these items (the search above).
  cargo-semver-checks counts an item that became `#[doc(hidden)]` as a
  break.
- ADR-0118's paragraphs naming `shuyi_assets::image::mark`, and its
  other items, say where each lives now and that they are hidden.

**Out (do not do):**
- The contract's types that #417 added (`RecipientCopy`, `MarkKind`,
  `FoundMark`, `Given`, `RecipientFound`, `contract::export::detect`):
  they are the `export` operation's answer, and stay public.
- `shuyi_app::recipient()`: a game reads its copy's code with it
  (ADR-0118), so it stays public.
- `pub mod page`, `page_layer`, `report`, `PageLimits` and `PageError`:
  hello-grid uses them (3069).
- Replacing the `WITHOUT_EXAMPLES` sentence with a frontmatter field.
  That changes a format games' files hold, for no gain before a second
  marker exists.
- Flagging new items of this kind as PRs add them. That is 2096's (the
  `api` check classifies new public types).
- `#[non_exhaustive]` on `FontAtlasDef`, `Sound` and `DoctorInput`:
  2095's.
- Moving the recipient overlay's logic, or changing its marks or the
  ledger's format.

**Estimated size:** about 600 lines (code 250, tests 200, docs 150), the
move of `image::mark` and its tests (about 380 lines) counted once if
`cargo xtask pr-size` sees it as a rename.

## Implementation steps

1. **Pin the pack's bytes first.** Before changing `pack.rs`, add a test
   in `crates/assets/src/pack/tests.rs` that writes a pack holding a
   stored raw file (incompressible bytes), a compressed raw file (a run
   of one byte) and a baked file. It compares the result with an
   `insta` snapshot taken on `main`. Commit the snapshot unchanged.
2. `pack.rs`: `struct RawFile<'a>` with `kept: Cow<'a, [u8]>`.
   `RawFile::kept` takes a `Cow<'a, [u8]>` and keeps it as it is when
   stored. `write_first` passes `Cow::Borrowed(bytes)` for
   `Content::Raw` and `Cow::Owned(baked_file(..))` for `Content::Baked`.
   `Raw` holds `Vec<RawFile<'static>>`, and `Raw::read` and
   `Raw::replace` pass owned bytes. `assemble` takes `&[RawFile<'_>]`.
3. `shuyi-assets`: `#[doc(hidden)]` on `pub mod recipient`, `Raw` and
   `PackMark`, each with a doc line ("The engine's own, for `shuyi
   export --recipient` (ADR-0118); not covered by semver").
4. Move `crates/assets/src/image/mark.rs` and `image/mark/tests.rs` to
   `crates/cli/src/export/recipient/mark.rs` and `mark/tests.rs` with
   `git mv`. Make it a private `mod mark;` of `export::recipient`,
   import `shuyi_assets::image::Image` and
   `shuyi_assets::recipient::{CODE_BYTES, Code}`, and drop `pub mod
   mark` from `image.rs`. Update `export/recipient.rs` and
   `export/recipient/tests.rs`.
5. `shuyi-cli`: `pub(crate) mod recipient` in `export.rs`, and
   `pub(crate)` on what it and `ledger` export. Make
   `FILE_ATTRIBUTE_COMPRESSED` a private `const` and `WITHOUT_EXAMPLES`
   `pub(crate)`. Remove any item that becomes dead code (Clippy says
   which); don't `#[allow]` it.
6. `shuyi-app`: `#[doc(hidden)]` on `run_game_checked`, `PageCheck`,
   `page::page_check`, `page::CheckReport` and `page::report_check`,
   each doc saying the book's exercises use it and semver doesn't cover
   it (ADR-0117).
7. Docs: amend ADR-0117 (*Consequences*: `run_game_checked` is hidden,
   with no second user; a game that wants checks asks for them) and
   ADR-0118 (where `mark` lives; the items are hidden). Check
   `docs/formats.md:776` still reads right.
8. Run `cargo xtask api` (or read CI's `api` job). Write the `Breaking:`
   lines and `migration` blocks, and a `Book:` line (the book names none
   of these).

## Acceptance criteria

- [x] `cargo xtask api` (CI's `api` job) shows, for each item in the
      table, only lines removed from the listing, and no line added in
      `shuyi-assets`, `shuyi-app` or `shuyi-cli`. `image::mark` is gone
      from `shuyi-assets`.
- [x] `run_game_checked`, `PageCheck`, `page_check`, `CheckReport`,
      `report_check`, `recipient::*`, `Raw` and `PackMark` are each
      `#[doc(hidden)]`, and each doc says semver doesn't cover it.
- [x] `cargo xtask exercises` passes: the site still builds with the
      hidden items, through `shuyi::app::run_game_checked`.
- [x] The step-1 snapshot and
      `the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version`'s
      are unchanged, and `pack::VERSION` is still 19.
- [x] A unit test shows `RawFile::kept` of incompressible borrowed bytes
      keeps them `Cow::Borrowed`.
- [x] A test asserts `WITHOUT_EXAMPLES` equals its literal sentence
      ("This game was made without the skills' worked examples (`shuyi
      new --no-examples`)").
- [x] `export --recipient` and `detect` work as before:
      `export::recipient::tests`, the ledger's tests, the moved `mark`
      tests and `tools/export-check/recipient.sh` (CI's `export` job) pass unchanged but for
      paths.
- [x] The completion notes have a `Breaking:` line naming
      `shuyi-assets`, `shuyi-app` and `shuyi-cli` with their items, each
      with a `migration` block.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `RawFile::kept` borrows a stored file and owns a compressed one;
  `WITHOUT_EXAMPLES`'s text is pinned.
- Snapshot: a pack of a stored, a compressed and a baked file, written
  on `main` before the change and unchanged after it.
- The moved `mark` tests, the recipient overlay's tests and the
  exercises' build pass.

## Completion notes

What was done:

- **Pinned first.** `pack::tests::a_pack_of_a_stored_a_compressed_and_a_baked_file_is_pinned`
  was committed with its snapshot before `pack.rs` changed, and still
  passes unchanged after. `pack::VERSION` is still 19, and the
  binary-form snapshot is untouched.
- **`pack::write` borrows again.** `RawFile<'a>` keeps `Cow<'a, [u8]>`:
  `write_first` passes a stored `Content::Raw` file borrowed, a baked
  file owned, and LZ4's output is always owned. `Raw` holds
  `RawFile<'static>`: `Raw::read` and `Raw::replace` own their bytes.
  `a_stored_file_is_borrowed_while_the_pack_is_assembled` checks both
  cases.
- **Hidden** (`#[doc(hidden)]`, each doc saying semver doesn't cover it,
  ADR-0119): `shuyi_assets::recipient` (the module), `pack::Raw`,
  `pack::PackMark`; `shuyi_app::run_game_checked`, `PageCheck`,
  `page::page_check`, `page::CheckReport`, `page::report_check`.
- **Moved:** `shuyi_assets::image::mark` and its tests went to
  `crates/cli/src/export/recipient/mark.rs` and `mark/tests.rs` with
  `git mv`, a private `mod mark` of `export::recipient`.
- **Narrowed:** `shuyi_cli::export::recipient` and `ledger` are
  `pub(crate)`, as are their items; `WITHOUT_EXAMPLES` is `pub(crate)`
  with a test pinning its sentence and a doc saying games' files hold
  it; `FILE_ATTRIBUTE_COMPRESSED` is a private `const`.
- ADR-0117's *Consequences* line and ADR-0118's three paragraphs say
  where each item lives now and that it is hidden. `docs/formats.md`
  still reads right (it names `pack::Raw::write_marked`, which still
  exists).
- `cargo xtask api` shows only removed lines in `shuyi-assets`,
  `shuyi-app` and `shuyi-cli`, and none added.

Deviations:

- The ticket said `pack::VERSION` is 15; it is 19 on `main` (versions
  16–19 came since the retro). Fixed in the Scope and the acceptance
  criteria before the code.
- Step 2 kept `write_first` going through `Raw`. `Raw` holds
  `RawFile<'static>`, so `write_first` borrows only if it doesn't:
  it now orders the files with `grouped` and calls `assemble` itself,
  which is what `Raw::write` did for it. Same bytes (the snapshot).
- `mark::capacity` was used only by its own test once private, so it is
  removed (step 5: no `#[allow]`), and that test reads `carriers`.
  `BITS_PER_COLOR` and `MIN_CARRIERS` are private consts.
- `MarkError` derived `thiserror::Error`. `shuyi-cli` has no
  `thiserror`, and its `export` only formats the error, so `MarkError`
  has a hand-written `Display` with the same text rather than a new
  dependency.
- `FILE_ATTRIBUTE_COMPRESSED` sits inside the `cfg(windows)` block
  that reads it: a module-level private const would be dead code on
  Linux and macOS.
- Five doc links in `shuyi-cli`'s public docs pointed at the items now
  private (`export::recipient`, `WITHOUT_EXAMPLES`); rustdoc refuses
  that, so they are plain code spans.
- `tools/export-check/recipient.sh` runs in CI's `export` job only
  (a release build and `ssh-keygen`); it is not run locally.

Breaking: shuyi-assets: `recipient` (the module), `pack::Raw`, `pack::PackMark` are `#[doc(hidden)]`, and `image::mark` (`mark`, `read`, `capacity`, `palette_count`, `MarkError`, `PictureRead`, `BITS_PER_COLOR`, `MIN_CARRIERS`) is gone: a game changes nothing, since no game uses them

```migration
since: 0.1.0
what: The recipient overlay's own items in shuyi-assets are hidden, and the picture's mark moved into shuyi-cli, before the first release.
old: shuyi_assets::recipient, shuyi_assets::pack::{Raw, PackMark}, shuyi_assets::image::mark
new: "the same paths, #[doc(hidden)] and not covered by semver; image::mark is private to shuyi-cli"
before: |
  // No game calls these: a game reads its copy's code with shuyi_app::recipient().
after: |
  // Nothing changes.
```

Breaking: shuyi-app: `run_game_checked`, `PageCheck`, `page::page_check`, `page::CheckReport`, `page::report_check` are `#[doc(hidden)]`: a game changes nothing, since no game uses them (only the book's exercises do)

```migration
since: 0.1.0
what: The book's exercises' checks are hidden before the first release; a game that wants checks asks for a public shape.
old: shuyi_app::{run_game_checked, PageCheck}, shuyi_app::page::{page_check, CheckReport, report_check}
new: "the same paths, #[doc(hidden)] and not covered by semver"
before: |
  // No game calls these: games call shuyi_app::run_game.
after: |
  // Nothing changes.
```

Breaking: shuyi-cli: `export::recipient` and `export::recipient::ledger` are private to the crate, and so are `skills::WITHOUT_EXAMPLES` and `contract::doctor::FILE_ATTRIBUTE_COMPRESSED`: a game changes nothing, since no game uses them

```migration
since: 0.1.0
what: shuyi-cli's own items behind export --recipient, the skills' marker and a doctor constant are private before the first release.
old: shuyi_cli::export::recipient, shuyi_cli::skills::WITHOUT_EXAMPLES, shuyi_cli::contract::doctor::FILE_ATTRIBUTE_COMPRESSED
new: "private to shuyi-cli; the export operation's answer (contract::export) is unchanged"
before: |
  // No game calls these: a game runs `shuyi export --recipient` and reads its JSON.
after: |
  // Nothing changes.
```

Book: none, the book names none of these items (the exercises' `main.rs`, which calls `run_game_checked`, is never shown to a reader, ADR-0117 §2).

Game: nothing to adopt, the game uses none of these items. `gh search code --repo Zafnok/visions-of-shuyi` for `PackMark`, `shuyi_assets::recipient`, `image::mark`, `Raw::read`, `export::recipient`, `FILE_ATTRIBUTE_COMPRESSED`, `WITHOUT_EXAMPLES`, `page_check`, `CheckReport` and `report_check` finds nothing (2026-11-18); `run_game_checked` and `PageCheck` appear only in `docs/engine/pins.md`, a pin review's notes.

Follow-up tickets: none. 2096 (the `api` check classifying new public types) and 2095 (`#[non_exhaustive]` on `FontAtlasDef`, `Sound`, `DoctorInput`) are their own (2095 is done).
