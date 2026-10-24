---
id: "2516"
title: "References between assets: Ref<K> ids checked by shuyi check with file and line, and the shipped files no load read reported with the largest files"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2511"]
nick_input: none
completed:
---

# 2516 — References checked, and the files no load reads

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engine Defold 1.14. Unreal checks assets
with its Data Validation plugin
(https://dev.epicgames.com/documentation/en-us/unreal-engine/data-validation-in-unreal-engine)
and shows what refers to what in its Reference Viewer and Asset Audit.
Defold and Unity build only what the game's main collection or scenes
reach, so a file nothing uses never ships.

What Shuyi has today:

- **Ids are plain strings.** A game's table names another asset by a
  `String`, and only the game's own validators check it. Checks across
  assets run after loading, as functions the game calls (ADR-0019 §5;
  `docs/formats.md`, *Checks after loading*).
- `GameTools::check` (`crates/cli/src/game.rs` line 103) returns what
  the game's validators find, and `check::check`
  (`crates/cli/src/contract/check.rs`) lists it. A mistyped id is found
  only if the game wrote a check for it, or when the screen that loads
  it runs.
- Every registered format decodes through `format::decode_with`
  (`crates/assets/src/format.rs` line 303), called by
  `Formats::decode`. A loader may also call `parse_ron` or `load_ron`
  (`crates/assets/src/ron.rs`) itself, as the grid template's
  `Data::load` does (`templates/grid/crates/game/src/lib.rs`).
- `bundle::watch` (`crates/assets/src/bundle.rs` line 453) tells one
  process-wide watcher of every file `Bundle::bytes` reads. It is set
  once, and nothing calls it today. `Bundle::has` tells it nothing.
  The grid template reads its files with `include_dir`'s `get_file`, not
  through a bundle, so no watcher would see them.
- `error::line_of` (`crates/assets/src/error.rs` line 69) finds a line
  by searching a file's text.
- Sizes: 2204's `shuyi bake` prints a pack's total size, 3004 writes
  sizes to `export-report.json`, and 3008 gates a package's size. None
  lists the files a game never reads, or the biggest ones.
- 2511 (done) added the short path: `shuyi_cli::BasicTools`
  (`crates/cli/src/basic.rs`), `BasicData` with `formats()`
  (`crates/ui/src/basic.rs`) and `BasicGame::bundle`
  (`crates/platform/src/basic.rs`). The blank template's tool program
  is `examples/tools.rs`, on `BasicTools`.
- 2509 (done) gave the contract's problems stable codes and help
  (`crates/cli/errors/`).

Hello, Grid shows the second gap: it ships `font/Terminus-LICENSE.txt`
and `font/atlas_1bit.bin` (`examples/hello-grid/files/font/`), and
`Hello::load` (`examples/hello-grid/src/lib.rs`) reads neither.

What is planned:

- 2220's `Hot<T>` records the files each decode read;
- 2228's `Bundle::versions` reads every layer's copy of a file through a
  helper shared with `Bundle::bytes`;
- 2229 adds `.meta` settings files beside files;
- 2515 lists each format's schema;
  2517 adds `CheckOutput::warnings`; 2230 adds JSON and CSV tables;
- 3004 and 3008 measure packages.

Why it matters:

- **The game first.** Visions of Shuyi's data names other files by id.
  A typo should fail `shuyi check` with the file and the line, not show
  up when a playtester reaches that screen.
- **Package size.** Files nobody reads waste the Playdate's package and
  the web build's download (3010, 3008's budgets).
- **The end goal** (2026-10-19): agents write data files. A wrong id
  caught by `shuyi check` is the same kind of help the contract already
  gives for one file.

## Nick input

None.

## Scope

**In:**
- **An ADR** (the `write-adr` skill, the next free number) extending
  ADR-0019 §5:
  - a reference is collected while a file decodes, and resolved after
    the game's content has loaded, so a format still reads one file;
  - ids stay bundle paths, as `Formats::load` takes them: a renamed
    file breaks its references, and the check finds them;
  - a reference read by a loader outside the registry is still checked,
    but names no file;
  - the files report is a warning about one checked run, never proof
    that a file is unused, and it never removes anything.
- **`shuyi_assets::refs`** (`crates/assets/src/refs.rs`):
  - `Ref<K: AssetKind>`: the id of an asset of kind `K`, with or
    without its extension, written as a plain string in RON, JSON and
    CSV. `Ref::new(id)`, `Ref::id()`, `Ref::load(&self, formats,
    bundle)` (that is, `formats.load::<K>(bundle, self.id())`).
    `Serialize` and `Deserialize` as a string; `Clone`, `PartialEq`,
    `Eq`, `PartialOrd`, `Ord`, `Hash` and `Debug` written by hand
    (`PhantomData<fn() -> K>`). It builds without `std`.
  - **Collected while decoding (with `std`):** while this thread is
    inside `refs::collecting`, deserializing a `Ref<K>` records a
    `Reference { kind: &'static str, id: String, file:
    Option<AssetPath>, line: Option<u32>, resolve: fn(&Formats,
    &Bundle, &str) -> Result<(), Vec<ContentError>> }`. `kind` is
    `K::NAME`. `resolve` is `Formats::load::<K>`, dropping the asset.
  - **`format::decode_with`**, inside `collecting` only:
    - it marks the file it decodes for this thread, and puts the outer
      mark back when it returns, so a format that decodes another file
      inside it records each reference under its own file;
    - a failed decode drops the references it recorded (so a format
      tried first and refused adds none);
    - a successful one gives each of its references a line. The file's
      bytes, as UTF-8, are searched for the id as a whole word (neither
      neighbour a letter, a digit, `_`, `-`, `.` or `/`), starting after
      the previous reference's match in that file, so two references to
      one id get their own lines. If it isn't found there, the search
      starts again from the top. Bytes that aren't UTF-8 (a pack's
      binary asset) give no line. Add the search beside
      `error::line_of`.
  - A `Ref` read inside `collecting` but outside any `decode_with` (a
    loader that calls `parse_ron` itself) is recorded with no file and
    no line.
  - `refs::collecting<T>(f: impl FnOnce() -> T) -> (T, Vec<Reference>)`
    gives the references in the order they were read. It puts the old
    state back when `f` panics, as `bundle::reading` does (`bundle.rs`
    line 469). Nested, the outer call gets the inner one's references
    too.
  - Without `std`, and outside `collecting`, a `Ref` records nothing
    and `decode_with` marks nothing.
- **Reads recorded per thread** (`bundle::recording<T>(f: impl FnOnce()
  -> T) -> (T, BTreeSet<String>)`, with `std`): the paths
  `Bundle::bytes` found while `f` ran on this thread. Unlike `watch`, it
  is scoped and can be used by more than one tool at once. Nested, the
  outer call sees the inner one's reads too. `Bundle::has` still
  records nothing.
- **`Bundle::files_where(&self, keep: impl Fn(&Layer) -> bool) ->
  Vec<(&'static str, usize)>`**: the paths, sorted and once each, of the
  files of the layers `keep` accepts, each with its size in bytes in the
  first of those layers that has it. Like `has`, it reads nothing:
  neither `watch` nor `recording` hears of it.
- **The game hands over its assets:** `GameTools::assets(&self) ->
  Option<Result<GameAssets, String>>`, with `GameAssets { bundle:
  Bundle, formats: Formats }`: the bundle and formats its `check`
  loads with.
  - The default calls `self.bake(None)`. `Ok` gives its bundle and
    formats. The error `unsupported("bake and check --baked", "bake")`
    (the game doesn't offer `bake`) gives `None`. Any other error gives
    `Some(Err(e))`.
  - Hello, Grid gets it through its `bake`, with no change.
  - `shuyi_cli::BasicTools` (2511) writes it: a `Bundle` of its
    `BasicGame`'s files layer (`BasicGame::bundle`), as
    `BasicData::load` reads, and `BasicData::formats()`. It must not
    load the data: a game
    whose files are broken still gets its errors.
  - `Fake` (`crates/cli/src/contract/fake.rs`) writes it, for the tests.
  - The template's grid stage tools
    (`templates/shuyi-default/stages/top-down-grid/crates/game/src/stages/top_down_grid/tools.rs`)
    write `assets` as `BasicTools` does: 2501 moved the grid game's load
    onto `BasicData`, and its `Tools` calls `BasicTools` for the rest.
- **`shuyi check`** (`check::check`):
  - Only `game.check()` runs inside `refs::collecting` and
    `bundle::recording`. With `--baked`, `bake::check_baked` runs after,
    outside both: baking decodes and reads every source again, which
    would record each reference twice and hide the unread files.
  - **References.** With the game's assets, each `(kind, id)` is
    resolved once with its `resolve`, still inside the recording (a
    file reached only through a reference counts as read). A failure is
    an error in `errors` for each reference to it: the reference's
    file and line, and the message `no <kind> "<id>": <the load's first
    error>`. A reference with no file is an error of the id's own shown
    name (`bundle.display_path(id)`), whose message adds "(read outside
    the format registry)".
  - The answer's limits say how many references have no line, and how
    many have no file because a loader read them outside `Formats`.
  - **The files.** `CheckOutput` gains `files: Option<FilesReport>`
    (`#[serde(default)]`): `shipped: u32`, `read: u32`, `unread:
    Vec<String>` (shown names, sorted), and `largest: Vec<FileSize {
    file: String, bytes: u64 }>`, the ten largest shipped files,
    largest first.
    - **Shipped** means the files of layers that are neither test nor
      private (`files_where`). A private layer's files (bought art) are
      left out of both lists: they are often absent where `check` runs
      (CI checks out no bought art), so the lists would differ by
      machine. The limits say how many files of private layers were
      left out.
    - The report never fails `check`: `ok` doesn't change. The text
      prints "never read by this check: N files" and the names, then
      the ten largest with their sizes. The limits say it covers the
      files no load read in this checked run, and that a file the game
      reads only at run time, by an id it builds, or without the
      bundle, is listed too.
    - If the check read no shipped file at all, there is no report,
      and the limits say the game's `check` read nothing through its
      bundle.
  - Without the game's assets, references are collected but not
    resolved, and there is no files report. The limits say so, naming
    `GameTools::assets`. An `Err` from `assets` fails the answer, as an
    error of `bake` does in `check --baked`.
  - `check --baked` does all of this for its `check` half. `check
    --target` doesn't.
  - The answer changes shape: `contract::VERSION` raised by one from
    whatever it is when this lands, the schemas regenerated and the line
    added to `crates/cli/schemas/versions.json` (`docs/contract.md`,
    *The schemas*).
- **Either order with 2220:** if 2220 is done, it added
  `bundle::recording` as described here and `Hot<T>` records through it
  (its clause says so): reuse it. If not, 2220 uses
  `bundle::recording`.
- **Either order with 2228:** if 2228 is done, `Bundle::bytes` and
  `Bundle::versions` tell of a read through one private helper: record
  there, so `versions`' reads are recorded too. If not, record in
  `Bundle::bytes`, and 2228 moves it into its helper (its clause says
  so).
- **Either order with 2229:** if 2229 is done, a `<file>.meta` counts
  as read when its file was read, and a `_folder.meta` when any file in
  its folder or below was read. If not, the ADR states that rule, and
  2229 follows it.
- **Problem codes (2509, done; ADR-0028):** the dangling-reference
  error is a `Problem` with a content code of its own: a new row
  (`dangling-reference`, the next free `SH20nn`) in
  `crates/cli/src/contract/codes.rs`'s `TABLE`, its constant in
  `shuyi_assets::error::codes` and its page in `crates/cli/errors/`
  (`docs/contract.md`, *Problem codes*). The files report (`files`) is
  not a problem and gets no code.
- **Either order with 2515:** if 2515 is done, `Ref<K>` implements
  `JsonSchema` behind its `schema` feature: a string whose description
  names kind `K`. If not, 2515 adds that impl with the feature.
- **Either order with 2517:** the unread files stay in `files` and do
  not go in `CheckOutput::warnings`: they are a report about one run,
  not a problem of a file.
- **Either order with 2235** (nine-slices in a data file): if 2235
  is done, its entries' image paths are plain bundle paths: make them
  `Ref<ImageInfo>` (the `image` kind), so `check` gives a missing
  image its file and line. If not, nothing to do (2235 writes them
  as `Ref`s).
- **Either order with 3004:** if 3004 is done, `shuyi export` writes
  the files report's `unread` and `largest` into `export-report.json`,
  taken from a plain `shuyi check` run (`check --target`, which export
  runs first, makes no files report), with a test. If not, nothing to
  do (3004's clause says so).
- **Either order with 3008:** if 3008 is done, a package over budget in
  `shuyi bench` names the files report's `largest` and `unread` files,
  taken from `check`'s answer, not counted again. If not, nothing to do
  (3008 does it, its own clause says so).
- **Public types (ADR-0039):** the new public types follow it.
- **Docs:**
  - `docs/formats.md` gets a section "References": `Ref<K>`, what is
    checked, ids as paths, loads through `Formats` for a file and a
    line, and no `Ref` inside a `#[serde(untagged)]` enum, since serde
    may try a variant and drop it;
  - `docs/contract.md`: the `check` row says what it reports now, and
    *What a game gives* gets an `assets` row.
- **Size.** If the PR passes about 600 changed lines (the schemas
  aside), move the files report (`recording`, `files_where`,
  `FilesReport`) to a new ticket with the `write-ticket` skill, and
  land the references first.

No API break: a new module, new methods with defaults, and a new answer
field. `decode_with` changes only inside `collecting`. The game's next
pin bump needs no change; its tools get the report through `bake` or by
writing `assets`. Say so in the PR.

**Out (do not do):**
- references to an entry inside a table (an id that names a row, not a
  file): the game's validators check them today; a follow-up if a game
  asks;
- short ids under a folder (`"forest"` for `tiles/forest`);
- field paths in messages: serde doesn't tell a value where it sits;
- moving the grid template's load onto a bundle and the registry (2501
  moves it onto `BasicData`);
- map objects read into a `Ref<K>` (parked 2232 marks its map's file
  when it lands);
- JSON and CSV tables (2230) and schemas of formats (2515);
- removing unread files from a package or a pack: the report never
  removes anything (the ADR above), and no ticket plans it (3004 and
  2204 don't); a ticket if a game asks;
- a report per target, over 3007's `assets@<target>/` layers: a
  follow-up if a game asks;
- ids that survive a rename;
- the inspector (2507).

## Implementation steps

1. Read ADR-0019, `crates/assets/src/bundle.rs` (`watch`, `reading`,
   `story_only`, `list`, `Files::get`), `format.rs` (`decode_with`,
   `Formats::decode`, `Formats::load`), `ron.rs`, `error.rs`,
   `crates/cli/src/contract/check.rs`, `contract/bake.rs`
   (`check_baked`), `game.rs`, `contract/fake.rs`, and 2511's
   `BasicTools` and `BasicGame`. Check whether 2220, 2228, 2229, 2501
   and 2515 are in `tickets/done/`.
2. Write the ADR.
3. Add `refs` with `Ref<K>`, `Reference` and `collecting`, then the
   marking, the dropping and the line search in `decode_with`.
4. Add `bundle::recording` and `Bundle::files_where`.
5. Add `GameAssets` and `GameTools::assets` with its default, and write
   it in `BasicTools` and in `Fake`.
6. Extend `check::check` and its text; regenerate the schemas and add
   the version line.
7. Write the docs, settle the *Either order* clauses, and run the gates.

## Acceptance criteria

- [ ] A test game whose table, loaded through `Formats`, holds a
      `Ref<PaletteDef>` to a missing file fails `check` with an error
      naming the table's file, the id's line and "no palette".
- [ ] Two references to the same missing id in one file get two errors,
      each with its own line.
- [ ] A reference to a file that exists but isn't of its kind fails
      with that file's load error.
- [ ] A dangling `Ref` read by a loader that calls `parse_ron` itself
      fails `check` with an error of the id's shown name, and the limits
      count one reference with no file.
- [ ] `cargo shuyi check` in `examples/hello-grid` still passes, and its
      files report lists the shipped files its loader doesn't read
      (`files/font/Terminus-LICENSE.txt` and `files/font/atlas_1bit.bin`
      on 2026-10-20) and its ten largest files. `check --baked` lists
      the same unread files.
- [ ] A file of a test layer or a private layer is in neither list.
- [ ] A game whose tools offer neither `assets` nor `bake` gets the
      limit naming `GameTools::assets`, and so does a game from `shuyi
      new grid` while the grid template loads without a bundle.
- [ ] A game whose `check` reads no file through its bundle gets no
      files report, and the limit that says so.
- [ ] `shuyi-assets` still builds for `thumbv7em-none-eabihf` without
      default features.
- [ ] The contract's schema test passes with `VERSION` raised.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, `shuyi-assets`:
  - `Ref<K>` round trip through RON as a string;
  - `collecting` records the file and the line, drops a failed decode's
    references, keeps each nested decode's references under its own
    file, records no file outside `decode_with`, and restores its state
    after a panic;
  - the line search: two references to one id, an id that is part of a
    longer word, bytes that aren't UTF-8;
  - `recording` nests and sees only its own thread;
  - `files_where`: sizes, the first layer wins, and it records no read.
- Unit, `shuyi-cli`: `GameTools::assets`' default with a game that
  offers `bake`, one that doesn't, and one whose `bake` fails;
  `BasicTools::assets` on broken files.
- Property: decoding a table with random `Ref` ids records exactly one
  reference per `Ref`, in order.
- Integration: `check` on a fake game with a dangling reference, with
  and without assets, and with `--baked`; `check` on
  `examples/hello-grid`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
