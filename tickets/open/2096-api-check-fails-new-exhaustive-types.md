---
id: "2096"
title: "`cargo xtask api` fails a new public struct or enum that is neither #[non_exhaustive] nor listed as a plain value or a closed set"
type: infra
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2096 — `cargo xtask api` fails a new public struct or enum that is neither `#[non_exhaustive]` nor listed as a plain value or a closed set

## Context

[ADR-0039](../../docs/adr/0039-public-types-that-can-grow-non-exhaustive-config-and-views-with-constructors-plain-values-left-open.md)
sorts every public type into eight kinds. Outside `shuyi-app`, whose
`crates/app/src/lib.rs` turns on `clippy::exhaustive_structs` and
`exhaustive_enums` (line 175), only the skills' lines and the semver
check hold the rule, and the semver check sees the break only when a
field is added. 2082 to 2086 moved the types that open tickets planned
to grow. The retro of PRs #379–#484 (2026-11-17, problem 8) found that
the sweep missed types that are growing now (2095). The next miss would
be a type added today as exhaustive that grows a month later, after 0.1.

`cargo xtask api` already makes a listing (`cargo-public-api`, ticket
2010), and the listing shows the attribute:
`#[non_exhaustive] pub struct shuyi_core::bots::plan::PlanSettings`,
but `pub struct shuyi_core::grid::Pos`. Measured on 2026-11-17 with
`cargo +nightly-2026-07-06 public-api -p <crate> -sss` (default
features) over `shuyi-core`, `-assets`, `-ui`, `-platform`, `-app`,
`-cli`, `-sound` and `-script`: 759 listing lines of an exhaustive
`pub struct` or `pub enum` (about 550 types; a re-export is listed at
each of its paths), against 452 `#[non_exhaustive]` ones. Most are plain
values (`Pos`, `Rect`, `Rgb`) and closed sets (`Dir`), as ADR-0039
expects. Nothing says which.

3503 holds constructors to ADR-0039's new rule against neighbouring
numbers, checked over the same listing.

## Nick input

None.

## Scope

**In:**
- **`tools/exhaustive.ron`** lists, for each crate in `api.rs`'s
  catalog, the paths of its exhaustive public structs and enums. Paths
  are as the head's listing names them, and a re-export is listed at
  each of its paths. Each path goes under one kind:
  - `plain`: ADR-0039 kind 1;
  - `closed`: kind 6's closed sets;
  - `open`: a type that may grow but hasn't moved yet. It moves under
    ADR-0039 in the PR that grows it, with a `Breaking:` line. This list
    only shrinks.

  One line may hold several paths. A type an open ticket grows goes to
  2095, not to `open` (the clause below).
- **The check** goes in a new `crates/xtask/src/api/exhaustive.rs` and
  reads the head's listing of each crate. It fails:
  - an exhaustive `pub struct` or `pub enum` line that the file doesn't
    list;
  - a path in the file that the listing shows as `#[non_exhaustive]`,
    or doesn't show at all (a stale entry);
  - an `open` path that the base's file lacks. The base's file is read
    with `git show <base>:tools/exhaustive.ron` through `Runner`. When
    the base has no file, this rule doesn't apply.

  Each failure names the path and says what to do: make the type
  `#[non_exhaustive]` with a constructor (ADR-0039), or list it with its
  kind. The errors join `run`'s errors, as `outside_error`'s do, and
  `summary` shows them.
- ADR-0039's *How it is held* names the check and the file. The
  ADR-0039 lines of the `write-ticket` and `work-ticket` skills say that
  a new exhaustive public type is listed in the file with its kind.
- **Either order with 2095:** if 2095 is done, its types are
  `#[non_exhaustive]` and stay out of the file; if not, list
  `FontAtlasDef`, `FontInk`, `Sound` and `DoctorInput` as `open`, and
  2095 takes them out.
- **Either order with 3503:** if 3503 is done, reuse its parser of
  listing lines (`api/constructors.rs`) for the struct and enum lines;
  if not, write the parser here, in `api/exhaustive.rs`, and 3503 reuses
  it.
- **Either order with 2090:** if 2090 is done, its outside-crate check
  and this one both read the head's listing in `compare_crates`. Keep
  the two checks and their errors apart. If not, nothing to do (2090
  works around this one).

**Out (do not do):**
- Constructors and their arguments (3503).
- Moving the `open` types under ADR-0039 beyond what 2095 moves: each
  one moves in the PR that grows it.
- Struct-like enum variants (the last sentence of ADR-0039 kind 6) and
  traits (kind 7).
- `clippy::exhaustive_structs` in more crates: ADR-0039 says why not.

**Estimated size:** about 900 lines (code 250, tests 300, data 250,
docs 100).

## Implementation steps

1. Read ADR-0038 and ADR-0039. In `crates/xtask/src/api.rs`, read
   `compare_crates`, `CrateApi`, `outside_error`, `run` and `summary`,
   and the fakes in `api/tests.rs`.
2. List the head of each catalog crate with its features, as `listing`
   does. Sort its exhaustive structs and enums by kind into
   `tools/exhaustive.ron`. Search `tickets/open` for each name you would
   put in `open`. A type a ticket grows goes to 2095 if 2095 is still
   open; otherwise it moves under ADR-0039 here.
3. In `api/exhaustive.rs`, parse the file with the `ron` crate xtask
   already has. Parse the listing's lines: any `#[...]` before `pub`,
   then `struct` or `enum`, then the path up to `<`, `(` or the end of
   the line. Then apply the three rules.
4. Wire the check into `compare_crates` and `run`, and show its failures
   in `summary`.
5. Update ADR-0039's *How it is held* and the two skills' lines, then
   run `pwsh -File scripts/sync-agent-skills.ps1`. Run
   `cargo xtask api` and the gates.

## Acceptance criteria

- [ ] `cargo xtask api` passes on the branch with `tools/exhaustive.ron`
      in place, and no `open` path is a type that an open ticket grows.
- [ ] A fake listing with a new exhaustive struct that the file doesn't
      list fails and names the struct's path. With the struct listed as
      `plain`, it passes.
- [ ] A stale path fails, and so does a path the listing shows as
      `#[non_exhaustive]`.
- [ ] An `open` path that the base's file lacks fails; one the base's
      file has passes.
- [ ] ADR-0039 names the check and the file.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`api/exhaustive.rs`): parsing listing lines (attributes,
  generics, tuple structs, re-exports), and each failure above.
- Integration (`api/tests.rs`, faked `Runner`): a run with a new
  unlisted type fails, and the summary shows its path.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
