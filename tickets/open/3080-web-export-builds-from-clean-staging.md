---
id: "3080"
title: "Web export builds from clean staging and refuses stale files"
type: bug
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] A stale canary in dist is absent from the final web ZIP and published tree.
- [ ] Required current files and notices remain present and the web build runs.
- [ ] The book's chapter is updated (2802), or the completion notes have a Book: line saying why not.
- [ ] All gates in the run-gates skill pass.

## Tests required

- Integration: export twice with a stale file inserted between runs; inspect the second ZIP and run the web check.
- Unit: unexpected-file error names the path.

## Completion notes

*(Filled in by the session that completes the ticket.)*
