---
id: "2065"
title: "A merge commit brings no file of its own, and `.codex/config.toml` is tracked on purpose or not at all"
type: infra
milestone: After MVP
model: haiku-4.5 / gpt-6-luna
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] `.codex/config.toml` is either documented as shared or removed and
      ignored.
- [ ] A test branch whose merge commit adds a file fails the check.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: a temporary repository with a clean merge and a merge
  that adds a file.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
