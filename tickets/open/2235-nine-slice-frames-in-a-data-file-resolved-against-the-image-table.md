---
id: "2235"
title: "Nine-slice frames in a data file: a NineSliceDefs kind and its RON format, resolved against the image table, baked in the pack and read by BasicData"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["2338"]
nick_input: none
completed:
---

# 2235 — Nine-slice frames in a data file

## Context

2338 draws a picture in nine slices and frames the engine's panels
with it. Its `shuyi_assets::nine_slice` module holds the plain types
(`NineSlice`, `EdgeFill`, `CentreFill`), and `GameData::nine_slice`
gives the frame for a name (`"panel"`, `"focus"`, or a game's own).
Until this ticket a game builds those `NineSlice`s in code: it looks
its picture up in the `ImageTable` and writes the borders as numbers
in Rust. 2338 left the file format, and the pack's form of it, to
this ticket (its Out list and its step 8).

The engines 2338 compared with keep the borders as data beside the
picture: Godot's `StyleBoxTexture` resource, Unity's sprite borders,
GameMaker's nine-slice per sprite, Defold's slice-9. Here they come
from a standard text file too, so:

- a frame changes with an edit to a data file, not a rebuild;
- an agent can write one, and `shuyi check` catches its mistakes with
  the file and the entry;
- it follows ADR-0007: a kind, a standard text format of it, and the
  pack as its optimized form, proven equal (ADR-0023);
- a game on the short path (`BasicData`, ADR-0036) frames every
  engine panel by adding one picture and one file, with no code. That
  is the end goal's "games in few lines" (2026-10-19).

A format reads one file (ADR-0019 §5). An `ImageId` exists only in an
`ImageTable`, so the file names its image by bundle path, and a
`resolve` after loading turns the entries into `NineSlice`s, as the
other checks across assets do (`docs/formats.md`, *Checks after
loading*).

## Nick input

None.

## Scope

**In:**
- **The kind**, in `shuyi_assets::nine_slice`
  (`crates/assets/src/nine_slice.rs`, 2338's module), building
  without `std`:
  - `NineSliceDefs { frames: BTreeMap<String, NineSliceDef> }`, an
    `AssetKind` whose `NAME` is `"nine-slice frames"`;
  - `NineSliceDef { image: String, rect: Option<ImageRect>, left: u32,
    top: u32, right: u32, bottom: u32, edges: EdgeFill, centre:
    CentreFill, zoom: u8 }`: the image's full bundle path (as
    `ImageTable::id` takes it, extension and all), the part of it the
    frame uses (the whole image when `None`), the borders in image
    pixels, how the edges and the centre fill, and how many console
    pixels one image pixel is. `rect`, `edges` (`Stretch`), `centre`
    (`Stretch`) and `zoom` (1) may be left out of the file
    (`#[serde(default)]`). An unknown field is an error
    (`#[serde(deny_unknown_fields)]`), so `center` for `centre` is
    caught;
  - both derive `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`,
    so the kind is `Bakeable`. `EdgeFill` and `CentreFill` gain
    `Serialize` and `Deserialize`, and so does `ImageRect` if it
    doesn't have them yet. Each of these only adds.
- **Its format, `NineSliceRon`** (id `nine-slices`, extension `ron`,
  `TEXT_MAX_BYTES`), registered in `Formats::register_standard`. The
  file is a map of name to entry, as `names.ron` is:

  ```ron
  {
      "panel": (
          image: "ui/frames.png",
          rect: (x: 0, y: 0, w: 24, h: 24),
          left: 8, top: 8, right: 8, bottom: 8,
          edges: Tile,
          zoom: 2,
      ),
      "focus": (
          image: "ui/focus.png",
          left: 6, top: 6, right: 6, bottom: 6,
      ),
  }
  ```

  It is read with `parse_ron_implicit_some`, so `rect` is written
  without `Some(…)`. It refuses these, reporting every problem with
  the file, the entry's name and its line (`error::line_of`, as
  `names::from_source` finds a name's line):
  - a `zoom` of 0;
  - a `rect` 0 wide or 0 tall;
  - borders past a given `rect`: `left + right > w` or `top + bottom >
    h`, added without overflow. Borders that meet (`left + right ==
    w`) are allowed: 2338 then draws no edge or centre there;
  - an empty name, and a name listed twice (RON's map keeps only the
    last one, so the source is searched as `names::from_source` does).
- **`NineSliceDefs::resolve(&self, file: &str, images: &ImageTable) ->
  Result<BTreeMap<String, NineSlice>, Vec<ContentError>>`**: each
  entry's `NineSlice`, built with 2338's constructor, its `Picture`
  being `images.id(&def.image)` and the rect. `file` is the shown name
  of the file the defs came from (`bundle.display_path`). It reports
  every problem, each naming `file` and the entry:
  - no image at that path in the table;
  - a `rect` that reaches past the image's edges (added in `u64`);
  - with no `rect`, borders past the whole image.
- **The pack.** The kind goes in
  `the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version`
  (`crates/assets/src/pack/tests.rs`), and in that file's `sources()`
  and `every_kind()`. The snapshot gains the kind's line, so `VERSION`
  (`crates/assets/src/pack.rs`) becomes the next number, as the test's
  comment says.
- **`shuyi import`:** `Importers::standard()`
  (`crates/cli/src/contract/import.rs`) lists the kind, summed up with
  `count` (`2 frames`).
- **The short path:** `BasicData` (`crates/ui/src/basic.rs`) reads
  `frames.ron` (a new constant, `FRAMES_RON`) when the bundle has it
  (`Bundle::has`), resolves it against its `images`, and keeps the
  `NineSlice`s in a private field. Its `GameData::nine_slice` answers
  from them. With no file it answers `None`, as before. A broken file,
  or one naming a missing image, fails `BasicData::load` with every
  problem, so `shuyi check` on a game with `BasicTools` shows them.
- **Public types (ADR-0039):** `NineSliceDefs` and `NineSliceDef` are
  data read from files (its kind 5), built by games as its kind 2:
  `#[non_exhaustive]` with public fields; `NineSliceDefs` with
  `Default` and `new(frames)`; `NineSliceDef::new(image, left, top,
  right, bottom)` with the optional fields at their defaults, to be
  assigned. `NineSliceRon` is a unit struct, as `NamesRon` is. No
  `Breaking:` line: every change here adds.
- **Docs:** `docs/formats.md` gets the kind's row in *The built-in
  formats*, a bullet in *Checks after loading* (`resolve` against the
  image table) and `frames.ron` in what `BasicData::load` reads. The
  module docs show the file above. `GameData::nine_slice`'s doc says
  that `BasicData` reads `frames.ron`.
- **No ADR:** the kind, its format and its check follow ADR-0007 and
  ADR-0019 as they stand.
- **Either order with 2516:** if 2516 is done, `image` is a
  `Ref<ImageInfo>` (the `image` kind), written as the same string, so
  `shuyi check` gives a missing image its file and line. `resolve`
  looks `Ref::id` up with `ImageTable::id`, and the pack's binary form
  doesn't change (a `Ref` is serialized as its string). If not,
  `image` is a plain `String`, and 2516 makes it a `Ref` (its clause
  says so).
- **Either order with 1123** (parked): if 1123 is done, add
  Aseprite's slice centres as a second format of the kind (`AGENTS.md`
  rule 10), as 1123's Scope describes them. A slice of `meta.slices`
  with a `center` becomes an entry named as the slice; its `image` is
  the sheet `meta.image` names beside the JSON, its rect the slice's
  bounds placed in the sheet by its frame, and its borders the
  centre's distance from the bounds' edges. It reads `.json`, is found
  as 1123's Aseprite format is (by its `meta.app`), uses 1123's JSON
  reading, has a property test, and gives errors with the file and the
  JSON path. If not, nothing to do (1123 reads them into this kind,
  its clause says so).
- **Either order with 2330:** if 2330 is done and its `.ui.ron` nodes
  have no `frame` field (2330 landed before 2338), add it as 2330's
  Scope says: an optional `frame: "name"` on `Menu`, `TitledBox`,
  `Question` and `TextBox`, a `GameData::nine_slice` name drawn in
  place of the frame `Ctx::panel_frame` gives, with a look test. If
  2330 has the field, or isn't done, nothing to do (2330 adds it, since
  2338 is done).

**Out (do not do):**
- a picture inside engine panels, and borders scaled by anything but a
  whole zoom (2338's Out list, which still holds);
- slice borders read from 2229's import settings: 2229 plans no
  section for them (a follow-up if a game asks);
- borders kept per image or per sprite (Unity's way): a frame is a
  named entry;
- other tools' nine-patch data (TexturePacker's, Godot's `.tres`): a
  follow-up if a game asks;
- an `Encode` for the format and `shuyi import --print` of it: a
  follow-up if a game asks;
- a `GameData` other than `BasicData` reading `frames.ron`: a game
  with its own `GameData` loads and resolves the kind itself, as the
  module docs show;
- frames in the templates and the examples.

## Implementation steps

1. Read 2338's `nine_slice` module as built (its `NineSlice`
   constructor and its derives), `names.rs` (a RON map format and its
   name lines), `ron.rs` (`parse_ron_implicit_some`), `image.rs`
   (`ImageTable::id`, `ImageTable::info`, `ImageRect`),
   `pack/tests.rs`, `format/builtin_tests.rs` and
   `crates/ui/src/basic.rs`. If 2338's types came out other than its
   ticket says, follow the code. Check whether 1123, 2330 and 2516 are
   in `tickets/done/`.
2. Write `NineSliceDefs`, `NineSliceDef` and `NineSliceRon`, with
   their errors, and register the format in
   `Formats::register_standard`.
3. Write `resolve`.
4. Add a `FRAMES` fixture to `format/builtin_tests.rs`: in
   `fixtures()`, in the id, extension and limit test, in the UTF-8
   test, and in `no_bytes_panic_a_ron_sidecar`. Add the kind to the
   pack's tests, raise `VERSION` and accept the new snapshot.
5. Add the kind to `Importers::standard()`.
6. Read `frames.ron` in `BasicData` and answer `nine_slice` from it.
7. Do the either-order parts whose tickets are done: 2516's `Ref`,
   1123's Aseprite slices, 2330's `frame` field.
8. Write the docs.
9. Run the gates, with the no-std build of `shuyi-assets`.

## Acceptance criteria

- [ ] The fixture's two entries (one with a rect, `Tile` edges and
      zoom 2; one of a whole image with the defaults) load through
      `Formats::standard()` and resolve to the `NineSlice`s built by
      hand with 2338's constructor.
- [ ] A zoom of 0, a rect 0 wide, borders past the rect, an empty
      name and a name listed twice each fail decoding with the file,
      the entry's name and its line. One file with all of them reports
      them all.
- [ ] Borders that meet exactly (`left + right == w`) are accepted.
- [ ] A missing image, a rect past the image's edges and, with no
      rect, borders past the image each fail `resolve`, naming the
      file and the entry.
- [ ] The pinned binary-form test has the kind's line, `VERSION` is
      the next number, and the pack's test that reads every kind from
      a pack passes with the new kind.
- [ ] `shuyi import` on the fixture names the kind and says
      `2 frames`.
- [ ] `BasicData` with a `frames.ron` gives its `"panel"` entry from
      `nine_slice`; with none, `None`; with one naming a missing
      image, `BasicData::load` fails, naming `frames.ron` and the
      entry.
- [ ] `docs/formats.md` lists the kind, its check after loading, and
      `frames.ron` in what `BasicData::load` reads.
- [ ] `shuyi-assets` still builds for `thumbv7em-none-eabihf`.
- [ ] If 2516 is done: a `frames.ron` naming a missing image fails
      `shuyi check` with its file and line.
- [ ] If 1123 is done: a slice with a centre in 1123's Aseprite
      fixture reads into an entry, its rect and borders checked by
      hand.
- [ ] If 2330 is done without the field: a `.ui.ron` menu with a
      `frame` name wears that frame (a look test).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the fixture against hand-built `NineSlice`s; each refusal of
  the format and of `resolve`; borders that meet; `BasicData` with,
  without, and with a broken `frames.ron`; the import summary.
- Property: random bytes, and the fixture with a few characters
  changed, never panic the format (`no_bytes_panic_a_ron_sidecar`).
  `resolve` over random rects, borders, zooms and image sizes, at the
  `u32` extremes, never panics, and every `NineSlice` it gives has its
  rect inside its image and its borders within its rect.
- Determinism (ADR-0007 §5): the fixture decodes to the asset built by
  hand, with integers only, so it is the same on every platform.
- Snapshot: the pack's pinned binary form, with the kind's line.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
