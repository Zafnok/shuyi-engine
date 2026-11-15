---
id: "2088"
title: "Before the first release, what the contract's version promises: an ADR on when it rises, what a client may rely on between two engine versions, and how it follows the crates' semver"
type: decision
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2088 — What the contract's version promises, before the first release

## Context

The tool contract answers every operation with its version
(`contract::VERSION`, `crates/cli/src/contract.rs:76`, 52 on
2026-11-14). `docs/contract.md` says to raise it by one for any change
to an input or an answer
([ADR-0022](../../docs/adr/0022-the-tool-contract-answers-with-limits-schemas-by-version-and-a-session-over-the-harness.md)),
additive ones included. About 15 open tickets raise it (2225, 2232,
2502, 2515, 2516, 2517, 2520, 2523, 2526, 2527, 2529, 2550, 2552, 2569,
2583).

After 2005, people's agents and scripts read the contract through the
MCP server and `--json`. Nick wants no run of breaking versions after
the MVP (2082 has his words of 2026-11-14). Today a version that rises
on every change says nothing about whether a client written for 52
still works with 60. 2005's version policy ADR covers the crates, not
the contract. 2086 moves the contract's Rust types under ADR-0039 and
leaves the JSON's promise to this ticket.

## Nick input

None.

## Scope

**In:**
- An ADR (the `write-adr` skill) amending ADR-0022. It decides:
  - **What a client may rely on:**
    - answers only gain fields between two engine versions that keep
      the contract's major;
    - a client ignores fields it doesn't know;
    - inputs gain only optional fields.
  - **The version's shape:** for example a major that rises only on a
    break, beside a revision that rises on every schema change. Or one
    number that rises only on a break, with the schemas' hashes for the
    rest. Pick one, with why.
  - **How a contract break relates to the crates' semver:** a break in
    the contract is a breaking release of `shuyi-cli`, recorded with a
    `Breaking:` line as ADR-0038 records the API's.
  - **What CI checks:** the schema test that today asks for a raise
    tells an additive change from a breaking one (a field removed or
    renamed, a required input added, a value's type changed), and asks
    for the right raise.
- `docs/contract.md`'s section on changing an input or an answer,
  following the ADR.
- The listed tickets' "raise `contract::VERSION`" steps, as the ADR
  now says.

**Out (do not do):**
- Changing any operation's inputs or answers.
- The contract's Rust types (2086).

## Implementation steps

1. Read ADR-0022, `docs/contract.md`'s *Changing the contract* section,
   `crates/cli/schemas/versions.json` and the schema test that checks
   the raise.
2. Write the ADR.
3. Change the schema test and `versions.json` to the ADR's shape, if it
   changes the shape. A client of version 52 must still read the
   answers.
4. `docs/contract.md`, and the listed tickets' steps.

## Acceptance criteria

- [ ] The ADR answers each question in the Scope and is linked from
      `docs/adr/`'s index.
- [ ] The schema test fails a breaking change without the ADR's raise,
      and passes an additive one with only the revision raised (both
      tried, in the notes).
- [ ] `docs/contract.md` and the listed tickets say the same as the ADR.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the schema check, given an added field and a removed one.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
