---
id: "3447"
title: "shuyi playtest --merge: join the reports of one stage's run split into seed ranges, judged as one run"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3447 — `shuyi playtest --merge`

## Context

- **The game ticket:** game#1404
  (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1404-playtest-merge-on-the-engines.md`),
  from the game's third engine-friction sweep, game#1397 (row C17). It
  is not a blocker. Game#1207 (the game's playtest command on the
  engine's) needs it too.
- **The job:** a game whose slowest stages take too long for one CI job
  plays each stage in two or more seed ranges, then judges all the
  tries as one run.
- **The friction numbers:** sweep game#1397.
  - What the game would keep: 0 lines.
  - What it has today: about 85 production lines in the game's
    `crates/tools/src/playtest.rs` (`merge_files` and `merge`, game
    ticket 0173), plus its `--merge` flag.
  - Ratio: there is nothing to keep, so no ratio. Outcome 1, an engine
    feature.
  - It is the last piece of the game's own playtest command that the
    engine lacks.
- **What the engine has:** `shuyi playtest` with `--seed`, `--tries`
  and `--report <json>` (`crates/cli/src/contract/playtest.rs`). It has
  no way to join reports.
- **The generic shape:** `shuyi playtest --merge a.json b.json …`.
  - It reads the `--report` JSON of each part.
  - It refuses parts that disagree on the stage, the entrants, the turn
    cap, the game's start arguments or the checks, naming what differs
    and which part.
  - It joins every part's tries in seed order and judges them once by
    the first part's checks, so `blocked` is that of one run.
  - The time per try is the parts' average. The date and commit are the
    first part's.
  - It prints the same summary as a run, and its exit code follows
    `blocked`.
- **An example from a game that isn't tactics:** a roguelike's 2,000
  seeded floors, split across four CI jobs.

## Nick input

None.

## Scope

**In:**
- `--merge` on the command and on the contract's `playtest` operation.
- A `--report` for the merged result.
- The history line: a merge writes one history record, unless
  `--no-history` is given.
- **The API is additive.** It adds a flag. The `--report` JSON gains a
  version field if it lacks one, so a part from another engine version
  is refused.

**Out (do not do):**
- Splitting a run into jobs: the game's workflow does that.
- Merging different stages into one report.

## Implementation steps

1. Read the game's `merge` (public repository,
   `crates/tools/src/playtest.rs`) for its rules and errors.
2. Add the flag to `crates/cli/src/contract/playtest.rs`. Read the
   reports, check that they agree, concatenate the tries, and re-judge
   with the engine's own judging of a run.
3. Summarize and write the history through the existing paths.
4. Update the `shuyi-balance-bots` skill and the book's playtest page.

## Acceptance criteria

- [ ] Seeds 0–49 and 50–99, merged, give the same checks, `blocked` and
      counts as one run of seeds 0–99 (fixture game).
- [ ] Parts with different stages or checks are refused with the name
      of what differs and the part's number.
- [ ] The exit code is non-zero when the merged run is blocked.
- [ ] The book's chapter is updated (2802), or the completion notes
      have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated, or the completion notes
      have a `Skills:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: agreement checks and averaging.
- Integration: two `--report` runs on the template game, merged and
  compared with one run.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
