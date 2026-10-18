# ADR-0013: Starting a game: a wizard over the catalog, and templates made of parts

- **Status:** Accepted
- **Date:** 2026-10-17
- **Related tickets:** 2501 (templates as data and the composer), 2502
  (the wizard), 2503 (several perspectives in one game), 2504 (targets in
  the wizard), 2505 (templates from anywhere), 1129 (`shuyi new`), 2003
  (the catalog and `shuyi add`), 2004 (guides), 3001 and 3004 (targets),
  1134 (skills), 2205 and 2206 (MIDI), 3017 (capabilities and needs),
  3108 and 3109 (the 3D templates)
- **Extends:** ADR-0005 §1–§2 (the contract and its surfaces), ADR-0009
  §2, §4 and §6 (concerns, module metadata, templates are directions),
  ADR-0003 §1 (stages and the flow), ADR-0008 §1 (target profiles),
  ADR-0011 §2 (the direction menu of `shuyi new`), ADR-0012 §1
  (capabilities and needs)

## Context

Nick, 2026-10-17 (`docs/decisions.md`): "some create-react-app type
command with templates and let it be extensible too". It should ask "do
you want top down? side scroller?", let the developer "tick off
multiple" because a game "can be both (i.e. transition between them)",
and ask about physics and "what kind of physics", MIDI sound, and "what
export targets they want to support".

What the plans have:

- **`shuyi new <template>`** (1129) copies one of two fixed templates,
  `grid` or `blank`.
- **The catalog** (2003, ADR-0009 §6) lists modules by concern in
  `docs/modules.ron`. Each module has metadata: its concern, its answer,
  the directions it serves, its alternatives. `shuyi add <module>` turns
  one on.
- **ADR-0009 §6** says a template is a direction and turns on only the
  modules its direction needs. That gives one template per direction,
  and with several perspectives, physics choices, music choices and
  targets, the number of combinations grows past what anyone can keep as
  separate templates.
- **Target profiles** (3001) and **`shuyi export`** (3004) exist per
  target, and the game manifest gets a `targets` table.
- **Capabilities and needs** (ADR-0012 §1, the parked 3017): each
  profile says what its hardware can do, each module states its `needs`,
  and `capabilities_fit` says whether a game fits a target.
- **The 3D directions** (ADR-0011 §2, parked 3101–3109) add first
  person, third person and top-down 3D to the perspective concern, and a
  *World drawing* concern whose answers include HD-2D. ADR-0011 §2 asks
  `shuyi new` to list the directions as a menu when no template is
  named, and 3108 builds that menu.

What other tools do:

- **create-react-app** (deprecated in 2025) took one template name and
  asked nothing. Its successors **create-vite** and **create-next-app**
  ask a few questions, and every question also has a flag.
- **Yeoman** generators and **cargo-generate** let anyone publish a
  template. cargo-generate runs Rhai hook scripts from the template, and
  Yeoman runs JavaScript, so using a stranger's template runs their code
  on your machine.
- **Angular schematics** and **Nx generators** compose a project from
  several small generators that each add a piece.

## Decision

### 1. The questions are data, and the catalog writes most of them

`shuyi new` with no template asks questions. They come from three
places:

1. **The engine's own:** the game's name and folder, its perspectives,
   and its targets.
2. **The catalog** (`docs/modules.ron`, 2003): one question per concern.
   The answers are the catalog's answers for that concern. Adding a
   module to the catalog adds its answer to the wizard, with no change
   to the wizard.
3. **The template's own** questions (§3), for anything a template needs
   that isn't a concern.

Each concern in `docs/modules.ron` gains two fields:

- `pick`: `One` or `Many`. Perspective and targets are `Many`. Bodies and
  motion is `Many`, because ADR-0009 §2 lets alternatives share a game
  in different stages or on different things. Online play is `One`.
- `question`: the words the wizard shows, such as "How do bodies move?".

The answers to "How do bodies move?" are the catalog's: the movers and
drawn clips of the default engine (ADR-0004 §3–§4), rigid-body physics
(`shuyi-physics`, 1128), and procedural animation (`shuyi-procanim`,
2701). That is the question Nick called "what kind of physics". A game
with only grid stages isn't asked it, because the grid kit moves its
pieces itself.

The 3D directions are answers like any other: first person, third
person and top-down 3D under "Which perspectives?", and HD-2D under the
*World drawing* concern (ADR-0011 §2). The direction menu ADR-0011 §2
asks for is this wizard's perspective question; where the wizard
exists, no separate menu is written.

The time concern (turns or ticks) is not asked. It follows from the
perspectives: a grid stage takes turns, every other stage takes ticks.
A template can ask it when a game needs both.

### 2. Every answer has a flag, and an agent gets the questions as JSON

The wizard is a surface over the contract (ADR-0005 §2), like the CLI
and MCP:

- **`shuyi new <dir> --questions --json`** returns every question, its
  answers, which answers are ready, and why the others aren't. An agent,
  or the MCP server (1133), asks its user and then calls `new` with the
  answers.
- **Flags.** Every question has one: `--perspective top-down,side-view`,
  `--bodies kit,rigid`, `--music recorded`, `--targets desktop,web`,
  `--online none`, and `--set <id>=<value>` for a template's own
  questions.
- **`--answers <file.ron>`** reads them all from a file.
- **Nothing is only interactive.** A question left out takes its default,
  and with `--no-input` (or when stdin isn't a terminal) the wizard asks
  nothing.
- At the end, the wizard prints the one command that makes the same game
  with no questions, and writes the answers into the new game's
  `shuyi.ron` under `created_with`.

### 3. A template is a folder of data, made of parts

A template is a folder with a `template.ron` and the files it writes:

```ron
Template(
    id: "shuyi-default",
    name: "Shuyi Engine's own template",
    version: "1.0.0",
    engine: ">=0.2, <0.3",
    license: "MIT OR Apache-2.0",
    questions: [ /* the template's own (§1, item 3) */ ],
    parts: [
        Part(id: "base", when: Always, files: "base/"),
        Part(id: "stage-top-down-grid",
             when: Picked("perspective", "top-down-grid"),
             files: "stages/top-down-grid/",
             modules: [], stage: Some("top-down-grid")),
        Part(id: "physics-demo",
             when: Picked("bodies", "rigid"),
             files: "bodies/rigid/", modules: ["physics"]),
    ],
)
```

- **A part** is a set of files plus what it turns on: modules (through
  the same code as `shuyi add`), targets, and a stage. It says `when` it
  applies: `Always`, `Picked(question, answer)`, or `All`/`Any` of those.
  A part may `require` or `conflict` with other parts, and the wizard
  explains a conflict in words before writing anything.
- **Placeholders, not a template language.** A file may use
  `{{name}}`, `{{crate_name}}` and the answers' ids. There are no loops
  and no conditions inside files; a condition is a part. That keeps
  every file valid Rust or RON, so the engine's CI builds each template
  part as it is.
- **Shared files are merged by the engine, not by text.** `Cargo.toml`
  features, the `shuyi.ron` manifest, the stage list of the flow screen,
  the targets table and the CI matrix are built from what every part
  declares. Any other file written by two parts is an error that names
  both parts.
- **The composer is pure.** `compose(&Template, &Answers) -> Result<Plan,
  ComposeError>` returns the files and their bytes. Writing them is a
  separate step. The same answers give the same bytes, which a golden
  test checks.
- **The engine's own templates use this format.** 1129's `grid` and
  `blank` become parts of `templates/shuyi-default`, and `shuyi new
  <dir> --template grid` stays as a short name for a set of answers.

`template.ron` is a tool file, not a game asset, so it isn't registered
in the asset registry (ADR-0007). It still follows its rules: it is
text, it takes `deny_unknown_fields`, it carries a format version, and a
fuzz-style test feeds it arbitrary input.

### 4. Several perspectives make several stages and a flow between them

When more than one perspective is picked:

- each perspective's part adds its stage (ADR-0003 §1), generic over its
  marker (ADR-0009 §3);
- the base part's flow screen lists the stages in the order picked,
  starts in the first, and moves between them;
- for a pair with a **move guide** (ADR-0009 §5), the move part of the
  perspective that owns the guide is used, such as the touch that starts
  a battle (1125);
- for a pair with no move guide yet, the flow uses a plain door: an
  action named `travel` in the game's action set, bound in the default
  keymap (rule 8, never a hard-coded key). The new game's README says
  that move has no guide yet;
- the new game has a Harness test that crosses each move and back.

### 5. Targets are checked against the modules picked

- Picked targets fill the manifest's `targets` table (3001) with the
  profile's defaults.
- Before writing anything, the wizard runs the capabilities check
  (`capabilities_fit`, ADR-0012 §1, 3017) on the picked modules' `needs`
  against each picked target, and refuses a game that doesn't fit, with
  the check's own words: "module shuyi-3d needs three_d: software;
  target gba has three_d: none". No second list of targets per module is
  kept: `needs` is the one place a module says what hardware it wants.
  Until 3017 is built, the wizard skips this check and says so in
  `limits`.
- The new game's CI runs `shuyi check --target` and `shuyi export` for
  each picked target.
- A target with its own duties brings its part. With `playdate` picked,
  the new game gets a palette pattern map (3006) and the controls check
  (3002) in its CI.

### 6. Planned answers are shown, not offered

An answer whose module is planned or parked in the catalog is listed with
its status and its ticket ("procedural animation: parked, 2701"). It
can't be picked, and its flag gives the same message. The catalog says
what is coming, and the wizard never makes a game that can't build.

### 7. Templates from anywhere, and none of them run code

`--template` takes:

- a short name of the engine's own (`grid`, `blank`);
- a local folder;
- a git repository at a revision: `--template
  git+https://host/owner/repo@<rev>`. The revision is required, so the
  same command always gives the same game. `shuyi-cli` runs the `git`
  program to fetch it; no git library is added.

Rules for every template:

- **No code runs when the game is made.** A template has no hooks and no
  scripts. `shuyi new` reads data and writes files. The game's own code
  runs only when its developer builds it.
- **Before writing,** the wizard shows a template not shipped with the
  engine: where it comes from, its revision, its license and its
  version. A license outside ADR-0002 §8 gets a warning, not a refusal:
  the game is its developer's.
- **`shuyi template check <dir>`** lints a template: the manifest, every
  part's files, the placeholders, and that every combination of its
  answers composes without a conflict nobody explained. With `--build`,
  it makes a game for each answer combination its parts change and builds
  them.
- **`shuyi template new <dir>`** writes a small template to start from.
- There is no template registry. `docs/templates.md` says how to write
  and share one.

## Consequences

- A developer gets a game that fits what they said they want, in one
  command, and sees what the engine doesn't do yet.
- Every module is a wizard answer as soon as it is in the catalog. A
  module's ticket adds a part only when its answer needs starter files.
- Agents and CI make games the same way people do, from flags, an
  answers file or JSON.
- The template format is a public interface. Changing it needs a format
  version bump, and `shuyi template check` must keep old templates
  readable or say how to upgrade them.
- Combinations multiply. CI can't build every one, so it builds each
  part alone over the base, every pair of perspectives, and one game
  with every ready answer picked (2502).
- With no hooks, a template can't do anything the format doesn't
  describe. Anything one needs is a request for the format.

## Alternatives considered

- **One template per direction** (ADR-0009 §6 as written) — simple, but
  several perspectives times bodies, music and targets is too many
  templates to keep, and each one copies the others.
- **A template language with conditions in files** (Tera, Handlebars, as
  in cargo-generate and Yeoman) — powerful, but files with `{% if %}`
  aren't valid Rust until rendered, so CI can't build the parts as they
  are, and a mistake shows up only in some combinations.
- **Hook scripts in templates** (cargo-generate's Rhai, Yeoman's
  JavaScript) — they make templates more flexible, but using a stranger's
  template would then run their code before anyone read it.
- **A template registry** — a service to run and moderate. A git URL
  with a revision does the job until someone asks for more.
- **Questions only, no flags** — agents and CI couldn't use it, which
  ADR-0005 rules out.
