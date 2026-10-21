---
id: "2353"
title: "A generated pseudo-language and a Harness walk over every screen for cut text and text not from a key"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2511"]
nick_input: none
completed:
---

# 2353 — A generated pseudo-language and a walk over every screen

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Three of them can show a game in a made-up language
built from the source text, so that a developer sees which text is cut
by its box and which text never went through translation:

- Godot's pseudolocalization
  (<https://docs.godotengine.org/en/stable/tutorials/i18n/pseudolocalization.html>):
  accented letters, doubled vowels, an expansion ratio, a prefix and a
  suffix, placeholders left alone.
- Unity's Localization package has a Pseudo-Locale with an Accenter, an
  Expander and an Encapsulator
  (<https://docs.unity3d.com/Packages/com.unity.localization@1.5/manual/Pseudo-Localization.html>).
- Unreal's localization tools can show leetified text to find text that
  isn't localized.
- Bevy has nothing built in.

Shuyi today:

- Language packs over English, with missing and stale entries shown in
  English (`crates/assets/src/lang.rs`), and `lang-status`, which lists
  them for a translator (`crates/cli/src/lang_status.rs`). Both are
  static: they read files, not screens.
- A debug language a game writes by hand: `GameData::debug_lang`, which
  `Ctx::effective_lang` shows only with debug tools on
  (`crates/ui/src/ctx.rs`). The toy's is three uppercase lines
  (`DEBUG_LANG` and `DEBUG_UI` in `crates/ui/src/toy.rs`).
- `check-text` is a line scanner over source (`crates/cli/src/check_text.rs`);
  2006 (MVP) will run it on the engine's own code. It can't see text a
  screen builds at run time.
- 2209 (done) checks at load that a dialogue line fits its box; 2302
  will add a `max_cells` per screen-text key and an overflow list in
  `lang-status`. Both cover text whose place is declared, before it is
  drawn.
- No tool can pick the language: `frame-png`, `run` and `scene` have no
  `--lang`, and the Harness has no setter (only `h.ctx_mut().lang`).

Why it matters:

- **The game first.** Visions of Shuyi will ship Japanese and Chinese
  made by a machine (game#0718, game#0725), and nobody on the project
  can read the Japanese. Translated text often takes more cells than
  English (a Japanese character takes two, 2301), and a screen that
  cuts it is found today only by someone looking. A pseudo-language
  shows every screen at about a third longer, in a build with debug
  tools, before a single line is translated.
- **Other 2D games.** Every game that ships in more than one language
  has the same question: does every screen survive translation?
- **The end goal (2026-10-19).** An agent can't look at every screen.
  A walk that answers in a list, run by the tests, and a `--lang` flag
  on `frame-png`, `run` and `scene`, give an agent and CI a check with
  no translator.

## Nick input

None.

## Scope

**In:**

- **The generator, `shuyi_assets::lang::pseudo`** (`crates/assets/src/lang/pseudo.rs`,
  `no_std` with `alloc` like the rest of the crate):
  - `pub const PSEUDO: &str = "qps"`, a valid `LangCode` today (2 to 8
    lowercase letters; `qps` is in ISO 639's range for local use,
    `qaa` to `qtz`). **Either order with 2226:** if 2226 is done, the
    code is `qps-ploc`, the pseudo-locale tag Windows uses; if not,
    `qps`, and 2226 renames it (its clause says so).
  - `pub fn pseudo_text(text: &str, can_draw: &dyn Fn(char) -> bool) -> String`:
    1. everything between `{` and `}` is copied as it is, as
       `tip::placeholders` reads them; an unclosed `{` and the rest of
       the text are copied as they are too;
    2. each ASCII letter outside them becomes the first of its accented
       forms that `can_draw` accepts, from a fixed table in the module
       (`a` → `á à â ä å ā`, `E` → `É È Ê Ë Ē`, and so on); a letter with
       none stays as it is, so an ASCII or 1-bit atlas gets plain letters;
    3. the text is lengthened to at least `w + ceil(w × 35 / 100)` cells,
       not counting the brackets, where `w` is `text_width` of the text
       outside placeholders: by doubling vowels from the start, then by
       `~` before the closing bracket if the vowels run out (whole
       numbers only);
    4. it is wrapped in `[` and `]`.
  - **Either order with 2301:** if it is done, `pseudo_text` and
    `with_pseudo` also take the atlas's width table and measure with
    `text_width` over it; if not, they use `text_width` as it is, and
    2301 adds the table when it changes `text_width`.
  - **Either order with 2348:** if it is done, `pseudo_text` reads a
    text with `Message::parse`. It keeps a message's syntax as it is
    (argument names and types, branch selectors, `#`) and changes only
    the words inside its branches, and the property test checks each of
    `Message::forms`. If not, 2348 makes `pseudo_text` do that when it
    lands (its clause says so).
  - `Lang::with_pseudo(self, can_draw: impl Fn(char) -> bool) -> Result<Self, String>`:
    adds a pack under `PSEUDO` with an entry for every English screen
    text, every data text and every dialogue line and caption
    (`LineText::One`), each with `source` today's English, and a
    `LangInfo` named `[Pseudo]`, `made_by: Machine`. It fails, naming the
    glyph or the code, if `can_draw` refuses `[`, `]` or `~`, or if a
    pack under `PSEUDO` exists.
  - **Either order with 2351:** if it is done, a game calls
    `with_pseudo` after `with_engine_english`, so the engine's own keys
    get pseudo text too, and the users below do so; if not, 2351 puts
    its `with_engine_english` calls before `with_pseudo`.
  - `LangPack::texts(&self) -> impl Iterator<Item = (&str, &str)>`: key
    and text of every screen and data entry and every one-text line, for
    the walk to name a key.
- **Builds with debug tools only.** `Ctx::effective_lang` treats
  `PSEUDO` as it treats the game's debug language: English unless
  `debug_tools` is on. A game adds the pack where it loads its text,
  under `shuyi_ui::DEBUG_TOOLS`. `GameData::debug_lang` is unchanged; a
  game keeps its own debug language if it has one.
  **Either order with 2349:** if it is done, its options row offers
  `PSEUDO` only with debug tools and its first-start pick never takes it
  from the system, as for the debug language; if not, 2349 does that.
- **Picking the language in tools:**
  - `Harness::set_lang(&mut self, code: &str) -> &mut Self` sets
    `ctx.lang` and redraws (`Game::redraw`, as `snapshot_as` does); a
    code that isn't a `LangCode` panics like a bad chord in a test
    script. A code with no pack shows English, as `Lang::text` does.
  - `Step::Lang(String)` in `shuyi_cli::frame_png`: played with
    `set_lang` in `Script::play`, checked by `Step::check`, read from
    `--lang <code>` by `Script::parse_step`. `frame_png::parse_args` and
    `contract::run::parse_args` pass `--lang` to `parse_step` beside
    `--keys`, `--pad` and `--wait`, so `frame-png`, `run` and `scene`
    take it. It is a step like the others: every step after it plays in
    that language. In the contract's JSON script it is
    `{"lang": "qps"}`, so the MCP server (1133) gets it too.
  - The contract's schema changes, so raise `contract::VERSION`,
    regenerate `crates/cli/schemas/` and add the line in
    `crates/cli/schemas/versions.json` (ADR-0022 §5).
- **The walk, `shuyi_platform::harness::walk`** (`crates/platform/src/harness/walk.rs`,
  behind the `harness` feature, `no_std` with `alloc` like the Harness):
  - `pub struct Stop<G: GameData> { pub screen: &'static str, pub reach: fn(&mut Harness<G>), pub skip: Vec<Rect> }`:
    a screen, the script that opens it from the start, and the cells
    not to read (a map, a picture), as `shuyi_ui::frame::Rect`s.
  - `pub fn walk_text<G: GameData>(start: impl Fn() -> Harness<G>, stops: &[Stop<G>], allow: &[&str]) -> Vec<TextProblem>`.
    For each stop: a fresh Harness from `start`, `set_lang(PSEUDO)`,
    text speed `TextSpeed::Instant` (through `Ctx::preview_settings`, so
    a text box has finished showing), `reach`, then it reads the glyph
    buffer's cells (`Game::buffer`) in reading order, outside `skip`.
    Brackets pair in reading order over the whole buffer, so a text
    wrapped over rows keeps its pair.
  - `TextProblem`, with `Display` naming the screen and the cell:
    - `Missed { screen, shown }`: the stop's script ended on another
      screen (`shown` is `Harness::top_screen`);
    - `Cut { screen, x, y, row, key }`: a `[` whose `]` never comes, or a
      `]` with no `[` before it, at cell `(x, y)`, with `row` the text of
      that row. This is text cut by its box (the `cut` helper in
      `shuyi_ui::frame::draw` drops the closing bracket with the tail),
      clipped at the buffer's edge, or drawn over. `key` names the
      pseudo text (`LangPack::texts`) that starts with what is shown up
      to its first placeholder, when exactly one does;
    - `NotFromKey { screen, x, y, text }`: a run of two or more letters
      (`char::is_alphabetic`) outside every bracket pair: text that
      didn't come through `Ctx::text` or the pack's data. Digits,
      punctuation, symbols, box drawing and single letters (a keycap, a
      map glyph) are never reported. A string in `allow` (the player's
      own name, say) is not reported where it stands on the row.
  - **Rule 9:** cut text is a look, so the walk reads cells, as tests of
    a look may (`AGENTS.md` rule 9).
  - **Either order with 2301:** if it is done, the walk skips wide-glyph
    continuation cells; if not, 2301 teaches it to.
  - **Either order with 2323 (parked):** if 2323 is done, its
    `check_text_sizes` and this walk share `Stop` and the cut finding,
    and the walk runs at every offered size; if not, 2323 builds its
    check on this walk.
- **Who runs it:**
  - the toy (`crates/ui/src/toy.rs`): `Toy::new` always adds the pack
    (it is a test game). It has no font, so its `can_draw` accepts every
    character and its text snapshots show the accents. A test in
    `crates/ui/tests/it/pseudo.rs` walks every screen the toy opens:
    its `home`, `menu`, `picker`, `gallery`, `lobby`, `talk` and
    `field`, and the engine's `options`, `key_bindings` and
    `layout_picker`. It calls `walk_text` once per start: `toy::game()`,
    `toy::lobby()`, and a `Field` game as `crates/ui/tests/it/grid.rs`
    makes one;
  - `examples/hello-grid`: `Hello::load_from` adds the pack under
    `DEBUG_TOOLS` with `|c| font.glyph_rect(c).is_some()`; a test in
    `examples/hello-grid/tests/it/pseudo.rs` walks `title`, `talk`,
    `stage` (its map in `skip`), `leave` and `end`, and the engine's
    `options` and `profiles` from the title;
  - the templates: the same line in the grid template's `Data::load`
    and in `BasicData::load` (`crates/ui/src/basic.rs`, 2511), so every
    game on the short path has the check, and a test in each template's
    `crates/game/tests/it/main.rs` that walks its screens. A game made
    with `shuyi new` has the check from its first commit.
    **Either order with 2501:** if it is done and the grid template
    loads through `BasicData`, the line in `BasicData::load` covers it;
    if not, the grid template keeps its own line, and 2501 drops it when
    it moves the template.
  - Fix what the walk finds in the engine's own screens
    (`crates/ui/src/screens/`, `crates/ui/src/widgets/`) and the toy's
    (its lobby's items are literals today, `Lobby::ITEMS`) when the fix
    is a few lines. Otherwise skip it with the stop's `skip` or the `allow`
    list, with a comment saying why, and write a ticket. Text an example
    doesn't translate yet (a dialogue scene it shows without
    `Lang::scene`) is skipped the same way.
- **Docs:** the `frame-png --help` text (`frame_png::usage`), `run`'s
  usage (`contract::run::usage`) and the steps paragraph of
  `docs/contract.md` name `--lang`; the doc comment of `walk` says how
  to list a game's stops.
- **For the game:** new items, a new `Step` variant and a new contract
  version. A game that matches `Step` needs an arm for `Lang`; a game
  that only builds steps needs nothing. Say so in the PR.
- **Size:** if the PR would pass about 600 changed lines (snapshots and
  schemas not counted), the walk and its runs on the toy, hello-grid
  and the templates (steps 5 and 6) go to a follow-up ticket written
  with the `write-ticket` skill (After MVP, blocked by 2353), with
  their acceptance criteria. The PR says so.

**Out (do not do):**

- The static overflow list per key (2302's `max_cells`) and the
  load-time box check for dialogue (2209): this walk is their runtime
  half, not a replacement.
- Reading 2329's `Layout::overflows()`: the walk reads what was drawn.
- Text drawn off the cell grid (sprites, 2334's text runs): a follow-up
  once 2334 is done.
- Markup inside text files: 2347 keeps its styles out of them (its
  Out), and the ticket that adds markup adds it to what `pseudo_text`
  copies as it is.
- Walking 2352's machine-translated engine packs: the walk finds cut
  text by the pseudo-language's brackets, which a real language lacks.
  2352's Out leaves cut text to this walk, whose pseudo text covers the
  engine's keys too.
- Fake right-to-left text (Godot's `fake_bidi`): right-to-left is the
  parked research 2908.
- Pseudo text made of wide glyphs, or a longer expansion for short
  labels.
- The pseudo-language in a build without debug tools: a game adds the
  pack only under `DEBUG_TOOLS`, and `effective_lang` shows English
  without them.
- Translator files (the parked 2227) and machine translation.

## Implementation steps

1. Read `crates/assets/src/lang.rs` (`Lang`, `LangPack`, `Entry`,
   `text_width`, `entries`' placeholder rule), `lang/lines.rs`
   (`LineEntry`, `LineText`), `lang/source.rs` (`SourceText`),
   `tip::placeholders`, `Ctx::effective_lang` and the Harness
   (`crates/platform/src/harness.rs`). Check the *Either order* tickets
   (2226, 2301, 2323, 2348, 2349, 2351, 2501) for which are done.
2. Write `pseudo_text` with its accent table and its tests, then
   `Lang::with_pseudo` and `LangPack::texts`.
3. Gate `PSEUDO` in `Ctx::effective_lang`, with a test beside the debug
   language's in `crates/ui/src/ctx/tests.rs`. Add the pack in
   `Toy::new`.
4. Add `Harness::set_lang`, `Step::Lang` and `--lang`; raise the
   contract's version and regenerate the schemas
   (`cargo shuyi schema --out crates/cli/schemas`).
5. Write the walk, `mod walk;` in `harness.rs`, and its tests in
   `crates/platform/tests/it/walk.rs`: two test screens on the toy
   (`Game::new(toy::ctx(), …, toy::hooks())`, as the other test screens
   in `crates/platform/tests/it/main.rs`), one that prints a literal and
   one that `cut`s a toy text key into a box too narrow for its pseudo
   text.
6. Add the walk to the toy's tests, then the pack and the walk to
   hello-grid, the grid template and `BasicData::load` with the blank
   template's test; fix or skip what it finds. The templates aren't
   formatted by `cargo fmt --all`: run the template job's checks too.
7. Write the docs, then run the gates.

## Acceptance criteria

- [ ] `pseudo_text` keeps every placeholder, starts with `[`, ends with
      `]`, is at least `w + ceil(w × 35 / 100)` cells without its
      brackets, and holds only characters `can_draw` accepts plus the
      text's own (property test).
- [ ] With an atlas of ASCII only, the pseudo text has no accented
      letter; with one that has `á`, it uses it (unit test).
- [ ] The pack `with_pseudo` makes covers every English screen text,
      data text and dialogue line, and `Lang::status` for the `PSEUDO`
      code lists nothing missing or stale (unit test).
- [ ] With `debug_tools` off, a `Ctx` whose `lang` is `PSEUDO` shows
      English (unit test).
- [ ] With `--lang` and the `PSEUDO` code, `frame-png` on the toy writes
      a picture other than the English one (`crates/cli/tests/it/toy.rs`),
      and `run` on hello-grid answers a `frame` that holds the pseudo
      text of `title.name`; `--lang 7` is an argument error for both
      (integration tests).
- [ ] On a test screen, a literal string is reported as `NotFromKey` and
      a label in a box too narrow for it as `Cut` naming its key, each
      with the screen's name (integration test).
- [ ] The walk finds no problem on the toy, hello-grid and both
      templates, with every `skip` and `allow` commented.
- [ ] `shuyi-assets`, `shuyi-ui` and `shuyi-platform` still build for
      `thumbv7em-none-eabihf` (3005's gate).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `pseudo_text` on placeholders, unclosed braces, text with no
  vowels, empty text and a line break; `with_pseudo`'s errors; the
  `effective_lang` gate; `Harness::set_lang`; `Step::check` and
  `Script::parse_step` with a bad code.
- Property: the `pseudo_text` criteria above over random text and random
  atlases; `with_pseudo` keeps the placeholder rule of
  `pack_from_files` (text and source have the same placeholders).
- Snapshot / integration: an insta text snapshot (`Harness::snapshot`)
  of the toy's options screen in the pseudo-language; `frame-png` and
  `run` with `--lang`; the walk on the toy, hello-grid and the
  templates; the walk failing on the test screens.

## Completion notes

*(Filled in by the session that completes the ticket.)*
