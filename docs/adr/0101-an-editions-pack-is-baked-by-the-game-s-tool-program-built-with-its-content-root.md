# ADR-0101: An edition's pack is baked by the game's tool program built with its content root

- **Status:** Accepted; extended by ADR-0109 (§4: the packed desktop and web exports bake through it)
- **Date:** 2026-11-10
- **Related tickets:** 3304, 3082, 3300

## Context

ADR-0100 hands an edition's content root to the desktop and web builds as
`SHUYI_CONTENT`, and the game's build script decides what it replaces (the
template points `include_dir!` at it). The Playdate is different: its pack is
baked by `contract::bake` from the bundle the game's tools return
(`GameTools::bake`), in the tool program that runs the export, which embedded
`files/` when it was built. So §7 of ADR-0100 refused a Playdate edition with
a `content` root.

Two ways were open (ticket 3304):

- (a) build the game's tool program with `SHUYI_CONTENT` set, as the game's
  own build is, and let it bake;
- (b) lay the edition's folder over the bundle's files layer, as `run --watch`
  lays its folder, replacing that layer.

(b) needs the engine to know which of the bundle's layers the folder
replaces. A bundle is the game's: the template has one layer, `files`; Hello,
Grid has `files` and `files@playdate`; a game may embed bought art or
per-target layers of its own. On desktop the game's build script decides what
`SHUYI_CONTENT` replaces; under (b) the engine would decide again for the
pack, by a second rule a game would have to keep in step with the first, and
a mismatch leaves the full game's files in the demo's pack.

## Decision

1. **`bake` takes a content root.** `BakeInput::content` (`bake --content
   <folder>`) is a folder of the project, from its root with `/`, checked as
   ADR-0100's `content` is (`contract::bake::content_folder`). With one,
   `bake` doesn't bake in its own process: it runs the game's tool program
   (`shuyi.ron`'s `game`) with `cargo run`, built with the folder's absolute
   path as `SHUYI_CONTENT`, on the same input less the folder and with
   `--json`, and answers that program's answer, a limit naming the folder
   added. A project with no `game` part is refused (`SH1002`), a folder that
   isn't one (`SH3004`), and a program that gives no answer (`SH3005`).
2. **A profile of its own.** That build is in the Cargo profile
   `shuyi-content` (`contract::bake::CONTENT_PROFILE`), declared on the
   command line (`--config profile.shuyi-content.inherits="dev"`), as `bench`
   declares its profile. The tool program running the export can't be
   relinked under itself on Windows, and its own build isn't undone for the
   next command; the profile's folder in the target directory keeps the
   content build between exports.
3. **The Playdate takes it.** `PlaydateExporter::prepare` bakes
   `playdate::bake_input(cx)`: the target's profile, the export's folder, and
   the edition's content root if it has one. `edition_fits` refuses only
   `default_features: true` now; this replaces ADR-0100 §7's refusal of
   `content`. `exposure-report.json`'s `inputs.content_root` already names
   the edition's folder, the folder the pack was baked from.
4. **The way for packed exports.** A packed desktop or web export (ticket
   3300) bakes an edition's pack through the same field.

## Consequences

- The pack holds what the program would embed: one rule, the game's build
  script, decides what a content root replaces, on every target.
- The first edition export with a content root builds the tool program again
  in `shuyi-content`, with every dependency: minutes on a cold cache. Later
  ones rebuild the game's crates only when the folder changes.
- The tool program is built with its own features (`game.features`), not the
  edition's: its bake must take what the folder holds. A file the bake names
  that the demo's folder lacks is a bake error, which the export reports.
- Sounds and music on the Playdate still come from the running tools
  (`GameTools::audio`), not from the edition's folder (ticket 3306).
- `bake`'s input gains a field: contract 44.

## Alternatives considered

- **(b) Replacing a layer of the bundle in the bake:** in process and fast,
  but the engine would need a rule for which layer is the content root's
  (by name, or every layer that isn't a target's or a test's), which can
  disagree with the game's build script and leak the full game's files.
- **A `GameTools` method that bakes from a given folder:** the game says what
  the folder replaces, in process, but a second place to say what the build
  script already says, and every game must write it before any edition of
  it can go to the Playdate.
- **The same target directory and profile as the running tool program:**
  no second build of the dependencies, but on Windows Cargo can't replace
  the program that is running, and the next command would rebuild it back.
