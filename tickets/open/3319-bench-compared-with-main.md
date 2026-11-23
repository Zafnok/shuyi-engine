---
id: "3319"
title: "The bench job compares each frame's instructions with main's and fails a rise past a margin unless the PR says why"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3319 — The bench job compares each frame's instructions with main's

## Context

Found by the retro of 2026-11-17 (PRs #379–#484), its problem 3, which
three reviewers reached on their own. #476 (3218) made every text fill
parse a message. The game measured +80k instructions a frame (+4%) on
its Playdate target, and its cursor sweep went over budget (2,050,631
against 2,000,000; game PR #732). The engine's CI passed it: the fix is
3219.

What CI does today, checked on 2026-11-17 (`main` at `5a1b6e29`):

- The `bench` job (`.github/workflows/ci.yml`, lines 987–1058; ticket
  3008, ADR-0081) runs `cargo shuyi bench` on Hello, Grid under
  Valgrind's Callgrind. It fails a frame over a target's
  `frame_instructions` budget, a heap peak or a package over budget,
  and a frame whose count differs between two runs in the same job. It
  never compares with `main`. So a rise that stays under the budget
  passes, however large.
- It runs on `pull_request` and on `push` to `main`, when
  `needs.changes.outputs.code` is true. It uploads `bench-report.json`
  and `.md` as the artifact `bench-report`, kept 7 days. The workflow's
  `concurrency` cancels an older run on `main` when a newer push comes,
  so not every commit of `main` has a report.
- `bench-report.json` (`crates/cli/src/bench/report.rs`, `Report`,
  `deny_unknown_fields`) holds `runs`: one per scenario and set of
  targets (`scenario`, `on`, `one_bit`), each with `instructions` (a
  `Spread`: `most`, `frame`, `p99`, `mean`) and every measured frame's
  `instructions`.
- Instructions are counted, not timed (`crates/cli/src/bench.rs`:
  "the numbers are proxies that come out the same on every run"), and
  CI checks that two runs on one runner agree exactly. Whether two
  different runners agree is not checked anywhere: code that picks its
  path by the CPU's features at run time (`memchr`, `hashbrown`'s
  SIMD) can count differently on another CPU model.
- Hello, Grid fills two texts a frame on its stage (`stage.turn` and
  its help, `examples/hello-grid/src/screens/stage.rs`) and its menus'
  rows through `Ctx::label` (`crates/ui/src/widgets/menu/glyph.rs`). A
  game fills many more, so a cost per fill shows much smaller in Hello,
  Grid than in the game.

No open ticket plans this: `grep -rn "bench-report\|Perf:" tickets/open`
finds none; 3020 (parked) gates emulator counts against budgets, not
against `main`.

A ticket of the export-targets block: it guards the targets' budgets
(ADR-0008 §6).

## Nick input

None.

## Scope

**In:**
- **`shuyi bench --baseline <bench-report.json>`** (`crates/cli`): it
  compares each run with the baseline's run of the same `scenario`,
  `on` and `one_bit`, and writes a `Change` column to the table (each
  scenario's `most` and `mean` against the baseline's, in instructions
  and per cent) and a `rises` list to the report: each scenario and
  targets whose `most` or `mean` rose past the margin, with both
  numbers. A per-frame table of every frame that changed by more than
  the margin goes under it, for reading. A rise doesn't change the exit
  code: budgets still do. A scenario the baseline lacks, or whose
  `.bench.ron` the PR changed, is listed as "new" or "changed, not
  compared". A baseline that doesn't read (another format) is a
  warning, not an error. `rises` is `#[serde(default)]`, so an older
  report reads. A game's CI can use the flag the same way.
- **The margin: a rise of more than 1% and more than 2,000
  instructions,** on `most` (the number the budget gates) or `mean`.
  Why 1%: counts are deterministic on one runner, so the margin only
  has to absorb what differs between runners (step 1 measures it). 1%
  of the Playdate's 2,000,000 budget is 20,000 instructions, a quarter
  of what 3218 cost the game. The 2,000 floor keeps a cheap scenario's
  small frames from failing on a few hundred instructions. The gate is
  on `most` and `mean`, not on each frame: a change that moves an
  animation by a frame shifts per-frame counts without costing more.
  **If step 1 finds runners differ** by more than 0.25% on any
  scenario, the margin is 4 times the largest difference measured,
  written in the job's comment and the completion notes. If that is
  over 2%, the comparison lands warn-only, and the completion notes
  name a new ticket that makes the counts agree across runners (the
  bench profile built for one CPU level, say).
- **The baseline:** on a push to `main`, the `bench` job uploads
  `bench-baseline` (run 1's `bench-report.json`, and `bench-env.txt`
  with `valgrind --version`, `rustc -V` and the CPU model from
  `/proc/cpuinfo`), kept 90 days. On a pull request it downloads the
  `bench-baseline` of the CI run on `main` whose `head_sha` is the
  base's tip (`HEAD^1`, as in `changes.yml`) through `gh api`, the
  artifact found by id, not `gh run download` (which can return an
  earlier attempt's after a re-run). The job gets `actions: read`.
  An artifact, not a cache: the rust-caches fill the repository's
  10 GB and evict others, and a pull request's run never writes to
  `main`'s artifacts, a fork's included, so the baseline is only ever
  `main`'s.
- **An older baseline:** when the base's tip has none (its bench was
  skipped, or cancelled by a newer push), the job walks back the base's
  first parents (at most 50) to the newest commit with one. If nothing
  under `crates/`, `examples/hello-grid/`, `Cargo.lock` or
  `rust-toolchain.toml` changed between it and the base's tip, it
  compares and gates; else it compares and only warns, naming the
  commit and how many commits of code lie between. With none in 50, or
  when `bench-env.txt` names another Valgrind or `rustc`, it warns and
  passes.
- **The `Perf:` line:** the job fails when `rises` isn't empty, unless
  the PR's body has a line `Perf: <scenario>: <why>` for each risen
  scenario (`Perf: cursor-sweep: +1.8% on playdate, the fade's new
  palette step`). The body is read as it is now (`gh api
  repos/$REPO/pulls/$PR --jq .body`, as `scripts/pr-title.sh` reads the
  title), so a re-run after an edit reads the new one; the event's body
  is the fallback. Each accepted rise and its line go in the job
  summary. On a push to `main` nothing is compared.
- **A scenario that fills text every frame:** Hello, Grid gains
  `examples/hello-grid/bench/text-fills.bench.ron`: the options menu
  opened and held still for 20 measured frames, its rows' labels and
  its help filled every frame. It is gated against the budgets like
  the others.
- **Docs:** the `run-gates` skill's *The performance budgets* section
  (what the comparison does, and the `Perf:` line); the `work-ticket`
  skill's PR body section: a `Perf:` line for each risen scenario, and
  a `Behaviour:` line in the ticket's notes for a rise a game will see
  (its frames cost more), which the changelog already lists;
  `docs/targets/README.md`'s `shuyi bench` section (`--baseline`).
  Then `pwsh -File scripts/sync-agent-skills.ps1 -Check`.
- **An ADR** (the `write-adr` skill) amending ADR-0081: the bench is
  compared with `main`, the margin and why, the artifact over a cache,
  and the `Perf:` line.
- **For the game:** nothing changes; `--baseline` is new and optional.
  No public Rust API changes (`shuyi-cli`'s bench types are
  `pub(crate)`).
- **Either order with 3342:** if 3342 is done, compare each run's
  `start_instructions` with the baseline's too, with the same margin,
  listed in `rises` as `<scenario> (start)` (a run without a start
  count on either side is not compared); if not, nothing to do (3342
  adds it to this comparison).

**Out (do not do):**
- Wall time: never a gate (ADR-0081).
- The heap's peak and the packages' sizes against `main`: a follow-up
  if a rise in them slips through; the budgets gate them today. The
  start's instructions are compared once 3342 lands (above).
- Drift: many rises each under the margin. The budgets still gate the
  sum; comparing with a release's numbers is another ticket's.
- The game's own bench and its CI: the game's.
- Fixing 3218's cost: 3219 does it. Step 8 runs on commits before
  3219, so either order works. Emulator counts against budgets: 3020
  (parked).
- Making counts agree across runners: a follow-up only if step 1 finds
  they don't (see the margin above).

**Estimated size:** about 1,400 lines (code 450, tests 500, CI 120,
docs and the ADR 330).

**Split:** past 1,000 lines, split off the older baseline (the walk
back and the warn-only mode, step 6) into a new ticket; until it lands,
a PR whose base's tip has no baseline only warns.

## Implementation steps

1. **Measure the runners.** Run the `bench` job on `main` five times
   (re-runs land on different machines; print `/proc/cpuinfo`'s model
   in the summary) and compare every frame's count across them. Write
   the largest difference per scenario in the PR and settle the margin.
2. Read `crates/cli/src/bench.rs`, `bench/report.rs` and
   `bench/tests.rs`, and the `bench` job.
3. `--baseline`: read the baseline report; match runs by `scenario`,
   `on` and `one_bit`; compute changes; `rises` in the report; the
   `Change` column and the per-frame table in the Markdown.
4. `text-fills.bench.ron`. Check by hand that its frames fill the
   menu's labels and help (a count in a debug run is enough).
5. CI: upload `bench-baseline` on a push to `main`; on a pull request,
   find and download the base's tip's baseline, run `cargo shuyi bench
   --baseline`, and check `rises` against the PR body's `Perf:` lines.
6. The older baseline: the walk back, the code-path check, the
   warn-only mode, the env check.
7. The skills, the targets doc, the ADR; sync the skill loaders.
8. Show the gate would have caught 3218: run `cargo shuyi bench` on
   Hello, Grid at `c345aab4^` (before #476) and at `c345aab4` (#476)
   with the first as the baseline, and put the table in the PR.
9. The gates.

## Acceptance criteria

- [ ] With step 8's two commits, `cargo shuyi bench --baseline` lists
      `text-fills` in `rises`, and the CI logic run on that report
      fails without a `Perf: text-fills: …` line and passes with one.
- [ ] A report compared with itself has no rises and a `Change`
      column of zeros.
- [ ] A rise of 1.5% and 30,000 instructions on `most` is a rise; 1.5%
      and 1,500 instructions is not; 0.9% and 30,000 is not (with the
      margin step 1 settles, if it isn't 1%, the cases move with it).
- [ ] A scenario only in the new report is "new"; one only in the
      baseline is listed as gone; neither fails.
- [ ] A baseline whose JSON doesn't read is a warning, and the bench
      still gates the budgets.
- [ ] A pull request's `bench` job downloads `main`'s `bench-baseline`
      for its base's tip, by artifact id, and its summary names the
      baseline's commit.
- [ ] The job has `contents: read` and `actions: read` only, and
      zizmor passes.
- [ ] The PR states step 1's runner differences and the margin chosen.
- [ ] The book's chapter is updated (2802), or the completion notes
      have a `Book:` line saying why not (a CI gate, not a game's API).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/cli/src/bench/tests.rs`): the comparison on hand-made
  reports: no change, a rise past the margin on `most`, on `mean`, under
  the floor, a fall, a new and a gone scenario, a one-bit run matched
  only to a one-bit run, and a baseline from an older format.
- Unit: the Markdown table's `Change` column and the per-frame table.
- Integration: the `Perf:` check as a script with its own test (each
  risen scenario named, one missing, none), wherever the repository
  keeps its CI scripts' tests (`scripts/` or `crates/xtask`).
- CI: the job itself on this PR, compared with `main`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
