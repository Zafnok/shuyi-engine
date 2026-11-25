---
id: "3533"
title: "Snake in CI's export job: its packed build, and the export check's file lists by program"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2739"]
nick_input: none
completed:
---

# 3533 — Snake in CI's export job

## Context

2739 wrote `examples/snake`. Its ticket asked it into CI's `export` job
(3004): `shuyi export` of each of the job's targets (`windows`, `linux`,
`web`, and `macos` on pushes to main), then `tools/export-check/ci.sh`.
That job exports `examples/hello-grid` alone (`dir:` in
`.github/workflows/ci.yml`), and its checks are Hello, Grid's:

- `tools/export-check/<target>.txt` list each package's files by the
  program's name (`{base}/hello-grid`, `hello-grid.wasm`) and Hello,
  Grid's own extras (`ABOUT.md` and `capture/frames.ron`, its `export`
  part's `beside`);
- `check.sh`'s `packed` step fails a program that holds the text of
  `files/keymap.ron` or whose report names no pack (ticket 3300): the
  game must read its files from a pack beside the program in a release
  export, which a game does through a `packed` feature;
- `check.sh` fails an exposure report without `"declared": true`: the
  game needs a `disclosure` part in `shuyi.ron`.

Snake embeds its files, has no `packed` feature, no `export` part and no
`disclosure` part, so 2739 left it out of the job.

## Nick input

None.

## Scope

**In:**
- **Snake's packed build**: a `packed` feature, and its files read from
  `snake.pack` beside the program in a packed build
  (`shuyi::app::pack_beside_program`), as the template's base does
  (`templates/shuyi-default/base/crates/game/src/lib.rs`, `files()`);
  its `disclosure` part in `shuyi.ron` (`allowed_pack_entries` of its
  files).
- **The export check by program**: `check.sh`'s lists take the program's
  name (`{program}`, from the report's `package`), and a game's extras
  live in a list of its own: `tools/export-check/<target>.txt` keeps
  what every package holds, and `tools/export-check/<game>/<target>.txt`
  adds the game's (Hello, Grid's `ABOUT.md` and `capture/frames.ron`).
  `ci.sh` passes the game's folder name.
- **The job**: the `export` matrix gains a `game` dimension
  (`hello-grid`, `snake`), each leg exporting and checking its game.
  Say in the PR how many legs that adds per pull request.

**Out (do not do):**
- The Playdate export, which never runs in CI (3010; 3352 builds snake's).
- Any change to what a package holds.

## Implementation steps

1. Read 3004's, 3300's and 3081's completion notes, `export.yml`,
   `tools/export-check/`, and the base template's `files()`.
2. Snake's `packed` feature and `disclosure` part; `cargo shuyi export
   linux --release` and `web --release` in its folder pass `check.sh`
   by hand.
3. The lists by program and `ci.sh`'s game argument; Hello, Grid's
   extras in their own lists.
4. The matrix, and the gates.

## Acceptance criteria

- [ ] CI's `export` job exports and checks snake on each of its targets.
- [ ] Hello, Grid's legs check the same files as before.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the `export` job's legs; snake's packed build reads its
  pack (`cargo shuyi export web --release`'s page boots in the
  `web-render` job).

## Completion notes

*(Filled in by the session that completes the ticket.)*
