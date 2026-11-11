# ADR-0105: A wrong finding of the API gate is argued with here alone, with no issue upstream

- **Status:** Accepted; amends ADR-0038 §7
- **Date:** 2026-11-10
- **Related tickets:** 3307, 2028, 2061

## Context

ADR-0038 §7 answers a wrong finding of the public API gate with "a
`Breaking:` line that says why it is wrong, and an issue upstream".
Nick has ruled out any PR or issue on another project ([decision of
2026-11-10](../decisions.md#2026-11-10-no-prs-or-issues-on-other-projects),
`AGENTS.md` rule 12).

## Decision

1. A wrong finding of the gate still gets a `Breaking:` line that says
   why it is wrong (ADR-0038 §7, unchanged), and nothing goes to the
   tool's project.
2. A crash or bug of a tool the gate runs (`cargo-semver-checks`,
   `cargo-public-api`) is worked around in `crates/xtask/src/api.rs`,
   with the tool's version and what goes wrong written beside the
   workaround. A later version of the tool that no longer needs it is
   found when the pinned version moves.
3. The rest of ADR-0038 holds.

## Consequences

- A wrong finding the tool's project never hears of may come back on
  each PR that touches the item, and each time gets its line.
- The workarounds are ours to keep, and to take out when a tool's
  release no longer needs them.

## Alternatives considered

- **An issue upstream, as ADR-0038 §7 said**: ruled out by Nick.
