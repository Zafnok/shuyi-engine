---
id: "3535"
title: "The document skill names the homes sessions miss, and every retro checks the docs against what merged"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3535 — The `document` skill names the homes sessions miss, and every retro checks the docs against what merged

## Context

The docs audit of 2026-11-25 (tickets 2850, 2851, 3352, 3426–3428 and
3533) found that sessions updated the obvious home and missed the
second one:

- 2225 and 3425 updated the quick start, not the book's project
  (`site/tutorial/`);
- 3424 updated `docs/contract.md`, not the MCP tool's description
  (`mcp::description`) or its input;
- 2203 and 2234 updated `docs/formats.md`, not the skill that teaches
  importing (`shuyi-import-art`);
- chapters bolded new terms and no row went into the glossary.

The checks in 3534, 2852, 3429, 3427 and 2850 catch most of these
mechanically. Two process gaps are left:

- The `document` skill (`.claude/skills/document/SKILL.md`) says "find
  each one's row" and holds no copy of the map. That is right, but it
  names none of the second homes sessions keep missing.
- The `retro` skill (`.claude/skills/retro/SKILL.md`, §2 *Themes*) has
  six themes it always runs, and none reads the docs against what
  merged. This audit had to be asked for.

## Nick input

None.

## Scope

**In:**

- **The `document` skill:** a short list after step 2, *Homes that are
  easy to miss*. Each item is one line linking its row in the map; it is
  not a copy of the map:
  - a template change → the book's project too (`site/tutorial/`);
  - a contract op or field → the MCP tool's description and input;
  - a format → the skill that teaches it;
  - a new term in the book → the glossary;
  - a new `pub mod` → the crate doc.

  The skill stays the map's reader. The list says these are the homes
  past audits found missed.
- **The `retro` skill:** a seventh theme it always runs, *docs drift*.
  The reviewer reads each PR of the window that changed something public
  (an op, a field, a format, the template, a public item) against the
  homes the docs map names for it: the book and its project, the
  glossary, `docs/contract.md`, the MCP descriptions, the skill pack,
  `docs/formats.md`, the crate docs. It reports each home left behind,
  with the PR and file:line. Its findings become tickets the way the
  retro's others do.
- `pwsh -File scripts/sync-agent-skills.ps1`, so the Codex loaders
  follow (`-Check` passes).

**Out (do not do):**

- the mechanical checks (3534, 2852, 3429, 3427, 2850, 3533);
- a scheduled run of the retro.

## Implementation steps

1. Read both skills and `docs/README.md`.
2. Write the list and the theme.
3. Sync the loaders; run `cargo xtask check-links` and the gates.

## Acceptance criteria

- [ ] The `document` skill lists the easy-to-miss homes, each linking
      its row in the map.
- [ ] The `retro` skill runs a docs-drift theme every time, with what it
      reads and reports.
- [ ] `scripts/sync-agent-skills.ps1 -Check` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None beyond the links and the loaders' check.

## Completion notes

*(Filled in by the session that completes the ticket.)*
