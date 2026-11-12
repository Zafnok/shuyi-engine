# ADR-0110: Desktop and web exports ship a baked pack beside the program

- **Status:** Accepted
- **Date:** 2026-11-11
- **Related tickets:** 3300; 2204 (the pack); 3081 (the disclosure gate);
  3304 (an edition's pack); 2215 (the embedded files' table, parked);
  2917 (the research on what a pack protects)
- **Extends:** ADR-0023 (packs are the shipping form on every target; the
  embedded folder is the development form), ADR-0058 (the build-path search
  reaches into a pack), ADR-0101 §4 (the packed export bakes an edition's
  pack through `bake --content`)

## Context

A game made from the template embeds its whole `files/` folder with
`include_dir!`, so a desktop or web export carries every RON file as written,
with its path: `strings` on the program lists every table, map and line of
text. Only the Playdate export baked a pack (ADR-0023, ADR-0101), which the
device reads from its `.pdx`.

ADR-0023 measured a pack loading 2.5× (native) to 4× (web) faster than the
embedded text, and smaller. Research ticket 2917 says a pack's obscurity buys
hours at most and encryption buys nothing a public engine can keep: this is
about tidiness and speed, not protection.

The forces:

- A developer never needs a pack to play: `cargo run` and `shuyi run` keep
  the embedded files, which `--watch` and hot reload work on.
- The web has no file beside the program; its page fetches files, and
  macroquad's own `load_file` is asynchronous, while a game builds its
  bundle before its first frame, synchronously, on every target.
- The template's `UiAction` has no serde form, so the keymap of a game on
  the short path can't be baked without a bound every such game would have
  to meet.
- An edition has its own content root (ADR-0100), which only the game's tool
  program built with it can bake (ADR-0101).

## Decision

1. **A `packed` feature.** A game's package with a `packed` feature reads its
   files from a pack beside the program in place of embedding them: the
   template's game crate (`templates/shuyi-default/base/crates/game`) and
   Hello, Grid have one. Built without it, a game embeds its files as before.
2. **The release export turns it on.** `export::ExportCx::features_on` adds
   `packed` (`export::PACKED`) to a `--release` build of a `desktop` or `web`
   profile's target whose package has the feature; a debug export does only
   when its features name it. A package without the feature exports as before.
3. **Baked, then proven, before the build.** `ExportCx::bake_pack`, the
   desktop and web exporters' `prepare`, runs `contract::bake` for the
   target's profile into `<export folder>/<package>.pack` (with the edition's
   content root, ADR-0101), then `bake --check` (`BakeInput::check`, new): the
   pack file is read back and `Bake::check` proves it against the sources,
   through the content root's tool program when there is one. The export fails
   with the first difference and their count.
4. **Where it goes.** `<package>.pack`, the package's name, which the game's
   library knows as `CARGO_PKG_NAME`: beside the program on Windows and
   Linux, in `Contents/Resources` in a `.app` (before it is signed), and
   beside `index.html` on the web. `ExportOutput::pack` names it with its
   size.
5. **How the game reads it.** `shuyi_app::pack_beside_program(name, stamp)`
   reads `<name>.pack` from the program's folder (and a `.app`'s
   `Resources`), leaks its bytes for the run as embedded files are, and
   opens it with the stamp; a missing, unreadable or refused pack is a
   `PackFileError` that says to bake it again. On the web the page's loader
   fetches the pack before it starts the game (`crates/cli/web/pack-file.js`,
   `index.html`'s `%PACK%`, written by `shuyi web` from `web::Options::pack`)
   and hands its bytes over through the plugin, so the call is synchronous on
   every target.
6. **The stamp** of a short-path game's pack is `AppInfo::pack_stamp`, its id
   and version, which `BasicGame::made` already used as its build's word.
7. **What a short-path game bakes.** `BasicData::bake(stamp)`: the English,
   the palette and the font atlas; `BasicData::formats` registers their
   `Packed` formats, so its one loader reads either. The keymap, the pictures
   and the game's own part go in as their own bytes. `BasicGame::bake` and
   `BasicTools::bake` give it to the tools; the grid stage's tools pass it on.
8. **A pack that couldn't be read** is the game's one problem, shown in place
   of the game: `BasicGame::files_unreadable(why)` for a short-path game, and
   Hello, Grid's `pack_problem` for its own.
9. **Searched and declared.** The release export searches the pack for the
   build machine's paths (`export::paths::check_pack`): its own bytes, each
   path of its index and each file it unpacks to. The disclosure gate lists
   it as one `Pack` entry and reads each of its files, as it does the
   Playdate's, and also scans each entry's path for the canaries, naming the
   pack and the entry.

## Consequences

- A release program holds no text of the game's files and no path of them
  (CI's export check greps the program for a line of Hello, Grid's
  `keymap.ron`), and loads its data from baked forms.
- A shipped game is two files at least: a player who copies only the program
  sees "the game's files, `<name>.pack`, aren't beside the program".
- A game on the short path ships its keymap as RON text in the pack. Baking it
  needs its action set to have a serde form; a game whose actions do bakes it
  with its own `GameTools::bake`, as Hello, Grid does.
- `cargo shuyi export` on a game of its own shape needs a `packed` feature,
  a `GameTools::bake`, and code that opens `pack_beside_program` when the
  feature is on; without the feature its exports embed the files, as before.
- A source changed during the build is caught only on an edition's export,
  whose check runs its own tool program again; the export's own tool program
  embedded its files when it was built. The check still proves the file that
  ships against the sources the bake read.
- The template's unpacked path keeps `include_dir!` until ticket 2215.
- Contract 48: `bake`'s `check` and `export`'s `pack`.

## Alternatives considered

- **Load the pack asynchronously in the Rust program on the web** (macroquad's
  `load_file` inside the main future) — every game would need an async start,
  and `BasicGame` is built before the window. Fetching in the page keeps one
  synchronous call on every target.
- **A fixed name, `game.pack`, as the Playdate's** — robust to a renamed
  program, but a folder of two editions' programs could not hold both packs.
  The package's name is known to the game's library at compile time and to
  the exporter.
- **Embed the pack in the program (`include_bytes!`)** — one file to ship,
  but the build would need the pack first, and its compressed files would hide
  nothing the disclosure gate doesn't already read. The ticket asks for a pack
  beside the program, which a player or a modder can also replace.
- **Turn `packed` on in every export** — a debug export is for developers,
  and the feature changes how the game reads its files; only the shipping form
  takes it by default.
- **Bake the short path's keymap by adding a serde bound to `BasicData`** — a
  break for every game whose action set has none, for one small file.
