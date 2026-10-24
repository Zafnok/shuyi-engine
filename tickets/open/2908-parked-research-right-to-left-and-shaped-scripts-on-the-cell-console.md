---
id: "2908"
title: "Parked: Research: right-to-left and shaped scripts (Arabic, Hebrew, Thai, Devanagari) on the cell console and in text runs: what a cell holds, reordering, joining, line breaks, mirroring and the cost per target"
type: research
milestone: "Engine: localization"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2301"]
nick_input: none
completed:
---

# 2908 — Research: right-to-left and shaped scripts on the cell console

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need): the game doesn't ask for it, so this waits until after the MVP (`docs/ROADMAP.md`), and after 2301, whose wide cells it starts from. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engine Defold.

**What the benchmarks have.**

- Godot shapes text, runs the bidirectional algorithm, and mirrors a
  control's anchors and margins in a right-to-left language, with a
  `layout_direction` to override it
  (https://docs.godotengine.org/en/stable/tutorials/i18n/internationalizing_games.html).
- Unreal's Slate shapes text with HarfBuzz and reorders it with ICU.
- Unity's TextMeshPro has a right-to-left setting, and UI Toolkit in
  Unity 6 has a text generator for shaped scripts.
- Bevy's text goes through a Rust shaping library.
- Defold added text shaping to its engine in 1.12
  (https://forum.defold.com/t/defold-1-12-0-beta/81981).

**What Shuyi has today.** Text is drawn left to right, one `char` to a
cell, with nothing joined or reordered:

- `shuyi_ui::frame::glyph_buffer::Cell`
  (`crates/ui/src/frame/glyph_buffer.rs`, line 33) holds one
  `glyph: char` and is `Copy`, `Eq` and `Hash`. `GlyphBuffer::print`
  (line 631) puts one character in each cell, left to right.
- `shuyi_assets::lang::text_width` (`crates/assets/src/lang.rs`, lines
  53 to 58) is `text.chars().count()`. `print_centred` and `print_cut`
  (`crates/ui/src/frame/draw.rs`) count the same way.
- `shuyi_assets::text::word_wrap` (`crates/assets/src/text.rs`),
  re-exported as `shuyi_ui::widgets::word_wrap`, breaks at spaces only
  (`split_whitespace`), and cuts a word longer than the line into
  pieces.
- `shuyi_assets::font::FontAtlasDef` (`crates/assets/src/font.rs`) maps
  a `char` to one cell. `font-atlas` keeps only `ATLAS_RANGES`
  (`crates/cli/src/font_atlas.rs`): ASCII, Latin-1, Greek, punctuation,
  arrows and symbols. No Hebrew, Arabic, Thai or Devanagari, and no
  combining marks.
- Combining marks already break "one char, one cell": a decomposed `é`
  (`e`, then U+0301) takes two cells, and `shuyi-app`'s renderer draws
  the second as the fallback `?` in magenta (`crates/app/src/render.rs`,
  `atlas_cell`).
- `UiAction::Left` and `Right` (`crates/assets/src/keymap/action_set.rs`)
  have no reading direction. The options screen changes a value with
  them (`crates/ui/src/screens/options.rs`, lines 898 and 899), and the
  name grid moves across with them
  (`crates/ui/src/widgets/name_entry.rs`, lines 259 and 260).
- `shuyi_ui::text::fill` (`crates/ui/src/text.rs`) puts key names and
  values into a line of text, with no isolation from the text around it.
- Dialogue source text (`.dlg`) is plain ASCII
  (`crates/assets/src/dialogue/parse.rs`, around line 130). Translated
  packs have no such limit, so a translation is where these scripts would
  first appear.
- Before this ticket, a search of the code, the tickets and the docs for
  right-to-left, bidi, Arabic, Hebrew, Thai, grapheme and shaping found
  nothing (2026-10-20). Game ADR-0045 says that languages with wide or
  accented glyphs fit its text scheme, and "a right-to-left one would
  not".

**What is planned.**

- 2301 to 2303: wide glyphs for Japanese and Chinese (a wide character
  takes a cell and a continuation cell), wrapping without spaces, and
  sets of wide glyphs per language. 2301's `Out` points here and keeps
  its continuation mark behind one function, so this research can change
  what a cell holds in one place.
- 2350: the atlas keeps the characters the text uses, and text is
  checked against the atlas.
- 2224 and 2334: proportional fonts baked from TTF and OTF, drawn as text
  runs. Both leave shaping and right-to-left text to this ticket.
- 2329: layout with `Start` and `End`, which mean left and right on a
  horizontal axis. Its `Out` points here.
- 2346: names typed in any language, from letters the font draws.
- 2347: colour roles inside a line of text.
- 2348: plural rules per language. Arabic's rules wait for this
  research.
- 2226: `LangCode` as BCP 47 tags, whose script tells a reading direction.
- 2353: a pseudo-language walked over every screen. Its `Out` leaves a
  fake right-to-left language to this research.
- 3034 (parked): winit for text input and IME, and a 2D renderer over
  wgpu. Its ADR decides whether that renderer shapes text at run time
  through a library (either order with this ticket, scope 3).

**Why it matters.**

- **The game:** nothing. It plans Japanese and Chinese (2301 to 2303),
  not these scripts.
- **Other games:** Arabic, Persian, Urdu, Hebrew, Thai and Hindi have
  hundreds of millions of readers. Today a translator can't make a pack
  in any of them that reads correctly, whatever font the game ships. No
  one has decided at engine level whether that limit is meant.
- **The end goal** (2026-10-19): many ports. Whatever the engine does
  here, an English game on the Playdate must pay nothing for it.

This research weighs the choices and writes the ADR and the parked
tickets. It builds nothing that ships.

## Nick input

None. Unparking this ticket is Nick's call on whether the engine aims at
these languages; on 2026-10-20 he kept parked the gaps the game doesn't
need. The build tickets it writes are parked too, so starting each one
is his call. If the research finds a choice that changes what every game
ships (a larger web build by default, say), it asks Nick with the
`ask-nick` skill, with a recommendation.

## Scope

**In:** each question below is answered in the report with its options,
measurements where they matter, and a recommendation. The ADR decides.

1. **What a cell holds.** After 2301 a cell holds a `char`, and the
   second cell of a wide character holds a continuation mark. Weigh:
   - **Keep `char`,** and bring text down to single code points before
     it reaches a cell: NFC normalization (a decomposed `é` becomes
     `é`); Arabic letters to their presentation forms (U+FB50 to U+FDFF,
     U+FE70 to U+FEFF); a Thai or Devanagari cluster to a private-use
     code point that `font-atlas` bakes, as 2301 bakes the wide
     characters the packs use.
   - **A base `char` and a few marks** drawn in the same cell (Thai vowel
     signs and tone marks, Hebrew points), with `Cell` still `Copy`.
   - **A glyph id** from the atlas, with text turned into glyph ids
     before printing.

   For each: what changes in `Cell`, `GlyphBuffer::print`, `text_width`,
   2301's continuation cell, the text form of snapshots, `shuyi-app`'s
   renderer, the one-bit renderer (`crates/ui/src/frame/onebit.rs`),
   `frame-png`'s `Painter` (or `shuyi_ui::frame::raster` once 2332 is
   done), 2350's atlas check, 2346's name rules, and the game's code at
   its next pin. Prefer an option that adds to the API over one that
   breaks it.
2. **Reordering** (UAX #9). Packs keep text in logical order. Decide
   where the visual order is made: for each line after `word_wrap` has
   broken the text in logical order, in the glyph look. Views keep
   logical text, and a test of what happened reads the scene, not the
   cells (`AGENTS.md` rule 9). Cover:
   - numbers and Latin names inside Arabic or Hebrew text;
   - a value or key name that `text::fill` puts in, wrapped in isolates
     (U+2066 to U+2069) so it can't flip the sentence around it;
   - a line with colour roles (2347): each piece keeps its colour after
     reordering;
   - the caret and the typed text in a name box (2346).
3. **Joining and shaping.** Arabic letters change shape with their
   neighbours, and lam then alef is one ligature. Weigh:
   - presentation forms in the cell atlas, chosen by a joining table
     from Unicode's `ArabicShaping.txt`, at print time or at bake time;
   - shaping at tool time into glyph ids, for 2224's `text-font` and
     2334's runs;
   - shaping at run time through a library, only where the font is an
     outline: a run-time TTF format after 2222, or the wgpu renderer
     that 3034 plans, which may draw text through a library such as
     `parley` or `cosmic-text`. **Either order with 3034:** if it is
     done, its ADR says whether that renderer shapes text at run time,
     and this answer starts from it; if not, the report says which
     scripts need run-time shaping, for 3034 to use.

   Among shapers, weigh `harfrust`, which reads fonts through
   `read-fonts` (2224's `skrifa` brings it already), and `rustybuzz`.
   `rustybuzz` brings `ttf-parser`. `deny.toml` ignores its advisory
   (RUSTSEC-2026-0192, unmaintained) because macroquad brings it, and
   2224's acceptance keeps it off every other path
   (`cargo tree -i ttf-parser`). Devanagari conjuncts and vowel signs
   that move before their consonant need a real shaper: say whether
   cells can ever draw them, or only text runs.
4. **Line breaks.** Thai, Lao, Khmer and Burmese put no spaces between
   words and break by dictionary. Weigh:
   - break points found at tool time and kept in the pack as a zero-width
     space (U+200B), which `word_wrap` would learn to break at (its
     `split_whitespace` doesn't today) and the renderers draw as
     nothing, so no dictionary ships;
   - UAX #14 and a dictionary at run time (`icu_segmenter`), with its
     data measured in the `.wasm` and the Playdate build;
   - whether UAX #14 should one day replace the hand-written no-break
     sets of 2302 and 2303. Those stay keyed by language, whatever is
     chosen.
5. **Mirroring.** In a right-to-left language:
   - whether 2329's `Start` and `End` flip in a `Row`;
   - what `UiAction::Left` and `Right` do in the options screen's value
     rows, the name grid and 2355's focus map: mirrored or not, as Godot
     and the platforms' own toolkits choose;
   - which way the text box aligns, which way a meter fills, and where a
     titled box puts its title;
   - how a language says its direction: a field in `lang.ron`
     (`LangInfo`, where 2303 and 2346 add theirs), or the script of
     2226's BCP 47 tag. Most tags leave the script out (`ar` and `he`,
     not `ar-Arab`), so the tag alone needs a table from language to
     script.
6. **The cost per target, and opting out.** A table with a column for
   each of desktop, the web build, the Playdate (3010) and the GBA (3021,
   parked), and a row for each candidate crate and table: code size,
   data size (bidi data, joining tables, break dictionaries), memory, and
   whether it builds `no_std` with `alloc`, since `shuyi-assets` and
   `shuyi-ui` build for `thumbv7em-none-eabihf` (3005). A game in English
   pays nothing: a cargo feature or a module (ADR-0009), and data baked at
   tool time where it can be. For each crate: its license as published
   on the day, checked against `deny.toml` (MIT, Apache-2.0 and
   Unicode-3.0 are on its list), its last release, and default features
   that pull in more (ADR-0004 §7). Candidates to start from:
   `unicode-bidi`, `unicode-normalization`, `unicode-segmentation`,
   `unicode-linebreak`, `icu_segmenter`, `harfrust` and `rustybuzz`.
7. **A spike,** in a scratch folder or a throwaway branch, deleted
   after: one Arabic line and one Thai line drawn on the cell grid at
   8 × 16 through `frame-png`'s `Painter`, after the reordering and
   joining that questions 2 and 3 recommend. The font has those scripts
   under a license on `deny.toml`'s list and is not committed: GNU
   Unifont's BDF, taken under its OFL-1.1 license, say (check its
   license files on the day). On the branch, add Arabic (U+0600 to
   U+06FF), its presentation forms (U+FB50 to U+FDFF, U+FE70 to U+FEFF)
   and Thai (U+0E00 to U+0E7F) to `ATLAS_RANGES`, bake the atlas
   with `font-atlas`, print the lines into a `GlyphBuffer`, and draw it
   with `Painter::new` and `Painter::render`. If 2334 is done, the same
   lines as text runs too. Each picture is compared by eye with the same
   text in the same font in a web browser, which shapes and reorders it.
8. **Outputs:**
   - a report, `docs/research/2908-right-to-left-and-shaped-scripts.md`,
     with the options, the cost table and the spike's findings, the facts
     dated, and the specifications read: UAX #9, UAX #14, UAX #29,
     `ArabicShaping.txt`, and the OpenType script guides for Arabic, Thai
     and Devanagari;
   - an ADR (the `write-adr` skill) that decides questions 1 to 6 and
     says, for each script family (Hebrew; Arabic, Persian and Urdu; Thai
     and its neighbours; Devanagari and the other Indic scripts), whether
     the cells, the text runs, both or neither draw it;
   - `Parked:` build tickets, milestone "Engine: localization", each one
     PR, with `blocked_by` in order, and their rows in
     `docs/ROADMAP.md`'s *Parked* section. Likely ones: a language's
     direction and mirrored layout; reordering per line; Arabic joining;
     breaking text without spaces; a right-to-left pseudo-language for
     2353's walk; Arabic's plural rules as a row of 2348's table;
     the engine's own screens in Hebrew and Arabic, machine-translated
     as 2352 does, which leaves them to this research: if 2352 is done,
     a pack per language on its `engine_packs()`; if not, the same
     ticket, with 2352 in its `blocked_by`. Either way it is blocked by
     the tickets that draw the script;
   - the line about what text the cells draw in `docs/formats.md` (which
     2301 writes) updated to match the ADR.

**Out (do not do):**

- Building any of it.
- Changing the tickets it reads: wide glyphs and breaking for Japanese
  and Chinese (2301 to 2303), proportional fonts and text runs (2224,
  2334), language tags (2226), layout (2329), the name box (2346) and
  the wgpu renderer's text (3034). The report proposes their hooks, and
  the parked tickets it writes make them.
- Vertical text, furigana and ruby (2302's `Out`).
- Emoji and colour fonts.
- Typing these scripts. IME composition and the caret on macOS, Linux
  and the web wait for winit (3034). The report notes only what the name
  box (2346) would need.
- Which languages a game ships: that is the game's call.
- Copying a library's code. Whatever the engine writes itself (a joining
  table, a reordering pass) comes from Unicode's specifications and data
  files, as ADR-0021 §2 asks of the platform layer. Reading a library's
  documentation and examples to judge it is fine.

## Implementation steps

1. Read 2301's cells as built: `crates/ui/src/frame/glyph_buffer.rs`
   (`Cell`, `print`, the continuation mark),
   `crates/ui/src/frame/draw.rs`, `crates/assets/src/text.rs`
   (`word_wrap`), `crates/ui/src/text.rs` (`fill`),
   `crates/assets/src/lang.rs` (`text_width`, `LangInfo`),
   `crates/assets/src/font.rs`, `crates/cli/src/font_atlas.rs`,
   `crates/ui/src/frame/onebit.rs`, `crates/app/src/render.rs` and
   `crates/cli/src/frame_png.rs`. Read whichever of 2224, 2226, 2302,
   2303, 2329, 2332, 2334, 2346, 2347, 2348, 2350, 2355 and 3034 are
   done.
2. Read the specifications (scope 8) and list them in the report.
3. Survey the crates (scope 6), and fill the table.
4. Measure the shortlist on a throwaway branch: for each crate, the
   size it adds to `examples/hello-grid`'s `cargo shuyi web --release`
   `.wasm`, with one call into it so it isn't stripped, and to a
   `cargo build --no-default-features --target thumbv7em-none-eabihf`
   of a scratch crate that calls it (or the error that stops it building
   there).
5. Do the spike, and put its pictures in the PR.
6. Write the report and the ADR. Ask Nick only as *Nick input* says.
7. Write the parked tickets, their roadmap rows, and the `docs/formats.md`
   line.
8. Run the gates.

## Acceptance criteria

- [ ] The report answers questions 1 to 6, each with its options, a
      recommendation and the facts dated.
- [ ] The cost table has measured sizes for the web build and the
      `thumbv7em-none-eabihf` build for each shortlisted crate (or the
      error that stops it building there), and each crate's license is
      checked against `deny.toml`.
- [ ] The PR shows the spike's Arabic and Thai lines beside a browser's
      rendering of the same text, and no spike code or font is committed.
- [ ] The ADR decides questions 1 to 6 and names, for each script family,
      the path that draws it (or that none does), and
      `docs/adr/README.md` has its row.
- [ ] Every ticket written is one PR's worth, `Parked:`, milestone
      "Engine: localization", with its `blocked_by` chain, and
      `docs/ROADMAP.md`'s *Parked* section has its row.
- [ ] `docs/formats.md` says what text the cells draw, as the ADR
      decides.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None shipped (research). The tickets it writes say their tests, and
  each one that reorders or joins text has a property test that the
  logical text a scene holds never changes.

## Completion notes

*(Filled in by the session that completes the ticket.)*
