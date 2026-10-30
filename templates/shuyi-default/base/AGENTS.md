# AGENTS.md

How to work on {{name}}, for people and coding agents alike. Claude
Code reads it through `CLAUDE.md`.

{{name}} is a game on [Shuyi Engine](https://github.com/Zafnok/shuyi-engine),
made with `shuyi new`. The engine's own rules are in its
[`AGENTS.md`](https://github.com/Zafnok/shuyi-engine/blob/main/AGENTS.md);
these are the ones a game on it keeps.

## The layers

- Keep the game's rules in a crate of their own (`crates/rules`, beside
  `crates/game`), pure and deterministic: no I/O, no clock, no floats,
  no randomness but a seeded `SimRng` kept in the state. The state
  changes only through commands (`shuyi::core::TurnRules`). The same seed
  and the same commands always give the same game. A game made with
  `shuyi new --template grid` has one, and its grid stage's rules crate
  comes with its fence: `crates/rules/clippy.toml` and
  `crates/rules/tests/layers.rs` turn these rules into errors, each
  saying what to use instead (`cargo shuyi lint` reports them). Fix a
  failure in the code, never by loosening `clippy.toml` or the test. A
  folder with an `AGENTS.md` of its own (`crates/rules/AGENTS.md`) has
  more rules for its files.
- Time a rule in turns or steps with `shuyi_core::timer` (`Countdown`,
  `Every`): advance it by one each turn in `apply`, or once per step of
  `input.ticks()`. Never time a rule with `input.dt()`, which is for
  what only looks.
- `crates/game` is everything else: the game's data (`files/`, embedded),
  its screens, its look, its tools. Only the engine's `shuyi-app`
  touches the window, the files, the audio device and the clock.
- **Graphics are a skin.** A screen builds plain data (a view) and a
  skin paints it. Tests of what happened read the view or the state;
  only tests of a look read cells and colors.
- **Never hard-code a key.** Code reacts to actions (the six moves of
  `UiAction`); the keys are the player's, in `files/keymap.ron`, and text
  names a key as `{Confirm}`, which shows the player's own key.
- **Ask `ctx.platform`, never `cfg!`:** offer Quit only where
  `ctx.platform.can_quit()`. Screens read the facts; the rules crate never
  does.

## On Linux and in a cloud agent's container

- Run `bash scripts/setup-linux.sh` once before the first build; it is
  safe to run again. It installs the C linker, `pkg-config`, the system
  libraries the engine links and opens, and the pinned Rust.
- `cargo shuyi doctor` builds the game first, so it can't report a
  missing library; the script installs everything doctor checks.
- A cloud agent's container starts bare: run the script first, or name
  it in the agent's setup step.
- `.devcontainer/devcontainer.json` runs it in a dev container or a
  GitHub Codespace.

## The tools

`cargo shuyi <command>` runs the engine's tools on this game, in its own
tool program (`crates/game/examples/tools.rs`, with the game's tools,
`game::stages::tools()`). Each takes `--json` for an
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
| `cargo shuyi modules` | the engine's modules by concern (perspective, bodies and motion, online play, kind of game), each with its status, and the ones this game has on |
| `cargo shuyi add <module>` | turns a module on (its feature of `shuyi` in `Cargo.toml`), says what it is for, and warns of an alternative already on; the game already installs every module it turns on (`shuyi::enabled_modules`) |
| `cargo shuyi doctor` | what your toolchain lacks and how to install it |
| `cargo shuyi explain SH2003` | what an error's code means and how to fix it; `explain` alone lists every code |
| `cargo shuyi mcp install` | registers the engine's MCP server with your coding agents (Claude Code's `.mcp.json` and others it finds): each command is a tool, and `start`, `step`, `scene` and `frame_png` play the game and look at it. The entry builds the server from the game's engine, and follows it: a worktree's calls go to a server built from that worktree, and a server whose engine moved under it hands its calls to a fresh one. The dev container runs it for you when it is created |

`scene`, `replay` and `bots` need a stage and rules: a stage's own
`GameTools` gives them (a game made with `shuyi new --template grid`
has one).

## The skills

`.claude/skills/` holds the engine's skills, a folder each: how to do a
kind of task on this game (start, test, add a screen or a stage, replay
a bug, balance it, ship it), what to run, and what each problem code
means. Before a task, read the skill it needs (`SKILL.md`); its worked
example (`example.ron`) is the same steps as commands, which the engine's
CI runs on a new game. `cargo shuyi add <module>` adds a module's skill
when it turns the module on. `cargo shuyi check-skills .claude/skills`
checks a skill the game writes for itself.

## The short path

The game is on the engine's short path: `crates/game/src/lib.rs` names
its files, its data type (`BasicData<UiAction, stages::Own>`: the text,
the keys, the palette and the font from `files/`, and its stages' own
part), its first screen (the title) and every screen it can show in a
`BasicGame`, which installs the engine's modules its `Cargo.toml` turns
on (`shuyi::enabled_modules`); `src/main.rs` runs it with the prelude's
`run_game`; `examples/tools.rs` gives its tools, `game::stages::tools()`
(the engine's `BasicTools`, or a stage's own on top of them). A screen
names what it needs from `shuyi::prelude`.

Who the game is, its name, id and version, is `shuyi.ron`'s `app`.
`cargo shuyi app-info` writes it into `crates/game/src/app_info.rs`,
which the game reads (`app_info::APP`): run it after each change there,
never edit the file. `cargo shuyi check --target all` fails while the
file is out of date, and warns while the id is the placeholder
`com.example.…`: set your own before you hand out a build, since the
folder the player's saves are kept in follows it. Its icon is `app.icon`,
one square PNG of 512 pixels or more: `app-info` downscales it into the
window's icons (`crates/game/icon`), which `APP` carries.

## The stages

`crates/game/src/stages.rs` lists the game's stages, in order: `shuyi
new` wrote it from the parts it was made with. The title's Confirm
starts the first. Each stage is a module, `crates/game/src/stages/`,
that gives:

- `Own`, its part of the game's data, and `own`, which reads it from the
  files (a map, a table of rules), or `()` and `Ok(())`;
- `first`, the screen the title starts, and `screens`, every screen it
  can show;
- `Tools`, made with `Tools::new(game())`: its `GameTools`, which adds
  what the stage has (a scene, records, bots) and calls `BasicTools` for
  the rest.

`stages.rs` names them: change it by hand to add or take out a stage.

## When the game outgrows the short path

Grow it by adding a part, not by rewriting the start:

- **Its own actions:** an enum of its own that implements `ActionSet`
  (a debug key, a map's actions), in place of `UiAction`, with its
  names in `files/keymap.ron`.
- **A part of its own:** a stage's `Own`, read by its `own` function
  from the files (a stage, a table of rules): it is `BasicData::own`.
- **Its own tools:** a stage's `GameTools` (a scene, records, bots),
  which calls `BasicTools`' methods for the rest. `shuyi new --template
  grid` shows one.
- **The long form:** its own `GameData` (sounds, voices, themes,
  settings of its own, `GameHooks`), and macroquad's entry point calling
  `shuyi::app::run` with its `AppConfig`.

## The gates

Tests are the review. Before every commit, run what CI runs
(`.github/workflows/ci.yml`), and never weaken one to pass it:

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo shuyi check
cargo shuyi check --target all
cargo shuyi check-keys
cargo shuyi lint
```
