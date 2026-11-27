---
id: "3540"
title: "The local site gate copies the examples' shots and previews before zola check, as CI's site job does"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-26
---

# 3540 — The local site gate copies the examples' shots and previews

## Context

Found while working 2243 (2026-11-26). The `site` gate of `cargo xtask
gates` (`crates/xtask/src/gates.rs`, the `gate("site", …)` entry) runs
`bash site/errors.sh`, `cargo xtask site-data --site site` and `bash
site/copy-playground.sh`, then `zola check --skip-external-links`. CI's
`site` job (`.github/workflows/ci.yml`) runs two more copies before
`zola check`: `bash site/copy-previews.sh` (ticket 2811) and `bash
site/copy-shots.sh` (ticket 2813). In a fresh worktree,
`site/static/shots/` is empty, so every chapter with a `shot` shortcode
fails the local gate:

```text
Error: Failed to render content of …\site\content\learn\book\targets-and-export.md
Error: Reason: Failed to render shot shortcode
Error: Reason: `get_image_metadata`: Cannot find path: shots/hello-grid/stage.png
```

and every gate after `site` is "not run". CI passes, so the local gate
fails a change CI passes. Running `bash site/copy-shots.sh` by hand
before `cargo xtask gates` works around it. The gate's comment says it
runs the steps "As CI".

## Nick input

None.

## Scope

**In:**
- `crates/xtask/src/gates.rs`: the `site` gate runs `bash
  site/copy-previews.sh` and `bash site/copy-shots.sh` after `bash
  site/copy-playground.sh` and before `zola check`, in CI's order.
- Its `local` paths include `examples/` (each example's `shots/` and
  `preview.png`), not only `examples/hello-grid/`, if a test shows the
  gate otherwise skips a change to another example's shots.
- A unit test in `gates.rs`'s tests that the `site` gate's steps before
  `zola check` include every `bash site/copy-*.sh` script in `site/`
  (read from the directory), so a new copy script can't be left out
  again.

**Out (do not do):**
- Changing CI's `site` job, or the scripts themselves.
- Any other gate.

## Implementation steps

1. Read `gates.rs`'s `gate("site", …)` and CI's `site` job.
2. Add the two steps in CI's order.
3. Add the test over `site/copy-*.sh`.
4. In a fresh worktree (empty `site/static/shots/`), run `cargo xtask
   gates` and see the `site` gate pass.

## Acceptance criteria

- [x] In a worktree with no `site/static/shots/`, the local `site`
      gate passes on `main`.
- [x] The test fails if a `site/copy-*.sh` script is missing from the
      gate's steps.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the copy scripts all run before `zola check`.

## Completion notes

- `crates/xtask/src/gates.rs`: the `site` gate runs `bash
  site/copy-previews.sh` and `bash site/copy-shots.sh` after `bash
  site/copy-playground.sh` and before `zola check`, in CI's order, and
  its comment says so.
- Its `local` paths take `examples/` in place of
  `examples/hello-grid/`: a test (`an_examples_pictures_run_the_site_gate`)
  shows a change to `examples/snake/preview.png` or
  `examples/snake/shots/` now runs the gate, which the old prefix
  skipped. A change to an example's code now runs the site gate too, a
  few seconds of zola.
- `the_site_gate_runs_every_copy_script_before_zola_check` reads
  `site/copy-*.sh` and checks each runs before `zola check`; it fails
  with `copy-shots.sh` left out (checked by hand).
- The `run-gates` skill's site steps list the two copies too.
- In this fresh worktree (no `site/static/shots/`), `cargo xtask gates`
  passed the `site` gate.

Deviations: none.

Book: none, the change is to the local gates only; no public API, page or
chapter changes.
Skills: none, no schema, template, format or problem code changed (the
`run-gates` skill under `.claude/skills/` is updated, not the shipped
pack).
