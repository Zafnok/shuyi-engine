---
id: "3443"
title: "shuyi check-leaks: no public file quotes a private script line or map row, and no job with the private files caches target/ or uploads them"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3443 — A leak check for a game's private files

## Context

- **The game ticket:** game#1400
  (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1400-the-leak-check-on-the-engines.md`).
  It came from the game's third engine-friction sweep, game#1397
  (`docs/research/2026-11-27-engine-shaped-code-audit.md`, row C1). The
  game ticket is not a blocker.
- **The job:** check that a game's public repository never quotes the
  private files it builds with, and that no CI job that checks them out
  keeps them in a cache or an artifact.
- **The friction numbers:** sweep game#1397. The game would keep about
  20 lines: its paths and its thresholds in `shuyi.ron`. It has about
  390 production lines today: `crates/xtask/src/leaks.rs` (about 325,
  game ticket 0157, game ADR-0118) and `content_layer.rs` (about 65). It
  also has a 388-line `workflow_guard.rs` compiled only for its tests,
  which reads every workflow. The ratio is about 19×. Outcome 1: an
  engine feature.
- **Today the engine states both rules only as prose.**
  `skills/shuyi-private-files/SKILL.md` §6 ("Never in public") has no
  check behind it. Its rules are: no private file copied into the
  public repository, and no job that checks the private files out
  caches `target/` or uploads an artifact embedding them. The scripts
  (`.dlg`) and maps (`.map`) are the engine's own formats, so the engine
  can read them without the game's help.
- **The generic shape:** a check, in the commands `shuyi.ron`'s
  `private_assets` part turns on, with two halves:
  1. **Quotes.** It reads every `.dlg` line (speech, narration, caption,
     reply) and every `.map` grid row from the private checkout laid
     over the public files. It reports any tracked public file that
     holds a run of N words of one of those lines (case, punctuation
     and line breaks ignored), a whole short line of at least M words,
     or a whole map row of at least R characters with three kinds of
     tile. A `{token}` splits a line into parts that are searched apart.
     The report names places only (file:line of the quote and of its
     source), never the text, because CI logs are public.
  2. **Workflows.** It reports any job in `.github/workflows/*.yml`
     that uses the private files' action and also caches `target/`
     (`rust-cache` without `cache-targets: false`, or another cache
     step) or uploads an artifact. An allow list of
     `(workflow, job, artifact)` covers a release's packages.
- **An example from a game that isn't tactics:** a visual novel kept
  public for its code, with its script in a private repository. Its
  snapshot tests or a pasted README excerpt would quote a line in the
  same way.

## Nick input

None.

## Scope

**In:**
- `shuyi check-leaks` in `crates/cli`, run when `private_assets` is
  set. Its settings go in `private_assets.leaks` (`#[non_exhaustive]`,
  every field defaulted):
  - `phrase` (default 7);
  - `short` (4);
  - `row` (12);
  - `unsearched` (path prefixes, default `["tickets/"]`);
  - `uploads` (the allowed `(workflow, job, artifact)` triples).
- The tracked files come from `git ls-files`. The private lines are read
  through the engine's own `.dlg` and `.map` readers from the layers
  the project already names.
- The template's `ci.yml` (private-files part) runs it.
- The `shuyi-private-files` skill's §6 names the check.
- **The API is additive.** It adds a new command and a new optional
  config part, and breaks nothing.
- **What changes for the game:** game#1400 deletes its copy and runs
  the engine's check, in the PR that moves the pin past this ticket.

**Out (do not do):**
- Searching issues, pull requests or commit messages: the check reads
  only the tracked files.
- Reading any private format but `.dlg` and `.map`.
- Quoting a found line in the output.

**Estimated size:** about 1,000 lines (code 400, tests 500, docs 100).

**Split:** past 900 lines, split off the workflow half (its reader and
allow list) into a new ticket.

## Implementation steps

1. Read the game's `crates/xtask/src/leaks.rs` and `workflow_guard.rs`
   (public: `github.com/Zafnok/visions-of-shuyi`) as the reference for
   the matching rules. Their tests are the cases to port.
2. Add `Leaks` to `PrivateAssets` in `crates/cli/src/project.rs`
   (additive: **from 2096 (done)** it is `#[non_exhaustive]`).
3. Write `crates/cli/src/check_leaks.rs`:
   1. sources: lines and rows by place;
   2. the search over the tracked files;
   3. the workflow half.
4. Wire the command in `crates/cli/src/lib.rs` next to `check-keys` and
   `check-text`, with its usage text.
5. Run it in the template's private-files CI, and update the skill and
   the book's private-files page.

## Acceptance criteria

- [ ] A fixture project whose public `README.md` holds 7 words of a
      private `.dlg` line fails, naming the README's line and the
      source's file:line, and its output contains none of the words.
- [ ] A line wrapped over two lines of a snapshot is still found. A
      common 6-word phrase is not.
- [ ] A `.map` row of 12 characters and three tile kinds is found. A
      row of one repeated tile is not.
- [ ] A workflow job with the private files' action and a
      `Swatinem/rust-cache` step without `cache-targets: false` fails.
      The same job with it passes, and an allowed upload passes.
- [ ] A project without `private_assets` says the check has nothing to
      do and passes.
- [ ] The book's chapter is updated (2802), or the completion notes
      have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated (`shuyi-private-files` §6),
      or the completion notes have a `Skills:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the word runs, short lines, tokens, map rows, and the workflow
  reader. Port the game's cases.
- Integration: a temporary git repository with a private layer, run
  through the command.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
