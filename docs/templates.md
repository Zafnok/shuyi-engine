# Templates

A template is what `shuyi new` makes a game from: a folder of data with a
`template.ron` and the files of its **parts**. The answers a game picks
choose the parts, and the composer turns the template and the answers
into the new game's files ([ADR-0013](adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)
§3, [ADR-0042](adr/0042-the-template-format-merged-ron-and-cargo-files-a-generated-list-of-stages-and-short-names.md)).
No template runs code: there are no hooks and no scripts, and `new` only
writes files.

The engine's own template is [`templates/shuyi-default`](../templates/shuyi-default/),
embedded in the `shuyi` binary. It is the example on this page.

## Making a game

```
shuyi new moon-garden --template grid
shuyi new moon-garden --answers answers.ron
shuyi new moon-garden --set perspective=top-down-grid
```

The answers come from, in order, each in place of the one before for
the questions it answers:

1. `--template <name>`, a short name of the template's for a set of
   answers (`grid`, `blank`);
2. `--answers <file.ron>`, an answers file;
3. each `--set <question>=<answer[,answer]>`.

A question none of them answers takes its default. `new` refuses a
folder that isn't empty; `--force` writes into it, and its answer lists
the files it wrote over. Its answer (`--json`, the `new` schema) lists
the files made, the parts, the modules turned on, the stages, the skills
(below), and in its `limits` what the template didn't do (an answer no
part serves).

An answers file maps each question to the ids of its answers:

```ron
{
    "perspective": ["top-down-grid"],
}
```

## `template.ron`

```ron
Template(
    id: "shuyi-default",
    name: "Shuyi Engine's own template",
    version: "1.0.0",
    engine: ">=0.1, <0.2",
    license: "MIT OR Apache-2.0",
    format_version: 1,
    questions: [
        Question(
            id: "perspective",
            text: "Which perspectives?",
            pick: Many,
            answers: [
                Answer(id: "top-down-grid", text: "a top-down grid, turn by turn"),
            ],
            default: [],
        ),
    ],
    short_names: {
        "blank": {},
        "grid": { "perspective": ["top-down-grid"] },
    },
    parts: [
        Part(id: "base", when: Always, files: "base", targets: ["desktop", "web"]),
        Part(
            id: "stage-top-down-grid",
            when: Picked("perspective", "top-down-grid"),
            files: "stages/top-down-grid",
            stage: Some("top-down-grid"),
            requires: ["base"],
        ),
    ],
)
```

| Field | What it is |
| ----- | ---------- |
| `id`, `name` | The template's id and how people call it |
| `version` | Its own version (semver) |
| `engine` | The engine versions it makes games for, a semver requirement: `new` refuses a template that doesn't match its engine |
| `license` | The license of the files it writes, an SPDX expression |
| `format_version` | The template format it is written in: `1` |
| `questions` | Its own questions (below) |
| `short_names` | Names for sets of answers, which `--template` takes |
| `parts` | Its parts, in the order their files are written and merged |

Every struct takes no field it doesn't know. `template.ron` is a tool
file, not a game asset, so it isn't in the asset registry.

### `Question`

| Field | What it is |
| ----- | ---------- |
| `id` | What `Picked`, the answers file and `--set` name |
| `text` | The words that ask it |
| `pick` | `One` (one answer at most) or `Many` |
| `answers` | Each `Answer(id, text)` |
| `default` | The answers picked when none is given; none if left out |

### `Part`

| Field | What it is |
| ----- | ---------- |
| `id` | What `requires` and `conflicts` name |
| `when` | When it applies (below) |
| `files` | Its folder, from the template's: each file under it is written at the same path from the game's folder |
| `modules` | The engine's modules it turns on (`physics`), as `shuyi add` does: their features of `shuyi` in the game's root `Cargo.toml` |
| `targets` | The targets it ships on |
| `stage` | The stage it adds, if any (below) |
| `requires` | The parts it needs picked too |
| `conflicts` | The parts it can't be picked with: `new` says which answers picked each |

### `When`

- `Always`;
- `Picked("question", "answer")`: the question has that answer picked;
- `All([...])`: each holds (`All([])` always does);
- `Any([...])`: one holds.

## What `new` checks

Reading a template, before anything is written, with the file and the
field of each problem:

- `template.ron` is there, is RON and has no field it doesn't know;
- `format_version` is 1, and `engine` matches the engine;
- each question, answer and part has its own id; each default and each
  short name's answer is an answer of its question, and a `One`
  question has one at most;
- each part's `files` is a folder inside the template that holds a
  file;
- each `requires` and `conflicts` names a part, and each `Picked` names
  a question and one of its answers;
- each `stage` is lowercase words joined by `-`.

Composing, for the answers picked: the answers are the template's, each
picked part's `requires` is picked and no `conflicts` is, each module is
one the engine builds, and the files merge (below).

## Placeholders

A text file of a part, and the name of any file, may hold these, and no
other `{{`:

| Placeholder | Becomes |
| ----------- | ------- |
| `{{name}}` | The game's name, as given (`Moon Garden`) |
| `{{crate_name}}` | Its crate's name (`moon-garden`) |
| `"{{engine.<crate>}}"` | With its quotes: the dependency on that engine crate (`shuyi`, `core`, `assets`, `ui`, `platform`, `app`, `cli`), a TOML inline table: the engine's repository at `main` or a commit, or a checkout |
| `{{engine_ref}}` | The engine's commit the game builds with, as a workflow's `uses: Zafnok/shuyi-engine/…@` names it: the 40-digit `--engine-rev`, else `main` |
| `{{answer.<question>}}` | That question's answers, joined by `,` |

A GitHub Actions expression, `${{ … }}`, is left as it is. An unknown
placeholder fails, naming its file and line. There are no conditions
and no loops: a condition is a part. That keeps every file valid Rust,
TOML or RON.

## Merged files

Each file is written by the parts whose folders hold it. A file two
parts write is merged by the engine, never by text:

- **every `Cargo.toml`**: key by key, each with its comments. A table
  both have is merged the same way, an array gains the items it lacks,
  and a key both have with different values fails, naming it. The
  modules the parts turn on are then the `shuyi` features of the root
  `Cargo.toml`;
- **every `.ron` file** (`shuyi.ron`, `en.ron`, `palette.ron`, …): the
  first part's text is kept with its comments, and the next part's
  struct fields and map entries it lacks are added, each with its
  comments; a struct or a map both have is merged the same way, a list
  gains the items it lacks, and anything else both have must be the
  same;
- **`crates/game/src/stages.rs`**, the flow's list of stages, which
  `new` writes when the template has a part with a `stage`, from the
  parts picked. No part writes it.

Any other file two parts write fails, naming both.

The built-in template's `stage-top-down-grid` part writes, beside the
files of its own, a `Cargo.toml` with its workspace's dependencies, a
`crates/game/Cargo.toml` with its game crate's, a `shuyi.ron` with its
`rules`, `check_text` and the folder `check_keys` adds, and the
`en.ron` and `palette.ron` of its stage's text and colours.

## Stages

A part with `stage: Some("top-down-grid")` writes the module
`crates/game/src/stages/top_down_grid.rs` (the id with `_` for `-`). It
gives:

- `Own`, the stage's part of the game's data (`BasicData`'s own part),
  and `own(&Bundle)`, which reads it from the game's files;
- `first(ctx)`, the screen the title starts, and `screens(ctx)`, every
  screen it can show;
- `Tools`, with `Tools::new(game)`: the game's `GameTools`, which add
  the stage's and call `BasicTools` for the rest.

`stages.rs` declares the module and names these for the base part:
`stages::Own`, `stages::own`, `stages::first`, `stages::screens`,
`stages::tools` and `stages::names`. With no stage, they are `()`, none,
and `BasicTools`. A game has one stage for now; a second part with a
stage fails.

## The skills

Every game gets the engine's skill pack (`skills/`,
[ADR-0043](adr/0043-the-skill-pack-skills-at-the-root-worked-examples-run-by-the-xtask-check-skills-in-the-cli-and-the-pack-folder-as-the-plugin.md))
in `.claude/skills/<name>/`, whatever its template: the composer adds
them after the parts' files, from the engine the game builds with (the
checkout at `--engine-path`, or the pack the `shuyi` binary embeds). It
leaves out a module's skill (one named after a module's crate, built or
not: `shuyi add` copies it when the game turns the module on) and a skill
whose folder a part already writes. `new`'s answer lists them in
`skills`.

## The engine's template

| Part | When | What it writes |
| ---- | ---- | -------------- |
| `base` | always | The game crate on the short path (`BasicGame`, `BasicData`, `run_game`, `BasicTools`), its title, its files, `shuyi.ron`, `AGENTS.md`, CI with the gates |
| `stage-top-down-grid` | `perspective` is `top-down-grid` | A top-down grid stage: a rules crate over `TurnRules`, the stage's map, lights and seed as `BasicData`'s own part, and its tools (`--seed`, its scene, records, bots, its map's importer) |

`--template blank` is `base` alone, and `--template grid` is both. CI
makes a game with each short name and runs its gates.
