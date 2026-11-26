---
id: "2093"
title: "Move the reverse-engineering research scripts to a repository of their own and keep their reports"
type: infra
milestone: "Engine: networking"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-25
---

# 2093 — Move the research scripts out, keep the reports

## Context

The public demo defenses track (`docs/ROADMAP.md`, *Public demo
defenses*) ran four experiments, 2912 to 2915, each with throwaway
Python, JavaScript and Rust under `tools/research/` (48 files, about
4,700 lines of Python and JavaScript, plus two workspace crates). Their
findings are written in `docs/research/2912-…` to `2915-…`, and 2917's
[report](../../docs/research/2917-reverse-engineering-angles-not-yet-covered.md)
closed the track: only the engine's own leaks were worth engine work, and
3099, 3300 and 3303 removed them.

They still cost something on every pull request:

- `tools/research/2912/fixture` (`cloning-fixture`) and
  `tools/research/2914/web-fixture` (`cold-rules-web-fixture`) are
  workspace members (`Cargo.toml`, `members`), so every workspace build,
  clippy run, test run and `Cargo.lock` carries them.
- CI's `web-render` job has a step *Cold rules browser replay matches
  native* that builds both crates in three arms (plain, `layout-kernel`,
  `vm-kernel`) for native and `wasm32-unknown-unknown` and runs
  `tools/research/2914/web/browser_smoke.py` on each.

This ticket first waited for 3083 to take what it needed from them, but
3083 waits for 2611, which Nick won't do for a while. Nick, 2026-11-25
([decisions](../../docs/decisions.md#2026-11-25-the-research-scripts-move-to-a-repository-of-their-own)):
move them to a public repository of their own now, so nothing is lost if
the preview work resumes, and link it from the tickets and the research
reports only.

## Nick input

None. Nick decided the move in chat on 2026-11-25.

## Scope

**In:**
- A public repository, `Zafnok/shuyi-engine-research`, holding
  `tools/research/` whole at its root (2912, 2913 with its `capture/`
  crate and `results/`, 2914, 2915 with `results/`), with its history
  (`git subtree split`), a README saying what each folder backs and how
  to run it, and the engine's two license files.
- Delete `tools/research/` from the engine.
- Remove the two workspace members from `Cargo.toml` and let
  `Cargo.lock` drop `cloning-fixture` and `cold-rules-web-fixture`.
- Remove the *Cold rules browser replay matches native* step from
  `.github/workflows/ci.yml`'s `web-render` job.
- In `docs/research/2912-…`, `2913-…`, `2914-…` and `2915-…`, replace
  each passage that runs or links a file under `tools/research/` with a
  pointer: the materials moved to the new repository (a link), and the
  engine commit that last had them in place (a link to its tree). Every
  other line of the reports, results tables and hashes included, stays.
- The open tickets that name these paths: 3083 (what it keeps now comes
  from the new repository), 3507 (no `browser_smoke.py` to extend) and
  3510 (two fewer members to mark `publish = false`).

**Out (do not do):**
- A link to the new repository anywhere but the tickets and the research
  reports (Nick: not in the book, the README or other docs).
- Rewriting, shortening or re-deciding any report's findings.
- `tools/sound-parity/`, `tools/web-check/`, `tools/export-check/` and
  the other non-Rust tools: they are in use by CI or a skill.
- Tickets in `tickets/done/` that name these paths: done tickets are
  history and stay as written.

## Implementation steps

1. Note the current `origin/main` commit (`git rev-parse origin/main`):
   the last one with the scripts.
2. `git subtree split --prefix=tools/research origin/main`, push it with
   a README and the license files to the new public repository, and
   check that its tree equals `origin/main:tools/research`.
3. `git rm -r tools/research`.
4. Edit `Cargo.toml`'s `members` to
   `["crates/*", "crates/embed/fixture", "examples/*", "site/snippets"]`,
   then run any `cargo` command without `--locked` once so `Cargo.lock`
   loses the two packages, and nothing else in it changes.
5. Delete the *Cold rules browser replay matches native* step from
   `ci.yml`'s `web-render` job.
6. Edit the four reports and the three tickets as Scope says.
   `git grep -n "tools/research" -- ':!tickets/done'` lists every line
   left to change.
7. Move this ticket to `tickets/done/`.

## Acceptance criteria

- [x] `Zafnok/shuyi-engine-research` is public and its tree, less its
      README and license files, equals `tools/research/` at the noted
      commit.
- [x] `tools/research/` does not exist.
- [x] `git grep -n "tools/research\|cloning-fixture\|cold-rules-web-fixture\|cold_rules_web_fixture" -- ':!tickets/done'`
      finds only the four reports' *Reproduce* sections (their commands,
      which run at the noted commit or with the repository cloned into
      place), `docs/ROADMAP.md`'s row and `docs/decisions.md`'s entry.
- [x] `git grep -n "shuyi-engine-research"` finds only tickets and
      `docs/research/`.
- [x] `Cargo.lock`'s diff removes only `cloning-fixture` and
      `cold-rules-web-fixture` (and dependencies only they used).
- [x] Each of the four reports links the new repository and the commit
      that still has its materials, and its results tables and hashes are
      unchanged.
- [x] CI's `web-render` job passes without the removed step.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- None new: this removes code. The gates (workspace build, clippy, tests,
  links, typos) and CI's `web-render` job show nothing else depended on
  it.

## Completion notes

Done as rescoped by Nick on 2026-11-25.

- **The repository.** `git subtree split --prefix=tools/research` of
  `origin/main` at `f4b902f89e317504bcf1a4fc3544517923ca1c77` kept the
  folder's four commits (2912, 2913, 2915, 2914). Pushed with a README
  and the engine's two license files to the public
  [`Zafnok/shuyi-engine-research`](https://github.com/Zafnok/shuyi-engine-research).
  Its tree before the README commit is `ae1fc93d`, the same as
  `f4b902f8:tools/research`. The folders sit at its root, so cloning it
  into `tools/research/` of an engine checkout puts every relative path
  back where the scripts expect it; the README says so, and that
  `f4b902f8` runs them as they last ran.
- **The engine.** `tools/research/` deleted; the two members left
  `Cargo.toml`, and `Cargo.lock` lost `cloning-fixture` and
  `cold-rules-web-fixture` alone (19 lines); the *Cold rules browser
  replay matches native* step left `web-render` (the job's comment
  didn't name it).
- **The reports.** Each of 2912 to 2915 has a *Where the materials are*
  paragraph linking the repository, its study's folder and the engine
  tree at `f4b902f8`. The commands stay as written, since they run at
  that commit or with the repository cloned into place; 2912's two
  relative links now point into the new repository, and 2913's and
  2915's *Removing the study* paragraphs say it was removed. 2914's
  "CI's headless browser gate replays" now says it did until this
  ticket. No finding, table or hash changed.
- **The tickets.** 3083 says where 2915's scripts are now and that it
  brings what it keeps into the engine (its acceptance line changed to
  match). 3507 lost its `browser_smoke.py` work (its step 3 is left
  empty so its step numbers and its split hold). 3510 has two fewer
  members to mark `publish = false`.

Deviations:

- The scope changed (Nick, 2026-11-25, [decisions](../../docs/decisions.md#2026-11-25-the-research-scripts-move-to-a-repository-of-their-own)):
  the scripts went to a repository of their own instead of only being
  deleted, and this ticket stopped waiting for 3083 (which waits for
  2611, which Nick won't do for a while). It was unparked; its
  milestone is now *Engine: networking*.
- Step 4's `members` kept `crates/embed/fixture`, which the old ticket's
  list had missed.
- The repository is linked from tickets and `docs/research/` only, as
  Nick asked. `docs/ROADMAP.md`'s row and `docs/decisions.md` name the
  move without a link.
- The grep criterion was reworded: the reports keep their commands as
  written (they run at `f4b902f8`, or with the repository cloned into
  `tools/research/`), so `tools/research` still appears in their
  *Reproduce* sections, beside the pointer, and in the roadmap's row and
  the decision, which name the folder without linking the repository.

Size: 19,574 counted lines, nearly all the deletion of `tools/research/` (its results JSON most of all); the folder leaves whole, and its copy is in the new repository.

Book: none, no public API, contract, format or chapter changed; the book
never named the research scripts.
