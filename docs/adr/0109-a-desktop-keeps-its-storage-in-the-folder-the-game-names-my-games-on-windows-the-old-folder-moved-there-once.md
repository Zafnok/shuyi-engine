# ADR-0109: A desktop keeps its storage in the folder the game names: My Games on Windows, the old folder moved there once

- **Status:** Accepted
- **Date:** 2026-11-11
- **Related tickets:** 3310, game#0875

## Context

`shuyi-app` keeps a game's storage (saves, settings, crash reports, kept
sessions, the player's exports) in `data_dir(storage_name)`: `%APPDATA%`
on Windows, `~/Library/Application Support` on macOS, `$XDG_DATA_HOME`
elsewhere. Players of PC games look for saves in `Documents\My Games\<game>`
on Windows; `%APPDATA%` is hidden. The game asked for that folder
(game#0875), with the saves already in `%APPDATA%` moved there, and
nothing ever lost or written over.

Five places read the folder (the storage, crash reports, kept sessions,
the player's files, the storage place said to the player), each through
`data_dir(name)`.

## Decision

1. **`AppConfig::desktop_folder: DesktopFolder`**, `Data` by default (the
   folder of today). `DesktopFolder::MyGames(folder)` is
   `<Documents>\My Games\<folder>` on Windows; on macOS and Linux it is
   `Data`, as neither has a My Games convention (Application Support and
   the XDG data folder are those systems' places for it). The web keeps
   `localStorage` whatever it says.
2. **Documents is the known folder**, `SHGetKnownFolderPath(FOLDERID_Documents)`
   through `winapi` (already a dependency; features `shlobj`,
   `knownfolders`, `combaseapi`, `winerror`), so a Documents moved to
   OneDrive or another drive is followed; `%USERPROFILE%\Documents` if the
   call fails. The one `unsafe` function sits in `storage/desktop.rs`
   under `#[allow(unsafe_code)]`, as ADR-0104's does.
3. **The folder is found once**, as `run` starts (`StorageAt::find`), and
   handed to every place that reads it, so they agree, and the move runs
   once. `storage::desktop_dir(name, desktop)` names the folder without
   moving anything (for a game's tests and docs).
4. **The move.** When the new folder differs from `data_dir(name)` and
   holds no key (no `.ron` or `.ron.bak` at its top), every file under the
   old folder is copied to the same place under the new one, each with
   `create_new` (never over a file) and flushed. Only when all are copied
   are the old files removed, then its empty folders. A copy that fails
   removes the copies the move made, and the launch keeps the old folder
   and logs why; the next launch tries again. A removal that fails leaves
   an old file beside its copy, logged; the next launch finds keys in the
   new folder and uses it. A new folder that already holds keys is used as
   it is, and the old one left alone.

## Consequences

- A game opts in with one line; games that don't change nothing.
- Copy-then-remove works across drives (a Documents on OneDrive or
  another disk), where a rename would fail.
- Steam Auto-Cloud and similar syncs point at the new folder.
- `storage::platform(name)` and `storage::place(name)` keep meaning the
  `Data` folder; the backend no longer calls them.

## Alternatives considered

- **Rename the old folder** — fails across drives, and a half-done rename
  has no clean way back.
- **Read the registry's `User Shell Folders\Personal`** — holds unexpanded
  `%USERPROFILE%` strings, and Microsoft documents the known-folder call
  instead.
- **A path the game builds itself** — every game would repeat the
  Documents lookup and the move; the engine already owns the folder.
- **`Documents` on macOS and Linux too** — against those systems'
  conventions, and a move for players who never asked for one.
