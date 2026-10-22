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
| `cargo shuyi run --script "Enter" --frames 30` | the screens, what happened and the sounds |
| `cargo shuyi frame-png title.png` | a picture of a frame, with no window |
| `cargo shuyi import crates/game/files/palette.ron` | what a file reads as, or its errors |
| `cargo shuyi doctor` | what your toolchain lacks and how to install it |
| `cargo shuyi explain SH2003` | what an error's code means and how to fix it; `explain` alone lists every code |

`scene`, `replay` and `bots` need a stage and rules: the game's tools
offer them once `tools.rs` does (`shuyi new grid` shows how).

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
