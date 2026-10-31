---
id: "3066"
title: "The web build's vendored scripts point at their notices: a header line in dist, and a rel=\"license\" link on the page"
type: feature
milestone: MVP
model: implementation
effort: low
status: done
blocked_by: ["3039"]
nick_input: none
completed: 2026-10-28
---

# 3066 — The web build's vendored scripts point at their notices

## Context

3039 made `shuyi web` write `NOTICES.txt` beside the page
(`crates/cli/src/notices.rs`), with a row for each file of the web
shell in `SHELL_NOTICES` (`crates/cli/src/web.rs`): the vendored
scripts (`mq_js_bundle.js`, `sapp_jsutils.js`, `quad-storage.js`, MIT)
with their work, version, source and license text, and the engine's
own files. Its scope also had each vendored script in `dist` start
with a line pointing at that file, and the page link it with
`<link rel="license">`. Its diff passed the 600 lines its **Size** line
allowed, so those two pieces moved here, as that line said.

Why it matters: a script copied out of the page alone (a CDN cache, a
"view source" save) still says where its license is, and the page
names its notices the way browsers and crawlers read them. Nick,
2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): "license
notices in web builds" is one of the MVP's protections.

## Nick input

None.

## Scope

**In:**
- **The header.** In `web.rs`, `package` writes each file of `SHELL`
  whose `SHELL_NOTICES` row is `Origin::Vendored` with a first line
  `/*! <file>: <work> <version>, MIT. See NOTICES.txt. */` (from the
  row's `work` and `version`, and `notices::FILE`), then the committed
  file. A comment before `"use strict";` keeps the file strict, and
  `/*!` keeps it through minifiers. The engine's own files
  (`Origin::Engine`) get none. The header is added only in `dist`: the
  committed files under `crates/cli/web/` don't change, so the
  vendoring notes in `crates/cli/web/README.md` (and `quad-storage.js`'s
  one local patch) stay true.
- **The link.** `crates/cli/web/index.html` gains
  `<link rel="license" href="NOTICES.txt">` in its `<head>`, after the
  `<title>`.
- **Docs.** `crates/cli/web/README.md`: its *Files* list says the
  vendored scripts get the header in `dist`, and its *Notices* section
  says `index.html` links the file.
- **Either order with 3052:** both change `index.html`. If 3052 is
  done, keep its markup and `shell.js`, and put the link in the
  `<head>` beside its parts; if not, nothing more to do (3052 keeps the
  link, its clause says so).
- **Either order with 3049:** both change `index.html`'s `<head>`. If
  3049 is done, keep its icon links and put the license link after
  them; if not, nothing more to do (3049 keeps the link, its clause
  says so).
- **Either order with 3068:** if 3068 is done, `SHELL` also holds the
  engine's own `clock.js` (`Origin::Engine`): it gets no header, and
  the test that `gamepad.js` is `shell(name)` alone checks `clock.js`
  the same way; if not, nothing to do (3068 adds it as
  `Origin::Engine`, which gets no header).

**Out (do not do):**
- what `NOTICES.txt` lists, or the table's rows (3039, done; 2223 for
  the credits; 3004 for other packages);
- a header in the engine's own files;
- changing the committed scripts under `crates/cli/web/`.

## Implementation steps

1. In `crates/cli/src/web.rs`, add `fn shipped_shell_file(name, text)
   -> String` beside `shell_works`: the header and `text` for a
   vendored row, `text` as it is otherwise. `package` calls it for each
   file of `SHELL` but `index.html`.
2. Add the link to `crates/cli/web/index.html`.
3. Update the tests in `web.rs`:
   - `package_copies_wasm_and_writes_the_shell`: each vendored script
     in `dist` is its header and then `shell(name)`; `gamepad.js` is
     `shell(name)` alone; `index.html` holds the link;
   - a new test: the header's first line is a `/*!` comment that ends
     on that line, for each vendored row of `SHELL_NOTICES`.
4. Update `crates/cli/web/README.md`.
5. Run the gates.

## Acceptance criteria

- [x] `cargo shuyi web --release` in `examples/hello-grid` writes
      `mq_js_bundle.js`, `sapp_jsutils.js` and `quad-storage.js`, each
      starting with its `/*!` header; `gamepad.js` has none.
- [x] The committed copies under `crates/cli/web/` are unchanged
      (`git diff` shows only `index.html` and `README.md` there).
- [x] `dist/web/index.html` has `<link rel="license" href="NOTICES.txt">`
      in its `<head>`.
- [x] Hello, Grid's web build still loads and runs (the page's console
      has no new error).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the header for each vendored row, and none for the engine's
  files.
- Integration: `package` writes the headers and the page's link.

## Completion notes

- `web.rs` gained `shipped_shell_file`: a file of `SHELL` whose
  `SHELL_NOTICES` row is `Origin::Vendored` gets
  `/*! <file>: <work> <version>, MIT. See NOTICES.txt. */` as its first
  line in `dist`; the engine's own files are written as committed.
  `package` calls it for every file but `index.html`.
- `index.html` links `NOTICES.txt` with `<link rel="license">` after the
  `<title>`. 3052, 3049 and 3068 were not done, so nothing else to keep;
  their clauses now say what holds.
- Tests: `package_copies_wasm_and_writes_the_shell` checks each script's
  exact bytes in `dist` (header then file, `gamepad.js` alone) and the
  link; `a_vendored_script_starts_with_a_one_line_header_and_the_engines_files_get_none`
  checks the header for each row of `SHELL_NOTICES`.
- Checked by hand: Hello, Grid's `cargo shuyi web --release` writes the
  three headers and the link, and the page loads and runs frames in the
  browser with only the console line it already had (`quad_net` unused).
- Docs: `crates/cli/web/README.md` (*Files* and *Notices*).
- No break: no public API changed. No follow-up tickets.
