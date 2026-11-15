---
id: "2085"
title: "Before the first release, the target profiles, packs, language packs, credits and bot settings that open tickets grow move under ADR-0039"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2085 — The assets' and the platform's data types that open tickets grow move under ADR-0039 before the first release

## Context

The fourth of five tickets that move, before 2005, every exhaustive
public type an open ticket plans to grow (2082 has the reasons and
Nick's words of 2026-11-14). These are data types, most of them read
from files (ADR-0039 kind 5). Checked exhaustive on 2026-11-14:

| Type | Where | Grown by |
| ---- | ----- | -------- |
| `TargetProfile` | `crates/assets/src/target.rs:57` | 3017, 3023 |
| `Status` (a target's) | `target.rs:82` | the export targets' tickets |
| `Display` | `target.rs:92` | 3017 |
| `Color` | `target.rs:131` | 3017 |
| `Input` (a target's) | `target.rs:178` | 2406, 3021, 3024 |
| `Budgets` | `target.rs:203` | 3009, 3020 |
| `PackInfo` | `crates/assets/src/pack.rs:104` | 3059 |
| `LangInfo` | `crates/assets/src/lang.rs:175` | 2346 |
| `PackRules` | `lang.rs:520` | 2241 |
| `SourceLine` | `crates/assets/src/lang/source.rs:36` | 2241 |
| `FileCredit` | `crates/assets/src/credits.rs:138` | 2223 |
| `CreditsFile` | `credits.rs:198` | 2223 |
| `PlanSettings` | `crates/core/src/bots/plan.rs:48` | 2133 |
| `Presented::Game` (a variant) | `crates/platform/src/headless.rs:47` | 2333, 2346 |

`RolloutSettings` (`crates/core/src/bots/rollout.rs:82`) is already
`#[non_exhaustive]`. 2241 says its new fields can't take the attribute.
This ticket adds the attribute, and its PR rewrites that line of 2241.

## Nick input

None.

## Scope

**In:**
- Each type in the table under ADR-0039. Data read from files takes
  `#[serde(default)]` on every later field, so files keep loading. A
  type games build in code gets its `new`; `Presented::Game` becomes a
  `#[non_exhaustive]` variant with a constructor.
- Every engine, template, example and tutorial-step call site.
- The listed tickets' planned `Breaking:` lines for these types become
  additive, and 2241's line about the attribute is rewritten.
- A `Breaking:` line in the completion notes, after a search of the
  game for literals of each type.

**Out (do not do):**
- Any field the listed tickets add; `pack::VERSION` (a baked pack's
  format, not a type).
- 2133's `deny_unknown_fields`, a change to which files load, which
  2133 makes itself before the release.
- `shuyi-cli`'s types (2086).

**Estimated size:** about 700 lines (code 350, tests 200, docs 150).

## Implementation steps

1. The attributes and constructors, file by file.
2. Build the workspace, the templates and the tutorial steps, and fix
   each literal and exhaustive match. `shuyi.ron`'s targets and the
   engine's target profiles must load unchanged.
3. Edit the listed tickets' planned `Breaking:` lines, and 2241's.

## Acceptance criteria

- [ ] Every type in the table is `#[non_exhaustive]`, and a test outside
      its crate builds each one that games build through its
      constructor.
- [ ] Every target profile, language pack, credits file and bot
      settings file in the repository and the templates loads
      unchanged.
- [ ] `cargo xtask api` lists each change, and the `Breaking:` line
      names each type.
- [ ] No open ticket still plans a `Breaking:` line for one of these
      types.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the constructors build what a literal built before.
- The existing target, pack, language and credits tests pass unchanged.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
