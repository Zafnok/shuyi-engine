# ADR-0108: A `Behaviour:` line for a change a game sees without a break

- **Status:** Accepted
- **Date:** 2026-11-11
- **Related tickets:** 2066, game#1254 (the game's pin review reads the
  line)
- **Amends:** [ADR-0038](0038-the-public-api-gate-a-listing-diff-and-semver-checks-on-every-pr-and-a-breaking-line-for-each-break.md)
  §6 (the `Breaking:` line gets a sibling)

## Context

ADR-0038's `Breaking:` line and CI's `api` job cover the public API's
signatures, file formats and golden values. The retro of 2026-11-08
found that the changes which surprised the game at pin time were
behaviour, with no signature changed:

- 2350 loosened the `.dlg` parser and said "nothing in the game must
  change"; game ADR-0045 keeps `.dlg` text ASCII, and the game's pin PR
  wrote about 45–80 lines to keep that rule.
- 2226 (#276) said "no break" and changed `lang-status` labels and panic
  text the game's tests pinned.
- 2315 and 3210 changed the game's tests and snapshots.

The game's pin review (its `engine-pin` skill, through the watcher of
ADR-0045) reads `Breaking:` lines from each engine PR's body and its
ticket's completion notes. A behaviour change has no line, so the game
finds it when a test fails, or never.

## Decision

1. **The line.** A ticket whose change a game can see without a break of
   the public API writes, under `## Completion notes`, one line per
   change, in `Breaking:`'s shape:

   ```text
   Behaviour: <crate>[, <crate>…]: <what a game sees change>
   ```

   It is for a validation loosened or tightened, a changed default, an
   error's code or message text, a label or output a game might test,
   and a changed timing or order. It takes no `migration` block.
2. **One reader.** `behaviour_lines` in `crates/xtask/src/tickets.rs`
   reads it with the same `notes_lines` helper as `breaking_lines` (a
   `- ` marker allowed, lines outside the notes ignored).
3. **The changelog** (`cargo xtask changelog`) lists the lines in a
   *Behaviour changes* section after *Breaking changes*, and as
   `**Behaviour:**` items under *Changed* in the Keep a Changelog shape.
4. **No gate.** Nothing detects a behaviour change automatically; the
   line is the author's. The `work-ticket` skill (§5) makes the author
   earn "nothing in the game must change": search the game's code and
   ADRs for the rule touched (`gh search code --repo
   Zafnok/visions-of-shuyi …`) and name the search in the notes.
5. **The game hears it as it hears `Breaking:`.** ADR-0045 §2 keeps the
   dispatch payload to the sha; the watcher reads the PR body and the
   ticket's notes itself. The PR body repeats the `Behaviour:` lines as
   it repeats the `Breaking:` lines, so `notify-dependents.yml` doesn't
   change. Reading the line is the game's (game#1254).

## Consequences

- A behaviour change reaches the game's pin review as a line with a
  decision to make, before a game test fails.
- It depends on the author: a line nobody writes is missed as before.
  The search rule narrows that, but doesn't close it.
- The release notes say what a game will see change, not only what it
  must rewrite.

## Alternatives considered

- **Detecting behaviour changes** (diffing error texts, snapshot
  outputs) — most such changes leave no mark a tool can tell from an
  internal refactor; the ticket keeps it out.
- **Writing them as `Breaking:` lines** — CI's `api` job and the game's
  ledger treat a break as something a game must change; most behaviour
  changes need nothing, and mixing them would hide real breaks.
- **Sending the lines in the dispatch payload** — `workflow_dispatch`
  inputs are short strings, and ADR-0045 already has the receiver read
  the PR and the tickets itself; a second channel would carry the same
  lines twice.
