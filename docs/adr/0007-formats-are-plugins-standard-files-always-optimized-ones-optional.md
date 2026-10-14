# ADR-0007: Formats are plugins: standard files always, optimized ones optional, text stays the source

- **Status:** Accepted
- **Date:** 2026-10-14
- **Related tickets:** 2201 (the registry), 2202 (dialogue formats), 2203
  (image and audio formats), 2204 (an optimized pack), 2901 (research:
  better images), 1123 and 1124 (Aseprite, Tiled, LDtk)
- **Extends:** ADR-0004 (standard files in, the engine's own data out)

## Context

Nick, 2026-10-14 (`docs/decisions.md`): "making certain parts modular,
such as - if we want to introduce a highly optimized file format, this
should be easier than ever. But also maintain broad compatibility with
normal, widely used file formats. I'm thinking like our dialogue scripts,
asset files, etc. If you can find a way to make png/jpegs better using AI
black magic then we can ship a separate repo for that, and support their
loading too."

What the game's code does today (game `docs/engine/boundary.md`):

- Each loader is one function that reads one format from the bundle, with
  a pure parser beside it (`load()` plus `from_source(file, text)`).
- An asset kind and its file format are the same thing: a portrait *is*
  a PNG plus a RON sidecar, and a scene *is* a `.dlg` file.
- Adding a second format for a kind means writing a second loader and
  changing every caller.

ADR-0004 already plans Tiled, LDtk, Aseprite and TexturePacker importers.
This ADR gives them, and every later format, one shape.

## Decision

### 1. Asset kinds are separate from file formats

The engine defines the asset kinds as plain types in `shuyi-assets`:

- `Image`, `Atlas` and `AnimationSet`;
- `TileMap` and `Tileset`;
- `Font`;
- `Sound` and `MusicTrack`;
- `DialogueScene`;
- `LanguagePack`;
- `Cinematic`;
- `Keymap`.

A game adds its own kinds (its data tables). Code that uses an asset asks
for the kind by id and never sees which file it came from.

### 2. A format is a plugin

```rust
pub trait Format: Send + Sync + 'static {
    type Asset;
    /// Stable id, e.g. "png", "tiled-tmx", "yarn".
    fn id(&self) -> &'static str;
    /// File extensions this format claims, lowercase, without the dot.
    fn extensions(&self) -> &'static [&'static str];
    /// True if these bytes look like this format (magic bytes); `false` when
    /// the format has none.
    fn sniff(&self, bytes: &[u8]) -> bool { let _ = bytes; false }
    /// Every problem found, each with file and place; never a panic.
    fn decode(&self, file: &AssetPath, bytes: &[u8], cx: &DecodeCx)
        -> Result<Self::Asset, Vec<ContentError>>;
}
```

- **Writing back is optional.** A format that can also write implements
  `Encode` (`encode(&Asset) -> Vec<u8>`), which tools and the optimized
  pack (§4) use.
- **The registry.** `Formats` holds the formats per asset kind. The game,
  or a plugin crate, registers more at start-up.
- **Lookup order.** For each file the registry tries the formats that
  sniff true, then the ones that claim its extension. Two formats
  claiming the same extension is an error at registration, unless the
  later one says it replaces the earlier.
- **Plugins are crates.** A format outside the engine is a crate that
  implements `Format` and that the game depends on (behind a feature if
  it is optional). There is no runtime loading of plugin libraries.

### 3. Standard formats always load

`shuyi-assets` registers these built-in formats by default:

- PNG;
- our RON sidecars;
- `.dlg`, `.map`, language packs, cinematics and keymaps;
- WAV and OGG;
- and, as their tickets land, Aseprite JSON, TexturePacker JSON, Tiled
  and LDtk.

More formats come as plugins in the engine's own crates, each behind a
feature:

- JPEG, WebP, QOI and JPEG XL (2203);
- Yarn Spinner and Ink (2202).

A game never has to convert a common file before using it.

### 4. Text stays the source; optimized formats are derived

- **Sources stay text.** Authored content (data, maps, dialogue,
  cinematics) stays in text formats that people, agents and git read and
  diff.
- **Optimized formats are derived by a tool.** `shuyi bake` turns
  sources into them: a packed, compressed bundle for shipping (2204), or
  a smaller image format. Two rules hold:
  - **Same asset.** Decoding the baked file gives an asset equal to
    decoding its source. A test in the format's crate checks this on
    every fixture, and `shuyi check --baked` checks a whole game.
  - **Either one loads.** The runtime reads the baked file or the
    source. A game may ship either.
- **Lossy formats.** Equality uses the format's own tolerance, stated in
  its docs (for example, the maximum per-pixel difference). The lossy
  format is never the source.

### 5. Every decoder is safe and deterministic

- **Untrusted input.** Bytes may come from mods or downloads. A decoder
  never panics, has size limits, and has a fuzz test (`cargo fuzz`
  target or a property test over random bytes).
- **Deterministic.** The same bytes give the same asset on every
  platform, because content hashes are part of replays and the network
  handshake (ADR-0006 §2). A lossy decoder that uses floats must prove
  this with a fixture whose decoded pixels are checked in.
- **Licensed.** Every format crate passes ADR-0002 §8 with
  `default-features = false`. It is pure Rust, or at least builds for
  the web build (`wasm32-unknown-unknown` without `wasm-bindgen`) and on
  Windows with the GNU toolchain.

### 6. "Better images" is a separate project

Research ticket 2901 studies ways to make the images a game ships
smaller, faster to load, or better looking:

- learned (neural) image compression;
- JPEG XL;
- GPU texture compression;
- upscaling for large screens.

If one is worth shipping, it lives in its own repository (Nick's ask) as
a `Format` plugin plus a `shuyi bake` step. The engine's only part is the
registry it plugs into. A plugin from that repository must still meet §5:
no decoder whose output differs between machines, and no large model in
the runtime unless the research shows a game is better for it.

## Consequences

- **Adding a format touches nothing else.** A new format is one crate
  and one `register` call. Nothing that uses assets changes.
- **Some code moves.** The carve (game 1103–1105) already splits every
  loader into reading and parsing. 2201 turns the parsers into `Format`s
  without changing what any of them accepts.
- **Optimized formats can't drift from their sources.** Every format
  needs a fuzz test, and a lossy one needs its tolerance written down.
- **Lookup has rules.** Registration order and extension conflicts are
  part of the API, documented and tested.

## Alternatives considered

- **One format per kind; convert everything else in a build step** — the
  game does this for bought art (game ADR-0040). As the general rule,
  modders and hot reload would need the build tool, and copies drift.
- **Bevy-style asset loaders with an async asset server** — the same
  idea, but tied to Bevy's ECS and async runtime. Our version is
  synchronous, works from bytes, and stays testable without a window.
- **Dynamic plugin libraries loaded at run time** — Rust has no stable
  ABI, and the web build can't load them.
- **Make an optimized format the source of truth** — agents and git
  can't read it, and the text-first edge is lost.
