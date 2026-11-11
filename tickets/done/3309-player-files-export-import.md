---
id: "3309"
title: "Files the player keeps: a game hands the player a text file and takes one back, on the desktop and the web, and says where its storage lives"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-11
---

# 3309 — Files the player keeps: hand a file out, take one back, and say where storage lives

## Context

1. **The game tickets:** game#0872 (export saves, and show where they are,
   [link](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0872-export-saves-and-show-where-they-are.md))
   and game#0873 (import saves,
   [link](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0873-import-saves-with-a-version-check.md)).
   Nick, 2026-11-10: most of his testing is on the web build, where saves
   sit in `localStorage` and he cannot see, copy or move them. He wants it
   easy to see where saves are, and to export them as a file (a download
   on the web) and import that file again, on the desktop and the web.
2. **The job in one sentence:** "let the player save a text file the game
   made, open a text file of their own into the game, and see where the
   game keeps its data".
3. **Friction numbers:** no friction review: a missing feature.
4. **The generic shape:** two platform requests and one fact.
   - *Offer a file:* the game gives a file name and its text; the player
     ends up with that file outside the game (a browser download on the
     web; on the desktop, a file written to an exports folder that the
     game can then open in the system's file manager).
   - *Ask for a file:* the game asks the player to pick a text file; the
     answer comes back later as a system event with the file's name and
     text, or "cancelled". On the web: a hidden `<input type="file">`.
     On the desktop: there is no picker (below); the game says "drop the
     file on the window", which `SystemEvent::FilesDropped` (3092, 3307)
     already delivers.
   - *Where storage lives:* a fact the game can show the player: a folder
     path on the desktop (`storage::data_dir`), "this browser's site
     storage, under the prefix `<name>/`" on the web
     (`storage::web_prefix`), none on targets with neither.
5. **Another game that would write the same code:** a farming game's
   "Back up my farm" button; a puzzle game that shares a level as a file;
   a roguelike that imports a seed file a friend sent. Nothing here is
   tactics.

What exists (read on 2026-11-10 at `edb6c888`):

- `crates/cli/web/download.js`: the `shuyi_download` plugin (version 1),
  used only by `crates/app/src/sessions/web.rs` for the debug menu's
  "Save session". A test in `crates/cli/src/web.rs` (~:1752) compares the
  JS functions with the Rust `extern` block.
- `crates/cli/web/page-files.js` / `crates/app/src/page.rs`: the pattern
  for a plugin that hands strings back to Rust (`sapp_jsutils.js`
  objects).
- `crates/app/src/drops.rs`, `crates/platform/src/dropped.rs`:
  dropped files, with their limits (`MAX_DROPPED_BYTES`) and the fact
  `PlatformFacts::file_drops`.
- `crates/app/src/storage/mod.rs`: `data_dir(name)`, `web_prefix(name)`.
- `crates/ui/src/ctx.rs`: `Ctx::request_frame_cap` is the pattern for a
  screen's request that the backend carries out.

## Nick input

None.

## Scope

**In:**
- `Ctx::offer_file(name, text)` and `Ctx::ask_for_file(accept)` (a list of
  extensions such as `[".ron"]`), queued on `Ctx` and carried out by the
  backend after the frame, like the frame cap request.
- `SystemEvent::FileChosen(ChosenFile)` and `SystemEvent::FileOffered(Offered)`
  (`#[non_exhaustive]` enums or structs with constructors, ADR-0039):
  the answer to each request. `Offered` says where the file went:
  `Downloaded` (web), `Written(path)` (desktop), or `Failed(reason)`.
  `ChosenFile` is `Picked { name, text }`, `Cancelled`, or
  `Refused(reason)` (not UTF-8, over the size limit).
- `PlatformFacts::storage_place: StoragePlace` (`Folder(String)`,
  `Browser { prefix }`, `Unknown`), and `PlatformFacts::file_picker: bool`
  (true on the web only, for now). Additive fields.
- Desktop: `offer_file` writes `<data_dir>/exports/<name>` (a name that
  exists gets ` (2)`, ` (3)` …, never an overwrite); `Ctx::open_storage_folder()`
  opens a folder in the system's file manager (`explorer`, `open`,
  `xdg-open` through `std::process::Command`, no new dependency): the
  storage folder, or the exports folder.
- Web: `download.js` grows into the general plugin (version 2,
  `shuyi_offer_file`), and a new function `shuyi_ask_for_file` opens a
  hidden file input. The browser lets a page open a picker only shortly
  after the player pressed something (transient user activation, about 5
  seconds in Chromium): a request made within a frame or two of the key
  press is inside it. If the browser refuses anyway, the answer is
  `Refused` and the game falls back to "drop the file here".
- The headless platform and the Harness: offers are recorded (a test reads
  what was offered), and a test queues the answer to the next
  `ask_for_file`.
- The session record (ADR-0074): a chosen file's answer is an input like a
  dropped file, so a replay gets the same text. Follow how 3092 records
  `FilesDropped`.

**Out (do not do):**
- A native file dialog (a picker or a "save as" box) on the desktop: it
  needs a dependency (`rfd`) with GTK or portal back ends on Linux. Drop
  on the window and the exports folder cover it. Not ticketed.
- What a save bundle holds, its versions and how it is checked: 2138.
- Steam Cloud (3037), the Playdate (it has no files a player reaches),
  binary files (text only, as `Storage` holds text).
- Replacing quad-storage (3033).

## Implementation steps

1. `crates/platform/src/platform.rs`: the two `SystemEvent` variants and
   their types; a new module `crates/platform/src/player_files.rs` for
   `ChosenFile`, `Offered`, the size limit (reuse `MAX_DROPPED_BYTES`)
   and the UTF-8 check.
2. `crates/ui/src/platform_facts.rs`: `StoragePlace`,
   `PlatformFacts::storage_place` and `file_picker`, with builder methods
   like `with_file_drops`. The default is `Unknown` / `false`, so a
   session record made before reads as before.
3. `crates/ui/src/ctx.rs`: the request queue (`offer_file`,
   `ask_for_file`, `open_storage_folder`), drained by the backend each
   frame, as the frame cap is.
4. `crates/app/src/player_files.rs` (new), split `native` / `web` like
   `storage/`: the desktop writes into `exports/` and opens folders; the
   web calls the plugin. Move `sessions/web.rs`' `extern` block here and
   have the sessions use `offer_file`.
5. `crates/cli/web/download.js` → `crates/cli/web/player-files.js`:
   `shuyi_offer_file` (the old `shuyi_download`, any MIME `text/plain`)
   and `shuyi_ask_for_file` (a hidden input with `accept`; on `change`,
   read the file with `File.text()` and keep `{name, text}` or
   `cancelled` until Rust polls `shuyi_file_answer`). Update
   `index.html`, the list `shuyi web` copies, and the JS/Rust comparison
   test in `crates/cli/src/web.rs`.
6. `crates/platform/src/headless.rs` and `harness.rs`: record offers,
   answer asks from a queue, report `storage_place` as
   `Folder("<memory>")`.
7. Docs: the book's chapter on saving (2802) gets a section "Files the
   player keeps"; the `shuyi-screen` skill a line on offering a file.

Expected size: about 900–1,200 changed lines with tests.

## Acceptance criteria

- [x] A Harness test: a screen calls `offer_file("a.ron", "x")`; the
      harness records it and the next frame's events hold
      `FileOffered(Written(..))` or the headless equivalent.
- [x] A Harness test: a queued answer `Picked { name, text }` arrives as
      `SystemEvent::FileChosen` after `ask_for_file`; a `Cancelled` one
      too; a file that isn't UTF-8 or is too big is `Refused`.
- [x] A desktop test (temp dir as the data dir): two offers of `a.ron`
      write `exports/a.ron` and `exports/a (2).ron`.
- [x] `shuyi web` copies `player-files.js`; the comparison test lists
      `shuyi_offer_file`, `shuyi_ask_for_file`, `shuyi_file_answer`.
- [x] A session record that holds a `FileChosen` replays to the same
      state hash.
- [x] `PlatformFacts::storage_place` is `Folder(..)` on the desktop
      backend, `Browser { prefix }` on the web one (unit tests of the
      functions that build them).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the name de-duplication, the UTF-8 and size checks,
  `StoragePlace` from `data_dir` / `web_prefix`.
- Integration: the Harness tests above; the web plugin's JS/Rust
  comparison test.
- Manual (recorded in the completion notes): one download and one pick in
  Chromium and Firefox with `shuyi web` on the hello example.

## Completion notes

What was done:

- `shuyi-ui`: `Ctx::offer_file`, `Ctx::ask_for_file`,
  `Ctx::open_storage_folder` queue a `player_files::FileRequest`
  (`Ctx::take_file_requests` for `App`). `PlatformFacts::storage_place`
  (`StoragePlace::Folder`, `Browser { prefix }`, `Unknown`) and
  `PlatformFacts::file_picker` (`FilePicker`), with `with_storage_place`,
  `with_file_picker` and `can_pick_files()`; both default and are skipped
  when written, so an older session record reads.
- `shuyi-platform`: `SystemEvent::FileOffered(Offered)` and
  `SystemEvent::FileChosen(ChosenFile)` (`player_files`, with
  `ChosenFile::from_bytes` keeping the UTF-8 check and the 256 MB limit,
  `numbered_name`, `check_offered_name`). Three `Platform` methods with
  defaults (`offer_file`, `ask_for_file`, `open_folder`). `App` carries out
  the requests after the frame and puts the answers given at once first in
  the next frame's events, so a session record holds them. The headless
  backend keeps the offers, answers asks from a queue, notes opened folders,
  says its storage is `Folder("<memory>")`, and answers `None` while a
  record plays. `Harness::offered` and `Harness::answer_next_ask`.
- `shuyi-app`: a desktop writes into `<data folder>/exports/` with
  `create_new` and ` (2)`, ` (3)` … and opens folders with `explorer`,
  `open` or `xdg-open`; the web downloads and opens a hidden file input
  through `player-files.js` (the old `download.js`, version 2:
  `shuyi_offer_file`, `shuyi_ask_for_file`, `shuyi_file_answer`), one ask
  at a time. The debug menu's "Save session" downloads through it.
- ADR-0106 records the shape. Docs: `docs/platform-backends.md`, the web
  shell's README, the book's chapter 7 (*Files the player keeps*, with a
  snippet and its test), the `shuyi-screen` skill.

Deviations:

- `PlatformFacts::file_picker` is a `FilePicker` enum (`Opens`, `Absent`),
  not a `bool`: clippy's pedantic `struct_excessive_bools` refuses a fourth
  bool in the struct, as it did for `FileDrops` (ADR-0103 §5).
- A desktop keeps `Platform::ask_for_file`'s default, refused with "drop the
  file on the window"; the web's ask is refused at once when
  `navigator.userActivation` says there was no recent press.
- A session record replayed by keys: reaching into a recorded game between
  frames (`Harness::screen_mut`) stops its record, so the replay test drives
  its screen with key presses.

Manual: `player-files.js` checked in Chromium (the built-in browser) on a
page that stubs `sapp_jsutils`: a download is handed a `Blob` named
`farm.ron`; an ask with no recent press answers 0; after a real click a
file set on the hidden input comes back as `kind: 1` with its name and
bytes, one over the limit as `kind: 3` with the reason, and a `cancel` as
`kind: 2`. Not done: the browser's own picker dialog, Firefox, and a game's
wasm build polling the answer, since no example calls `ask_for_file` yet;
game#0872 and game#0873 are the first callers.

Follow-ups: none written. The game's tickets game#0872 and game#0873 use this
and will move the pin.

No break: the new `SystemEvent` variants, `Platform` methods and
`PlatformFacts` fields are additive (`#[non_exhaustive]` or defaulted).
