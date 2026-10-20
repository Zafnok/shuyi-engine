# ADR-0023: The pack: one indexed file of baked assets, a bundle layer, proven equal to its sources

- **Status:** Accepted
- **Date:** 2026-10-20
- **Related tickets:** 2204 (the pack); 3005 and 3010 (the Playdate,
  which loads only packs); 3001 and 3007 (targets, and a bake per
  target); 2207 (bundle layers from outside the binary)
- **Extends:** ADR-0007 §4 (text stays the source; optimized formats are
  derived), ADR-0019 (the format registry)

## Context

ADR-0007 §4 plans "a packed, compressed bundle for shipping" made by
`shuyi bake`, with two rules: the asset read from the baked file equals
the one read from its source, and either one loads. Ticket 2204 makes the
first optimized format. The facts it has to fit:

- The game embeds its files (`include_dir!`, game ADR-0004) and parses
  text at start-up. Its loaders ask the registry for an asset by id
  (ADR-0019 §3), or will.
- The Playdate loads only packs (ADR-0008 §5). The reader must build
  without `std`, and `shuyi-assets` already does (3005's `no-std` CI job).
- Some files are read raw by something other than their kind's format:
  the platform decodes a PNG and plays an OGG, and the font atlas's format
  reads the PNG beside it.
- Packs may come from mods and downloads: a decoder never panics
  (ADR-0007 §5).
- The engine's kinds had no binary form. Some had serde derives for RON
  only, with `skip_serializing_if`, which a format that doesn't name its
  fields can't read back.

## Decision

`shuyi_assets::pack` (`crates/assets/src/pack.rs`, `pack/bake.rs`),
documented for users in `docs/formats.md`.

### 1. The file

`.shuyipak`, little-endian:

| Bytes | What |
| ----- | ---- |
| 8 | `SHUYIPAK` |
| 4 | `pack::VERSION` |
| 8 | the checksum of every byte after the header |
| 4 | the index's length |
| n | the index: postcard of `{ info: { stamp, target }, entries }` |
| … | each file's bytes, in path order, one after another |

- An entry is a file's path, the kinds it was baked as (none for a file
  kept as its own bytes), its codec (`Stored` or `Lz4`), its offset, its
  stored size and its size.
- A file is compressed with LZ4's block format (`lz4_flex`) when that
  makes it smaller, and stored as it is otherwise.
- The checksum is FNV-1a's constants over 64-bit words, each step
  followed by `h ^= h >> 32`, then the length. Every step is a bijection
  of the hash, so any one byte changed is caught.

### 2. Baked files keep their source's path

- A **baked file** is `SHUYIBKD` and postcard of a list of `(kind name,
  binary form)`: one section per kind the file was read as (a PNG read as
  an image's size and as its pixels has two).
- It sits at its source's path (`data/palette.ron`). `Packed<K>`, a
  `Format` of kind `K`, knows it by its magic bytes and claims no
  extension, so lookup (ADR-0019 §3) finds it first and the code that
  asks for the asset doesn't change.
- A file not baked is kept as its own bytes, so the platform still gets
  its PNG files and sounds.

### 3. A pack is a bundle layer

- `Pack::open(bytes, stamp)` checks the header, the version, the
  checksum, the stamp and every entry, then unpacks.
- A stored file borrows the pack's bytes (`Pack<'a>`, `Cow`), so a pack
  embedded with `include_bytes!` costs no copy. A compressed one is
  unpacked into memory of its own.
- `Pack<'static>::into_layer(name)` is a `Layer` like any other: a game
  puts it under its loose files, or alone. `into_owned` serves a pack read
  at run time.

### 4. What a game bakes

- `Bake::new(stamp)`, then `.asset::<K>(id)` and `.under::<K>(dir)`: the
  same two ways a loader asks for assets (`Formats::load`,
  `Formats::load_under`).
- `Bake::register` adds `Packed<K>` for each kind to the game's registry.
- `Bake::bake(formats, bundle, target)` refuses a source with errors and
  returns every one.
- `Bake::check(formats, source, pack)` is the proof. Every planned asset
  must read equal from both bundles (`K: PartialEq`). Every other file
  must be byte-equal. No file may be missing or extra.
- `Bake::load` reads every planned asset, for timing.
- A kind is `Bakeable` when it is `Serialize + DeserializeOwned +
  PartialEq + Debug`. Two kinds of one name can't be in one bake: a pack
  tells kinds by name.
- Bake only kinds whose files nothing else reads raw.

### 5. Versions: refused, never migrated

- `pack::VERSION` covers the container and the binary form of the
  engine's own kinds. A test pins the binary form of every engine kind
  (an `insta` snapshot of each fixture's postcard bytes). A change to one
  fails it: make `VERSION` the next number.
- The game's own kinds are covered by its **stamp**. `Pack::open` refuses
  another stamp.
- Either way, the message says to bake again.

### 6. The engine's kinds get a binary form

- Every engine kind derives `Serialize` and `Deserialize`, with its
  nested types: `ImageInfo`, `Image`, `Sound`, `MusicTrack`,
  `AudioManifest`, `CreditsFile`, `FontAtlasDef`, `PaletteDef`,
  `Lighting`, `Names`, `DialogueFile<D>`, `GridMap<T, F>`,
  `KeymapDef<A>`, `CinematicFile<S, P>`, `LangPack` and `TargetProfile`.
- `Layout` and `Button` are serialized by their names (a button is read
  by its name in a target profile, too).
- A map's look is written whole (`grid_map::look_in_full`); its RON form
  leaves out the default light.
- `CinematicFile` gains `PartialEq`.
- `TerrainTiles` holds a `&'static str` image id and isn't bakeable. No
  engine format reads it.

### 7. The tools

- `shuyi bake [--target <id>] [--out <file>]` is a contract operation
  (ADR-0022), contract version 3. It writes
  `target/shuyi/<target or game>.shuyipak` and says what went in, the
  sizes, the bake time, and how long loading takes from the sources and
  from the pack.
- `shuyi check --baked` bakes in memory and runs `Bake::check`.
- A game gives `GameTools::bake(target) -> GameBake { bake, bundle,
  formats }`. The target is passed through as it is: the game picks
  layers and kinds by it. Ticket 3001 gives targets profiles, and 3007
  uses the hook for one-bit images.

## Consequences

- A game that ships a pack loads faster and ships less. Measured in
  `docs/formats.md`:
  - Hello, Grid loads 2.5–4× faster from its pack.
  - The game's public assets load about 25% faster and the pack is 74%
    of their size. Their PNG and OGG files don't compress, and they dominate.
- Every kind a game bakes needs serde both ways and `PartialEq`, its own
  types too (`Directive`s, map features, actions, shots).
- A baked asset keeps what it held when baked, such as the name its
  scenes' errors show (`base/dialogue/ch01.dlg`). That is the source's
  name, which is the file to edit.
- `Pack::open` unpacks every compressed file at once. Streaming is out of
  scope (ticket 2204). A ticket comes if measurements show a need.
- LZ4's output differs between a 64-bit and a 32-bit baker, so the pack's
  bytes do too. The assets read from them don't. Bake on the developer's
  machine, as `shuyi bake` does.
- A change to an engine kind's fields is now a pack version bump. The
  pinned snapshot says so.

## Alternatives considered

- **bincode** — would work too (version 2 builds without `std`).
  postcard is made for small targets, writes varints by default, and is
  the one the ticket names.
- **zstd or deflate** — smaller packs, but zstd is C, and pure-Rust
  deflate decodes slower than LZ4. Load time is the point,
  and the big files (PNG, OGG) are compressed already.
- **A separate namespace for baked assets** (`@baked/palette.ron`) with
  the sources kept — a pack twice as big, and every loader would ask for
  two paths.
- **Migrating old packs** — every old binary form would have to be kept
  readable. Baking again is one command, and the sources are the truth.
- **A per-kind schema hash, computed by tracing serde** — robust, but
  more machinery (or `serde-reflection`) than a pinned snapshot and a
  stamp. Worth revisiting if stamps prove easy to forget.
- **A cryptographic hash** — this checksum guards against cut-short and
  changed files, not attackers. Signing packs would be a ticket of its
  own, and the ticket rules out encryption.
