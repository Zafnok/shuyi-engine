---
id: "3354"
title: "The game's identity on the web: its icons beside the page, and the page's title from app.name"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["3049"]
nick_input: none
completed:
---

# 3354 — The game's identity on the web

## Context

Split from 3049 by its own size rule: past about 600 changed lines, the
web part of "the game's identity in every package" goes to a ticket of
its own, and hello-grid's `web.title` stays until it lands. 3049 wrote
the desktop part:

- **From 3049 (done):** `crates/cli/src/export/identity.rs` holds
  `Identity::of(part, root, version)`, `icons(identity, sizes)` (a PNG at
  each size no larger than the source: the picture `app.icon_sizes` draws
  for a side, else downscaled from `app.icon`, ADR-0061; empty without an
  icon; a bad file is an error naming its path) and `ico(pictures)` (a
  PNG-in-ICO writer). The Windows, macOS and Linux packages carry the
  identity; the web build doesn't.

What the web build has (read on 2026-11-27):

- The web shell (`crates/cli/web/index.html`) has `<title>%TITLE%</title>`,
  then `<link rel="license" href="NOTICES.txt">` (3066), and no icon.
- `fill` in `crates/cli/src/web.rs` (called by `package`, which `shuyi
  web` and `shuyi export web` both reach) writes `%TITLE%` from
  `web.title` (`Web::title`, a `String`, empty when unset) unescaped, and
  `notices::write(dist, &rules.title, works)` takes the same title for
  `NOTICES.txt`'s heading (3039). `crates/cli/web/README.md` says so.
- `ExportCx::of` (`crates/cli/src/export.rs`) takes `title` from
  `web.title` too, else the program's name.
- hello-grid's `shuyi.ron` has `web.title: "Hello, Grid"`, the same as
  its `app.name`: the name's second copy.

Why it matters: a browser tab with the game's icon and name, written
once in `app`. The end goal (Nick, 2026-10-19, `docs/decisions.md`): an
agent ports a game to many targets from one file, in few lines.

## Nick input

None.

## Scope

**In:**
- `web::run` and `package` take `Option<&AppPart>` beside `rules`;
  `lib.rs`'s `web` passes `project.config.app.as_ref()`, and the web
  exporter (`crates/cli/src/export/web.rs`) passes `cx.app.as_ref()`.
- `favicon.ico` (16, 32 and 48 px, through 3049's `icons` and `ico`) and
  `apple-touch-icon.png` (180 px) beside the page, and a `%ICONS%`
  placeholder in `index.html`'s `<head>`, after the `<title>`, that
  becomes `<link rel="icon" href="favicon.ico">` and
  `<link rel="apple-touch-icon" href="apple-touch-icon.png">`, or nothing
  without an icon. The license link (3066) stays, after the icons'.
- **The title from the name:** `Web::title` becomes `Option<String>`.
  Without it the page takes `app.name`; with neither, `web` fails with
  "shuyi.ron has no `web.title` or `app.name`". The title is written with
  `&`, `<`, `>` and `"` escaped. `NOTICES.txt`'s heading and
  `ExportCx::title` take the same title.
- hello-grid's `web.title` goes, since its `app.name` is the same.
- `crates/cli/web/README.md` and `web.rs`'s module docs say where the
  title comes from and what `%ICONS%` becomes. `tools/export-check/web.txt`
  is unchanged for hello-grid (no icon); CI's `export` job's `web` leg
  exports the fixture game Lune Café (`tools/export-check/lune-cafe`,
  3049) for `web` in `tools/export-check/identity.sh` and checks both
  icons are there and `index.html` links them.
- A `Breaking:` line for `Web::title`'s type, for code that builds a
  `Web` by hand; a `shuyi.ron` with `web.title` reads as before.

**Out (do not do):**
- a web app manifest and an install script;
- the shell's progress bar and error panel (3052): whichever lands second
  keeps the other's parts of `index.html`;
- an icon for hello-grid (3049's Out).

**Estimated size:** about 400 lines (code 150, tests 180, docs 70).

## Implementation steps

1. Read `crates/cli/src/web.rs` (`run`, `package`, `fill`), the shell's
   `index.html` and README, `export/web.rs`, and 3049's `identity.rs`.
2. `Web::title: Option<String>`, the title's fallback and its escaping,
   in one function the page, `NOTICES.txt` and `ExportCx::title` share.
3. The icons and `%ICONS%`.
4. hello-grid's `web.title`, the README, the module docs, the CI check.
5. Run the gates.

## Acceptance criteria

- [ ] `shuyi web` on the fixture game writes `favicon.ico` and
      `apple-touch-icon.png` and links them. Without an icon it writes
      neither, and no `<link rel="icon">`.
- [ ] Without `web.title` the page's title is `app.name`, escaped; with
      neither, `web` fails with the message above.
- [ ] `NOTICES.txt`'s heading is the page's title.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `%ICONS%` with and without an icon; the title's fallback and
  escaping (`&`, `<`, `>`, `"`); the failure with neither.
- Integration: `package` on a project with an icon (both files, both
  links); CI's `web` leg on Lune Café.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
