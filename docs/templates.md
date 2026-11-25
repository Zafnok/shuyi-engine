# Templates

A template is what `shuyi new` makes a game from: a folder of data with a
`template.ron` and the files of its **parts**. The answers a game picks
choose the parts, and the composer turns the template and the answers
into the new game's files ([ADR-0013](adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)
§3, [ADR-0042](adr/0042-the-template-format-merged-ron-and-cargo-files-a-generated-list-of-stages-and-short-names.md)).
No template runs code: `new` only writes files, and runs nothing while it
makes a game. A file a template ships may be a program the developer or
git runs later: `scripts/setup-linux.sh`, which they run, and a git hook
(`scripts/git-hooks/post-checkout`), which git runs only once the game's
`core.hooksPath` names its folder. `new` sets nothing in git; `shuyi
repos`, which makes the game's git repository, sets it (ticket 3402).

The engine's own template is [`templates/shuyi-default`](../templates/shuyi-default/),
embedded in the `shuyi` binary. It is the example on this page.

## Making a game

```
shuyi new moon-garden --template grid
shuyi new moon-garden --answers answers.ron
shuyi new moon-garden --no-input --perspective top-down-grid --targets desktop,web \
    --id io.github.you.moon-garden
shuyi new moon-garden --set perspective=top-down-grid
shuyi new moon-garden --template blank --set private-files=yes \
    --private-url https://github.com/you/moon-garden-assets.git
```

The answers come from, in order, each in place of the one before for
the questions it answers:

1. `--template <name>`, a short name of the template's for a set of
   answers (`grid`, `blank`);
2. `--answers <file.ron>`, an answers file;
3. each `--set <question>=<answer[,answer]>`, and each of the wizard's
   flags (below: `--perspective top-down-grid`, `--targets desktop,web`),
   in the order given.

A question none of them answers takes its default: the template's
`default`, or a concern's first ready answer, so `shuyi new demo
--no-input` makes the same game as `--template grid`. `new` refuses a
folder that isn't empty; `--force` writes into it, and its answer lists
the files it wrote over. Its answer (`--json`, the `new` schema) lists
the files made, the parts, the modules turned on, the stages, the skills
(below), and in its `limits` what the template didn't do (an answer no
part serves, but a question's default, which adds nothing by design:
`private-files` = `no`).

In a terminal, `shuyi new` asks the wizard's questions (below) that
nothing given answers, the engine skills' worked examples among the
template's (`examples`, below; `--examples`, `--no-examples`, `--set
examples=…` or the answers file answer it). It then asks whether
to put the game on GitHub (with `gh`, signed in), each repository's
visibility, and which kinds of files go in a private repository
(`files/text`, `files/sprites`, `files/sounds`, `files/music`,
`files/data`, and folders of the developer's own). With a private
repository it answers `private-files` = `yes` and sets `--private-url` to
`https://github.com/<owner>/<crate name>-assets.git` itself, writes the
game, then runs `shuyi repos` in it ([ADR-0112](adr/0112-shuyi-repos-makes-a-games-repositories-keys-and-secrets.md)).
Each question has its flag (`--examples`/`--no-examples`,
`--github`/`--no-github`, `--owner`,
`--visibility`, `--assets-visibility`, `--private <kinds>|none`,
`--private-folder <path>`). Off a terminal, with `--json` and from the
MCP server's `new` tool, nothing is asked and no repository is made
(`--github` is refused with `--json`); there `--private` adds the part
only with `--private-url`, and the examples are in unless `examples` is
`no`.

## The wizard's questions

`shuyi new` asks its questions from three places (ADR-0013 §1), in this
order:

1. the engine's: the game's folder, its name (`--name`, the folder's
   name without) and its reverse-DNS id (`--id`, checked as `app-info`
   checks it; `com.example.<crate name>` without), which `{{id}}` fills;
2. the perspectives, the catalog's `perspective` concern;
3. one question per concern of the catalog, [`modules.ron`](modules.ron),
   in its order: each concern's `question`, taking one answer or many
   (its `pick`). `time` isn't asked: it follows from the perspectives,
   and a template that needs both asks its own;
4. the template's own `questions`;
5. the targets, from the engine's target profiles (`targets/`), the
   template's always-on parts' targets by default.

Every question has a flag: `--perspective`, `--bodies`, `--online`,
`--world-drawing`, `--kind`, `--scripting` (one per concern, named by
its id, several answers with commas), `--targets`, and `--set
<question>=<answers>` for the template's (`--examples`/`--no-examples`
for `examples`). An answer whose status in the catalog is `planned` or
`parked`, or a planned target, is shown with its tickets and never
picked: by flag it fails, naming its status and its tickets
(ADR-0013 §6).

A question isn't asked, and takes its default, when:

- none of its answers is for the perspectives picked: a concern's
  `serves` names the perspectives its answers are for, and bodies and
  motion is for the free ones, since the grid kit moves its pieces
  itself;
- fewer than two of its answers are ready or planned, so there is
  nothing to pick between (every concern but the perspectives, which are
  always asked);
- another question answers it: `private-files` is answered by the GitHub
  questions, or by `--set private-files=yes` with `--private-url`.

With `--no-input`, as off a terminal, nothing is asked: each question
the flags leave takes its default, and no repository is made.

`shuyi new <dir> --questions --json` lists the questions for an agent,
for the answers given so far, and makes nothing: each with its `id`, who
asks it (`asker`: `engine`, `catalog` or `template`), its `text`, what it
`takes` (`text`, `one` or `many`), its `answers` (each with its `id`, its
`text`, its `status` in the catalog and, if it isn't `ready`, its
`tickets`), its `default`, its `flag` and, when it isn't asked, why
(`skipped`). Its shape is `new.questions.json` among the contract's
schemas. The answers given change what is asked, so an agent asks the
perspectives first, lists again with `--perspective`, then asks the
rest. Without `--json` it prints the same for people.

The GitHub questions follow the targets: `github` (`--github` or
`--no-github`), `visibility`, `keep-private` (no is `--private none`),
`private` (the kinds), `private-folder` and `assets-visibility`, each
asked by the engine. Those after `github` are skipped unless `--github`
is given. `github`'s `agent` field says what an agent does with it: ask
its user, and never answer `yes` itself, since `--github` makes
repositories, keys and secrets on their account. No other question has
an `agent`.

At the end, `new` prints the one command that makes the same game again
with nothing asked, also its answer's `command`: `shuyi new <dir>
--no-input --name <name>`, `--id` if given, each concern's answers by
its flag (none as `--set <id>=`), `--targets`, `--examples` or
`--no-examples`, `--set` for the template's other questions, the engine
(`--engine-rev`, `--engine-path`) and `--private-url`. Each word is
quoted so PowerShell and a POSIX shell read it the same; a word no quote
keeps in both is quoted for a POSIX shell, and a limit says so.

The game's `shuyi.ron` keeps how it was made, under `created_with`: the
template, its version and every answer, the defaults among them. No
tool reads it.

```ron
created_with: (
    template: "shuyi-default",
    version: "1.0.0",
    answers: {
        "examples": ["yes"],
        "perspective": ["top-down-grid"],
        "targets": ["desktop", "web"],
        // …
    },
),
```

A picked answer that is a module (its `by` is a crate) turns the module
on as `shuyi add` does, through a part that names it or, with none, by
the composer itself. `shuyi.ron`'s `targets` are the template's parts'
until 2504: a target picked beside them is a limit. Asking the questions
In a terminal, each is asked that nothing given answers (a flag, `--set`,
the short name or the answers file), in the order above, then the GitHub
questions:

- the name and the id are a line of text, checked as `new` checks them
  (the id as `app-info` does) when Enter is pressed; Enter alone takes
  the default shown;
- a question taking one answer is a list to pick from (the arrows move,
  Enter picks), and one taking many a list to tick (space ticks, Enter
  ends), its default picked or ticked; a help line under each list names
  the keys;
- only `ready` answers are in a list: the `planned` and `parked` ones
  are under it, greyed, each with its status and tickets, and can't be
  picked;
- the perspectives come first, and the questions are listed again with
  them, so a concern skipped for them isn't asked; a question that isn't
  asked isn't shown, and takes its default (`--questions` lists every
  question, each skipped one with why: the view to check their wording
  in);
- Esc stops at a question, and nothing is made.

The answers go into the game as their flags would, so the command `new`
prints at the end makes the same game with nothing asked.

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
            id: "examples",
            text: "Include the engine skills' worked examples?",
            pick: One,
            answers: [Answer(id: "yes", text: "yes"), Answer(id: "no", text: "no")],
            default: ["yes"],
        ),
    ],
    short_names: {
        "blank": { "perspective": [] },
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
| `id` | What `Picked`, the answers file and `--set` name: never a concern's of the catalog, nor `targets`, which the engine asks |
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
  a question of the template's, or a concern of the engine's catalog,
  and one of its answers;
- each `stage` is lowercase words joined by `-`.

Composing, for the answers picked: the answers are the template's or
the catalog's (a concern's must be `ready`), each
picked part's `requires` is picked and no `conflicts` is, each module is
one the engine builds, and the files merge (below).

## Placeholders

A text file of a part, and the name of any file, may hold these, and no
other `{{`:

| Placeholder | Becomes |
| ----------- | ------- |
| `{{name}}` | The game's name, as given (`Moon Garden`) |
| `{{crate_name}}` | Its crate's name (`moon-garden`) |
| `{{id}}` | Its reverse-DNS id, `--id` (`com.example.<crate name>` without), as `shuyi.ron`'s `app` and `app_info.rs` hold it |
| `"{{engine.<crate>}}"` | With its quotes: the dependency on that engine crate (`shuyi`, `core`, `assets`, `ui`, `platform`, `app`, `cli`), a TOML inline table: the engine's repository at `main` or a commit, or a checkout |
| `{{engine_ref}}` | The engine's commit the game builds with, as a workflow's `uses: Zafnok/shuyi-engine/…@` names it: the 40-digit `--engine-rev`, else `main` |
| `{{engine.docs}}` | A Markdown list of the engine's rules and docs for the game's agent. At `main`: `AGENTS.md` and the site's `llms.txt` and `llms-full.txt`. At a commit: `AGENTS.md` and the book's Markdown at that commit. In a checkout: the same two, as paths |
| `{{private_url}}` | The game's private repository, `--private-url` (`https://github.com/<owner>/<repo>.git`). Only a game with private files has one: `new` needs the flag when `private-files` is `yes` and refuses it otherwise, and a file that holds the placeholder fails without it |
| `{{answer.<question>}}` | That question's answers, joined by `,` |

A GitHub Actions expression, `${{ … }}`, is left as it is. An unknown
placeholder fails, naming its file and line. There are no conditions
and no loops: a condition is a part. That keeps every file valid Rust,
TOML or RON.

## Manifests are `Cargo.toml.tmpl`

A part's manifest is named `Cargo.toml.tmpl` in the template's folder,
and the game gets it as `Cargo.toml`: everything below, and `new`'s
output, names it `Cargo.toml`. Why: a game that depends on the engine
by git makes cargo read every `Cargo.toml` of the engine's repository,
and a template's `name = "{{crate_name}}"` printed errors in each of the
game's builds (ticket 2067). A template that names its manifests
`Cargo.toml` still reads; a folder that holds both fails, naming the
`.tmpl`. `cargo xtask check-pins` and `check-toolchain` read the
template's `Cargo.toml.tmpl` files.

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
- **every `.gitignore`**: the first part's lines, then each line of the
  next part's it lacks, with the comments and blank lines just above it;
- **every Markdown file** (`.md`): the first part's text, then each next
  part's after a blank line. The private files' part adds its section to
  the base's `AGENTS.md` and `README.md` this way;
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
- `crash_record(game)`, its record of play for a crash report, or
  `None` when it isn't open (docs/crash-reports.md);
- `Tools`, with `Tools::new(game)`: the game's `GameTools`, which add
  the stage's and call `BasicTools` for the rest.

`stages.rs` declares the module and names these for the base part:
`stages::Own`, `stages::own`, `stages::first`, `stages::screens`,
`stages::crash_record`, `stages::tools` and `stages::names`. With no
stage, they are `()`, none, and `BasicTools`; the base's `game()` gives
`BasicGame::crash_record` the stage's hook when there is one. A game has one stage for now; a second part with a
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

The composer reads one question itself, `examples`
([ADR-0113](adr/0113-a-game-made-without-the-skills-worked-examples-says-so-in-each-skill.md)):
no part serves it, and its `no` is no limit. With `examples` = `no` each
skill comes without its worked examples (`example.ron`, `example-*.ron`
and the `example/` folder), its other files kept. A link of its Markdown
to one of them becomes the link's words, and its `SKILL.md` ends with a
sentence saying the game was made without them and that they are in the
engine's `skills/<name>/`, at the version the game builds with.
`shuyi check-skills` asks no worked example of a skill whose `SKILL.md`
says so, so the game's `cargo shuyi check-skills .claude/skills` passes
either way.

## The engine's template

| Part | When | What it writes |
| ---- | ---- | -------------- |
| `base` | always | The game crate on the short path (`BasicGame`, `BasicData`, `run_game`, `BasicTools`), its title, its files (with its menu sounds: `files/audio.ron` and four `.sfx.ron` sound effects in `files/sfx/`, the `sfx` feature of `shuyi-assets` turned on, ticket 2225), `shuyi.ron`, `AGENTS.md`, CI with the gates, and its web build (`shuyi.ron`'s `web`, `crates/game/build.rs` and the web target in `rust-toolchain.toml`) |
| `stage-top-down-grid` | `perspective` is `top-down-grid` | A top-down grid stage: a rules crate over `TurnRules`, the stage's map, lights and seed as `BasicData`'s own part, and its tools (`--seed`, its scene, records, bots, its map's importer) |
| `private-files` | `private-files` is `yes` | The game's private files ([ADR-0111](adr/0111-a-games-private-files-a-second-repository-laid-over-its-files-whenever-checked-out.md)): `shuyi.ron`'s `private_assets` (the repository `{{private_url}}`, checked out into `private/` at the commit `private.rev` names), an empty `private.rev` (no pin yet), `/private/` in `.gitignore`, the CI step's action `.github/actions/private-files`, and a section of `AGENTS.md` and `README.md` |

Its questions, after the catalog's (the perspectives among them):

| Question | Pick | Answers | Default |
| -------- | ---- | ------- | ------- |
| `private-files` | one | `yes`: the game's writing, art and sounds in a second, private repository; `no` | `no` |
| `examples` | one | `yes`: each skill's worked example; `no`: the skills alone (above) | `yes` |

`--template blank` is `base` alone (no perspective), and `--template
grid` is `base` and the grid stage, as no answer at all is; neither has
private files. CI makes a game with each
short name, the grid one with private files and without the worked
examples, runs its gates (`check-skills` among them) and builds it for
the web.

### The private files

The base's build lays the game's private files over its files in every
game, so a game can add the part later: `crates/game/build.rs` embeds
`private/files/` when it is there (else an empty folder, and for an
edition with its own content root always the empty one), and
`src/lib.rs` hands it to `BasicGame::private`, a private layer read
before every other, the folder `run --watch` lays over the files
included. The part adds the rest: what fetches the checkout, ignores
it, checks it out in CI before the gates (failing a run on the game's
own repository without the secret `PRIVATE_ASSETS_KEY`, warning on a
fork's or Dependabot's pull request) and tells the game's agent of it.
The action installs `openssh-client` first when the job's image has no
`ssh`, which the keyed checkout runs: Valve's sniper image, which
`export.yml` builds the Linux package in, has none (ticket 3504;
`every_container_job_with_the_private_files_has_ssh_for_their_checkout`
checks every container job that checks the files out).
No workflow of the template uploads an artifact **or saves a cache**
that embeds the private files (ADR-0111 §5 as
[ADR-0130](adr/0130-no-cache-of-a-build-with-the-private-files-and-who-can-read-them.md)
amends it; `crates/cli/src/contract/new/tests.rs`'s
`every_job_with_the_private_files_caches_no_build_and_uploads_none`
reads every workflow to check it), but `export.yml`'s package of a release:
a private repository's as it is, a public one's sealed first with
`PRIVATE_ASSETS_KEY`, which only the release's jobs open
([ADR-0133](adr/0133-a-release-publishes-nowhere-by-default-debug-files-in-a-draft-sealed-packages.md)).
A tag's release is a draft unless the developer asks for a public one,
and its debug files are in a draft of their own. A pull request's run, a fork's too, may
restore its base branch's caches, so with the part's action there the
base's `ci.yml` caches only the cargo registry, not `target/`
(`rust-cache`'s `cache-targets`). The grid stage's
`playtest.yml` passes the same secret to the engine's playtest
workflow, which runs the part's action before the bots' build when the
game has it ([`contract.md`](contract.md), *The playtest in CI*). The private
repository's own layout (`files/`, `docs/`, `library/`) is in ADR-0111.
