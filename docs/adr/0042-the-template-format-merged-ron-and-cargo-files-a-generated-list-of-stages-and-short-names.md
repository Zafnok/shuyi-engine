# ADR-0042: The template format: merged RON and Cargo files, a generated list of stages, and short names

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related tickets:** 2501 (templates as data and the composer), 2502
  (the wizard), 2503 (several perspectives), 2505 (templates from
  anywhere), 2511 (the short path), 1129 (`shuyi new`), 2003 (`shuyi add`)
- **Extends:** ADR-0013 §3 (a template is a folder of data, made of
  parts), ADR-0036 (the short path)

## Context

ADR-0013 §3 makes a template a folder with a `template.ron` and the
files of its parts. It says shared files are merged by the engine:
`Cargo.toml` features, the `shuyi.ron` manifest and the flow's list of
stages. Any other file two parts write is an error naming both.

Ticket 2501 moves 1129's two templates onto that format: `blank`
becomes the part `base`, and `grid` becomes `base` plus a grid stage
part. Doing it showed what the grid stage has to change in files the
base already writes:

- **its crates' manifests:** the root's `[workspace.dependencies]` gains
  `shuyi-core`, `shuyi-assets`, `serde` and `ron`; the game crate's
  `[dependencies]` gains `rules`, `serde` and `ron`. ADR-0013 names
  only the features;
- **its text and colours:** the stage's lines in `files/en.ron` and its
  map's colours in `files/palette.ron`. `BasicData` (ADR-0036) reads
  one English file and one palette, so they can't go in files of the
  stage's own;
- **its project file:** `shuyi.ron` gains `rules`, `check_text` and a
  folder in `check_keys.rust`. Its comments are what agents read about
  each field;
- **the game's code:** the base's `lib.rs` names its data type
  (`BasicData<UiAction, T>`), its first screen and its tool program. The
  stage needs its own part of the data, a screen the game reaches, and
  its own `GameTools`;
- **the base's tests**, which every game made from the template runs,
  must hold with a stage or without one.

Two grid and blank names must stay working (`--template grid`), and
ADR-0013 §7 says templates from anywhere come later (2505), so how a
short name maps to answers is part of the format.

## Decision

### 1. Every `Cargo.toml` and every `.ron` file merges

A path that two parts write is merged when it is a `Cargo.toml` (at any
depth) or a `.ron` file:

- **`Cargo.toml`**, with `toml_edit`: the next part's keys are added to
  the first's, each with its comments (its key's decor); a table both
  have is merged the same way; an array gains the items it lacks; a key
  both have with values that differ fails with
  `ComposeError::Merge { path, parts, at, message }`, `at` its dotted
  path. Then the parts' `modules` become the `shuyi` features of the
  root `Cargo.toml` through `shuyi add`'s `catalog::turn_on`.
- **`.ron`**, with a reader of our own (`template/ron_merge.rs`) that
  only finds where each value starts and ends: the first part's text is
  kept as it is, comments and layout with it, and the next part's
  struct fields and map entries it lacks are inserted before the
  closing bracket, each with the comment lines above it, indented as the
  first's items are. A struct (same name, or unnamed) or a map both have
  is merged the same way; a list gains the items it lacks; anything else
  both have must be the same once spaces and comments are left out.

Parts merge in the template's order. Any other path two parts write is
`ComposeError::SameFile { path, parts }`. A part's piece of a merged
file holds only what it adds, and is valid TOML or RON as it is.

### 2. The flow's list of stages is generated, and a stage is a module

When a template has a part with a `stage`, `shuyi new` writes
`crates/game/src/stages.rs` (no part may) from the picked parts'
stages, in order. A part with `stage: Some("top-down-grid")` writes the
module `crates/game/src/stages/top_down_grid.rs`, which gives:

- `Own` and `own(&Bundle) -> Result<Own, Vec<ContentError>>`: its part
  of the game's data, `BasicData`'s own part (ADR-0036);
- `first(&mut Ctx<Data>)` and `screens(&mut Ctx<Data>)`: the screen the
  title starts, and every screen it can show;
- `Tools`, with `Tools::new(BasicGame<UiAction, Own>)`: the game's
  `GameTools`, which add the stage's and call `BasicTools` for the rest.

`stages.rs` declares the modules and re-names them for the base:
`names()`, `Own`, `own`, `first` (an `Option`), `screens` and `tools()`.
With no stage they are `&[]`, `()`, `Ok(())`, `None`, no screen and
`BasicTools`. The base's `lib.rs` is written once against these names:
`BasicGame::with_own(name, storage, files, stages::own, title)`.

The title is always the first screen. Its Confirm starts `first`, when
there is one, and its help line says so; a stage's Cancel goes back to
it. The base's tests then hold for every game the template makes.

A game has one stage until 2503: a second part with a stage fails with
`ComposeError::Stage`. 2503 makes `Own` a struct of every stage's part
and the title a list of them.

### 3. Short names are the template's data

`template.ron` has `short_names`: a map from a name to a set of answers
(`"grid": { "perspective": ["top-down-grid"] }`). `--template <name>`
takes one, then `--answers` and `--set` answer in its place question by
question. A template from anywhere (2505) names its own sets the same
way.

### 4. The rest of the format

- `engine` is a semver requirement (the `semver` crate, MIT OR
  Apache-2.0, no dependencies), matched against `shuyi-cli`'s version.
- The engine's own questions come with 2502. Until then the built-in
  template asks `perspective` as a question of its own.
- `compose` takes the placeholders' values (`Fill`: the name, the crate
  name and the engine's dependencies) beside the template, the answers
  and the catalog, so it stays pure.
- With no checkout of the engine, `new` reads the catalog the binary
  embeds (`Catalog::embedded`: `docs/modules.ron` and the `shuyi`
  crate's features).

## Consequences

- A part adds to the files every game has without copying them, and a
  file it adds to keeps the base's comments, so an agent reading a new
  game's `shuyi.ron` still reads why each field is there.
- The RON merge is a reader of our own (about 600 lines with its tests),
  kept small because it never rewrites a value. A fuzz-style test feeds
  it any text.
- A `.ron` file that two parts mean to own outright can't be: the rule
  is by extension. Two values that differ still fail, naming both parts,
  so nothing is overwritten silently.
- Every game has a title before its stage, so scripts start with
  `Enter` (`cargo shuyi scene --script "Enter Right"`).
- `stages.rs` is generated Rust. Its text is a golden file's (the
  plans' test), and CI builds and formats the games made from it.

## Alternatives considered

- **Only the files ADR-0013 names merge** (`Cargo.toml` features,
  `shuyi.ron`) — the grid stage's text, colours and dependencies would
  then have to be in the base already, so `blank` would carry the grid
  game's lines, or be a second copy of the base.
- **A part's file replaces the base's** — simple, but a game with two
  parts that both change `en.ron` would lose one's lines, and no test
  would say so. ADR-0013 rejected overrides for that reason.
- **RON merged as values** (`ron::Value`, written back) — it loses every
  comment, which is what makes `shuyi.ron` readable.
- **Placeholders for the stage in the base's `lib.rs`**
  (`{{stage_type}}`) — a template language by another name, which
  ADR-0013 §3 rules out: the base's Rust would no longer build as it is.
- **The grid game starting on its stage, as 1129's did** — the base's
  tests would then differ by game, and the flow would have nowhere to
  list several stages (2503).
- **Short names in the engine's code** — a template from anywhere could
  then have none.
