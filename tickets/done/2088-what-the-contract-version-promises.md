---
id: "2088"
title: "Before the first release, what the contract's version promises: an ADR on when it rises, what a client may rely on between two engine versions, and how it follows the crates' semver"
type: decision
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-25
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
the contract. 2086 (done) moved the contract's Rust types under ADR-0039 and
left the JSON's promise to this ticket.

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

- [x] The ADR answers each question in the Scope and is linked from
      `docs/adr/`'s index.
- [x] The schema test fails a breaking change without the ADR's raise,
      and passes an additive one with only the revision raised (both
      tried, in the notes).
- [x] `docs/contract.md` and the listed tickets say the same as the ADR.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the schema check, given an added field and a removed one.

## Completion notes

What was done:

- ADR-0150 amends ADR-0022 §5. It answers the Scope's four questions:
  - **What a client may rely on:** while `contract` stays the same, an
    answer only gains (fields, values, tagged variants, operations) and
    an input only gains optional fields. The client ignores fields and
    values it doesn't know.
  - **The version's shape:** one number that rises only on a break. Every
    other schema change keeps it and adds a line with the same version
    and its new fingerprint to `versions.json`, so the fingerprints are
    the revisions. `contract` keeps its type and goes on from 62, so a
    client of 52 still reads every answer. Nothing is promised across the
    versions before 62.
  - **The crates' semver:** a contract break is a breaking release of
    `shuyi-cli`, with a `Breaking: shuyi-cli: contract: …` line and its
    `migration` block (ADR-0038).
  - **What CI checks:** `crates/xtask/src/contract/compat.rs` compares
    the base's schemas with the head's and lists every break.
    `contract::compare` then wants the base's version when there is no
    break, and the base's plus one when there is one. CI's `api` job runs
    it, and adds the breaks to `shuyi-cli`'s findings so that its rule
    asks for the `Breaking:` line. `cargo xtask contract [--base <rev>]`
    runs the same check locally.
- `crates/cli/tests/it/schemas.rs`: the last line of `versions.json` must
  be the current schemas' fingerprint and name `VERSION`. Each line keeps
  the version of the line before it or raises it by one. The failure
  message gives both lines and points at `cargo xtask contract`.
- `docs/contract.md`'s *The schemas* section now has *What the version
  promises* and *Changing an input or an answer*. The `work-ticket`
  skill's renumbering line and the open tickets' "raise
  `contract::VERSION`" steps now say what ADR-0150 says.
- Tests: `contract::compat`'s unit tests (a field added, a field removed,
  a rename, a required input added, a field made optional or nullable,
  values and tagged variants, refs and recursion, a file gone). There are
  also `contract`'s tests against fake bases, and an `api` test that a
  contract break needs a line naming `shuyi-cli`.
- Both cases tried, before the merge of `main` took `VERSION` to 62 (2277's
  `sfx`), with `cargo xtask contract --base origin/main`,
  schemas edited by hand in the working tree and then restored:
  - `scene.output.json` gained a field, `VERSION` still 61: "no break of
    a client", `contract: ok`;
  - `scene.output.json` lost `limits` and `run.input.json` gained a
    required `seed`, `VERSION` still 61: both breaks were listed and the
    check failed with "raise it to 62";
  - the same breaks with `VERSION` at 62: `contract: ok`.
- Calibration: the last sixteen real schema changes (#530 to #587), each
  compared with its parent, were all additive. Under this rule none of
  them would have raised the version.

Deviations:

- The Scope's example shape, a major beside a revision, was not chosen.
  It would add a field to every answer and to `Answer`'s Rust type (a
  break of `shuyi-cli`'s API), and every additive PR would still
  renumber the revision. ADR-0150's alternatives say why.
- The additive/breaking check is in `cargo xtask contract` and CI's `api`
  job, not in `cargo test`'s schema test. Once the schemas are
  regenerated, the working tree no longer has the old ones; only a
  comparison with the base has them. The schema test keeps
  `versions.json` in step.
- The Scope listed 15 tickets. Every open ticket with a "raise
  `contract::VERSION`" step was updated, not only those 15. The others
  would have contradicted the ADR.
- No `versions.json` change: this PR changes no schema, so `VERSION`
  stays at `main`'s 62 and needs no new line.

Size: about 1,410 counted lines, past 1,000. The clean split would put
the check (`crates/xtask`) in one PR and the ADR, the docs and the
rewritten ticket steps in another. They landed together so that no
ticket would say one rule while CI held another.

Book: the *Tools for agents* chapter now says what `contract` promises
to a script that reads `--json`.

Behaviour: shuyi-cli: `contract` in every answer now rises only when a
change breaks a client (ADR-0150); additive changes keep it. A client
that used a higher number to detect a new field checks that the field is
there instead.

No break: no public item, schema or answer changed shape. `gh search code
--repo Zafnok/visions-of-shuyi 'contract::VERSION'` and `'versions.json'`
find nothing in the game.

Game: nothing to adopt, the game doesn't read the contract's version.

Nick input: none.
