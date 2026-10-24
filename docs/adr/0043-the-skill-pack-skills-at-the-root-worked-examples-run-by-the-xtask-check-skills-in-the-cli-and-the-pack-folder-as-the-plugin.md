# ADR-0043: The skill pack: skills at the root, worked examples run by the xtask, `check-skills` in the CLI, and the pack's folder as the plugin

- **Status:** Accepted
- **Date:** 2026-10-24
- **Related tickets:** 1134 (the skill pack), 2501 (the composer), 2003
  (`shuyi add` and a module's skill), 2524 and 2525 (the agent eval),
  2523 (`shuyi-upgrade`), 2808 (docs agents read)
- **Extends:** ADR-0005 §2 and §3 (skills as the third surface over the
  contract, and the first fifteen), ADR-0041 §5 (a module's skill comes
  from its package or the pack beside it), ADR-0042 (the template
  format)

## Context

ADR-0005 makes skills the third surface over the tool contract: folders
in the open Agent Skills format, each with when to use it and when not
to, its steps, the operations it calls, and files loaded on demand. They
ship from the engine's repository as a Claude Code plugin, through the
common skill installers, and in every game `shuyi new` makes. Each has a
worked example that CI runs, so a skill that has stopped being true
fails. ADR-0005 left open:

- **the shape of a worked example**, and where its runner lives. The
  agent eval (2524) grades any agent's game through the same kind of
  script, so the runner must be one function both call, not steps
  written into a workflow;
- **where the checks of a skill run.** They read the contract's
  operations and the problem-code table, which `shuyi-cli` holds. The
  xtask doesn't depend on `shuyi-cli` (ADR-0041), and its CI jobs build
  nothing else;
- **the frontmatter.** The open format allows `name`, `description`,
  `license`, `compatibility`, `metadata` (strings by key) and
  `allowed-tools`. Claude Code ignores keys it doesn't know, but uploads
  to claude.ai and the Skills API refuse them. The pack needs a version
  and, for a skill that waits for parked work, what it waits for;
- **the plugin's folder.** A Claude Code plugin whose source is the
  repository's root also installs the root's `.mcp.json`, which runs the
  engine's own MCP server with `cargo run -p shuyi-cli` and works only
  here. Plugin files can't point outside the plugin's folder;
- **what a new game gets.** ADR-0041 §5 gives a game a module's skill
  only when it turns the module on (`shuyi add`), so an agent's context
  holds only what the game uses.

## Decision

### 1. The pack is `skills/` at the repository's root

Each skill is `skills/<name>/`:

- `SKILL.md`, its frontmatter the open format's keys only. The pack's own
  keys go in `metadata`, each a string: `version` (`"1.0"`) and, for a
  skill that teaches parked work, `waits_for`, the ticket ids it waits
  for, separated by spaces;
- `example.ron`, its worked example (§2), and `example/`, the files the
  example writes into the game;
- `references/`, detail an agent loads when it needs it.

The body has the same sections in every skill: when to use it, when not
to (naming the skill to use instead), the steps, the operations and
their answers, what to do on each problem code (branching on the code,
never the message, and pointing at `shuyi explain <code>`), and what
the worked example does. It is read by people making games, so it holds
nothing of this repository's process: no ticket ids, no owner, no pins.
It names the engine's rules and docs through the project's `AGENTS.md`,
never a fixed URL on `main`. The ids a skill waits for are in its
frontmatter only, and its body says what isn't built yet in the game
developer's words.

`skills/README.md` says what the pack is and how to install it, for
people.

### 2. A worked example is a script of contract commands, run by the xtask

`example.ron` is an `Example`: the template its game is made from
(`blank`, `grid`), an optional `manual` reason (an example that needs a
window, which CI lists and doesn't run), and its steps. The steps are
`crates/xtask/src/script.rs`'s `Step`, read with `deny_unknown_fields`:

- `Shuyi { args, ok, expect, save }`: `cargo shuyi <args> --json` in the
  game's folder. The answer's `ok` must be `ok`, the exit code 0 exactly
  when it is, and each `Expect` must hold: `Is` (equal: a string without
  its quotes, anything else as the JSON it reads as), `Has` (there and
  not `null`) and `Holds` (an array's item or a string's piece), each at
  a JSON pointer. `save` writes one value of the answer to a file of the
  game (a record to replay);
- `Cargo { args, ok }`: `cargo <args>` (the gates, `cargo shuyi
  frame-png`), which must succeed or fail;
- `Write`, `Edit` and `Holds`: a file of the game written from the
  skill's own files, one piece of its text replaced (it must be there
  exactly once), and a file that exists or holds a text. A path never
  leaves the game's or the skill's folder.

`script::run(steps, files, game, runner)` runs them in order, goes on
after a failure, and answers a `Verdict` per step: its place, what it
did, whether it passed, why not (what didn't hold, the answer's error,
code, help and limits, or the last 20 lines of the output), and its
time. It reads answers as `serde_json::Value`, so the xtask still
doesn't depend on `shuyi-cli`. The tests fake the `Runner`. The eval's
grader (2524) runs its checks through it and adds what it needs to it.

`cargo xtask skill-examples [<skill>...] [--shard <i>/<n>]` makes each
skill's game with `shuyi new` on this checkout
(`target/skill-examples/<skill>`, `--engine-path`), named after the
skill, and runs its script. Every game builds in this repository's
target folder unless `CARGO_TARGET_DIR` names another, so the engine
builds once. Each game takes its skill's name because cargo can take
one game's build of a crate for another's when two workspaces give it
the same name in one target folder. CI's `skills` job runs the examples
in three shards.

### 3. `check-skills` is a command of `shuyi`

`shuyi check-skills [<dir>] [--tickets <dir>]` (`shuyi_cli::skills`)
needs no project. It checks a pack (`skills/`, or a game's
`.claude/skills/`), each problem with its file and line:

1. the frontmatter reads as the open format's keys, its `name` is its
   folder's, its description says when to use it (`Use …`) and when not
   (`Not …`), and `metadata.version` is numbers and dots;
2. every `shuyi <command>` in the code of its Markdown is a command of
   `shuyi`: an operation of the contract, a session tool, or one of
   `shuyi_cli::COMMANDS`, which a test holds against the usage;
3. every relative link names a file of the pack (a game gets the pack
   and nothing beside it), and none names the engine's repository on
   `main`;
4. the name (64 characters), the description (1,024), `compatibility`
   (500) and `SKILL.md` (500 lines) are within the format's guidance;
5. with `--tickets`, every id in `waits_for` is an open ticket, and a
   skill that waits for nothing has a worked example;
6. every problem code (`SH2003`) in its files is a row of the table that
   isn't retired.

A test in `crates/cli/tests/it/skills.rs` runs it on `skills/` against
`tickets/open/` on every operating system, and another holds the binary's
embedded pack equal to the checkout's. A game can check its own skills
with the same command.

### 4. `shuyi new` copies the pack, all but the modules' skills

The catalog (ADR-0041) carries its engine's pack: `Catalog::read` reads
`skills/` from a checkout, and `Catalog::embedded` the copy the binary
embeds (`include_dir`, as the template is). `template::compose` adds
every skill of the catalog's pack to the plan under `.claude/skills/`,
byte for byte, but:

- a module's skill: one named after a crate the engine builds as a
  module, or one a concern names as an answer's (`shuyi-net`, built or
  not). `shuyi add` copies it (ADR-0041 §5);
- a skill whose folder a template's part already writes.

`Plan::skills` and `new`'s answer (`skills`) list the skills copied. The
plans' golden file leaves the skills' files out, and a test holds them
equal to the pack's. A skill that waits for parked work is copied too:
its body says what isn't built, which an agent asked for that kind of
game needs to know.

### 5. The plugin's folder is `skills/`

`.claude-plugin/marketplace.json` lists one plugin, `shuyi`, whose
`source` is `./skills` and whose `skills` is `./`: the plugin's root is
a folder of `<name>/SKILL.md` folders. No `.mcp.json`, command or agent
comes with it. It carries no `version`, so an install follows the
repository's commits. The same folders at the root are the layout skill
installers read.

### 6. One source for what holds in every game

When a skill of the pack teaches what one of this repository's own skills
teaches (`shuyi-input` and `input-actions`, `shuyi-gates` and
`run-gates`), the pack's skill is the source for what holds in every
game. This repository's skill keeps only what holds here (the engine's
own crates, CI's jobs, the game at the next pin, the owner's machine) and
links the pack's. The docs map lists each such pair. A later skill that
overlaps one of ours follows the same rule.

## Consequences

- An agent gets the same skills from a game made with `shuyi new`, from
  the plugin and from the repository. CI proves each skill's steps
  against a fresh game on every pull request that changes the code, the
  templates or the pack.
- A change to the contract, a problem code or a template that a skill
  names fails the pack's test or its example in the same PR. The skill
  is fixed there, as docs must be.
- The `skills` job builds a game per skill. It costs CI minutes, in three
  shards, and `shuyi-ship`'s example builds a web build.
- The binary embeds the pack, so a change to a skill rebuilds
  `shuyi-cli`.
- `Catalog` and `Plan` have a field more: a break for code that builds
  them field by field.
- A skill's worked example writes Rust into a game. That code must pass
  the game's own gates (fmt, clippy with `-D warnings`, `lint`).

## Alternatives considered

- **The runner as steps in the workflow** (`run:` lines with `jq`): the
  eval's grader couldn't call it, and a local run would differ from CI's.
- **`check-skills` in the xtask, reading the contract from its schemas
  and the codes from their pages**: the retired flag and the commands
  that aren't operations aren't in those files, and a game couldn't check
  its own skills.
- **A plugin whose source is the repository's root:** it installs the
  root's `.mcp.json` too, and the docs give no way to leave it out.
- **The pack copied into the plugin's own folder** (or linked there): two
  copies to keep equal, and Windows checkouts without symlinks.
- **A YAML list for `waits_for` and a top-level `version`:** the open
  format's `metadata` holds strings only, and uploads refuse unknown keys.
- **Copying only the skills that wait for nothing:** an agent asked for a
  platformer would then not learn that tick stages aren't built yet.
