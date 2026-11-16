---
id: "2093"
title: "Parked: Remove the reverse-engineering research scripts and keep their reports"
type: infra
milestone: "Parked: networking"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: blocked
blocked_by: ["3083"]
nick_input: none
completed:
---

# 2093 — Remove the research scripts, keep the reports

> **Parked** This waits for 3083, which is parked behind the hosted
> preview service (2602). Nobody starts it until Nick unparks 3083 and
> 3083 is done.

## Context

The public demo defenses track (`docs/ROADMAP.md`, *Public demo
defenses*) ran four experiments, 2912 to 2915, each with throwaway
Python, JavaScript and Rust under `tools/research/` (48 files, about
4,700 lines of Python and JavaScript, plus two workspace crates). Their
findings are written in `docs/research/2912-…` to `2915-…`, and 2917's
[report](../../docs/research/2917-reverse-engineering-angles-not-yet-covered.md)
closed the track: only the engine's own leaks were worth engine work, and
3099, 3300 and 3303 removed them. Nick, 2026-11-16, in chat: the scripts
are throwaway, keep the findings, and remove them once 3083 is done and
says it no longer needs them.

They still cost something on every pull request:

- `tools/research/2912/fixture` (`cloning-fixture`) and
  `tools/research/2914/web-fixture` (`cold-rules-web-fixture`) are
  workspace members (`Cargo.toml`, `members`), so every workspace build,
  clippy run, test run and `Cargo.lock` carries them.
- CI's `web-render` job has a step *Cold rules browser replay matches
  native* that builds both crates in three arms (plain, `layout-kernel`,
  `vm-kernel`) for native and `wasm32-unknown-unknown` and runs
  `tools/research/2914/web/browser_smoke.py` on each.

3083 is the only open ticket that still names them: its Context calls
2915's `verify_public.py` "the starting point for the public-build
test". 3083 is a blocker so that it takes what it needs first.

## Nick input

None.

## Scope

**In:**
- Delete `tools/research/` whole: 2912, 2913 (with its `capture/` crate
  and `results/`), 2914 and 2915 (with `results/`).
- Remove the two workspace members from `Cargo.toml` and let
  `Cargo.lock` drop `cloning-fixture` and `cold-rules-web-fixture`.
- Remove the *Cold rules browser replay matches native* step from
  `.github/workflows/ci.yml`'s `web-render` job, and its mention in the
  job's comment if any.
- In `docs/research/2912-…`, `2913-…`, `2914-…` and `2915-…`, replace
  each *Reproduce* / materials passage that runs or links a file under
  `tools/research/` with one sentence: the scripts, crates and results
  were removed by ticket 2093, and are at commit `<the last commit on
  main that has them>` (a link to the tree at that commit). Every other
  line of the reports, results tables and hashes included, stays.

**Out (do not do):**
- Rewriting, shortening or re-deciding any report's findings.
- Any file 3083 kept or moved out of `tools/research/` for its own use
  (its completion notes say which); those stay where 3083 put them.
- `tools/sound-parity/`, `tools/web-check/`, `tools/export-check/` and
  the other non-Rust tools: they are in use by CI or a skill.
- Tickets in `tickets/done/` that name these paths: done tickets are
  history and stay as written.

## Implementation steps

1. Read 3083's completion notes. If they say 3083 still uses a file
   under `tools/research/` in place, stop and write that in this
   ticket's completion notes instead of deleting it; delete the rest.
2. Note the current `origin/main` commit (`git rev-parse origin/main`):
   the last one with the scripts.
3. `git rm -r tools/research`.
4. Edit `Cargo.toml`'s `members` to
   `["crates/*", "examples/*", "site/snippets"]`, then run
   `cargo metadata --format-version 1 > /dev/null` (or any `cargo`
   command with `--locked` dropped once) so `Cargo.lock` loses the two
   packages, and nothing else in it changes.
5. Delete the *Cold rules browser replay matches native* step from
   `ci.yml`'s `web-render` job.
6. Edit the four reports as Scope says. `git grep -n "tools/research"
   -- ':!tickets/done'` lists every line left to change.
7. Move this ticket to `tickets/done/`.

## Acceptance criteria

- [ ] `tools/research/` does not exist.
- [ ] `git grep -n "tools/research\|cloning-fixture\|cold-rules-web-fixture\|cold_rules_web_fixture" -- ':!tickets/done'`
      finds only this ticket and the reports' one-sentence pointers.
- [ ] `Cargo.lock`'s diff removes only `cloning-fixture` and
      `cold-rules-web-fixture` (and dependencies only they used).
- [ ] Each of the four reports links the commit that still has its
      materials, and its results tables and hashes are unchanged.
- [ ] CI's `web-render` job passes without the removed step.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None new: this removes code. The gates (workspace build, clippy, tests,
  links, typos) and CI's `web-render` job show nothing else depended on
  it.

## Completion notes
