# Shuyi Engine's skill pack

Skills that teach a coding agent to make games on Shuyi Engine, in the
open [Agent Skills](https://agentskills.io) format. Each skill is a folder:

- `SKILL.md`: when to use it and when not to, the steps, the engine's
  tools it calls (`cargo shuyi <command>`, or the MCP server's tools),
  and what to do on each problem code an answer gives;
- `example.ron`: its worked example, a script of the tools' commands that
  CI runs on a game fresh from `shuyi new`. If the example fails, the
  skill is wrong. A skill that teaches what the engine doesn't have yet
  says so, and has none;
- sometimes `example/` (the files its example writes into the game) and
  `references/` (detail an agent reads when it needs it).

## Getting them

- **In a new game:** `shuyi new` copies the pack into the game's
  `.claude/skills/`, all but the skills of the engine's modules. `shuyi
  add <module>` copies a module's skill when it turns the module on.
- **As a Claude Code plugin:** this repository is a plugin marketplace
  (`.claude-plugin/marketplace.json`), and its plugin is this folder:

  ```text
  /plugin marketplace add Zafnok/shuyi-engine
  /plugin install shuyi@shuyi-engine
  ```

- **With another agent:** each skill is a `skills/<name>/SKILL.md`
  folder at this repository's root, the layout skill installers read.
  Copy the folders you want into your agent's skills folder.

## Checking them

```bash
cargo shuyi check-skills skills
cargo xtask skill-examples
```

`check-skills` checks each skill's frontmatter, that every `shuyi`
command it names exists, that every file it links is in the pack, its
length, its worked example (or what it waits for), and that every
problem code it names is in `shuyi explain`'s table. A game checks its
own skills the same way: `cargo shuyi check-skills .claude/skills`.
`skill-examples` runs each worked example on a new game.

## The skills

- Starting and checking: [`shuyi-new-game`](shuyi-new-game/SKILL.md),
  [`shuyi-gates`](shuyi-gates/SKILL.md).
- Building the game: [`shuyi-input`](shuyi-input/SKILL.md),
  [`shuyi-screen`](shuyi-screen/SKILL.md),
  [`shuyi-stage-grid`](shuyi-stage-grid/SKILL.md),
  [`shuyi-stage-field`](shuyi-stage-field/SKILL.md),
  [`shuyi-import-art`](shuyi-import-art/SKILL.md),
  [`shuyi-animate`](shuyi-animate/SKILL.md),
  [`shuyi-cinematic`](shuyi-cinematic/SKILL.md),
  [`shuyi-dialogue`](shuyi-dialogue/SKILL.md).
- Testing and tuning: [`shuyi-test`](shuyi-test/SKILL.md),
  [`shuyi-debug-replay`](shuyi-debug-replay/SKILL.md),
  [`shuyi-balance-bots`](shuyi-balance-bots/SKILL.md).
- Shipping: [`shuyi-ship`](shuyi-ship/SKILL.md).
- A module's: [`shuyi-net`](shuyi-net/SKILL.md), online play.

Each skill's description, in its frontmatter, says what it is for.
