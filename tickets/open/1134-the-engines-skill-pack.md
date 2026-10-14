---
id: "1134"
title: "The engine's skill pack: Agent Skills for making games on the engine, tested in CI and shipped as a plugin"
type: feature
milestone: "Engine: public"
model: opus-5.5
effort: xhigh
status: todo
blocked_by: ["1129"]
nick_input: none
completed:
---

# 1134 — The engine's skill pack

## Context

Nick, 2026-10-14: "Add plenty of shippable skills to it, similar to how
https://github.com/morluto/rea became huge by adding ai skill wrappers
around pre-existing tools."

ADR-0005 §2–§3 sets the shape. Each skill is a `SKILL.md` folder in the
open Agent Skills format, with:

- when to use it, and when not to;
- the steps;
- the contract operations it calls;
- reference files that load on demand.

The research found that no game engine ships official skills yet.
Remotion's official skills and plugin are the closest model.

## Nick input

None.

## Scope

**In:**
- `skills/` (in the engine repository after 1131) holds the 15
  skills listed in ADR-0005 §3, from `shuyi-new-game` to `shuyi-net`.
  `shuyi-net` waits for 1140 and says so in its body until then.
- Each skill is generalized from this repository's practice and never
  copies our game's content. Sources: `run-gates`, `keyboard-input`, the
  ADRs named in the table.
- Each skill has a worked example as a script of contract operations.
  CI runs every example against a fresh `shuyi new` game. If an example
  fails, the skill is wrong.
- `shuyi-cli check-skills` checks four things:
  - frontmatter (`name`, `description` with when and when-not, a
    version);
  - every operation a skill names exists in the contract;
  - every file a skill links exists;
  - the length is within the format's guidance.
- Packaging:
  - a Claude Code plugin (a marketplace entry in the engine repository);
  - the layout the common skill installers read;
  - `shuyi new` copies the skills into new projects.

**Out (do not do):** skills for our game (they stay here).

## Implementation steps

1. Write `check-skills` and its tests.
2. Write the skills and their worked examples, three or four per PR if
   the diff grows past one PR's worth (split into follow-up tickets).
3. Do the packaging, and add a docs page (1132).

## Acceptance criteria

- [ ] `check-skills` passes on every skill.
- [ ] CI runs every worked example green.
- [ ] Installing the plugin in Claude Code lists the skills (manual
      check noted in the PR).
- [ ] All gates pass.

## Tests required

- Unit: `check-skills`.
- Integration: worked examples in CI.

## Completion notes
