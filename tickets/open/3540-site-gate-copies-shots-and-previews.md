---
id: "3540"
title: "The local site gate copies the examples' shots and previews, as CI's site job does"
type: bug
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3540 — The local site gate copies the examples' shots and previews, as CI's site job does

## Context

CI's `site` job (`.github/workflows/ci.yml`) copies the examples'
previews (`bash site/copy-previews.sh`, ticket 2811) and shots (`bash
site/copy-shots.sh`, ticket 2813) before `zola check`. The local `site`
gate in `crates/xtask/src/gates.rs` runs `errors.sh`, `site-data` and
`copy-playground.sh`, and neither copy. So in a fresh worktree `cargo
xtask gates` fails `zola check`:

```text
Error: Failed to render content of …\site\content\learn\book\targets-and-export.md
Error: Reason: `get_image_metadata`: Cannot find path: shots/hello-grid/stage.png
```

Found by 3539 on 2026-11-26 (its diff touched `tickets/`, which runs the
gate); running `bash site/copy-shots.sh` by hand made it pass.

## Nick input

None.

## Scope

**In:**
- The `site` gate's steps in `crates/xtask/src/gates.rs` run
  `bash site/copy-previews.sh` and `bash site/copy-shots.sh` after
  `copy-playground.sh`, in `ci.yml`'s order.
- A test in `gates.rs`'s tests that every `run:` of `ci.yml`'s `site` job
  that runs a `site/*.sh` script is among the gate's steps, so the next
  copy added to CI fails it until the gate has it too.

**Out (do not do):**
- Any change to the scripts themselves or to CI's `site` job.

## Implementation steps

1. Add the two steps to the `site` gate.
2. Add the test, reading `.github/workflows/ci.yml` as the existing test
   of `ci.yml`'s jobs in `gates.rs` does.
3. From a worktree whose `site/static/shots/` and previews are absent,
   run `cargo xtask gates` and see the `site` gate pass.

## Acceptance criteria

- [ ] `cargo xtask gates` in a fresh worktree passes the `site` gate
      without a script run by hand.
- [ ] The test fails with either step taken out of the gate.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the `site` gate runs each `site/*.sh` script CI's `site` job runs.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
