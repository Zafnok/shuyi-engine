---
id: "3310"
title: "A desktop keeps its storage in the folder the game names: My Games on Windows, the old folder moved there once"
type: feature
milestone: "Engine: demos"
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-11
---

# 3310 — A desktop keeps its storage in the folder the game names

## Context

1. **The game ticket:** game#0875
   ([link](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0875-keep-desktop-saves-in-my-games.md)).
   Nick, 2026-11-11: saves should never default to AppData; Documents or
   My Games is more accessible.
2. **The job in one sentence:** "keep my game's saves where PC players
   look for them, and bring the old ones along".
3. **Friction numbers:** no friction review: a missing feature.
4. **The generic shape:** a game chooses which folder a desktop keeps its
   storage in: the OS's data folder (today), or `Documents\My Games\<name>`
   on Windows. On the first launch that uses the new folder, the storage
   of the old one moves there, all of it or none, never over a file.
5. **Another game:** a farming game or a platformer sold on Steam keeps
   its saves in My Games the same way.

## Scope

**In:** `AppConfig::desktop_folder` and `storage::DesktopFolder`; the
Documents known folder on Windows; the folder found once at start and
handed to the storage, crash reports, kept sessions, the player's files
and the storage place; the move; `storage::desktop_dir`; ADR-0109; the
crash-reports doc.

**Out:** the short path's `shuyi.ron` (`AppInfo`) naming the folder; the
web's storage.

The API only grows (a field of a `#[non_exhaustive]` config, a new enum
and function). The game's PR that moves the pin follows this one.

## Acceptance criteria

- [x] A first launch with storage in the old folder moves every file
      (exports too) to the new one; a second launch finds them there.
- [x] A new folder that already holds keys is used as it is; the old
      folder is left alone.
- [x] A move that fails keeps the old folder whole, writes over nothing,
      and logs why.
- [x] On Windows, `MyGames` is under the system's Documents.
- [x] `DesktopFolder::Data` (the default) changes nothing.

## Completion notes

- `storage/desktop.rs`: `desktop_dir`, `settle` (the move) and the
  Documents lookup (`SHGetKnownFolderPath`, the one `unsafe` function).
  `storage::StorageAt` is found once in `run` (and in the capture mode)
  and handed to the backend, crash reports and kept sessions.
- Tests in `storage::desktop::tests`, with temporary folders for both
  places.
- The short path (`BasicGame`, `shuyi.ron`'s `app`) can't name the folder
  yet. No ticket: no short-path game has asked; it is one more field of
  `AppInfo` when one does.
- Docs: ADR-0109 and `docs/crash-reports.md`.
- Game: game#0875.
- No break and no `Behaviour:` line: `DesktopFolder::Data`, the default,
  keeps every game's folder, and `storage::platform` and `storage::place`
  keep meaning it. Searched the game (`gh search code --repo
  Zafnok/visions-of-shuyi 'storage::platform'`, `'storage::place'`,
  `'data_dir'`): it calls only `data_dir` and `web_prefix`, in tests, and
  both are unchanged. The game sees the move only through game#0875,
  which opts in.

Book: none, a desktop storage folder is a shipping detail the book's
chapters don't cover; ADR-0109 and `docs/crash-reports.md` say it.
