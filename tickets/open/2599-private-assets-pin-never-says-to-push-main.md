---
id: "2599"
title: "`private-assets --pin` never tells you to push the private repository's main"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: ["2593"]
nick_input: none
completed:
---

# 2599 — `private-assets --pin` never tells you to push the private repository's main

## Context

`cargo shuyi private-assets --pin` refuses a checkout whose commit isn't
on the private repository's `main`, and its message says how to fix it:
"push it (`git -C <checkout> push origin HEAD:main`), then run this
again" (`crates/cli/src/private_assets.rs`, `pin`).

That is the one thing the `shuyi-private-files` skill (2593) says never
to do: a change to the private files goes through a pull request there,
and pushing its `main` puts a file removed or renamed into the next pin
move of whoever comes next. A ruleset on that `main` (ADR-0112) asks
for a pull request, but the owner may bypass it, and on a free
account's private repository there is no ruleset at all. An agent
following the message does exactly what the skill forbids. Found while
writing 2593, whose skill warns about the message ("don't push `main` as
its message suggests").

## Nick input

None.

## Scope

**In:**
- `pin`'s refusal when the commit isn't on `origin/main` says what holds
  instead: the commit isn't merged into the private repository's `main`
  yet; open or merge its pull request there, then `git -C <checkout>
  fetch origin`, `git -C <checkout> switch --detach origin/main` and run
  `--pin` again. It names no push to `main`.
- The tests of `private_assets.rs` that read the message
  (`isn't on the private repository's main branch`) read the new words.
- `skills/shuyi-private-files/SKILL.md`, step 4: the warning about the
  message goes, and the refusal is described as it now reads.

**Out (do not do):**
- Any other check of `--pin`, or a change to the fetch.

## Implementation steps

1. Change the message in `pin`.
2. The tests, and the skill's step 4.
3. A `Behaviour:` line: the refusal's text changes.

## Acceptance criteria

- [ ] `--pin` on a commit that isn't on the private `main` refuses with
      words that name a pull request and `switch --detach origin/main`,
      and no `push … HEAD:main`.
- [ ] The skill's step 4 no longer warns about the message.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the refusal's text, in `private_assets.rs`'s tests.
- The skill pack's gate (`check-skills`, the worked examples).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
