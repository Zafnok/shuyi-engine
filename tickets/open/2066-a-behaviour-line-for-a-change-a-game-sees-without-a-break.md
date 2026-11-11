---
id: "2066"
title: "A `Behaviour:` line for a change a game sees without a break"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2066 — A `Behaviour:` line for a change a game sees without a break

## Context

ADR-0038's `Breaking:` lines and the `api` job see signatures. The retro
of 2026-11-08 found that the changes which surprised the game at pin
time were behaviour, not signatures:

- **2350 (#256) loosened the `.dlg` parser** to reject only control
  characters. Its notes said "nothing in the game must change". The
  game keeps `.dlg` text plain ASCII (game ADR-0045), so pin PR game
  #547 had to write about 45–80 lines (`AsciiDlg`,
  `dialogue::parse_dlg`) to keep its rule. The game's tests caught it
  only because they pinned exact errors and columns.
- **#276 (2226) said "no break"** but changed `lang-status` labels and
  panic wording that the game tested.
- **2315 and 3210 changed the game's tests and snapshots** in pin PR
  game #572.
- **#266's types lost `Send`/`Sync`.** The `api` job did catch this one,
  after the PR body said "No break".

The game's pin review (its `engine-pin` skill) reads the engine's
`Breaking:` lines through its watcher. A behaviour change has no line to
read, so it is found when a game test fails, or not at all.

## Nick input

None.

## Scope

**In:**
- A `Behaviour:` line in `## Completion notes`, beside `Breaking:` (the
  same shape: `Behaviour: <crate>: <what a game sees change>`). It is
  for a loosened or tightened validation, a changed default, a changed
  error code or message text, a changed label or output a game might
  test, and a changed timing or order.
- `cargo xtask changelog` lists `Behaviour:` lines in their own section
  of the release notes.
- `notify-dependents.yml`, and the data it sends, carry the lines like
  `Breaking:` lines, so the game's watcher can put them in its
  *Breaking* table (game#1254 reads them).
- A rule in `work-ticket` §5: "nothing in the game must change" is
  written only after searching the game's code and ADRs for the rule
  touched (`gh search code --repo Zafnok/visions-of-shuyi`), with the
  search named.
- `tickets/README.md`'s *Breaking changes* section describes the line.

**Out (do not do):**
- Detecting behaviour changes automatically.
- The game's reading of the line (game#1254).

## Implementation steps

1. Add `behaviour_lines(body)` beside `breaking_lines` in
   `crates/xtask/src/tickets.rs` (the same `notes_lines` helper).
2. Add the section in `crates/xtask/src/changelog.rs`.
3. Find how `notify-dependents.yml` passes `Breaking:` lines to the game
   and pass `Behaviour:` lines the same way.
4. Update `tickets/README.md`, `work-ticket` and `document` where they
   describe `Breaking:`, then run `pwsh -File
   scripts/sync-agent-skills.ps1`.
5. Write an ADR amending ADR-0038 (`write-adr`), and update ADR-0038's
   status line to name it.

## Acceptance criteria

- [ ] A done ticket with `Behaviour: shuyi-assets: …` shows in `cargo
      xtask changelog`'s output under its own heading.
- [ ] The payload `notify-dependents` sends includes the line (shown by
      a dry run or a test of the payload builder).
- [ ] `work-ticket` §5 has the game-search rule.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `behaviour_lines` and the changelog section.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
