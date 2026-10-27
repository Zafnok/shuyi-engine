---
id: "3052"
title: "The web shell: a download progress bar, readable errors in place of a black page, and notes for hosting on Pages, itch.io and in a frame"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3052 — The web shell: download progress, readable errors and hosting notes

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6,
Unreal 5.8 and Defold 1.14. Godot's web export ships an HTML shell
that shows a progress bar while the files download and a message when
the browser lacks a feature the game needs; a game may replace the
shell
(<https://docs.godotengine.org/en/stable/tutorials/platform/web/customizing_html5_shell.html>).
Defold's HTML5 page shows a progress bar and retries a failed download
(<https://defold.com/manuals/html5/>). Unity 6's Web templates show a
loading bar fed by the loader's progress callback. Portals take a web
game as a zip with `index.html` at its root and play it in a frame
(<https://itch.io/docs/creators/html5>).

What Shuyi has today (checked 2026-10-20):

- **The page.** `crates/cli/web/index.html` is a black full-window
  `<canvas id="glcanvas">`, the audio unlock, the plugins, and a last
  line `load("%WASM%");`. It shows nothing else.
- **The loader.** The vendored `mq_js_bundle.js` (macroquad 0.4.16)
  defines `load(url)`: `fetch(url)`, then `WebAssembly.compileStreaming`,
  `WebAssembly.instantiate` and the game's `main()`. Every failure goes
  to its `.catch`, which only calls `console.error`. So:
  - while the wasm downloads and compiles, the page is black, with no
    progress;
  - a missing or broken `.wasm`, or a game that fails in `main()`,
    leaves it black for good;
  - opened from disk (`file://`), the fetch fails and the page stays
    black. `crates/cli/web/README.md` says to serve the folder over
    HTTP, but a person who opened the file never sees that;
  - the game's `main()` calls the bundle's `init_webgl` first
    (miniquad's `run`, in its `src/native/wasm.rs`). With no WebGL 1
    context, that throws a `TypeError` on the null context, which the
    `.catch` logs, and the page stays black. Its `alert("Unable to
    initialize WebGL…")` is only on the WebGL 2 path. When WebGL 1
    lacks `OES_vertex_array_object` or `WEBGL_depth_texture`, it calls
    `alert`. miniquad 0.4.11 asks for WebGL 1 by default
    (`WebGLVersion` in its `src/conf.rs`), and `shuyi_app::window_conf`
    (`crates/app/src/lib.rs`) keeps the default;
  - when a running game panics, miniquad's hook writes it to the
    console (`console_log`, in its `src/native/wasm.rs`), the wasm
    aborts with a `WebAssembly.RuntimeError`, and the canvas stops on
    its last frame.
- **The loading card** (ADR-0024 §6; `Card` in
  `crates/platform/src/app.rs`, white on black by default) shows from
  the game's first frame: only after the wasm has downloaded, compiled
  and started.
- **`cargo shuyi web`** (`crates/cli/src/web.rs`) writes the files in
  `SHELL` (line 25), with the game's title for `%TITLE%` and its binary
  for `%WASM%`. `package` writes the shell before it runs `wasm-opt`
  (line 198), so the page can't be given the final size today.
- **The budget.** `targets/web.ron` sets `package_bytes` at 25 MB and
  `startup_ms` at 5 000 (at 25 Mbit/s), both provisional (3008).
- **Fullscreen already reaches the browser.** The Fullscreen setting
  (`Settings::fullscreen` in `crates/ui/src/settings.rs`,
  `OptionRow::fullscreen()` in `crates/ui/src/screens/options.rs`) and
  the shortcut (`SystemEvent::ToggleFullscreen`, from
  `keys::fullscreen_shortcut` in `crates/app/src/keys.rs`) end in
  macroquad's `set_fullscreen`, which the bundle runs as
  `canvas.requestFullscreen()`. A button on the page would be a second
  way to do the same. Nothing reads the browser's state back: miniquad
  declares `sapp_is_fullscreen` (`src/native/wasm.rs`) and never calls
  it, and `Macroquad::present` (`crates/app/src/backend.rs`) only
  compares the frame with its own flag. So when the player leaves
  fullscreen with the browser's own key, the setting still says on.
- **Hosting.** `.github/workflows/pages.yml` publishes Hello, Grid's
  `dist/web` as it is; 2801 (MVP) moves it to the website's Examples
  page. No doc says how to put a build on itch.io or in a frame on
  another site. In a frame, the Gamepad API needs `allow="gamepad"`
  (`crates/cli/web/README.md`, *The engine's own `gamepad.js`*), and
  `requestFullscreen` needs `allow="fullscreen"`.

What is planned:

- 3004 (After MVP) packages the web folder. An edit request there adds
  the zip with `index.html` at its root that portals take.
- 2019 (After MVP) opens the web build in headless Chromium and fails
  on console errors. Its either-order clause with this ticket reads the
  page's state. Its `render_card` page is a copy of `index.html` with
  only `%TITLE%` and `%WASM%` written in by `sed`.
- 2528 (After MVP) serves the web build with `shuyi web --serve` and
  sends the browser's log to the terminal through its `dev.js`. Its
  either-order clause with this ticket sends the page's state too.
- 3039 (done) added `NOTICES.txt` beside the page, 3066 (MVP) adds a
  `rel="license"` link to it, and 3049 (After MVP) the game's icons and
  title.
- 3043, 2522, 2519 and 2349 (After MVP) add `lifecycle.js`, `net.js`,
  `download.js` and `languages.js` to `SHELL`.
- 3046 (After MVP) leaves "the web page showing an error over a stopped
  canvas" to this ticket.
- 3034 (parked) would move the web build to `wasm-bindgen` and drop
  the bundle. So this ticket works around the bundle from outside and
  patches nothing in it.

Why it matters:

- **The game first.** Visions of Shuyi's web build is made by
  `cargo shuyi web` from its `web` part in `shuyi.ron`. It gets the
  loading screen and the errors at its next pin bump, with no code
  change.
- **Other 2D games.** Every game built with `cargo shuyi web`, and the
  website's examples (2801) and tutorial steps (2803), where a black
  page during a download is the first thing a visitor sees.
- **The end goal** (2026-10-19, `docs/decisions.md`): an agent's game
  comes "with a web ver". Today an agent, or a person, can't tell
  "still loading" from "broken" without the browser's console. A state
  on the page that a script can read tells them.

## Nick input

None.

## Scope

**In:**
- **`crates/cli/web/shell.js`**, the engine's own file (not vendored).
  It is one function expression run at once that defines only
  `window.shuyiShell`: the bundle's strict-mode globals (`canvas`,
  `load`, `plugins` and the rest) must not be declared twice (the note
  in `index.html`). `shuyiShell.start(wasmUrl, wasmBytes)` runs the
  checks, starts the count and returns whether `load` may run.
  `wasmBytes` is a string: a whole number is the wasm's size; anything
  else (a copy of `index.html` that filled in only `%TITLE%` and
  `%WASM%`, as 2019's `render_card` page does) counts nothing, and the
  line says "Loading" with no number. Every error goes through one
  `fail(code, detail)`, with the code as a string literal in the call.
  - **The state, for scripts:** `<html data-shuyi="…">`, one of
    `loading` (the wasm downloads), `starting` (all of it is in; it
    compiles and starts), `running` and `error`. With `error`,
    `data-shuyi-error` holds the code.
  - **Checks before loading,** in `start`, in this order. A failed one
    shows its error and `start` returns `false`, so `load` never runs:
    - `file`: `location.protocol` is `file:`;
    - `wasm`: `typeof WebAssembly !== "object"`;
    - `webgl`: a throwaway canvas gives no `webgl` context, or the
      context lacks `OES_vertex_array_object` or `WEBGL_depth_texture`
      (the two the bundle alerts on), so the bundle's `alert` is never
      reached. The check then frees its context (`WEBGL_lose_context`).
      If a later macroquad asks for WebGL 2, check that instead.
  - **The download, counted.** `start` wraps `window.fetch`. The
    bundle's own `fetch(url)` of the wasm URL (only that URL; every
    other fetch passes through, and the bundle loads other files with
    `XMLHttpRequest`) gets a new `Response` over a `ReadableStream`
    that counts the bytes as `compileStreaming` reads them, so
    streaming compilation keeps working. The new response:
    - has `Content-Type: application/wasm`, so a host that sends
      `.wasm` with another type still works (`compileStreaming` refuses
      any other type);
    - counts against `wasmBytes`, the size `cargo shuyi web` writes
      into the page, not against `Content-Length`: a host that
      compresses sends a smaller length while the stream gives the
      whole bytes. The bar stops at 100 % if more arrive;
    - when the response isn't `ok` (a 404, say), the network fails, or
      a read fails part way, fails with `download`, naming the URL and
      the status;
    - when the browser gives no `response.body` stream, passes the
      response through uncounted, and the line says "Loading" with no
      number.
  - **Starting and running.** `start` wraps `WebAssembly.instantiate`
    and `WebAssembly.instantiateStreaming`, passing each call on
    unchanged. When one resolves, `start` queues a `setTimeout(…, 0)`.
    The bundle calls `main()` in that promise's next `then`, so the
    timer runs after `main()` has returned or thrown. Then the state
    becomes `running`, unless an error came first. A timer, not an
    animation frame, so a hidden tab, or a headless page that drives
    frames itself with `wasm_exports.frame()`, still gets there. These
    are the browser's own functions, so the hooks work with any loader,
    `wasm-bindgen`'s too (3034).
  - **Errors; the first one wins,** and later ones only reach the
    console. `start` adds the listeners and wraps `console.error`; the
    wrapper always calls the original:
    - at any time: a `WebAssembly.RuntimeError`, uncaught or given to
      `console.error` (3043's plugin logs its frame's errors that way),
      fails with `crash`. A Rust panic aborts the wasm with one. This
      rule is checked first;
    - before `running`: any other `Error` object given to
      `console.error` (the bundle's `.catch` does that) or uncaught
      (`window`'s `error` event) fails with `start`, with the error's
      name and message;
    - never: a string given to `console.error` (the bundle's version
      warning), or a rejected promise (the bundle's `exitFullscreen()`
      rejects when the browser has already left fullscreen, and
      `requestFullscreen()` in a frame without `allow="fullscreen"`).
- **What the player sees** (markup and CSS in `index.html`; the words in
  `shell.js`):
  - over the canvas, a panel in the loading card's default colours
    (white on black, `Card::text`): the game's title (`%TITLE%`), a bar
    (`role="progressbar"`, with `aria-valuenow` in percent), and a line
    "4.2 of 18.0 MB" (1 MB = 1 048 576 bytes, as `targets/web.ron`
    counts), then "Starting" once all of it is in;
  - the panel has `pointer-events: none` while loading, so a click
    reaches and focuses the canvas. It goes at `running`, and the
    loading card takes over;
  - an error replaces the bar with plain text the player can select and
    copy (`aria-live="assertive"`, and `pointer-events: auto`): one
    sentence on what happened, one on what to do, then the detail (the
    code, the error's name and message, the URL and status);
  - a "Reload" button (`location.reload()`) under `download`, `start`
    and `crash`, focused when it shows;
  - no text names a key (`AGENTS.md` rule 8): "reload the page", never
    a key, and "the browser's developer console", never its key.

  | Code | When | What the player reads first |
  | ---- | ---- | --------------------------- |
  | `file` | opened from disk | This page runs only from a web server, not as a file. Serve the folder with any static file server, for example `python -m http.server`. (With 2528 done: serve it with `shuyi web --serve`.) |
  | `wasm` | no WebAssembly | This browser can't run WebAssembly. Try a current Firefox, Chrome, Edge or Safari. |
  | `webgl` | no WebGL 1, or its two extensions | This browser can't draw with WebGL here. Turn on hardware acceleration in its settings, or try another browser. |
  | `download` | not `ok`, or the network failed | The game's file didn't download. Check the connection and reload the page. |
  | `start` | failed to compile, link or start | The game didn't start. Reload the page; if it happens again, the details below help whoever made it. |
  | `crash` | a `WebAssembly.RuntimeError` | The game stopped because of an error. Reload the page to play again. |

- **`index.html`** loads `shell.js` first, before `var
  register_plugin;` and the bundle, and its last call is
  `if (shuyiShell.start("%WASM%", "%WASM_BYTES%")) load("%WASM%");`.
  The size is in quotes, so a copy that leaves the placeholder in is
  still valid JavaScript. `mq_js_bundle.js` stays byte-for-byte
  upstream's.
- **`cargo shuyi web`** (`crates/cli/src/web.rs`):
  - `SHELL` gains `shell.js`;
  - `package` runs `wasm-opt` before it writes the shell, and writes
    the final wasm's size in bytes for `%WASM_BYTES%`;
  - the module's doc comment names `shell.js`, and `SHELL`'s names
    `%WASM_BYTES%`.
- **`crates/cli/web/README.md`:**
  - *Files* lists `shell.js` and `%WASM_BYTES%`;
  - a section "Loading and errors": the states and `data-shuyi`, the
    table above, why `fetch` and `WebAssembly.instantiate` are wrapped
    from outside (the bundle stays upstream's, and 3034 carries
    `shell.js` over), the forced `application/wasm`, and the count
    against `%WASM_BYTES%` (none when a copy leaves it unfilled);
  - a section "Putting a web build online", each point checked on the
    day:
    - **any static server, and GitHub Pages:** serve the folder as it
      is; the type the host sends `.wasm` with no longer matters;
    - **itch.io:** upload a zip of the folder with `index.html` at its
      root, marked to be played in the browser; the embed's size; its
      fullscreen button option, which fullscreens itch's own frame; and
      whatever its page says on the day about file names and limits,
      with its link. **Either order with 3004:** if 3004 is done, name
      the zip that `shuyi export web` writes; if not, say to zip the
      folder by hand, and 3004 names its zip here (its clause says so);
    - **a frame on another site:** `<iframe src="…/index.html"
      allow="autoplay; fullscreen; gamepad">`, why each is there (sound
      without a second click where the browser allows it, the
      Fullscreen setting, controllers), and that keys reach the game
      once the frame has the focus, usually after a click into it;
    - fullscreen comes from the game's Fullscreen setting and its
      shortcut, not from the page.
- **Checks in a browser,** on Hello, Grid's release web build:
  - **either order with 2019:** if it is done, add to
    `tools/web-check/check.py`: `boot` waits for `data-shuyi="running"`
    (within its `--seconds`) and fails on `data-shuyi="error"`, printing
    `data-shuyi-error` and the panel's text; the build with its `.wasm`
    removed must show `download`, and a run with Chromium's
    `--disable-webgl` must show `webgl`. Its `render_card` page, which
    leaves `%WASM_BYTES%` unfilled, still runs. If not, run the cases
    below by hand, and 2019 adds them (its clause says so);
  - **by hand** (the `run` skill and the browser pane), each recorded in
    the Completion notes, with screenshots in the PR. If the pane is
    hidden and frames don't run, drive them from the console with
    `wasm_exports.frame()`:
    - the bar on a slow download: a few lines of Python in the
      scratchpad that serve the folder at 256 KiB a second and send
      `.wasm` as `application/octet-stream` (not committed);
    - a copy of `index.html` with `%WASM_BYTES%` left in: the game
      starts, and the line says "Loading" with no number;
    - each error: `file` (opened from disk); `download` (the `.wasm`
      renamed); `start` (the `.wasm` replaced by a few bytes of text);
      `webgl` and `wasm` (a scratch copy of the page with one line
      before `shell.js` that makes `getContext` return `null`, or
      deletes `window.WebAssembly`); `crash` (from the console,
      dispatch an `ErrorEvent` whose `error` is a
      `WebAssembly.RuntimeError`);
    - in a cross-origin frame (a scratch page on another port with the
      notes' `<iframe>`): the game loads, a pad works, and the
      Fullscreen setting fills the screen. Without `allow="fullscreen"`
      the setting does nothing and no error panel shows;
    - leaving fullscreen with the browser's own key while the setting
      is on: what the setting shows after.
- **Either order with 3049:** both change `index.html`; whichever
  lands later keeps the other's parts.
- **Either order with 3066:** if 3066 is done, keep its
  `<link rel="license" href="NOTICES.txt">` in the `<head>`; if not,
  nothing to do (3066 adds it beside this ticket's parts).
- **From 3039 (done):** `shell.js` goes in `SHELL_NOTICES` (`web.rs`'s
  notices table) as the engine's own (its test fails until it does).
- **Either order with 3043, 2522, 2519, 2349 and 2377:** they add
  `lifecycle.js`, `net.js`, `download.js`, `languages.js` and
  `motion.js` to `SHELL` and load them between the bundle and
  `load(...)`. `load("%WASM%")` stays in the page, so their tests hold
  either way; whichever lands later sets `SHELL`'s length.
- **Either order with 2528:** if it is done, its
  `crates/cli/web/dev.js` gains a `MutationObserver` on `<html>`'s
  `data-shuyi`. It sends each state through its queue as `{ source:
  "state", state, error }`, where `error` is `data-shuyi-error` when
  the state is `error`. `shuyi web --serve` prints it as a `state`
  event (`{"event":"state","build":2,"state":"running"}`, plus
  `"error":<code>` when the state is `error`), and without `--json` as
  a line `page: running` or `page: error <code>`. The `file` message
  names `shuyi web --serve`, as the table says. If not, 2528 does both
  (its clause says so).
- **Either order with 3062** (the Fullscreen setting follows the
  browser): if 3062 is done, the by-hand check of leaving fullscreen
  checks that the setting turns off and the next toggle enters
  fullscreen at once, and the notes' fullscreen point says that
  leaving fullscreen with the browser's own control turns the setting
  off. If not, write what the setting showed into the line in 3062's
  *Context* kept for it, and 3062 adds that sentence (its clause says
  so).
- **For the game:** its web build gets `shell.js` from `cargo shuyi
  web` at its next pin bump. No API changes.

**Out (do not do):**
- a fullscreen button on the page: the game's setting and shortcut
  already call the Fullscreen API, and itch.io's frame has its own;
- the Fullscreen setting following the browser when the player leaves
  fullscreen with the browser's own key: it needs `shuyi-app` code that
  reads the browser's state, which 3062 adds (the clause with it says
  what the check above does);
- the zip with `index.html` at its root (3004, an edit request there),
  and uploading it to itch.io or any portal (3050);
- pre-compressed `.br` and `.gz` copies: hosts compress what they serve,
  and a pre-compressed file needs server headers;
- a web app manifest, an offline service worker, or installing the game
  as an app: no game asks for it;
- the page's icons and the game's identity in it (3049);
- the panel in the game's own colours or picture: it uses the card's
  defaults; a follow-up if a game asks;
- retrying a failed download by itself (the Reload button does it);
- counting the music and voice files, which the game fetches later;
- portal SDKs and ads: each needs a JS plugin of our own, so a ticket
  only if Nick names one; threaded wasm: no game asks for it; WebGPU:
  3034 plans it, with WebGL 2 as the fallback;
- the panel's words in other languages: the game's text isn't loaded
  yet;
- phones' browsers (2407, parked);
- any change to `mq_js_bundle.js`.

## Implementation steps

1. Read `crates/cli/web/` (the page, the README, the bundle's `load`
   and `init_webgl`), and `crates/cli/src/web.rs` with its tests. Check
   whether 2019, 2349, 2519, 2522, 2528, 3043 and 3049 are done,
   and read `index.html` and `SHELL` as they left them.
2. Write `shell.js`, then the panel's markup and CSS and the last call
   in `index.html`.
3. In `web.rs`: `SHELL`, the order in `package`, `%WASM_BYTES%`, the
   doc comments.
4. Write the Rust tests below.
5. Write the README's two sections.
6. If 2528 is done, add the `state` line to `dev.js` and its server,
   with its tests. Add `shell.js` to 3039's notices table
   (`SHELL_NOTICES`).
7. Run the browser checks (2019's `check.py`, or by hand). For leaving
   fullscreen, do what the clause with 3062 says.
8. Run the gates, the web build included.

## Acceptance criteria

- [ ] On a slow download the page shows the title, the bar and "n of m
      MB" within the first second, then "Starting", then the loading
      card (screenshots in the PR).
- [ ] Each of `file`, `wasm`, `webgl`, `download`, `start` and `crash`
      shows its text, sets `data-shuyi="error"` and its
      `data-shuyi-error`, and opens no `alert` (recorded in the
      Completion notes, or run by 2019's `check.py`).
- [ ] A host that sends `.wasm` as `application/octet-stream` still runs
      the game.
- [ ] A normal start ends at `data-shuyi="running"`, and the console has
      no error it didn't have before.
- [ ] A copy of `index.html` with `%WASM_BYTES%` left in still starts
      the game.
- [ ] If 2528 is done, `shuyi web --serve --json` prints a `state` event
      with `running` on a normal start.
- [ ] `git diff` on `crates/cli/web/mq_js_bundle.js` is empty.
- [ ] In a cross-origin frame with the notes' `allow`, a pad works and
      the Fullscreen setting fills the screen.
- [ ] The README has both sections, and its table lists exactly the
      codes `shell.js` uses (test).
- [ ] Leaving fullscreen with the browser's own key: with 3062 done,
      the setting turns off and the next toggle enters fullscreen at
      once; if not, what the setting showed is in 3062's *Context*.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/cli/src/web.rs`):
  - `index.html` loads `shell.js` before `var register_plugin;` and
    the bundle, and calls
    `if (shuyiShell.start("%WASM%", "%WASM_BYTES%")) load("%WASM%");`
    after the plugins;
  - `package` writes the wasm's size after `wasm-opt`: `"3"` with the
    fake that writes `opt`, `"10"` for the debug build's `wasm-bytes`;
    none of `%TITLE%`, `%WASM%` and `%WASM_BYTES%` is left (the CSS
    keeps its `100%`); `shell.js` is in the folder
    (`package_copies_wasm_and_writes_the_shell` lists it);
  - the codes in `shell.js`'s `fail("…"` calls equal the codes in the
    README's table;
  - `shell.js` names no key: none of `F5`, `F11`, `F12`, `Ctrl`,
    `Escape` or `Enter` is in it.
- If 2528 is done, its server's tests: a posted `state` line prints as
  the `state` event with `--json` and as `page: …` without it.
- Browser: the checks above, through 2019's `check.py` or by hand.

## Completion notes

*(Filled in by the session that completes the ticket.)*
