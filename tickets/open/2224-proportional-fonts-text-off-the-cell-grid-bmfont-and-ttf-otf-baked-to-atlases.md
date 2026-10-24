---
id: "2224"
title: "Proportional fonts: an ADR for text off the cell grid, a font kind of its own, BMFont read directly, and TTF/OTF baked by the font tools"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2224 — Proportional fonts: text off the cell grid, BMFont, and TTF/OTF baked to atlases

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker, Defold and Construct 3.
Every one of them loads TrueType and OpenType fonts and draws
proportional text anywhere on the screen. Godot also reads BMFont files
and draws outlines and shadows
(https://docs.godotengine.org/en/stable/tutorials/ui/gui_using_fonts.html).
Bevy has `Text2d` for text in the world and `TextShadow` for UI text.
Defold's fonts are made from TTF, OTF or BMFont files. Most free and
bought pixel fonts ship as TTF, OTF or BMFont, not BDF.

What Shuyi has today:

- `shuyi_assets::font::FontAtlasDef` (`crates/assets/src/font.rs`): one
  atlas of fixed cells, a map from `char` to cell index. A game hands it
  to the platform as `Look::font` (`crates/platform/src/platform.rs`).
- `font-atlas` (`crates/cli/src/font_atlas.rs`) reads BDF only
  (`parse_bdf`, `read_font`) and writes `atlas.png`, `atlas.ron` and
  `atlas_1bit.bin`. `one_bit` refuses cells wider than 8 pixels, so
  `run` does too.
- ADR-0004 §2, the Fonts row: "BDF, through `shuyi-cli font-atlas`".
- ADR-0007 §3: "A game never has to convert a common file before using
  it." Fonts are already the exception: a BDF is converted by
  `font-atlas`.
- ADR-0003 §5 ends: "Menus, text boxes and help bars stay glyph cells in
  every style (game ADR-0038 §3)". That is the game's rule, copied into
  the engine.
- The frame's items are `Item::Rect` and `Item::Sprite`
  (`crates/ui/src/frame/glyph_buffer.rs:305`).
- `deny.toml` ignores RUSTSEC-2026-0192: `ttf-parser`, which reaches us
  through `fontdue` through macroquad, is unmaintained.

What is planned:

- 2301 and 2303 give the cell font wide glyphs.
- 2350 makes `font-atlas` keep the characters the game's text uses
  (`AtlasChars`), and `shuyi check` fail on text the atlas can't draw.
- 2338 draws picture frames on panels, with an ADR that extends ADR-0003
  §5 while panel text stays in cells.
- 2323 (parked) scales the cells. Its Out list leaves a second cell
  font to a follow-up, and sends a sharper big font off the grid here
  and to 2334.
- 2329 and 2330 lay widgets out.
- 2722 (parked) draws visual-novel text in cells because of ADR-0003 §5,
  and as text runs (2334) when 2334 is done and its `vn.ron` names a
  font.

No ticket covers a font that isn't cells.

Why it matters:

- **The game:** nothing it draws changes. It stays all glyph cells by
  its own rule (game ADR-0038 §3). `FontAtlasDef` and `read_font` are
  untouched. The one API change is the new parameter of
  `font_atlas::run`: the PR says so, with a `Breaking:` line (2010), and
  the game's PR that moves its pin follows it.
- **Other 2D games:** a score, a "+100" over an enemy, a title or a name
  needs text off the grid, in the font the developer has. Nick
  (2026-10-14, choosing the engine's name, `docs/decisions.md`): glyph
  "is underselling our 2D capabilities (we should still be able to
  support glyphs)". On 2026-10-20 he put the features most 2D games
  expect after the MVP, not parked.
- **The end goal** (2026-10-19): an agent asked for a game uses the font
  it finds, without hunting for a BDF or converting by hand.

This ticket writes the ADR and the asset half: the kind, its formats
and the tools. 2334 draws it (text runs) and builds fonts by id.

## Nick input

None.

## Scope

**In:**

- **The ADR** (the `write-adr` skill), at the next free number, e.g.
  `docs/adr/NNNN-text-off-the-cell-grid-proportional-fonts-and-text-runs.md`.
  It decides:
  1. **Text and UI off the grid are allowed.** This replaces ADR-0003
     §5's last line. The engine's widgets keep their glyph look by
     default, and no engine screen changes. A game may draw text and UI
     anywhere in console pixels: text runs (2334), sprites and
     rectangles. "Menus stay glyph cells" stays the game's rule (game
     ADR-0038 §3). **Either order with 2338:** if 2338's ADR (picture
     frames on panels) is accepted first, this ADR cites it and replaces
     what is left of the line; if not, 2338's ADR cites this one.
  2. **A proportional font is a kind of its own**,
     `font::ProportionalFont`, beside `FontAtlasDef`, not a
     generalisation of it. A cell font finds a glyph by its place in a
     grid, 2301 and 2303 reshape it for wide cells, and ADR-0019 §1 makes
     each kind a type.
  3. **Its formats.** Standard ones (ADR-0007 §3): the engine's own RON
     form, and BMFont's text `.fnt`, whose pages are PNG files the
     platform reads like any picture. At tool time: TTF and OTF, through
     `font-atlas` (cells) and `text-font` (proportional), written as the
     engine's RON and a PNG that the game checks in, as BDF is today.
     This extends ADR-0004 §2's Fonts row, and is an exception to
     ADR-0007 §3 as BDF already is. Why TTF is not read at run time:
     - a pixel font is drawn at one size, chosen when it is made;
     - the platform reads pictures as files (`AppConfig::files` in
       `crates/app/src/lib.rs`, `Look::images`), so pixels made while
       loading have nowhere to go until 2222 packs pages in memory;
     - a baked file sits at its source's path (ADR-0023 §2), so the pack
       can't turn a TTF into page pictures beside it either;
     - the web build, the Playdate and the GBA never carry an outline
       parser or a rasterizer.
     A run-time TTF format is a follow-up after 2222, if a game needs
     sizes chosen while it runs.
  4. **The crates**, in `shuyi-cli` only, each with
     `default-features = false` and only the features named (ADR-0004
     §7):
     - `skrifa` 0.48 (`std`) for outlines, metrics and the character
       map: MIT OR Apache-2.0. It brings `read-fonts` 0.45 and
       `font-types` (MIT OR Apache-2.0), `bytemuck` (Zlib OR Apache-2.0
       OR MIT), `foldhash` 0.2 (Zlib), `once_cell` (MIT OR Apache-2.0),
       `yoke`, `yoke-derive`, `zerofrom` and `zerofrom-derive`
       (Unicode-3.0), `stable_deref_trait` (MIT OR Apache-2.0) and
       `synstructure` (MIT). `deny.toml` allows all of these.
       `foldhash` 0.1 is already in the tree, so `cargo deny` warns of
       two versions; `multiple-versions` is a warning there, not an
       error.
     - `ab_glyph_rasterizer` 0.1 (`std`) for coverage: Apache-2.0, no
       dependencies.
     - `write-fonts` 0.54 (`tables`), a dev-dependency for the fixture
       font: MIT OR Apache-2.0. It uses the same `read-fonts` 0.45 as
       `skrifa`, and brings `kurbo`, `indexmap` and `log` (MIT OR
       Apache-2.0).
     - Not `fontdue` or `ab_glyph`: both bring `ttf-parser`. `zeno`
       (MIT OR Apache-2.0) is the alternative considered.
  5. **Fonts by id**, built by 2334: `FontTable` in `shuyi_assets::font`
     (like `ImageTable`), `FontId` (`Copy`, like `ImageId`), each font's
     pages resolved to `ImageId`s at `AssetPath::sibling` of the font's
     path, and `GameData::fonts()` returning an empty table by default.
  6. **Text in the frame**, built by 2334: a text run is laid out into
     ordinary `Sprite` items, each tagged with its run, plus a `TextRun`
     record in the `GlyphBuffer` that the snapshot prints as one line.
     There is no new `Item` variant:
     - `Item` is `Copy + Eq + Hash`, so it can't own the run's string;
     - every painter already draws sprites (`shuyi-app`, `frame-png`,
       the one-bit renderer, and the Playdate and GBA backends after
       them).
     The colour is `Paint::Solid`, or the page's own colours with
     `Paint::Image`. An outline is the glyph drawn one pixel away in
     eight directions. A shadow is one copy at an offset.
  7. **Out:** rasterising at run time, SDF and MSDF, shaping,
     right-to-left text (2908), emoji, styled spans (2347), and kerning
     read from a TTF's `kern` or `GPOS` tables.
  - The `Status` lines of ADR-0003, ADR-0004 and ADR-0007 name the new
    ADR and the part each one loses or gains. `docs/adr/README.md` gets
    the new row and the changed ones.
- **`font::ProportionalFont`** in `shuyi-assets`
  (`crates/assets/src/font/proportional.rs`, re-exported from `font`),
  `no_std`, integers only, `AssetKind::NAME` `"proportional font"`:
  ```rust
  pub struct ProportionalFont {
      pub line_height: u16,   // one line's top to the next, in pixels
      pub baseline: u16,      // a line's top to its baseline
      pub pages: Vec<String>, // the pages' file names, beside the font file
      pub glyphs: BTreeMap<char, Glyph>,
      pub kerning: BTreeMap<(char, char), i16>, // added to the advance
  }
  pub struct Glyph {
      pub page: u16,
      pub rect: AtlasRect,    // in its page, in pixels
      pub offset: (i16, i16), // from the pen and the line's top to the rect
      pub advance: u16,
  }
  ```
  - `Debug, Clone, PartialEq, Eq` and serde derives, for the RON form and
    the pack's binary form (ADR-0023 §6); `AtlasRect` gains serde derives
    too;
  - `problems(required) -> Vec<String>`: a line height of 0, a baseline
    below the line, a glyph on a page the font lacks, a glyph of
    `required` missing, a kerning pair naming a glyph the font lacks;
  - `page_problems(page, width, height) -> Vec<String>`: a glyph's rect
    outside its page.
- **`font::ProportionalFontRon`**: id `proportional-font`, extension
  `ron`, `requiring(glyphs)` as `FontAtlasRon` has. The file names its
  pages by file name (`"title_0.png"`), and `pages` keeps them as
  written. The format finds each page with `AssetPath::sibling`, reads
  its PNG header with `DecodeCx::bytes` and `png_size`
  (`crates/assets/src/image.rs`), and checks it with `page_problems`. A
  page the bundle lacks is an error that names the page's path, as
  `FontAtlasDef::from_source` names a missing PNG.
- **`font::bmfont::BmFont`** (`crates/assets/src/font/bmfont.rs`): id
  `bmfont`, extension `fnt`, sniffing `info `, the AngelCode text form
  (https://www.angelcode.com/products/bmfont/doc/file_format.html):
  - a line is a tag and `key=value` pairs; a value is a number, a comma
    list or a quoted string that may hold spaces;
  - it reads `common` (`lineHeight`, `base`, `pages`, `packed`), `page`
    (`id`, `file`), `char` (`id`, `x`, `y`, `width`, `height`,
    `xoffset`, `yoffset`, `xadvance`, `page`) and `kerning` (`first`,
    `second`, `amount`), and skips other tags and keys;
  - its pages are found and checked as the RON form's are;
  - a kerning pair naming a character with no `char` line is dropped,
    since it can never apply;
  - it refuses, naming the line: `packed=1` (glyphs packed into colour
    channels), an `id` that isn't a Unicode scalar value, a page id past
    `pages`, a number that doesn't fit its field;
  - it refuses the binary form (`BMF` magic) and the XML form, with a
    message that says to export the text form;
  - `no_std`, no new dependency, `requiring(glyphs)` like the RON form.
- Both formats in `Formats::register_standard`
  (`crates/assets/src/format.rs`), limited to `TEXT_MAX_BYTES`, with rows
  in `docs/formats.md`: the standard formats table, "What each one
  reads", `requiring` under the formats configured with the game's
  rules, and the kinds of ADR-0007 §1, where `Font` becomes
  `FontAtlasDef` for cells and `ProportionalFont` for text runs.
- **`shuyi import <file>.fnt --print`** prints the engine's RON form:
  `ProportionalFont` joins `Importers::standard()`
  (`crates/cli/src/contract/import.rs`) through `kind_with_text`, with a
  summary such as "95 glyphs, 12 pixels a line, 1 page" and its text
  from `text_font::font_ron`, the writer `text-font` uses.
- **The pack:** the RON fixture and its page join `sources()`,
  `every_kind()` and the pinned binary forms (`form!`) in
  `crates/assets/src/pack/tests.rs`. The snapshot changes, so
  `pack::VERSION` (`crates/assets/src/pack.rs`) becomes the next number,
  as the test's comment says.
- **TTF and OTF in the tools**, `crates/cli/src/font_atlas/outline.rs`
  (std):
  - `rasterize(bytes, px, smooth, wants: impl Fn(char) -> bool) ->
    Result<Outlines, String>`: each glyph of the font's character map
    that `wants` keeps, drawn unhinted at `px` pixels per em through
    `skrifa`'s `OutlinePen` into `ab_glyph_rasterizer`.
    `Outlines { line_height, baseline, glyphs: BTreeMap<char, Raster> }`
    and `Raster { left, top, w, h, advance, alpha: Vec<u8> }`, with
    `left` and `top` from the pen and the line's top. The metrics are
    rounded to whole pixels: the line height from ascent, descent and
    line gap, the baseline from ascent, an advance per glyph. A pixel
    half covered or more is opaque (255) and the rest clear; with
    `smooth`, the coverage is kept as alpha. Bytes that aren't a font
    are an error, never a panic;
  - the characters: for `font-atlas`, those of `ATLAS_RANGES` (what
    `build_atlas` keeps); for `text-font`, those of `ATLAS_RANGES` plus
    `--glyphs`. **Either order with 2350:** if 2350 is done, the
    characters its `AtlasChars` wants (`font_atlas::AtlasChars::wants`:
    the ranges, the characters the game's text uses and the required
    set, never a combining mark): `font-atlas` passes the `AtlasChars`
    it builds, and `text-font` passes `AtlasChars::default()` and adds
    `--glyphs`; if not, as above, and 2350 hands `read_any` its
    `AtlasChars` when it lands (its scope says so).
  - **cells:** `read_font(path)` stays as it is, BDF only, because
    Hello, Grid's test (`examples/hello-grid/tests/it/files.rs`) calls
    it, and the game may. A new `font_atlas::read_any(path, outline:
    Option<OutlineSize>) -> Result<(BitmapFont, Vec<char>), String>`
    reads by extension: `.bdf` through `read_font`, `.ttf` and `.otf`
    through `rasterize`, anything else refused with the three it reads.
    The `Vec<char>` lists the glyphs it cut.
    `OutlineSize { px: u16, cell: Option<(u32, u32)> }` comes from
    `font-atlas <font>... <out-dir> --px <n> [--cell <w>x<h>]`. A TTF or
    OTF without `--px` is refused with a message that asks for it. The
    cell is the widest advance by the line height, unless `--cell` says
    otherwise; each glyph sits at its `left` and `top` in its cell. A
    glyph past its cell is cut, and the summary names it. It builds a
    `BitmapFont`, so `build_atlas`, `one_bit` and the RON header don't
    change (`one_bit` still refuses cells wider than 8 pixels, as it does
    for a BDF).
    `run` reads every font through `read_any` and gains the
    `outline: Option<OutlineSize>` parameter. **Either order with
    2301:** if 2301 is done, `--wide` reads its font through `read_any`
    too, with the wide cell as its `--cell`; if not, 2301 does it (its
    scope says so).
  - **proportional:** a new command, `text-font <font.ttf|.otf>
    <out-dir> --px <n> [--smooth] [--glyphs <chars>]`
    (`crates/cli/src/text_font.rs`). It runs in a project, as
    `font-atlas` does, for the command its header names, but needs no
    part of `shuyi.ron`. It writes `<stem>.ron` (a `ProportionalFont`,
    with a header comment naming the source file and the command, as
    `atlas.ron`'s has) and one page `<stem>_0.png`, white with the
    glyphs' alpha:
    - the page is 256 pixels wide, or as wide as the widest glyph;
    - glyphs go in code-point order, left to right, one pixel apart; a
      new row starts when the next glyph doesn't fit, one pixel below
      the tallest glyph of the row above;
    - a page taller than `MAX_SIDE` (4096, `crates/assets/src/image.rs`)
      is refused; more pages are a follow-up if a font needs them;
    - it fails if the font lacks a glyph of `--glyphs`;
    - `text_font::font_ron(&ProportionalFont) -> String` writes the RON
      body with LF line endings, as `atlas_ron` does.
  - usage lines for both in `crates/cli/src/lib.rs`.
- **Fixtures, all our own** (no row in `THIRD_PARTY_ASSETS.md`):
  - a tiny TTF built in the tests with `write-fonts`
    (`pub(crate) fn fixture_ttf()` in
    `crates/cli/src/font_atlas/outline/tests.rs`, which `text-font`'s
    tests use too):
    1,000 units per em, an ascent of 875, a descent of 125 and no line
    gap, and glyphs whose outlines are squares on a 125-unit grid: `A`,
    `B` and `?` 5 pixels wide with an advance of 6, a space, and `W` 10
    pixels wide with an advance of 11. At `--px 8` the line is 8 pixels,
    the baseline 7, and each square one whole pixel. No font file is
    committed;
  - a BMFont `.fnt` with two pages and a kerning pair, and its RON twin,
    as `pub(crate)` constants in `crates/assets/src/format/builtin_tests.rs`
    beside `ATLAS`, with their pages' headers in `fixtures()`
    (`png_header`);
  - `text-font`'s reference outputs for the fixture TTF at `--px 8`,
    committed in `crates/cli/src/text_font/`.
- **Either order with 2020:** if 2020 is done, put the fixture pages'
  PNG headers in `fuzz::fixtures()`'s static table, where 2020 moved
  `builtin_tests.rs`'s `fixtures()`; add `Packed<ProportionalFont>` to
  `fuzz::registry()`; and add `proportional-font` and `bmfont` to
  `fuzz::formats`' walk inside `fixtures()`, each at its fixture's path
  (2020's skills' lines). If not, nothing to do: 2020 adds them.

**Out (do not do):**

- text runs, fonts by id and anything in the frame (2334);
- changing `FontAtlasDef`, the cell atlas's format or any engine screen
  (2301 and 2303 change the atlas);
- the characters the game's text uses, and `shuyi check` on text the
  atlas can't draw (2350);
- picture frames on panels (2338) and a text size the player picks
  (2323, parked);
- TTF or OTF read at run time (a follow-up after 2222, if a game asks);
- BMFont's binary and XML forms, and a proportional BDF (`DWIDTH`) as a
  text font (follow-ups if a game asks);
- the Playdate's own `.fnt` text format, which shares the extension (a
  follow-up if a Playdate game asks; `BmFont`'s sniff tells them apart);
- kerning from a TTF, hinting, SDF and MSDF;
- a fuzz target of its own: 2020's harness walks every registered
  format, and the clause with 2020 above says who adds these two; this
  ticket adds the property tests every built-in format has;
- shipping any font: a game picks and licenses its own.

## Implementation steps

1. Read `crates/assets/src/font.rs`, `format.rs` (`Format`, `AssetPath`,
   `DecodeCx`, `register_standard`), `format/builtin_tests.rs`,
   `pack/tests.rs`; `crates/cli/src/font_atlas.rs`, `lib.rs`
   (`font_atlas`, `usage`), `contract/import.rs` and its tests;
   `docs/formats.md`; ADR-0003 §5, ADR-0004 §2 and §7, ADR-0007 §3,
   ADR-0023 §2 and §5.
2. Write the ADR, the `Status` lines of ADR-0003, ADR-0004 and
   ADR-0007, and the rows in `docs/adr/README.md`.
3. Add `ProportionalFont`, `Glyph`, their `AssetKind` and their checks,
   with unit tests.
4. Add `ProportionalFontRon` and `BmFont`, register them, and add the
   fixtures, the property tests in `format/builtin_tests.rs`, and the
   pack's fixture, forms and `VERSION`.
5. Add `text_font::font_ron`, the `import` kind and summary (update
   `the_standard_kinds_are_the_engines`), and the `docs/formats.md`
   rows.
6. Add `skrifa`, `ab_glyph_rasterizer` and `write-fonts` (dev) to
   `crates/cli/Cargo.toml`, each with a comment saying what it is for.
   Run `cargo deny check` and `cargo tree -i ttf-parser`.
7. Write `fixture_ttf()` and `rasterize`, then `read_any`, `OutlineSize`
   and the flags of `font-atlas`, then `text-font`, with their usage
   lines.
8. Run `text-font` on the fixture, look at the page, and commit the
   reference outputs.
9. Run the gates.

**Size:** about 1,200 to 1,400 changed lines without fixtures, past one
PR. Steps 1 to 5 (the ADR, `ProportionalFont`, `ProportionalFontRon`,
`BmFont`, the `import` kind and the pack) stay in this PR. When the diff
runs long, steps 6 to 8 (TTF and OTF in the tools: `rasterize`, `read_any`,
`font-atlas --px`/`--cell`, `text-font`, the three new dependencies and
the fixture TTF) move to a follow-up ticket of the same milestone (the
`write-ticket` skill, blocked by this one), which takes the either-order
clauses with 2350 and 2301. The completion notes say so.

## Acceptance criteria

- [ ] The ADR is written, and ADR-0003's, ADR-0004's and ADR-0007's
      `Status` lines and `docs/adr/README.md` name it.
- [ ] The BMFont fixture and its RON twin load through
      `Formats::standard()` as equal `ProportionalFont`s (test).
- [ ] A rect outside its page, a page the bundle lacks, a missing
      required glyph, `packed=1` and a binary `.fnt` are each an error
      that names the file, and for a `.fnt` the line (tests).
- [ ] Random bytes and mutated fixtures never panic either format (the
      property tests every built-in format has).
- [ ] `shuyi import` of the `.fnt` fixture with `--print` prints a RON
      form that `ProportionalFontRon` reads back equal (test).
- [ ] The pack's pinned binary forms include `ProportionalFont`, and
      `pack::VERSION` is the next number.
- [ ] `text-font` on the fixture TTF writes a PNG and a RON byte-equal
      to the committed references. CI runs the tests on Windows, Linux
      and macOS, so the output is the same on all three.
- [ ] Each fixture glyph's opaque pixels are exactly the squares it was
      drawn with (a test of a look, reading pixels).
- [ ] `read_any` on the fixture TTF at `--px 8` gives 11 by 8 cells;
      with `--cell 8x8` it cuts `W` and lists it; a TTF without `--px`
      is refused (tests).
- [ ] `font-atlas <fixture.ttf> <out-dir> --px 8 --cell 8x8` writes an
      atlas that `FontAtlasDef::from_source` accepts, and its summary
      names `W` as cut (test).
- [ ] `read_font` keeps its signature, and every BDF test and output is
      unchanged (Hello, Grid's `files` test among them).
- [ ] `cargo deny check` passes, `cargo tree -i ttf-parser` shows only
      macroquad's path, and `shuyi-assets` still builds for
      `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `ProportionalFont::problems` and `page_problems`, one case each;
  each BMFont tag, quoted values with spaces, a dropped kerning pair, and
  each refusal; page names resolved beside the font file; `rasterize` on
  bytes that aren't a font; the cell size from a TTF, with `--cell` and
  a cut glyph; `read_any` by extension, and without `--px`; `text-font`
  refusing a font that lacks a `--glyphs` character.
- Property: random bytes and mutated fixtures never panic
  `ProportionalFontRon` or `BmFont`; a decoded font always passes its own
  checks.
- Snapshot / integration: the `text-font` references; the `import
  --print` output read back; the pack's pinned binary form of the
  fixture.

## Completion notes

*(Filled in by the session that completes the ticket.)*
