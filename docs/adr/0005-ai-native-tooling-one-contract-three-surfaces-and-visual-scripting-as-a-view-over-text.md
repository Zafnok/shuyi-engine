# ADR-0005: AI-native tooling: one contract, three surfaces (CLI, MCP, skills); visual scripting as a view over text

- **Status:** Accepted (Nick chose plan B, 2026-10-14; see `docs/decisions.md`). First proposed as the game repository's ADR-0066 (game ticket 1101)
- **Date:** 2026-10-14
- **Related tickets:** 1117 (tools move), 1129 (`shuyi new` and the
  contract), 1133 (MCP server), 1134 (the skill pack), 1135 (visual
  scripting), 1011 and 1017 (scripting runtime)
- **Extends:** game ADR-0007 (tests are the review), game ADR-0017 (the Harness),
  game ADR-0038 §4 (`frame-png`), ADR-0003 (deterministic records)

## Context

Nick, 2026-10-14: the engine should have "good AI vibe coding support as
much as possible", with "plenty of shippable skills, similar to how
https://github.com/morluto/rea became huge by adding AI skill wrappers
around pre-existing tools", and "eventually UI / graphical coding (like
Blueprints from Unreal)".

What the research found (sources in the game repository's [feasibility report](https://github.com/Zafnok/visions-of-shuyi/blob/main/docs/engine/feasibility.md)):

- **REA** (MIT) wraps tools people already trust (Ghidra, IDA, JADX, …)
  behind **one contract exposed three ways**: a CLI (JSON output), an MCP
  server that one command registers with many agents, and Agent Skills
  (`SKILL.md` folders with routing rules, "when not to use", and reference
  files loaded on demand). Answers carry evidence and their limits.
- **Agent Skills** became an open format (agentskills.io, December 2025).
  No game engine ships official skills yet; Remotion (a video library)
  ships `remotion-dev/skills` plus a Claude Code plugin, the closest
  model.
- Engines with agent support (Godot community MCP servers, Unity's and
  Unreal's official MCP plugins, Bevy's `bevy_brp_mcp`) all give agents
  the same things: start the game headless, send input, read the scene
  and logs, take screenshots. Unreal's Blueprints and levels are binary
  `.uasset` files that agents and git can't diff or merge.
- **Visual scripting:** Godot removed VisualScript in 4.0 (about 0.5% of
  users used it as their main language; it lacked high-level building
  blocks; costly to maintain). Blueprints work because of the huge
  gameplay framework under them.
- **Our repository already does most of what agents need**: the
  Harness drives the game headlessly with key scripts and reads the scene
  as plain data; snapshots are text; `frame-png` renders any frame
  without a window; the rules replay from seed plus commands; data files
  are text with validators that name file and line; `check-keys` and
  `check-text` catch mistakes agents make. It is a large part of why
  about 250 PRs landed in this repository's first 20 days.

## Decision

### 1. The contract

A set of operations, each with typed JSON input and output, implemented
once in `shuyi-cli` as library functions over the Harness, the
importers and the validators:

| Operation | Does | Returns |
| --------- | ---- | ------- |
| `new <template>` | A new game from a template: the crates, a screen, an example stage, `AGENTS.md`, the skills, CI with the gates | the files made |
| `check` | Load all content through every validator | every error: file, line, message |
| `run --script "<keys>" [--frames n]` | Drive the game headlessly with a key or pad script | the top screen, the scene as text, events, sounds started |
| `frame-png --script … --out f.png` | Render a frame with no window | the PNG path, its size |
| `scene` | The current scene or stage as text or JSON (a map scene, a world's things) | the dump |
| `replay <record>` | Re-run a turn or tick record (ADR-0003) | a state hash per step; the first step where two runs differ |
| `bots <n>` | Play a stage with the engine's bots | outcomes, lengths, measures |
| `import <file> [--print]` | Run an importer (Tiled, LDtk, Aseprite) | the result as our text format, or errors |
| `lint` | `check-keys`, `check-text`, the boundary gate | findings with file and line |
| `web` | Build and package the web version | the folder |
| `doctor` | Check the toolchain (Rust, targets, MSYS2 on Windows GNU, wasm) | what is missing and how to install it |

Every answer says what it could not determine (for example "no frame
drawn: the screen quit on frame 3"), as REA's answers carry their limits.

### 2. Three surfaces over the contract

- **CLI**: `shuyi <operation>`, human
  text by default, `--json` for tools.
- **MCP server**: `shuyi mcp` (stdio). Each operation is a tool; a
  session keeps one headless game running between calls so an agent can
  `run`, look at `scene`, `run` more, then `frame-png`. One command
  (`shuyi mcp install`) registers it with the agents it finds, after
  showing what it will write.
- **Skills**: `skills/<name>/SKILL.md` in the Agent Skills format,
  each with: when to use it, when not to, the steps, the contract
  operations it calls, and `references/` loaded on demand. Shipped in the
  engine repository, as a Claude Code plugin, installable with the
  common skill installers, and copied into every project by `shuyi new`.

### 3. The first skills

Generalized from this repository's own skills and practice:

| Skill | From | Teaches |
| ----- | ---- | ------- |
| `shuyi-new-game` | — | Start a game: template, first screen, first stage, gates |
| `shuyi-gates` | `run-gates` | The checks to run before every commit, and why none is weakened |
| `shuyi-input` | `keyboard-input` | Actions, never keys; rebinding; help text from the keymap |
| `shuyi-screen` | game ADR-0054 | A screen is logic, a view, a skin |
| `shuyi-stage-grid` | the tactics kit | A turn-based grid stage with `TurnRules` |
| `shuyi-stage-field` | ADR-0003 | A tick stage: movers, collision, camera |
| `shuyi-import-art` | ADR-0004 | Tiled, LDtk, Aseprite in; checking the result |
| `shuyi-animate` | ADR-0004 §3 | Clips, time-based frames, particles |
| `shuyi-cinematic` | game ADR-0060 | Timed shots, music-synced |
| `shuyi-dialogue` | game ADR-0011, game ADR-0055 | `.dlg` scenes, choices, `@if` |
| `shuyi-test` | game ADR-0007 | Unit, property, Harness, snapshot, `frame-png` review |
| `shuyi-debug-replay` | game ADR-0039 | Reproduce a bug from a record; find a desync by hash |
| `shuyi-balance-bots` | game ADR-0033 | Bots that play a stage and report |
| `shuyi-ship` | game ADR-0009, game ADR-0013 | Builds, licenses, credits, store pages |
| `shuyi-net` | ADR-0006 | Lockstep, rollback, sessions |

Our game's own skills (`work-ticket`, `ask-nick`, `story-writing`,
`ascii-art`, `write-ticket`) stay in this repository.

### 4. The engine is written so agents can work on it too

`AGENTS.md` at the engine's root, the same gates as the game, text
formats only (no binary scene or graph files, ever), and every new
feature with an example an agent can run. The contribution policy (0059
question 4) says how pull requests written with AI are judged: by the
gates and a clear description, like any other.

### 5. Visual scripting is a view over text, built last

- No second runtime. A node graph is a **text file** (`*.graph.ron`:
  nodes with ids and fields, edges) that compiles to things the engine
  already runs: a state machine, triggers and their actions, dialogue
  branches, or a script for the scripting runtime (1011, 1017). Agents
  and git read and diff the same file people edit as boxes and wires.
- The editor is a later tool (1135 decides between a web tool with
  `egui-snarl`, MIT/Apache, and our own glyph UI). It waits for the
  scripting runtime and for at least two examples that show what nodes
  people would need, so the graph has high-level building blocks from
  the start (the lesson of Godot's VisualScript).

## Consequences

- The engine's pitch is concrete and checkable: an agent can make,
  run, see, test and replay a game without a window or a human. That is
  the gap the research found (Bevy has no editor and heavy builds; Godot
  has an editor but its agent tools are third-party, and its foundation
  closed its own repository to AI-written contributions; the one small
  "agent-native" 2D engine found has a handful of users).
- Every operation is code we keep working: the contract has its own
  tests, and a skill that names an operation fails the engine's
  `check-skills` lint if the operation changes.
- The skills are documentation that has to stay true. They are tested:
  each skill's worked example is a script that CI runs.
- An engine that welcomes agents will get low-quality AI pull requests;
  the gates and 0059's contribution policy are the defence.
- Visual scripting is the last thing built. Nick should expect it after
  the engine is public, not before.

## Alternatives considered

- **Skills only (no CLI or MCP)** — skills that tell an agent to run
  `cargo xtask …` work today, but each agent then re-learns flags and
  parses text. One contract makes the skills short and the answers
  reliable.
- **MCP only** — not every agent or person uses MCP; the CLI is also
  what CI and humans run.
- **A Blueprint-style runtime (graphs interpreted directly, stored
  binary)** — agents and git can't work with it, and it is a second
  language to keep correct. Unreal can afford it; we can't.
- **No visual scripting** — possible, and Godot's numbers say few would
  miss it, but Nick asked for it for the engine's future users; making
  it a view over text keeps the cost to the editor alone.
