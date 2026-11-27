# ADR-0161: Domain preludes beside the short path's, and an import check on the code a game learns from

- **Status:** Accepted
- **Date:** 2026-11-27
- **Related tickets:** 3438 (this), 3413 (doc aliases and the plain-text
  API), 3442 (one spelling of identifiers), 3520 (the genre kits behind
  features)
- **Amends:** ADR-0036 §6 (the prelude in `shuyi-platform`: "nothing
  else")

## Context

An agent writes a game from memory and from the code it is shown: the
template, `examples/`, the book's snippets (`site/snippets/`) and the
skill pack's worked examples (`skills/*/example/`). The blind-spot audit
of 2026-11-23 (*Agent experience*, candidate 63) found that it guesses
deep paths, and each miss costs a Rust compile round. On 2026-11-27 those
files named about 430 distinct engine paths from about 74 modules
(`shuyi::ui::frame::glyph_buffer`, `shuyi::core::bots::playtest`,
`shuyi_cli::contract::playtest`, …). `shuyi::prelude`
(`shuyi_platform::prelude`, ADR-0036 §6) held about 30 names, and a
rules crate, which depends on `shuyi-core` alone, had no prelude: it
named the crate's root. A path in the examples is copied, so a deep one
taught a deep one.

## Decision

1. **Domain preludes, re-exports only.** Beside `shuyi::prelude`, which
   keeps every name it had:
   - `shuyi_core::prelude` (a game's crate writes
     `shuyi::core::prelude`): the RNG, `TurnRules`, `TickSim` and the
     step's types with the `tick` module, the timers, `Pos`, `Dir`, the
     core's `Grid`, paths, the ease and tween types, a state's hash, a
     save's header and the bots that play a turn game (`Bot`,
     `RandomBot`, `Rollout`, `play`);
   - `shuyi::prelude::grid`: the grid kit (`shuyi-ui`'s `grid`: the
     camera, cursor, skins, scene and painters), the map file
     (`GridMap`, `MapText`) and its lights;
   - `shuyi::prelude::text`: styled text and its roles, the font's atlas
     and widths, a message, the languages;
   - `shuyi::prelude::audio`: the audio manifest and its cues, a screen's
     requests and the sound groups;
   - `shuyi::prelude::test`: the Harness (with `harness`), its session
     files, the playtest runner's checks and the autobalance search;
   - `shuyi_cli::prelude`: the game's tools (`GameTools`, `BasicTools`,
     the plans), `main_with`, the project, the contract's operations a
     game's tests call with their inputs and outputs, and `frame-png`'s
     scripts and shots.

   The four `shuyi::prelude::<domain>` modules live in
   `shuyi_platform::prelude`, where each feature (`harness`) is known,
   and reach `shuyi::prelude` through its glob re-export. Each holds what
   the template, the examples, the snippets and the skills' examples
   named on 2026-11-27, and no more: a name joins a prelude when such
   code names it. A prelude is not a second home for a module: an item
   has one prelude, and the bots are the core's, since a shipped game's
   enemy plays with them.
2. **The import check.** `cargo xtask check-imports`
   (`crates/xtask/src/imports.rs`), a fast gate (`cargo xtask gates
   --fast`, the pre-push hook) and a step of CI's `layers` job, reads each
   `.rs` file under `templates/`, `examples/`, `site/snippets/` and
   `skills/`: every leaf of its `use` trees and every path in its code
   that starts with an engine crate (a crate under `crates/`, by its
   package's name), with `shuyi check-text`'s lexer, so a comment or a
   string is not read. A path passes under a prelude, or under an entry of
   `tools/imports.ron`'s `allowed` in the files or folders the entry
   names, each entry with its `why`. The check fails, naming the file and
   the line, on any other engine path, and fails an entry nothing names,
   one with no reason, one listed twice, or one whose files are gone.
3. **What stays a module's path.** A module a game turns to for one job
   (dialogue, cinematics, menus, settings, storage, the debug menu), the
   backend's start (`shuyi::app`), the assets' files read at loading (the
   palette, the keymap, the formats' registry) and the tools' own modules
   (`frame_png`'s functions, the contract's other modules) are named by
   their module, each with its entry in `tools/imports.ron`. A domain
   prelude for one of them is its own ticket, when games on that domain
   name it.
4. **ADR-0036 §6** is amended: `shuyi_platform::prelude` keeps its
   names, and holds the four domain modules besides.

## Consequences

- An agent's first guess, `use shuyi::prelude::grid::*;`, `use
  shuyi_core::prelude::*;` or `use shuyi_cli::prelude::*;`, compiles, and
  every example it copies teaches one of them or a path whose reason it
  can read.
- A new deep import in that code is a visible choice in `tools/imports.ron`,
  made in the PR that adds it.
- Two `Grid`s meet in a file that globs both the core's prelude and the
  grid kit's (the rules' board and the screen's layout): the file names
  one by its path. The preludes say so.
- A glob of `shuyi::prelude` brings the four module names (`grid`,
  `text`, `audio`, `test`) into scope; a game's own module of that name
  shadows the glob, as Rust does for every glob.
- The game (`Zafnok/visions-of-shuyi`) is outside the check: its own code
  keeps its paths, all of which still resolve.

## Alternatives considered

- **One big prelude** (everything in `shuyi::prelude`): about 430 names in
  one glob collide (`Grid`, `Step`, `Layer`, `Look`) and make every
  screen's scope the whole engine.
- **A prelude per crate** (`shuyi_ui::prelude`, `shuyi_assets::prelude`):
  the crates are the engine's layers, not a game's domains; a screen that
  shows a map would still import from three.
- **Allowed paths with no files** (a list valid everywhere): a module
  allowed for the dialogue skill's example would pass in the template too,
  and the list would stop saying where a deep path is taught.
- **A lint in `shuyi lint`** for games: the game and every game made from
  the template would fail on code they own; the check is about what this
  repository teaches, so it is this repository's gate.
