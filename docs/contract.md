# The tool contract

The operations an agent, CI or a person runs on a game, each with a
typed input and a typed answer
([ADR-0005](adr/0005-ai-native-tooling-one-contract-three-surfaces-and-visual-scripting-as-a-view-over-text.md),
[ADR-0022](adr/0022-the-tool-contract-answers-with-limits-schemas-by-version-and-a-session-over-the-harness.md)).
They are library functions in `shuyi_cli::contract`
(`crates/cli/src/contract.rs`), and `shuyi <op>` runs them on the
command line. The MCP server (ticket 1133) and the skills (ticket 1134)
are the other two ways to reach them.

## The operations

| Op | Does | Needs |
| -- | ---- | ----- |
| `new <template> <dir>` | A new game in `dir` from `templates/grid` or `templates/blank` | nothing |
| `check [--baked]` | Loads the game's content through every validator: every error, with file, line and message. With `--baked`, bakes the game's pack too, and every asset read from it must equal the one read from its source | the game |
| `check --target <id \| all>` | Whether the game's frame at its scale fits a target's display inside its safe area, and its smallest text is as tall as the target asks (ADR-0008 §1): each problem with its target, its setting in `shuyi.ron`'s `targets` and the numbers; a warning doesn't fail it | the project file |
| `run [steps] [--frames N]` | Starts the game headless, plays the steps, runs N frames more: the top screen, the screens, the scene, the frame as text, what happened, the sounds | the game |
| `scene [steps]` | The same, answering with the scene or stage on screen | the game |
| `replay <record> [<other>]` | Re-runs a record by the game's rules: a state hash per step, and the first step where two runs differ | the game |
| `bots <games>` | The game's bots play: how each play ended, how long it took, the game's measures, a record of each | the game |
| `import <file> [--kind K] [--print]` | Reads a file through the format registry as every kind that knows it, or gives its errors | the game's kinds, or the engine's |
| `bake [--target <id>] [--out <file>]` | Bakes the game's sources into one pack ([`formats.md`](formats.md#the-pack)): what went in, the sizes, how long baking and loading took; refuses sources with errors | the game |
| `doctor` | Checks Rust, Cargo, rustup's targets, MSYS2's linker on the Windows GNU toolchain, and the web target, and says how to fix each | nothing |
| `schema [<name> \| --out <dir>]` | The contract's JSON schemas | nothing |

A step is `--script "Enter Up"` (or `--keys`: each key chord pressed and
released in turn), `--pad "South"` (controller buttons, by position) or
`--wait 0.5` (seconds). Key and button names are the Harness's test-script
names. The game's own start options (`frame-png --help` lists them, such
as Hello, Grid's `--stage` and `--seed`) go to `run`, `scene` and `bots`
too.

From a game's folder, `cargo shuyi <op>` runs them (the alias the
templates write in `.cargo/config.toml`, or the engine's for its own
examples).

## The answer

Every operation answers with the same shape, `Answer<T>`:

```json
{
  "contract": 3,
  "op": "scene",
  "ok": false,
  "result": { "frames": 0, "top_screen": "title", "scene": null },
  "error": null,
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
- `limits` says what the operation could not determine: frames after the
  game quit, a stage that isn't open, a record that ran twice in one
  process only, a kind printed without a writer of our text format.

`shuyi <op>` prints the answer for people; `--json` prints it as one JSON
document. The exit code is 0 when `ok`, 1 when not, and 2 for wrong
arguments (with `--json`, a failed answer with the error).

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

## What a game gives

The `shuyi` binary knows no game. A game gives its tools in a program of
its own that calls `shuyi_cli::main_with` with its `GameTools`, and
`shuyi.ron`'s `game` names that program. Each operation that runs the
game is a method of `GameTools`, with a default that says the game
doesn't offer it:

| Method | For | What a game writes |
| ------ | --- | ------------------ |
| `check` | `check` | its loader's errors (`ContentError`s) |
| `session` | `run`, `scene` | `Headless::new(harness)`, with `with_scene` (its stage's `MapScene` as text and its state as JSON) and `with_events` (its rules' events) |
| `replay` | `replay` | its record parsed, and `replay::turns(file, first_state, &commands)` |
| `bots` | `bots` | each play by `shuyi_core::play`, summed up by `bots::tally` |
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
