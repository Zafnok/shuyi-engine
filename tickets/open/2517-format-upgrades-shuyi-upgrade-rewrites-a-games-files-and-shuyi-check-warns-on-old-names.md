---
id: "2517"
title: "Format upgrades: each engine format lists its old names, shuyi check warns on them, and shuyi upgrade rewrites a game's files keeping their comments"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2005", "2511"]
nick_input: none
completed:
---

# 2517 — Format upgrades: shuyi upgrade, and old names in shuyi check

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6,
Unreal 5.8 and GameMaker. When an engine's file formats change, the
others carry a project's files over:

- Godot converts a 3.x project's files with `--convert-3to4`, after a
  dry run with `--validate-conversion-3to4`
  (`docs.godotengine.org/en/stable/tutorials/migrating/upgrading_to_godot_4.html`),
  and since 4.4 its editor resaves old resources in the current form
  (*Upgrade Project Files*);
- Unreal reads a renamed class or property under its old name through
  Core Redirects until the asset is saved again
  (`dev.epicgames.com/documentation/en-us/unreal-engine/core-redirects-in-unreal-engine`);
- Unity reads a renamed field's data with `FormerlySerializedAs`
  (`docs.unity3d.com/ScriptReference/Serialization.FormerlySerializedAsAttribute.html`);
- GameMaker converts a project's files when a newer IDE opens it;
- Bevy publishes a migration guide per release, in prose, with no tool
  (`bevy.org/learn/migration-guides/`).

What Shuyi has (read on 2026-10-21):

- A game's files are in the engine's text formats: its keymap, palette
  and English, language packs, `.dlg` dialogue, `.map` maps, the font
  atlas's `atlas.ron`, any target profiles of its own
  (`examples/hello-grid/files/`,
  `templates/shuyi-default/stages/top-down-grid/crates/game/files/`),
  and `shuyi.ron` at its root.
- None of them has a version. The only versions are binary:
  `bitmap::VERSION` (`crates/assets/src/bitmap.rs`) and `pack::VERSION`
  (`crates/assets/src/pack.rs`), and a pack of another version is
  refused, never migrated (ADR-0023 §5). The engine's target profiles
  (`targets/*.ron`) have no version field either.
- `crates/assets/src` has 33 `#[serde(deny_unknown_fields)]` in 10
  files (audio, cinematic, credits, grid_map, keymap, lang, lighting,
  target, tip, voice), and `shuyi.ron`'s `Config`
  (`crates/cli/src/project.rs`) has it too. A renamed or removed field
  makes every older file fail with a parse error. No type in `crates/`
  uses `#[serde(alias)]` yet.
- `Format` (`crates/assets/src/format.rs`) has an id, extensions,
  `sniff`, `max_bytes` and `decode`. Nothing says what a format used to
  read. `DecodeCx` holds limits and a bundle, and no warnings.
- `shuyi check` (`crates/cli/src/contract/check.rs`) reports the
  content's errors only: `CheckOutput { errors, targets }`. Only
  `--target` has warnings, inside its target reports.
- `import` (`crates/cli/src/contract/import.rs`) reads a file as each
  kind of the game's `GameTools::importers()`. `Importers::standard()`
  has no keymap, map, language pack or target profile kind. hello-grid
  adds the map and the keymap (`examples/hello-grid/src/tools.rs`), the
  grid template adds the map only, and 2511's `BasicTools` adds none.
- 2218's writers (`Encode`) write through RON's `PrettyConfig`, which
  drops comments, and 2218 leaves `.dlg`, `.map`, keymaps, cinematics and
  language packs out for that reason. A writer can't be how a
  hand-written, commented file is brought up to date.
- Plans: 2005's version-policy ADR covers the crates' API and the
  engine's text formats (the rule below), and leaves finding and
  rewriting old names to this ticket. 2804 lists `Breaking:` lines first in the changelog. 3017 (parked)
  moves `display.color` to `capabilities.colour` and reads older files
  "with a migration", on its own. 3006 (done) found nothing to migrate:
  palettes already named colours by role. 2110 reads old saves through
  typed steps, and leaves rewriting a game's files to this ticket. 2501
  gives `template.ron` its own `format_version`.

Why it matters:

- Visions of Shuyi follows each engine change by hand when it moves its
  pin. That stays cheap until the first release (2005). After it, the
  game's files (keymaps, palettes, maps, dialogue) are many, and other
  games hold the same formats.
- Every game made with `shuyi new` holds these files. After a rename,
  `shuyi check` fails with a parse error, and nothing says what changed
  or fixes it.
- The end goal (2026-10-19, `docs/decisions.md`) is a game an agent
  makes in under an hour. An agent moving a game to a new engine version
  should run one command that rewrites the files and says what is left
  to do by hand.

The rule for the engine's formats, as 2005's version-policy ADR states
it (this ticket's ADR points to it):

1. a field added with `#[serde(default)]` is not breaking;
2. a renamed or removed name is still read for one more minor release,
   and its format lists the change;
3. a change of meaning always comes with a new name, so the old form
   can be found;
4. anything else is breaking: a `Breaking:` line names the format and
   the field.

This ticket builds the tools that rule leans on.

## Nick input

None.

## Scope

**In:**
- **Changes as data,** in `shuyi_assets::format::changes`
  (`crates/assets/src/format/changes.rs`, `no_std` with `alloc` like
  the rest of the crate):
  - `Change { since: &'static str, at: &'static [&'static str], kind:
    ChangeKind }`: the engine version the change came in, and where the
    name is, as the field names from the file's root (`&[]` for the top,
    `&["display"]` for a field inside `display: (…)`);
  - `ChangeKind::Rename { old, new }`: a field name, or a value name (an
    enum variant), now called `new`;
  - `ChangeKind::Manual { name, say }`: a name whose change no tool
    makes (a field removed, moved into another struct, or split in
    two); `say` tells what to do;
  - `const fn` constructors (`Change::rename(since, at, old, new)`,
    `Change::manual(since, at, name, say)`), so a format lists its
    changes in a `static`; both types are `#[non_exhaustive]`;
  - `find(text: &str, changes: &[Change]) -> Vec<Found>`: each place an
    old name is used, with its byte span, its 1-based line and column
    (as `ContentError` shows them) and its change;
  - `apply(text: &str, changes: &[Change]) -> (String, Vec<Found>)`:
    every rename made, in list order (a later change can rename what an
    earlier one made), each one replacing only the name's bytes, so
    comments, layout and line endings stay byte for byte. The manual
    changes are found and left.
- **One RON span parser, moved, not written again.** The `ron` crate
  gives positions only in its errors (`crates/assets/src/ron.rs`), but
  `shuyi-cli` already reads RON for spans (re-read 2026-10-28, found by
  an analysis of the engine for areas that do the same job twice):
  `template/ron_merge.rs` (2501), private to `shuyi new`'s composer.
  Its `Parser` reads text into a `Node` tree of where each value,
  container, struct name, field name and item starts and ends. It skips
  whitespace, nested comments, strings, raw strings and chars, and never
  rewrites the text. `shuyi-assets` can't depend on `shuyi-cli`, so
  move that parser and `Node` down into a public module of
  `shuyi-assets` beside `format::changes` (`no_std` with `alloc`), and
  `ron_merge` calls it from there with its tests passing unchanged.
  `find` walks the tree for the paths below. Add what this ticket needs
  and it lacks (byte strings, numbers, the `#![enable(…)]` line,
  stopping at `---`). If the move can't keep `ron_merge`'s tests
  unchanged, write the scanner private to `format::changes` as before,
  and the completion notes say why. Either way:
  - it skips whitespace, `//` and nested `/* */` comments, strings
    (`"…"` with escapes, raw `r"…"` and `r#"…"#`, byte strings), chars,
    numbers, and the `#![enable(…)]` line;
  - it stops at the first line that is `---` alone, so a `.map`'s RON
    head is scanned and its grid isn't;
  - the path: a name followed by `:` inside `(…)` is a field, and adds
    its name to the path of the value after it, up to the `,` or the
    closing bracket that ends that value. Brackets add nothing: a list,
    a map, a tuple, `Some(…)`, and the `(…)` after a struct's or a
    variant's name. So in `targets: {"web": (frame: …)}` the field
    `frame` is at `["targets"]`, and in `colour: Indexed(banks: 16)` the
    field `banks` is at `["colour"]`;
  - any other name (a variant, a struct's name) is a value, at the path
    of the field whose value it is in;
  - with unbalanced brackets it finds nothing (`shuyi check` already
    reports the parse error).
- **Each format lists its own changes:**
  - `Format::changes(&self) -> &'static [Change]`, a provided method
    that returns `&[]`, so no format changes today;
  - a change stays in the list after its old name stops being read, so
    a game several versions behind still upgrades;
  - `Formats::changes::<K>(&self, file: &AssetPath, bytes: &[u8]) ->
    Vec<(&'static str, &'static [Change])>`: the formats of kind `K`
    that sniff `bytes` or claim `file`'s extension, in the order
    `Formats::decode` tries them, each with its id.
- **`shuyi.ron`'s own changes:** `project::CHANGES: &[Change]` in
  `crates/cli/src/project.rs`, empty today, applied to the project file
  the same way.
- **`shuyi upgrade [--check] [--json]`**, a contract op
  (`crates/cli/src/contract/upgrade.rs`, `Op::Upgrade`):
  - it runs the game (`contract::cli::runs_the_game` lists it): its
    kinds come from `GameTools::importers()`, as `import`'s do. When
    `shuyi.ron` names no game program, it uses `Importers::standard()`
    and says so in its limits, as `import` does there
    (`crates/cli/src/lib.rs`, `run`). The limits also name the kinds
    tried, so a file of a kind the game doesn't list is known to be
    skipped;
  - it walks the project from its root, skipping `target/` and every
    folder whose name starts with `.`. It reads a file only when a
    format of one of the kinds claims it and lists changes;
  - **the kind of a file:** many kinds claim `.ron`, so a kind's
    changes apply to a file only when the file reads as that kind
    (`Formats::decode::<K>`) before the changes (the old name is still
    read) or after them (it no longer is). When two formats of the kind
    list changes for the file, the first in `Formats::changes`' order
    under which it reads is used. A file with old names that reads as no
    kind, or as two, is left alone and listed in the limits. The files a
    file names come from its folder, as `import` reads them
    (`folder_bundle`, made `pub(super)`), each folder's bundle built
    once per run;
  - the rule is a new method of `import.rs`'s `ImportKind`, which is
    private today: make the trait and `Importers`' `kinds` field
    `pub(super)` (or add a `pub(super)` accessor), so `upgrade.rs` and
    `check.rs` share one walk;
  - `shuyi.ron` gets `project::CHANGES`, and is written only if
    `Config::parse` reads the result;
  - it writes each file whose text changed, unless `--check`;
  - `UpgradeInput { check: bool }`; `UpgradeOutput { files:
    Vec<UpgradedFile> }`; `UpgradedFile { file, kind, format, changes:
    Vec<FoundChange> }`; `FoundChange { line, column, since, message,
    by_hand: bool }`. Paths are from the project's root;
  - `ok`: with `--check`, nothing was found (so CI can fail on old
    names); without it, nothing is left to do by hand and every write
    worked;
  - the text answer has a line per change, such as
    ``files/swatch.ron:3:5: renamed `color` to `colour` (since 0.2.0)``,
    with `by hand:` before a manual one;
  - `contract::VERSION` goes up by one, the schemas are written again
    (`crates/cli/schemas/upgrade.input.json`, `upgrade.output.json` and
    `check.output.json`), and `crates/cli/schemas/versions.json` gets the
    new fingerprint (ADR-0022 §5).
- **Warnings in `shuyi check`:**
  - `check` walks the project the same way, with the same kind rule and
    `find` only;
  - each old name is a warning in a new `CheckOutput::warnings:
    Vec<Problem>` (`#[serde(default)]`): "`color` is an old name in the
    \"swatch\" format (since 0.2.0); it is `colour` now: `shuyi
    upgrade` renames it", or a manual change's `say`;
  - warnings don't fail `check`; `check::check` takes the project's
    root for the walk (its one caller, `contract/cli.rs`'s `with_game`,
    passes `project.root`).
- **The tools read a new game's keymap,** so `import`, `check` and
  `upgrade` see it:
  - `templates/grid/crates/game/src/tools.rs` registers
    `KeymapRon::<Action>` in `importers()` and adds the
    `KeymapDef<Action>` kind, as `examples/hello-grid/src/tools.rs`
    does;
  - the blank template's tools are `shuyi_cli::BasicTools` (2511), whose
    `importers()` is the default today. It becomes
    `Importers::standard()` with `KeymapRon::<A>` registered and the
    `KeymapDef<A>` kind added (the format `BasicData::formats()` already
    registers). **Either order with 2558:** if 2558 is done,
    `BasicTools::importers()` hands the game's own `add` function the
    standard importers: register the keymap kind in them before `add`
    runs, so a game's own kinds sit on top of it. If not, nothing to do
    (2558 builds its `add` on top of the keymap kind, by its own
    clause);
  - each template's `AGENTS.md` gets a row for `cargo shuyi upgrade` in
    its table of tools;
  - templates are outside `cargo fmt --all`: format them as CI's
    template job checks them.
- **The test kinds, end to end,** in
  `crates/cli/src/contract/upgrade/tests.rs`:
  - a kind `Swatch` with `#[serde(deny_unknown_fields)]`: a field
    `#[serde(alias = "color")] colour: String`, a field `shade: Shade`
    whose variant `Grey` is also read as `#[serde(alias = "Gray")]`, and
    `size: Option<serde::de::IgnoredAny>` still read but unused. Its
    format `SwatchRon` claims `.ron` and its `changes()` lists
    `Change::rename("0.2.0", &[], "color", "colour")`,
    `Change::rename("0.2.0", &["shade"], "Gray", "Grey")` and
    `Change::manual("0.2.0", &[], "size", "a swatch has no size now:
    delete the field")`;
  - a second kind `Label { color: String }` with its own `.ron` format
    and no changes, so a file of another kind holds the old name at the
    same path;
  - a scratch project under `std::env::temp_dir()` (as
    `contract/import/tests.rs` makes one), and a fake game whose
    importers know both kinds (`contract/fake.rs`);
  - `check` warns; `upgrade --check` changes nothing and fails;
    `upgrade` rewrites the swatch file keeping its comments, and leaves
    the label file as it was; `check` then has no warning.
- **An ADR** (the next free number in `docs/adr/`):
  - the four points of the rule in *Context*, for the engine's formats
    (if 2005's ADR already states them, point to it instead);
  - a format lists its changes as data (`Format::changes`);
  - the tools find and rewrite old names in the text at token spans,
    never by writing the asset again;
  - a file's kind is the kind it reads as;
  - `shuyi upgrade`, and the warnings in `shuyi check`.
  It extends ADR-0019 and ADR-0022.
- **Docs:**
  - `docs/formats.md`, a section "Changing a format":
    - add a field with `#[serde(default)]`;
    - rename with `#[serde(alias = "old")]` and a `Change::rename`;
    - remove or move a field by keeping it read and ignored for one
      release (for example as `Option<serde::de::IgnoredAny>` under its
      old name, where the type has `deny_unknown_fields`), with a
      `Change::manual`;
    - never change what a field means under the same name;
  - `docs/contract.md`: the op's row, the warnings in `check`'s row, and
    `upgrade` and `check` in the `importers` row of *What a game gives*;
  - the usage in `crates/cli/src/lib.rs`, the op table in
    `crates/cli/src/contract.rs`'s module docs, the module docs of
    `contract/cli.rs` (`upgrade`, like `import`, runs without a game
    program), and `GameTools::importers`' doc comment
    (`crates/cli/src/game.rs`).
- **The skills:**
  - the `work-ticket` skill's *Public API* bullet gets one line: a
    change to an engine format's names is listed in that format's
    `changes()`, tested with a fixture of the old form, and run with
    `cargo shuyi upgrade` in `examples/hello-grid`;
  - the `write-ticket` skill's *A new file format* bullet says the same
    for a ticket that changes a format.
- **Either order with 2516:** 2516 adds `CheckOutput::files`, and this
  ticket adds `CheckOutput::warnings`. Whichever lands second keeps the
  other's field, prints its own after it in `check::text`, and raises
  `contract::VERSION` again.
- **Problem codes (2509):** the warning and the manual change get new
  rows in 2509's table (content codes `SH2nnn`, named `old-name` and
  `upgrade-by-hand`), each with its page and a help naming `shuyi
  upgrade`; the warnings carry them in `Problem`'s `code` and `help`.
- **Either order with 2523:** if 2523 is done, add a step to
  `skills/shuyi-upgrade/SKILL.md`, after the game builds on the new
  engine: run `cargo shuyi upgrade`, then `cargo shuyi check`. If 2523
  isn't done, it writes that step itself (its step 7).
- **The templates (2501, done):** the templates' changes go where the
  parts of `templates/shuyi-default` hold those files.
- **Public types (ADR-0039):** `UpgradeInput` (an input, its kind 4)
  and the new public types follow it.
- **MCP:** `shuyi mcp` (1133, done) makes its tools from `Op::ALL`; a new operation needs its arm in `mcp::host::one_off` and in `mcp::description`, which the compiler asks for, and is in `contract::cli::runs_the_game` if it runs the game (so it takes `project` and runs in the game's tool program).
- **Either order with 3017:** if 3017 is done, list its move of
  `display.color` as a `Change::manual` of `TargetRon`, and add the
  target profile kind to `Importers::standard()` so `upgrade` finds a
  game's own profiles; if not, 3017 lists its change when it lands.
  3006 (done) changed no palette form (palettes already named colours
  by role), so `PaletteRon` has no changes to list.
- **Either order with 2236** (frozen copies of the formats' fixtures
  from each release): if 2236 is done, add a line to its two tests'
  failure messages (`crates/assets/src/format/frozen_tests.rs` and the
  project file's test in `crates/cli/src/project.rs`) and to its
  *Frozen copies* section in `docs/formats.md`: list a rename as a
  `Change` (in the format's `changes()`, or in `project::CHANGES` for
  `shuyi.ron`), so `shuyi check` warns on it and `shuyi upgrade`
  rewrites it, and link *Changing a format*; if not, nothing to do
  (2236 adds both when it lands).
- **Size:** if the PR passes about 600 changed lines (the schemas,
  fixtures and snapshots not counted), the warnings in `shuyi check`
  (`CheckOutput::warnings`, the root on `check::check`, the 2516
  clause) go to a follow-up ticket (`write-ticket`), After MVP, blocked
  by this one, and the PR says so. `upgrade --check` already finds every
  old name.
- **For the game:** nothing changes in its files. The library API does
  change: a new `Op` variant, a new field of `CheckOutput`, and
  `check::check` takes the root. Say so in a `Breaking:` line (2010's
  form, `work-ticket` §5), with a `migration` block under it (the
  `work-ticket` skill asks for one, 2804); the game's PR that moves
  the pin follows.

**Out (do not do):**
- Deleting a removed field by itself: in this first cut a removal is a
  manual change, whose answer names the file, the line and the field. A
  `Remove` kind comes with the first removal that needs it.
- Text that isn't RON: `.dlg` dialogue and the grid below a `.map`'s
  `---`. When one of them changes, its ticket adds a finder for it.
- A version stamp in `shuyi.ron` or in each file: a change is found by
  its old name, so files without a stamp upgrade the same way, and
  the rule's point 3 (a new meaning gets a new name) keeps it so.
- Moving a game's Rust code to a new engine version: 2523's skill, which
  runs this op as one of its steps.
- Packs (ADR-0023 §5: bake again), saves and records (2110, 2111), and
  `template.ron` (2501 and 2505 version it).
- The target profile kind in `Importers::standard()` while 3017 isn't
  done, and other kinds there: a game lists the kinds of its own types
  (its map, its language pack) in its own `importers()`.
- Frozen copies of every format's fixtures from each release, tested to
  still read: a separate gate (2236), not this ticket.
- Writing assets with 2218's `Encode`: the upgrade never re-serializes.

## Implementation steps

1. Read ADR-0019, ADR-0022, 2005's version-policy ADR,
   `crates/assets/src/format.rs`, `crates/cli/src/contract/import.rs`,
   `crates/cli/src/contract/check.rs`, `crates/cli/src/contract/cli.rs`
   and 2511's `BasicTools`, and `crates/cli/src/template/ron_merge.rs`.
2. Move `ron_merge`'s parser into `shuyi-assets` (or write the scanner,
   as above), then `find`, then `apply`, with their unit and property
   tests.
3. Add `Format::changes`, `Formats::changes` and `project::CHANGES`.
4. Write the op, its walk and its answer. Add it to `Op` and `Op::ALL`,
   `schemas()`, `runs_the_game` and `with_game` in `contract/cli.rs`,
   the no-game branch of `run` in `lib.rs`, the usage and the schemas.
5. Add the warnings to `check`.
6. Write the test kinds and the end-to-end test. Add the keymap to the
   grid template's `importers()` and to `BasicTools::importers()`.
7. Write the ADR, the docs and the skills' lines, and settle the
   *Either order* clauses. Run the gates, the templates job's commands
   included.

## Acceptance criteria

- [ ] `find` finds an old name at its path and nowhere else: not in a
      comment, a string, a raw string or a char, not below a `---`
      line, and not at another path (a unit test for each).
- [ ] After `apply`, the text differs from the input only inside the
      spans of the renamed names (property test).
- [ ] `apply` twice gives what `apply` once gives (property test).
- [ ] The end-to-end test passes: `check` warns with file, line and
      column; `upgrade --check` fails and writes nothing; `upgrade`
      rewrites the swatch file, keeps its comments and lists the manual
      change; `check` then warns no more, and the file reads as
      `Swatch`.
- [ ] The label file, which holds `color` at the same path, is not
      changed.
- [ ] `cargo shuyi upgrade --check` passes in `examples/hello-grid` (no
      engine format lists a change yet).
- [ ] In a new game from each template, `cargo shuyi import
      crates/game/files/keymap.ron` reads a keymap (said in the PR).
- [ ] The schemas and `versions.json` match the new `contract::VERSION`.
- [ ] The ADR is written and listed in `docs/adr/README.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the scanner on nested comments, strings with escapes, raw
  strings with `#`, chars, the `#![enable]` line, a `---` line, maps,
  lists, `Some(…)` and a variant's fields; `find` at a path; renames
  made in list order; a manual change found and not made; unbalanced
  brackets find nothing; `Formats::changes` by sniff and by extension;
  `shuyi.ron` not written when the result doesn't parse;
  `BasicTools::importers()` lists the keymap kind.
- Property: for random text built of fields, comments, strings and
  whitespace around old names, the span rule and idempotence above;
  `find` never panics on random text.
- Integration: the end-to-end test over a scratch project;
  `upgrade --check` on Hello, Grid.

## Completion notes

*(Filled in by the session that completes the ticket.)*
