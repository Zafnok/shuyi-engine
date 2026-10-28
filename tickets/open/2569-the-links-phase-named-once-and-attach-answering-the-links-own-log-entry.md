---
id: "2569"
title: "The live link's phase named once, so a new phase can't read as running, and attach answering the link's own log entry"
type: bug
milestone: After MVP
model: sonnet-5
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2569 — The live link's phase named once, and `attach` answering the link's own log entry

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-28). `shuyi attach` (and the MCP session attached to a running
build) reads the live link's answers (ADR-0047) and copies them, field
by field, into the contract's own types. One of the copies guesses:

- **The phase, with a wildcard.** `shuyi_platform::link::Phase`
  (`crates/platform/src/link.rs:212`) is `#[non_exhaustive]`:
  `Loading`, `Running`, `Failed`, written on the wire in snake case.
  `attach::output` (`crates/cli/src/contract/attach.rs:999`) turns it
  into `AttachOutput::phase`, a `String`, with its own `match`:
  `Loading` is `"loading"`, `Failed` is `"failed"`, and `_ => "running"`
  (line 1004). The wildcard is needed there, since `shuyi-cli` is
  another crate, so a phase the link adds later (a `Suspended`, a
  `Quitting`) is answered as `"running"`, with no warning and no
  failing test.
- **The log entry, four times.** One line of the log is
  `shuyi_ui::log::LogLine` (`crates/ui/src/log.rs:105`). The link
  answers it as `link::LogEntry` (`link.rs:305`: `seq`, `frame`, and
  `level`, `category` and `text` as strings, built in `link::log`,
  line 784). `attach` copies that into `attach::AttachLogEntry`
  (`attach.rs:153`), the same five fields with the same names and
  types, only so it can derive `JsonSchema` for the contract's schema.
  The run and scene answers have `session::LogEntry`
  (`crates/cli/src/contract/session.rs:139`, the same without `seq`,
  with `From<&LogLine>` at line 152), and the MCP session attached to a
  build copies `link::LogEntry` into it with a private `log_entry`
  (`crates/cli/src/mcp/attached.rs:277`).

The link's types can't derive `JsonSchema` today: `shuyi-platform` has no
`schemars`. 2566 adds a `schema` feature to `shuyi-platform` for its
script steps, so the clause below shares it.

## Nick input

None.

## Scope

**In:**
- **`link::Phase::name(self) -> &'static str`**: `"loading"`,
  `"running"`, `"failed"`, the same words as its serde form, written
  with an exhaustive `match` in `shuyi-platform`, where no wildcard is
  needed. A phase added later doesn't compile until it has its name.
  `attach::output` sets `phase: status.phase.name().to_owned()`, and
  its own `match` goes.
- **`link::LogEntry`** derives `JsonSchema` behind `shuyi-platform`'s
  `schema` feature (`#[cfg_attr(feature = "schema",
  derive(schemars::JsonSchema))]`, as `shuyi_ui::perf` does). Its field
  docs become the schema's descriptions, so they take
  `AttachLogEntry`'s fuller words (the list of categories, "frames count
  from 1", "in English").
- **`AttachOutput::log`** is a `Vec<shuyi_platform::link::LogEntry>`.
  The struct `AttachLogEntry` goes; `attach.rs` keeps the name as
  `pub use shuyi_platform::link::LogEntry as AttachLogEntry;`, so
  `shuyi_cli::contract::attach::AttachLogEntry` still names a type with
  the same fields. `attach::output` moves `seen.log.lines` in as they
  are.
- **`session::LogEntry`** stays (the run and scene answers have no
  `seq`), and both of its conversions sit in `session.rs`: the
  existing `From<&LogLine>`, and `From<shuyi_platform::link::LogEntry>`,
  which replaces `attached.rs`'s private `log_entry`.
- **The schema:** `attach.output.json`'s `$defs` entry is now
  `LogEntry`. Regenerate `crates/cli/schemas/`, raise
  `contract::VERSION` by one and add its line to
  `crates/cli/schemas/versions.json` (`docs/contract.md`, *The
  schemas*). The JSON `attach` answers doesn't change, and nor does the
  link's: `link::VERSION` stays.
- **Either order with 2566:** if 2566 is done, `shuyi-platform` has the
  `schema` feature (`["script", "dep:schemars"]`, turned on by
  `shuyi-cli`) and `schemars` in its `outside` list in
  `tools/layers.ron`: derive `JsonSchema` on `link::LogEntry` behind it;
  if not, add the feature here as `schema = ["dep:schemars"]`, with
  `schemars = { version = "1", optional = true, default-features =
  false, features = ["derive"] }` in `crates/platform/Cargo.toml` (as
  `crates/ui/Cargo.toml` has it), `"schema"` in `shuyi-cli`'s
  `shuyi-platform` features, and `schemars` in `shuyi-platform`'s
  `outside` list with its reason ("The JSON schemas of the live link's
  answers, for the tool contract (its optional `schema` feature)"), and
  2566 adds `script` to the feature.
- **For the game:** no break expected. `Phase::name` and the
  `JsonSchema` derive are additive; `AttachLogEntry` stays a public name
  with the same fields, still `#[non_exhaustive]`. If CI's `api` job
  reports a break anyway, write its `Breaking:` line (ADR-0038 §6).

**Out (do not do):**
- Sharing `link::Status` with `AttachOutput`: `AttachOutput` keeps
  `frames`, `screens` and the rest at its top level, and serde's
  `flatten` doesn't work with `deny_unknown_fields`, which both have.
  The phase was the one field copied with a guess.
- Making `AttachOutput::phase` a `Phase` instead of a `String`: a
  Rust API break for the same JSON.
- `limits_of`'s `_ => {}` (`attach.rs`): a phase with nothing to say
  adds no limit, which is right for a new one too.
- Merging `session::LogEntry` into `link::LogEntry`: adding `seq` to
  the run and scene answers is a change of the contract, not of the
  code.
- `LogLine` itself, and showing log lines on screen (2375).

## Implementation steps

1. Add `Phase::name` to `crates/platform/src/link.rs`, with its doc
   (the words are its serde form).
2. Settle the 2566 clause: the `schema` feature, then the
   `cfg_attr` derive on `link::LogEntry`, and copy `AttachLogEntry`'s
   field docs onto it.
3. In `crates/cli/src/contract/attach.rs`: replace the `phase` `match`
   in `output` with `Phase::name`; delete the `AttachLogEntry` struct,
   re-export the link's type under that name, and move the log lines
   into `AttachOutput::log` without copying.
4. Move `attached.rs`'s `log_entry` into `session.rs` as
   `impl From<shuyi_platform::link::LogEntry> for LogEntry`, and call
   `LogEntry::from` (or `.map(Into::into)`) in `attached.rs`.
5. `cargo shuyi schema --out crates/cli/schemas`, raise
   `contract::VERSION`, add the `versions.json` line.
6. Run the gates.

## Acceptance criteria

- [ ] `grep -rn '_ => "running"' crates` finds nothing.
- [ ] `grep -rn 'pub struct AttachLogEntry\|fn log_entry' crates`
      finds nothing.
- [ ] A test in `crates/platform/src/link/tests.rs` checks, for each of
      `Loading`, `Running` and `Failed`, that `serde_json::to_string`
      of the phase is `name()` in quotes.
- [ ] `attach`'s existing tests (`crates/cli/src/contract/attach/tests.rs`)
      and the attached MCP session's (`crates/cli/src/mcp/attached/tests.rs`)
      pass unchanged.
- [ ] The schema test passes with the regenerated schemas and the new
      `versions.json` line.
- [ ] `cargo xtask check-layers` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-platform`): `Phase::name` against its serde form, as
  above.
- Unit (`shuyi-cli`, `session.rs`'s tests): a `link::LogEntry` read
  from `{"seq":3,"frame":2,"level":"warn","category":"assets","text":"x"}`
  converts to a `session::LogEntry` with the same frame, level,
  category and text.
- The existing tests of `attach`, the attached MCP session and the
  schemas.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
