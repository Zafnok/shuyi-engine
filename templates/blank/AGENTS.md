# AGENTS.md

How to work on {{name}}, for people and coding agents alike. Claude
Code reads it through `CLAUDE.md`.

{{name}} is a game on [Shuyi Engine](https://github.com/Zafnok/shuyi-engine),
made with `shuyi new blank`: one screen, ready for the rest. The engine's
own rules are in its
[`AGENTS.md`](https://github.com/Zafnok/shuyi-engine/blob/main/AGENTS.md);
these are the ones a game on it keeps.

## The layers

- Keep the game's rules in a crate of their own (`crates/rules`, beside
  `crates/game`), pure and deterministic: no I/O, no clock, no floats,
  no randomness but a seeded `SimRng` kept in the state. The state
  changes only through commands (`shuyi_core::TurnRules`). The same seed
  and the same commands always give the same game. `shuyi new grid`
  shows one.
- `crates/game` is everything else: the game's data (`files/`, embedded),
  its screens, its look, its tools. Only the engine's `shuyi-app`
  touches the window, the files, the audio device and the clock.
- **Graphics are a skin.** A screen builds plain data (a view) and a
  skin paints it. Tests of what happened read the view or the state;
  only tests of a look read cells and colors.
- **Never hard-code a key.** Code reacts to actions (the six moves of
  `UiAction`); the keys are the player's, in `files/keymap.ron`, and text
  names a key as `{Confirm}`, which shows the player's own key.

## The tools

`cargo shuyi <command>` runs the engine's tools on this game, in its own
tool program (`crates/game/examples/tools.rs`, with the engine's
`BasicTools`). Each takes `--json` for an
answer a program can read, which also says what it couldn't determine
(its `limits`).

A command that fails names a problem code (`error[SH1002]`), with a help
line. The exit code says what to do: 2, change the command; 3, fix the
setup (`shuyi.ron`, or the toolchain: `cargo shuyi doctor`), then run the
same command again; 1, it ran and found something wrong.

| Command | What it tells you |
| ------- | ----------------- |
| `cargo shuyi check` | every error in `files/`, with file and line |
| `cargo shuyi run --script "Enter" --frames 30` | the screens, what happened and the sounds |
| `cargo shuyi frame-png title.png` | a picture of a frame, with no window |
| `cargo shuyi import crates/game/files/palette.ron` | what a file reads as, or its errors |
| `cargo shuyi doctor` | what your toolchain lacks and how to install it |
| `cargo shuyi explain SH2003` | what an error's code means and how to fix it; `explain` alone lists every code |
| `cargo shuyi mcp install` | registers the engine's MCP server with your coding agents (Claude Code's `.mcp.json` and others it finds): each command is a tool, and `start`, `step`, `scene` and `frame_png` play the game and look at it. The entry builds the server from the game's engine, and follows it: a worktree's calls go to a server built from that worktree, and a server whose engine moved under it hands its calls to a fresh one |

`scene`, `replay` and `bots` need a stage and rules: the game's tools
offer them once it writes its own `GameTools` (`shuyi new grid` shows
how).

## The short path

The game is on the engine's short path: `crates/game/src/lib.rs` names
its files, its data type (`BasicData<UiAction>`: the text, the keys, the
palette and the font from `files/`) and its first screen in a
`BasicGame`; `src/main.rs` runs it with `shuyi_app::run_game`;
`examples/tools.rs` gives its tools with `BasicTools`. A screen names
what it needs from `shuyi_platform::prelude`.

## When the game outgrows the short path

Grow it by adding a part, not by rewriting the start:

- **Its own actions:** an enum of its own that implements `ActionSet`
  (a debug key, a map's actions), in place of `UiAction`, with its
  names in `files/keymap.ron`.
- **A part of its own:** `BasicGame::with_own`, with a function that
  reads it from the files (a stage, a table of rules): it is
  `BasicData::own`.
- **Its own tools:** a `GameTools` of its own (a scene, records, bots),
  which calls `BasicTools`' methods for the rest. `shuyi new grid` shows
  one.
- **The long form:** its own `GameData` (sounds, voices, themes,
  settings of its own, `GameHooks`), and macroquad's entry point calling
  `shuyi_app::run` with its `AppConfig`.

## The gates

Tests are the review. Before every commit, run what CI runs
(`.github/workflows/ci.yml`), and never weaken one to pass it:

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo shuyi check
cargo shuyi check-keys
```
