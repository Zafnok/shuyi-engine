---
id: "3413"
title: "Names agents guess resolve: doc aliases from the other-engine table, a plain-text API per crate, domain preludes and one spelling"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: in-progress
blocked_by: []
nick_input: none
completed:
---

# 3413 — Names agents guess resolve: doc aliases from the other-engine table, a plain-text API per crate, domain preludes and one spelling

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
    the row is out of date: the tween case);
  - no public item's name holds a UK spelling of
    `api_text::UK_SPELLINGS` (`centre`, `centred`, `colour`, `behaviour`,
    `normalise`, `initialise`, `favour`, `licence`), unless it is
    `#[deprecated]`.
- **Domain preludes.** Beside `shuyi::prelude`, which stays as it is:
  - `shuyi_core::prelude`, for a rules crate: the RNG, `TurnRules`,
    `TickRules`, the step and time types, `Pos`, the ease and tween types;
  - `shuyi::prelude::grid` (the grid kit, the camera, `GridMap`),
    `shuyi::prelude::text` (styled text, the font), `shuyi::prelude::audio`
    and `shuyi::prelude::test` (the `Harness`, the playtest bots);
  - `shuyi_cli::prelude`, for a game's tool program and its tests: the
    contract types the templates import today.
  Each re-exports what the templates, `examples/hello-grid`,
  `site/snippets` and the shipped skills' worked examples import today,
  and no more.
- **An import check.** `cargo xtask check-imports`, a row in the gates
  table (`crates/xtask/src/gates.rs`, the fast set): code under
  `templates/`, `examples/` and `site/snippets/` names an engine item only
  through a prelude, or through a path in its list of allowed deeper
  paths, each with its reason.
- **One spelling.** US spelling for identifiers, matching `Color` and
  Rust's own crates; prose keeps the house spelling. A new ADR records
  it (the `write-adr` skill). Each of the eight UK names gets its US name
  (`print_centered`, `print_centered_styled`, `center_x`, `center`,
  `centered_on`, `NarrationAlign::Centered`, `Scroll::Centered`,
  `Rule::SharedColor`); the old
  name stays one release as a `#[deprecated(note = "renamed …")]` alias,
  as 2226 did. A variant's alias is an associated `const` (the three enums
  derive `PartialEq` and `Eq`, so it still matches in a pattern). Nothing
  in `crates/`, `templates/`, `examples/`, `site/` or `skills/` calls an
  old name but one `#[expect(deprecated)]` test per alias.
- **First doc sentences.** Each item that carries an alias starts its
  doc with the usual word ("A sprite: part of an image drawn at a pixel
  rectangle"), then the house wording.
- **Public types** (ADR-0039): no new type. The renamed variants keep
  `NarrationAlign`'s `#[non_exhaustive]`; `Scroll` and `Rule` stay as they are.
  `grey` (`lighting.rs:78`, a field and a RON key) is not renamed: it is
  a US spelling too, and the list leaves it out. The
  preludes are re-exports only.

**Either order with 2845:** if 2845 is done, check the paths in
`engine-terms.toml` with its index of rustdoc names and read the docs
and aliases from it; if not, build that index here
(`crates/xtask/src/rustdoc_index.rs`: rustdoc JSON on `api::NIGHTLY`,
read with `serde_json`) and 2845 reuses it.

**From 3412 (done):** feed the per-crate plain-text API listing into
3412's docs index (`crates/cli/src/contract/docs/`) as `HitKind::Api`
hits (one chunk per item: path, signature, first doc sentence), and the
alias table as extra terms on the items it names. ADR-0154 §7 says how a
source joins: a `shuyi_embed` table in `crates/cli/build.rs` (of a
tracked copy of the `.txt` files, which `cargo xtask api-text --check`
keeps current, since `site/public/` is ignored by git), an `Api`
variant of `HitKind` (`#[non_exhaustive]`, so additive), and a function
in `corpus.rs` that cuts it into chunks, added to `corpus::chunks`.

**Either order with 2108:** if 2108 is done, move its entity words from
the page's table into `engine-terms.toml`; if not, nothing to do (2108
writes them in the data file).

**Either order with 2142:** if 2142 is done, move its stats row from the
page's table into `engine-terms.toml`; if not, nothing to do (2142
writes it in the data file).

**Either order with 3520:** if 3520 is done, put `shuyi::prelude::grid`
behind the grid kit's feature, and the other preludes' items behind
theirs; if not, nothing to do (3520 gates the preludes with the rest).

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
8. **`shuyi_core::prelude`** and `shuyi::prelude::{grid, text, audio,
   test}`, `shuyi_cli::prelude`, each with module docs saying who imports
   it. ADR-0036 §6 is amended by a line pointing at the new ADR.
9. **`cargo xtask check-imports`** (`crates/xtask/src/imports.rs`) and
   its row in the gates table, then the templates, `hello-grid`,
   `site/snippets/` and the skills' worked examples moved onto the
   preludes until it passes.
10. **The ADR** for one spelling of identifiers.
11. **The renames**: the US names, the deprecated aliases with their
    tests, `shuyi_platform::prelude` exporting `print_centered` beside
    the deprecated `print_centred` (so a game's glob import still
    builds), every call site in this repository (`crates/`, `templates/`,
    `examples/`, `site/`, `skills/`), and `check_text.rs`'s list of text
    sinks, which keeps both names so `shuyi check` reads a game on
    either (`crates/cli/src/check_text.rs:842`).
12. **Docs** (the `document` skill): the book's *Tools for agents*
    chapter says where the plain-text API is and which prelude to
    import; *Games and screens* uses the new names; the shipped skills'
    text follows.

## Acceptance criteria

- [ ] `grep -rn 'doc(alias' crates` finds every alias of
      `engine-terms.toml`, and rustdoc's search for `draw_sprite`,
      `is_key_down`, `just_pressed`, `TileMap`, `Vec2` and `Timer` each
      shows the item the data names.
- [ ] Appendix A's table and its Markdown twin are drawn from
      `engine-terms.toml`; its tween row names `Tween`.
- [ ] `cargo xtask api-text --check` fails, each with its own message,
      on: a path in the data that names no item; an alias in the data
      missing from its item; a `doc(alias)` in `crates/` the data lacks;
      a "none yet" row whose ticket is in `tickets/done/`; a public,
      non-deprecated item named with a UK spelling. It passes on this
      branch.
- [ ] `site/public/api/shuyi_ui.txt` (and one per crate of `api.toml`)
      has a line for `GlyphBuffer::add_sprite` with its signature, its
      first doc sentence and its aliases; `llms.txt` links each file,
      and the link check resolves them.
- [ ] `cargo xtask check-imports` passes on the templates, `hello-grid`,
      `site/snippets/` and the skills' examples, and fails, naming the
      file and line, on a fixture that imports
      `shuyi::ui::frame::glyph_buffer::Sprite` with no allowed reason.
- [ ] The template's code imports from preludes only: no `use` path
      deeper than `shuyi::prelude::<domain>` or a prelude of
      `shuyi_core` or `shuyi_cli`, but the allowed ones.
- [ ] `print_centered`, `print_centered_styled`, `center_x`, `center`
      (both), `centered_on`, `NarrationAlign::Centered` and
      `Scroll::Centered` and `Rule::SharedColor` exist; each old name is `#[deprecated]`, gives
      what its new name gives (one `#[expect(deprecated)]` test each),
      and still matches in a pattern for the three variants.
- [ ] No code in this repository calls an old name but those tests.
      `shuyi check` reads a text sink by either name.
- [ ] The ADR for one spelling is in `docs/adr/` and its README.
- [ ] Completion notes: a `Behaviour:` line per renamed item (the old
      names warn), or, if the `api` job counts a deprecation as a semver
      finding, a `Breaking:` line with a `migration` block whose `find`
      lists the old names.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the rustdoc reader on a fixture JSON (an item, its docs, an
  alias, a deprecation); each `--check` failure on a small fixture; the
  import check's allowed list and a failing fixture; the shape test of
  `engine-terms.toml`.
- Unit: one `#[expect(deprecated)]` test per alias, comparing it with its
  new name (so mutation testing of the diff kills each alias's mutants).
- Snapshot: the twin of appendix A, and one crate's `.txt` from a fixture
  crate (insta, as `llms` does).
- Integration: CI's `api` job runs `api-text --check`; pages.yml writes
  the files, and `cargo xtask llms` checks their links.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
