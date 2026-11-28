---
id: "3445"
title: "Each change's own private pin file: shuyi private-assets merges a base pin with pin files per branch, so two pull requests never edit one pin line"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3445 — Each change's own private pin file

## Context

- **The game ticket:** game#1402
  (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1402-private-pin-files-on-the-engines.md`),
  from the game's third engine-friction sweep, game#1397 (row C3). It is
  not a blocker.
- **The job:** let several changes to a game's private files be in
  flight at once without their pull requests conflicting on the one
  line that pins the private repository.
- **The friction numbers:** sweep game#1397.
  - What the game would keep: about 30 lines, its names and the fold
    workflow's call.
  - What it has today: about 400 production lines in
    `crates/xtask/src/assets_pin.rs` (game tickets 0194 and 0195), and
    77 lines of `.github/actions/private-assets/resolve.sh`.
  - Ratio: about 15×. Outcome 1, an engine feature.
  - The game's ADR-0135 names this ticket as its own alternative: "Pin
    files read by the engine's `private-assets` itself. This is the
    general form, and every Shuyi game with private files could use
    it."
- **What the engine has:** `shuyi private-assets` knows one pin file
  (`PrivateAssets::pin_file`) and `--pin`. The `shuyi-private-files`
  skill §4–§5 describes "two pull requests that both move it" and the
  conflict that follows, which is the problem this solves.
- **The generic shape:**
  - The **base pin** only moves forward, to a commit on the private
    repository's `main`.
  - A change in flight writes its **own pin file**,
    `<pin_dir>/<branch>.rev`, naming the head of its private branch
    (`private-assets --add`).
  - The **effective pin**, what every build and CI checkout reads, is
    the base merged with each pin file, in file-name order.
  - After the game's merge, a fold moves the base past the landed pin
    files and deletes them.
  - `--check` fails when the base moves back, moves sideways or leaves
    `main`, and when a pin file isn't its branch's own or is already in
    the base.
- **An example from a game that isn't tactics:** a narrative game
  whose writer and artist each have a private-art pull request open
  the same week.

## Nick input

None.

## Scope

**In:**
- `PrivateAssets.pin_dir: Option<String>` (default none, which keeps
  today's behavior exactly).
- `private-assets --add`, `--resolve`, `--check [--base <ref>]` and
  `--fold`.
- The effective pin used by the plain fetch when `pin_dir` is set.
- The template's `private-files` action: it resolves the same way,
  through the command, so there is no shell script to keep in step.
- A fold job in the template's workflows, run after a merge to `main`.
- **The API is additive.** It adds an optional field and new flags.
- **What changes for the game:** game#1402 deletes its own tool and
  script in the PR that moves the pin.

**Out (do not do):**
- Changing the private repository's own workflows.
- Merging private branches by any means other than git's own
  fast-forward and merge; a conflict there stays a human's job.

**Estimated size:** about 900 lines (code 350, tests 450, docs 100).

## Implementation steps

1. Read the game's `crates/xtask/src/assets_pin.rs`, `resolve.sh`,
   `private-land.yml` and ADR-0135 (public repository) for the rules
   and the cases.
2. Put a `History` trait over git in `crates/cli/src/private_assets.rs`
   so the merge order and the checks are tested without a network.
3. Add the flags, the effective pin and the fold.
4. Update the template's action and workflows, and the
   `shuyi-private-files` skill (§4–§5: the pin file replaces "two pull
   requests that both move it").

## Acceptance criteria

- [ ] With two pin files of two branches, the effective pin holds both
      heads, merged in file-name order (tested on a temporary
      repository).
- [ ] `--check` fails a base pin that moves back, sideways or off
      `main`, and a pin file whose branch isn't its name's.
- [ ] `--fold` after a merge moves the base and deletes the landed pin
      files, and leaves unlanded ones.
- [ ] A project without `pin_dir` behaves exactly as before (the
      existing tests pass unchanged).
- [ ] The book's chapter is updated (2802), or the completion notes
      have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated (`shuyi-private-files`), or
      the completion notes have a `Skills:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the order, the checks, and the fold over a fake `History`.
- Integration: temporary git repositories for a public and a private
  side.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
