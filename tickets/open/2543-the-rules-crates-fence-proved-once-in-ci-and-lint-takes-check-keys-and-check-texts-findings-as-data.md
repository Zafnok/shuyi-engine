---
id: "2543"
title: "The rules crate's fence proved once in CI, the grid template's gates run lint without check-keys, and lint takes check-keys' and check-text's findings as data"
type: infra
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2543 — The fence proved once, and `lint` takes the checks' findings as data

## Context

2015 fenced a new game's rules crate
([ADR-0032](../../docs/adr/0032-the-layers-as-data-checked-over-cargo-metadata-a-float-ban-in-the-simulations-and-a-new-games-rules-crate-kept-pure.md)
§4): `templates/grid/crates/rules/clippy.toml` bans hash maps, floats and
the clock, `src/lib.rs` denies float arithmetic, and `tests/layers.rs`
fails on a screen or device crate under the rules. It wrote 2534 for the
contract's `lint` instead of building it, and 2534 made `shuyi lint`:
`check-keys`, `check-text` and the fence (clippy's JSON for the three
lints, then `cargo tree`) in one answer with file and line
(`crates/cli/src/contract/lint.rs`). The fence's rules live in one place,
the template's `clippy.toml`, and a test keeps `lint`'s list of crates
never under the rules equal to the template's. That coordination worked.

A retro of PRs #38–#68 (2026-10-22) found what the layering left behind:

- **CI proves the fence twice.** In `ci.yml`'s `templates` job, 2534's
  step *shuyi lint finds what breaks the game's code* and 2015's step
  *The rules crate's fence holds* each add a `breaks_the_rules` function
  with a hash map, a float, the clock and float arithmetic to the made
  game's rules crate. 2534's step counts the four SH2103 findings; 2015's
  step greps clippy's four messages. Only 2015's second half (the layers
  test fails on `shuyi-ui` under the rules) proves something 2534's step
  doesn't. Two fixtures must change when `clippy.toml` does.
- **The grid template's gates run `check-keys` twice.** Its
  `.github/workflows/ci.yml` and its `AGENTS.md` (*Before every commit*)
  run `cargo shuyi check-keys` and then `cargo shuyi lint`, which runs
  `check-keys` again. The engine's `templates` job lists both too.
- **`lint` reads the checks' printed lines back.** `check_keys::run`
  returns `Vec<String>` of `file:line: what` (`crates/cli/src/check_keys.rs`),
  and `check_text::run_names` the same, so `lint`'s `reported` splits
  each string on `": "` and `':'` to find the file and line. It works
  today, but the line format is now an agreement between two files that
  no type checks: a change to how `check-keys` prints a line would lose
  `lint`'s file and line with no test failing.
- **`lint` drops a note of `check-text`'s.** `check_text::verdict` says
  "under the N allowed: lower `check_text.max_literals` in shuyi.ron to
  M" when a game has fewer literals than its cap. `lint` compares
  `hits.len() > rules.max_literals` itself and never says it.

## Nick input

None.

## Scope

**In:**
- **One proof of the fence's clippy half.** 2534's step keeps its
  injected code and its checks, and also checks that the four SH2103
  findings' messages say what 2015's step greps for today (the
  disallowed `HashMap`, the disallowed `f32`, the disallowed
  `Instant::now`, float arithmetic), with `jq` over `lint.json`. 2015's
  step keeps only its layers half: add `shuyi-ui` under the rules, and
  the layers test fails naming it. Its comment says the clippy half is
  in the step before.
- **No `check-keys` in the grid template's gates:**
  `templates/grid/.github/workflows/ci.yml`, `templates/grid/AGENTS.md`'s
  list, and the engine's `ci.yml` *Run its gates* for the grid template
  (`check-keys` stays for `blank`, which has no `lint` step). The
  `check-keys` command itself stays, and `AGENTS.md`'s table may still
  name it.
- **From 2022 (done):** the `run-gates` skill's section *A new game
  from each template* copies the templates job's gates in one block
  for `grid` and `blank`. Change it as `ci.yml` changes: `grid` runs
  `lint` and no `check-keys`, and `blank` keeps `check-keys`.
- **Findings as data:** `check_keys::run` and `check_text::run_names`
  return a list of `Hit { file, line: Option<usize>, message }` (or reuse
  `check_text::Hit` if it fits), and their commands print them as today,
  byte for byte. `lint` maps them to `Finding`s with no string parsing.
  `reported` stays only for an error reading a file, if one is still a
  string.
- **`lint` uses `check_text::verdict`**: an over-the-cap count gives the
  findings as today, and the under-the-cap note becomes one of the
  answer's `limits`.
- `docs/contract.md`'s `lint` row if what it says changes.

**Out (do not do):**
- a fence or `lint` in the blank template (2511 owns the minimal
  template);
- changing what the checks look for, or `lint`'s answer's shape;
- `lint` failing (exit 3) when `shuyi.ron` lacks a part: it records a
  limit and runs what the project has, by design;
- `check-keys` and `check-text` on the engine's own code (2006).

## Implementation steps

1. Read `crates/cli/src/contract/lint.rs` and its tests,
   `check_keys.rs`, `check_text.rs`, `lib.rs` (the `check-keys` and
   `check-text` arms), `ci.yml`'s `templates` job, and the grid
   template's `ci.yml` and `AGENTS.md`.
2. `Hit` for `check_keys::run` and `check_text::run_names`, test first:
   the commands' output is unchanged (their existing tests and the
   binary's), and `lint` gets the file and line with no parsing.
3. `lint` calls `check_text::verdict`; a test for the note as a limit.
4. The CI steps and the template's gates. Run the templates job's
   commands locally for `grid` (the `run-gates` skill's templates section).

## Acceptance criteria

- [ ] `ci.yml` injects the fence's clippy breaks once, and the lint step
      checks the four messages.
- [ ] The grid template's `ci.yml` and `AGENTS.md` run `lint` and no
      `check-keys`.
- [ ] The `run-gates` skill's templates section runs no `check-keys`
      for `grid` and still runs it for `blank`.
- [ ] `reported` (or its successor) no longer parses `check-keys`' or
      `check-text`'s findings.
- [ ] `lint` on a game with fewer literals than its cap has the
      `max_literals` note in `limits`.
- [ ] All gates in the `run-gates` skill pass, the `templates` job too.

## Tests required

- Unit: `lint`'s file and line for a `check-keys` and a `check-text`
  finding; the under-the-cap note; `check-keys`' and `check-text`'s
  printed lines unchanged.
- Integration: CI's `templates` job (grid and blank).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
