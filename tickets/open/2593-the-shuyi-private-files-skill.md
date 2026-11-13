---
id: "2593"
title: "The shuyi-private-files skill: change a game's private files in their repository and move the pin, shipped to every new game"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2590", "2591"]
nick_input: none
completed:
---

# 2593 — The `shuyi-private-files` skill

## Context

Nick, 2026-11-12 (`docs/decisions.md`, *`shuyi new` sets up a game's
repositories*): the skill the game uses to work with its private files
should ship in the template for `shuyi new`.

That skill is the game's `content-change`
(`github.com/Zafnok/visions-of-shuyi/blob/main/.claude/skills/content-change/SKILL.md`):
the checkout, a branch on the private repository made with
`--no-track` (a branch made from `origin/main` that tracks it pushes to
`main` on a plain `git push`), never pushing the private `main`, a pull
request there first and then the game's pull request moving the pin, and
what to do when two pull requests both move the pin. Its words are the
game's (`assets-private/`, `content/`, `story/`, `trpg-content`, the
bought art's credits), so it can't be copied: rule 4 of `AGENTS.md`.

2590 sets the layout of a new game's private repository (`files/`
laid over the game's files, `docs/`, `library/`; the checkout in
`private/`, the pin in `private.rev`); 2591 makes it with `shuyi
repos`.

**From 2591 (done, ADR-0112):** `cargo shuyi repos` makes, or repairs
when run again, the game's repository, the private one with its README
and folders, the pin and the checkout, two read-only deploy keys
(`<name> builds (read-only)` and `<name> dependabot (read-only)`) whose
private halves are the game's Actions and Dependabot secret
`PRIVATE_ASSETS_KEY`, and a ruleset on the private repository's default
branch (a warning, not a failure, on a free account's private
repository). `--dry-run` shows the plan, and `--new-keys` replaces both
keys. Its codes are `SH0006`, `SH1007`–`SH1009`, `SH3013` and `SH3014`
(`shuyi explain`). The engine's skill pack (`skills/`, `crates/cli/src/skills.rs`,
ADR-0043) is copied into every new game's `.claude/skills/` by `shuyi
new`, and `shuyi check-skills` checks it.

## Nick input

None.

## Scope

**In:**
- **`skills/shuyi-private-files/SKILL.md`**, in the pack's format
  (frontmatter `name`, `description`, `metadata.version`), written for
  a game made by `shuyi new` with the private part, and naming nothing
  of any game. Its sections:
  1. *What is where*: the private repository, the checkout
     `private/`, `files/` laid over the game's files whenever the
     checkout is there (no feature to turn on), `docs/` and `library/`
     never built, the pin `private.rev`; a fork or a machine without
     access builds on the public files.
  2. *The checkout*: `cargo shuyi private-assets` (and `--library`);
     without access, say so and stop.
  3. *A branch there*: `git -C private fetch origin`, `git -C private
     switch --no-track -c <branch> origin/main`, and why `--no-track`.
     Never push the private `main`, even where a ruleset would let the
     owner.
  4. *Change, commit, push, pull request* in the private repository
     (`gh pr create --repo <the private repository>`), then `cargo
     shuyi private-assets --pin` once it is merged, and the game's pull
     request with `private.rev`; while it isn't merged, the game's CI
     reads the old pin.
  5. *Two pull requests moving the pin*: pull the private `main`, `--pin`
     again.
  6. *Never in public*: no private file copied into the public
     repository, quoted in an issue, a pull request, a commit message
     or a public artifact; a test's log may quote a line, which is the
     price of testing the real files.
  7. *Setting up*: `cargo shuyi repos` (2591) makes or repairs the
     repositories, keys and secrets; it acts on the developer's GitHub
     account, so **ask the developer before running it**, and never run
     it on your own.
- **A worked example** (`example.ron`, the pack's script of contract
  commands) if the contract can show part of it (a file laid in a
  fixture checkout changing what `scene` shows); else
  `metadata.waits_for` with the reason, as the pack's README allows.
- **The template's `AGENTS.md`** names the skill inside 2590's private
  part's text (a game without the part doesn't see it). **From 2590
  (done):** the composer appends a part's Markdown to the base's, and
  the private part's section is
  `templates/shuyi-default/private-files/AGENTS.md` (*The private
  files*): name the skill there.
- `skills/README.md`'s list gains the skill; `shuyi check-skills`
  passes.

**Out (do not do):**
- The game's `content-change` skill and the game's adoption of this
  one: a game ticket, if the game wants it.
- Credits for bought files (the existing `private_assets.credits` part
  and its checks in `--pin`).
- Any change to `shuyi private-assets` or `shuyi repos`.

## Implementation steps

1. Read the game's `content-change` skill (link above) and 2590's ADR.
2. Write `skills/shuyi-private-files/SKILL.md` in the engine's words.
3. The worked example or `waits_for`.
4. The template's `AGENTS.md`, `skills/README.md`.
5. `cargo xtask gates` (the skill pack's worked examples are a gate).

## Acceptance criteria

- [ ] `shuyi new g --template blank --set private-files=yes
      --private-url https://github.com/o/g-assets.git` makes
      `.claude/skills/shuyi-private-files/SKILL.md`, and the game's
      `AGENTS.md` names it.
- [ ] The skill names no game, class, character, story or game file;
      every `shuyi` command it names exists (`check-skills`).
- [ ] It says to ask the developer before `cargo shuyi repos`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `shuyi check-skills` over the pack (a gate).
- The template's tests: the skill is in a game made with the part.
- The worked example, if there is one, runs in the skill pack's gate.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
