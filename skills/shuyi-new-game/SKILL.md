---
name: shuyi-new-game
description: Start a new game on Shuyi Engine with `shuyi new`, on the engine's short path (BasicGame, BasicData, run_game, BasicTools), with its gates, its tools and the engine's MCP server for coding agents. Use when making a game from nothing, picking a template (blank or grid), getting the engine's tools or MCP server running on a game, or deciding when a game leaves the short path. Not for growing a game that exists (shuyi-screen adds a screen, shuyi-stage-grid a stage) or for the checks before a commit (shuyi-gates).
license: MIT OR Apache-2.0
metadata:
  version: "1.0"
---

# Start a game on Shuyi Engine

Shuyi Engine makes 2D games in Rust that an agent can build, run, look
at, test and replay without a window. `shuyi new` writes a game that
builds and passes its own gates as it is. Start from it, and grow the
game one part at a time.

## When to use it

- There is no game yet: make one with `shuyi new`.
- A game exists but its tools or its MCP server aren't set up.
- The game outgrows the short path, and you need to know what to change.

## When not to

- Adding a screen: `shuyi-screen`. A turn-based grid stage: `shuyi-stage-grid`.
- Keys and controls: `shuyi-input`. The checks before a commit:
  `shuyi-gates`. Tests: `shuyi-test`.

## Steps

1. **Get the engine.** Clone it, and check the toolchain from its
   folder. `doctor` says what is missing and how to install it (on
   Windows with the GNU toolchain, MSYS2's linker). On Linux, or in a
   cloud agent's bare container, run `bash scripts/setup-linux.sh`
   first: it installs the C linker, `pkg-config`, the system libraries
   and the pinned Rust, and the new game gets its own copy.

   ```bash
   git clone https://github.com/Zafnok/shuyi-engine
   cd shuyi-engine
   cargo shuyi doctor
   ```

2. **Make the game.** Pick a template by its short name. `blank` is one
   screen, the title. `grid` adds a turn-based grid stage, with its
   rules in a crate of their own, a bot and records. Pin the engine to
   the commit you cloned:

   ```bash
   cargo shuyi new ../moon-garden --template grid --name "Moon Garden" --engine-rev "$(git rev-parse HEAD)"
   ```

   Without `--engine-rev` the game follows the engine's `main`, and the
   answer's `limits` say so. `--engine-path <dir>` builds with a checkout
   on this machine instead. The answer lists the files made, the parts,
   the stages and the skills. A folder that isn't empty is refused
   (`SH0001`); `--force` writes into it.

3. **Read the game's `AGENTS.md`.** It holds the rules a game on the
   engine keeps (the layers, skins, actions), the tools, the short path
   and the gates. It names the engine's own rules and docs. Every later
   step assumes them.

4. **Build it and run its gates** from the game's folder. `cargo shuyi
   <command>` runs the engine's tools in the game's own tool program
   (`crates/game/examples/tools.rs`). The first one builds it:

   ```bash
   cargo test --workspace
   cargo shuyi check
   cargo shuyi run --frames 30
   cargo shuyi frame-png title.png
   ```

   `run` answers with the top screen, the screens, the scene, what
   happened and the sounds. `frame-png` draws a frame with no window:
   look at it. `cargo run` opens the game in a window for a person.

5. **Set up the MCP server**, once per game, and commit `.mcp.json`:

   ```bash
   cargo shuyi mcp install
   cargo shuyi mcp --ready
   ```

   `install` shows what it writes for each agent it finds (Claude
   Code's `.mcp.json` always) and asks first; `--dry-run` writes nothing.
   `--ready` builds the server and the game's tool program, so the first
   connection doesn't wait. Each tool is an operation of the contract,
   and `start`, `step`, `scene` and `frame_png` play the game and look
   at it between calls. Call `start` again after changing the game's
   code. The server follows the session's folder and the engine the game
   builds with by itself: after moving the engine's pin there is nothing
   to rebuild or reconnect.

6. **Turn modules on only when the game needs them.** `cargo shuyi
   modules` lists the engine's modules by concern, each with its status;
   `cargo shuyi add <module>` turns one on, copies its skill into
   `.claude/skills/`, and says what it is for.

## The short path

A new game names only what the engine can't read from its files:

- `crates/game/src/lib.rs`: `game()` returns a `BasicGame` (made with
  `BasicGame::with_own`): who the game is (`app_info::APP`), its
  `files/` folder, its stages' own part and its first screen, with
  `.screens(...)` (every screen it can show) and
  `.modules(shuyi::enabled_modules)`.
- `shuyi.ron`'s `app`: the game's name, its reverse-DNS id and its
  version. `cargo shuyi app-info` writes them into
  `crates/game/src/app_info.rs`; never edit that file. The id names the
  folder the player's saves are kept in: replace the placeholder
  `com.example.…` before a build is handed out, and never change it
  after (or set `storage` to the old name).
- `Data = BasicData<UiAction, stages::Own>`: the text, keys, palette and
  font read from `files/`, with the six moves of `UiAction` as its
  actions.
- `src/main.rs`: `run_game(game::game())`.
- `examples/tools.rs`: the game's tools, `game::stages::tools()`, the
  engine's `BasicTools` or a stage's own on top of them.

A screen names what it needs from `shuyi::prelude`.

## When to leave it

Grow a game by adding a part, never by rewriting its start:

- **Actions of its own** (a jump, a debug key): an enum of the game's
  that implements `ActionSet`, in place of `UiAction`, with its names in
  `files/keymap.ron` (`shuyi-input`).
- **A part of its own** (a map, a table of rules): a stage's `Own`,
  read by its `own` function from the files. `BasicGame::with_own`
  takes it.
- **Tools of its own** (a scene, records, bots): a `GameTools` that
  adds what it has and calls `BasicTools`' methods for the rest. A
  `grid` game's stage has one.
- **The long form:** a `GameData` of its own (sounds, voices, themes,
  settings, `GameHooks`), and macroquad's entry point calling
  `shuyi::app::run` with its `AppConfig`. Only when a part can't do it.

## When it fails

Branch on the answer's `code`, never on its message, and read its page
with `cargo shuyi explain <code>`:

- `SH0001`: the command is wrong (a folder that isn't empty, an unknown
  template). Change the command.
- `SH1001`: no `shuyi.ron` here or above: run it from the game's folder.
- `SH1003`: the toolchain lacks something: `cargo shuyi doctor` says
  what and how to install it.
- `SH1004`: the game doesn't depend on the engine through the `shuyi`
  crate.
- `SH3005`: the game's tool program didn't build: the answer carries
  the end of the compiler's output. Fix the code, then run it again.

## Worked example

[`example.ron`](example.ron) runs on a new `blank` game: the short path
in its code, the engine's skills in `.claude/skills/`, `check`, `run`
on the title and with Cancel, a picture of the title, `modules`, and a
problem code's page.
