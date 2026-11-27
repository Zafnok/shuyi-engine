---
id: "3413"
title: "Names agents guess resolve: doc aliases from the other-engine table and a plain-text API per crate"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-27
---

# 3413 — Names agents guess resolve: doc aliases from the other-engine table and a plain-text API per crate

## Context

Found by the blind-spot audit of 2026-11-23 (its *Agent experience*
section, candidates 63 and 68). An agent writes a game's code from
memory. It guesses the names other engines use, and the paths it has
seen in other crates. Here most guesses miss, and each miss costs a Rust
compile round.

**What is true today** (checked on this branch, 2026-11-23):

- `grep -rn 'doc(alias' crates` finds **0** attributes. Searching the
  public API for the usual words finds nothing: no `pub fn` named
  `draw_sprite`, `draw_text`, `draw_texture`, `is_key_down`,
  `just_pressed`, `spawn` or `load_scene`, and no type named `Transform`,
  `Vec2`, `TileMap`, `Texture`, `Entity` or `Timer`. The real names are
  `GlyphBuffer::add_sprite` (`crates/ui/src/frame/glyph_buffer.rs:818`),
  `Sprite` (`:263`), `FrameInput::presses` and `held`
  (`crates/ui/src/input/frame.rs:133`, `:145`), `GridMap`
  (`crates/assets/src/grid_map.rs:184`), `Pos` (`crates/core/src/grid.rs:17`),
  `Countdown` (`crates/core/src/timer.rs:111`) and `Tween`
  (`crates/core/src/ease.rs:477`).
- The API exists for agents only as rustdoc HTML. `llms.txt` links each
  crate as "rustdoc's HTML" (`crates/xtask/src/llms.rs:1504`). The `api`
  job (2010, 2041, [ADR-0038](../../docs/adr/0038-the-public-api-gate-a-listing-diff-and-semver-checks-on-every-pr-and-a-breaking-line-for-each-break.md))
  already builds each crate's listing with `cargo-public-api` on the
  pinned nightly (`api::NIGHTLY`, `crates/xtask/src/api.rs:264`), but
  publishes nothing.
- The book's appendix A, *Coming from another engine*
  (`site/content/learn/book/from-another-engine.md`), maps Godot, Unity,
  Bevy, GameMaker and PICO-8 words to Shuyi's in a hand-written table.
  Nothing checks it, and it has drifted: its tween row says "none yet",
  but 2109 (done) shipped `Tween`.
- `shuyi::prelude` (`crates/shuyi/src/lib.rs:45`) re-exports
  `shuyi_platform::prelude` (`crates/platform/src/lib.rs:108`, about 30
  names, ADR-0036 §6) and `run_game`. Past it, the templates and
  `examples/hello-grid` import from **about 74** distinct module paths
  (counted with `grep -rhoE "use shuyi[a-z_]*(::[a-z_0-9]+)+::(\{|[A-Z*])"
  examples templates --include=*.rs`), such as
  `shuyi::ui::frame::glyph_buffer`, `shuyi::core::bots::playtest`,
  `shuyi::assets::grid_map`, `shuyi_cli::contract::playtest` and so on.
  A rules crate depends on `shuyi-core` alone, and `shuyi-core` has no
  prelude.
- The public names mix spellings. `Color`, `EngineColor`, `ColorSet` and
  `Colors` are US. Eight names are UK: `print_centred`
  (`crates/ui/src/screens.rs:44` and `frame/draw.rs:205`),
  `print_centred_styled` (`screens.rs:53`), `centre_x` (`screens.rs:37`),
  `centre` (`crates/ui/src/cinematic.rs:192`, `debug/camera.rs:125`),
  `centred_on` (`crates/ui/src/grid/camera.rs:21`), and the variants
  `NarrationAlign::Centred` (`dialogue/text_box/glyph.rs:26`),
  `Scroll::Centred` (`widgets/list.rs:20`) and the one-bit check's
  `Rule::SharedColour` (`crates/assets/src/onebit.rs:523`). The `centre`
  family is used about 246
  times in 64 files outside `tickets/` (the book's tutorial holds 24 of
  those files). An agent writes `print_centered` and misses.
- The precedent for a rename: 2226 kept `Lang::english` one release as a
  `#[deprecated]` alias of `source_text`, with one `#[expect(deprecated)]`
  test per alias, and needed no `Breaking:` line.

**Why it matters:**

- **The game first.** Nothing it uses is removed. `gh search code --repo
  Zafnok/visions-of-shuyi` (pin `50472bbc`) finds `print_centred` in 8
  files (`crates/ui/src/screens/*/glyph.rs`), `centre_x` in 8,
  `centred_on` in 6 (`map_view/place.rs`, `screens/battle/mod.rs`) and
  `NarrationAlign` in `screens/dialogue/glyph.rs`. The old names stay as
  deprecated aliases, so the game builds. Its PR that moves the pin
  renames its calls, as each deprecation warning points at one.
- **Other games and agents** find `draw_sprite`, `is_key_down` or
  `TileMap` in rustdoc's search, grep one text file per crate, and import
  from a few preludes instead of guessing deep paths.
- **The end goal** (Nick, 2026-10-19, [`docs/decisions.md`](../../docs/decisions.md)):
  an agent makes snake in few lines. Every wrong name is a wasted round.

Whether this joins the MVP, before the API freezes at the first release
(2005), is Nick's question in 3528. This ticket is written to land
either way: nothing is removed.

## Nick input

None. The spelling of identifiers and the aliases are technical choices,
recorded in an ADR.

## Scope

**In:**

- **The table as data.** `site/data/engine-terms.toml`: one
  `[[term]]` per row of appendix A's table, with the Shuyi words, the
  Rust paths they name, the chapter link, each engine's word, and the
  `aliases` the first path carries. A row Shuyi has no counterpart for
  names the open ticket that plans it, or says `planned = false`. A
  `[[guess]]` list beside it holds names no row covers (`draw_sprite`,
  `draw_texture`, `is_key_down`, `just_pressed`, `TileMap`, `Vec2`,
  `Timer`, `Texture` …), each with the path it resolves to.
- **The page reads the data.** A shortcode `engine_terms`
  (`site/templates/shortcodes/engine_terms.md`, `load_data`) draws the
  table, and `cargo xtask llms` expands it in the Markdown twins (its
  rule: every shortcode has an expansion, `llms.rs:16`). The tween row
  is fixed: `Tween`, linked to chapter 2's *Things that move*
  (`frame-and-skins.md`).
- **Doc aliases.** `#[doc(alias = "…")]` on each item the data names,
  for each of its aliases. Rustdoc refuses an alias with a space, so the
  data's `aliases` hold identifiers only; the engines' words with spaces
  stay in the table's text.
- **The plain-text API.** `cargo xtask api-text --out <dir>` writes
  `<dir>/<crate>.txt` for each crate of `site/data/api.toml`: one line
  per public item, its `cargo-public-api` listing line (the signature),
  then its first doc sentence, its aliases, and `deprecated: <note>` when
  it is. pages.yml writes it into `site/public/api/`, and `llms.txt`
  links each crate's text beside its HTML.
- **The checks** (`cargo xtask api-text --check`, in CI's `api` job,
  which already has the nightly and `cargo-public-api`):
  - every path in `engine-terms.toml` resolves to a public item;
  - every alias the data names is on that item, and no `doc(alias)` in
    `crates/` is missing from the data;
  - a "none yet" row's ticket is in `tickets/open/` (a done one means
    the row is out of date: the tween case).
- **Moved to 3438 at the split:** the domain preludes, the import
  check, one spelling of identifiers (the ADR, the US names beside
  deprecated UK ones) and `--check`'s spelling check.
- **First doc sentences.** Each item that carries an alias starts its
  doc with the usual word ("A sprite: part of an image drawn at a pixel
  rectangle"), then the house wording.
- **Public types** (ADR-0039): no new type, and none changes shape.

**Either order with 2845:** if 2845 is done, check the paths in
`engine-terms.toml` with its index of rustdoc names and read the docs
and aliases from it; if not, build that index here
(`crates/xtask/src/rustdoc_index.rs`: rustdoc JSON on `api::NIGHTLY`,
read with `serde_json`) and 2845 reuses it.

**Moved to 3439 at the split:** feeding the plain-text API into 3412's
docs index (`HitKind::Api`, a tracked copy of the `.txt` files).

**Either order with 2108:** if 2108 is done, move its entity words from
the page's table into `engine-terms.toml`; if not, nothing to do (2108
writes them in the data file).

**Either order with 2142:** if 2142 is done, move its stats row from the
page's table into `engine-terms.toml`; if not, nothing to do (2142
writes it in the data file).

**Out (do not do):**

- Removing an old name. The aliases go in a later release, by its own
  ticket.
- Search over the docs and the API in the MCP server and the CLI: 3412.
- Linking the book's item names to the API: 2845.
- Tested examples on the entry-point types, docs.rs metadata and feature
  labels: 2842.
- Compiler messages on the traits a game implements: 2510.
- Measuring how agents do: 2524, 2525 and 2562.
- Renaming anything for taste beyond the UK spellings, or changing the
  house prose style.
- Moving this ticket into the MVP: 3528 asks Nick.

**Estimated size:** about 2,400 lines (code 1,100, tests 700, docs and
data 600), counted from the steps and doubled. The renames are about 250
call sites, about 500 counted lines.

**Split:** past 1,100 lines, split off steps 8–11 (the preludes, the
import check and the one spelling) into a new ticket that keeps 3413's
either-order clause with 3520.

## Implementation steps

1. **The rustdoc index** (or 2845's, as its clause says): for each crate
   of `site/data/api.toml`, `cargo +<NIGHTLY> rustdoc -p <crate>
   <features> --lib -- -Z unstable-options --output-format json`.
   The features are a parameter of the index: 2845 builds it with the
   features `/api/` is built with (pages.yml's `cargo doc`, the default
   features), and this ticket with `api::listing`'s. Read each public item's
   path, kind, docs, attributes and deprecation into a map from path to
   item. A fixture JSON made by the pinned nightly tests the reader.
2. **`engine-terms.toml`** with every row of today's table, the tween row
   fixed, and the `[[guess]]` list. A test of the data's shape in
   `crates/xtask/src/site_data.rs`, beside `api.toml`'s.
3. **The shortcode** `engine_terms` and its expansion in
   `llms.rs`; the page calls it in place of its table. The twin's
   snapshot shows the same table.
4. **The aliases** on the items in `crates/` (core, assets, ui, platform,
   app), and the first sentences of those items.
5. **`cargo xtask api-text`** (`crates/xtask/src/api_text.rs`, its lines
   in `main.rs`'s docs, `USAGE` and `match`): the listing from
   `api::listing`, joined to the index by path. A listing line with no
   item in the index (an impl, a blanket trait) is written alone.
   `--check` runs the four checks of the Scope and writes nothing.
6. **pages.yml** installs the nightly and `cargo-public-api` at the
   `api` job's versions and runs `api-text --out site/public/api` after
   `cargo doc`. `llms.rs` links `api/<crate>.txt` beside the HTML, and
   its link check resolves it.
7. **CI's `api` job** runs `cargo xtask api-text --check`.
8. Steps 8–11 (the preludes, the import check, the ADR and the
   renames) moved to 3438 at the split.
9. **Docs** (the `document` skill): the book's *Tools for agents*
   chapter says where the plain-text API is.

## Acceptance criteria

- [x] `grep -rn 'doc(alias' crates` finds every alias of
      `engine-terms.toml`, and rustdoc's search for `draw_sprite`,
      `is_key_down`, `just_pressed`, `TileMap`, `Vec2` and `Timer` each
      shows the item the data names.
- [x] Appendix A's table and its Markdown twin are drawn from
      `engine-terms.toml`; its tween row names `Tween`.
- [x] `cargo xtask api-text --check` fails, each with its own message,
      on: a path in the data that names no item; an alias in the data
      missing from its item; a `doc(alias)` in `crates/` the data lacks;
      a "none yet" row whose ticket is in `tickets/done/`. It passes on
      this branch.
- [x] `site/public/api/shuyi_ui.txt` (and one per crate of `api.toml`)
      has a line for `GlyphBuffer::add_sprite` with its signature, its
      first doc sentence and its aliases; `llms.txt` links each file,
      and the link check resolves them.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the rustdoc reader on a fixture JSON (an item, its docs, an
  alias, a deprecation); each `--check` failure on a small fixture; the
  shape test of `engine-terms.toml`.
- Snapshot: the twin of appendix A, and one crate's `.txt` from a fixture
  crate (insta, as `llms` does).
- Integration: CI's `api` job runs `api-text --check`; pages.yml writes
  the files, and `cargo xtask llms` checks their links.

## Completion notes

What was done:

- `crates/xtask/src/rustdoc_index.rs`: the index of a crate's public
  items from rustdoc's JSON on `api::NIGHTLY` (`build` runs `cargo
  rustdoc` with the features it is given, `Index::read` reads the file).
  It walks the modules and the `pub use`s of the crate's own items, so an
  item is found at every public path (`shuyi_core::Pos` and
  `shuyi_core::grid::Pos`), with its kind, docs, aliases and deprecation;
  a trait impl's items are left out. Tested on a fixture crate whose JSON
  and listing the pinned nightly wrote (`rustdoc_index/fixture/`).
- `site/data/engine-terms.toml`: the 19 rows of appendix A and 25
  guesses. The tween row names `Tween` and chapter 2's *Things that
  move*; the prefab row names 2221, which plans scenes and instances.
- `site/templates/shortcodes/engine_terms.md` draws the page's table;
  `llms.rs` expands it in the twins (`api_text::table`), and the docs
  search's corpus reads it too (`corpus.rs`), so `shuyi docs` keeps the
  appendix's words.
- 74 `#[doc(alias)]`s on 26 items of `shuyi-core`, `shuyi-assets`,
  `shuyi-ui` and `shuyi-platform`, each item's doc now starting with the
  usual word ("A tile map: …", "Draws a sprite, part of an image, at a
  pixel rectangle: …").
- `cargo xtask api-text --out <dir>` writes `<crate>.txt` per crate of
  `api.toml` (listing line, first doc sentence, aliases, deprecation; the
  auto traits' `impl` lines left out, as noise); `--check` runs the three
  checks, each with its own message. pages.yml installs the `api` job's
  nightly and `cargo-public-api` and writes `site/public/api/*.txt`;
  `llms.txt` links each beside its HTML, and its link check resolves
  them. CI's `api` job runs `--check`. Run locally on this branch: both
  pass, and `shuyi_ui.txt` has `GlyphBuffer::add_sprite` with its
  signature, sentence and aliases.
- Rustdoc's search: the built search index (`target/doc/search.index/`)
  holds `draw_sprite`, `is_key_down`, `just_pressed`, `TileMap`, `Vec2`
  and `Timer`. The search page itself wasn't driven: the browser pane
  shows a local file as a static snapshot, without its script.
- Docs: the book's *Tools for agents* has *The API as text*;
  `site/README.md`, *The docs for agents*, says how the files and the
  table are made and checked; the docs map has the new pair (the terms
  and the aliases) and the twins' row names the text files.

Deviations:

- **Retitled** from *…, a plain-text API per crate, domain preludes and
  one spelling*: those two went to 3438.
- **Split at the valve, and one piece past it.** Steps 8–11 (the domain
  preludes, the import check, the ADR for one spelling, the renames) and
  `--check`'s UK spelling check went to 3438, which keeps the clause
  with 3520 (3520's side now names 3438). The *From 3412* clause, the
  plain-text API in `shuyi docs`'s index with a tracked copy of the
  files, went to 3439: it was over the size too, and lands apart.
- The shape test of `engine-terms.toml` is in `api_text`'s tests, not
  `site_data.rs`: there was no test of `api.toml` there to sit beside
  (`site/check-build.sh` checks that file).
- There is no `TickRules`: the fixed-steps row names `TickSim`, the trait
  of a game that steps in time.
- `--check` runs three checks, not four (the spelling one moved).
  `shape`, which includes the "none yet" ticket check, also runs in
  xtask's tests on every PR, as the `api` job runs only when the crates
  change.
- `changes.yml` runs the `api` job on a change to
  `site/data/engine-terms.toml` too, so a data-only edit is checked.

Follow-up tickets: 3438 (domain preludes, the import check, one
spelling), 3439 (`shuyi docs` searches the API text).

Game: nothing to adopt, the change adds doc aliases and doc text and
moves no item: `gh search code --repo Zafnok/visions-of-shuyi
'doc(alias'` and `'engine-terms'` find nothing in the game.

No break: no item is added, removed or changed; `#[doc(alias)]` and doc
text don't show in the listing or the semver check.

Book: chapter 10, *Tools for agents*, has *The API as text*; appendix A
is drawn from the data.

Skills: none, no schema, template, format or problem code changes.

Formats: none, only the rustdoc of `Formats` changed (its first words
and two doc aliases); no format or the registry's behaviour did.

Size: 2,331 counted lines, past the limit after the valve's split: the
index, the terms, the aliases and the check are one change (aliases
without the check drift, the check without the aliases fails, the
page's table without the data has no source), and the text files are
the same index written out, in the same command.
