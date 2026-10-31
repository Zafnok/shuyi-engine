---
id: "2559"
title: check-skills checks a project's own skills without asking for a worked example
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-25
---

# 2559 — check-skills checks a project's own skills without asking for a worked example

## Context

1. **The game ticket:** game#1192
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/1192-take-the-engines-skill-pack-and-fold-keyboard-input-and-run-gates.md`).
   The game took this pack as a copy in its `.claude/skills/`, beside its
   own skills, and ran `shuyi check-skills .claude/skills`, as
   `skills/README.md` says a game does. The pack's 15 skills passed. Each
   of the game's 13 own skills failed on one rule only: "no worked example
   (example.ron): a skill that waits for nothing has one". The game's
   follow-up, game#1194, adds the check to its gates once this lands.
2. **The job:** check every skill in a project's skills folder, the
   engine's and the project's own, and pass when each is right for what
   it is.
3. **No friction review:** a check that doesn't fit one kind of skill.
4. **The generic shape:** a worked example is a script of `shuyi`'s
   commands that `cargo xtask skill-examples` runs on a game fresh from
   `shuyi new` (ADR-0043). Only the pack's skills have one: a project's
   own skill teaches its own workflow (how it files tickets, asks its
   owner, writes its story) and there is nothing for a fresh game to run.
   So `check` asks for a worked example only from a skill of the engine's
   pack, which is the skill whose name `Pack::embedded()` has. Every
   other rule (frontmatter, `Use …` and `Not …`, `metadata.version`,
   commands, links, length, problem codes) still holds for every skill.
5. **Not tactics:** a platformer made with `shuyi new` that writes a
   `level-design` skill of its own in `.claude/skills/` fails the check
   today the same way.

## Nick input

None.

## Scope

**In:**
- `shuyi_cli::skills::check`: the worked-example rule (rule 5's second
  half) applies only to a skill the embedded pack has, by name. A skill
  that isn't the pack's and has a `waits_for` is still checked against
  the open tickets.
- `check`'s doc comment, `skills/README.md` *Checking them* and
  `shuyi-gates` (if it names the check) say that a project's own skills
  need no worked example.
- API: no break. `check(pack, against)` keeps its signature; if the
  rule needs the pack's names passed in, it is a new field of `Against`
  (`#[non_exhaustive]` with a constructor if it isn't yet, ADR-0039).
- The game's PR that moves the pin follows this one (game#1194).

**Out (do not do):**
- The other rules: a project's own skill keeps every one of them.
- Running a project's own worked examples (`skill-examples` is the
  engine's).

## Implementation steps

1. In `crates/cli/src/skills.rs`, `check_skill`: ask for `example.ron`
   only when `Pack::embedded().skill(&skill.name)` is `Some` (compute the
   names once in `check`, not per skill).
2. Update `check`'s doc comment (rule 5) and `skills/README.md`.
3. Tests in `crates/cli/src/skills/tests.rs`: a pack with a skill the
   embedded pack has and no example fails; one with a name it doesn't
   have (`level-design`) and no example passes; the second still fails
   on a missing `Not …` sentence.

## Acceptance criteria

- [x] `shuyi check-skills` on a folder of the pack's skills plus a
      project's own skill with no `example.ron` reports nothing for the
      project's skill.
- [x] A pack skill with no `example.ron` and no `waits_for` still fails.
- [x] `shuyi check-skills skills` still passes.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the three cases in step 3.

## Completion notes

- `check` asks for a worked example only from a skill whose name the
  embedded pack has (the names gathered once, in `check`); `check_skill`
  takes that as a flag. The message now says "a skill of the engine's
  pack that waits for nothing has one". A project's own skill keeps every
  other rule, and its `waits_for` is still checked against the open
  tickets.
- No new field of `Against`: the names come from `Pack::embedded()`
  inside `check`, so its signature and `Against` are unchanged.
- Docs: `check`'s rule 5 and `skills/README.md` *Checking them*.
  `shuyi-gates` doesn't name `check-skills`, so it is unchanged.
- Tests: `only_a_skill_of_the_engines_pack_needs_a_worked_example`
  (a pack skill without an example fails; `level-design` without one
  passes, still fails on a missing `Not …`, and its `waits_for` is still
  checked). Checked by hand too: the pack's 15 skills plus a
  `level-design` skill with no `example.ron` pass `shuyi check-skills`.
- The game's PR that moves the pin is game#1194.
- No break.
