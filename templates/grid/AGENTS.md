# AGENTS.md

How to work on {{name}}, for people and coding agents alike. Claude
Code reads it through `CLAUDE.md`.

{{name}} is a game on [Shuyi Engine](https://github.com/Zafnok/shuyi-engine),
made with `shuyi new grid`. The engine's own rules are in its
[`AGENTS.md`](https://github.com/Zafnok/shuyi-engine/blob/main/AGENTS.md);
these are the ones a game on it keeps.

## The layers

- `crates/rules` is the game's rules: pure and deterministic. No I/O, no
  clock, no floats, no randomness but the seeded `SimRng` kept in the
  state. The state changes only through commands (`TurnRules::apply`).
  The same seed and the same commands always give the same game, which
  replays, bots and tests rest on. Clippy (`crates/rules/clippy.toml`)
  and the layers test (`crates/rules/tests/layers.rs`) turn these rules
  into errors, each saying what to use instead. Fix a failure in the
  code, never by loosening `clippy.toml` or the test (a test that must
  use a float writes `#[expect(clippy::disallowed_types, reason = "...")]`).
- `crates/game` is everything else: the game's data (`files/`, embedded),
  its screens, its look, its tools. Only the engine's `shuyi-app`
  touches the window, the files, the audio device and the clock.
- **Graphics are a skin.** A screen builds plain data (a map scene, a
  view) and a skin paints it. Tests of what happened read the scene or
  the state; only tests of a look read cells and colors.
- **Never hard-code a key.** Code reacts to actions (`src/action.rs`);
  the keys are the player's, in `files/keymap.ron`, and text names a key
  as `{Confirm}`, which shows the player's own key.

## The tools

`cargo shuyi <command>` runs the engine's tools on this game, in its own
tool program (`crates/game/src/tools.rs`). Each takes `--json` for an
answer a program can read, which also says what it couldn't determine
(its `limits`).

A command that fails names a problem code (`error[SH1002]`), with a help
line. The exit code says what to do: 2, change the command; 3, fix the
setup (`shuyi.ron`, or the toolchain: `cargo shuyi doctor`), then run the
same command again; 1, it ran and found something wrong.

| Command | What it tells you |
| ------- | ----------------- |
| `cargo shuyi check` | every error in `files/`, with file and line |
| `cargo shuyi lint` | everything wrong with the code's shape, with file and line: a key named in code or text, screen text written in the code instead of `files/en.ron`, and what keeps `crates/rules` from being pure (a hash map, a float, the clock, files, a screen or device crate under it), each saying what to use instead |
| `cargo shuyi run --script "Right Down" --frames 30` | the screens, the scene, what happened and the sounds |
| `cargo shuyi scene --script "Right"` | the stage's map and state |
| `cargo shuyi bots 20` | how 20 games played by bots ended, with a record of each |
| `cargo shuyi replay record.ron` | a hash of the state at every step of a record |
| `cargo shuyi frame-png stage.png --script "Right"` | a picture of a frame, with no window |
| `cargo shuyi import crates/game/files/stage.map` | what a file reads as, or its errors |
| `cargo shuyi modules` | the engine's modules by concern (perspective, bodies and motion, online play, kind of game), each with its status, and the ones this game has on |
| `cargo shuyi add <module>` | turns a module on (its feature of `shuyi` in `Cargo.toml`), says what it is for, and warns of an alternative already on; the game already installs every module it turns on (`shuyi::enabled_modules`) |
| `cargo shuyi doctor` | what your toolchain lacks and how to install it |
| `cargo shuyi explain SH2003` | what an error's code means and how to fix it; `explain` alone lists every code |
| `cargo shuyi mcp install` | registers the engine's MCP server with your coding agents (Claude Code's `.mcp.json` and others it finds): each command is a tool, and `start`, `step`, `scene` and `frame_png` play the game and look at it. The entry builds the server from the game's engine, and follows it: a worktree's calls go to a server built from that worktree, and a server whose engine moved under it hands its calls to a fresh one |

## The gates

Tests are the review. Before every commit, run what CI runs
(`.github/workflows/ci.yml`), and never weaken one to pass it:

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo shuyi check
cargo shuyi check-keys
cargo shuyi lint
```
