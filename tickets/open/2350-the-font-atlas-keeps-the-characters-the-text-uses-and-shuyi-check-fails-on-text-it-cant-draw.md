---
id: "2350"
title: "The font atlas keeps the characters the text uses, and shuyi check fails on text it can't draw"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2350 — The font atlas keeps the characters the text uses

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8.

**What the benchmarks have.** Each finds the characters a game's text
uses; none asks the developer to list them.
- Godot pre-renders a font's glyphs on import, among them the
  characters the project's translations use, and a font lists fallback
  fonts for characters it lacks
  (https://docs.godotengine.org/en/stable/tutorials/ui/gui_using_fonts.html).
- Unity's TextMesh Pro: a dynamic font asset adds the characters text
  asks for, a font asset has a fallback list, and a character no font
  has is reported.
- Unreal's composite fonts pick a sub-font by character range and by
  culture.
- Bevy rasterizes a glyph the first time text uses it, so any character
  its font file has can be drawn.

**What Shuyi has today** (checked 2026-10-21).
- `font-atlas` (`crates/cli/src/font_atlas.rs`) keeps only the glyphs in
  `ATLAS_RANGES` (lines 19 to 31): ASCII, Latin-1, Greek, punctuation,
  arrows and symbols. No Latin Extended-A or B, no Cyrillic. Hello,
  Grid's `files-src/ter-u16n.bdf` has 1,356 glyphs, among them `Ж`
  (U+0416), `ł`, `č`, `ğ` and `ı`. Its `files/font/atlas.ron` and the
  templates' copies hold 547, none of those. Polish, Czech, Turkish and
  Russian text draws as `?`.
- The only gate is a fixed list. `FontAtlasDef::problems(required)`
  (`crates/assets/src/font.rs`, line 154) fails an atlas that lacks a
  glyph of `shuyi.ron`'s `font_atlas.required_glyphs` or a game's
  `REQUIRED_GLYPHS`. Nothing compares the atlas with the text. A
  character the atlas lacks shows as the fallback `?`, with one `warn!`
  per character (`crates/app/src/render.rs`, `atlas_cell`, lines 290 to
  302; `crates/cli/src/frame_png.rs`, lines 585 to 589, draws the same
  `?`).
- The other way round, `.dlg` text and names are held to printable
  ASCII. `dialogue::char_problem` (`crates/assets/src/dialogue/parse.rs`,
  lines 115 to 135) rejects `é` ("dialogue text is plain ASCII"), though
  the atlas draws it, and suggests ASCII for `’ “ ” … — –`.
  `names::from_source` (`crates/assets/src/names.rs`, line 287) uses it
  too. A language pack's text has no rule on characters at all, and
  nothing compares it with the atlas.
- The templates ship `atlas.png` and `atlas.ron` (copies of Hello,
  Grid's, `THIRD_PARTY_ASSETS.md`) but not the BDF they came from, and
  their `shuyi.ron` has no `font_atlas` part. A new game can't remake
  its atlas.

**Planned.**
- 2301 (game#0236): `font-atlas --wide` takes the wide characters the
  packs use, with a staleness check. 2303 (game#0239) checks a pack's
  wide characters against its set. Both are for wide (CJK) glyphs only.
  2301 waits for this ticket (its `blocked_by`) and reads this ticket's
  scan (`AtlasChars::used`) and check (`font::undrawn`) instead of
  building its own; 2303 builds on 2301.
- 2346: typing names in any language, which refuses at run time a
  letter the font lacks. Either order with this ticket (In, 1).
- 2224: TTF and OTF fonts cut to cells. Its either-order clause: if this
  ticket is done, it keeps the characters `AtlasChars::wants`.

**Why it matters.**
- **The game first.** The game's Japanese and Chinese (2301, 2303) need
  the characters its text uses found and checked. This ticket builds
  that scan and that check once, for narrow glyphs, and 2301 and 2303
  extend them to wide ones. The game needs no Cyrillic or Latin
  Extended text today.
- **Other games.** A Polish or Russian translation draws as `?` today,
  and no gate fails. The template's own font has the letters.
- **The end goal** (Nick, 2026-10-19, `docs/decisions.md`): an agent
  making a game learns of a missing glyph from `shuyi check`, with the
  file, the key and the character, and one command fixes it. Today it
  learns from a picture.

## Nick input

None.

## Scope

**In:**

1. **One scan of the text** (`shuyi_assets::lang`, `no_std`):
   - `TextPart`, `#[non_exhaustive]` so a later ticket can add a part:
     `Screen`, `Data`, `Dialogue`, and `LangName` (a pack's
     `LangInfo::name`, which the language menu shows whatever the
     language is).
   - `TextAt<'a> { lang: Option<&'a LangCode>, part: TextPart, key: &'a
     str, file: &'a str }`: the language (`None` for English), the part,
     the key (`name` for a `LangName`), and for a dialogue line the stem
     of its `.dlg` file (`SourceLine::file`; empty otherwise).
   - `Lang::texts(&self) -> Vec<(TextAt<'_>, &str)>`: every text the
     `Lang` holds. First English: its screen text, its data text (names
     included, as `names.<id>`), its dialogue lines and captions
     (`source.lines`). Then each pack in code order: its name, screen
     text, data text and dialogue lines, every variant of a
     `LineText::Variants` (`LineText::all`). A pack line's file is its
     English line's (`source.lines`), or empty for an orphan.
   - This is the scan 2301 and 2303 use for wide glyphs.
   - **Either order with 2346:** if 2346 is done, add
     `TextPart::NameGrid`: each pack's name grid pages
     (`GridPage::rows` of `LangInfo::name_grid`) are texts too; if not,
     2346 adds them.
   - 2351 merges the engine's English into the `Lang`'s English, so the
     scan lists it whichever lands first.
2. **The check:** `shuyi_assets::font::undrawn(atlas: &FontAtlasDef,
   lang: &Lang) -> Vec<ContentError>`, a check across assets after
   loading (ADR-0019 §5):
   - one error per language and character the atlas has no glyph for,
     in the order of their first use in `texts`. Control characters
     (`char::is_control`, a `\n` in a text) are skipped;
   - **file:** the file of the character's first use. For a pack, its
     file under the game's languages folder, as `lang::from_files`
     reads it: `ru/lang.ron` for its name, `ru/ui.ron`, `ru/data.ron`,
     and for a dialogue line `ru/dialogue/` plus its English line's
     `.dlg` stem (`ru/dialogue/ch01.ron`, the usual layout; a
     `LangPack` doesn't record which of its files held a line). For
     English: a line's `.dlg` file (`ch01.dlg`), and `en` for screen and
     data text, since a `Lang` doesn't record which file the game read
     them from (Hello, Grid and the templates read `en.ron`, not
     `en/ui.ron`). **Either order with 2226:** whichever lands second
     uses the source language's code instead of `en`;
   - **no line:** a `Lang` keeps keys, not lines. The message names the
     key, which finds the line;
   - **message:** the character, its code point, the first key that uses
     it, how many more keys of that language do, and what to do, for
     example `"Ж" (U+0416) isn't in the font atlas (menu.title and 3
     more keys); remake the atlas with font-atlas from a font that has
     it`;
   - an undrawn `‘ ’ “ ” … – —` also names its ASCII spelling, the hint
     `char_problem` gives today;
   - a combining mark (U+0300 to U+036F) is an error even when the atlas
     has a glyph for it (Terminus has U+0301): a cell draws it beside
     its letter, not over it. The message asks for the accented letter
     as one character (`é`, U+00E9).
3. **`.dlg` text and names hold any character but a control
   character.** `dialogue::char_problem` returns a problem only for
   `char::is_control` ("control character U+0007 in dialogue text"),
   still with the line and column `check_chars` gives. Its doc says the
   new rule. `names::from_source` uses it as today; its doc (line 249,
   "plain ASCII") says the new rule. A character the font lacks is the
   check's (2) to find, not the parser's.
4. **`font-atlas` keeps the characters the text uses**
   (`crates/cli/src/font_atlas.rs`):
   - `AtlasChars`, the characters an atlas keeps: every one of
     `ATLAS_RANGES` (unchanged), every character the game's text uses,
     and `font_atlas.required_glyphs`; never a combining mark or a
     control character. `AtlasChars::default()` is the ranges alone,
     today's atlas. `AtlasChars::of_lang(lang: &Lang, required: &str)`,
     `wants(&self, c: char) -> bool`, and `used(&self) ->
     &BTreeSet<char>` (the text's characters, without control
     characters and combining marks, which 2301's `--wide` reads).
   - `build_atlas(font: &BitmapFont, chars: &AtlasChars)`: the font's
     glyphs it wants, in code-point order, as today. Its other callers
     pass `&AtlasChars::default()`: `crates/cli/src/toy.rs`,
     `crates/cli/src/glyph_tiles/tests.rs` and `font_atlas.rs`'s tests.
   - `run(rules, command, font_paths, out_dir, chars: &AtlasChars)`: its
     summary adds a line naming the characters the text uses that the
     fonts lack (at most 20, then a count). That isn't a failure:
     `check` (6) is the gate, and 2301's wide font may draw them.
   - An atlas whose text uses nothing outside the ranges and the
     required set is byte for byte what it is today.
5. **`font-atlas` runs with the game's text** (`crates/cli/src/lib.rs`):
   - when `shuyi.ron` names a tool program (`game`), `font-atlas` runs in
     it, as `lang-status` does, and reads the text through
     `GameTools::lang`. If `lang()` fails, `font-atlas` fails with its
     error and says that `--no-text` remakes the atlas without the text;
   - with `--no-text` (taken out of the arguments before the fonts and
     the folder), or with no tool program, it keeps the ranges and the
     required set only, and prints one line saying so. `--no-text` is
     for a game whose tool program doesn't build yet or whose text
     doesn't load;
   - the usage text (in `usage` and in `font_atlas`) says both:
     `font-atlas [--no-text] <font.bdf>... <out-dir>`.
6. **`check` checks the text against the atlas**
   (`crates/cli/src/game.rs`, `crates/cli/src/contract/check.rs`):
   - `GameTools::font_atlas(&self) -> Option<Result<FontAtlasDef,
     String>>`, `None` by default, as `GameTools::controls` is;
   - `check::check`: when the game's validators found no error, it runs
     `font::undrawn` on `font_atlas()` and `lang()`'s `lang` and adds
     its errors. Those errors have no line, so the existing limit "some
     errors are about a whole file" comes with them;
   - `None` adds the limit "the text isn't checked against the font
     atlas: the game's tools don't offer it (GameTools::font_atlas)".
     `Some(Err)` or a `lang()` error fails the answer, as a controls
     error fails `check --target`. With validator errors, it adds the
     limit "the text is checked against the font atlas once the content
     loads without errors";
   - no schema changes: the errors are `Problem`s and the notes are
     limits, so `--json` (and 1133's MCP server, once it exists) carry
     them as they are;
   - **why `check` and not the game's load:** a game whose atlas lacks a
     glyph still loads and runs (it shows `?`), so `font-atlas`, which
     loads the game's text in its tool program, can always remake the
     atlas.
7. **Hello, Grid and the templates:**
   - Hello, Grid's `HelloTools` (`examples/hello-grid/src/tools.rs`) and
     both templates' `Tools` (`crates/game/src/tools.rs`) offer
     `font_atlas`: their loaded `font`.
   - Hello, Grid's `Hello::load_from` (`examples/hello-grid/src/lib.rs`)
     builds its `Lang` with its dialogue's English
     (`Lang::with_source`, `SourceText::add_dialogue`, and
     `DialogueTable::files` naming `intro.dlg` for its scenes), so its
     `.dlg` text is checked. Its test
     `the_font_atlas_is_what_font_atlas_makes_of_the_bdf`
     (`tests/it/files.rs`) builds with `AtlasChars::of_lang` over
     `Hello::load()`'s `Lang`. The comments in its `shuyi.ron` on `game`
     and `font_atlas` say that `font-atlas` runs the game and keeps the
     characters its text uses.
   - Both templates get the font they were made from:
     `files-src/ter-u16n.bdf` (Hello, Grid's file, unmodified, OFL-1.1
     like the atlas), and a `font_atlas` part in `shuyi.ron` with
     `required_glyphs` equal to the template's `REQUIRED_GLYPHS` and
     `readme: "README.md"`, whose comment gives the command (`cargo
     shuyi font-atlas files-src/ter-u16n.bdf crates/game/files/font`).
     The comment on `game` adds `font-atlas` to the commands that run
     the game. Their atlas files don't change.
   - `_typos.toml` excludes `templates/*/files-src/*.bdf`, as it does
     Hello, Grid's.
   - The templates' README ("Art and licenses", the files table) and
     `AGENTS.md` say it in a sentence each: `cargo shuyi check` names a
     character the atlas lacks, and `font-atlas` adds it if the font has
     it. `THIRD_PARTY_ASSETS.md`'s templates row names the BDF copies.
8. **Docs:** `docs/contract.md` (the `check` row, and `font_atlas` in
   the table of what a game gives), `crates/cli/src/game.rs`'s module
   docs (the commands that run the game), Hello, Grid's README ("Files
   made by the tools"), and `font_atlas.rs`'s module docs.

About 600 changed lines, without the two BDF copies. If it runs well
over, the templates' part (In 7, implementation step 9) is the piece to
move to a follow-up ticket (`write-ticket`).

**For the game:**
- `GameTools::font_atlas` has a default, so nothing breaks. Until the
  game's tools implement it, `check` says in its limits that the text
  isn't checked against the atlas.
- `font_atlas::build_atlas` and `font_atlas::run` take an
  `&AtlasChars`. A caller that wants today's atlas passes
  `&AtlasChars::default()`. `ATLAS_RANGES`, `ONE_BIT_FILE` and
  `read_font` don't change.
- `.dlg` text and names table values may hold any character but a
  control character. The parser no longer rejects `é` or `’`; `check`
  finds a character the atlas lacks, once the game's tools offer
  `font_atlas`.
- `font-atlas` runs in the game's tool program when `shuyi.ron` names
  one.
- The game's atlas changes only if its text uses a character outside
  `ATLAS_RANGES` that its fonts have.

The PR says so (with a `Breaking:` line if 2010 is done), and the game's
PR that moves its pin follows it.

**Out (do not do):**
- Wide (CJK) glyphs, wide sets per language and widths by language:
  2301 and 2303, which build on this scan and this check.
- Line breaking and text too long for its place (2302).
- Typing names: `NameRules`, the name grid's pages and the IME (2346).
- TTF and OTF fonts, and proportional text (2224, 2334). 2224 picks its
  characters with `AtlasChars` (its either-order clause).
- Adding Latin Extended or Cyrillic blocks to `ATLAS_RANGES`: every
  game's atlas, the game's included, would change at its next
  regeneration. A game gets those letters by using them.
- An atlas of only the characters the text uses, with no ranges, for a
  small target's budgets (3008): a follow-up if a target needs it.
- Combining marks, stacked diacritics, right-to-left and shaped
  scripts: precomposed letters only (2908, parked research).
- Lines and columns for a pack's text: a `Lang` keeps keys.
- Several atlases per game (fonts by id, 2334).
- Screen text written as literals in Rust (`check-text`), and the key
  names that `{Cursor}` and other key placeholders fill in (the
  keymap's).
- The names table's other helpers in `crates/assets/src/names.rs`:
  `literal_form` and `contains_word` (ASCII letters only) and
  `capitalise` (Latin letters only, through `is_latin_letter`): a
  follow-up ticket if a game's names need them.

## Implementation steps

1. Read `crates/assets/src/lang.rs` (`Lang`, `LangPack`, `LangInfo`,
   `from_files`), `lang/source.rs` (`SourceText`, `SourceLine`),
   `lang/lines.rs` (`LineEntry`, `LineText`), `font.rs`,
   `dialogue/parse.rs` (`char_problem`, `check_chars`), `names.rs`;
   `crates/cli/src/font_atlas.rs`, `lib.rs` (`run`, `usage`,
   `font_atlas`, `lang_status`), `game.rs`, `contract/check.rs`,
   `contract/fake.rs`, `toy.rs`, `tests/it/binary.rs`; Hello, Grid's
   `shuyi.ron`, `src/lib.rs`, `src/tools.rs`, `tests/it/files.rs` and
   `tests/it/contract.rs`; the templates' `shuyi.ron`,
   `crates/game/src/tools.rs`, `crates/game/tests/it/main.rs`, README
   and `AGENTS.md`.
2. Write `TextPart`, `TextAt` and `Lang::texts`, with unit tests.
3. Write `font::undrawn`, with unit tests.
4. Change `char_problem` and its docs. Update the tests that expect the
   ASCII errors: `non_ascii_text_names_its_column`
   (`crates/assets/src/dialogue/tests.rs`) and `bad_values_are_reported`
   (`crates/assets/src/names/tests.rs`, which expects "dialogue text is
   plain ASCII" for `Zoë`).
5. Write `AtlasChars`, and change `build_atlas`, `run` and their
   callers, with tests on a small BDF fixture that has `Ж`, `ł` and
   U+0301.
6. Run `font-atlas` in the game's tool program, with `--no-text`.
   Update the usage line that `crates/cli/tests/it/binary.rs` expects.
7. Add `GameTools::font_atlas` and the check in `check::check`. `Fake`
   (`contract/fake.rs`) gains `lang: Lang` (default `Lang::default()`)
   and `font_atlas: Option<Result<FontAtlasDef, String>>` (default
   `Some(Ok(FontAtlasDef::default()))`, so the existing tests keep their
   limits). Build the test's `ru` pack with `lang::from_files` from an
   `en/ui.ron`, a `ru/lang.ron` and a `ru/ui.ron`. Update
   `check_passes_with_no_error_and_lists_each_with_where` for the new
   limit when the validators find errors.
8. Hello, Grid: `font_atlas`, its `Lang` with its dialogue's English,
   its `shuyi.ron` comments and its tests. Run `cargo shuyi font-atlas
   files-src/ter-u16n.bdf files/font` in `examples/hello-grid` and see
   that `git status` shows no change.
9. The templates: the BDF copies, `shuyi.ron`, `font_atlas` in their
   tools, README, `AGENTS.md`, `THIRD_PARTY_ASSETS.md`, `_typos.toml`.
   From the repository's root, make a game with `cargo shuyi new grid
   <dir> --engine-path <this checkout>`, add a word with `ł` to its
   `crates/game/files/en.ron`, run its `cargo shuyi check` (it names
   `ł`), then `cargo shuyi font-atlas files-src/ter-u16n.bdf
   crates/game/files/font` (it also writes `atlas_1bit.bin`, which the
   templates don't ship) and `check` again (it passes). Say in the PR
   what they printed.
10. The docs. Run the gates.

## Acceptance criteria

- [ ] `Lang::texts` lists every text of a `Lang` with English's screen,
      data and dialogue text and a pack with a name, screen text, data
      text and a line with two variants, each with its part, key and
      file (unit test).
- [ ] With an atlas that has every character of the fake game's text but
      `Ж`, `check` on a fake game whose `ru` pack's `menu.title` is "Жa"
      fails with exactly one error: file `ru/ui.ron`, a message naming
      `Ж`, U+0416 and `menu.title`. With the glyph it passes (test with
      the fake game).
- [ ] Two keys of one pack using the same missing character give one
      error that counts the other key; the same character in two packs
      gives two errors (unit test).
- [ ] A combining mark (U+0301) is an error even with its glyph in the
      atlas; an undrawn `’` names `'` as its ASCII spelling; a `\n` is
      never an error (unit tests).
- [ ] `check` on a game whose tools don't offer `font_atlas` passes as
      before, with the limit saying the text isn't checked against the
      atlas; with validator errors, the limit says it is checked once
      the content loads (tests).
- [ ] `.dlg` text and names table values with `é`, `ł` and `Ж` load; a
      control character (U+0007) is still an error with its line and
      column (unit tests).
- [ ] `font-atlas` with text using `Ж` and `ł` keeps both from a font
      that has them, keeps every glyph of `ATLAS_RANGES` the font has,
      never keeps U+0301, and names a used character the font lacks
      without failing (test).
- [ ] Hello, Grid's and the templates' `atlas.png` and `atlas.ron` are
      unchanged. A test in `crates/cli` finds each template's atlas equal
      to what `build_atlas` makes of its `files-src/ter-u16n.bdf` with
      `AtlasChars::default()`.
- [ ] `check` on Hello, Grid (`examples/hello-grid/tests/it/contract.rs`)
      and on each template (`crates/game/tests/it/main.rs`) passes with
      no limit about the font atlas.
- [ ] `shuyi-assets` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Lang::texts` over each part and variants; `font::undrawn`
  (one error per language and character, the order, files, the count,
  the hints, combining marks, control characters skipped);
  `char_problem`; `AtlasChars` (ranges, used, required, never a
  combining mark); `build_atlas` with extra characters; `check` with
  and without `font_atlas`, with a `font_atlas` or `lang` error, and
  with validator errors.
- Property: for any `Lang` made of random strings, every character of
  every text is in `AtlasChars::of_lang(..).used()` unless it is a
  control character or a combining mark; and `undrawn` is empty exactly
  when the texts hold no combining mark and every other character but a
  control character has a glyph in the atlas.
- Snapshot / integration: Hello, Grid's atlas built from its BDF and its
  text equals the committed files; each template's atlas equals what its
  BDF makes; `check` through the contract on Hello, Grid and on each
  template.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
