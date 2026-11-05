---
id: "2252"
title: "The text checks measure with the font atlas's wide glyphs, and a reloaded atlas changes the frame's widths"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-04
---

# 2252 — The text checks measure with the font atlas's wide glyphs

## Context

- **From 2301 (done):** a font atlas may hold wide glyphs
  (`shuyi_assets::font::FontAtlasDef::wide`), two cells each, and
  `shuyi_assets::lang::text_width(text, &CellWidths)` counts a wide
  character as two. The frame (`GlyphBuffer::widths`) and the context
  (`Ctx::widths`) get the atlas's widths once the game has loaded
  (`Game::set_widths`, called by `shuyi_platform::App` from the look's
  atlas), so screens and widgets lay text out by its cells.
- **What 2301 left narrow:** the checks that keep text to a length run
  as the text loads, before anything has the atlas, and measure with
  `CellWidths::NARROW`:
  - `shuyi_assets::dialogue::check::longest_width` and `token_cells`
    (`crates/assets/src/dialogue/check.rs`), and through them
    `lines_at_longest`, `Checker::step` and a pack's `add_lines`
    (`crates/assets/src/lang/lines.rs`);
  - `Names::longest` (`crates/assets/src/names.rs`);
  - a tip's title and lines (`crates/assets/src/tip.rs`, `parse`).

  So a Japanese pack's line of 20 wide characters passes a 30-cell
  limit it overflows on the screen.
- **And one thing at run time:** a hot reload of the atlas
  (`shuyi-app`'s `Renderer::remake`, `Remake::Font`) gives the renderer
  the new atlas, but the frame keeps the widths it was given at load.
- No friction review: found while doing 2301.
- **Outside tactics:** any game with Japanese or Chinese text and a text
  box, a tip or a name limit.

## Nick input

None.

## Scope

**In:**
- The length checks above take a `&CellWidths` and measure with it.
- Where the game hands the rules over, a way to give the widths: a
  field `widths: CellWidths` on `lang::PackRules` and on `tip::TipRules`
  (both plain structs with public fields: adding one breaks a game that
  builds them, so a `Breaking:` line), and the dialogue `Checker` takes
  them where it takes its `DialogueLimits`.
- `shuyi check` and the game's tools measure with the atlas's widths:
  `GameTools::font_atlas` is there already (2350); the text checks run
  again over the loaded text with `CellWidths::of(&atlas)`, or the
  atlas loads first and its widths go into the rules. Pick one, say why
  in the completion notes.
- The template's and Hello, Grid's loads pass the atlas's widths
  (`CellWidths::of`) into their rules.
- A hot reload of the font atlas calls `Game::set_widths` with the new
  atlas's widths (`crates/platform/src/app.rs`, where the App tells the
  game of changed files, or `shuyi-app`'s `backend.rs`).

**Out (do not do):**
- Line breaking for text without spaces (2302).
- Widths by language (2303).
- Any change to how a cell draws.

## Implementation steps

1. Give `longest_width`, `token_cells`, `lines_at_longest`,
   `Names::longest` and the tip check a `widths: &CellWidths`, and pass
   `CellWidths::NARROW` where nothing has an atlas yet (the engine's own
   tests).
2. Add `widths: CellWidths` to `PackRules` and `TipRules`, and thread it
   to the checks (`add_lines`, `Checker`, `tip::parse`).
3. In the template (`templates/shuyi-default`) and `examples/hello-grid`,
   load the font atlas before the text and pass its widths.
4. On a reload of the font atlas, `Game::set_widths(CellWidths::of(…))`.
5. Unit tests: a wide character counts as two in each check.
6. Say in `docs/formats.md`, at the language pack, that its limits count
   a wide character as two cells.

**Either order with 2239:** if 2239 is done, `box_fit`
(`crates/assets/src/dialogue/check.rs`) measures with `longest_width`
and `lines_at_longest`: give it the `CellWidths` too, and its two
callers pass the table; if not, nothing to do (2239 passes it, as its
clause says).

## Acceptance criteria

- [x] A pack's speech line of wide characters over `max_text` cells
      fails its check, though its characters are fewer (unit test).
- [x] A tip and a name of wide characters are measured as two cells a
      character (unit tests).
- [x] `shuyi check` measures with the atlas's widths (test).
- [x] A reloaded atlas with a new wide glyph prints it in two cells
      (test).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each check with a wide character.
- Snapshot / integration: `shuyi check` on a pack of wide text; a hot
  reload of the atlas in the Harness.

## Completion notes

- **The checks take the widths.** `dialogue::longest_width` and
  `lines_at_longest` (and the private `token_cells`) take a
  `&CellWidths`; `Names::longest` too. The dialogue `Checker` takes the
  widths from a new `SceneRules::widths`, beside `limits`, whose default
  is `CellWidths::NARROW`, so a game's own `SceneRules` keep compiling.
  A pack's `add_lines` measures its lines and its names with
  `PackRules::widths`, and a tip's title and lines with
  `TipRules::widths`.
- **Deviation: `widths: &'a CellWidths`, not `CellWidths`.** `PackRules`
  is `Copy` and games build it as a `const`; `CellWidths` holds an `Rc`,
  so an owned field would have taken `Copy` away too. A reference keeps
  both (`&CellWidths::NARROW` works in a `const`). `TipRules` takes a
  reference the same way, to match.
- **The language-pack format.** `LangPackRon` builds its `PackRules`
  itself, and a `Format` must be `Sync`, which `CellWidths` isn't. It
  keeps the wide characters as a set and makes the widths as it reads a
  pack: `LangPackRon::widths_of(&atlas)` (new; an addition).
- **`shuyi check`: the atlas loads first, and its widths go into the
  rules.** The other way, running the text checks again over the loaded
  text in the CLI, can't work: the limits (`DialogueLimits`, `TipLimits`,
  a tip's rules through `data_problem`) are the game's, and `GameTools`
  hands over the text, not them. A game's `GameTools::check` runs its
  loader, so a loader that passes `CellWidths::of(&atlas)` makes `check`
  measure with them. Test: `check_measures_the_text_with_the_font_atlass_wide_glyphs`
  (`crates/cli/src/contract/check.rs`).
- **Deviation: the template and Hello, Grid had nothing to pass.**
  Neither builds a `PackRules`, `TipRules` or `SceneRules`: they load no
  language packs or tips, and Hello, Grid reads its dialogue with the
  plain `Dlg` format, which runs no length check. Step 3 is a no-op; the
  docs and the book say to load the atlas before the text, for a game
  that does.
- **The box's lines.** `word_wrap` still counts characters (2302 makes it
  count cells), so `lines_at_longest` writes each wide character as `WW`
  before wrapping: the cells are counted right, but no line breaks
  between two wide characters yet. 2302 now says to pass the widths to
  `word_wrap` and drop the `WW`s.
- **Hot reload.** After the platform tells the game of changed files,
  `App` hands the game the widths of its look's atlas
  (`use_look_widths`, also used at load). `Game::set_widths` changes and
  redraws nothing when they are the same. Test:
  `a_reloaded_font_atlas_with_a_new_wide_glyph_prints_it_in_two_cells`
  (`crates/platform/src/app/tests.rs`), on the headless platform the
  Harness drives.
- **2239** (`box_fit`) isn't done: its clause on this ticket now says
  what holds, that `box_fit` takes the `&CellWidths` its callers pass.
- The error messages still say "characters" where they count cells
  ("text is 82 characters; the limit is 80"); changing the wording was
  out of scope.
- Docs: `docs/formats.md` (the language pack's limits count cells;
  hot reload sets the widths) and the book's *Assets and formats*.

Breaking: shuyi-assets: lang::PackRules: a new field `widths: &CellWidths`; a game that builds one adds `widths: &CellWidths::NARROW`, or its atlas's `&CellWidths::of(&atlas)`.

```migration
since: 0.1.0
what: A language pack's rules say how many cells each character takes, so its limits count a wide character as two.
old: "PackRules { data_problem, tokens, limits, variants }"
new: "PackRules { data_problem, tokens, limits, widths, variants }"
before: |
  const RULES: PackRules<'static> = PackRules {
      data_problem,
      tokens: &NoTokens,
      limits: LIMITS,
      variants: &[],
  };
after: |
  let widths = CellWidths::of(&atlas);
  let rules = PackRules {
      data_problem,
      tokens: &NoTokens,
      limits: LIMITS,
      widths: &widths,
      variants: &[],
  };
```

Breaking: shuyi-assets: tip::TipRules: a new field `widths: &CellWidths`; a game that builds one adds it, the atlas's widths or `&CellWidths::NARROW`.

```migration
since: 0.1.0
what: A tip's rules say how many cells each character takes, so its title and lines count a wide character as two.
old: "TipRules { limits, placeholder, face_problem, default_face }"
new: "TipRules { limits, widths, placeholder, face_problem, default_face }"
before: |
  let rules = TipRules {
      limits,
      placeholder: &placeholder,
      face_problem: &face,
      default_face: "calm",
  };
after: |
  let widths = CellWidths::of(&atlas);
  let rules = TipRules {
      limits,
      widths: &widths,
      placeholder: &placeholder,
      face_problem: &face,
      default_face: "calm",
  };
```

Breaking: shuyi-assets: dialogue::longest_width, dialogue::lines_at_longest, names::Names::longest: each takes a last argument `widths: &CellWidths`; a game that calls them passes its atlas's widths, or `&CellWidths::NARROW` for one cell a character.

```migration
since: 0.1.0
what: The text measures count a wide character of the font atlas as two cells.
old: longest_width(text, tokens, longest_name); lines_at_longest(text, tokens, longest_name, width); Names::longest()
new: longest_width(text, tokens, longest_name, widths); lines_at_longest(text, tokens, longest_name, width, widths); Names::longest(widths)
before: |
  let longest = names.longest();
  let cells = longest_width(text, &tokens, Some(longest));
after: |
  let widths = CellWidths::of(&atlas);
  let longest = names.longest(&widths);
  let cells = longest_width(text, &tokens, Some(longest), &widths);
find: ["names.longest()"]
```
