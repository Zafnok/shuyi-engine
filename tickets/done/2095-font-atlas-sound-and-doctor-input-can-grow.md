---
id: "2095"
title: "Before the first release, the font atlas's description, the Sound kind and DoctorInput move under ADR-0039"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-17
---

# 2095 — Before the first release, the font atlas's description, the `Sound` kind and `DoctorInput` move under ADR-0039

## Context

2082 to 2086 moved, before 2005, every exhaustive public type that an
open ticket planned to grow
([ADR-0039](../../docs/adr/0039-public-types-that-can-grow-non-exhaustive-config-and-views-with-constructors-plain-values-left-open.md),
its *When*; Nick's decision of 2026-11-14). The retro of PRs #379–#484
(2026-11-17, problem 8) found types that sweep missed. Each was checked
on 2026-11-17:

| Type | Where | Why it moves |
| ---- | ----- | ------------ |
| `shuyi_assets::font::FontAtlasDef` | `crates/assets/src/font.rs:112` | #463 (3215) added `ink`, #468 (3216) `bold` and `bold_ink`: two breaks of the game in one day |
| `shuyi_assets::font::FontInk` | `font.rs:157` | the same description, from 3215; its `columns` is already `#[serde(default)]` |
| `shuyi_assets::audio::Sound` (the kind, `audio::formats::Sound`) | `crates/assets/src/audio/formats.rs:22` | 2378 adds `length_ms: Option<u32>` |
| `shuyi_cli::contract::doctor::DoctorInput` | `crates/cli/src/contract/doctor.rs:44` | parked 3051 adds `signing: bool` |

- `FontAtlasDef` is written as a literal 48 times in 25 files
  (`grep -rn "FontAtlasDef {" --include=*.rs`), outside `shuyi-assets`
  in `shuyi-app`, `shuyi-cli`, `shuyi-platform`, `shuyi-playdate` and
  `shuyi-ui`. The game builds one too: its `crates/xtask/src/icon.rs`,
  line 237, names every field (`gh search code --repo
  Zafnok/visions-of-shuyi FontAtlasDef`, at the game's pin
  `e738ffc2`); its `docs/engine/pins.md` rows for #463 and #468 each fix
  that literal.
- `Sound` is built by literal only inside `shuyi-assets`
  (`formats.rs:60`, `format/builtin_tests.rs:504`). A game's own sound
  format (ADR-0007) must build one, so it gets constructors. The game
  names no `formats::Sound` (same search).
- `DoctorInput` is `pub struct DoctorInput {}`: every caller writes
  `DoctorInput::default()` (`contract/cli.rs:134`, `doctor/linux.rs`,
  hello-grid's `tests/it/contract.rs`). The game names none.
- `AudioManifest` has been `#[non_exhaustive]` since 2341 (#174), yet
  2378 (its *For the game*) and 2223 (step 7) still plan a `Breaking:`
  line for its new field. The PR that adds this ticket rewrote both as
  additive, and 3051's `DoctorInput` line too.

No open ticket plans a field for `FontAtlasDef` today: 2224 and 2346
leave it alone (2224's **Out**), and 2334, 2346 and 2353 add methods. It
moves because it grew three fields in a day and is data a game builds.

## Nick input

None.

## Scope

**In:**
- `FontAtlasDef` and `FontInk`: kind 5 (data read from files, built in
  code by tests), `#[non_exhaustive]`, each later field
  `#[serde(default)]`. `FontAtlasDef::new(cell_w, cell_h, glyphs,
  columns)`, a `const fn`, with every set empty and no ink. Of 3503's
  constructor rule (no two neighbouring numbers), it breaks only the
  allowed case: `cell_w` then `cell_h`, a width then a height.
  `FontInk::new` exists already.
- `Sound`: kind 5, `#[non_exhaustive]`, built with the `const fn`s
  `Sound::mono(sample_rate: u32)` and `Sound::stereo(sample_rate: u32)`.
  Not a `new(channels: u16, sample_rate: u32)`: two neighbouring
  integers, which 3503's rule forbids.
- `DoctorInput`: kind 4, `#[non_exhaustive]`, built with `default()`.
  Its `#[serde(deny_unknown_fields)]` stays, so its schema and the
  contract's `VERSION` don't change.
- Every literal outside the defining crate becomes a constructor call
  with fields assigned. Literals inside `shuyi-assets` may stay.
- A `Breaking:` line for each type in the completion notes, with a
  `migration` block for `FontAtlasDef` (the game's `icon.rs` literal is
  its `before`).
- **The audit, again:** every exhaustive public type of
  `shuyi-assets` (the description types: fonts, audio, images, maps,
  palettes, packs, keymaps, languages, targets) that an open ticket
  plans to grow, found by searching `tickets/open` for each type's name
  beside "gains", "new field" or `Type::field:`. A first pass on
  2026-11-17 over the 142 exhaustive type names in `crates/assets/src`
  found none beyond this table (2224's `AtlasRect` gains derives only;
  3059's `Bake::languages` is `GameBake`'s, already non-exhaustive).
  Each one found moves here, as the table's do.
- **Either order with 2378:** if 2378 is done, `Sound` has
  `length_ms`: `mono` and `stereo` start it at `None`; if not, nothing
  to do (2378 adds it, then additively).
- **Either order with 3051:** if 3051 is done, `DoctorInput` has
  `signing` and nothing else changes; if not, nothing to do (3051 adds
  it, then additively).
- **Either order with 2096:** if 2096 is done, the audit reads its
  `tools/exhaustive.ron` (its `open` entries are the candidates), and
  each type moved here leaves that file in this PR; if not, nothing to
  do (2096 lists the types as this PR leaves them).
- **Either order with 3503:** if 3503 is done, `FontAtlasDef::new`
  joins its `tools/constructors.ron` ("a cell's width then its
  height"); if not, nothing to do (3503 lists it).

**Out (do not do):**
- Any new field (2378's `length_ms`, 3051's `signing`).
- `MusicTrack`, `InkBox` and `AtlasRect`: plain values (kind 1), no
  ticket grows them.
- Types of other crates than `shuyi-assets`, beyond `DoctorInput`, and
  a check that fails the next miss (2096).
- `pack::VERSION`: no binary form changes.

## Implementation steps

1. Read ADR-0039 and 2085's completion notes (the same work for other
   types). Run the audit (Scope) and add what it finds to the table in
   the completion notes.
2. `crates/assets/src/font.rs`: the attribute, `FontAtlasDef::new`, and
   a doc line naming the kind on both types. `audio/formats.rs`:
   `Sound`'s attribute, `mono` and `stereo`. `contract/doctor.rs`:
   `DoctorInput`'s attribute and doc line.
3. Build the workspace, the templates, the tutorial's steps, the site's
   snippets and the skill pack's examples; fix each literal outside its
   crate (`cargo check --workspace --all-targets`, then
   `cargo xtask gates`).
4. Check that every `atlas.ron` in the repository and the templates
   loads unchanged, and that `cargo shuyi schema --out crates/cli/schemas`
   changes nothing.
5. The `Breaking:` lines and the `migration` block; run the gates.

## Acceptance criteria

- [x] `FontAtlasDef`, `FontInk`, `Sound` and `DoctorInput` are
      `#[non_exhaustive]`, each with a doc line naming its ADR-0039 kind.
- [x] A test outside `shuyi-assets` (`crates/assets/tests/it/constructors.rs`)
      builds a `FontAtlasDef` in a `const` item and a `Sound` with
      `mono` and `stereo`; one in `crates/cli/tests/it` builds a
      `DoctorInput` with `default()`.
- [x] `FontAtlasDef::new(8, 16, glyphs, 16)` equals the literal with the
      same values and every set empty.
- [x] Every font atlas, sound and pack in the repository and the
      templates loads unchanged; `crates/cli/schemas` is unchanged.
- [x] No open ticket plans a `Breaking:` line for one of these types or
      for `AudioManifest`'s fields.
- [x] `cargo xtask api` lists each change, and the completion notes have
      a `Breaking:` line naming each type.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `FontAtlasDef::new`, `Sound::mono` and `Sound::stereo` build
  what a literal built before.
- Integration: the constructors outside their crates, a `const` atlas
  among them.
- The existing font, audio, pack and `doctor` tests pass unchanged.

## Completion notes

What was done:

- `FontAtlasDef` and `FontInk` (kind 5, data read from files) and the
  `Sound` kind (kind 5) are `#[non_exhaustive]`, and `DoctorInput`
  (kind 4) too; each has a doc line naming its ADR-0039 kind
  (`DoctorInput`'s is a `//` comment, as the contract's other inputs
  have, so its schema keeps its description).
  - `FontAtlasDef::new(cell_w, cell_h, glyphs, columns)`, a `const fn`:
    no wide set, no named sets, no bold face, no ink. `FontInk::new`
    was there already.
  - `Sound::mono(sample_rate)` and `Sound::stereo(sample_rate)`, both
    `const fn`.
  - `DoctorInput` keeps `Default` and `#[serde(deny_unknown_fields)]`.
- No field and no serde attribute changed: every `atlas.ron`, sound
  and pack loads as before (the assets crate's 761 unit tests and its
  integration tests pass, the font-atlas, audio and pack fixtures among
  them), `pack::VERSION` stays, and `cargo shuyi schema --out
  crates/cli/schemas` changes no file, so the contract's `VERSION`
  stays.
- Every literal outside `shuyi-assets`, 25 in all, became
  `FontAtlasDef::new` with the rest assigned, or a clone with a field
  assigned where a test wrote `..other` (`shuyi-app`, `shuyi-cli`,
  `shuyi-platform`, `shuyi-playdate`, `shuyi-ui`, and the doc example
  in `crates/app/src/lib.rs`). `Sound` and `DoctorInput` had no literal
  outside their crates. The templates, the tutorial's steps, the
  site's snippets and the skill pack build none of the four.
- Tests: `the_constructor_builds_what_a_literal_built` (`font.rs`) and
  `a_sounds_constructors_build_what_a_literal_built` (`audio.rs`);
  outside the crates, `crates/assets/tests/it/constructors.rs` builds a
  `FontAtlasDef` in a `const` item and another with its wide set and
  ink assigned, and a `Sound` with `mono` and `stereo`;
  `crates/cli/tests/it/doctor.rs` builds a `DoctorInput` with
  `default()` and reads it as `{}`.
- `docs/formats.md`, *Writing a format*, says a format outside the
  engine builds an engine kind with its constructors.

The audit (Scope), over the 139 exhaustive `pub struct` and `pub enum`
names of `crates/assets/src` (parsed from the source; the ticket's 142
counted differently), each searched in `tickets/open` in the same
sentence as "gains", "new field" or `Type::field:`. It found nothing
beyond the table:

| Hit | Why it doesn't move |
| --- | ------------------- |
| `AtlasRect` (2224), `ImageRect` (2231, 2235) | gain derives only |
| `Bundle` (2228), `Formats` (2228, 2232, 2517, 3220, 3316), `Ron` (2230), `Lang`, `Message` (2266), `LangPackRon` (2241), `Layout` (2333) | private fields (kind 8): no literal to break |
| `CinematicRon` (2364) | a marker; its file's types are 2084's, and 2364 already adds their fields additively |
| `Step` (2361), `Action` (2374) | other types of the same name (`frame_png::Step`, the game's `Action`) |

The clauses that waited on this ticket now say what holds: 2096 (the
four types stay out of `tools/exhaustive.ron`, and a type a ticket grows
moves in 2096's PR), 2378 (`length_ms` is additive), 3051 (parked;
`signing` is additive) and 3503 (`FontAtlasDef::new` goes in its file).
2225's `SfxRon` now builds its kind with `Sound::mono(SAMPLE_RATE)`. No
open ticket plans a `Breaking:` line for these types or for
`AudioManifest`'s fields.

Deviations:

- Some existing tests changed, against "pass unchanged": those outside
  `shuyi-assets` that built a `FontAtlasDef` by literal or with `..`,
  which the attribute makes necessary. Their assertions are unchanged.

The game, searched with `gh search code --repo Zafnok/visions-of-shuyi`
for `FontAtlasDef`, `FontInk`, `DoctorInput`, `audio::Sound` and
`formats::Sound` on 2026-11-17 (its `main` at `58176da0`, pin
`e738ffc2`): one literal, `crates/xtask/src/icon.rs` line 237 (a test's
tiny atlas). `crates/content/src/font.rs` re-exports the type and loads
it from files; the `audio::Sound` hit (`crates/xtask/src/playdate_audio.rs`)
is a comment. Nothing names `FontInk`, the `Sound` kind or `DoctorInput`.

Game: nothing to adopt beyond the pin move: the game's next pin move past
this change rewrites its one `FontAtlasDef` literal; no game ticket is
needed before it.

Book: none, no page of the book or snippet builds a `FontAtlasDef`,
`FontInk`, `Sound` or `DoctorInput` (`grep` over `site/`).

Breaking: shuyi-assets: font::FontAtlasDef, font::FontInk: #[non_exhaustive]; build an atlas with the const fn FontAtlasDef::new and assign its other fields, and ink with FontInk::new

```migration
since: 0.1.0
what: A FontAtlasDef built in code uses FontAtlasDef::new, and its wide sets, bold face and ink are assigned.
old: FontAtlasDef { cell_w, cell_h, columns, glyphs, wide, wide_sets, bold, ink, bold_ink }
new: FontAtlasDef::new(cell_w, cell_h, glyphs, columns)
before: |
  let def = FontAtlasDef {
      cell_w: 2,
      cell_h: 4,
      columns: 2,
      glyphs: [('b', 0), ('a', 1)].into_iter().collect(),
      wide: BTreeMap::new(),
      wide_sets: BTreeMap::new(),
      bold: BTreeMap::new(),
      ink: None,
      bold_ink: None,
  };
after: |
  let def = FontAtlasDef::new(2, 4, [('b', 0), ('a', 1)].into_iter().collect(), 2);
find: ["FontAtlasDef {", "FontInk {"]
```

Breaking: shuyi-assets: audio::Sound: #[non_exhaustive]; a format of a game's own builds it with Sound::mono or Sound::stereo

```migration
since: 0.1.0
what: The Sound kind is built with Sound::mono or Sound::stereo, not by a literal.
old: Sound { channels, sample_rate }
new: Sound::mono(sample_rate), Sound::stereo(sample_rate)
before: |
  Ok(Sound { channels: 2, sample_rate: SAMPLE_RATE })
after: |
  Ok(Sound::stereo(SAMPLE_RATE))
find: ["Sound {"]
```

Breaking: shuyi-cli: contract::doctor::DoctorInput: #[non_exhaustive]; build it with DoctorInput::default()

```migration
since: 0.1.0
what: DoctorInput is built with default(), not by the literal DoctorInput {}.
old: DoctorInput {}
new: DoctorInput::default()
before: |
  let answer = doctor(&ThisMachine, &DoctorInput {});
after: |
  let answer = doctor(&ThisMachine, &DoctorInput::default());
find: ["DoctorInput {}"]
```
