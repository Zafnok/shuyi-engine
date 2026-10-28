---
id: "2346"
title: "Typing names in any language: letters the font draws, name grid pages from the language pack with an accented page, and the IME on only in the name box"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2346 — Typing names in any language

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8.

**What the benchmarks have.**
- Godot's `LineEdit` takes text in any script, and its `DisplayServer`
  turns the IME (the input method a Japanese or Chinese player types
  with) on and off per window and places its window
  (`window_set_ime_active`, `window_set_ime_position`;
  https://docs.godotengine.org/en/stable/classes/class_displayserver.html).
- Unity's `Input.imeCompositionMode` is `Auto` by default: the IME is on
  only while a text field has focus
  (https://docs.unity3d.com/ScriptReference/Input-imeCompositionMode.html).
- Bevy's window, through winit, has `ime_enabled` and `ime_position`.
- Console naming screens (Dragon Quest, Pokémon) spell a name on a
  letter grid with pages: capitals, small letters, accented letters,
  kana.

**What Shuyi has today.**
- `crates/ui/src/widgets/name_entry.rs` (2310): `NameBox` (typed) and
  `NameEntry` (the letter grid). `NameRules::allows` (lines 54 to 61)
  takes only `is_ascii_alphabetic` letters, digits, `-` and `'`.
  `LETTER_ROWS` (lines 159 to 164) is A to Z and a to z. So "Zoë",
  "José" or "Jürgen" gets the Denied sound, typed or on the grid.
- Elsewhere the engine already counts Latin-1 and Latin Extended-A and
  B as letters: `is_latin_letter` in `crates/assets/src/names.rs`
  (line 197).
- The font atlas tool keeps ASCII and Latin-1 (`ATLAS_RANGES` in
  `crates/cli/src/font_atlas.rs`). Hello, Grid's atlas
  (`examples/hello-grid/files/font/atlas.ron`) draws `é`, `ü` and `ñ`,
  and no kana.
- `crates/app/src/keys.rs` (line 139) passes every typed character on as
  `RawInputEvent::Text`. `Harness::type_text` sends a character no key
  makes as `Text` alone.
- The pinned miniquad 0.4.11 (`Cargo.lock`) turns a Windows IME's
  committed text into those characters (`WM_IME_COMPOSITION` with
  `GCS_RESULTSTR`), so kana already arrive, and `NameRules` throws them
  away. Windows draws the composition window itself.
- **A bug:** miniquad leaves the IME on for the window from the start,
  and `shuyi-app` never turns it off (nothing in `crates/` calls
  `set_ime_enabled`). On Windows, a player with a Japanese or Chinese
  IME in its typing mode can have letter keys go to the IME instead of
  the game during play. miniquad 0.4.11 has `window::set_ime_enabled`
  and `window::set_ime_position`. They work on Windows; on macOS, X11
  and Wayland they do nothing yet, and the web build ignores them.
- Where the device has a keyboard of its own (Steam's on the Steam
  Deck, a console's), a screen asks it through `Ctx::system_keyboard`
  (3003, `crates/ui/src/system_keyboard.rs`). 3012 rules out a
  keyboard drawn by the engine.

**Planned.**
- 2301 to 2303 draw Japanese and Chinese (game#0236, game#0239). 2301
  adds a project set of wide glyphs "so name entry and tests don't
  depend on a translation". Nothing lets a player type them.
- game#0042 asks Nick how a player names a character in Japanese: a
  kana grid, Latin letters only, or no entry.

**Why it matters.** The game plans Japanese and Chinese text, and
players with accented names can't type them today. Any game with a
"name your hero" or "name your farm" box in French, German or Spanish
needs the same. Where there is no keyboard and no system keyboard (the
Playdate, the GBA and DS targets, a pad on a plain PC) the grid is the
only way to type, so its pages must hold the language's letters.

## Nick input

None.

## Scope

**In:**

1. **Letters of any language** (`crates/ui/src/widgets/name_entry.rs`):
   - `NameRules` keeps its two fields (`max_chars`, `digits`). What it
     allows becomes: any letter (`char::is_alphabetic`) the game's font
     draws, `-` and `'`, ASCII digits where the rules allow them, and
     single spaces between words. `×`, `÷`, punctuation and symbols stay
     refused.
   - The length is counted with `shuyi_assets::lang::text_width` (one
     per character today). **Either order with 2301:** whichever lands
     second passes 2301's width table (its `CellWidths`), so a wide
     letter counts as two.
   - **`Ctx` learns what the font draws:** a public field `font_chars:
     Option<BTreeSet<char>>` in `crates/ui/src/ctx.rs` (`BTreeSet` from
     `alloc`, as the crate builds without `std`). `None`, the default,
     draws everything. `App::load` (`crates/platform/src/app.rs`), when
     it has a look, fills it with the characters the look's atlas draws
     (`FontAtlasDef::drawn`, below) once the game is made, so the
     windowed and web builds refuse a letter that would show as the
     atlas's `FALLBACK_GLYPH` (`?`). `App::running` has no look, so the
     Harness keeps `None`; a test that needs a font sets the field
     itself.
   - **One answer to "can the atlas draw this character?"**:
     `FontAtlasDef::draws(&self, c: char) -> bool` and `drawn(&self) ->
     impl Iterator<Item = char>`, the characters `draws` accepts in
     code-point order (`crates/assets/src/font.rs`). Today that is the
     keys of `glyphs`; 2301 adds its wide glyphs and 2303 every wide
     set, in these two functions only, so a kana the atlas draws is in
     `font_chars` with no change here. **Either order with 2350:** if
     2350 is done, it added both for `font::undrawn`: fill `font_chars`
     from its `drawn`; if not, add them here, and 2350's `undrawn` calls
     `draws`. **Either order with 2353:** if 2353 is done, Hello, Grid's
     `can_draw` (`Hello::load_from`) becomes `|c| font.draws(c)`, unless
     2350 already made it so; if not, nothing to do (2353 calls
     `draws`).
   - Names already saved load as they are: the rules apply only while
     typing.
   - The module's docs (lines 10 to 12) say the new rules.
2. **Grid pages as data** (`crates/assets/src/lang.rs`):
   - `LangInfo` (a pack's `lang.ron`) gains `name_grid: Vec<GridPage>`
     with `#[serde(default)]`, so every existing pack reads as before.
     Not `skip_serializing_if`: a baked pack holds postcard, which needs
     every field.
   - `GridPage` is one of:
     - `Latin`: today's four rows (`LETTER_ROWS` moves here, with its
       `// check-text: not player text` comment);
     - `Accented`: the 62 Latin-1 letters, U+00C0 to U+00FF without `×`
       and `÷`: the 30 capitals in code order, then the 32 small letters
       (`ß` first) starting a new row, 13 to a row. That is six rows
       (13, 13, 4, 13, 13, 6);
     - `Rows(Vec<String>)`: a pack's own letters, one string per row.

     `GridPage::rows()` gives a page's rows. A French pack says
     `name_grid: [Latin, Accented]`.
   - Validation in `pack_from_files`, as errors on the pack's
     `lang.ron`: at most 8 pages; a `Rows` page has 1 to 6 rows of 1 to
     13 characters; every character is a letter (`char::is_alphabetic`),
     `-` or `'`; no character twice in one page. Each error names the
     page by its number from 1, and the row too for a row's rule. Six
     rows is the `Accented` page's count: with the digit row and the
     bottom row the grid's box is then 20 cells tall
     (`glyph::grid_height`), which fits Hello, Grid's 24-row console
     under the help line. Eight would fill it.
   - `LangPack`'s binary form changes, so `pack::VERSION`
     (`crates/assets/src/pack.rs`) goes up by one, as its doc says, and
     the snapshot of
     `the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version`
     (`crates/assets/src/pack/tests.rs`) is accepted again.
   - `docs/formats.md`'s language pack bullet names the field.

   This is a field of an existing format (ADR-0007, ADR-0019): no new
   kind and no new format.
3. **Pages on the grid** (`NameEntry`):
   - `Ctx::name_grid(&self) -> &[GridPage]`: the `name_grid` of the
     pack for `effective_lang()` (`self.game.lang().pack(..)`), or
     `[GridPage::Latin]` when there is no pack or its `name_grid` is
     empty. English has no `lang.ron`, so English keeps today's one page
     and every existing snapshot stays the same. **Either order with
     2226:** if it is done, `Ctx::name_grid` takes the first pack along
     `Lang::chain(effective_lang())` that has a `name_grid`; if not, the
     pack for `effective_lang()` as written here, and 2226 moves it to
     the chain.
   - `NameEntry::with_pages(self, pages: Vec<GridPage>) -> Self`.
     `NameEntry::new` keeps one `Latin` page. `ProfilesScreen::open_name`
     (`crates/ui/src/screens/profiles.rs`) takes no `Ctx` today: it
     gains a `ctx: &Ctx<G>` argument (its two callers, `confirm` and
     `act`, have one) and passes `ctx.name_grid().to_vec()` to the grid.
   - A page shows its letter rows, then the digit row (where the rules
     allow digits), then the bottom row. With more than one page, the
     bottom row gains a `GridCell::Page` cell before Done, labelled by
     the text key `name_entry.grid.page` ("Page"). Confirm on it shows
     the next page (after the last, the first), with the focus on the
     new page's Page cell. With one page there is no Page cell, as
     today.
   - A cell whose character the font doesn't draw (`Ctx::font_chars`)
     is left out of its row, and a row left empty is left out.
   - `name_entry::grid(rules)` stays as it is (the `Latin` page).
   - The new text key: **either order with 2351.** If 2351 is done, it
     goes in the engine's built-in English. If not, it goes in the toy's
     English (`crates/ui/src/toy/en.ron`) and Hello, Grid's
     (`examples/hello-grid/files/en.ron`), and 2351 moves it with the
     other `name_entry.*` keys.
   - The toy's debug pack (the `info` string `crates/ui/src/toy.rs`
     reads as `dbg/lang.ron`) adds `name_grid: [Latin, Accented]`, so
     the tests have a language with two pages.
   - **Either order with 2354:** if its `GridCursor` is done, the paged
     grid uses it; if not, `NameEntry` keeps its own cursor and 2354
     moves the paged grid onto `GridCursor`.
4. **The IME on only in a typing box:**
   - **Logic says when** (`crates/ui`): `Ctx::takes_text(&mut self)`
     marks that a text box takes typed text this frame, and
     `Ctx::is_taking_text()` reads the mark. `Game::frame`
     (`crates/ui/src/game.rs`) clears it before the screens update and
     reports it as a new field, `FrameOutput::typing`. `NameBox::handle`
     calls `takes_text`; the grid doesn't (it reads actions). A game's
     own text box calls it too.
   - **The skin says where** (`AGENTS.md` rule 9):
     `GlyphBuffer::set_text_caret(x, y)` and `text_caret() ->
     Option<(i32, i32)>`, in cells. Filling the whole buffer with a solid
     cell (how a frame is cleared) clears it, as it clears the backdrop.
     `name_entry::glyph::paint_typing` sets it on the cell after the
     typed name, where it prints `_`. The text form of snapshots doesn't
     show it.
   - **The platform** (`crates/platform`): `Frame` gains `typing:
     bool` (false in `Frame::new`), which `present_game`
     (`crates/platform/src/app.rs`) sets from `ctx.is_taking_text()`,
     as it sets `fullscreen` from the settings. The headless backend
     keeps it in `Presented::Game`, and `Harness::typing()` reads it,
     like `Harness::fullscreen()`. A test reads the caret from
     `Harness::game().buffer().text_caret()`.
     `docs/platform-backends.md`'s `present` section says what a backend
     does with it.
   - **`shuyi-app`** (`crates/app/src/backend.rs`):
     - it remembers what it last told the IME, starting from miniquad's
       default (on), so its first frame turns the IME off;
     - when `frame.typing` changes it calls
       `miniquad::window::set_ime_enabled`;
     - while typing, when the caret cell moves it calls
       `set_ime_position` with the window pixel just under that cell
       (so the IME's list doesn't hide the name), placed as `render.rs`
       places the console: `shuyi_ui::frame::console::layout`, in
       physical pixels. **Either order with 2333:** if it is done, its
       placement function instead;
     - with no caret it only turns the IME on;
     - the decisions are a pure function, tested without a window.
   - On macOS, Linux and the web these calls do nothing in miniquad
     0.4.11; the backend's docs say so.
   - **Either order with 2338:** it gives `paint_typing` a frame
     argument; the caret line is the same either way.
5. If the PR would pass about 600 changed lines (fixtures and snapshots
   not counted), part 4 goes to a follow-up ticket written with the
   `write-ticket` skill (After MVP, not parked, blocked by 2346), with
   steps 5 to 7 below and the acceptance criteria on
   `Harness::typing()` and on `shuyi-app`'s IME function. The PR says
   so.

**For the game:** mostly additive. `GridCell` gains a variant;
`LangInfo`, `Presented::Game` and `FrameOutput` gain a field, so code
that matches them exhaustively or builds them adds the new one (`Frame`
gains one too, but it is `#[non_exhaustive]`). Unless 2351 is done, the
game's English gains `name_entry.grid.page`, which a language with two
pages shows. A pack baked before this is refused with a message to bake
it again (`pack::VERSION`). The PR says so, with a `Breaking:` line
(2010), and the game's PR that moves its pin follows it. On
Windows the game's players get the IME off during play. If the game
answers game#0042 with a kana grid, its Japanese pack lists the pages
as `Rows` once 2301 is done.

**Out (do not do):**
- Kana pages and a dakuten cell (a cell that voices the kana before
  it): they need wide glyphs (2301) and the game's answer in game#0042.
  Then a pack lists kana as `Rows` with no engine change (10 to a row
  fits the 46-cell box once a kana is two cells wide); a dakuten cell
  is a follow-up.
- Pinyin or other grids for Chinese (2303's Out keeps "typing
  Chinese").
- The IME's composing text drawn in the box (preedit, underlined), and
  the IME on macOS, Linux and the web: miniquad 0.4.11 has neither.
  Parked 3034's research names text input and the IME over winit.
- The clipboard and paste: a name box doesn't need them. Chat (1149,
  parked) and the editor (2507, parked) will, when they start.
- The profiles screen asking the system keyboard (`Ctx::system_keyboard`)
  when there is one, and checking its answer with the same rules: 2380
  (blocked by this one). It checks a whole name with the rule for one
  typed character, so keep that rule in one place (`NameRules`).
- A keyboard drawn by the engine (3012's Out: Steam's keyboard on the
  Deck).
- A page action or shoulder buttons for the grid: pages turn through
  their cell, so there is no new `UiAction` (the `input-actions` skill).
- Accents on the English grid, and the grid of a source language other
  than English (2226): a follow-up if a game asks.
- Which characters the atlas keeps (Latin Extended-A's `Œ`, `Ł`, `Ő`):
  2350's.
- Checking a pack's grid characters against the atlas. **Either order
  with 2350:** if 2350 is done, add each pack's pages
  (`GridPage::rows` of `LangInfo::name_grid`) to its scan as
  `TextPart::NameGrid`; if not, 2350 adds them. Until then a cell the
  font lacks is left out at run time (In, 3).
- The mouse and touch on the letter grid: 2409, 2405's follow-up for
  the engine's other widgets. If 2409 is done first, the new pages'
  cells and the Page cell take the pointer as the other cells do.

## Implementation steps

1. Read `name_entry.rs`, its glyph skin and its tests,
   `screens/profiles.rs`, `system_keyboard.rs`, `lang.rs` (`LangInfo`,
   `pack_from_files`), `pack.rs` (`VERSION`), `platform.rs` (`Frame`),
   `app.rs` (`load`, `present_game`), `headless.rs` (`Presented`),
   `harness.rs`, `crates/app/src/backend.rs` and `render.rs`, and
   miniquad 0.4.11's `window::set_ime_enabled` and `set_ime_position`.
2. `GridPage` and `LangInfo::name_grid`, the validation,
   `pack::VERSION` and the pinned binary form snapshot, and
   `docs/formats.md`.
3. `FontAtlasDef::draws` and `drawn` (unless 2350 added them),
   `Ctx::font_chars`, the new letter rule, the module's docs, and
   `App::load` filling it.
4. Pages in `NameEntry`, `GridCell::Page`, `Ctx::name_grid`,
   `open_name`'s new argument, the text key and the toy's debug pack.
5. `Ctx::takes_text`, `FrameOutput::typing`, the text caret and
   `paint_typing`.
6. `Frame::typing`, the headless backend, `Harness::typing`, and
   `docs/platform-backends.md`.
7. The IME in `shuyi-app`: the pure function, then its calls. If the
   machine has a Japanese or Chinese IME, try Hello, Grid by hand (play
   with the IME in its typing mode, then type a profile name) and say in
   the PR what was seen; if not, say it wasn't tried.
8. Run the gates.

## Acceptance criteria

- [ ] Typing "Zoë", "José" and "Jürgen" in the name box keeps them with
      no Denied sound; `×` and `÷` are refused (unit tests).
- [ ] With `font_chars` lacking `ë`, typing it sounds Denied and leaves
      the name as it was. With a set holding `ゆ` and `き`, typing "ゆき"
      keeps it; with `font_chars` set from Hello, Grid's atlas
      (`Hello::font.drawn()`) it is refused (tests).
- [ ] `App::load` with a look fills `Ctx::font_chars` with the look's
      glyphs; `App::running` leaves it `None` (unit test in
      `crates/platform`).
- [ ] A `lang.ron` without `name_grid` reads as before; one with
      `[Latin, Accented, Rows([...])]` reads; each validation rule
      (9 pages, 7 rows, 14 characters in a row, a symbol, a character
      twice in a page) has a failing case whose error names the page,
      and the row for a row's rule (unit tests).
- [ ] A pack with a `name_grid` reads the same from a baked pack as
      from its source, and `pack::VERSION` is one more than before.
- [ ] `GridPage::Accented` holds exactly the 62 Latin-1 letters, each
      `is_alphabetic`, in six rows of at most 13 (unit test).
- [ ] In English the grid has one page and no Page cell, and every
      existing snapshot is unchanged.
- [ ] In the toy's debug language (with `debug_tools` on) the Page cell
      turns to the accented page and back with the focus on it, and
      "Zoë" is spelled across the pages (unit test). A snapshot of the
      accented page is committed and reviewed.
- [ ] A cell the font doesn't draw is left out of its page (unit test).
- [ ] In Hello, Grid, `Harness::typing()` is false on the title, true in
      the typing name box with the text caret on the cell after the name,
      false on the grid, and false again after Done
      (`examples/hello-grid/tests/it/profiles.rs`).
- [ ] `shuyi-app`'s IME function turns the IME off on the first frame,
      on with a position when typing starts, moves only the position
      when the caret moves, turns it off when typing ends, and calls
      nothing when nothing changed (unit test).
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the letter rule (accented letters, symbols, the font's set, the
  length); `App::load` filling `font_chars`; `GridPage` and its
  validation; a pack with a `name_grid` read from a baked pack;
  pages and the Page cell; cells left out; the typing mark cleared each
  frame; the caret cell `paint_typing` sets, and a full fill clearing
  it; the IME function and the caret's window pixel from a `Layout`.
- Property: whatever characters are typed into a `NameBox`, the name it
  keeps holds only characters the rules allow and is never longer than
  `max_chars` by `text_width`.
- Snapshot / integration: the accented page of the grid; Hello, Grid's
  typing flag and text caret through the Harness; a profile named "Zoë"
  added by typing.

## Completion notes

*(Filled in by the session that completes the ticket.)*
