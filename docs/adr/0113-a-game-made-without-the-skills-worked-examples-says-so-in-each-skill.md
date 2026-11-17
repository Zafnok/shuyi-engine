# ADR-0113: A game made without the skills' worked examples says so in each skill, and `check-skills` reads it

- **Status:** Accepted
- **Date:** 2026-11-13
- **Related tickets:** 2597, 2502 (the wizard that will ask it)
- **Amends:** ADR-0043 §3 (`check-skills` asks a worked example of each
  skill of the engine's pack that waits for nothing)

## Context

Nick decided on 2026-11-13 that a new game's example files are optional,
on by default ([decisions](../decisions.md)): a newcomer keeps them, a
skilled developer can leave them out. They are the skill pack's worked
examples (ADR-0043 §2), which `shuyi new`'s composer copies into
`.claude/skills/` with each skill (`crates/cli/src/template/compose.rs`,
`with_skills`): each skill's `example.ron`, `example-*.ron` and
`example/` folder. A blank game was 77 files, 48 of them skills'.

Two things lean on them in a game:

- `shuyi check-skills` (`crates/cli/src/skills.rs`, `check`) fails a
  skill of the engine's pack that waits for nothing and has no
  `example.ron`, and a game's `AGENTS.md` names `cargo shuyi check-skills
  .claude/skills`;
- 13 `SKILL.md` files link their examples (`[`example.ron`](example.ron)`,
  `[…](example/intro.rs)`), and `check-skills` fails a link to a file
  the pack doesn't hold.

The engine's own `skills/` stays as it is (the ticket's **Out**).

## Decision

### 1. A template question the composer reads

The engine's template asks `examples` (`yes`/`no`, default `yes`), so
`--set examples=no`, an answers file and the MCP `new` tool's `set` all
answer it and `new --json` lists it. No part serves it: the composer
reads it (`template::EXAMPLES`), and its `no` is not a limit.
`shuyi new` asks it in a terminal before the GitHub questions
(ADR-0112), "yes or no, or press Enter for yes", unless `--examples`,
`--no-examples`, `--set examples=…` or the answers file answers it. The
flags exist beside `--set` because every question of `new` in a terminal
has its flag (2592), and `--no-examples` is what a developer types; they
write `examples` into `--set`, and a flag and a `--set` that disagree are
refused.

### 2. What `examples` = `no` copies

`Skill::without_examples` (`skills.rs`): every file but the worked
examples (`skills::is_example`: `example.ron`, `example-<name>.ron`, any
file of `example/`). In its Markdown files a link whose target, from the
skill's folder, is one of them becomes its words (`[`example.ron`](example.ron)`
→ `` `example.ron` ``). A skill that had examples gets one sentence at the
end of its `SKILL.md`, `skills::WITHOUT_EXAMPLES` followed by where they
are: "in the engine's repository, in `skills/<name>/`, at the version the
game builds with". It names no URL: the game's `AGENTS.md` already names
the engine at its version, and a link at `main` is a `check-skills`
problem. A skill with no example is copied as it is.

### 3. `check-skills` reads the sentence

A skill of the engine's pack that waits for nothing needs its worked
example unless its `SKILL.md` holds `WITHOUT_EXAMPLES`
(`Skill::left_out_examples`). The rest of ADR-0043 §3 holds. The
templates' gate and CI's `templates` job make the grid game with
`--no-examples` and the blank one with them, and both run `cargo shuyi
check-skills .claude/skills`.

## Consequences

- A game made either way passes its own gates; a game made with the
  examples writes the same files as before.
- The marker is per skill, in the file an agent reads: a skill that
  `shuyi add` copies later with its example is checked as before, and
  one whose example was deleted by hand without the sentence still
  fails.
- The engine's own pack could excuse a missing example by writing the
  sentence; nobody writes it by accident, and `cargo xtask skill-examples`
  runs every example of the pack anyway.
- The composer knows one question id by name. A template without the
  question copies the examples, as before.
- Ticket 2502 moves the terminal question into its wizard, with the
  template's others; the flags stay.

## Alternatives considered

- **A line in `shuyi.ron`** — `check-skills` takes a folder and needs no
  project; reading the game's project file from it ties the check to
  where the pack sits, and a pack copied elsewhere would fail again.
- **A marker file in `.claude/skills/`** — one more file in the folder an
  agent lists, saying less than the sentence, which also tells the reader
  where the examples are.
- **`check-skills` asks for examples only in the engine's own pack** —
  by path (`skills/` against `.claude/skills/`) it is a guess, and a
  game made with the examples would lose the check that its copy is
  whole. A rule "a pack with no example at all was made without them"
  breaks when `shuyi add` copies a module's skill with an example.
- **Leave the links dead** — `check-skills` fails them, and an agent
  following one finds nothing.
- **Only `--set examples=no`, no flags** — every other question `new`
  asks in a terminal has a flag; one without would be the odd one out.
