---
id: "3451"
title: "One flag cursor for the CLI's hand-rolled parsers, one wording for their errors, and contract ops that validate once"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3451 — One flag cursor for the CLI's hand-rolled parsers

## Context

The redundancy sweep of 2026-11-28 found about 26 hand-written flag
loops in `crates/cli` with the same shape. Each value-taking flag is 5–10
lines: take the next argument, refuse a missing one, parse a number,
check its range, and say which flag. Examples:

- `contract/attach.rs`, about 298–372: `--frames`, `--log-since`,
  `--perf-since` and `--for`;
- `contract/sfx.rs`, about 169–212;
- `contract/export.rs`, `run.rs` and `new.rs`;
- `reveal.rs`, `frame_png.rs`, `docs`, `playtest` and `bench`.

Their error texts differ for the same mistake. For an unknown flag, sfx
says "no flag --x" and attach says "unknown option: --x". sfx refuses a
repeated flag ("given twice"), while attach lets `--keys` repeat.

**Validation runs twice.** In `contract/attach.rs`, `parse_args` checks
the token, the address, the `follow_for`/`perf_follow` pairing and the
`perf_snapshot` rule, and `AttachInput::check` checks them again in other
words ("--for: only with --perf-follow" against "follow_for: only with
perf_follow").

About 200 lines go.

## Nick input

None.

## Scope

**In:**
- **`contract::args::Cursor`**, with:
  - `value(flag, what)`;
  - `number_in(flag, range, unit)`;
  - `once(slot, flag)`;
  - one arm for an unknown option and one for an extra positional
    argument.

  Each flag says whether it may repeat and whether a value may begin
  with `--`, as today.
- **The larger parsers move onto it**: attach, export, run, new, reveal,
  frame_png, sfx, docs, playtest and bench.
- **A contract op whose `parse_args` repeats its input's `check` calls
  `check`** and drops its own copy.
- **One wording** for an unknown flag, a missing value, a bad number and
  a repeat, used by every parser.
- **Behaviour:** these messages change (a `Behaviour:` line in the
  completion notes). The docs and the tests that quote them are updated.
- **The MCP schemas do not change:** `contract::flags`' test still
  passes.

**Out (do not do):**
- A generic argv-to-input mapper. Flag names don't map to fields
  (`--for` is `follow_for`), and step flags are ordered.
- The `check_keys`, `check_text` and `check_skills` wrappers in
  `lib.rs`, which print differently.
- xtask's parsers.

**Estimated size:** about 900 lines changed (about 550 removed, 350
added, most of it tests' new message texts).

## Implementation steps

1. Write `crates/cli/src/contract/args.rs` with its unit tests.
2. Move one parser (`sfx`), and update its tests' texts.
3. Move the rest, one per commit. For each op with an input `check`,
   delete the duplicated checks from `parse_args`.
4. Update `docs/contract.md` and the skill pack where they quote a
   message.

## Acceptance criteria

- [ ] Each parser still accepts every flag its usage names
      (`contract::flags`' test).
- [ ] An unknown flag gives the same text from every op.
- [ ] `shuyi attach --for 5` without `--perf-follow` fails once, with
      the input check's wording.
- [ ] Every repeat and dash-value rule a parser had before is kept: a
      test per parser for a repeated flag.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Cursor`. A missing value, a number out of range, a repeat
  refused and a repeat allowed, a value beginning with `--` refused and
  allowed, and an unknown option.
- The parsers' existing tests, with the new texts.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
