---
id: "2228"
title: "Bundle::versions: every layer's copy of a file, for loaders that merge layers (mods, DLC, per-target layers)"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2228 — Every layer's copy of a file, for loaders that merge layers

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and by a split that game#1157 missed.

- **The game ticket:** game#1013
  ([`1013-mod-data-tables-merge-by-id.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1013-mod-data-tables-merge-by-id.md)).
  Its mods add, replace and remove a data table's entries by id instead
  of replacing the whole file. Its step 2 adds
  `bundle::all_versions(path)`, "every layer's copy of a path in order".
  It was written when the bundle was still the game's. The bundle is
  now the engine's (`crates/assets/src/bundle.rs`), so that function
  is this ticket. The merge itself (keys, remove lists, what two mods
  that change one entry do) stays the game's: game#1011 decides it, and
  2207's Out leaves merging data tables entry by entry to the game's
  loaders.
- **The job:** a loader reads the game's copy of a data table and the
  copy each mod or DLC layer adds, and folds them into one table.
- **Friction:** no friction review: a missing feature.
- **What the benchmarks have:** Unreal's Data Registries merge rows
  from ordered sources that Game Feature plugins add
  (<https://dev.epicgames.com/documentation/en-us/unreal-engine/data-registries-in-unreal-engine>).
  Godot's resource packs
  (<https://docs.godotengine.org/en/stable/tutorials/export/exporting_pcks.html>)
  replace whole files, and Godot, Bevy and Unity leave merging data to
  each game. So the engine gives the read that makes a merge possible,
  not a merge framework.
- **What Shuyi has today** (`crates/assets/src/bundle.rs`):
  - `Bundle::bytes`, `Bundle::file` and `Bundle::display_path` see only
    the first layer that has a path (the private `find`, line 393);
  - `Files::get` (line 45), `Bundle::read` (line 195) and
    `story_only_now` (line 516) are private;
  - `Bundle::test_part` (line 248) is a narrow form of this read: the
    test layers' own copy of a path a shipped layer also has, "the
    entries a data table gains in a build with the test files";
  - `Bundle::layers` and the public `Files` enum would let a game match
    each variant itself. It would then copy the engine's lookup, and it
    could not hide the test layers inside `story_only` (the flag is
    private). It would skip the `watch`er (`watch`, line 453), so
    2220's `Hot<T>` and 2516's read log would not see those reads. And
    it would break when 2207 adds a run-time variant.
  - In a baked build the game's tables sit in the pack (2204, done:
    `pack::Pack::into_layer`, `crates/assets/src/pack.rs`) in binary
    form, not as RON text. So a loader must decode each copy through
    the registry (`Formats`, `crates/assets/src/format.rs`), not parse
    raw text.
- **The generic shape:** every layer's copy of one path, in the order
  the bundle reads layers, each with the name errors show it by
  (`mods/more-crops/data/crops.ron`); and the same through the registry,
  each copy decoded as a kind.
- **Outside tactics:** a farming game whose mods each add crops to
  `data/crops.ron`; a platformer's DLC adding levels to a level list; a
  build with per-target layers (3007) that adds rows to a table instead
  of replacing it.

## Nick input

None.

## Scope

**In:**
- **`bundle::LayerCopy`**: one layer's copy of a file, a plain struct
  with `#[derive(Debug, Clone, PartialEq, Eq)]`:
  - `layer: &'static str`, the layer's name;
  - `shown: String`, its human-facing name (`base/data/crops.ron`), as
    `display_path` shows a file of that layer;
  - `bytes: &'static [u8]`;
  - `fn text(&self) -> Option<&'static str>`, the bytes as UTF-8.
- **`Bundle::versions(&self, path: &str) -> Vec<LayerCopy>`**:
  - every layer this thread reads that has the file, **in the order the
    bundle reads layers**: the first is the copy `Bundle::bytes` reads
    (a mod laid over the game comes before the game's copy). A loader
    that folds lower layers under higher ones walks it with
    `.iter().rev()`. The doc comment says this in so many words,
    because game#1013's text says "game first, mods after", which is
    the reverse;
  - inside `story_only` the test layers' copies are hidden, as every
    other read hides them;
  - empty when no layer has the file;
  - it tells the `watch`er once, `(path, found)`, as `Bundle::bytes`
    does. Both go through one private helper (the watcher call moved
    out of `bytes`), so a later read log hooks in at one place. Not
    once per copy: a `Watcher` is told a path, not a layer, so each
    copy would repeat the same path, and a change to any layer's copy
    is a change at that path for 2220's read tracking. The doc comment
    says so.
- **`Bundle::test_part`** is rebuilt on the same private walk over the
  layers, with the same answers (its test passes unchanged). It keeps
  not telling the watcher, as today.
- **`Formats::load_versions::<K>(&self, bundle: &Bundle, id: &str) ->
  Result<Vec<(LayerCopy, K)>, Vec<ContentError>>`**: the path is found
  as `Formats::load` finds it (`Formats::find`, extension optional; a
  missing or ambiguous id is `find`'s error), and each copy from
  `Bundle::versions` is decoded as kind `K` with `Formats::decode`, in
  the same order:
  - each copy's errors name that copy
    (`AssetPath::new(path).shown_as(copy.shown.as_str())`), so a broken
    mod file is reported as `mods/<id>/data/crops.ron`, not as the
    game's file;
  - every copy's errors are reported, not only the first (all or
    nothing, as `load_under`);
  - the registry's `limits` apply to each copy
    (`DecodeCx::in_bundle(bundle).with_limits(self.limits)`, as
    `Formats::read` does);
  - a file a format names through `DecodeCx` (a sidecar's picture) is
    still read from the whole bundle, first layer first; the doc comment
    says so.
- **Docs:** the module comment of `bundle.rs` gets one paragraph, and
  `docs/formats.md` *The parts* gets one bullet: a loader that merges
  entries (mods or DLC adding to a table) reads `versions` or
  `load_versions` and folds the copies; the merge rules are the game's.
- **For the game:** additive. The game's PR for game#1013 adds
  `"engine#2228"` to its `blocked_by`, uses `Formats::load_versions`
  (or `Bundle::versions`) in place of its own `all_versions`, walks the
  copies in reverse, and moves the pin after this one.
- **Either order with 2207.** Run-time layers go through the same
  private walk (`Files::get`). If 2207 is done first, this ticket adds
  a unit test that a run-time layer laid over an embedded one gives its
  copy first; if not, 2207's tests add that test.
- **Either order with 2516.** If 2516's `bundle::recording` is in,
  `versions` records its path when found, as `bytes` does, through the
  shared helper. If not, 2516 adds its recording in that helper, so
  `versions` is recorded too.
- **Public types (ADR-0037):** `LayerCopy` follows it.

**Out (do not do):**
- Merging entries: a `Merge` trait, `Keyed<T>`, remove lists, key
  functions, `Formats::load_layered`. They are the game's (game#1011,
  game#1013, 2207's Out, `AGENTS.md` rule 4). A merge helper comes when
  a second game needs one (`docs/decisions.md`, 2026-10-16, and
  ADR-0009 §7: "A module is written when a kind of game needs it").
- Hiding a lower layer's whole file (`Layer::removing`): no user. Mods
  remove entries by key, inside a table; `Layer::leaving_out` already
  marks files a build does without.
- Mods' folders, manifests and load order (game#1014, over 2207).
- Merging maps, pictures, sounds or dialogue: they replace whole files.
- Copies of one table at different extensions (a mod's `crops.json`
  over the game's `crops.ron`, once 2230 adds JSON tables):
  `Formats::find` already refuses two candidates for one id. A
  follow-up if a game asks.
- Per-target layers themselves (3007): they replace whole files by
  design, and this ticket changes nothing there.
- Read tracking for hot reload and checks (2220's `Hot<T>`, 2516's
  `bundle::recording`): they see `versions` through the shared helper,
  and this ticket adds no tracking of its own.
- Merging 2229's `.meta` settings across layers: 2229 leaves it out
  too, until a game asks.

## Implementation steps

1. Read ADR-0004 §1, ADR-0019 §5, `crates/assets/src/bundle.rs`
   (`find`, `Bundle::read`, `Bundle::bytes`, `test_part`, `watch`) with
   its tests, and `Formats::find`, `Formats::load` and `Formats::read`
   in `crates/assets/src/format.rs`. Check whether 2207 and 2516 are in
   `tickets/done/`.
2. In `bundle.rs`, beside `find`, add a private `copies(layers, path)`
   iterator over `(&Layer, &'static [u8])` for each layer that has the
   path, in order. Make `find` its first item.
3. Move what `Bundle::bytes` tells of a read (the watcher call, and
   2516's recording if it is done) into a private helper that takes
   `(path, found)`, and call it from `bytes`.
4. Add `LayerCopy` and `Bundle::versions` on `copies(self.read(),
   path)`, calling the helper once. Rebuild `test_part` on `copies`.
5. In `format.rs`, add `Formats::load_versions`, decoding each copy as
   `Formats::read` decodes one file but with the copy's shown name.
6. Write the tests below, the module comment and the `docs/formats.md`
   bullet. Run the gates, including the `no_std` build of
   `shuyi-assets`.

## Acceptance criteria

- [ ] `versions` returns every layer's copy in read order over directory,
      table, private and test layers; its first item agrees with `bytes`
      and `display_path` (unit tests).
- [ ] Inside `story_only`, `versions` has no test layer's copy (unit
      test).
- [ ] A path no layer has gives an empty list, and the watcher is told
      `(path, false)`; a path two layers have tells it `(path, true)`
      once (unit test, in the one test that sets the watcher).
- [ ] `test_part`'s existing test passes unchanged.
- [ ] `load_versions` decodes each copy through the registry; a broken
      copy in an upper layer is reported under that layer's name, errors
      from two broken copies are both reported, and a missing id gives
      `Formats::find`'s error (unit tests).
- [ ] `load_versions` reads a RON copy laid over a pack layer and the
      pack's baked copy under it, each through the registry, and the
      baked copy equals the table read from its source (a test in
      `crates/assets/src/pack/tests.rs`).
- [ ] If 2207 is done: a run-time layer's copy comes before an embedded
      layer's copy of the same path (unit test).
- [ ] `shuyi-assets` builds without `std`
      (`cargo build -p shuyi-assets --no-default-features --target
      thumbv7em-none-eabihf --locked`).
- [ ] The `docs/formats.md` bullet and the module comment say the order,
      and that the merge rules are the game's.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`bundle.rs`): `versions` over the existing fixtures
  (`PRIVATE`, `PUBLIC`, `PRIVATE_TEST`, `TEST`, the `OVER` table):
  `data/a.ron` under `Bundle::new(vec![Layer::table("over", OVER),
  PRIVATE, PUBLIC])` is `over`, `bought`, `base` in that order, and
  `data/b.ron` in `with_tests()` is `base`, then `tests`; `story_only`;
  a missing path; `LayerCopy::text` on a binary file (`BINARY`) is
  `None`; the watcher's calls added to
  `the_watcher_is_told_of_every_file_read`, the only test that sets it.
- Unit (`format/tests.rs`, with its `Word` kind and `words()`
  registry): `load_versions::<Word>` over two table layers of their own
  (`mod` over `base`), each with a `words/a.low`: each copy decoded, in
  order; a `no-lower` copy in `mod` named `mod/words/a.low`; two broken
  copies both reported; the id `words/a` given without its extension; a
  missing id.
- Property (`bundle.rs`, proptest): for any bundle of up to four table
  layers (random sorted paths, leaked as `Pack::into_layer` leaks
  them), some marked test, and any path, inside and outside
  `story_only`: the first of `versions(path)` equals `bytes(path)` and
  `display_path(path)`, `versions(path)` is empty exactly when
  `bytes(path)` is `None`, and its length is the number of read layers
  whose table holds the path.
- Integration (`crates/assets/src/pack/tests.rs`, reusing its `baked()`,
  `every_kind()`, `pack_formats()`, `sources()` and the `Table` kind): a
  `Layer::table("mod", ...)` holding `data/table.ron` as RON text other
  than `TABLE`, laid over
  `Pack::open(&baked().bytes, STAMP).unwrap().into_layer("pack")`,
  read through `load_versions::<Table>(.., "data/table")` with
  `pack_formats(&every_kind())`: two copies, shown as
  `mod/data/table.ron` and `pack/data/table.ron` in that order, the two
  tables differ, and the pack's equals
  `formats().load::<Table>(sources(), "data/table")`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, the game PR that moves the pin
for game#1013, notes for Nick.)*
