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
matching tag (or commit, if no tag exists) and update this note.

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
just be re-copied from the new version of the crate.

## The engine's own `gamepad.js`

- Exposes the connected pads to `engine/crates/app/src/pads/web.rs` as
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
otherwise skips that step silently. Prints the output folder and the wasm
binary's size. Serve that folder over HTTP (any static file server; the
WASM binary must not be opened as a `file://` URL).
