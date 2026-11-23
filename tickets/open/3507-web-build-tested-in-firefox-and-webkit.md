---
id: "3507"
title: The web build is tested in Firefox and WebKit as well as Chromium, and the web sound in Firefox
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3507 — The web build is tested in Firefox and WebKit as well as Chromium, and the web sound in Firefox

## Context

The web is a first-class target. `pages.yml` publishes every example,
the book's playground runs in a browser, and a game ships its
`shuyi export web` package to itch.io. Every check of that build runs in
one browser engine: Chromium.

- CI's `web-render` job (`.github/workflows/ci.yml`, about lines
  307–470, ticket 2019) installs one browser:
  `python -m playwright install --with-deps chromium` (line 336).
- `tools/web-check/check.py` launches only
  `playwright.chromium.launch(args=CHROMIUM_ARGS)` (line 380). Its modes
  are `boot`, `capture`, `fixtures` and `frame` (3505).
- `tools/research/2914/web/browser_smoke.py` (the cold rules' replay
  against native, run by `web-render`) launches Chromium only (line 66).
- `.github/workflows/sound-parity.yml` installs Chromium only (line 54),
  and `tools/sound-parity/capture.py` records the web sound in Chromium
  only (`Browser.open`, about line 473). `tools/sound-parity/budget.ron`
  lists the web platforms `browser-chromium` and `browser-in-house`, both
  Chromium.

[ADR-0053](../../docs/adr/0053-the-web-build-checked-in-headless-chromium-playwright-for-python-and-a-capture-mode-that-logs.md)
accepted this: its Consequences say a bug of Firefox's or WebKit's isn't
seen. Ticket 2019, which built the checker, put Firefox and WebKit Out
as "a follow-up if a browser's bug shows up", and none was written.
Several done tickets (3033, 3505, 3309, 2413) record Firefox as never
checked. `grep -rniE 'firefox|webkit|safari' tickets/open` finds no
ticket that runs either in CI: only DOOM's manual Firefox check (2782),
an error message's text (3052), the parked phone browsers (2407), and
Out lines in 3336, 3338 and 3340 that name this ticket.

Why it matters. WebKit is the only browser engine on iOS, and Safari's
desktop share is not small. The browsers differ most where this engine
does its own work: Web Audio (autoplay, resuming an `AudioContext`, the
in-house worklet of ADR-0075), WebGL 1 and its extensions, keyboard
focus inside another site's frame (3505), pointer and key events, and
wasm memory growth. The engine also runs macroquad's own JS loader
(game ADR-0034) and its own plugins (`crates/cli/web/*.js`: `gamepad.js`,
`sound.js`, `storage.js`, `pack-file.js` and the rest), and that is the
code that drifts between browsers. Bevy runs its wasm examples in
Chromium and Firefox (ADR-0053 cites it). Playwright for Python, already
pinned (`tools/web-check/requirements.in`, `playwright==1.64.1`), drives
Firefox and WebKit too, and on a free Linux runner: no money, no new
package ecosystem.

Source: the blind-spot audit of 2026-11-22, candidate 76, checked
against the files above for this ticket.

## Nick input

None.

## Scope

**In:**
- `check.py --browser chromium|firefox|webkit` (Chromium unless given),
  for every mode. Each browser starts with what it needs to draw WebGL
  without a GPU, written as a constant beside `CHROMIUM_ARGS` with a
  comment saying why.
- `browser_smoke.py --browser`, the same three.
- A new CI job, `web-browsers`, with a matrix of `firefox` and `webkit`.
  It runs the same page checks `web-render` runs on the pages
  `web-render` built, without building them again. It is in
  `ci-result`. `web-render` stays the Chromium job, unchanged in what it
  checks.
- The web sound recorded in Firefox: a platform `browser-firefox` in the
  sound gate, judged on capture and parity. Its timings are recorded but
  not judged, as no budget was measured for them.
- A known difference between browsers is written down, never hidden: a
  table in `check.py` with the browser, the check and the open bug
  ticket that fixes it.
- A new ADR that extends ADR-0053 (the `write-adr` skill), and the docs
  that say which browsers CI runs.

**Out (do not do):**
- Phones' browsers and touch (2407, parked; the scope is Nick's,
  decision ticket 3527). WebKit here is desktop WebKit on Linux.
- Real Safari on a Mac runner, and Windows' Edge. Playwright's WebKit
  build on Linux is the WebKit cell.
- The tutorial's steps: they boot in Chromium only (3502 moves them to
  a job of their own).
- WebGPU, and anything on a wasm-bindgen web build.
- Measured frame times in each browser: 3340 (the clause below).
- Exported packages launched on Windows and macOS: 3336.
- What a target's support tier promises: 3338 (the clause below).
- A lint that every CI job has a timeout, and CI's cost: 3508. This
  ticket sets its own new job's `timeout-minutes` only.
- Making a cell a required check on `main`: branch protection is
  Nick's (decision ticket 3529).
- Widening a capture's tolerance for a browser. ADR-0052 §5 keeps
  `Tolerance::default()` for every backend; a browser that misses it has
  a bug, and a bug ticket.
- Holding the Firefox sound to a performance budget. That needs a
  calibration of its own (`calibrate.yml`, ticket 2064), in a later
  ticket once the cells are stable.
- Node, npm or a `package.json` (ADR-0053's alternatives).

**Licenses.** Playwright's Firefox (MPL-2.0) and WebKit (LGPL and BSD
parts) are downloaded by `playwright install` on the runner, as
Chromium is. They are never shipped, vendored or linked: ADR-0002 §8
governs shipped dependencies and assets, and these run in CI only, as
ADR-0053 §2 says of the checker. The ADR says so.

**Public types.** None: this is CI, Python and `xtask`. The new
`Contract` variant is in `xtask`, which is not published.

**Either order with 3340:** if 3340 is done, `check.py measure` takes
`--browser` (`chromium`, `firefox`, `webkit`) as the other modes do, and
3340's `.github/workflows/wall-times.yml` measures in each browser; if
not, nothing to do (3340 makes `check.py measure` take the `--browser`
this ticket adds, and measures in each browser, as its own clause says).

**Either order with 3338:** if 3338 is done, add a line for headless
Firefox and one for headless WebKit in CI to `docs/targets/web.md`'s
`## On the device`, beside Chromium's; if not, nothing to do (3338
writes those lines from the CI jobs it finds, these among them).

**Either order with 3229:** if 3229 is done, `check.py lose` runs in
each browser too, and `web-browsers` runs it on the render card's page
and fails its copy without the plugin, as `web-render` does; if not,
nothing to do (3229's `lose` mode takes the `--browser` this ticket
adds, and runs in `web-browsers` beside the other modes).

**Estimated size:** about 1,500 lines (code 800, tests 300, docs 400).

**Split:** past 900 lines, split off steps 7–10 (the web sound in
Firefox) into a new ticket that names this one.

## Implementation steps

1. **`check.py --browser`.** In `tools/web-check/check.py`, add
   `--browser` with the choices `chromium`, `firefox` and `webkit`
   (default `chromium`). `main` launches
   `getattr(playwright, name).launch(...)` with that browser's options:
   `CHROMIUM_ARGS` for Chromium as today, and for Firefox the
   `firefox_user_prefs` it needs for WebGL without a GPU (found by
   trying; `webgl.force-enabled` is the likely one) and for sound to
   start without a gesture. Each option gets a comment saying why. The
   first line printed names the browser and its version, as the
   Chromium line does now. Update the module's docstring.
2. **Known differences.** Add `BROWSER_GAPS: dict[tuple[str, str], str]`
   beside `ALLOWED_CONSOLE_ERRORS`: the key is the browser and the
   check (`boot <page>`, `capture <page>`, `frame`, …), the value the
   reason and the open bug ticket. A check listed there still runs, and
   prints `gap: <reason>` with its failures instead of failing. Empty to
   start. Each entry the session adds names a bug ticket it writes
   (`write-ticket`) in the same PR. `check.py` fails an entry for a
   check that now passes, so a fixed gap is removed.
3. **`browser_smoke.py --browser`.** In
   `tools/research/2914/web/browser_smoke.py`, the same flag and the
   same launch options. Import them from `check.py` (as
   `tools/sound-parity/capture.py` imports `serve`), not a copy.
4. **The pages, once.** In `web-render` (`ci.yml`), upload the pages the
   job built and checks as an artifact `web-pages`: `target/web/*`
   (every example and the render card), Hello, Grid's capture build
   (`examples/hello-grid/dist/web` with its `capture.ron`), and the
   packed export. Upload it before the checks that can fail, so a
   Chromium failure doesn't hide what the other browsers say.
5. **The `web-browsers` job.** A new job in `ci.yml`, after the
   `web-render` comment block, with its own comment. `needs:
   [changes, web-render]`, the same `if` on `code`, `runs-on:
   ubuntu-latest`, `timeout-minutes` set, actions pinned by digest,
   `persist-credentials: false`. A matrix `browser: [firefox, webkit]`
   with `fail-fast: false`. It installs the checker from
   `requirements.txt` with `--require-hashes`, runs
   `playwright install --with-deps ${{ matrix.browser }}`, downloads
   `web-pages` (a wasm build of the cold rules too, or it builds the
   three small `cold-rules-web-fixture` wasm files and the native
   fixture as `web-render` does: whichever is shorter), and runs with
   `--browser ${{ matrix.browser }}`:
   - `check.py fixtures`;
   - `check.py boot` on every example page;
   - the copy without `var register_plugin;`, which must fail;
   - `check.py frame` on Hello, Grid and the copy that takes no focus on
     fullscreen, which must fail;
   - `check.py boot` on the packed export;
   - `check.py capture` on Hello, Grid's plan and the render card;
   - `browser_smoke.py` for the three kernels.
   The consoles go in the step summary, and `target/web-check/` is
   uploaded as `web-render-<browser>`. Add `web-browsers` to
   `ci-result`'s `needs` (and to its check of each result, as the other
   jobs are).
6. **What fails first.** Run the job on the PR. For each check that
   fails in a browser: fix it in the engine's web code if the fix is
   small and the browser is right; otherwise add a `BROWSER_GAPS` entry
   and write its bug ticket. A cell that fails now and then without a
   change goes in `tools/flaky.ron` as `Job("web-browsers (<browser>)")`
   with its ticket, as the run-gates skill says, never dropped from the
   matrix.
7. **A sound platform that is not timed.** In
   `crates/xtask/src/sound_parity.rs`, add a `Contract` variant
   `Untimed(String)`: every cell but the timings is mandatory, and the
   timings are recorded and reported as `unsupported` with that reason.
   `platform_cells` and `perf_failures` skip `performance_cells`,
   `decoder_speed_cells` and `mixer_speed_cells` for it. Add
   `"browser-firefox": Untimed("no budget is measured for Firefox's
   timings yet")` to `tools/sound-parity/budget.ron`'s `platforms`.
8. **Firefox records.** In `tools/sound-parity/capture.py`, the
   `browser` command takes `--browser chromium|firefox` (Chromium
   unless given). `Browser.open` launches Firefox with the prefs that
   let sound start (no mute, autoplay allowed) and the same click to
   unlock it, and its output reaches the PulseAudio sink as Chromium's
   does. Recording in Firefox writes `D/browser-firefox/`, its
   `meta.json` naming `firefox` and its version. The memory reading
   (`chrome_kib`) is Chromium's; for Firefox it is left out of
   `perf.jsonl`, which the `Untimed` cells don't judge. `--only` takes
   `browser-firefox`.
9. **The workflow.** In `sound-parity.yml`, install `chromium firefox`,
   and add a step after "Record the web backend on its real loader"
   that records the quad-snd page in Firefox. `retry.py`'s `HERE` needs
   no Firefox: its timings are not judged. The port-ready step checks
   `browser-firefox` too.
10. **The sound's tests.** In `crates/xtask/src/sound_parity/tests.rs`,
    an `Untimed` platform's matrix: its capture and parity cells are
    mandatory and judged, a missing recording fails it, and its timing
    cells are `unsupported` and not mandatory. `capture_test.py` covers
    the browser choice.
11. **The ADR.** A new ADR (number from `cargo xtask next-id adr`) that
    extends ADR-0053: three browsers, Chromium the job that was there,
    Firefox and WebKit a matrix on the same pages, the gaps table, the
    untimed Firefox sound, and the licenses paragraph above. ADR-0053
    gets an "Extended by" line and its index row in
    `docs/adr/README.md` says so.
12. **The docs.** `crates/cli/web/README.md`, *Checked in CI*: which
    browsers, which checks, and how to run one by hand
    (`check.py boot <dir> --browser firefox`). `docs/targets/web.md`
    says the build is checked in Chromium, Firefox and WebKit.
    `tools/sound-parity/README.md` lists `browser-firefox` and what
    `Untimed` means.

## Acceptance criteria

- [ ] `check.py boot <dir> --browser firefox` and `--browser webkit`
      run on a machine with Playwright's browsers, and print the browser
      and its version first; with no `--browser` it runs Chromium with
      `CHROMIUM_ARGS`, as before.
- [ ] `check.py fixtures --browser firefox` and `--browser webkit` each
      fail all three fixtures, naming why, as in Chromium.
- [ ] The `web-browsers` job runs on a PR that changes code, in two
      cells, `firefox` and `webkit`, on the pages `web-render` built, and
      `ci-result` fails when either cell fails.
- [ ] In each cell: every example page boots, the page without `var
      register_plugin;` fails, the frame check passes on Hello, Grid and
      fails on its copy, the packed export boots, both capture plans
      pass at `Tolerance::default()`, and the three cold-rules replays
      match native. Or the check is in `BROWSER_GAPS` with a reason and
      an open bug ticket.
- [ ] A `BROWSER_GAPS` entry whose check passes fails `check.py`.
- [ ] `web-render`'s Chromium checks are unchanged: the same steps, the
      same pages, the same failures.
- [ ] The sound gate's matrix has `browser-firefox` cells: capture and
      parity mandatory and passed, timings `unsupported` with the reason.
- [ ] A `browser-firefox` folder with no recording fails the gate.
- [ ] No capture tolerance, no `ALLOWED_CONSOLE_ERRORS` entry and no
      sound limit was loosened to pass a browser.
- [ ] The new ADR is accepted and ADR-0053 points to it.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`xtask`): an `Untimed` platform's cells (mandatory capture and
  parity, a missing recording failed, timings `unsupported`), and that
  `budget.ron` parses with `browser-firefox`.
- Unit (Python): `capture_test.py` for the browser choice and the
  folder it writes; the `BROWSER_GAPS` rule (a listed check that passes
  fails) as a small test beside `check.py`, run by the `web-browsers`
  job.
- Integration: the `web-browsers` cells and the sound gate's Firefox
  recording, in CI on the PR. Name the run in the completion notes.
- Negative: each cell's fixtures and the two copies that must fail.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
