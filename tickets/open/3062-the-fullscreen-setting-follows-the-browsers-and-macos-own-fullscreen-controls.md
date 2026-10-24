---
id: "3062"
title: "The Fullscreen setting follows the system: leaving fullscreen with the browser's own control turns it off, and switching with macOS's own control turns it either way"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3062 — The Fullscreen setting follows the system's own fullscreen control

## Context

3052 (the web shell) promised this ticket in its **Out** line: the
Fullscreen setting following the browser when the player leaves
fullscreen with the browser's own control. It needs `shuyi-app` code
that reads the browser's state. The 2026-10-24 backlog audit found it
unwritten.

What Shuyi does today (read 2026-10-24):

- **The setting asks, and nothing reads back.** The Fullscreen setting
  (`Settings::fullscreen`, `crates/ui/src/settings.rs`) changes through
  the Options row (`OptionRow::fullscreen`,
  `crates/ui/src/screens/options.rs`) and the window's shortcut
  (`keys::fullscreen_shortcut`, sent as `SystemEvent::ToggleFullscreen`,
  which `App` turns into `Game::toggle_fullscreen`). Each game frame
  carries it as `Frame::fullscreen`. `Macroquad::present`
  (`crates/app/src/backend.rs`) keeps its own `fullscreen` flag and
  calls macroquad's `set_fullscreen` when a frame differs from it. No
  code reads what the system's window really is.
- **The web.** miniquad 0.4.11 queues `set_fullscreen` and runs it in
  its next frame as the bundle's `sapp_set_fullscreen`
  (`canvas.requestFullscreen()` or `document.exitFullscreen()`). The
  browser switches a few frames later, or never: it refuses
  `requestFullscreen` with no click or key just before it, and in a
  frame without `allow="fullscreen"`. The bundle
  (`crates/cli/web/mq_js_bundle.js`) also defines `sapp_is_fullscreen`:
  true when `document.fullscreenElement` is the canvas. miniquad
  declares it (`macroquad::miniquad::native::wasm`) and never calls it.
  So when the player leaves fullscreen with the browser's own control,
  `Settings::fullscreen` still says on, and an Options row shows On.
  The next toggle turns the setting off and calls `exitFullscreen()`
  again, which does nothing (its promise rejects). The toggle after
  that enters fullscreen. The player presses twice for one switch.
- **macOS has the same gap, both ways.** miniquad's macOS window
  (`src/native/macos.rs`) keeps a private `fullscreen` flag. Its
  `set_fullscreen` sends `toggleFullScreen:` only when the flag
  differs, and the window delegate sets the flag once the window has
  entered or left fullscreen (`window_did_enter_fullscreen`,
  `window_did_exit_fullscreen`; 3044's *Context*). So the system's own
  control (the window's green button) moves miniquad's flag but not
  the setting. After the player leaves that way, the next toggle turns
  the setting off and miniquad sends nothing; the toggle after enters.
  Entering with the system's control is the mirror: the setting says
  off while the window is fullscreen. miniquad 0.4.11 has no getter
  for its flag. But its `native::apple::frameworks` module is public.
  It re-exports the `objc` crate's `msg_send!`, `class!`, `sel!` and
  `sel_impl!`, and defines `NSWindowStyleMask::NSFullScreenWindowMask`
  (`1 << 14`), the bit a window's `styleMask` holds while it is
  fullscreen.
- **Windows and Linux have nothing to follow.** On Windows, miniquad's
  fullscreen is a borderless window that only its own `set_fullscreen`
  changes; the system has no control that takes it out. On Linux,
  miniquad 0.4.11 can't read the window's state, and can't leave
  fullscreen at all (its X11 TODO, 3044's *Context*).
- **What 3052's browser check saw** when the player left fullscreen
  with the browser's own control: *(3052's PR writes it here if it
  lands first.)*
- **Either order with 3052:** if 3052 is done, its Completion notes
  say what the setting showed, and its PR copied that into the line
  above. Add one sentence to the fullscreen point of *Putting a web
  build online* in `crates/cli/web/README.md`: leaving fullscreen with
  the browser's own control turns the setting off. If not, nothing to
  do for it here: 3052's by-hand check then checks that the setting
  turns off, and its notes say so (its clause says so).

Why it matters:

- **The game first.** Its web build, and a macOS build, get it at the
  next pin bump with no code change. An Options screen with the
  Fullscreen row tells the truth, and the shortcut switches on the
  first press.
- **Other 2D games.** Every web build made with `cargo shuyi web`, the
  website's examples, and every macOS build on `shuyi-app`.
- **The contract.** `docs/platform-backends.md` says what a backend
  sends when the system changes the window's mode by itself, so a
  later backend that can read it does the same.

## Nick input

None.

## Scope

**In:**
- **A watch on the system's mode** (`crates/app/src/fullscreen.rs`,
  new, pure, built on every target): `FullscreenWatch { mode: bool,
  settled: bool }`, `pub(crate)`, with:
  - `new(mode: bool) -> Self`: the mode the window opened in, not yet
    settled;
  - `mode(&self) -> bool`: the mode the backend takes the window to be
    in;
  - `ask(&mut self, mode: bool)`: the backend has called
    `set_fullscreen(mode)`. It sets the mode and unsettles the watch;
  - `observe(&mut self, system: Option<bool>) -> Option<bool>`, where
    `system` is what the system says now:
    - `None` (no reader on this system, or none right now) does
      nothing and gives `None`;
    - unsettled: the watch settles once `system` equals the mode, and
      gives `None`;
    - settled: when `system` differs from the mode, the mode becomes
      `system` and it gives `Some(system)`: the system changed the
      mode by itself. The watch stays settled.

  Why it waits: a browser enters fullscreen frames after the request
  and refuses one with no click or key before it; macOS animates for
  about half a second. Reading the old mode meanwhile must not count as
  the player's change.
- **The readers**, `fullscreen::system() -> Option<bool>`, one per
  system, picked with `cfg`, in the layout of `pads.rs` and
  `pads/web.rs`:
  - wasm32 (`crates/app/src/fullscreen/web.rs`): `Some` of the
    bundle's `sapp_is_fullscreen()`, called through miniquad's own
    declaration
    (`macroquad::miniquad::native::wasm::sapp_is_fullscreen`) in an
    `unsafe` block. Its `// SAFETY:` comment: it takes nothing
    and gives a bool, and the bundle always defines it. No second
    declaration, no change to the bundle and no new plugin;
  - macOS (`crates/app/src/fullscreen/macos.rs`): the key window's
    style mask, through `macroquad::miniquad::native::apple::frameworks`
    (`class`, `msg_send`, `nil`, `sel`, `sel_impl`, `ObjcId` and
    `NSWindowStyleMask`). Ask `NSApplication` for its
    `sharedApplication`, and that for its `keyWindow`. A `nil` window
    (the app isn't active, or the window isn't shown yet) gives `None`.
    Otherwise read `styleMask` as a `u64` (`NSUInteger`) and give
    `Some(mask & NSFullScreenWindowMask != 0)`. One `unsafe` block.
    Its `// SAFETY:` comment: these are messages to Cocoa objects on
    the main thread, where macroquad runs the game, and the window is
    checked for `nil` before it is sent anything;
  - every other target: `None`.
- **`unsafe` stays in the backend.** Each reader with `unsafe` carries
  `#[allow(unsafe_code)]` on its function, as `pads/web.rs` does on its
  block. The comments that call `pads/web.rs` the only such file
  (`crates/app/Cargo.toml`'s on `unsafe_code`, and the one above the
  block in `pads/web.rs`) name every file with an
  `#[allow(unsafe_code)]`, as they stand when this lands.
- **The backend** (`crates/app/src/backend.rs`):
  - `Macroquad`'s `fullscreen: bool` becomes `fullscreen:
    FullscreenWatch`, made with `FullscreenWatch::new(false)` (windowed,
    as today);
  - `present` compares `frame.fullscreen` with `mode()`. When they
    differ it calls `ask` and then `set_fullscreen` with the frame's
    mode. Every `set_fullscreen` call in the backend goes with an `ask`
    of the same mode;
  - `system_events` first calls `observe(fullscreen::system())`. On
    `Some(on)` it pushes `SystemEvent::FullscreenChanged(on)`, before
    `ToggleFullscreen` and `QuitRequested`: what the system did comes
    before what the player asked in the same frame. It calls no
    `set_fullscreen`, since the window is already in that mode.
- **The event** (`crates/platform/src/platform.rs`):
  `SystemEvent::FullscreenChanged(bool)`. Its doc comment: the system
  put the window in fullscreen (`true`) or out of it (`false`) with
  its own control (the browser's, macOS's window button), not because
  a frame asked; `App` (an intra-doc link to `crate::App`) sets the
  Fullscreen setting to match. `SystemEvent` is `#[non_exhaustive]`, so the variant is
  additive (ADR-0039), and no `Breaking:` line is needed.
- **`App`** (`crates/platform/src/app.rs`, `on_system_event`):
  `FullscreenChanged(on)` calls `game.set_fullscreen(on)` while a game
  runs. The game's hook hears it after, as it hears every event.
- **`Game::set_fullscreen(&mut self, on: bool)`**
  (`crates/ui/src/game.rs`), beside `toggle_fullscreen`:
  `change_settings(|s| s.fullscreen = on)`. So the setting is saved as
  the shortcut saves it, and nothing is written when it already says
  `on`. An open Options screen shows it on its next frame (the row's
  `get` reads the settings). Discarding that screen's changes puts the
  setting back to what it was when the screen opened, and the window
  follows, as after the shortcut (`restore_settings`; the test
  `discard_after_another_save_puts_the_saved_settings_back_too`).
- **Docs:**
  - `docs/platform-backends.md`: the `system_events` table gains a row,
    `FullscreenChanged(on)` | the system put the window in or out of
    fullscreen with its own control (the browser's, macOS's window
    button) | the Fullscreen setting becomes `on` and is saved. The
    `frame.fullscreen` paragraph under `present` adds: a backend that
    can read the system's window mode sends it, waits until the system
    shows a mode a frame asked for before it reads again, and never
    sends it for a switch a frame asked for;
  - `Frame::fullscreen`'s doc comment names the event;
  - no new ADR: this is one more event in the set ADR-0024 made
    non-exhaustive for such additions, and its contract is in
    `docs/platform-backends.md`. ADR-0024 is not edited.
- **No key named** (`AGENTS.md` rule 8): the docs and comments say
  "the browser's own control" and "the window's fullscreen shortcut",
  never a key.
- **Either order with 3044** (the window record and the cursor): both
  change `Macroquad`'s fullscreen flag in `backend.rs`. If 3044 is
  done, start the watch with `FullscreenWatch::new` and the mode in
  its `Opened` (unsettled, so a fullscreen start on macOS waits for
  its animation). Its first-frame `set_fullscreen(true)` on Windows
  goes with an `ask` too. `present` asks for what `wanted_fullscreen`
  gives, and its window record and `IdleCursor` read `mode()`. So a
  mode changed with macOS's own control is recorded, and hides or
  shows the cursor, as a switch from the game does. If not, the watch
  starts windowed, and 3044 makes those changes (its clause says so).
- **For the game:** additive only. Its web build and a macOS build get
  it at the next pin bump with no code change. If its hook matches on
  `SystemEvent`, its `_` arm covers the new event. The game's PR that
  moves the pin follows this one.

**Out (do not do):**
- a request the system refuses. A browser refuses `requestFullscreen`
  with no click or key just before it (so a web start with the setting
  saved on is refused), and in a frame without `allow="fullscreen"`.
  The watch then never settles, and the setting stays on with the page
  windowed until the player turns it off, as today. The refusal
  reaches only a promise inside the bundle; seeing it needs the bundle
  patched or a web plugin of our own;
- readers on Windows and Linux: there is nothing to follow (*Context*);
- a toggle pressed during macOS's own fullscreen animation, which may
  need a second press, as today;
- a fullscreen button on the page (3052's **Out** says why);
- any change to `mq_js_bundle.js`, a new web plugin, or a change to
  `SHELL` in `crates/cli/src/web.rs`;
- the window's size and place, and the cursor (3044), apart from the
  clause above.

## Implementation steps

1. Read `crates/app/src/backend.rs` (`present`, `system_events`),
   `pads.rs` and `pads/web.rs`, `crates/platform/src/platform.rs`
   (`SystemEvent`, `Frame`), `app.rs` (`on_system_event`),
   `crates/ui/src/game.rs` (`toggle_fullscreen`) and
   `docs/platform-backends.md`. In the cargo registry, read miniquad
   0.4.11's `src/native/wasm.rs` (`sapp_is_fullscreen` and the queued
   requests), `src/native/macos.rs` (`set_fullscreen` and the
   delegate) and `src/native/apple/frameworks.rs`. Check whether 3044
   and 3052 are done.
2. Write `crates/app/src/fullscreen.rs` with `FullscreenWatch` and its
   tests.
3. Add the readers in `fullscreen/web.rs` and `fullscreen/macos.rs`,
   and `fullscreen::system`. Build for `wasm32-unknown-unknown`
   (the web build in the `run-gates` skill).
4. Add `SystemEvent::FullscreenChanged`, `Game::set_fullscreen` and
   `App`'s arm, with their tests.
5. Wire the backend: the watch, `ask` in `present`, `observe` in
   `system_events`.
6. Write the docs and the two comments on `unsafe_code`. If 3052 is
   done, add its README's sentence.
7. Check the web build by hand: Hello, Grid's web build, served and
   opened in the browser pane (the `run` skill; if the pane is hidden
   and frames don't run, drive them from the console with
   `wasm_exports.frame()`):
   - turn fullscreen on with the window's fullscreen shortcut; the
     canvas fills the screen;
   - leave with the browser's own control;
   - the game's `settings` record in `localStorage` (under the game's
     storage name) says `fullscreen:false`;
   - the shortcut once more enters fullscreen at once.

   If the pane can't go fullscreen, play the browser's part from the
   console. After the shortcut, `Object.defineProperty(document,
   "fullscreenElement", { configurable: true, get: () => canvas })`;
   a few frames later, define it again to give `null`. Wrap
   `canvas.requestFullscreen` to count its calls: the next shortcut
   calls it once. Record in the Completion notes which way was used
   and what was seen, with screenshots in the PR.
8. Run the gates, the web build included. CI's macOS job builds the
   macOS reader; say in the PR that it was not checked on a Mac.

## Acceptance criteria

- [ ] `FullscreenWatch` passes every case under *Tests required*.
- [ ] In the web build, after leaving fullscreen with the browser's own
      control (or its stand-in from the console), the saved settings
      say `fullscreen:false`, and the next shortcut enters fullscreen
      at once (it calls `requestFullscreen` once). Recorded in the
      Completion notes, with screenshots in the PR.
- [ ] Through a `Harness`, `FullscreenChanged(false)` after the
      shortcut turns the setting off, saves it, and the next frame
      asks for a window; `FullscreenChanged(true)` does the reverse;
      the game's hook hears both.
- [ ] `Game::set_fullscreen` with the value the setting already has
      writes nothing to storage.
- [ ] `git diff` on `crates/cli/web/mq_js_bundle.js` and on `SHELL` in
      `crates/cli/src/web.rs` is empty.
- [ ] The macOS reader builds in CI's macOS job, and the PR says it was
      not checked on a Mac.
- [ ] `docs/platform-backends.md` lists `FullscreenChanged` in the
      `system_events` table.
- [ ] No text this ticket adds names a key.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/app/src/fullscreen.rs`), with no window:
  - `new(false)`, then `observe(Some(false))` gives `None` and
    settles; `observe(Some(true))` then gives `Some(true)`, and
    `mode()` is `true` (macOS's own control entering);
  - after `ask(true)`, `observe(Some(false))` many times gives `None`
    (the browser hasn't switched yet); `observe(Some(true))` gives
    `None`; then `observe(Some(false))` gives `Some(false)` and
    `mode()` is `false` (the browser's own control leaving); a
    further `observe(Some(false))` gives `None`;
  - after `ask(false)` from fullscreen, `observe(Some(true))` gives
    `None` (still leaving), and `observe(Some(false))` settles;
  - a refused request: after `ask(true)`, `observe(Some(false))` never
    gives anything; after `ask(false)`, `observe(Some(false))` settles
    and the watch reads again;
  - `observe(None)` never gives anything and never settles, and
    `mode()` stays what was asked.
- Unit (`crates/ui/src/game/tests.rs`): after `toggle_fullscreen`,
  `set_fullscreen(false)` turns the setting off and the stored
  settings say off; `set_fullscreen(false)` again writes nothing.
- Unit (`crates/platform/src/app/tests.rs`): with a hook that records
  what it hears, `FullscreenChanged(false)` after `ToggleFullscreen`
  makes the last frame windowed, and the hook heard
  `fullscreenchanged(false)`.
- Integration (`crates/platform/tests/it/main.rs`), through the
  `Harness`: the shortcut, then `FullscreenChanged(false)`: not
  `fullscreen()`, the setting off, and the stored `settings` record
  says `fullscreen:false`; then `FullscreenChanged(true)` turns both
  on.
- Manual: the web check in step 7, in the Completion notes.

## Completion notes

*(Filled in by the session that completes the ticket.)*
