---
id: "2067"
title: The templates' manifests stay out of cargo's scan of the engine as a git dependency
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2067 — The templates' manifests stay out of cargo's scan of the engine as a git dependency

## Context

A game depends on this repository by a git revision. Cargo then reads
every `Cargo.toml` in the checkout to find the packages. The templates'
manifests are not valid as they stand: for example
`templates/shuyi-default/base/crates/game/Cargo.toml` has `name =
"{{crate_name}}"`. The game's retro of 2026-11-08 (game PRs #572–#580)
found that this prints misleading `{{crate_name}}` errors in the game's
CI logs. A reader takes them for real failures, and real warnings can
hide among them. Seen again on 2026-11-08 building the game's `xtask`
at pin `c1ba9ec`. The build succeeds, but prints three of these:

```text
error: invalid character `{` in package name: `{{crate_name}}`, …
 --> …\.cargo\git\checkouts\shuyi-engine-…\c1ba9ec\templates\shuyi-default\base\crates\game\Cargo.toml:2:8
```

## Nick input

None.

## Scope

**In:**
- Confirm the messages in a game build log (`gh run view --log` on a
  recent game CI run, searching for `crate_name`).
- Keep the templates' manifests out of the scan. For example, name the
  template files `Cargo.toml.tmpl` and have `shuyi new` write
  `Cargo.toml`, or anything else cargo documents as skipped. Pick the
  way that keeps `shuyi new`, the `templates` CI job and the skill
  pack's examples working.

**Out (do not do):**
- Changing what a new game's files contain.

## Implementation steps

1. Reproduce (already seen above; confirm on the branch): a scratch crate with `shuyi-engine = { git = "…", rev =
   "<main>" }` and `cargo metadata`, then read the warnings.
2. Rename or move, and update `shuyi new`'s template reader (`grep -rn
   "Cargo.toml" crates/cli/src/new*`) and the `templates` job.
3. Make sure the template files are still formatted and checked:
   `templates/` isn't formatted by `cargo fmt --all`, and CI's
   `templates` job checks them.

## Acceptance criteria

- [ ] `cargo metadata` in a crate depending on the engine by git prints
      no `{{crate_name}}` message.
- [ ] `shuyi new` from each template still builds a game (the
      `templates` job).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The `templates` CI job, and the reproduction above recorded in the
  notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
