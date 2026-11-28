---
id: "3544"
title: Fast gates locally, the full gates in CI
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-27
---

# 3544 — Fast gates locally, the full gates in CI

## Context

The `run-gates` skill says to run the full `cargo xtask gates` before
every commit you intend to push, and `AGENTS.md` rule 6, `work-ticket`
§4 and §6 and `sonar-fix` step 6 repeat it. On Nick's Windows machine
(GNU host, no MSVC) a full run takes about two hours for one change,
while CI's hosted runners run the same gates as parallel jobs on every
PR. On 2026-11-27, while fixing Sonar issue 127, a full local run on a
one-line change was stopped partway, and Nick said the rule is wrong:
fast gates locally, the full suite in CI.

`cargo xtask gates --fast` (ticket 2054) already runs the seconds-long
gates (fmt, typos, the ticket lint, the engine lint when Rust changed)
and prints the same summary block, and the pre-push hook runs it. What
it leaves out are clippy and the tests: a targeted `cargo clippy` and
`cargo test -p <crate>` for the crates the diff touches catches most of
what CI would, in minutes.

A full local run is also what ends with `cargo xtask trim-targets`
(ticket 2076, ADR-0121), so with fewer full runs the idle worktrees are
trimmed less often.

## Nick input

`None.` Nick decided this on 2026-11-27 in the session that fixed Sonar
issue 127.

## Scope

**In:**
- `.claude/skills/run-gates/SKILL.md`: its `description` and its
  opening say the default before a push is `cargo xtask gates --fast`
  plus `cargo clippy -p <crate> --all-targets --locked -- -D warnings`
  and `cargo test -p <crate> --locked` for each crate the diff touches;
  the full `cargo xtask gates` is CI's. A single slow gate runs locally
  only to reproduce a CI failure on it (or where a section says a
  change needs it before the push, as the wasm32 golden run and the API
  check do).
- The places that repeat the old rule, made to agree: `AGENTS.md` rule 6
  and its Environment note on the hooks and the trim, `work-ticket` §4
  and §6, `sonar-fix` step 6, and `write-post` step 2's aside.
- A new ADR recording the change, amending ADR-0121 on when the trim
  runs by itself.
- `pwsh -File scripts/sync-agent-skills.ps1`, and `-Check` passing.

**Out (do not do):**
- No change to `cargo xtask gates`, the hooks or `ci.yml`.
- No change to the open tickets' "All gates in the `run-gates` skill
  pass" line: it reads the skill, which now says what passing means.
- Running the trim after `--fast` is its own ticket (3545).

## Implementation steps

1. Rewrite the `run-gates` description and add a section *Before a
   push* at the top: the commands, the targeted crates, when a single
   slow gate runs locally, and that the summary block of `--fast` goes
   in the PR body.
2. In the same skill, say the trim runs after a full run only, and how
   to run it alone.
3. Edit `AGENTS.md` rule 6 and the hooks note, `work-ticket` §4 and §6,
   `sonar-fix` step 6 and `write-post` step 2.
4. Write the ADR (`write-adr`), and mark ADR-0121 amended by it.
5. Run `pwsh -File scripts/sync-agent-skills.ps1`, then `-Check`, then
   `cargo xtask gates --fast`.

## Acceptance criteria

- [x] `run-gates`' description and opening name `--fast` plus targeted
      clippy and tests as the default before a push, and the full run as
      CI's.
- [x] `AGENTS.md`, `work-ticket`, `sonar-fix` and `write-post` say the
      same; no text says to run the full gates before every push.
- [x] The ADR is in `docs/adr/` with its index row, and ADR-0121's
      status names it.
- [x] `pwsh -File scripts/sync-agent-skills.ps1 -Check` passes.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- None new: the change is to instructions. `cargo xtask gates --fast`
  (typos, the ticket lint) and `cargo xtask check-docs` (the ADR index,
  links) check the text; CI runs the rest.

## Completion notes

- `run-gates` opens with *Before a push*: `cargo xtask gates --fast`
  plus `cargo clippy -p <crate> --all-targets --locked -- -D warnings`
  and `cargo test -p <crate> --locked` per touched crate; the full
  `cargo xtask gates` is CI's, and a slow gate runs locally only to
  reproduce a CI failure on it or where its section says a change needs
  it first (wasm32 golden run, API check, the game at the next pin).
  Its `description` says the same. The trim paragraph says to run
  `cargo xtask trim-targets` alone until 3545.
- `AGENTS.md` rule 6 and the hooks note, `work-ticket` §4 and §6 (the
  PR body carries the `--fast` block and the crates checked),
  `sonar-fix` step 6 and `write-post` step 2 agree.
- Deviation: `bump-rust` step 4 keeps a full local run, on the GNU host,
  as CI's Windows leg is MSVC; `run-gates` and ADR-0166 name it as the
  one exception. `dependency-upkeep` and `give-up-sound-replacement`
  only point at `run-gates` and needed no change; nor do the open
  tickets' "All gates in the `run-gates` skill pass" lines.
- [ADR-0166](../../docs/adr/0166-fast-gates-locally-the-full-gates-in-ci.md)
  records it and amends ADR-0121 §4 (the trim after a full run).
- Follow-up: ticket 3545, a passing `--fast` ends with the trim too.
- No break: no code changes.
- Book: none, the book doesn't describe the engine's own gates.
- Skills: none, the shipped `shuyi-gates` skill is a game's gates, not
  this repository's.
- Guides: none, no direction, module or move is added.
