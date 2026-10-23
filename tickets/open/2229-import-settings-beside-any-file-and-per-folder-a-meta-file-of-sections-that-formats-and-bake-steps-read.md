---
id: "2229"
title: "Import settings beside any file and per folder: a .meta file of typed sections that formats and bake steps read, named so no RON format claims it"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2229 — Import settings beside any file and per folder

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Each has settings for how a file is imported, and
Bevy, Godot and Unity keep them in a file beside it:

- **Bevy:** a `.meta` file beside an asset holds its loader's and
  processor's settings (since 0.12,
  <https://bevyengine.org/news/bevy-0-12/>).
- **Godot:** a `.import` file beside each source, committed to the
  repository, and import defaults per importer in the project settings
  (<https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/import_process.html>).
- **Unity:** a `.meta` file per asset with its importer's settings, and
  Presets that set defaults by folder or name.
- **Unreal:** Interchange pipelines that set how a file is imported,
  kept with the imported asset rather than beside the source.

What Shuyi has today:

- `shuyi_assets::format` (`crates/assets/src/format.rs`): a format reads
  one file and the files it names through `DecodeCx { limits, bundle }`
  (line 226). ADR-0019 §5 puts what a game decides in the format's
  value when it is registered, so every choice is game-wide.
- The engine's "RON sidecars" (font atlas, palette, lights, names, the
  audio manifest; `docs/formats.md`, *The built-in formats*) are assets
  in their own right. None is a file of settings for how another file
  is read.
- `Bake` (`crates/assets/src/pack/bake.rs`) bakes each kind with no
  per-file options.
- No code reads per-file or per-folder settings. No `.meta` file is in
  this repository or in the game's (checked 2026-10-20).

What is planned, and leans on a file nothing defines:

- **3007** (After MVP): "Per-image settings in the image's sidecar:
  `onebit: (dither: atkinson, alpha_cut: 128, invert: false)`", and
  ADR-0008 §4: "An image's sidecar may choose `dither: ...`". Nothing
  says what that file is called, where it sits, what a folder may set,
  or how a typo is reported.
- **3019** (parked): `indexed: (group, bpp, dither, alpha_cut)` "in the
  sidecar".
- **3025** (parked): a DS texture format "chosen per material in its
  sidecar".
- **2328** (parked): "an atlas's sidecar may name a normal map".

Without one shape, each of them invents its own file, and `shuyi check`
can't see a typo in settings whose bake step isn't running. Not
covered here, on purpose: 2222's `atlas.ron` is an asset (`AtlasSpec`,
a folder packed into pages), not settings, and 3029's ADPCM has no
per-file settings.

**Why the name is `.meta`.** A name like `hero.png.import.ron` would be
claimed by every RON format: `Formats::claims::<K>(path)` takes the
extension after the last dot (`AssetPath::extension`), so
`Formats::load_under`, `Formats::load_each` and `Bake::under` would
decode a settings file as a palette or a game's data table
(`ron::Ron<T>`). `LangPackRon` calls any other file in a language
directory an error (`crates/assets/src/lang/formats.rs`, line 149), and
so does `lang::from_files` for any other file under `lang/`
(`crates/assets/src/lang.rs`, line 575). No format claims `meta`. Bevy
and Unity use the same name; the contents are RON, and ours.

Why it matters:

- **The game:** Visions of Shuyi needs nothing new from this now. Its
  sidecars are assets of their own. Its Playdate port (its ticket 1025,
  parked until after launch) goes through 3007, which reads this file.
- **Other 2D games:** an artist picks how a picture becomes 1-bit or
  indexed for a whole folder or for one file, with no code.
- **The end goal** (`docs/decisions.md`, 2026-10-19): an agent making a
  Playdate version learns one convention, and `shuyi check` catches a
  typo in a settings file even when no Playdate bake runs.

Nick put the features most 2D games expect after the MVP, not parked
(`docs/decisions.md`, 2026-10-20). This one goes before 3007, which
waits for it.

## Nick input

None.

## Scope

**In:**

- **The ADR** (the `write-adr` skill), at the next free number, e.g.
  `docs/adr/NNNN-import-settings-a-meta-file-beside-a-file-and-folder-defaults.md`.
  It extends ADR-0007 and ADR-0019 §5, and decides:
  1. **The files.** `<file>.meta` beside its file, with the file's full
     name kept (`images/hero.png.meta`), so `hero.png` and `hero.ogg`
     never share one. `_folder.meta` in a folder holds the defaults for
     every file in it and below it. Not a dotfile. The contents are RON:
     a struct of sections, `(onebit: (dither: bayer), indexed: (group:
     "ui"))`.
  2. **`meta` is reserved.** No format may claim it, and every walker
     over a folder skips settings files.
  3. **Sections are typed and registered.** A section is a struct that
     implements `ImportSettings`, named by its `SECTION`. The engine
     registers its own in `Formats::register_standard`, and a game its
     own, at start-up, whether or not the step that reads it runs. A
     section is named after the step that reads it (`onebit`,
     `indexed`), never after a file format (`AGENTS.md` rule 10).
  4. **Which wins.** For a file, the files looked at are its own
     `.meta`, then `_folder.meta` in its folder, then in each folder
     above it up to the root. A section is taken whole from the first
     of them that has it: fields are not merged across files. With
     none, the section's `Default`.
  5. **Layers.** A settings file is replaced by a higher layer like any
     file: `assets@playdate/images/hero.png.meta` (3007's layers)
     replaces `assets/images/hero.png.meta`. Nearness wins over layers:
     a target layer's `_folder.meta` doesn't override a file's own
     `.meta` in the base layer; a `<file>.meta` in the target layer
     does.
  6. **Read through the bundle.** Settings are read with `DecodeCx`'s
     bundle, so `bundle::watch` hears of every settings file looked
     for, and 2220's `Hot` tracks it with the asset's other reads.
  7. **Errors** name the settings file and the line in it: bad RON, an
     unknown section, an unknown field, a bad value, and a `<name>.meta`
     whose file the bundle lacks (a file a layer leaves out on purpose,
     `Bundle::left_out`, is no error). `Formats::check_import_settings`
     checks every settings file; `shuyi check` and `Bake::bake` run it.
  8. **Packs.** A settings file goes into a pack as its own bytes, like
     any other file: `Bake::bake` keeps every file, `Bake::check` wants
     every file, and a file kept raw in the pack and decoded at run
     time reads the same settings it read from the sources. Leaving
     settings out when every file they apply to is baked is a
     follow-up, if a target's budget (3008) needs the bytes.
  9. **Settings are not data.** They say how a file is read or baked.
     The engine's RON sidecars stay assets, and a game's data stays its
     own kinds. The engine's sections hold integers, bools, enums and
     strings, never floats.
  10. **ADR-0019 §5 still holds** for game-wide choices: they stay in
      the format's value at registration. Settings are for choices that
      differ by file or folder.
  - The `Status` lines of ADR-0019 and ADR-0008 name the new ADR
    (ADR-0008 §4's "an image's sidecar" is this file), and
    `docs/adr/README.md` gets its row.
- **`shuyi_assets::import_settings`** (`crates/assets/src/import_settings.rs`,
  `pub mod` in `lib.rs`), building without `std` (CI's `no-std` job):
  - `pub trait ImportSettings: DeserializeOwned + Default + 'static {
    const SECTION: &'static str; }`. Its doc says a section is a struct
    with `#[serde(default, deny_unknown_fields)]`;
  - `SETTINGS_EXTENSION` (`"meta"`) and `FOLDER_SETTINGS`
    (`"_folder.meta"`);
  - `is_settings_file(path) -> bool`, `settings_of(file) -> String`
    (`images/hero.png` to `images/hero.png.meta`), and
    `settings_chain(file) -> Vec<String>` (its own, then each folder's
    `_folder.meta` up to the root);
  - a private splitter that reads a settings file into its sections,
    each with its text and the line it starts on. It reads the file as
    a struct whose values are `&ron::value::RawValue`, through
    `deserialize_any` and a visitor's `visit_map` (`visit_unit` for an
    empty `()`). `RawValue` borrows each section's text from the
    source, so the section's start is its pointer offset in the source
    (safe code: `as_ptr() as usize`, checked subtraction), counted into
    a line. Its text starts right after the colon, with the whitespace
    before the value. An error inside a section, which RON counts from
    the section's own line 1, is moved to the file's line (and, on that
    first line, the file's column). Checked on 2026-10-20 and again on
    2026-10-21 with ron 0.12.2 (the version in `Cargo.lock`): a file
    `(onebit: (...), indexed: (...))` with a comment before it splits
    this way, `()` and ` () ` give no section, and an unknown field's
    error points inside the section's own text;
  - errors are `ContentError`s with the settings file's shown name, a
    line and a column, as `shuyi_assets::ron::parse_ron` gives them.
- **`DecodeCx::import_settings::<S: ImportSettings>(&self, file:
  &AssetPath) -> Result<S, Vec<ContentError>>`**, in `format.rs` beside
  `DecodeCx::bytes`: it walks `settings_chain(file.path())`, reads each
  file with `self.bytes` (so the watcher is told, found or not), and
  stops at the first that has `S::SECTION`. Without a bundle, or with
  no file that has the section, it gives `S::default()`. A settings file
  with bad RON on the way is an error, even if a farther one has the
  section.
- **The registry** (`Formats`, `format.rs` line 438):
  - `register_import_settings::<S: ImportSettings>(&mut self) ->
    Result<(), RegisterError>`. It refuses a second section with the
    same name, a type that can't read `()` (a field has no default),
    and a type that reads `(shuyi_probe: ())` (it doesn't deny unknown
    fields), tried in that order. Each refusal is a new `RegisterError`
    variant whose message names the section and, for the last two, the
    attribute to add (`#[serde(default)]`, `#[serde(deny_unknown_fields)]`).
    It keeps, by section name, a plain function that reads a section's
    text as `S` and gives its errors, which `check_import_settings`
    calls;
  - `import_settings_sections(&self) -> Vec<&'static str>`, in order of
    name, for messages and for later tools;
  - `check_import_settings(&self, bundle: &Bundle) -> Vec<ContentError>`:
    every settings file of the bundle (`files_under("")`, filtered by
    `is_settings_file`). Each section must be registered (else an error
    at its line naming the registered sections) and must read as its
    type. A `<name>.meta` whose file the bundle lacks (`Bundle::has`)
    and doesn't leave out on purpose (`Bundle::left_out`), or whose
    file is itself a settings file, is an error naming both;
  - `register` and `replace` refuse a format that claims `meta`, with a
    new `RegisterError::Reserved` naming the format;
  - the `Debug` of `Formats` lists the sections;
  - `Formats::standard()` registers no section yet. Each step's ticket
    registers its own: `onebit` (3007), `indexed` (3019), `ds` (3025),
    `lit` (2328).
- **Walkers skip settings files:**
  - `Formats::load_under`, `Formats::load_each`, `Bake::under` and
    `ImageTable::load_with` already skip them, because they keep only
    files a format claims; `ImageTable::load` keeps only `.png` files.
    Tests prove each;
  - `LangPackRon` (`crates/assets/src/lang/formats.rs`, the loop at
    line 141) and `lang::from_files` (`crates/assets/src/lang.rs`, the
    loop at line 566) skip a settings file instead of calling it "not a
    file a language has".
- **Baking:** `Bake::bake` returns `check_import_settings`'s errors with
  its own, so a typo in a settings file refuses the bake. `shuyi check
  --baked` runs both, so it drops an error equal to one already listed
  and reports each once.
- **`shuyi check`** (`check` in `crates/cli/src/contract/check.rs`):
  after `GameTools::check`, it runs `check_import_settings` on the
  bundle and formats of `GameTools::bake(None)`. When that call gives
  an error (the default method's error says the tools don't offer
  `bake`), the
  check goes on without it, and the answer's `limits` say "settings
  files weren't checked: " followed by that error. `docs/contract.md`'s
  row for `bake` (the `GameTools` table) says `check` uses it too.
- **The toy reads a section:** the `*.txtimg` toy
  (`crates/assets/tests/it/txtimg.rs`) gets `TxtImgSettings { mirror:
  bool }`, section `txtimg`, read in `TxtImg::decode` through
  `cx.import_settings`, and registered in its tests. It is the example
  `docs/formats.md` shows.
- **Docs:** `docs/formats.md` gets a section *Import settings*: the two
  names, which wins, layers, a section as a struct, registering it,
  reading it in a format, the errors, and packs. Its *Writing a format
  plugin* gets the toy's settings as a step, and *The rules* a bullet:
  no format claims `meta`. The language pack bullet's "Any other file
  in its directory is an error" leaves settings files out.
- **For the game:** additive, except the new `RegisterError` variants:
  a `match` on `RegisterError` with no wildcard arm stops compiling.
  The PR says so, and the game's PR that moves the pin follows it.
- **Public types (ADR-0039):** the new types follow it. `RegisterError`
  is still exhaustive, and this PR adds to it, so it becomes
  `#[non_exhaustive]` here, as the ADR's *When* says.
- **Either order with 2220.** If 2220 is done, add a test that a `Hot`
  value of the toy kind reloads when its `.meta` changes. If not,
  nothing here: the reads go through the bundle, which 2220 tracks.
- **Either order with 2516.** If 2516 is done, follow the rule its ADR
  records for its files report: a `<file>.meta` counts as read when its
  file was read, and a `_folder.meta` when any file in its folder or
  below was, with a test. If not, 2516 applies it (its clause says so).

**Size.** If the PR passes about 600 changed lines (the ADR and the
docs not counted), the **`shuyi check`** bullet (the contract's `check`
and its `limits` line) goes to a follow-up ticket (`write-ticket`),
After MVP, and the PR says so. `Bake::bake` keeps its check here.

**Out (do not do):**

- Reload work of any kind (2220).
- Per-target layers themselves (3007's `assets@<target>/`).
- 2222's `atlas.ron`, and the engine's RON sidecars (font atlas,
  palette, lights, names, audio manifest): they are assets, and stay as
  they are. Moving any of them into `.meta` is out.
- Merging fields across settings files or across layers. 2228's
  `Bundle::versions` would allow it if a game asks.
- Named presets beyond folder defaults, and an editor GUI.
- Leaving settings files out of packs (ADR point 8): a follow-up if a
  target's budget (3008) needs the bytes.
- Any engine section: each step's ticket registers its own (3007, 3019,
  3025, 2328).
- `shuyi import` showing the settings a file is read with; sections
  and their schemas in `shuyi kinds` (2515); settings files in the
  unused-files report past the 2516 clause above (2516's).
- The templates offering `GameTools::bake`, so `shuyi check` covers
  their settings files: a follow-up when a template bakes.

## Implementation steps

1. Read ADR-0007, ADR-0019, ADR-0023 and ADR-0008 §4; in
   `crates/assets/src/`: `format.rs` (`DecodeCx`, `Formats`,
   `RegisterError`), `bundle.rs` (`bytes`, `has`, `left_out`,
   `files_under`, `watch`), `ron.rs` (`parse_ron`, `error`),
   `pack/bake.rs`, `lang.rs` (`from_files`), `lang/formats.rs` and
   `image.rs`; `crates/cli/src/contract/check.rs`; and 3007's scope.
2. Write the ADR.
3. Write `import_settings.rs`: the names, the chain and the splitter,
   with their unit tests.
4. Add `DecodeCx::import_settings`, the registry's methods, the
   `RegisterError` variants and the `meta` reservation.
5. Skip settings files in `LangPackRon` and `lang::from_files`, and
   test every walker.
6. Add the check to `Bake::bake` and to the contract's `check`, with
   tests through `crates/cli/src/contract/fake.rs`, which offers
   `bake`.
7. Add the toy's section, `docs/formats.md` and `docs/contract.md`.
8. Run the gates, with the `no_std` build of `shuyi-assets`.

## Acceptance criteria

- [ ] A format reads its section from `images/hero.png.meta`; with none,
      from `images/_folder.meta`; with none, from `_folder.meta` at the
      root; with none, `S::default()` (unit tests over a table bundle).
- [ ] A section is taken whole from the nearest file that has it: a
      folder's `dither` doesn't fill in a file's own section that
      leaves `dither` out (unit test).
- [ ] A settings file in a higher layer replaces the lower layer's
      (unit test over two table layers).
- [ ] Bad RON, an unknown section, an unknown field and a bad value
      each give an error with the settings file's shown name and the
      line of the problem in the file, also when it is in the second
      section (unit tests).
- [ ] `register_import_settings` refuses a second section of one name, a
      type without `deny_unknown_fields`, and a type with a field that
      has no default (unit tests).
- [ ] `register` and `replace` refuse a format that claims `meta` (unit
      test).
- [ ] `check_import_settings` reports a `<name>.meta` whose file is
      missing, and a settings file of a settings file, but not a
      `<name>.meta` whose file a layer leaves out on purpose (unit
      tests).
- [ ] `load_under`, `load_each`, `Bake::under`, `ImageTable::load`,
      `ImageTable::load_with`, `LangPackRon` and `lang::from_files` skip
      settings files; a folder of `ron::Ron<T>` tables with a
      `_folder.meta` in it still loads (tests).
- [ ] `Bake::bake` refuses a bundle with a bad settings file; a good one
      is in the pack as its own bytes, and `Bake::check` passes (test).
- [ ] `check` reports a bad settings file of the fake game's bundle,
      `check --baked` reports it once, and `check` says in its `limits`
      that settings weren't checked for a game without `bake` (contract
      tests).
- [ ] The txtimg toy is mirrored by its own `.meta` and by a
      `_folder.meta` (integration test).
- [ ] `bundle::watch` hears of every settings file looked for, found or
      not (in the one test that sets the watcher).
- [ ] Random bytes as a settings file never make `import_settings` or
      `check_import_settings` panic (property test).
- [ ] `shuyi-assets` builds without `std`
      (`cargo build -p shuyi-assets --no-default-features --target
      thumbv7em-none-eabihf --locked`).
- [ ] The ADR, its row in `docs/adr/README.md`, the `Status` lines of
      ADR-0019 and ADR-0008, and the `docs/formats.md` section are in.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`import_settings.rs`): `settings_of` and `settings_chain` for a
  file at the root and a nested one; `is_settings_file`; the splitter on
  `()`, two sections, a comment before a section, a section holding a
  string with `)` and `,` in it, and the line of an error in the second
  section; which wins (own, folder, parent folder, none); whole
  sections; two layers.
- Unit (`format/tests.rs`): the registry's refusals; `meta` reserved in
  `register` and `replace`; `check_import_settings` on an unknown
  section (the message lists the registered ones), an unknown field at
  its line, a missing file, a left-out file, a settings file of a
  settings file, and a clean bundle; the walkers skipping settings
  files.
- Unit (`bundle.rs`): `the_watcher_is_told_of_every_file_read` reads a
  section and sees `("images/hero.png.meta", false)`, then
  `("images/_folder.meta", true)`.
- Unit (`lang`): a `_folder.meta` in a language's directory is no error
  for `LangPackRon` or for `lang::from_files`.
- Unit (`pack`): the bake refusing a bad settings file; a good one kept
  as its bytes; `Bake::check` passing.
- Property: random bytes, and a valid settings file with a few
  characters changed, never panic the splitter, `import_settings` or
  `check_import_settings` (ADR-0019's never-a-panic rule).
- Integration: the txtimg toy with its settings
  (`crates/assets/tests/it/txtimg.rs`); `check` and `check --baked`
  with the fake game (`crates/cli/src/contract/fake.rs`, a bad settings
  file added to a copy of its sources).

## Completion notes

*(Filled in by the session that completes the ticket.)*
