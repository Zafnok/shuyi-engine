---
id: "3540"
title: "The local site gate copies the examples' shots and previews before zola check, as CI's site job does"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3540 — The local site gate copies the examples' shots and previews before zola check, as CI's site job does

## Context

`cargo xtask gates` runs CI's `site` job locally as the `site` gate
(`crates/xtask/src/gates.rs`, about line 312). Its steps are
`bash site/errors.sh`, `cargo xtask site-data --site site`,
`bash site/copy-playground.sh`, `zola check --skip-external-links`,
`cargo xtask check-book` and `bash site/check-build.sh`. CI's `site` job
(`.github/workflows/ci.yml`, about line 1560) also runs
`bash site/copy-previews.sh` and `bash site/copy-shots.sh` before
`zola check`.

2813 (#604) added the book's pictures: a `shot` shortcode that reads
`site/static/shots/<example>/<name>.png` (ignored by git), which `copy-shots.sh` writes
from the checked-in `examples/<name>/shots/`. In a fresh worktree the
local gate therefore fails on a change it didn't cause, as 3435's did on
2026-11-26:

```text
Error: Failed to render content of …\site\content\learn\book\targets-and-export.md
Error: Reason: Function call 'get_image_metadata' failed
Error: Reason: `get_image_metadata`: Cannot find path: shots/hello-grid/stage.png
```

Running `bash site/copy-previews.sh` and `bash site/copy-shots.sh` by
hand first makes it pass. `check-build.sh` also compares the built
shots with the examples' (2813).

## Nick input

None.

## Scope

**In:**
- The `site` gate's steps, in CI's order: `bash site/copy-previews.sh`
  and `bash site/copy-shots.sh` after `copy-playground.sh`, before
  `zola check`.
- The gate's `Local::Under` paths: add `examples/` (any example's
  `shots/` or preview feeds the site), in place of
  `examples/hello-grid/` alone, if CI's job runs on all of `examples/`;
  otherwise keep it as CI has it.
- The comment above the gate ("the codes' pages and the playground's
  files are written before `zola check` reads them"): it names the
  shots and previews too.

**Out (do not do):**
- Changing CI's `site` job, or the scripts themselves.
- Checking the shots in under `site/static/`.

## Implementation steps

1. Read the `site` gate in `crates/xtask/src/gates.rs` and the `site`
   job in `.github/workflows/ci.yml`: list CI's steps before
   `zola check` and their paths filter.
2. Add the two steps, and fix the paths and the comment as the Scope
   says.
3. If `crates/xtask` has a test that lists a gate's steps or compares
   them with CI's, extend it; if none does, a test that the `site`
   gate's steps hold every `bash site/copy-*.sh` that CI's `site` job
   runs (read from `ci.yml`), so the next copy script can't be missed.

## Acceptance criteria

- [ ] In a fresh worktree, with no `site/static/shots/` or
      `site/static/previews/`, `cargo xtask gates` passes its `site`
      gate.
- [ ] The `site` gate's steps run every `bash site/copy-*.sh` CI's
      `site` job runs, in the same order, and a test says so.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/xtask`): the `site` gate's copy steps match CI's `site`
  job's.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
