---
id: "3300"
title: "Desktop and web exports ship a baked pack beside the program, not the files folder embedded as text"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3099"]
nick_input: none
completed:
---

# 3300 — Desktop and web exports ship a baked pack

## Context

The default template embeds the whole `files/` folder with `include_dir!`
(`templates/shuyi-default/base/crates/game/src/lib.rs:22`). A desktop or
web export therefore carries every RON file as written, with its original
path: `strings` on the exe or the Wasm lists every rule table, map and
line of text. Only the Playdate export bakes a pack (`crates/cli/src/export/playdate.rs`,
`BAKED`), which the device reads from its `.pdx`
(`crates/playdate/src/backend.rs::read_pack`).

The pack exists since 2204 ([ADR-0023](../../docs/adr/0023-the-pack-one-indexed-file-of-baked-assets-proven-equal-to-its-sources.md)):
a postcard index of paths and kinds, baked postcard sections per kind,
LZ4 where smaller, proven equal to its sources by `Bake::check`, and
measured 2.5× (native) to 4× (web) faster to load. ADR-0023 says "a game
that ships a pack loads faster and ships less". This ticket makes the
desktop and web exports do it.

Research ticket 2917 ([report](../../docs/research/2917-reverse-engineering-angles-not-yet-covered.md),
sections 1 and 3) is why it is in the public-demo-defenses track: the
history of Godot, Unreal, RPG Maker and Ren'Py says a pack's obscurity
buys hours at most and encryption buys nothing a public engine can keep,
so this ticket claims no protection. Its value is tidiness and speed: no
RON text and no paths in the program, baked data instead of parsed text,
and the pack a single file the disclosure gate (3081) already lists.

Related: 2215 (parked) replaces `include_dir` with an in-house build-script
table for the embedded path. **Either order with 2215:** if 2215 is done,
the template's unpacked path uses its table and this ticket touches only
the packed path; if not, the unpacked path keeps `include_dir!` and 2215
replaces it later, the packed path being unaffected either way.

**From 3082 (done):** `shuyi.ron`'s `editions` build a public demo from its
own folder of files (`content`), handed to the build as `SHUYI_CONTENT`, which
the template's `build.rs` points `include_dir!` at
([ADR-0100](../../docs/adr/0100-editions-named-builds-in-shuyi-ron-a-content-root-handed-to-the-build-and-default-features-always-said.md)).
The packed path must bake an edition's folder, not `files/`.

**From 3304 (done):** `BakeInput::content` (`bake --content <folder>`) bakes
a content root by the game's tool program built with it as `SHUYI_CONTENT`, in
the Cargo profile `shuyi-content`
([ADR-0101](../../docs/adr/0101-an-editions-pack-is-baked-by-the-game-s-tool-program-built-with-its-content-root.md)).
Bake an edition's pack through it, as `playdate::bake_input` does for the
Playdate.

## Nick input

None.

## Scope

**In:**
- A `packed` Cargo feature in the template's game crate: with it, the
  game's `files` layer comes from a pack file rather than `FILES`
  (`Layer::dir`). The desktop program reads `<name>.pack` beside itself
  (and the macOS bundle's `Resources/`); the web page fetches it once
  beside `index.html`, through the existing loader, before the first
  frame.
- `shuyi export windows|linux|macos|web` in release bakes the pack with
  `contract::bake` for the target's profile (as the Playdate exporter
  does), builds the game with `packed`, places the pack in the package,
  and runs `Bake::check` against the source files before packaging.
- The pack carries no original paths it does not need: the index keeps
  the path (loaders look files up by it) but the exporter fails if a path
  holds a build-machine prefix (ADR-0058's search, extended to the pack).
- The disclosure gate (3081) lists the pack as one `Shipped` entry and
  also scans its entries' paths and raw (non-baked) bytes for the
  manifest's canaries, since a pack hides nothing from the gate.
- `shuyi run` and `cargo run` stay on the embedded files: a developer
  never needs a pack to play.
- Measurements in `docs/formats.md`: package bytes and load time of
  `hello-grid`, embedded against packed, native and web.
- ADR-0023 gains a paragraph: packs are the shipping form on every
  target; the embedded folder is the development form.

**Out (do not do):**
- Encrypting or signing the pack, or hiding the index. Section 3 of
  2917's report says why; 2603 excludes a client-held key.
- Removing `include_dir` from the template's development path (2215).
- Changing the pack format's `VERSION` unless a path check needs it; if it
  does, a `Breaking:` line and migration block (ADR-0038).
- Playdate, GBA or NDS exporters: unchanged.

## Implementation steps

1. Read `contract::bake`, `Bake`, `Pack::into_layer` and the Playdate
   exporter's bake step; decide where the pack lands per target
   (`<base>/<name>.pack` on Windows and Linux, `Contents/Resources/` on
   macOS, `dist/<name>.pack` on the web) and write it in `ExportReport`.
2. Template: add `packed = []` to `crates/game/Cargo.toml`; in `lib.rs`,
   `game()` builds its `files` layer from `FILES` without the feature and
   from `shuyi::app::pack_beside_program(APP)` with it. Add that helper to
   `shuyi-app` (native: read the file next to `current_exe`; web: the
   loader's fetch, see `crates/cli/web/` and `crates/app/src/backend.rs`
   for where the page hands bytes to Rust). Each is additive public API
   (ADR-0039).
3. Exporters: in `export/desktop.rs` and `export/web.rs`, before the cargo
   build in release, call `bake` as `playdate.rs:823-845` does with the
   target's profile; pass `--features <package>/packed`; copy the pack into
   the package; run `Bake::check`; fail with the first difference.
4. Extend ADR-0058's path search in `export.rs:370-376` to the pack's
   index paths.
5. 3081's gate: a `FileKind::Pack` entry; open it with `Pack::open` and
   scan entry paths and raw bytes for canaries; add the fixture game's
   canary inside a pack to the gate's tests.
6. Measure and write `docs/formats.md`; add the ADR-0023 paragraph; update
   the book's export chapter and the `shuyi-export` skill if one exists
   (the `document` skill's map says where).
7. CI's `export` job exports `hello-grid` packed for the host and the web
   and runs it headless one frame (the web through the existing Playwright
   gate from 2914's PR, if still present, else `check.sh`).

## Acceptance criteria

- [ ] A release desktop or web export of `hello-grid` contains a pack and
      no RON text of `files/` in the program (`strings` finds none of a
      known key from `files/keymap.ron`).
- [ ] The exported game plays the same: the capture plan of
      `hello-grid` (2018) passes on the packed build.
- [ ] `Bake::check` runs in every release export and a changed source file
      after baking fails the export with its path.
- [ ] A canary planted inside the pack fails 3081's gate with the pack's
      path and the entry's path.
- [ ] `docs/formats.md` has the embedded-against-packed measurements.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `pack_beside_program` finds the pack next to the program and
  reports a missing or foreign pack with ADR-0023's "bake it again".
- Property: for the fixture games, `decode(source) == decode(pack)` for
  every baked kind (already `Bake::check`; run it in the exporter's test).
- Integration: the packed export of `hello-grid` for the host and the web
  in CI's `export` job; the disclosure gate over a pack with a canary.

## Completion notes

*(Filled in by the session that completes the ticket.)*
