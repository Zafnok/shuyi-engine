---
id: "3535"
title: "The document skill names the homes sessions miss, and every retro checks the docs against what merged"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-25
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

- [x] The `document` skill lists the easy-to-miss homes, each linking
      its row in the map.
- [x] The `retro` skill runs a docs-drift theme every time, with what it
      reads and reports.
- [x] `scripts/sync-agent-skills.ps1 -Check` passes.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- None beyond the links and the loaders' check.

## Completion notes

- The `document` skill has *Homes that are easy to miss* under step 2:
  five one-line items (template → the book's project, contract op or
  field → the MCP tool's description and input, format → the skill that
  teaches it, new term → the glossary, new `pub mod` → the crate doc),
  each naming its row in the map's *People making games* table and
  linking that table. The map's rows have no anchors of their own, so
  each item links the table and names the row by its fact.
- The `retro` skill's §2 now always runs seven themes; the seventh,
  *docs drift*, says what it reads (each PR of the window that changed
  something public, against the homes the map names) and what it
  reports (each home left behind, with the PR and `file:line`, as
  proposed tickets).
- Deviation: the MCP tools' descriptions have no row of their own in the
  map; the item points at the contract's row and names
  `crates/cli/src/mcp.rs`. No ticket: 3427 keeps them in step with the
  tools mechanically, so a row would only repeat the code.
- `sync-agent-skills.ps1` rewrote no loader (the skills' names and
  descriptions are unchanged); `-Check` passes.
- Docs: the change is to two of this repository's skills, whose home is
  `.claude/skills/` itself; the map needs no new row.
