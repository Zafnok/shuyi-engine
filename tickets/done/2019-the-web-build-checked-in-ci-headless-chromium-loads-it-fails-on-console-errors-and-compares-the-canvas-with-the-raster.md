---
id: "2019"
title: "The web build checked in CI: headless Chromium loads it, fails on console errors and failed requests, and compares the canvas with the raster"
type: infra
milestone: After MVP
model: opus-5.5
effort: high
status: done
blocked_by: ["2018", "2392"]
nick_input: none
completed: 2026-10-29
---

# 2019 — The web build checked in CI

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Bevy's `validation-jobs.yml` builds its examples for
wasm and runs them in Chromium and Firefox with Playwright, saving
screenshots
(https://github.com/bevyengine/bevy/blob/main/.github/workflows/validation-jobs.yml).

Shuyi today:

- `ci.yml`'s `wasm` job builds hello-grid for the web (`cargo shuyi web
  --release`) and uploads it. Nothing opens it.
- `pages.yml` publishes the same build on every push to `main`,
  unopened.
- The web build has its own loader (macroquad's `mq_js_bundle.js`,
  vendored), JS plugins (`gamepad.js`, `quad-storage.js`,
  `sapp_jsutils.js`) and a shell (`crates/cli/web/index.html`). The
  shell carries fixes for errors that broke loading before: `var
  register_plugin;` (game ticket 0225) and the `quad-storage.js`
  version patch (`crates/cli/web/README.md`). Unit tests in
  `crates/cli/src/web.rs` keep both in the files
  (`index_html_declares_register_plugin_before_the_bundle`,
  `quad_storage_js_version_matches_the_locked_crate`), but nothing
  loads the page. A new error of the same kind, or a macroquad upgrade
  that breaks the loader, would reach players unseen.
- 2801 (the website) and 2803 (the tutorial) promise examples and steps
  that play in the browser. Nothing checks that they load.
- 2018 added the capture mode and plans, on the desktop only. On
  wasm32 its `capture` feature builds and does nothing. 2392 added the
  render card (`raster::card`) and its example,
  `crates/app/examples/render_card.rs`.
- **From 2392 (done):** the example captures through
  `shuyi_app::capture` (`window_layout`, `read_window`, `Report`),
  which is compiled on native only, and writes its files with
  `Report`; its window is 768 × 576 (the card's 256 × 192 times 3).
  Making it build and log on wasm32 is this ticket's.

Why it matters:

- **The game first.** Visions of Shuyi has a web build on the same
  loader and shell (game ADR-0034: macroquad's loader, no
  `wasm-bindgen`).
- **Other 2D games:** any game on `shuyi-app` builds for the web with
  `shuyi web` once its `shuyi.ron` has a `web` part; 2777 gives the
  templates one.
- **The end goal** (2026-10-19): a game an agent makes comes "with a
  web ver". Until Visions of Shuyi launches the agent check measures
  turn-based games (2026-10-20), and those have web builds too. Today
  no gate notices a web build that fails to load.

## Nick input

None.

## Scope

**In:**
- **The capture mode on the web** (`shuyi-app`'s `capture` feature,
  2018's code in `crates/app/src/capture.rs`):
  - on wasm32 the plan is `capture.ron` beside the page, read with
    `macroquad::file::load_string` (a fetch on the web, through
    miniquad's `fs::load_file`). A capture build without it logs an
    error and stops. No query string: reading one needs a JS plugin and
    `unsafe` in `shuyi-app`;
  - the run is 2018's: exact frames, only the plan plays, memory
    storage, silent audio, no focus events;
  - at a capture point, `get_screen_data()` reads the canvas. It copies
    the drawing buffer with `glCopyTexImage2D` and reads it with
    `glReadPixels` (macroquad 0.4.16's `Texture2D::grab_screen` and
    miniquad 0.4.11's `read_pixels`), both in WebGL. The picture is
    flipped, cut out and compared with the raster as on the desktop;
  - **no files.** Each capture logs one line to the browser console
    (miniquad's `console_info` and `console_error`, through its
    `log-impl` feature, which macroquad turns on). The line is
    `capture.txt`'s, made by 2018's private `line` in
    `crates/app/src/capture.rs` (the name, the size or both sizes when
    they differ, the scale, the pixels over the tolerance, the worst
    difference, the first such pixel, and `passed` or `FAILED`), with
    `shuyi-capture: ` in front. The web uses that one function, so the
    two never differ:
    - a pass, with `info!`;
    - a failure, with `error!`;
  - after the last step, or the first failure, it logs `shuyi-capture:
    done` and keeps showing that frame;
  - if reading the canvas doesn't work in WebGL (an error, or a picture
    of zeros), stop there: the boot checks below still land, the
    Completion notes say what was tried, and a follow-up ticket
    (`write-ticket`, After MVP) takes the pixel half.
- **The render card in the browser:** a new `crates/app/build.rs`
  passes `--allow-undefined` on wasm32 (the snippet in
  `crates/cli/web/README.md`, with a `//!` doc like
  `examples/hello-grid/build.rs`'s). A package's build script reaches
  its own examples (`cargo:rustc-link-arg`), so 2392's `render_card`
  builds for `wasm32-unknown-unknown`. On the web it logs instead of
  writing files. The `wasm` job also runs `cargo clippy --target
  wasm32-unknown-unknown -p shuyi-app --features capture --lib
  --examples --locked -- -D warnings`, so the wasm32 code is linted.
- **The checker, `tools/web-check/check.py`** (Python 3, Playwright for
  Python):
  - serves a folder on `127.0.0.1` with Python's `http.server`, `.wasm`
    as `application/wasm`;
  - opens it in headless Chromium at `--size WxH` (default `784x784`,
    hello-grid's `WINDOW` in `examples/hello-grid/src/main.rs`; for the
    card, the window 2392's example opens, the card's size times 3),
    device scale 1;
  - records every console message, page error (`pageerror`) and failed
    request (`requestfailed`, or a status of 400 or more, except
    `/favicon.ico`);
  - `check.py boot <dir> --seconds N`: lets N seconds of real frames
    run; fails on a console error, a page error, a failed request, or a
    canvas of one colour;
  - `check.py capture <dir>`: waits for a console line containing
    `shuyi-capture: done` (120 s at most); fails on a console error (a
    failed capture is one), a page error, a failed request, or no
    `done`;
  - one colour is found from Playwright's screenshot of the canvas
    element, decoded in a second blank page (an `<img>` drawn into a
    2D canvas, `getImageData`), so no image library is needed;
  - saves the canvas (`page.png`) and the console (`console.txt`) in
    `--out <dir>` (default `target/web-check/<mode>`);
  - keeps `ALLOWED_CONSOLE_ERRORS` in `check.py`: exact text to the
    reason, empty to start;
  - if WebGL doesn't start on the runner (it has no GPU), launches
    Chromium with its software WebGL flags (`--use-angle=swiftshader`,
    and `--enable-unsafe-swiftshader` on versions that need it), and
    the PR says which;
  - if frames don't advance in the headless page, drives them with
    `wasm_exports.frame()` from the script, and the PR says so;
  - `tools/web-check/requirements.in` names Playwright at one version;
    `requirements.txt` is made from it with `pip-compile
    --generate-hashes` (pip-tools, run by hand, not in CI), so every
    package is pinned with `==` and hashes (`pip install
    --require-hashes`). These run in CI only: never shipped and not in
    the Rust tree, so `deny.toml` doesn't cover them. The PR lists each
    with its license, checked on the day (Playwright: Apache-2.0); none
    may be one ADR-0002 §8 denies. `_typos.toml` excludes
    `requirements.txt`. Python, not Node: no `package.json` in the
    repository (2801 chose Zola for "no Node or npm");
  - `tools/web-check/fixtures/`: three tiny pages the job checks first
    and expects to fail: one logs a console error, one loads a missing
    `.wasm`, and one shows a canvas of one colour.
- **The CI job `web-render`** in `ci.yml` (`ubuntu-latest`, `needs:
  changes`, on code changes only, like the others):
  - makes a Python venv from the runner's `python3` (no new action),
    installs `requirements.txt`, runs `python -m playwright install
    --with-deps chromium`, and checks the three fixtures fail;
  - builds hello-grid's release web build as `pages.yml` does, and runs
    `check.py boot` on it;
  - builds the capture variant: `capture` added to `web.features` in
    `examples/hello-grid/shuyi.ron`, then `cargo shuyi web --release
    --capture`, with `examples/hello-grid/capture/frames.ron` copied
    beside it as `capture.ron`; runs `check.py capture`;
  - builds 2392's `render_card` for wasm32 with `capture`; copies
    `index.html` and every `.js` file of `crates/cli/web/` beside it,
    with `%TITLE%` and `%WASM%` written in by `sed`; runs `check.py
    capture` with `--size` set to the card's window;
  - runs `check.py boot` on every other web build `pages.yml` publishes
    when this is worked (the examples and tutorial steps 2801 and 2803
    add);
  - uploads the output folders as an artifact every time
    (`actions/upload-artifact`, pinned by SHA as the `wasm` job's is);
  - joins `ci-result` by 2018's rule: three green re-runs on the PR with
    the same results, or the PR says why not.
- **The docs:** `crates/cli/web/README.md` gets a section, "Checked in
  CI": what the job does; how to run `check.py` (Python 3 and
  Playwright; it isn't one of the local gates); the flags and the
  tolerance the runner needed; and that a new console error is fixed or
  allow-listed with a reason. The `run-gates` skill names the job as
  CI-only.
- **Either order:**
  - **Either order with 2566:** if 2566 is done, the web's
    `capture.ron` is 2018's plan with its steps on
    `shuyi_platform::script::Step` (2566 folds 2018's `capture::Step`
    into it), played
    through the same `Plan::frames`, so the copied `frames.ron` may
    press actions by name (`act`) and nothing else changes here; if
    not, nothing to do (2566 moves the plan's steps, on the desktop and
    the web alike);
  - with 2011 (Dependabot, done): `.github/dependabot.yml` gets a `pip`
    entry for `/tools/web-check` (weekly, the same cooldown, its updates
    in one group);
  - with 3041 (log sinks): if it is done, the capture lines go through
    its sink, with the target `platform` (3040's `LogCategory`, shown
    at `Info` by default), and `check.py` matches them inside its
    prefix; if not, they use macroquad's `info!` and `error!`, and
    3041's sweep of the macroquad log calls in `crates/app/src` moves
    them;
  - with 3052 (the web shell's progress bar and readable errors): if it
    is done, `boot` waits for `<html data-shuyi="running">` (within its
    `--seconds`) and fails on `data-shuyi="error"`, printing
    `data-shuyi-error` and the panel's text; hello-grid's build with its
    `.wasm` removed must show `download`, and a run with Chromium's
    `--disable-webgl` must show `webgl`. If not, 3052 adds these to
    `check.py`;
  - with 3043 (the web lifecycle): if it is done, `check.py` gets a
    `lifecycle` mode that does 3043's browser check, run in
    `web-render`: on hello-grid's debug web build, markers logged
    around a dispatched `visibilitychange` to hidden and a dispatched
    `pagehide`, with the backend's `Suspend` line between them and
    `Resume` after visible again. A capture run reads no lifecycle
    events. If not, 3043 adds the mode and keeps the capture mode so;
  - with 2522 (the live link for the web build): if it is done,
    `check.py` gains `link <dir>`. It starts `shuyi attach --serve`,
    opens hello-grid's link build with the hash it prints, runs `shuyi
    attach … --act Confirm --json`, and checks that the screens changed.
    `web-render` runs it, and 2522's wasm32 clippy step in the `wasm`
    job (`--features link --lib`) becomes the one above with
    `--features capture,link`, so one step lints both. If not, 2522
    adds the mode and puts `link` in that step's `--features`;
  - with 2333 (scale modes): if it is done, the hello-grid capture plan
    adds the `Fit` capture 2333 adds on the desktop, read from 2333's
    render target in WebGL; if not, 2333 adds it;
  - with 2801 (done): `pages.yml` builds every example whose
    `shuyi.ron` has a `web` section, today Hello, Grid alone, and each
    gets the `check.py boot` run above;
  - with 2803 (the tutorial's steps): if it is done, the web builds it
    added to `pages.yml` get the `check.py boot` run above; if not, it
    adds that run itself when it lands (its clause says so);
  - with 2739 (parked: snake, a web example): if it is done, its build
    is one of those `pages.yml` publishes, and gets the `check.py boot`
    run above; if not, 2739 adds that run when it lands (its clause
    says so);
  - with 2906 (research: asset memory): if it is done, nothing to do;
    if not, nothing to do either (2906 may drive its web runs with this
    ticket's headless Chromium);
  - with 2335, 2336 and 2337: the web shows 2392's render card, so
    their items and paints are compared on the web once they are on the
    card. Nothing more here. 2338 adds no item or paint: its pictures
    are plain sprites, which the card already shows.
  - with 2735, 2736, 2737 and 2738 (the examples, parked): if one is
    done, `check.py boot` opens its web build with the others above. If
    not, it opens its own when it lands (its clause says so): with no
    change if the job loops over the examples' folders, or added to the
    job's list by its PR if the job names the builds by hand;
  - with 2601 (HTTP requests, parked) and its web backend (`http.js`,
    the follow-up ticket 2601's step 2 writes; that PR puts its id
    here): if that backend is done, write a follow-up ticket
    (`write-ticket`) for a `check.py` mode doing its browser check (a
    fetch from the local server, a 404, `TooLarge`, and a refused
    origin). If not, the backend's PR writes that follow-up, as its Out
    line says.
- **Size.** If the PR passes about 600 changed lines (fixtures,
  snapshots and `requirements.txt` aside), the either-order modes
  (`lifecycle`, `link`, the shell's states) go to a follow-up ticket
  (`write-ticket`, After MVP) that names them, and the PR says so.

**Out (do not do):**
- Firefox and WebKit (a follow-up if a browser's bug shows up);
- phones' browsers (2407, parked);
- comparing the page's screenshot with the raster pixel by pixel: the
  comparison reads WebGL's own pixels inside the page; `page.png` is
  for a look;
- holding back `pages.yml`'s deploy (it publishes what `main`'s CI
  already checked);
- Node, npm or a `package.json`;
- the desktop (2018) and its render card (2392);
- a server for developers, rebuilt on change (2528, `shuyi web
  --serve`);
- determinism on wasm32 (2116 runs its golden tables under wasmtime,
  not in a browser);
- the shell's own loading states and error panel (3052), the lifecycle
  plugin (3043) and the live link (2522), beyond the clauses above.

## Implementation steps

1. Read 2018's capture code and ADR (`crates/app/src/capture.rs`,
   `crates/platform/src/capture.rs`), 2392's
   `crates/app/examples/render_card.rs`, 2332's `raster::compare`, `crates/cli/web/` and its README,
   `crates/cli/src/web.rs` and `pages.yml`. Check whether 2011, 2333,
   2522, 3041, 3043 and 3052 are done.
2. Make the capture mode read `capture.ron` and log on wasm32, with the
   line helper shared with `capture.txt`. Add `crates/app/build.rs` and
   the `wasm` job's clippy step.
3. Write `check.py`, `requirements.in`, `requirements.txt` and the three
   fixtures, and run them locally if Python 3 is there (otherwise first
   in CI on the PR). Add the `_typos.toml` line.
4. Add `capture` to hello-grid's `web.features`, and the job.
5. Measure on the runner. If the web needs another tolerance, put it in
   a web plan's `tolerance` and say why in the PR and the README.
6. Add the either-order modes that apply, or the follow-up ticket the
   size rule asks for.
7. Re-run the job three times and decide on `ci-result`.
8. Update the docs and run the gates.

## Acceptance criteria

- [x] `check.py boot` passes on the release build `pages.yml` publishes.
- [x] It fails on a copy whose `index.html` lacks `var register_plugin;`
      (the error game ticket 0225 fixed): shown once in the PR.
- [x] `check.py capture` passes on hello-grid's capture build and on
      the card, and every capture line says `passed`.
- [x] A failing capture fails the job, with its numbers in
      `console.txt`: shown once in the PR with a plan whose tolerance is
      `(channel: 0, pixels: 0)` on a frame that differs, or a one-pixel
      change.
- [x] The three fixtures fail `check.py`, naming the console error, the
      missing URL and the one-colour canvas.
- [x] `requirements.txt` installs with `pip install --require-hashes`,
      and the PR lists each package's license.
- [x] `render_card` builds for `wasm32-unknown-unknown` with `capture`,
      and the `wasm` job's clippy step passes.
- [x] Each either-order mode whose ticket is done runs in `web-render`
      and passes, or the follow-up ticket names it.
- [x] `web-render` is in `ci-result`, or the PR says why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the console line is `capture.txt`'s line with `shuyi-capture: `
  in front, for a pass, a failure and two sizes that differ (tested
  natively).
- Integration (CI): the `web-render` job, the three fixtures first.

## Completion notes

**Done.** CI's new `web-render` job opens the web builds in headless
Chromium with `tools/web-check/check.py` (Python 3, Playwright 1.64.0):
the three fixtures (each must fail), every example `pages.yml` publishes
booted, a copy of Hello, Grid's page without `var register_plugin;`
(must fail), Hello, Grid's capture build and the render card. It is in
`ci-result`. ADR-0053 records the choices.

- **The capture mode on the web** (`crates/app/src/capture.rs`, now
  compiled on wasm32 with `capture`): a web build always runs
  `capture.ron` beside the page (`WEB_PLAN`, read with
  `macroquad::file::load_string`), reads the canvas with
  `get_screen_data()` as on the desktop, and writes no files:
  `Report::console()` keeps the lines, and every capture line is logged
  as `shuyi-capture: ` + its line of `capture.txt` (one private
  `console`, used by the file report too: the desktop's log lines now
  carry the same prefix instead of `capture: `). The web stops at the
  first failure; `capture::finish` logs `shuyi-capture: done (exit code
  N)` and keeps drawing the last frame (the game's `present_again`, the
  card's renderer). New public items, all under `capture`: `WEB_PLAN`,
  `PREFIX`, `DONE`, `report()`, `finish()`, `Report::console()`.
- **Reading the canvas works in WebGL.** Every capture matched the
  raster exactly (worst 0) in Chromium 156 with SwiftShader, Hello,
  Grid's three and the card's: no web tolerance needed, so the pixel
  half needs no follow-up.
- **SwiftShader always:** `--use-angle=swiftshader
  --enable-unsafe-swiftshader`, on the runner and on a machine with a
  GPU, so every machine gets the same pixels. Frames advance in the
  headless page on their own: no `wasm_exports.frame()` driving.
- **`shuyi-app`:** a `build.rs` (`--allow-undefined` on
  `wasm32-unknown-unknown`, for `render_card`); its `image` dependency
  is on every target (the example encodes its PNG files on the web too);
  `tempfile` and `proptest` became native-only dev-dependencies, since
  an example builds with the dev-dependencies and they pulled
  `getrandom` (which needs `wasm-bindgen` on the web) into it.
- **The `wasm` job** runs `cargo clippy --target wasm32-unknown-unknown
  -p shuyi-app --features capture --lib --examples --locked -- -D
  warnings`.
- **Hello, Grid:** `capture` is in `web.features`, so `cargo shuyi web
  --release --capture` builds the capture variant.
- **Python packages** (`tools/web-check/requirements.txt`, CI only,
  checked 2026-10-29): playwright 1.64.0 (Apache-2.0), greenlet 3.5.6
  (MIT AND PSF-2.0), pyee 14.0.0 (MIT), typing-extensions 4.16.0
  (PSF-2.0). None is one ADR-0002 §8 denies. Dependabot has a `pip`
  entry for `/tools/web-check` (weekly, 7-day cooldown, one group).
- **Shown once, locally** (Windows, Chromium 156): a one-pixel change
  in the window's read failed `check.py capture` with `title: 768 × 768
  at scale 2; 1 pixels over (channel 2, pixels 0); worst 128; first 0,
  0; FAILED` in `console.txt`; the page without `var register_plugin;`
  failed with `page error: register_plugin is not defined` (CI repeats
  this one every run).
- **Either order:** 2011 (done): the `pip` entry. 2801 (done): the boot
  step loops over the examples as `pages.yml` does. 2566, 3041, 3052,
  3043, 2522, 2333, 2803, 2739 and 2735–2738 aren't done: their clauses
  now say what they add to `check.py` or the job, as **From 2019
  (done)** lines. 2601's backend isn't done, so its follow-up is its
  PR's. 2906: nothing to do.
- **Size:** about 1000 changed lines with the ADR, docs and ticket
  clauses, near 600 in code and CI. No either-order mode was due here
  (none of their tickets is done), so none moved to a follow-up.

No break: `shuyi-app`'s `capture` module gains items and builds on the
web; `Report::new`, `window_layout`, `read_window` and `cut_out` are
unchanged.

Book: none, the capture mode is a backend's check of its own pixels in
CI, not something a game uses or names.

Nick input: none.
