---
id: "2597"
title: "shuyi new asks whether to include the example files, yes by default, and a game made without them still passes its gates"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: sign-off
completed:
---

# 2597 — `shuyi new` asks whether to include the example files

## Context

Nick, 2026-11-13 (`docs/decisions.md`, *a new game's example files are
optional*), signing off 2592: a new game holds "a lot of crap … like
bots.rs, playtest.ron". It should be an option, on by default, so a
newcomer keeps them and a skilled developer can leave them out.

The files he named are the worked examples of the engine's skill pack,
which `new` copies into `.claude/skills/` with each skill
(`crates/cli/src/template/compose.rs`, `with_skills`): for each skill of
`skills/`, its `example.ron`, `example-*.ron` (`shuyi-perf`,
`shuyi-test`) and `example/` folder (`shuyi-balance-bots/example/bots.rs`,
`…/playtest.ron`, `shuyi-cinematic/example/intro.rs`, …). A run of
blank game today is 77 files, 48 of them under `.claude/skills/` (Nick's
run of 2026-11-13).

Two things depend on them:
- `shuyi check-skills` (`crates/cli/src/skills.rs`, `check`) fails an
  engine pack skill that waits for nothing and has no `example.ron`
  (`has_example`), and a game's `AGENTS.md` names `cargo shuyi
  check-skills .claude/skills`;
- a `SKILL.md` may link its example (`shuyi-balance-bots`' *Worked
  example* links `example.ron`), a link that would be dead without it.

2592 (done) added the GitHub questions to `new`
(`crates/cli/src/contract/new/questions.rs`), each with its flag and
"press Enter for <default>". 2502 (open) will gather every question of
`new` in one wizard.

## Nick input

**Sign-off:** run `shuyi new "Lean Test" --no-github` in a terminal,
answer no to the examples question, and say whether what is left is what
you want; then the same with every default, and say whether it is as
today.

## Scope

**In:**
- **A template question** `examples` in
  `templates/shuyi-default/template.ron` ("Include the engine skills'
  worked examples (bots.rs, playtest.ron…)?", `yes`/`no`, default
  `yes`), so `--set examples=no`, the answers file and the MCP `new`
  tool's `set` all take it, and `new --json` lists it like
  `private-files`.
- **The composer:** with `examples=no`, `with_skills` copies each
  skill's `SKILL.md` and `references/` but not its `example.ron`,
  `example-*.ron` or `example/`.
- **`check-skills` in a game:** an engine skill without its example
  isn't a problem when the game's answers left examples out. Decide
  how it knows (a line in `shuyi.ron`, a marker file, or `check-skills`
  only asking for examples in the engine's own pack) and record it in
  an ADR if it is a new rule; the game's gates must pass either way.
- **A `SKILL.md` link to a missing example:** reworded or made
  harmless (say which: a sentence naming the engine's copy at
  `ENGINE_GIT`/`skills/<name>/example.ron`, for instance).
- **The terminal question** in `new` (2592's style: question, then
  "yes or no, or press Enter for yes"), asked before the GitHub
  questions; `--examples`/`--no-examples` as its flags (or `--set
  examples=…` alone: say which, and why).
- Docs: `docs/templates.md`, the book's quick start, the
  `shuyi-new-game` skill.
- **Either order with 2502:** if 2502 is done, the question is asked in
  its wizard with the template's others; if not, `new` asks it as 2592's
  questions are asked, and 2502 moves it into the wizard (its clause
  says so).

**Out (do not do):**
- Slimming the template's game itself (its screens, font, CI, setup
  script, devcontainer): not what Nick named; a later decision.
- Leaving skills out (only their examples go).
- Changing the engine's own `skills/` pack or its examples.

## Implementation steps

1. `template.ron`: the `examples` question; `compose.rs`'s
   `with_skills` reads the answer.
2. `skills.rs`: `check-skills` in a game made without examples passes.
3. The `SKILL.md` links, if any must change.
4. `new`'s terminal question and flags beside 2592's in
   `crates/cli/src/contract/new/questions.rs`.
5. Docs; `cargo xtask gates` (the templates gate makes a blank and a
   grid game: add one with `examples=no` if the gate takes answers).

## Acceptance criteria

- [ ] `shuyi new g --template blank --no-github` (and every default)
      writes the same files as before this ticket.
- [ ] `shuyi new g --template grid --set examples=no --no-github`
      writes no `example.ron`, `example-*.ron` or `example/` under
      `.claude/skills/`, and every skill's `SKILL.md` is there.
- [ ] That game's `cargo test --workspace` and `cargo shuyi
      check-skills .claude/skills` pass.
- [ ] In a terminal, `new` asks the question, says Enter picks yes, and
      a no leaves them out; `--json` and the MCP tool ask nothing.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `with_skills` with each answer; the question with an empty
  line, `no`, and a wrong answer asked again.
- Integration: a grid game made with `examples=no` passes
  `check-skills` (the template tests' plans; `plans.golden` gains the
  case if the golden lists answers).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
