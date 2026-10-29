# ADR-0053: The web build checked in headless Chromium: Playwright for Python, and a capture mode that logs

- **Status:** Accepted; extends ADR-0052
- **Date:** 2026-10-29
- **Related tickets:** 2019

## Context

- Every example's web build was built in CI and published by `pages.yml`,
  and nothing opened it. The shell carries fixes for errors that broke
  loading before (`var register_plugin;`, game ticket 0225; the
  `quad-storage.js` version patch), kept in the files by unit tests but
  never loaded.
- ADR-0052 checks the desktop backend against the raster through a
  capture mode, under Xvfb. The web build runs the same backend in WebGL,
  where a page can't write files or exit.
- Bevy runs its wasm examples in Chromium and Firefox with Playwright in
  CI. Playwright drives a browser from Node, Python, Java or .NET.
- 2801 chose Zola for the site to keep Node and npm out of the
  repository. GitHub's Ubuntu runners have Python 3, and no GPU.

## Decision

1. **A checker in Python.** `tools/web-check/check.py` (Python 3,
   Playwright for Python) serves a folder on `127.0.0.1`, opens it in
   headless Chromium at a given size and a device scale of 1, and records
   every console message, page error and failed request. Its modes:
   `boot` (some seconds of frames; fails on a console error, a page error,
   a failed request, or a canvas of one colour), `capture` (waits for
   `shuyi-capture: done`; fails on the same errors), and `fixtures` (its
   own three pages must fail). Console errors it lets pass are listed in
   `ALLOWED_CONSOLE_ERRORS` with their reasons.
2. **Pinned with hashes.** `requirements.in` names Playwright at one
   version; `requirements.txt` is made from it with `pip-compile
   --generate-hashes`, by hand. CI installs it with `pip install
   --require-hashes` in a venv of the runner's `python3`. Dependabot's
   `pip` entry moves them weekly. They run in CI only and are never
   shipped, so `deny.toml` doesn't cover them; their licenses are checked
   against ADR-0002 §8's denied list when they change.
3. **Software WebGL, always.** Chromium starts with
   `--use-angle=swiftshader --enable-unsafe-swiftshader`, on the runner
   and on a machine with a GPU alike, so the pixels are the same
   everywhere.
4. **The capture mode on the web** (`shuyi-app`'s `capture` feature,
   extending ADR-0052): a web build with the feature always runs the plan
   `capture.ron` beside the page, fetched as macroquad loads a file. The
   canvas is read with `get_screen_data` as the window is, and compared
   with the raster inside the page. It writes no files: each capture logs
   `shuyi-capture: ` and its line of `capture.txt` (one function makes
   both), at the error level when it fails. The run stops at the first
   failure, logs `shuyi-capture: done`, and keeps showing that frame.
5. **One CI job,** `web-render`: the fixtures, every example `pages.yml`
   publishes booted, a page without `var register_plugin;` that must
   fail, Hello, Grid's capture build, and the render card built for
   `wasm32-unknown-unknown`. It is in `ci-result`.

## Consequences

- A web build that fails to load, logs an error, asks for a missing file
  or draws nothing fails a pull request, and the renderer is compared
  with the raster in WebGL as on the desktop.
- The repository has Python and a second package ecosystem, for CI
  alone. The checker isn't a local gate.
- `shuyi-app` has a build script (`--allow-undefined` on the web, for its
  example), its `image` dependency is on every target, and its
  `tempfile` and `proptest` dev-dependencies are native only.
- Chromium alone: a bug of Firefox's or WebKit's isn't seen.

## Alternatives considered

- **Playwright for Node** — Bevy's choice; it would bring a
  `package.json` and npm into a repository that keeps them out.
- **Comparing Playwright's screenshot with the raster** — the screenshot
  goes through the compositor and the page's scaling; the canvas read in
  WebGL is the renderer's own pixels, compared as on the desktop.
- **A plan named in the query string** — reading it needs a JS plugin of
  our own and `unsafe` in `shuyi-app`; a file beside the page needs
  neither.
- **The GPU's WebGL where there is one** — the pixels would differ
  between machines, and a pass on one wouldn't say much of the other.
