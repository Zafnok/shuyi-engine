# ADR-0059: A game's identity once, in `shuyi.ron`: written as a Rust constant checked like generated files, and the storage name following the id

- **Status:** Accepted; extends ADR-0036 and ADR-0039
- **Date:** 2026-10-30
- **Related tickets:** 3048 (this), 3072 (the icon), 3049 (the identity
  in every package), 3010 (Playdate's `pdxinfo`), 3046 (crash reports'
  version), 3050 (releases from a tag), 3015 (mobile, parked), 2502 (the
  wizard's id question), 2009 (fast compiles)

## Context

- **Other engines** keep a game's name, id, version and publisher once,
  as data, and every export reads them: Unity's Player settings (whose
  company and product names also choose where saves go,
  `Application.persistentDataPath`), Godot's project settings and export
  presets, Defold's `game.project`, GameMaker's Game Options. Bevy has
  none: a game sets its window's title in code.
- **Shuyi before this ADR.** The name was typed twice (`Window::title`
  in the game's code, `web.title` in `shuyi.ron`). The save folder was
  named in code by a `STORAGE_NAME` the template set to the crate's
  name, so two new games named "Snake" on one computer shared
  `%APPDATA%/snake`, and on one web origin every `localStorage` key
  under `snake/`, each overwriting the other's saves. No version or id
  was kept anywhere a package, a crash report or a store could read.
- **Plans waiting on it:** `export-report.json`'s version (3004),
  Playdate's `pdxinfo` (3010), crash reports (3046), releases from a tag
  (3050), the packages' icons and version resources (3049), a store's
  bundle id (3015).
- **The binary can't read `shuyi.ron`** at run time (no files on the web
  or a console), so the values must be compiled in. A build script that
  parses RON in every game would run in every build, which 2009's
  budget counts.
- **`font-atlas`** already writes a generated file with a `// Generated
  by … Do not edit` first line, and the tools compare outputs with what
  is on disk.

## Decision

1. **One part.** `shuyi.ron`'s `app` (`shuyi_cli::project::AppPart`,
   `deny_unknown_fields`) holds `name`, `id`, `version` (semver; without
   it, the crate's `CARGO_PKG_VERSION`), `build` (default 1),
   `publisher` (default empty), `storage` (default the id) and `code`,
   where `app-info` writes the Rust. Every tool that needs one of them
   reads this part: `export`'s version is `app.version` when it is set,
   else the package's.
2. **A Rust constant, generated and checked.** `shuyi app-info` writes
   `code` as one `pub const APP: shuyi::platform::AppInfo`, built with
   `AppInfo::new(name, id, version)` and a `with_` setter for each field
   off its default (storage, build, publisher), its strings escaping
   only `"`, `\` and control characters. Its first line is `// Generated
   by <command> app-info … Do not edit.`, its `const` is
   `#[rustfmt::skip]`, so rustfmt never rewrites it and the tool stays
   its only judge, and it carries a `check-text` marker, so a game's
   text check never counts it. `app-info --check` writes nothing and
   fails naming each file that differs (line endings don't count).
   Never a build script.
3. **`AppInfo`** (`shuyi_platform::app_info`, `no_std`) is config a game
   fills in (ADR-0039 §2): `#[non_exhaustive]`, public fields, `const fn
   new` and `const fn` setters, so the generated file and a game that
   keeps its own both build it in a `const` item. It is in the short
   path's prelude. `BasicGame::new` and `with_own` take it in place of
   the name and the storage name (ADR-0036's short path), and
   `run_game` titles the window with `name` and names the storage
   `storage`.
4. **The storage name follows the id** unless `storage` is set. The id
   is unique to a game by design, so a new game's saves are its own. A
   game that had a storage name before the part sets `storage` to it,
   so its players' saves keep loading.
5. **The rules** (`shuyi_cli::app_info::problems`), each problem naming
   `app.<field>`: the name not empty and with no control character; the
   id reverse-DNS, two or more parts of ASCII letters, digits and `-`,
   each starting with a letter (the characters Apple allows in a bundle
   id, a letter first as Android asks); the version semver, parsed by
   the `semver` crate the CLI already has; the build 1 or more; the
   storage name one folder name (ASCII letters, digits, `.`, `-`, `_`
   and spaces, no `.` or space first or last); and `code` up to date.
   `check --target` reports them on every target as `TargetCheck::App`;
   a project with no `app` gets a line in `limits`, not a problem.
6. **The placeholder is a warning.** A new game's id is
   `com.example.<crate>`, which every crate name `new` makes passes. An
   id under `com.example.` warns that two games with it share their
   saves: a new game must build and pass its gates as it is, and an id
   is a choice only its author can make, before the first build handed
   out.
7. **`Config` and `TargetCheck` are `#[non_exhaustive]`** (ADR-0039 §4
   and §6), since each gains here and will again.

## Consequences

- The name, id and version are typed once; the window, the storage, the
  stamp of a template game's records, `export` and later every package
  read the same values.
- A new game's saves are kept under its id, not its crate's name, so two
  games of one name no longer share them.
- Changing `shuyi.ron`'s `app` needs `cargo shuyi app-info` before the
  build sees it; `check --target` fails until it is run, so CI catches a
  forgotten one.
- The generated file is the one Rust file of a new game rustfmt doesn't
  lay out, and its line can be long.
- The icon (one source picture downscaled by the tool, the window's
  icons in `AppInfo`) comes in 3072, which extends this.
- A game that builds `Config` by a literal, matches `TargetCheck`
  without a wildcard, or calls `BasicGame::new(name, storage, ..)`
  changes at its next upgrade.

## Alternatives considered

- **A build script that reads `shuyi.ron`** — every game's every build
  would parse RON and rerun on each change to the file (2009), and a
  game's build would need the CLI's project types.
- **`env!` variables set by `.cargo/config.toml`** — a second copy of the
  values, and `[env]` reaches every crate of the workspace.
- **The storage name staying the crate's name** — two games of one name
  share their saves, the bug this fixes.
- **The placeholder id as an error** — a new game would fail its own
  gates as made; an agent would set a made-up id to pass them.
- **A hand-written parser for the version** — the CLI already depends on
  `semver`, which parses exactly the grammar, so a second parser would
  only drift.
