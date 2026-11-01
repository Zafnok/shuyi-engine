---
id: "3080"
title: "Web export builds from clean staging and refuses stale files"
type: bug
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-31
---

# 3080 — Web export builds from clean staging

## Context

The web packager creates the configured dist directory and overwrites known files, while the web exporter copies its whole tree into the ZIP (crates/cli/src/web.rs and crates/cli/src/export/web.rs). An unrelated file left from an older build can therefore be released. This is a code-derived risk, not evidence of a live leak. A public demo must not inherit stale files.

## Nick input

None.

## Scope

**In:**
- Build the web package in a fresh staging directory and move only the current output to dist.
- Make an unexpected file in the final export an error with its path.
- Keep the configured public output usable for local web preview.

**Out (do not do):**
- A general content-disclosure policy or binary scanner.
- Changes to the desktop and Playdate exporters.

## Implementation steps

1. Refactor web::package and export/web.rs to generate and package an explicit current-file set in clean staging.
2. Preserve generated notices and required user-declared web files, then reject unrelated leftovers.
3. Add an export regression fixture with an old harmless canary in dist.

## Acceptance criteria

- [x] A stale canary in dist is absent from the final web ZIP and published tree.
- [x] Required current files and notices remain present and the web build runs.
- [x] The book's chapter is updated (2802), or the completion notes have a Book: line saying why not.
- [x] All gates in the run-gates skill pass.

## Tests required

- Integration: export twice with a stale file inserted between runs; inspect the second ZIP and run the web check.
- Unit: unexpected-file error names the path.

## Completion notes

`shuyi web` now makes its package in `<web.dist>.staging` beside the folder,
then replaces everything in `web.dist`. A failed build leaves the last one
as it was. `web.dist` itself is kept, not swapped: a server running in it
keeps serving (checked on Windows with `http.server` running inside the
folder, rebuilt under it, and the page booted in a browser with no console
error). `export web` compares its folder with the files this build makes
(`current_files`), refuses any other by its path, and leaves no package.

Deviations and additions the ticket didn't name:

- Replacing a folder deletes what is in it, so a `web.dist` holding the
  project, the binary, the music or voice sources, a `.git` or a `Cargo.toml`
  is refused, before anything is touched.
- An export folder inside `web.dist` is refused (`SH1002`): with `dist: "dist"`
  the default `dist/export/web` would be deleted by the build it holds.
- A failed `wasm-opt`'s `.wasm.opt` no longer stays in the package.
- The integration test uses a stand-in binary through the real packager and
  exporter code, as this file's other package tests do. The real build ran by
  hand: a canary planted in Hello, Grid's `dist/web` was gone from the folder
  and the zip, and `tools/export-check/check.sh web` passed. CI's `web-render`
  and `export` jobs run the rest; `check.py` isn't one of the local gates.

A file copied in after `shuyi web`, as CI does with `capture.ron`, stays a
matter of that folder: an export rebuilds first and doesn't carry it.

No break: `web::run` and the other public items keep their shapes.

Handed to 2528: its watcher skips the staging folder.
