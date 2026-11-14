---
id: "2065"
title: "A merge commit brings no file of its own, and `.codex/config.toml` is tracked on purpose or not at all"
type: infra
milestone: After MVP
model: haiku-4.5 / gpt-6-luna
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-14
---

# 2065 — A merge commit brings no file of its own, and `.codex/config.toml` is tracked on purpose or not at all

## Context

#279 committed a local `.codex/config.toml` by mistake and removed it
again in the same PR. It came back through a merge of `main` into #285's
branch (`932f7173`), where it is in neither parent. It reached `main` in
#285's squash without anything naming it, and is tracked today. The
game's #558 committed one by mistake too. `AGENTS.md` says not to alter
another harness's settings, so a harness's local settings file reaching
`main` unnoticed is exactly what should be caught. The retro of
2026-11-08 found both. #277 also hid an 18-line code fix inside a merge
commit.

## Nick input

None.

## Scope

**In:**
- Decide whether `.codex/config.toml` belongs in the repository. If it
  holds this repository's shared Codex settings (as `.claude/settings.json`
  does for Claude Code), keep it and say so in `AGENTS.md`. If it is one
  machine's local file, remove it and add `.codex/` to `.gitignore`
  (keeping any shared file the Codex loaders need).
- A CI check on PRs: a merge commit on the branch that adds or changes a
  file beyond what resolving its two parents needs (`git diff --cc` /
  `git show --remerge-diff`) fails, naming the file.

**Out (do not do):**
- Changing Codex's settings themselves.

## Implementation steps

1. Read `.codex/config.toml` and the `.agents/skills` loaders, then
   decide. Record the reason in the notes.
2. Add the merge check as a small xtask command over `git rev-list
   --merges base..HEAD`, using `git show --remerge-diff <merge>`. A
   non-empty result outside conflict hunks is the error.
3. Run it in the `tickets` job.

## Acceptance criteria

- [x] `.codex/config.toml` is either documented as shared or removed and
      ignored.
- [x] A test branch whose merge commit adds a file fails the check.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Integration: a temporary repository with a clean merge and a merge
  that adds a file.

## Completion notes

- **`.codex/config.toml` stays, documented as shared.** It holds no
  machine's own setting: it registers the repository's `shuyi` MCP server
  for Codex (`cargo run -q -p shuyi-cli -- mcp`, the same command as the
  root `.mcp.json` for Claude Code) and the `MCP_TIMEOUT` its first build
  needs. Removing it would leave Codex sessions without the session tools
  the docs map tells every ticket to use. `AGENTS.md` (*Shared
  instructions and skills*) now names each harness's tracked settings, and
  the docs map's MCP row names the file. The `.agents/skills` loaders
  don't read it.
- **`cargo xtask check-merges [--base <rev>] [--head <rev>]`**
  (`crates/xtask/src/merges.rs`): for each merge in `base..head`
  (`origin/main..HEAD` by default), `git show --remerge-diff` against
  git's own remerge of the parents. A file git merged without a conflict
  that the merge changes fails, named; in a file with conflict markers, a
  run of changed lines that neither touches a marker nor lies inside a
  conflict region fails, named by its line; a whole-file conflict
  (modify/delete, rename) is its own resolution. An octopus merge fails:
  remerge-diff takes two parents.
- **CI:** the `tickets` job runs it on pull requests over `HEAD^1..HEAD^2`
  (HEAD is GitHub's merge of the PR, which is not checked) and now checks
  out the whole history (`fetch-depth: 0`), since the remerge needs each
  merge's merge base. Locally `cargo xtask gates` runs it among the
  tickets' checks, and the `run-gates` skill lists it.
- **Tests:** unit tests over a remerge diff as git 2.55 prints it, and
  temporary repositories: a clean merge passes, a merge that adds a file
  fails naming it, a bad range is an error. On this repository's history
  it names `.codex/config.toml` in `932f7173`, with the four other files
  that merge changed on its own.
- **Limit:** a change typed inside a conflict region passes; nothing can
  tell it from the resolution.

Book: none, a check of this repository's own history; no engine API, format or rule a game sees changes.

No break. No behaviour change a game sees.
