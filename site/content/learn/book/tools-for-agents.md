+++
title = "Tools for agents"
description = "One tool contract, offered as a CLI, an MCP server and Agent Skills, with a stable code on every error."
weight = 100

[extra]
chapter = 10
+++

A coding **agent** is an AI that writes code by reading your files and running commands: it
can't look at your window, and it can't watch a game play. A person often can't either, on a
build server, or at the moment a test fails. So Shuyi Engine is built so that you and your
agent can make, run, see, test and replay a game without a window and without guessing.

You have been using this all along. Every `cargo shuyi` command in this book is one set of
operations, the **tool contract**, offered three ways: the `shuyi` command line, an MCP server,
and Agent Skills that teach an agent to use them. A **contract** here means a promise of what
goes in and what comes out, which a program can rely on.

## The tool contract

Each operation takes a typed input and gives a typed answer, the same shape for every one:

- `ok`, the verdict: nothing wrong found, a game made, two runs that agree;
- `result`, what it found or made;
- `error`, `code` and `help` when it couldn't run;
- `limits`, what it couldn't determine (a scene on a screen that has none, a play that ran on one
  machine only). An answer never guesses.

You saw a `limits` in [chapter 1](@/learn/book/quick-start.md#run-it) if you read to the end of
what `run` printed:

```text
limits:
  - no scene: the game's tools give no scene (the top screen is `title`)
```

That isn't an error. It's the tool being plain that there was something it couldn't tell you.

`cargo shuyi <op>` prints the answer for people, and with `--json` as one JSON document, a
common text format for data, for a program, whose **exit code**, the number a program hands back
when it stops, says its class: 0 when `ok`, 1 when it ran and found something, 2 for a wrong
command, 3 for a setup to fix. The operations are `new`, `check`, `lint`, `modules`, `add`, `run`,
`scene`, `replay`, `bots`, `playtest`, `autobalance`, `import`, `bake`, `doctor`, `explain` and
`attach`. Each is also a function of `shuyi_cli::contract`, which a game's own tests call:

{{ snippet(file="src/tools.rs", anchor="contract") }}

> **Rust:** `CheckInput::default()` makes the input with every setting at its default, and `let mut`
> lets the next line change one. `"{answer:?}"` prints the whole answer in its debug form, if the
> test fails, as `Debug` was derived for it. `answer.result.as_ref().unwrap()` looks inside the
> `Option` without taking it out of `answer`. `.iter().any(|e| matches!(e, Event::Quit { .. }))` asks
> whether any event is a quit, whatever else it holds, which is what `..` means.

The [contract's page](https://github.com/Zafnok/shuyi-engine/blob/main/docs/contract.md) says what
each operation answers, and every input and answer has a **JSON schema**, a description of a
document's shape that a program can read: `cargo shuyi schema run.output` prints one.

A game gives the tools what they need of it in its tool program
([chapter 1](@/learn/book/quick-start.md#what-it-is-made-of)). On the short path that is `BasicTools`:
its frame, its text, its files' errors and its controls. A game with a scene, records, bots or
importers writes its own `GameTools`, adds those, and calls `BasicTools` for the rest; a `grid` game's
stage has one to read.

## The MCP server

**MCP**, the Model Context Protocol, is a common way for an agent to call tools. `shuyi mcp` serves the
contract over MCP: each operation is a tool. Set it up once in the game's folder, and commit what it
writes:

```bash
cargo shuyi mcp install
cargo shuyi mcp --ready
```

`install` shows what it would write for each agent it finds (Claude Code's `.mcp.json` always, and
Cursor's, VS Code's and Gemini CLI's where it finds them) and asks first; `--dry-run` writes nothing.
`--ready` builds the server and the game's tool program, so the first call doesn't wait for a build.

The server keeps one game running headless between calls, a **session**, so an agent can press a key,
look, and press another, as you would:

| Tool | Does |
| ---- | ---- |
| `start` | builds the game's tool program and starts the game, with its start options |
| `step` | plays a script of keys, then idle frames, and answers as `run` does |
| `scene` | the scene on screen now, and the log |
| `frame_png` | a picture of the frame now |
| `events` | what happened since the start, and the sounds |
| `perf` | the frames' times and memory |
| `reset` | starts the game again as `start` did |
| `stop` | stops it |

Call `start` again after changing the game's code. With `link`, `start` attaches the session to a debug
build running in its window instead, and `frame_png` then gives the real window's picture.

## The skills

An agent that knows the tools still has to know what to do with them. The engine's **skill pack** teaches
an agent each kind of task on a game: starting one, a screen, input, a grid stage, dialogue, tests,
replays, bots, performance, shipping. Each skill says when to use it, its steps, the tools it calls, and
what to do on each problem code, and each has a worked example that the engine's CI runs on a new game, so
a skill that has gone wrong fails there first.

- `shuyi new` copies the skills into the game's `.claude/skills/`, and `shuyi add <module>` copies a
  module's when it turns the module on.
- As a Claude Code plugin: the engine's repository is a plugin marketplace
  (`.claude-plugin/marketplace.json`), whose plugin is its `skills/` folder:

  ```text
  /plugin marketplace add Zafnok/shuyi-engine
  /plugin install shuyi@shuyi-engine
  ```

- With another agent: each skill is a folder with its `SKILL.md`, in the repository's
  [`skills/`](https://github.com/Zafnok/shuyi-engine/tree/main/skills), the layout the common skill
  installers read. Copy the folders into your agent's skills folder.

A game's own `AGENTS.md` holds the rules a game on the engine keeps (this book's chapters 3, 4 and 6, in
short), for any agent that reads it. It also links the engine's rules and docs, those of the engine the game
builds with: for a game on `main`, the site's `llms.txt` and `llms-full.txt`; for a game on a commit
(`--engine-rev`), this book's Markdown at that commit; for a game on a checkout (`--engine-path`), the same
files in that folder.

## Problem codes

Every error the tools give has a stable code: `SH` and four digits, whose first says its class. An agent
branches on the code, never on the message, as a message's words may improve over time and the code won't
change. It's handy for you too: `cargo shuyi explain` lists every code, and `cargo shuyi explain SH2003`
prints one code's page, what it means and how to fix it. The same pages are on this site, in [the problem
codes](@/learn/errors/_index.md).

```text
check: error[SH1002]: shuyi.ron names no game tool program (`game`), and check runs the game
  help: fix shuyi.ron as the message says, then run the same command again
  see: shuyi explain SH1002
```

## When it goes wrong

**A command the tools don't know.** It says so with a code, then lists every command, so this is also
the way to find one:

```text
cargo shuyi: error[SH0001]: unknown command: walk
  see: shuyi explain SH0001
```

**A setup to fix.** The `SH1002` above is what you get from a folder whose project can't run the game.
Its exit code is 3: nothing is wrong with your game, and the tools couldn't get as far as looking at it.
Check that you are in the game's folder, then run `cargo shuyi doctor`, which lists what your machine has
and how to get what it lacks:

```text
doctor: host x86_64-pc-windows-gnu
  ok   rust: rustc 1.99.1 (58cf8f9c3 2026-10-13)
  ok   cargo: cargo 1.99.1 (5f94df478 2026-08-27)
```

**A session showing old code.** The server runs the game as it was at `start`, so after changing code the
answers are of the old game. Call `start` again.

**An `ok` that didn't look at everything.** Read `limits`. `lint` on a game with no rules crate says so:

```text
lint: OK
limits:
  - the rules crate's fence wasn't checked: shuyi.ron names no rules package (`rules`)
```
