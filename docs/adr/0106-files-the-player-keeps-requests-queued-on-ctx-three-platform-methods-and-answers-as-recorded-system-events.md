# ADR-0106: Files the player keeps: requests queued on Ctx, three Platform methods, and answers as recorded system events

- **Status:** Accepted
- **Date:** 2026-11-11
- **Related tickets:** 3309, game#0872, game#0873

## Context

On the web a game's saves sit in `localStorage`, where the player can't
see, copy or move them. A game needs to hand the player a file (a backup
of their saves, a level to share), take one back, and say where its
storage lives (game tickets 0872 and 0873). The web reaches the browser
only through miniquad JS plugins (no `wasm-bindgen`, game ADR-0034); a
desktop native dialog would need `rfd`, whose Linux back ends are GTK or
the desktop portal, and a desktop already hears a file dropped on the
window ([ADR-0103](0103-dropped-files-a-system-event-with-names-and-bytes-limits-kept-by-app-and-a-fact-the-backend-says.md)).

A screen's request the platform carries out already exists: the frame
cap, asked for on `Ctx` and confirmed by the platform
([ADR-0050](0050-a-setting-the-platform-applies-a-request-with-a-serial-confirmed-by-a-report-before-the-frame.md)).
The system's events reach the game's hook through `App`, and a session
record notes them where `App` reads them
([ADR-0072](0072-session-records-app-notes-what-the-platform-gives-a-hash-per-frame-of-what-happened-and-ron-never-migrated.md)).
A browser opens a file picker only within its transient user activation
(about 5 seconds after a press), and answers later, once the player has
chosen.

## Decision

1. **The requests, on `Ctx`.** `Ctx::offer_file(name, text)`,
   `Ctx::ask_for_file(accept)` and `Ctx::open_storage_folder(folder)`
   queue a `shuyi_ui::player_files::FileRequest`; `App` takes them
   (`Ctx::take_file_requests`) after the frame is presented and carries
   them out in order. An offered name that isn't one file's (a folder in
   it, empty, `.` or `..`) fails in `App`, before a backend sees it.
2. **Three `Platform` methods, each with a default** (ADR-0039 §7, so no
   backend breaks): `offer_file(name, text) -> Option<Offered>`,
   `ask_for_file(accept) -> Option<ChosenFile>` and
   `open_folder(folder) -> Result<(), String>`. `Some` is an answer now;
   `None` means the backend gives it later through `system_events`. The
   defaults fail the offer, refuse the ask and have no folder: a device
   with no files a player reaches.
3. **The answers are system events.** `SystemEvent::FileOffered(Offered)`
   (`Downloaded`, `Written(path)`, `Failed(why)`) and
   `SystemEvent::FileChosen(ChosenFile)` (`Picked { name, text }`,
   `Cancelled`, `Refused(why)`), both `#[non_exhaustive]`
   (`shuyi_platform::player_files`). `App` puts the answers given at once
   first among the next frame's events, so the game's hook hears them,
   and a session record holds them, a chosen file's text included, as it
   holds a dropped file. A backend playing a record answers `None`: the
   record's frames hold the answers (the headless backend does while
   `push_frame` plays), so a replay neither hears an answer twice nor
   reads another file.
4. **The limits.** A chosen file is text, at most 256 MB, as a dropped
   file (`MAX_CHOSEN_BYTES`); `ChosenFile::from_bytes` refuses a file
   over it or not UTF-8, saying which. A backend reads bytes and hands
   them to it.
5. **Never over a file.** An offer writes `name`, else the first of
   `stem (2).ext`, `stem (3).ext` … that isn't there
   (`player_files::numbered_name`), with `create_new`, so a file made
   meanwhile is never written over.
6. **Two facts, the backend's.** `PlatformFacts::storage_place`, a
   `StoragePlace` (`Folder(path)`, `Browser { prefix }`, `Unknown`), and
   `PlatformFacts::file_picker`, a `FilePicker` (`Opens`, `Absent`: an
   enum for clippy's `struct_excessive_bools`, as `FileDrops`), with
   `can_pick_files()`. A profile says neither: `Unknown` and `Absent`
   from it and by default, skipped when written, so an older record
   reads and a new one is unchanged where nothing is said.
7. **`shuyi-app`.** A desktop writes into `<data folder>/exports/` and
   opens a folder with `explorer`, `open` or `xdg-open`
   (`std::process::Command`, no new dependency); it keeps the default
   ask, refused, since the player drops the file on the window. The web
   downloads and opens a hidden `<input type="file">` through
   `player-files.js` (the old `download.js`, version 2), polled each
   frame while an ask is open, one ask at a time.

## Consequences

- A game backs up and restores its saves on the desktop and the web with
  the same code, and a test drives both through the Harness
  (`Harness::offered`, `Harness::answer_next_ask`).
- The answers reach the game's hook, not the screen that asked: a game
  routes them, as it routes a dropped file.
- A desktop has no "save as" box and no picker. A game that needs one
  writes a ticket; `rfd`'s Linux back ends weigh against it.
- `Platform` grows by three defaulted methods and `SystemEvent` by two
  variants: no break, as both are `#[non_exhaustive]` or defaulted.

## Alternatives considered

- **Answers in `Platform` return values only.** A picker answers seconds
  later, after many frames: the answer has to come through
  `system_events` anyway.
- **The backend queues every answer itself.** Then a default can't
  answer (it has no queue), and a backend that forgets to answer leaves
  a game waiting forever.
- **A native dialog on the desktop (`rfd`).** GTK or the portal on
  Linux, a dependency for every game; dropping the file covers it.
