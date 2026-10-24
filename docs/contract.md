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
| `lint` | Everything wrong with the shape of the game's code, each finding with its file, line, problem code and what to do: `check-keys`' findings (`SH2101`), `check-text`'s (`SH2102`: every literal when there are more than `check_text.max_literals`, and every data name a screen reads), and the rules crate's fence ([ADR-0032](adr/0032-the-layers-as-data-checked-over-cargo-metadata-a-float-ban-in-the-simulations-and-a-new-games-rules-crate-kept-pure.md) §4) on the package `shuyi.ron`'s `rules` names: `cargo clippy` read for `disallowed_types`, `disallowed_methods` and `float_arithmetic`, with clippy's reason (`SH2103`, with the column), and `cargo tree` for a screen, device or tool crate or outside luck under it (`SH2104`, with the `cargo tree -i` that shows the way in). A part `shuyi.ron` lacks is a limit | the project file and cargo |
| `modules` | The catalog of the engine the game builds with ([`modules.md`](modules.md)), by concern: each answer, who gives it (`default` or a module's crate), its status and its tickets; every module the engine builds, with its feature of `shuyi` and its metadata (what it answers, serves and sits beside); and the modules the game has on, from its `Cargo.toml` | the game's `Cargo.toml` and its engine |
| `add <module>` | Turns a module of the engine on (`physics` or `shuyi-physics`): its feature of `shuyi` in the game's `Cargo.toml`, its layout kept; its skill copied into `.claude/skills/` if the game keeps one; its README's first paragraph. It warns of an alternative already on, with the README's section on using both; gives the line to add if the game's code never installs `shuyi::enabled_modules`, and the line for its `formats()` if the module adds formats. A second run changes nothing (`changed: false`); a name that isn't built fails with `SH0003` and the names that are | the game's `Cargo.toml` and its engine |
| `run [steps] [--frames N]` | Starts the game headless, plays the steps, runs N frames more: the top screen, the screens, the scene, the frame as text, what happened, the sounds, the [log](#the-log), and the modules the game installed with the hooks each declared (`before_update`, `draw_over`), its state's types and its debug menu entries | the game |
| `scene [steps]` | The same, answering with the scene or stage on screen and the log | the game |
| `replay <record> [<other>]` | Re-runs a record by the game's rules: a state hash per step, and the first step where two runs differ | the game |
| `bots <games>` | The game's bots play: how each play ended, how long it took, the game's measures, a record of each | the game |
| `playtest [--stage <id> \| --every-stage] [--entrant <name>]... [--tries N] [--seed N] [--turns N] [--threads N] [--report <file>]` | Each of the game's named bots (its entrants) plays N tries from the same seeds; the game's checks say per bot whether it succeeded as often as its band wants (✓, ✗ too hard, ⚠ too easy) and compare medians between bots. The tries run on every core (`--threads`), each timed: every entrant's answer has its seconds per try, the one number that changes with the machine. The report never depends on the thread count. `--stage` picks one of the game's stages (needed when it has several), `--every-stage` plays each in turn, a report each; `--entrant` picks entrants or a group of them (the game's choice by default, all of them if it has none); a number not given is the game's default (100 tries from seed 0, to turn 200, unless it says). `--report` also writes the JSON answer to a file. It fails only when a bot falls under a check's floor | the game |
| `autobalance [--stage <id>] [--entrant <name>]... [--tries N] [--quick N] [--budget N] [--time <seconds>] [--seed N] [--turns N] [--threads N] [--apply]` | The game's knobs change one of its stages, never its bots, until each of its entrants succeeds as often as its checks' bands want (`shuyi_core::autobalance_by`): the smallest change the search finds, each in the knob's own words, why it stopped, and the reports before and after. The stage as authored and the one found play `--tries` tries; the search's own runs play `--quick` (a fifth of them). Each setting's tries run on every core and are judged once in seed order, so what it finds never depends on `--threads`. It stops at `--budget` evaluations (100) or, with `--time`, after that many seconds, with the best setting seen: a limit says so, since that depends on the machine. `--apply` has the game write the changed stage to its files, and the answer lists them. It fails while a rate is outside its band | the game |
| `import <file> [--kind K] [--print]` | Reads a file through the format registry as every kind that knows it, or gives its errors | the game's kinds, or the engine's |
| `bake [--target <id>] [--out <file>]` | Bakes the game's sources into one pack ([`formats.md`](formats.md#the-pack)): what went in, the sizes, how long baking and loading took; refuses sources with errors | the game |
| `doctor` | Checks Rust, Cargo, rustup's targets, MSYS2's linker on the Windows GNU toolchain, and the web target, and says how to fix each | nothing |
| `explain [<code>]` | Every problem code, or one code's page: what it means and how to fix it ([below](#problem-codes)) | nothing |
| `schema [<name> \| --out <dir>]` | The contract's JSON schemas | nothing |

A step is `--script "Enter Up"` (or `--keys`: each key chord pressed and
released in turn), `--pad "South"` (controller buttons, by position) or
`--wait 0.5` (seconds). Key and button names are the Harness's test-script
names. The game's own start options (`frame-png --help` lists them, such
as Hello, Grid's `--stage` and `--seed`) go to `run`, `scene`, `bots`,
`playtest` and `autobalance` too.

From a game's folder, `cargo shuyi <op>` runs them (the alias the
templates write in `.cargo/config.toml`, or the engine's for its own
examples).

## The answer

Every operation answers with the same shape, `Answer<T>`:

```json
{
  "contract": 16,
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
`shuyi_cli::contract::codes::TABLE`; the content codes of the game's
files are `shuyi_assets::error::codes`, and those of its code, `SH21nn`
(`lint`), are the table's.

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
`code`. The tools that run the game, and `lint`, take `project`, the
game's folder from the session's folder (the session folder's own
project if it is left out). `lint` runs in the server, not the game's
tool program.

The **session's folder** is the client's first root: a client that
declares `roots` in `initialize` gets a `roots/list` request after
`notifications/initialized` (Claude Code lists the session's working
folder, a worktree's in a worktree session). Without roots it is the
server's working folder.

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
given. For Claude Code it also sets `MCP_TIMEOUT` to 5 minutes in the
project's `.claude/settings.json`, so Claude Code waits for the server's
first build to finish. After writing, it runs that entry with `--ready`
and says how long the build took.

On Windows the server, and the game's tool program it starts, move their
own program into `.shuyi-running/` beside it as they start, and leave a
copy in its place, so a build while they run writes a fresh program
instead of waiting for them to stop.

The entry builds the server from the project's own engine (a game's
moves with its pin). `<command> mcp --ready` builds the server and the
game's tool program and prints `mcp: ready (contract N)`, so a first
connection doesn't wait for a build.

**Each project gets its own server.** A client may start the server in
another checkout than the session's: Claude Code's desktop app starts a
worktree session's server in the repository's main checkout. So each
call's project is looked up to its cargo workspace (`cargo
locate-project --workspace`, once per project root), and a project in
another workspace than the server's goes to that workspace's
**forwarded server**: a `shuyi mcp` built from that workspace's engine,
one per workspace, kept for the session. It is the project's `command`
and `mcp`, run in its root; or, with no `command` in its `shuyi.ron` (or
no `shuyi.ron`, as at the engine repository's root), the `shuyi` entry
of the workspace's `.mcp.json`, run there; or `shuyi mcp` in the
project's root. A folder outside any workspace, or with neither a
project nor an entry, is the server's own. The call goes on with its
`project` (and `new`'s `dir`) made absolute, and its answer comes back
as it is. When the session's folder is in another workspace, its server
starts at the roots' answer, so its build runs while the agent works; a
call waits for it. One that doesn't build, or stops, answers `SH3005`
with the last of its output, and the next call starts it again. A
forwarded server forwards nothing itself (it runs with
`SHUYI_MCP_FORWARDED` set), and it ends when its stdin closes, as the
server does, so nothing runs from a worktree's `target` once its session
ends.

**The tool list follows.** The server declares `tools: { listChanged:
true }`. Once the session folder's forwarded server is ready, the server
fetches its `tools/list`; if it differs from its own, it sends
`notifications/tools/list_changed` and from then lists those tools, and
hands a tool it doesn't know itself to that server.

**A stale server restarts.** When the game's engine speaks another
contract than the server (the checkout moved under a running server), a
call in the server's own workspace goes to a fresh forwarded server of
that workspace, and so does every later one. Only if that one doesn't
start does the answer get a limit naming both contracts.

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
| `playtest_plan`, `playtest_stages`, `playtest_defaults` | `playtest` | a `PlaytestPlan` for a stage: a title, its entrants by name (each with a label, a mode, if it has one), named groups of them (`"all"`), the default ones, its checks over the entrants that play (a data file of `shuyi_core::bots::playtest::Check`s, or a `playtest::Targets` file by tier whose `checks` gives a stage's, keeping those whose `Check::entrants` all play), and a `Sync` function playing one try of one entrant from one seed, most simply `playtest::play_tries` over that seed; its stages' ids, if it has several; and its own default tries, seed and turn limit, if not the engine's |
| `autobalance_plan` | `autobalance` | an `AutobalancePlan` for a stage: a title, its entrants, groups, default entrants and checks as its `PlaytestPlan` has them, and the stage it may change, a boxed `Tunable` over the game's own stage type `S`: the stage as authored, its knobs (`shuyi_core::Knob<S>`, in the order the search tries them), a `Sync` function playing one try of one entrant from one seed on a given stage (most simply `playtest::play_tries` over that seed), and one writing a changed stage to the game's files, giving the files it wrote. Its stages and tries are `playtest`'s |
| `importers` | `import` | `Importers::standard()` and its own formats and kinds |
| `bake` | `bake`, `check --baked` | a `GameBake`: its `Bake` (the assets it bakes and its stamp), the bundle of its sources for the target, and its formats |

A game on the short path (a `shuyi_platform::BasicGame`) gives
`shuyi_cli::BasicTools::new(game)`: `frame` and `frame-png`, `lang`,
`check` (its files' errors), `session` (`Headless` over its Harness, with
no scene) and `controls` (its keymap and every screen it can show). A
game that adds a scene, a record, bots, importers or a bake writes its
own `GameTools` and calls `BasicTools`' methods for the rest.

Hello, Grid's `src/tools.rs` writes every one. The grid template's
`crates/game/src/tools.rs` writes every one but `bake`, which a new game
leaves out until it chooses a pack, and `autobalance_plan`, which needs
knobs of its own stage.

A state hash is `shuyi_core::state_hash`, over any `Serialize` state: the
same on every platform and every run, if the state's maps are ordered.

## The templates

`shuyi new grid <dir>` and `shuyi new blank <dir>` write a game that
builds and passes its own gates as it is:

- `grid`: a rules crate over `TurnRules` (walk to the goal while a
  wanderer takes a random step after each of yours), a game crate with
  one stage screen painted by the engine's glyph skin, a bot, records,
  and every operation of the contract wired;
- `blank`: one screen with the game's name, on the short path: its
  `lib.rs` names its files, its data type (`BasicData<UiAction>`) and its
  first screen in a `BasicGame`; its `main` is one call (the prelude's
  `run_game`); its tool program is `BasicTools`.

Each has its crates, `shuyi.ron`, `AGENTS.md` (and `CLAUDE.md`), CI with
the gates (`fmt`, `clippy`, the tests, `check`, `check-keys`; `grid`'s
also `lint`), and the
`cargo shuyi` alias. The game crate depends on the engine through the
`shuyi` crate, its front door, with no module on: the templates' own
directions need none. Each installs `shuyi::enabled_modules` once (`grid`
in its `Game::with_modules`, `blank` in its `BasicGame::modules`), so
`add` needs no change to the game's code. `grid`'s rules crate depends on
`shuyi-core` and `shuyi-assets` alone, which keeps it pure. The game
depends on the engine's repository at `main`, at `--engine-rev
<commit>`, or on the checkout at `--engine-path <dir>`. CI makes a game
from each template on every pull request and runs its gates.
