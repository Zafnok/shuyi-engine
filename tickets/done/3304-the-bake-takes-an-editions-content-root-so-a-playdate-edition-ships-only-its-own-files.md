---
id: "3304"
title: "The bake takes an edition's content root, so a Playdate edition ships only its own files"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-10
---

# 3304 — The bake takes an edition's content root

## Context

3082 added editions to `shuyi.ron`
([ADR-0100](../../docs/adr/0100-editions-named-builds-in-shuyi-ron-a-content-root-handed-to-the-build-and-default-features-always-said.md)):
`export <target> --edition <name>` builds a public demo from its own folder
of files (`content`), handed to the desktop and web builds as
`SHUYI_CONTENT`, which the template's `build.rs` points `include_dir!` at.

The Playdate can't do that yet. Its pack is baked by `contract::bake` through
the game's tools (`GameTools::bake` on the tool program's own bundle,
`crates/cli/src/export/playdate.rs`, `prepare`), and the tool program embeds
`files/` whatever the edition says. So `playdate.rs::edition_fits` refuses an
edition with a `content` root (and one with `default_features: true`, which
stays refused: the library never keeps them). A Playdate demo today can only
leave out code, by features, not files.

## Nick input

None.

## Scope

**In:**
- The Playdate export of an edition with a `content` root bakes the pack from
  that folder: the files the pack holds are the edition's folder's, never
  `files/`.
- The way the bake is told the folder is one a later packed desktop or web
  export (3300) can use too. **Either order with 3300:** if 3300 is done, its
  release bake of an edition takes the same folder through this ticket's way;
  if not, nothing to do here for it (3300 uses this ticket's way when it
  lands).
- `edition_fits` stops refusing `content` on the Playdate; it keeps refusing
  `default_features: true`.
- `exposure-report.json`'s `inputs.content_root` names the folder the pack was
  baked from, as it names the program's on desktop and web.

**Out (do not do):**
- Changing the pack's format.
- Editions per target (ADR-0100, *Alternatives*).

## Implementation steps

1. Read `contract::bake` (`prepared`, `GameBake`) and how the tool program
   gets its bundle (`BasicTools`, the template's `examples/tools.rs`). Decide
   between (a) building the tool program with `SHUYI_CONTENT` set, as the
   game's build is, and (b) the bake laying the edition's folder over the
   bundle's `files` layer as `run --watch` lays `assets` over it (the watch's
   layer code in `crates/cli/src/watch.rs`). (b) must replace the layer, not
   lay over it, or the full game's files stay in the pack.
2. Hand the choice to `PlaydateExporter::prepare` for `cx.edition`'s
   `content()`; write an ADR amending ADR-0100 §7.
3. Update `docs/targets/playdate.md` (the paragraph on `--edition`), the
   book's *A public demo* and the `shuyi-ship` skill.

## Acceptance criteria

- [x] A Playdate export of an edition whose `content` folder lacks a file of
      `files/` has no pack entry of that file, and its canaries pass.
- [x] The same export with `content` naming `files/` holds it.
- [x] An edition with `default_features: true` is still refused on the
      Playdate.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the bake input for an edition with and without a content root.
- Integration: a stand-in game's bake through the Playdate exporter's
  `prepare` with two content roots, its pack's entries read back (no SDK:
  stop before `pdc`, as `export/playdate/tests.rs` does).

## Completion notes

Option (a): the game's tool program is built with the edition's folder as
`SHUYI_CONTENT` and bakes the pack
([ADR-0101](../../docs/adr/0101-an-editions-pack-is-baked-by-the-game-s-tool-program-built-with-its-content-root.md),
amending ADR-0100 §7). (b) was turned down because the engine would need its
own rule for which layer of a game's bundle the folder replaces, which could
disagree with the game's build script and leave the full game's files in the
demo's pack.

- `BakeInput::content` (`bake --content <folder>`): with a folder, `bake` runs
  the game's tool program (`shuyi.ron`'s `game`) with `cargo run` in the Cargo
  profile `shuyi-content` (declared with `--config`, as `bench` declares its
  own), the folder as `SHUYI_CONTENT`, and answers its JSON answer, a limit
  naming the folder added. Its own profile because Windows can't relink the
  tool program running the export. `contract::bake::content_folder` checks
  the folder. Contract 44.
- `playdate::bake_input(cx)` is what `prepare` bakes, the edition's content
  root in it; `edition_fits` refuses only `default_features: true`.
  `exposure-report.json`'s `inputs.content_root` already named the edition's
  folder on every target.
- Tests: the bake's arguments, its refusals (not a folder `SH3004`, no `game`
  part `SH1002`), the Playdate's bake input with and without a content root,
  and a stand-in game whose tool program (on `shuyi-assets` alone, from the
  engine's lock file, offline) bakes two content roots through
  `PlaydateExporter::prepare`; the packs are read back by the disclosure
  gate: the demo's holds `maps/first.ron` alone and passes its canaries, the
  full game's holds the rest and fails the demo's part.
- Docs: `docs/contract.md`, `docs/targets/playdate.md`, the book's *A public
  demo*, the `shuyi-ship` skill.
- Follow-up: 3306, the Playdate's sounds and music, still baked from the
  running tools (`GameTools::audio`), come from the edition's folder too.
- For Nick: the first Playdate export of a demo builds the game's tools a
  second time, which takes some minutes once.

No break: `BakeInput` is `#[non_exhaustive]`; the new items are additions.
