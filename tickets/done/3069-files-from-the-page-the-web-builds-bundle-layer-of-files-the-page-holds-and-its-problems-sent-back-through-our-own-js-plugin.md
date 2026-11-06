---
id: "3069"
title: "Files from the page: a web build lays the files its page holds over its own, as a bundle layer, and sends its problems back, through our own JS plugin"
type: feature
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-06
---

# 3069 — Files from the page

## Context

Nick, 2026-10-28 ([`docs/decisions.md`](../../docs/decisions.md),
*pictures in the book, a playground, and a site that keeps up with the
tickets*): the website gets a playground, where a reader edits a game's
data files beside it and runs it again with no install. 2814 builds the
page. This ticket is what the game needs for it: a web build that reads
files the page hands it in place of its own, and tells the page what
was wrong with them.

What there is:

- **Bundle layers** (`crates/assets/src/bundle.rs`, ADR-0004 §1): a
  game reads its files from layers, in order, and a file comes from the
  first layer that has it. `Layer::owned(name, files)` takes files
  handed in at run time. On native, `shuyi_app::read_folder`
  (`crates/app/src/folder.rs`) reads a folder into one (a player's mod,
  a level they made). Its docs say "Native only: the web has no
  folders." The web has no way in.
- **Our own JS plugins**: `crates/cli/web/gamepad.js` gives the web build
  the browser's Gamepad API, declared in `crates/app/src/pads/web.rs`
  with a version the loader checks (a `shuyi-cli` test keeps the two
  equal), and `crates/cli/src/web.rs` puts it in every web build's
  `dist`. The web build uses macroquad's loader, not `wasm-bindgen`
  (game ADR-0034): a browser API is reached through a plugin of our own.
  `quad-storage.js` shows how a string crosses (`js_object`,
  `get_js_object` from `sapp_jsutils.js`).
- **Hello, Grid** (`examples/hello-grid/`) embeds `files/` and reads
  them as `layer()` and `file(path)` (`src/lib.rs`). `Hello::load_from`
  returns every problem found (`Vec<ContentError>`), not the first.
  Its web build plays on the site at `/examples/hello-grid/`.

The page hands the files over in **`sessionStorage`**, under
`shuyi-page-file:<path>`. Only a page of the same origin can write it,
it lives as long as the tab, it survives a reload (the playground runs
the game again by reloading its frame), and a read is synchronous, so
the game has its files before its first frame. Problems go the other
way with `postMessage` to the parent frame, at the page's own origin.

## Nick input

None.

## Scope

**In:**
- **`crates/cli/web/page-files.js`**, our own plugin, beside
  `gamepad.js` and built into `dist` the same way (`web.rs`'s list and
  its notices' `Origin::Engine`), with its version:
  - `shuyi_page_file_count()`: how many `sessionStorage` keys start with
    `shuyi-page-file:`; `shuyi_page_file_path(i)` and
    `shuyi_page_file_text(i)`: the `i`th one's path (after the prefix)
    and text, as `js_object`s, in key order. When `sessionStorage`
    throws (a page with storage turned off), the count is 0.
  - `shuyi_page_report(text)`: when the page is in a frame whose parent
    has the same origin, `parent.postMessage({ kind: "shuyi-report",
    text }, location.origin)`; else `console.info(text)`. An empty
    text means "no problems".
- **`shuyi_app::page`** (`crates/app/src/page.rs`):
  - `page_layer(name: &str, limits: PageLimits) -> Result<Option<Layer>,
    PageError>`: on `wasm32`, the page's files as `Layer::owned`, or
    `None` when the page holds none; on every other target, always
    `Ok(None)`. `PageLimits` (per file and in all, 256 KiB and 1 MiB by
    default) is `#[non_exhaustive]` with `PageLimits::new` (ADR-0039).
  - The work is in a function every target compiles and tests:
    `layer_from(name, entries: Vec<(String, String)>, limits)`.
    `PageError` (`#[non_exhaustive]`) says which file is too big, that
    the files are too big in all, or the path the layer refused.
  - `report(text: &str)`: on `wasm32`, calls `shuyi_page_report`; on
    every other target, does nothing.
  - The `unsafe extern` block and the exported version function follow
    `pads/web.rs`, with a `PLUGIN_VERSION` the same `shuyi-cli` test
    keeps equal to the JS's.
  - `folder.rs`'s module docs point at `page_layer` for the web.
- **Hello, Grid lays the page's files over its own** in its web build:
  - every file it reads, through `layer()` or `file(path)`, comes from
    the page's file at the same path when the page holds one, and from
    its own otherwise. The page's layer goes first in the bundle.
  - when its data doesn't load with the page's files, it sends every
    problem's text to `report`, one a line, each with its file and
    line as `check` prints them, then starts with its own files, so
    the reader sees the game and the problems both. When it loads, it
    reports the empty text.
  - when `page_layer` fails, it reports the `PageError` and starts with
    its own files.
  - its native build doesn't change.
- **The docs** (the `document` skill): `docs/platform-backends.md` (the
  web's plugins), the web shell's README (`crates/cli/web/README.md`),
  and the book's chapter that speaks of mods and layers, if one does.

**Out (do not do):**
- the playground's page, its editors and its menu entry: 2814;
- files from the page in templates or other examples. A game that
  wants them calls `page_layer` and lays the layer;
- pictures, sounds or other binary files from the page: the page's
  files are text;
- a page that changes the files while the game runs: the page reloads
  the game;
- files from a URL, a drop or a file picker.

## Implementation steps

1. Read `crates/assets/src/bundle.rs` (`Layer::owned`, the order),
   `crates/app/src/folder.rs`, `crates/app/src/pads/web.rs`,
   `crates/cli/web/gamepad.js` and `quad-storage.js`,
   `crates/cli/src/web.rs` and its plugin version test, and how
   Hello, Grid's `main.rs` and `lib.rs` read its files.
2. Write `page-files.js` and add it to `web.rs`'s files and notices.
3. Write `crates/app/src/page.rs`: `layer_from` and its tests first,
   then `page_layer`, `report` and the plugin declarations. Extend the
   version test.
4. Lay the page's layer in Hello, Grid's web start and report its
   problems.
5. Build Hello, Grid for the web (`cargo shuyi web` in its folder) and
   check it in the built-in browser: with no keys set it plays as
   before; with `sessionStorage['shuyi-page-file:files/stage.map']`
   set to a changed map and a reload, the changed map shows; with a
   broken map, the console (no parent frame) shows the problem with its
   line, and the game plays its own map.
6. Run the `document` skill, then the gates.

## Acceptance criteria

- [x] `layer_from` makes a layer whose files the bundle reads over the
      game's, and refuses a file over the limit, files over the limit
      in all, and a bad path, each naming it (unit tests).
- [x] On native, `page_layer` is `Ok(None)` and `report` does nothing
      (unit test).
- [x] The plugin's version test covers `page-files.js`, and every web
      build's `dist` holds it with its notice.
- [x] Hello, Grid's web build, with no page files, plays as before.
- [x] With a changed `files/stage.map` in `sessionStorage`, the reloaded
      web build plays that map (checked by hand in the browser; say so
      in the completion notes).
- [x] With a broken one, the problem's text, with its file and line,
      reaches the console, and the game plays its own map.
- [x] The book's chapter is updated (2802), or the completion notes say
      why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/app/src/page.rs`): `layer_from` with no entries, one
  file over the game's, a new file, each limit, a bad path; `page_layer`
  and `report` on native.
- `shuyi-cli`: the plugin version test over `page-files.js`; the web
  build's file list and notices name it.
- Example: a Hello, Grid test that its data loads from a bundle with a
  layer of changed files laid first (the native side of what the web
  build does), and that a broken file gives problems with their lines.

## Completion notes

What was done:

- `crates/cli/web/page-files.js`, the `shuyi_page_files` plugin: the four
  functions the scope names, over `sessionStorage` and `postMessage`. It
  is in `SHELL` and `SHELL_NOTICES` (`Origin::Engine`), so every web
  build's `dist` holds it with its notice; `index.html` loads it after
  `sapp_jsutils.js`, whose objects carry its strings. Text crosses
  through `TextEncoder`/`TextDecoder` first, so a lone surrogate in
  storage reaches Rust as U+FFFD, never as bytes that aren't UTF-8.
- `shuyi_app::page`: `page_layer`, `report`, `PageLimits` and
  `PageError` (`TooBig`, `TooMuch`, `Layer`), and `layer_from`, which
  every target compiles and tests. `shuyi-app` takes `sapp-jsutils` on
  `wasm32` (already in the tree under `quad-storage`; MIT or Apache-2.0).
- `shuyi-cli`'s tests: the functions and the version of `page-files.js`
  against `page.rs`, its place in `index.html`, and `dist`'s files.
- Hello, Grid: `main.rs` lays `page_layer("files", …)` first when the
  data loads with it, and reports its problems or the empty text;
  `problems_over` (the check) and `lay_over_files` (what `file(path)`
  reads first) are in `lib.rs`. Its test `tests/it/page.rs`: a changed
  map is the one played, a broken one gives `files/stage.map:<line>:…`.
- Docs: the web shell's README (its files and a section of its own),
  `docs/platform-backends.md` (*Files handed in at run time*), the
  book's *Layers* in chapter 5, `shuyi-app`'s crate docs, and Hello,
  Grid's README.
- The sound replacement's give-up footprint
  (`tools/sound-parity/footprint.ron`) and `tools/export-check/web.txt`
  follow the shell's new file.

Checked by hand in the built-in browser, on `cargo shuyi web` of Hello,
Grid served on localhost: with no keys it plays as before; with a
changed `files/stage.map` (the inner walls gone) and a reload, that map
showed on the stage and the console got the empty text; with an `X` in
it, the console showed `files/stage.map:11:6: 'X' is not in the legend`
and the game played its own map. In a same-origin parent frame, the
parent got `{ kind: "shuyi-report", text }` from the page's origin.

Deviations:

- `name` is also the folder: `page_layer("files", …)` takes the page's
  keys under `files/`, at the path after it, and the layer is named
  `files`, so problems show `files/stage.map` as `cargo shuyi check`
  does. Keys elsewhere are ignored; `""` takes every key. The scope's
  key (`files/stage.map`) is a project path while the game reads
  `stage.map`, and this joins the two.
- `layer_from` returns `Result<Option<Layer>, PageError>`: `None` for no
  file, which is what `page_layer` returns.
- With no file of the page, Hello, Grid reports nothing, not the empty
  text: a plain web build would log an empty line on every start. 2814
  has this in a **From 3069 (done)** note.
- `report` and the page's files run on `wasm32` only, so the checks of
  the browser side are by hand, as the acceptance criteria say; CI's
  `web-render` boots the build with no keys.

No break: `shuyi-app` gains the `page` module, and nothing changes.

Follow-ups: none new. 2814 builds the playground on this; 2828 moves the
plugin's version on for its check.

Nick input: none.
