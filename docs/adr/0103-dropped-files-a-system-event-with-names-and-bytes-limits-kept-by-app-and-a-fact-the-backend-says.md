# ADR-0103: Dropped files: a system event with names and bytes, limits kept by App, and a fact the backend says

- **Status:** Accepted
- **Date:** 2026-11-10
- **Related tickets:** 3092

## Context

Some games can't ship their content and need the player to bring it: a
flipbook made from the player's own copy of a video (2779), a WAD (2782),
a level, a mod or a save. The easiest way in, on the desktop and the web
alike, is to drop the file on the game's window. miniquad 0.4.11 reads a
drop on Windows, Linux, macOS and the web, and macroquad 0.4.16 keeps each
file's path and bytes until the end of the frame (`get_dropped_files`).

The system's events reach a game through `Platform::system_events` and
`App` ([ADR-0024](0024-the-platform-boundary-app-runs-a-frame-time-in-milliseconds-storage-handed-over-and-the-harness-on-the-headless-backend.md)),
and a session record notes them
([ADR-0072](0072-session-records-app-notes-what-the-platform-gives-a-hash-per-frame-of-what-happened-and-ron-never-migrated.md)).
Where a feature works is a platform fact, read from the target's profile
([ADR-0060](0060-platform-facts-at-run-time-read-from-the-targets-profile-handed-through-setup-and-read-by-screens-never-by-rules.md)).
Bundle layers take files handed in at run time (`Layer::owned`,
ADR-0004 §1).

## Decision

1. **The event.** `SystemEvent::FilesDropped(Vec<DroppedFile>)`, in
   `shuyi-platform`. `SystemEvent` stops being `Copy` (a break, written
   in 3092's `Breaking:` line). `DroppedFile`
   (`shuyi_platform::dropped`) is built with `DroppedFile::new(path,
   bytes)`, which keeps what follows the last `/` or `\`: a game sees
   `name()` and `bytes()`, never the player's folders. Its `Debug` shows
   the size, never the bytes.
2. **The layer.** `DroppedFile::layer(files)` makes a `Layer::owned`
   named `dropped`, each file at its name at the bundle's root; the game
   lays it over its own.
3. **The limits, in `App`.** `App::system_events` refuses a drop of more
   than 16 files whole and a file of more than 256 MB alone
   (`dropped::accept`), logs each as a `platform` warning in the game's
   log, and drops an event left empty, before a session record notes the
   events. Every backend and the Harness get the same limits and lines.
4. **The backend.** `shuyi-app` reads `get_dropped_files` in its
   `system_events`, so the web's skipped frames keep a drop too
   (`crates/app/src/drops.rs`), leaving out a file with no path or no
   bytes.
5. **The fact, the backend's.** `PlatformFacts::file_drops`, a
   `FileDrops` (`Heard` or `Unheard`, an enum as `Keyboard` is: clippy's
   `struct_excessive_bools`), with `can_drop_files()` and
   `with_file_drops`; `#[serde(default)]`, so an older record reads. A
   target profile doesn't say it: `from_profile` and the default give
   `Unheard`, and the backend says `Heard` where it reads drops
   (`shuyi-app` on `desktop` and `web`, not `steam-deck`, whose game
   mode has no desktop to drag from). The headless backend and the
   Playdate's say `Unheard`; `Harness::drop_file` drops a file on any
   target.
6. **The tools.** A script step `drop: "path"` (`--drop`), read from the
   working folder (the game's project folder for the contract), played by
   the Harness in a build with `std`; the live link and capture plans
   refuse it. The contract checks the file is there before playing.

## Consequences

- A game hears a drop in its system hook and keeps what it wants; a
  screen asks `ctx.platform.can_drop_files()` before offering one.
- A session record holds a dropped file's bytes, so a replay hears the
  same drop. A record of a big drop is big.
- A game whose hook copied a `SystemEvent` must clone or match by
  reference.
- miniquad reads the whole file before the engine sees it: the size
  limit bounds what the game is handed, not what the backend reads.
- A drop while the game loads reaches no hook and is lost.

## Alternatives considered

- **A unit event and the files on `Ctx`**: keeps `SystemEvent` `Copy`,
  but splits one fact across two places and puts platform input on the
  game's context.
- **Limits in `shuyi-app` alone**: each backend would repeat them, and
  the Harness would not test them.
- **A flag in the target profile**: a profile is an engine kind packed
  in a game's pack, so a field changes its binary form, the pack's
  version and the golden table, for a fact that only the backend knows
  (it is the backend that reads drops).
- **Session records holding names and sizes only**: a replay would not
  hear what the game heard.
