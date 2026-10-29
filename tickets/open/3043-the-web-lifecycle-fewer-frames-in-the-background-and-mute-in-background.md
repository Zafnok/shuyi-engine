---
id: "3043"
title: "The web lifecycle: a hidden tab suspends and saves inside the browser's event, a closing page saves, and in the background frames slow down and the player may mute"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["3042", "2321", "2511"]
nick_input: none
completed:
---

# 3043 — The web lifecycle, fewer frames in the background, and Mute in background

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and Construct 3. A browser game must save when its tab
is hidden: the browser's own guidance says `visibilitychange` to
hidden is the last event a page can count on, since a phone's browser
may discard a hidden tab without another one, and `unload` and
`beforeunload` don't fire reliably
(<https://developer.chrome.com/docs/web-platform/page-lifecycle-api>).
Construct 3, a web-first engine, gives a game "On suspended" and "On
resumed" when its tab goes to the background and comes back, and its
Audio object has a "Play in background" property. Bevy's `WinitSettings`
runs an unfocused window in a low-power mode (its `unfocused_mode`).

What Shuyi has today (checked 2026-10-21):

- **The web build never suspends.** `crates/cli/web/index.html` has no
  `visibilitychange`, `pagehide` or `pageshow` listener. The vendored
  `mq_js_bundle.js` has one listener on `visibilitychange`, `focus` and
  `blur`, which only calls `wasm_exports.focus` (false when the page is
  hidden or has no focus); 3042 turns that into `FocusLost` and
  `FocusGained`. Nothing sends `Suspend` or `Resume`.
- **Closing the tab never reaches the game's hook.** ADR-0024 §5 says
  `Suspend` and `QuitRequested` are where a game saves.
  `Macroquad::new` calls `prevent_quit` only on native
  (`crates/app/src/backend.rs:88`), so on the web `QuitRequested` never
  comes, and neither does `Suspend`. A game that follows the engine's
  documented pattern loses what it hasn't saved when the player closes
  the tab.
- **A hidden tab runs no frames.** Browsers stop `requestAnimationFrame`
  in a hidden tab, and the bundle's loop (`animation()` in
  `mq_js_bundle.js`) runs on it. So an event queued for the next frame
  is read only when the tab is shown again, or never if it closes. The
  save has to run inside the browser's event. `wasm_exports.frame()`
  runs one macroquad frame on demand; 3003's manual check drove the web
  build in a hidden pane with it.
- **The game's hook saves inside `App::frame`.** `App::frame`
  (`crates/platform/src/app.rs:372`) reads the system's events first,
  and `on_system_event` calls the game's hook at once. The test
  `suspended_ten_minutes_the_game_steps_at_most_once_after_resuming`
  (`crates/platform/src/app/tests.rs`) has a hook that writes its save
  on `Suspend`.
- **Saves are synchronous.** `WebStorage::write`
  (`crates/app/src/storage/web.rs`) calls `quad_storage`, whose
  `quad_storage_set` (`crates/cli/web/quad-storage.js:25`) is
  `localStorage.setItem`, so a write made inside the event has landed
  when the event returns.
- **The JS plugin pattern exists:** `crates/cli/web/gamepad.js` (ours)
  and `crates/app/src/pads/web.rs` (its `extern` block and version
  export; the only file in `shuyi-app` with `#[allow(unsafe_code)]`),
  with a version the loader checks and `shuyi-cli` tests that keep the
  two sides equal (`crates/cli/src/web.rs`,
  `gamepad_js_provides_exactly_the_functions_the_app_imports` and the
  two after it). `cargo shuyi web` writes the files listed in `SHELL`
  (`crates/cli/src/web.rs:25`).
- **The frame rate in the background:** 2321 (MVP) adds `FrameCap`, the
  pacer in `crates/app/src/pacing.rs` and the web path that skips
  `App::frame` when a frame comes before its due time. Its Out section
  asks for a ticket that lowers the rate while the window is unfocused
  or minimized: this one. On the desktop miniquad reports a minimize as
  a focus change (3042). On the web a hidden tab already runs no
  frames; a visible but unfocused page runs at the full rate.
- **Muting in the background:** 3042 gives the game a
  `GameSettings::BACKGROUND` policy (`Keep` or `Mute`). The player has
  no say.
- **The engine's own settings:** 2511 (MVP) adds `impl GameSettings for
  ()` in `crates/ui/src/settings.rs`, which the blank template uses in
  place of its own `settings.rs`.

Why it matters:

- **The game first.** Visions of Shuyi has a web build on the same
  loader and shell (game ADR-0034). Closing its tab today can lose
  progress its hook would have saved on `Suspend`. It gains this at its
  next pin bump with no code change. The lower background rate and the
  player's row are opt-in, since it has its own `GameSettings`.
- **Other 2D games.** Every game `shuyi new` makes has a web build, and
  portals embed web games in tabs players close without quitting.
- **The end goal** (2026-10-19, `docs/decisions.md`): an agent's game
  comes "with a web ver", and it should keep the player's progress
  when the tab closes without the agent knowing about page lifecycles.

## Nick input

None. The trait's defaults keep today's behaviour: the background frame
cap is off unless a game sets it, and the player's Mute in background
starts at the game's choice. The engine's `()` settings and the grid
template set a background cap of 30; they are for new games, not
Visions of Shuyi.

## Scope

**In:**
- **`crates/cli/web/lifecycle.js`**, our own miniquad plugin
  `shuyi_lifecycle` with a `version`, in the style of `gamepad.js`:
  - it keeps a queue of codes (1 = suspend, 2 = resume) and whether it
    has suspended, and gives Rust `shuyi_lifecycle_poll()`, which takes
    the next code (0 when there is none);
  - `visibilitychange` to `hidden`, and `pagehide`: if not suspended, it
    queues a suspend and then runs one frame synchronously, so `App`
    reads `Suspend` and the game's hook saves inside the browser's
    event. Only after the game's wasm has started (`wasm_exports`
    defined). An exception from the frame is caught and logged with
    `console.error`, so the page's own handlers still run. While the
    game is still loading (its loop awaits files), the frame doesn't
    reach `App::frame`, and the code waits for the next frame that does;
  - `visibilitychange` to `visible`, and `pageshow` with `persisted`
    (back from the back-forward cache) while visible: if suspended, it
    queues a resume. The next frame of the bundle's loop reads it;
  - **the frame:** one `wasm_exports.frame()`. If running a frame from
    inside the event proves wrong (a WebGL error in the console, or
    macroquad's loop out of step after it), use this instead: on wasm,
    `run` keeps the `App` and the backend in a `thread_local!`
    `RefCell`, `App` gains a public method that reads and acts on the
    system's events without stepping, and `lifecycle/web.rs` exports a
    function the plugin calls in place of `frame()`. The Completion
    notes say which and why.
- **`crates/app/src/lifecycle.rs`**: a pure queue that turns the codes
  into `SystemEvent::Suspend` and `Resume` (unknown codes are dropped),
  tested on native. The wasm part is `lifecycle/web.rs`: the `extern`
  block declaring `shuyi_lifecycle_poll`, a `PLUGIN_VERSION` and the
  exported `shuyi_lifecycle_crate_version`, under the same comment rules
  as `pads/web.rs`. `Macroquad::system_events` hands over what was
  drained, after 3042's focus events. Native has no codes.
- **The synchronous frame is never skipped.** 2321's web path skips
  `App::frame` when a frame comes before its due time, and a frame run
  from inside the browser's event is usually early. So `run` drains the
  plugin's queue into the backend before the skip check, and runs
  `App::frame` whenever a code is waiting, due or not. The `dt` handed
  on counts from the last frame that ran, as 2321's skip does.
- **`unsafe` named where it is:** `crates/app/Cargo.toml`'s comment on
  `unsafe_code` names every plugin `extern` block instead of calling
  `pads/web.rs` the only one, and `lib.rs`'s crate docs (line 31) name
  `lifecycle.js` beside `gamepad.js` among the JS plugins whose
  functions are imports.
- **`App` doesn't care about order:** a hidden tab gives `FocusLost`
  (the bundle's listener) and `Suspend` (ours) in either order, within
  one frame or across two. Showing it gives `Resume` and `FocusGained`
  in either order. `App` tests play each order.
- **`cargo shuyi web`:** `SHELL` gains `lifecycle.js`; `index.html`
  loads it after `mq_js_bundle.js` and before `load(...)`;
  `crates/cli/web/README.md` gets a section like `gamepad.js`'s: what
  the plugin sends, the synchronous frame and why, and that saves must
  stay synchronous. `shuyi-cli` tests, copied from the gamepad ones:
  the functions match `lifecycle/web.rs`, the versions match, and
  `index.html` loads it between the bundle and the game.
  `package_copies_wasm_and_writes_the_shell` lists the new file.
- **Fewer frames in the background**, on 2321's pacer:
  - `Background` (3042) gains `frame_cap: Option<FrameCap>` and a const
    `with_frame_cap(self, cap)`. `None`, the default, is today's
    behaviour;
  - while `App::is_focused()` is false, the pacer applies the lower of
    the player's cap and the game's background cap (`Off` counts as no
    limit). The request serial doesn't move; `Pacing` reports the cap
    applied and gains `background: bool`;
  - on the web the same rule goes through 2321's skip of early frames;
  - the engine's `impl GameSettings for ()` (2511), which the blank
    template uses, sets `const BACKGROUND: Background =
    Background::KEEP_RUNNING.with_frame_cap(FrameCap::Fps(30))`, so every
    game on the short path gets it. The blank template gains no line;
  - `templates/grid` sets the same in its `impl GameSettings`
    (`crates/game/src/settings.rs`). The template files aren't formatted
    by `cargo fmt --all`; CI's template job checks them;
  - names here are 3042's as its ticket writes them: use what it built.
    **From 2321 (done):** the pacer is `shuyi_app`'s private
    `pacing::Pacer` (`set_cap`, `start`, `wait`, `waited`, `web_frame`
    and `pacing`), handed the cap in `run`'s `pace` from the frame
    presented (`Macroquad::frame_cap`); its tests' fake clock is the
    `Machine` in `crates/app/src/pacing.rs`. The web skip is
    `WebFrame::Skip`: `Macroquad::hold_input` keeps that frame's input
    and system events, and `App::present_again` shows the last frame.
- **The player's Mute in background:**
  - `Settings::mute_in_background: bool`
    (`crates/ui/src/settings.rs`), whose default is the game's
    `GameSettings::BACKGROUND.audio == BackgroundAudio::Mute`, so
    `Settings::restored()` and Restore defaults return to it. `App`
    mutes while unfocused when it is true, in place of reading the
    game's policy directly (3042);
  - `OptionRow::mute_in_background()`, a toggle row with the text key
    `options.row.mute_in_background` ("Mute in background"), through
    `preview_settings` like `OptionRow::fullscreen()`;
  - the toy (`crates/ui/src/toy.rs`) shows the row after the sound
    volume and saves the field in its `File`; its text goes in
    `crates/ui/src/toy/en.ron`. Saves without the field load the
    default (`File` is `#[serde(default)]`);
  - the `()` settings' saved form (2511) saves the field too, with
    `serde(default)`, so a record written before it loads the default;
  - a game whose saved form doesn't save the field keeps the game's
    default each launch.
- **The check in a browser,** on hello-grid's debug web build (3042
  logs each system event the backend sends in debug builds):
  - in one script, log a marker, make `document.visibilityState` read
    `hidden` (`Object.defineProperty`) and dispatch `visibilitychange`,
    log a second marker. The console shows `Suspend` between the two
    markers. Then `visible` again: `Resume` follows on a later frame (if
    the page runs no frames, as in a hidden pane, call
    `wasm_exports.frame()` once). Then a `pagehide` event alone, the
    page still visible: `Suspend` between markers;
  - **from 2019 (done):** this is a `lifecycle` mode of its
    `tools/web-check/check.py`, run in its CI job, `web-render`.
- **From 2019's capture mode on the web (done):** a capture run reads
  no lifecycle events: `crates/app/src/capture.rs`'s `Capturing` takes
  no system event but quitting, and keeps so.
- **Either order with 2351:** if it is done, the row's English goes in
  the engine's built-in text; if not, in the toy's `en.ron` (and 2351
  moves it).
- **Either order with 2110:** if it is done, the `()` settings' form
  raises its `VERSION` for the new field and adds a fixture, as its
  guard asks; if not, the field is in 2110's version 1.
- **From 3039 (done):** `lifecycle.js` goes in `SHELL_NOTICES`
  (`crates/cli/src/web.rs`), the table of the shell's files, as the
  engine's own, beside `gamepad.js` (its test fails until it does).
- **Either order with 3052:** both change `index.html` and `SHELL`.
  Whichever lands second keeps the other's part; `load("%WASM%")` stays
  in the page, so both tickets' tests hold.
- **Either order with 2519:** if it is done, the `Cargo.toml` comment
  and `lib.rs`'s docs name its `download.js` block too.
- **From 3067 (done):** `run` reads the frame's time once,
  `pacing::seconds()` (the `start` it hands the pacer), and the
  synchronous frame for a lifecycle code hands `App::frame` that same
  reading, not a second clock.
- **Either order with 3033** (parked): if its `storage.js` is done,
  the lifecycle plugin sits beside it and nothing else changes; its
  writes are synchronous too.
- **Either order with 2408** (parked, rumble): if 2408 is done, its
  rule stops the motors on the `Suspend` a hidden tab now sends, and
  this ticket adds nothing for it; if not, nothing to do (2408 needs
  nothing more from this ticket).
- **Either order with 2601's web backend** (parked; the second
  follow-up ticket 2601 writes): if it is done, the `Cargo.toml`
  comment names its `http/web.rs` block too, and `lib.rs`'s docs its
  `http.js`; if not, nothing to do (that ticket names
  `lifecycle/web.rs` and `lifecycle.js`).
- **`docs/platform-backends.md`:** the `Suspend` and `Resume` rows say
  the web build sends them for a hidden and a shown tab and a page
  being left, inside the browser's event.
- **What changes for the game:** its web build gets `lifecycle.js`
  from `cargo shuyi web`, and its hook hears `Suspend` and `Resume`
  there. The API is additive (`Settings::mute_in_background`, a public
  field of a struct no game builds by literal since `layout` is
  private; `OptionRow::mute_in_background`; `Background::with_frame_cap`;
  `Pacing::background`). `Pacing` is `#[non_exhaustive]`, built with
  `Pacing::new` (2321), so the new field breaks nothing. The game's PR that moves the pin
  follows this one. The new public items follow ADR-0039.
- **Size.** If the PR passes about 600 changed lines (snapshots and
  fixtures aside), the player's Mute in background (the field, the row,
  the toy and the `()` form) goes to a follow-up ticket written with the
  `write-ticket` skill (After MVP, blocked by this one), and this PR
  keeps the rest.

**Out (do not do):**
- the focus events, held input let go, and the game's pause and
  Keep or Mute policy: 3042;
- the player's frame cap and its options row: 2321;
- a "Leave site?" prompt (`beforeunload`): it blocks the back-forward
  cache and phones ignore it;
- the Page Lifecycle API's `freeze` and `resume` events: a frozen page
  was hidden first, so `visibilitychange` covers it;
- holding the sound on `Suspend` (2394), and the browser's
  `AudioContext.suspend()` (3031, parked);
- `Suspend` for a minimized desktop window (3034; 3042 says why);
- a lower frame rate on an idle screen that has the focus (3013 writes
  that ticket if the Deck's idle draw needs it);
- phones' browsers as a target (2407, parked);
- the page's loading bar and error panel (3052), and its notices
  (3039);
- saves bigger than `localStorage` holds, or IndexedDB (3033's Out).

## Implementation steps

1. Read 3042's ADR and code, 2321's pacer, its web skip and `Pacing`,
   2511's `impl GameSettings for ()`, `gamepad.js`, `pads/web.rs`,
   `crates/cli/src/web.rs` and its tests, `index.html`, the web README,
   and `crates/app/src/lib.rs`'s `run`. Check whether 2019, 2110, 2351,
   2519, 3033 and 3052 are done.
2. Write `lifecycle.js`, `lifecycle.rs` and `lifecycle/web.rs`, wire
   `system_events`, and make `run` drain the queue before the web skip.
3. The `shuyi-cli` changes and tests; `index.html`; the README.
4. The `App` tests for both orders and the hook's write, in
   `crates/platform/src/app/tests.rs`.
5. The background cap on the pacer, with tests; the `()` settings' const
   and the grid template.
6. `Settings::mute_in_background`, the row, the toy and the `()` form,
   with tests.
7. The browser check (the `lifecycle` mode of 2019's `check.py`).
8. `docs/platform-backends.md`, the `Cargo.toml` and `lib.rs` comments,
   then the gates.

## Acceptance criteria

- [ ] `shuyi-cli` tests: `lifecycle.js` provides exactly the functions
      `lifecycle/web.rs` imports; the versions match; `index.html` loads
      it after the bundle and before `load(...)`; the package has it.
- [ ] `lifecycle.rs` tests: codes 1 and 2 give `Suspend` and `Resume`
      in the order queued; 0 and unknown codes give nothing.
- [ ] Pacer on the web path: a frame with a lifecycle code waiting runs
      before its due time, and the `dt` handed on after it counts from
      it.
- [ ] In a browser, hello-grid's debug web build logs `Suspend` between
      the markers around a dispatched `visibilitychange` to hidden, and
      around a dispatched `pagehide`; `Resume` after visible again. The
      console has no error.
- [ ] `App` tests: `FocusLost` then `Suspend`, and `Suspend` then
      `FocusLost`, each in one frame and across two, leave the game
      suspended and unfocused; showing it again in either order resumes
      it with no catch-up steps.
- [ ] `App` test: a system hook that writes a key on `Suspend` has
      written it to the storage when the one `App::frame` that read the
      event returns.
- [ ] Pacer with 2321's fake clock: unfocused with a background cap of
      30 and a player's cap of 60, frames come about 33.3 ms apart;
      focused again, the next frame is due at once; with the player's
      cap at 15, 15 holds; with the player's cap `Off`, 30 holds; with
      no background cap, nothing changes.
- [ ] `Settings::default().mute_in_background` follows the game's
      `BACKGROUND`; Restore defaults returns to it; the toy's save and
      the `()` form without the field load the default; with the row
      on, `App` mutes while unfocused (an `App` test).
- [ ] The options tests (`crates/ui/src/screens/options/tests.rs`) list
      `options.row.mute_in_background` in `ROWS` after the sound volume,
      and the row turns on and off with Left, Right and Confirm.
- [ ] The `()` settings' `BACKGROUND` has a frame cap of 30 (a unit
      test); a new game from `shuyi new grid` builds with the background
      cap set; the blank template has no new line.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the cli plugin tests; the lifecycle queue; the pacer's
  background rule and the web path's forced frame; the setting's
  default and saved forms (the toy's and `()`'s); the options row.
- Integration: `App` order tests and the hook's write through
  `HeadlessPlatform`; the mute while unfocused.
- Browser: the lifecycle check (2019's job, as above).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
