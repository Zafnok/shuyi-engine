---
id: "2067"
title: The templates' manifests stay out of cargo's scan of the engine as a git dependency
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-14
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
at the game's current pin (engine #302's merge). The build succeeds,
but prints three of these:

```text
error: invalid character `{` in package name: `{{crate_name}}`, …
 --> …\.cargo\git\checkouts\shuyi-engine-…\<rev>\templates\shuyi-default\base\crates\game\Cargo.toml:2:8
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

- [x] `cargo metadata` in a crate depending on the engine by git prints
      no `{{crate_name}}` message.
- [x] `shuyi new` from each template still builds a game (the
      `templates` job).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- The `templates` CI job, and the reproduction above recorded in the
  notes.

## Completion notes

What was done:

- Confirmed in the game's CI: run 43673490236 on its `main` (job
  `watch`, step "Report the engine PRs past the pin") prints the three
  messages, `{{crate_name}}` and twice `{{crate_name}}-rules`.
- Reproduced at the engine's `main` (95de4712): a scratch crate with
  `shuyi = { git = "https://github.com/Zafnok/shuyi-engine", rev = … }`
  and `cargo metadata` prints the same three errors, from
  `base/crates/game/Cargo.toml`, `stages/top-down-grid/crates/game/Cargo.toml`
  and `stages/top-down-grid/crates/rules/Cargo.toml`. The two workspace
  roots parse, as cargo reads them.
- Every manifest of the template (the five, roots too, so the rule is
  one) is now `Cargo.toml.tmpl`. `Template::read` gives each the name the
  game gets, `Cargo.toml`, before anything else reads the files, so the
  merge, the placeholders, `shuyi add` and the plans' golden are
  unchanged: a new game's files are the same. A template that still
  names its manifests `Cargo.toml` reads; a folder with both fails,
  naming the `.tmpl`. A test keeps every built-in manifest a `.tmpl`.
- `cargo xtask check-pins` and `check-toolchain` read the `.tmpl`
  manifests (and a plain one still).
- Docs: `docs/templates.md` (*Manifests are `Cargo.toml.tmpl`*), the
  docs map's row for the requirements, and the `bump-rust` and
  `dependency-upkeep` skills.
- After the change, the same scratch crate against this branch's commit
  (a `file://` git URL to the worktree) prints no `{{crate_name}}`
  message.

Deviations: none. `cargo fmt --all` never formatted `templates/`, and
the `templates` job checks the made game, so step 3 needed no change.

Book: none, no public API changed (the constant is crate-private), and
the book's chapters name the files a game has, which are unchanged.

No break. Behaviour: none a game sees: `shuyi new` writes the same
files. A template of a game's own may name its manifests either way.

Game: nothing to adopt, the messages go at the game's next pin move by
themselves; the game's code names no template file (searched
`gh search code --repo Zafnok/visions-of-shuyi 'templates/shuyi-default'`
and `'Cargo.toml.tmpl'`: one done ticket naming a workflow, nothing else).

Nick input: none.
