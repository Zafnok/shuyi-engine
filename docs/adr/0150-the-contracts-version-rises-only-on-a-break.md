# ADR-0150: The contract's version rises only on a break, and every other schema change is a fingerprint under it

- **Status:** Accepted
- **Date:** 2026-11-25
- **Related tickets:** 2088, 2052 (the version raised by exactly one), 2086
  (the contract's Rust types under ADR-0039), 2005 (the first release on
  crates.io), 2082 (Nick's words on breaking versions)
- **Amends:** ADR-0022 §5 (what a change to the schemas raises)

## Context

Every answer of the tool contract carries `contract`, the number
`contract::VERSION` (`crates/cli/src/contract.rs`, 61 on 2026-11-25).
ADR-0022 §5 raises it by one for any change to an input's or an answer's
schema, additive ones included, and `crates/cli/schemas/versions.json`
keeps one fingerprint per version. Ticket 2052 made CI refuse a raise of
more than one.

After the first release (2005), people's agents and scripts read the
contract through the MCP server and `--json`. Nick wants no run of
breaking versions after the MVP (ticket 2082, 2026-11-14). A number that
rises on every change says nothing to them: a client written for 52
can't tell whether 60 still answers as it expects. About fifteen open
tickets add a field or an operation and each raises the number. ADR-0038
gates the crates' Rust API; nothing gated what the JSON promises. 2086
moved the contract's Rust types under ADR-0039 and left the JSON to this
ADR.

Facts that shaped it:

- Every input and answer derives `deny_unknown_fields`, and the schemas
  say `additionalProperties: false`. They describe one set of schemas
  exactly, not what a later engine may add.
- In the last sixteen changes to the schemas (`git log --
  crates/cli/schemas/versions.json`, #530 to #587), the check of §4 finds
  no break: every one added a field, an operation or a value, or
  reworded a description. Raising the number for those told no client
  anything.

## Decision

### 1. What a client may rely on

Between two engine versions whose answers carry the same `contract`:

- **An answer only gains.** Its fields stay, with the types they had; a
  field that was always there stays always there. It may gain fields,
  new values of a field that names one of a list (an event's `kind`, a
  verdict), new tagged variants and new operations. A value is never
  renamed: a rename is a value gone.
- **An input only gains optional fields.** What a client sent is still
  taken, and means what it meant. An input may gain optional fields,
  values and variants, and take more types than it did. It never gains
  a required field, nor makes an optional one required.
- **The client's half:** it ignores a field it doesn't know, and treats
  a value or a variant it doesn't know as it would an unknown field
  (skips it, or reports it as unknown); it never fails on one. A client
  that validates answers with the checked-in schemas pins one set of
  them, since they say `additionalProperties: false`; one meant to span
  engine versions validates only the fields it reads.

The other way round is not promised: a client written for a newer
engine may send an optional field an older one refuses
(`deny_unknown_fields`).

What is not shape is not covered by the number: a changed default, an
error's text, a new problem code, a limit's wording. Those are
`Behaviour:` lines (ADR-0108), as for the crates.

### 2. The version's shape: one number that rises only on a break

`contract::VERSION` stays one `u32`, and `contract` in every answer stays
that number: a client of 52 reads the answers as it did. From this ADR
on, it **rises by one only when a change breaks §1's promise**, and any
other change keeps it.

Every set of schemas still gets its line in `versions.json`,
`{ "version": N, "fingerprint": "…" }`: a change that breaks nothing adds
a line with the same `version` and its new fingerprint, and a break adds
one with `version` one higher. The file is the history of every schema
set, the fingerprints are the revisions, and the number is the promise.
The schemas' test (`crates/cli/tests/it/schemas.rs`) checks that the last
line is the current schemas' fingerprint and names `VERSION`, and that
each line keeps the version of the one before or raises it by one.

The number goes on from 61: it is not reset, so no client sees it go
down. 61 is the first version under this promise; nothing is promised
across the versions before it.

### 3. A contract break is a breaking release of `shuyi-cli`

The contract is `shuyi-cli`'s: a break of it is a break of that crate,
whether or not its Rust API changed. It is recorded as ADR-0038 §5–§6
records the API's, with a `Breaking:` line and its `migration` block:

```text
Breaking: shuyi-cli: contract: <operation>: <what a client changes>
```

After the first release, such a PR is a major release of `shuyi-cli`
(a minor one while it is `0.x`, as Cargo reads semver), whatever the
semver checks find in its Rust API. Because Nick wants no run of
breaking versions, a change that can be made additive is made additive:
a field gained beside the old one, which stays until the next break
gathers several.

### 4. What CI checks

`cargo xtask contract [--base <rev>]` (`crates/xtask/src/contract.rs`)
compares the base's schemas (`git show <base>:crates/cli/schemas/…`)
with the working tree's, file by file (`contract::compat`), and lists
each break:

- in any file: a schema file gone, a field gone, a value or a tagged
  variant gone (variants are matched by their tag, else by place), any
  keyword that constrains a value changed (`format`, `minimum`,
  `additionalProperties`, …);
- in an answer (every file but `*.input.json`): a field that may now be
  absent, a type it can now have (`null` among them);
- in an input (`*.input.json`): a new required field, an optional one
  made required, a type it no longer takes.

`description`, `title`, `default` and `examples` are not shape and are
ignored. What the walk can't prove a client survives counts as a break.

Then the rule: with a break, `VERSION` is the base's plus one; with
none, it is the base's. CI's `api` job (ADR-0038) runs it against its
base, fails the PR that doesn't follow it, and adds the breaks to
`shuyi-cli`'s findings, so its rule asks for §3's `Breaking:` line
naming `shuyi-cli`. Two PRs that both break and raise to the same number
are caught as ticket 2052 caught them: on the merge ref the second's
`VERSION` is the base's again, and its break asks for one more.

## Consequences

- A client of the contract checks one number: the same `contract` as the
  one it was written for means everything it reads is still there.
- Most changes no longer touch `VERSION`, so parallel PRs stop
  renumbering it; they still both append a line to `versions.json` and
  meet there, as before.
- A break costs a major release of `shuyi-cli` and a `Breaking:` line,
  which pushes changes to be additive. The price is old fields kept
  beside new ones until a break gathers them.
- The check is conservative: a change it can't prove additive (a
  loosened `minimum`, an untagged variant moved) asks for a raise. The
  answer is to make the change in a form it can read, or to take the
  raise; never to edit the check to pass a PR.
- A client can no longer tell two additive revisions apart by the
  number. One that needs a field checks that the field is there.
- `deny_unknown_fields` and `additionalProperties: false` stay, so the
  schemas are exact for one set. §1 puts the tolerance in the client.

## Alternatives considered

- **A major beside a revision** (`contract` the revision, as today, and a
  new `contract_major` field) — it keeps a number per change, but adds a
  field to every answer and to `Answer`'s Rust type (a break of
  `shuyi-cli`'s API), keeps every additive PR renumbering the revision,
  and gives a client two numbers where one says what it needs. The
  fingerprints in `versions.json` already name each revision.
- **A semver string** (`"1.4.0"`) — it changes `contract`'s type from a
  number to a string, which breaks every client reading it today.
- **Raise on every change, as before** — the number promises nothing,
  which is what this ticket set out to fix.
- **The check in the schemas' own test** — the test sees only the
  working tree, where the regenerated schemas already replaced the old
  ones; only a comparison with the base knows what was there. The test
  keeps the lines in step, and `cargo xtask contract` runs the base
  comparison locally.
- **A hand-written `Breaking:` line with no schema diff** — it is what
  ADR-0022 had for the crates before ADR-0038: it relies on the author
  noticing, and the point is a check that notices.
