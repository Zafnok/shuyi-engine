# The tool contract

The operations an agent, CI or a person runs on a game, each with a
typed input and a typed answer
([ADR-0005](adr/0005-ai-native-tooling-one-contract-three-surfaces-and-visual-scripting-as-a-view-over-text.md),
[ADR-0022](adr/0022-the-tool-contract-answers-with-limits-schemas-by-version-and-a-session-over-the-harness.md)).
They are library functions in `shuyi_cli::contract`
(`crates/cli/src/contract.rs`), and `shuyi <op>` runs them on the
command line. The MCP server, `shuyi mcp` ([below](#the-mcp-server)),
is another way to reach them.

## The operations

| Op | Does | Needs |
| -- | ---- | ----- |
| `new <template> <dir>` | A new game in `dir` from `templates/grid` or `templates/blank` | nothing |
| `check [--baked]` | Loads the game's content through every validator: every error, with file, line and message. With `--baked`, bakes the game's pack too, and every asset read from it must equal the one read from its source | the game |
| `check --target <id \| all>` | Whether the game's frame at its scale fits a target's display inside its safe area, and its smallest text is as tall as the target asks (ADR-0008 §1): each problem with its target, its setting in `shuyi.ron`'s `targets` and the numbers; with the game's tools, that each screen's required actions fit the target's inputs (ADR-0025), and on a one-bit target that every palette role has patterns and the roles of a `distinct` group stay apart (ADR-0026, each problem with its `roles`); a warning doesn't fail it | the project file; the game for the controls and the patterns |
| `run [steps] [--frames N]` | Starts the game headless, plays the steps, runs N frames more: the top screen, the screens, the scene, the frame as text, what happened, the sounds, the [log](#the-log) | the game |
| `scene [steps]` | The same, answering with the scene or stage on screen and the log | the game |
| `replay <record> [<other>]` | Re-runs a record by the game's rules: a state hash per step, and the first step where two runs differ | the game |
| `bots <games>` | The game's bots play: how each play ended, how long it took, the game's measures, a record of each | the game |
| `playtest [--tries N]` | Each of the game's named bots plays N tries from the same seeds; the game's checks say per bot whether it succeeded as often as its band wants (✓, ✗ too hard, ⚠ too easy) and compare medians between bots. It fails only when a bot falls under a check's floor | the game |
| `import <file> [--kind K] [--print]` | Reads a file through the format registry as every kind that knows it, or gives its errors | the game's kinds, or the engine's |
| `bake [--target <id>] [--out <file>]` | Bakes the game's sources into one pack ([`formats.md`](formats.md#the-pack)): what went in, the sizes, how long baking and loading took; refuses sources with errors | the game |
| `doctor` | Checks Rust, Cargo, rustup's targets, MSYS2's linker on the Windows GNU toolchain, and the web target, and says how to fix each | nothing |
| `explain [<code>]` | Every problem code, or one code's page: what it means and how to fix it ([below](#problem-codes)) | nothing |
| `schema [<name> \| --out <dir>]` | The contract's JSON schemas | nothing |

A step is `--script "Enter Up"` (or `--keys`: each key chord pressed and
released in turn), `--pad "South"` (controller buttons, by position) or
`--wait 0.5` (seconds). Key and button names are the Harness's test-script
names. The game's own start options (`frame-png --help` lists them, such
as Hello, Grid's `--stage` and `--seed`) go to `run`, `scene`, `bots` and
`playtest` too.

From a game's folder, `cargo shuyi <op>` runs them (the alias the
templates write in `.cargo/config.toml`, or the engine's for its own
examples).

## The answer

Every operation answers with the same shape, `Answer<T>`:

```json
{
  "contract": 9,
  "op": "scene",
  "ok": false,
  "result": { "frames": 0, "top_screen": "title", "scene": null, "log": [] },
  "error": null,
  "code": null,
  "help": null,
  "limits": ["no scene: no stage is open (the top screen is `title`)"]
}
```

- `contract` is the contract's version.
- `ok` is the verdict: nothing found by `check`, two runs that agree, a
  game made, a toolchain complete, a scene found. `run` passes when it
  ran.
- `result` is what the operation found or made. It is absent when the
  operation could not run, and `error` says why ("the game's tools don't
  offer bots (GameTools::bots)").
- `code` is the error's problem code (`SH3001`), and `help` how to fix
  it ([Problem codes](#problem-codes)). An answer that ran and failed on
  the setup has them too (`doctor` with a failed check: `SH1003`). A
  problem in a file (`check`'s and `import`'s `errors`) has its own
  `code` and `help` when its class has one.
- `limits` says what the operation could not determine: frames after the
  game quit, a stage that isn't open, a record that ran twice in one
  process only, a kind printed without a writer of our text format.

`shuyi <op>` prints the answer for people; `--json` prints it as one JSON
document. The exit code is 0 when `ok`, else its code's class's: 1 when
it ran and failed, 2 for wrong arguments, 3 for a setup to fix (with
`--json`, a failed answer with the error, its code and its help).

## The log

`run` and `scene` answer with the game's log, `log`: what the engine
and the game logged while it ran (settings it couldn't read, a voice
manifest it refused, a game's own lines), oldest first. Each line is:

```json
{ "frame": 0, "level": "warn", "category": "storage", "text": "settings: unreadable: …, using the defaults" }
```

- `frame` is the frame it was logged in. Frames count from 1; 0 is
  before the first (what the game logged as it started).
- `level` is `error`, `warn`, `info` or `debug`; the log keeps `info`
  and more serious.
- `category` is what it is about: `assets`, `audio`, `input`,
  `screens`, `storage`, `platform`, or `game` for the game's own lines.
- `text` is English, never translated.

The log keeps the last 500 lines. When it dropped older ones, `limits`
says how many. For people, `shuyi run` and `shuyi scene` print the
warnings and errors.

## Problem codes

Every error the contract gives has a stable code, `SH` and four digits
([ADR-0028](adr/0028-problem-codes-a-stable-code-by-class-a-help-line-a-page-and-exit-code-3-for-the-setup.md)).
Its first digit is its class, and the class sets the exit code:

| Class | Codes | Exit code | Means |
| ----- | ----- | --------- | ----- |
| usage | `SH0nnn` | 2 | the arguments or the input are wrong: change the command |
| setup | `SH1nnn` | 3 | the project file or the toolchain needs fixing: fix it, then run the same command again |
| content | `SH2nnn` | 1 | a file of the game's content has a problem |
| operation | `SH3nnn` | 1 | the operation failed on something else: the game's tools, a target, a file it reads or writes |

An answer without a code that isn't `ok` exits 1: it ran and found
something (`check`'s errors, two runs that differ). No exit code means
"retry as it is".

For people, an error reads:

```text
check: error[SH1002]: shuyi.ron names no game tool program (`game`), and check runs the game
  help: fix shuyi.ron as the message says, then run the same command again
  see: shuyi explain SH1002
```

and a problem `files/stage.ron:3:4: error[SH2001]: Expected comma`, with
one `help[SH2001]: …` line after the list per code used.

`shuyi explain` lists every code; `shuyi explain SH2003` (or its name,
`missing-file`) prints its page, from
[`crates/cli/errors/`](../crates/cli/errors/). The table is
`shuyi_cli::contract::codes::TABLE`; the content codes are
`shuyi_assets::error::codes`.

A code is never reused or renumbered: a retired one keeps its row and
its page. **A new error gets a new code, its row and its page in the
same PR**; a test fails on a page with no row, a code twice, or a row no
source uses.

## The schemas

Each operation's input and answer has a JSON schema, generated from the
types and checked in under
[`crates/cli/schemas/`](../crates/cli/schemas/): `run.input.json`,
`run.output.json` and so on. `shuyi schema run.output` prints one.

They change only with the contract's version. A test fails when the
checked-in schemas are not what the types generate, and when their
fingerprint is not the one
[`versions.json`](../crates/cli/schemas/versions.json) lists for
`contract::VERSION`. To change an input or an answer:

1. change the type;
2. `cargo shuyi schema --out crates/cli/schemas` from the repository's
   root;
3. raise `contract::VERSION` by one;
4. add the line the test asks for to `versions.json`.

## The MCP server

`shuyi mcp` serves the contract over MCP, on stdin and stdout. Each
operation is a tool: its input is the operation's input schema
(`run.input.json`), and its result is the operation's answer, as
structured content and as JSON text. An answer that isn't `ok` is an
error result, with its `code` and `help` as they are: branch on the
`code`. The tools that run the game take `project`, the game's folder
from the server's working folder (the server's own project if it is
left out).

A **session** keeps one game running headless between calls:

| Tool | Input | Does |
| ---- | ----- | ---- |
| `start` | `project`, `start` (the game's start options) | builds the game's tool program and starts its game; the answer lists the options it offers. A game already running is stopped |
| `step` | `script`, `frames` | plays the script, then idle frames; answers as `run` does, for everything since the start |
| `scene` | nothing | the scene on screen now, and the log, as `scene` answers |
| `frame_png` | `scale` (1 to 8) | a picture of the frame now, as an image, with its size |
| `events` | nothing | what happened since the start, and the sounds |
| `reset` | nothing | the game started again as `start` started it |
| `stop` | nothing | the game stopped |

Their schemas are `session.<tool>.input.json` and
`session.<tool>.output.json`. The one-off `scene` is the session's here:
`run` answers with a one-off run's scene. A session tool with no game
running answers `SH0002`. The game runs in the game's tool program
(`cargo run`, as `shuyi mcp-host`), which `start` builds, so call
`start` again after changing the game's code. A program that doesn't
build, or stops, answers `SH3005` with the last of its output, and ends
the session. A game paints the session's frame with
`Headless::with_picture`.

`shuyi mcp install` registers the server with the agents it finds, in
the project's folder: Claude Code's `.mcp.json` always, and Cursor's,
VS Code's and Gemini CLI's files where it finds them. It shows the
changes and asks before writing, and keeps each file it changes as
`<file>.bak`. `--yes` doesn't ask; `--dry-run` writes nothing and prints
each file as it would write it (Claude Code's is also what `claude
--mcp-config <file>` reads). The entry runs `--command`'s words and
`mcp`: the project's `command` (`cargo shuyi` in a new game) unless
given.

## What a game gives

The `shuyi` binary knows no game. A game gives its tools in a program of
its own that calls `shuyi_cli::main_with` with its `GameTools`, and
`shuyi.ron`'s `game` names that program. Each operation that runs the
game is a method of `GameTools`, with a default that says the game
doesn't offer it:

| Method | For | What a game writes |
| ------ | --- | ------------------ |
| `check` | `check` | its loader's errors (`ContentError`s) |
| `session` | `run`, `scene`, the MCP session | `Headless::new(harness)`, with `with_scene` (its stage's `MapScene` as text and its state as JSON), `with_events` (its rules' events) and `with_picture` (its `frame-png` painter, for `frame_png`) |
| `replay` | `replay` | its record parsed, and `replay::turns(file, first_state, &commands)` |
| `bots` | `bots` | each play by `shuyi_core::play`, summed up by `bots::tally` |
| `playtest` | `playtest` | its `Entrant`s, `Measure` and checks: a data file of `shuyi_core::bots::playtest::Check`s, or a `playtest::Targets` file by tier whose `checks` gives a stage's; run by `playtest::run` |
| `importers` | `import` | `Importers::standard()` and its own formats and kinds |
| `bake` | `bake`, `check --baked` | a `GameBake`: its `Bake` (the assets it bakes and its stamp), the bundle of its sources for the target, and its formats |

Hello, Grid's `src/tools.rs` writes every one. The grid template's
`crates/game/src/tools.rs` writes every one but `bake`: a new game ships
its loose files until it chooses a pack.

A state hash is `shuyi_core::state_hash`, over any `Serialize` state: the
same on every platform and every run, if the state's maps are ordered.

## The templates

`shuyi new grid <dir>` and `shuyi new blank <dir>` write a game that
builds and passes its own gates as it is:

- `grid`: a rules crate over `TurnRules` (walk to the goal while a
  wanderer takes a random step after each of yours), a game crate with
  one stage screen painted by the engine's glyph skin, a bot, records,
  and every operation of the contract wired;
- `blank`: one screen with the game's name.

Each has its crates, `shuyi.ron`, `AGENTS.md` (and `CLAUDE.md`), CI with
the gates (`fmt`, `clippy`, the tests, `check`, `check-keys`), and the
`cargo shuyi` alias. The game depends on the engine's repository at
`main`, at `--engine-rev <commit>`, or on the checkout at
`--engine-path <dir>`. CI makes a game from each template on every pull
request and runs its gates.
