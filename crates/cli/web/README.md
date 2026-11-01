# The web shell

The page a game's web (WASM) build runs in. `shuyi web` builds the game's
binary for `wasm32-unknown-unknown` and packages it with these files into
the folder `shuyi.ron` names (`web.dist`). They are built into `shuyi`
itself, so a game needs no copy of its own.

## Files

- `index.html`: the page. A full-window black `<canvas id="glcanvas">`,
  no margins or scrollbars, focused on load and on click; it loads the
  bundle, then the game's `.wasm`. `shuyi web` writes the game's title
  (`web.title`) for `%TITLE%` and its binary (`web.bin`) for `%WASM%`.
  Before the bundle it declares a global `var register_plugin;`: the
  bundle is strict-mode JS and its unused `quad_net` plugin assigns to that
  undeclared global, which otherwise throws an uncaught `ReferenceError`
  on every load. It also keeps resuming the page's audio on every key
  press, click or touch until it runs: browsers keep audio off until the
  player's gesture, and the bundle's own unlock gives up after the first
  key.
- `mq_js_bundle.js`: macroquad's JS loader, vendored (see below). It already
  calls `preventDefault` on the sokol-mapped arrow/space/tab keys inside its
  own `canvas.onkeydown` handler; `index.html` adds a small backstop
  `keydown` listener for browser shortcuts it doesn't cover (Page Up/Down,
  Home/End).
- `sapp_jsutils.js`, `quad-storage.js`: miniquad JS plugins backing
  `shuyi-app`'s storage on the web (`localStorage`). `quad-storage` exposes
  `localStorage` to Rust and needs `sapp_jsutils` (its JS↔Rust
  string/object marshalling) loaded first. Both vendored, see below.
- `gamepad.js`: controllers, a miniquad JS plugin over the browser's
  Gamepad API. The engine's own code, not vendored. See below.

In `dist`, each vendored script starts with one line naming its work
and pointing at the notices, then the committed file:
`/*! <file>: <work> <version>, MIT. See NOTICES.txt. */`
(`shipped_shell_file` in `web.rs`, from the file's `SHELL_NOTICES` row).
A comment before `"use strict";` keeps the file strict, and `/*!` keeps
it through minifiers. The committed files here stay as vendored, and the
engine's own files get no header.

## Notices

Every build writes `NOTICES.txt` beside the page (`shuyi-cli`'s
`notices` module). It lists:

- the crates compiled in: the game's package's normal dependencies for
  `wasm32-unknown-unknown`, all the way down (`cargo metadata`), with the
  license files at the top of each crate's folder. The engine's crates
  get the engine's `LICENSE-MIT` and `LICENSE-APACHE`. Another crate
  with no file gets the standard text of each license it declares that
  the tool can write (Apache-2.0, and MIT with its authors), and one
  left with none is listed anyway, with a warning;
- this folder's files, from `SHELL_NOTICES` in `web.rs`: each vendored
  script with its work, version, source and license text (the
  `*-LICENSE-MIT.txt` files here), and the engine's own files under the
  engine's licenses. A file added to `SHELL` without its row there, or a
  vendored one without its row in `THIRD_PARTY_ASSETS.md`, fails a test;
- the game's own license texts, each file whole, as `shuyi.ron`'s
  `notices` part names them (`notices: (texts: [...])`).

Then every text once, headed by the works it covers. The same inputs
give the same bytes. `index.html` links the file in its `<head>` with
`<link rel="license" href="NOTICES.txt">`, the way browsers and
crawlers look for a page's license.

## The link argument a game's binary needs

These plugins' `extern "C"` functions are written in JS and loaded into the
page at run time, not linked in. Without `--allow-undefined`, `rust-lld`
takes them for link errors instead of leaving them as wasm imports. A link
argument only reaches a binary from its own package's build script, so a
game's binary sets it in its `build.rs`:

```rust
fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        println!("cargo:rustc-link-arg=--allow-undefined");
    }
}
```

Not in `.cargo/config.toml`'s `rustflags`: a `RUSTFLAGS` variable (as CI
sets, for `-D warnings`) replaces those wholesale, which would silently
drop the flag.

A release build (`shuyi web --release`, and `shuyi export web
--release`) sets `CARGO_ENCODED_RUSTFLAGS` too: the flags already in it
or in `RUSTFLAGS`, then a `--remap-path-prefix` for the workspace,
each crate read by a path outside it, Cargo's and rustup's homes and the target folder, so the `.wasm` holds
no path of the build machine (ADR-0058). That variable also wins over a
game's `rustflags` in `.cargo/config.toml`, which these builds don't
get. The built `.wasm` is then searched for the home folder, Cargo's
home and the game's folders, and refused with `SH3009` if one is there.

## Vendored `mq_js_bundle.js`

- Source: https://github.com/not-fl3/macroquad, `js/mq_js_bundle.js`.
- Version: matches the `macroquad` crate version pinned in the workspace
  `Cargo.lock` (currently `0.4.16`).
- **No git tag `v0.4.16` exists in the macroquad repo** (its tags jump from
  `v0.4.14` straight to `v0.4.1x`+ series releases that were published to
  crates.io from untagged commits). Vendored instead from the exact commit
  crates.io recorded for that release, `5e9b5ca912ac65962c05c0da842a4a70eaae34b9`,
  found in the published crate's `.cargo_vcs_info.json` and verified
  byte-for-byte identical to `js/mq_js_bundle.js` at that commit on GitHub.
- License: MIT (macroquad is dual MIT/Apache-2.0; the license text is
  committed at [`mq_js_bundle-LICENSE-MIT.txt`](mq_js_bundle-LICENSE-MIT.txt)).

When bumping the `macroquad` crate version, re-vendor this file from the
matching tag (or commit, if no tag exists) and update this note. The
bump comes as Dependabot's `backend` group, which moves macroquad,
miniquad, the `quad-*` crates, `sapp-jsutils` and `gilrs` together (the
`dependency-upkeep` skill, ADR-0037).

## Vendored `sapp_jsutils.js` and `quad-storage.js`

- `sapp_jsutils.js`: source https://github.com/not-fl3/sapp-jsutils,
  `js/sapp_jsutils.js`. Version matches the `sapp-jsutils` crate pulled in
  transitively by `quad-storage` (currently `0.1.7`); the file ships inside
  the published crate itself (`js/sapp_jsutils.js`), so it was copied
  straight from there rather than from GitHub. License: dual MIT/Apache-2.0
  per the crate's `Cargo.toml`; upstream ships no `LICENSE` file at all, so
  the standard MIT text was reconstructed with the crate's declared author
  as copyright holder, committed at
  [`sapp_jsutils-LICENSE-MIT.txt`](sapp_jsutils-LICENSE-MIT.txt).
- `quad-storage.js`: source https://github.com/optozorax/quad-storage,
  `js/quad-storage.js`. Version matches the `quad-storage` crate (`0.1.3`),
  vendored from the exact commit crates.io recorded for that release,
  `3760b953aec17d65cc4ca8edfa39c38e7337ec3a` (from the published crate's
  `.cargo_vcs_info.json`). License: MIT (dual MIT/Apache-2.0; text committed
  at [`quad-storage-LICENSE-MIT.txt`](quad-storage-LICENSE-MIT.txt), copied
  from the crate's own `LICENSE-MIT`).
  **Local patch:** the plugin's `version` is changed from upstream's
  `"0.1.2"` to `65536`, the number `quad_storage_crate_version()` in the
  `quad-storage-sys` crate (0.1.0) actually reports
  (`(major << 24) + (minor << 16) + patch`); otherwise the loader logs a
  version-mismatch error on every load. Re-apply it when re-vendoring; the
  test `quad_storage_js_version_matches_the_locked_crate` fails if it is
  lost.
- `index.html` loads `sapp_jsutils.js` before `quad-storage.js`, both before
  `mq_js_bundle.js`'s `load(...)` call.

When bumping the `quad-storage` crate version, re-vendor `quad-storage.js`
from the matching commit/tag and update this note; `sapp_jsutils.js` can
just be re-copied from the new version of the crate. Both come in
Dependabot's `backend` group too.

## The engine's own `gamepad.js`

- Exposes the connected pads to `crates/app/src/pads/web.rs` as
  its `shuyi_pad_*` functions. The two files must list the same functions
  and the same plugin version (`VERSION` here, `PLUGIN_VERSION` there):
  `shuyi-cli`'s tests fail otherwise. Bump both versions when the functions
  change.
- Only pads the browser reports with `mapping === "standard"` are listed.
  All the rules (sticks as directions, the Switch-style Confirm / Cancel
  swap, several pads as one) are in Rust.
- Browsers don't report a pad until one of its buttons is pressed while the
  page has focus.
- In an iframe, browsers only allow the Gamepad API if the frame has
  `allow="gamepad"`. If a host's frame lacks it, `getGamepads()` throws and
  the plugin reports no pads; the keyboard still works.
- `index.html` loads it after `mq_js_bundle.js` (it calls the bundle's
  `miniquad_add_plugin`) and before `load(...)`.

## Building and running

```
shuyi web [--release] [--<feature>...]
```

Builds the game's binary for `wasm32-unknown-unknown` (each `--<feature>`
turns on a feature `web.features` lists) and packages it with the shell. With
`--release` and `wasm-opt` on `PATH`, runs `wasm-opt -Oz` on the binary;
otherwise skips that step silently. Writes `NOTICES.txt` beside the page.
Prints the output folder and the wasm binary's size. Serve that folder over HTTP (any static file server; the
WASM binary must not be opened as a `file://` URL).

The package is made whole in `<web.dist>.staging` beside the folder, then
replaces everything in `web.dist`: a file left there is deleted by the next
build, and a build that fails leaves the last one as it was. The folder
itself stays, so a server running in it keeps serving. A `web.dist` that
holds the project, the binary, the music or voice sources, a `.git` or a
`Cargo.toml` is refused, since replacing it would destroy them.

`shuyi export web` runs that build, then copies the folder into its package.
It refuses an export folder inside `web.dist`, and a file in the package that
the build doesn't make (`current_files` in `web.rs`), by its path. A file
copied in after `shuyi web`, as CI does with `capture.ron`, is for checking
that folder and never reaches an export.

## Checked in CI

CI's `web-render` job (ticket 2019, [ADR-0053](../../../docs/adr/0053-the-web-build-checked-in-headless-chromium-playwright-for-python-and-a-capture-mode-that-logs.md)) opens web builds in headless Chromium
with [`tools/web-check/check.py`](../../../tools/web-check/check.py)
(Playwright for Python). It serves the folder on `127.0.0.1` and records
every console message, page error and failed request (a status of 400 or
more, `/favicon.ico` aside). In order:

1. `check.py fixtures`: the three pages in `tools/web-check/fixtures/`
   must fail, naming a console error, a missing `.wasm` and a canvas of
   one colour.
2. `check.py boot` on every example `pages.yml` publishes (each one whose
   `shuyi.ron` has a `web` part), built with `cargo shuyi web --release`:
   five seconds of frames, then it fails on a console error, a page error,
   a failed request, or a canvas of one colour.
3. The same page with `var register_plugin;` taken out must fail (the
   error game ticket 0225 fixed).
4. `check.py capture` on Hello, Grid's capture build (`cargo shuyi web
   --release --capture`, its `capture/frames.ron` beside the page as
   `capture.ron`) and on the render card (`shuyi-app`'s `render_card`
   example built for `wasm32-unknown-unknown` with `capture`, beside this
   folder's `index.html` and scripts, at `--size 768x576`). A capture
   build runs its plan, reads the canvas in WebGL and compares it with the
   raster inside the page, and logs a line per capture to the console:
   `shuyi-capture: ` and its line of `capture.txt`, an error when it fails.
   It stops at the first failure, logs `shuyi-capture: done` and keeps
   that frame on the page. `check.py` waits for `done` (two minutes at
   most) and fails on a console error.

Each page's canvas (`page.png`) and console (`console.txt`) are uploaded
as the job's artifact, and the consoles are in its summary.

What the runner needed: no GPU, so Chromium draws WebGL with SwiftShader
(`--use-angle=swiftshader --enable-unsafe-swiftshader`, always, so a
machine with a GPU gets the same pixels). Frames advance in the headless
page on their own. The captures match the raster exactly, within the
plans' own tolerance: the web needs none of its own.

A new console error is fixed, or added to `ALLOWED_CONSOLE_ERRORS` in
`check.py` with the reason it is harmless. Warnings never fail a page.

To run it yourself (Python 3; it isn't one of the local gates):

```bash
python3 -m venv .venv
.venv/bin/pip install --require-hashes -r tools/web-check/requirements.txt
.venv/bin/python -m playwright install chromium
.venv/bin/python tools/web-check/check.py boot examples/hello-grid/dist/web
```

(`.venv\Scripts\` on Windows.) `--out <dir>` says where `page.png` and
`console.txt` go (`target/web-check/<mode>` unless given), `--seconds N`
how long `boot` runs, `--size WxH` the page's size (`784x784`, Hello,
Grid's window, unless given). `tools/web-check/requirements.txt` is made
from `requirements.in` by hand with `pip-compile --generate-hashes
--strip-extras requirements.in` (pip-tools), every package pinned with its
hashes; Dependabot moves them weekly.
