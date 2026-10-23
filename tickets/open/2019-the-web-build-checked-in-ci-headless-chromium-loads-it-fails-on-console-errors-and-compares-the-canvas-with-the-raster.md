---
id: "2019"
title: "The web build checked in CI: headless Chromium loads it, fails on console errors and failed requests, and compares the canvas with the raster"
type: infra
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2018"]
nick_input: none
completed:
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
- 2018 adds the capture mode, plans and the render card, on the desktop
  only. On wasm32 its `capture` feature builds and does nothing.

Why it matters:

- **The game first.** Visions of Shuyi has a web build on the same
  loader and shell (game ADR-0034: macroquad's loader, no
  `wasm-bindgen`).
- **Other 2D games:** any game on `shuyi-app` builds for the web with
  `shuyi web` once its `shuyi.ron` has a `web` part. The templates list
  the `web` target but have no `web` part yet (2528's Out notes it).
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
    `log-impl` feature, which macroquad turns on). One pure function
    makes the line, and 2018's `capture.txt` line uses it too, with
    `shuyi-capture: ` in front:
    - a pass, with `info!`: `shuyi-capture: <name> ok: <w>x<h> at
      scale <s>, <over> pixels over <channel>, worst <worst>`;
    - a failure, with `error!`: the same numbers, the allowed `pixels`,
      and the first pixel with its two colours (`first at (x, y):
      window #rrggbb, raster #rrggbb`), or the two sizes when they
      differ;
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
  its own examples (`cargo:rustc-link-arg`), so 2018's `render_card`
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
    card, the window 2018's example opens, the card's size times 3),
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
  - builds `render_card` for wasm32 with `capture`; copies
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
    `web-render` runs it. If not, 2522 adds the mode;
  - with 2333 (scale modes): if it is done, the hello-grid capture plan
    adds the `Fit` capture 2018's clause describes, read from 2333's
    render target in WebGL; if not, 2333 adds it;
  - with 2335, 2336, 2337 and 2338: the web shows 2018's render card,
    so their items and paints are compared on the web once they are on
    the card. Nothing more here.
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
- the desktop (2018);
- a server for developers, rebuilt on change (2528, `shuyi web
  --serve`);
- determinism on wasm32 (2116 runs its golden tables under wasmtime,
  not in a browser);
- the shell's own loading states and error panel (3052), the lifecycle
  plugin (3043) and the live link (2522), beyond the clauses above.

## Implementation steps

1. Read 2018's capture code and ADR (`crates/app/src/capture.rs`,
   `crates/platform/src/capture.rs`, `crates/app/examples/render_card.rs`),
   2332's `raster::compare`, `crates/cli/web/` and its README,
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

- [ ] `check.py boot` passes on the release build `pages.yml` publishes.
- [ ] It fails on a copy whose `index.html` lacks `var register_plugin;`
      (the error game ticket 0225 fixed): shown once in the PR.
- [ ] `check.py capture` passes on hello-grid's capture build and on
      the card, and every capture line says `ok`.
- [ ] A failing capture fails the job, with its numbers in
      `console.txt`: shown once in the PR with a plan whose tolerance is
      `(channel: 0, pixels: 0)` on a frame that differs, or a one-pixel
      change.
- [ ] The three fixtures fail `check.py`, naming the console error, the
      missing URL and the one-colour canvas.
- [ ] `requirements.txt` installs with `pip install --require-hashes`,
      and the PR lists each package's license.
- [ ] `render_card` builds for `wasm32-unknown-unknown` with `capture`,
      and the `wasm` job's clippy step passes.
- [ ] Each either-order mode whose ticket is done runs in `web-render`
      and passes, or the follow-up ticket names it.
- [ ] `web-render` is in `ci-result`, or the PR says why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the capture line's text for a pass, a failure and two sizes
  that differ (the pure helper shared with `capture.txt`, tested
  natively).
- Integration (CI): the `web-render` job, the three fixtures first.

## Completion notes

*(Filled in by the session that completes the ticket.)*
