---
id: "2501"
title: "Templates as data: template.ron, parts that apply when an answer is picked, and a pure composer behind shuyi new"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: done
blocked_by: ["1129", "2003", "2511"]
nick_input: none
completed: 2026-10-24
---

# 2501 — Templates as data, and the composer

## Context

Nick, 2026-10-17 (`docs/decisions.md`): a "create-react-app type command
with templates", which is "extensible too". Several perspectives,
physics, music and targets give too many combinations to keep as whole
templates.
[ADR-0013](../../docs/adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)
§3 makes a template a folder of data made of **parts**, and a pure
composer that turns a template and a set of answers into files.

This ticket builds the format and the composer, and moves 1129's two
templates (`grid`, `blank`) onto them. 2511 has put `blank` on the
short path; the grid stage moves onto it here, so it is rewritten once.
Nothing asks questions yet: answers come from flags and from an answers
file. The questions are 2502, several perspectives are 2503, targets
are 2504, and templates from outside the engine are 2505.

It needs `shuyi new` (1129) and `shuyi add` with module metadata (2003),
because a part turns modules on through the same code as `shuyi add`.
It needs 2511's short path, which the `base` part is.

## Nick input

None.

## Scope

**In:**
- **Types in `shuyi-cli`, module `template`:**
  - `Template { id, name, version, engine: VersionReq, license,
    format_version: u32, questions: Vec<Question>, parts: Vec<Part> }`;
  - `Part { id, when: When, files: PathBuf, modules: Vec<String>,
    targets: Vec<String>, stage: Option<String>, requires: Vec<String>,
    conflicts: Vec<String> }`;
  - `When::{Always, Picked(question, answer), All(Vec<When>),
    Any(Vec<When>)}`;
  - `Question { id, text, pick: One | Many, answers: Vec<Answer>,
    default }` for a template's own questions. 2502 fills the engine's
    and the catalog's;
  - `Answers`: a map from question id to the picked answer ids,
    serializable as `answers.ron`.
  - All of them take `deny_unknown_fields`. `format_version` is 1.
- **Loading:** `Template::load(dir)` reads `template.ron` and checks:
  every part's `files` folder exists, every `requires` and `conflicts`
  names a part, every `Picked` names a question and an answer, and
  `engine` matches the running version. Each error names the file and
  the field.
- **The composer**, pure: `compose(&Template, &Answers, &Catalog) ->
  Result<Plan, ComposeError>`.
  - It picks the parts whose `when` holds, checks `requires` and
    `conflicts`, and returns `Plan { files: BTreeMap<PathBuf, Vec<u8>>,
    modules, targets, stages }`.
  - **Placeholders:** `{{name}}`, `{{crate_name}}` (1129's `{{crate}}`,
    renamed), 1129's `"{{engine.<crate>}}"`, 2812's `{{engine.docs}}`
    (in the base part's `AGENTS.md`) and `{{answer.<question>}}` in file
    contents and file names. `${{ … }}` (a GitHub Actions
    expression) passes through untouched, as 3050 makes `new`'s `fill`
    do. Any other unknown placeholder is still an error naming the file
    and line. There are no conditions or loops.
  - **Merged files:** `Cargo.toml` (the `shuyi` features from every
    part's `modules`, through 2003's code), `shuyi.ron` (the manifest)
    and the flow's stage list (`src/stages.rs`, generated from every
    part's `stage`). Any other path written by two parts is
    `ComposeError::SameFile { path, parts }`.
  - A `ComposeError::Conflict` says in words which answers led to the
    two parts.
- **Writing** is a separate function, `apply(&Plan, dir)`. It refuses a
  folder that isn't empty, unless `--force`, and then lists what it
  would overwrite first.
- **`shuyi new`**, extended:
  - `--answers <file.ron>` and `--set <question>=<answer[,answer]>`;
  - `--template grid` and `--template blank`, short names for answer
    sets of the built-in template. **From 1129 (done):** `new` takes
    its template as the first of two positional arguments today
    (`shuyi new grid <dir>`, `NewInput { template, dir, .. }`), and
    refuses any other `--` flag. Add `--template <name>` beside a lone
    `<dir>` (`shuyi new demo --template grid`), and keep `shuyi new
    <template> <dir>` working. `NewInput`'s schema changes, so bump
    `contract::VERSION`, with a `Breaking:` line if a field goes.
    `docs/contract.md` (*The templates*), the README and the MCP `new`
    tool show the new form in the same PR;
  - the result (ADR-0005) lists the files made, the modules turned on,
    and `limits` (for example "no part for answer X; nothing added").
- **The built-in template**, `templates/shuyi-default/`, replaces 1129's
  two folders:
  - part `base` (always): 2511's blank template on the short path
    (`BasicGame`, `BasicData`, `run_game`, `BasicTools`), with
    `shuyi.ron`, `AGENTS.md`, the flow screen and CI with the gates. It
    also carries what these tickets add to 1129's two folders, for each
    one that is done. One that isn't done puts its files in `base`
    itself, as 2225, 2223, 2513, 3039, 3048 and 3050 say in their
    *Either order* clauses:
    - 2225's menu sounds: in `crates/game/files/`, `audio.ron` and
      `sfx/move.sfx.ron`, `sfx/select.sfx.ron`, `sfx/cancel.sfx.ron` and
      `sfx/cursor.sfx.ron`, with `shuyi-assets` back in the root
      `Cargo.toml`'s `[workspace.dependencies]` (2003 took it out of the
      blank template and left `shuyi`, `shuyi-cli` and `include_dir`
      there), its `sfx` feature on in `crates/game/Cargo.toml`, and the
      sound test in `tests/it/main.rs`.
      On the short path `BasicData::load` and `run_game` play them, so
      the grid template's `sound_files()` hook goes with its long form;
    - the credits file `crates/game/files/credits.ron` (2223) and
      `shuyi.ron`'s `notices` (3039) and `credits` (2223) parts, so
      every new game is credited and checked. The grid stage's
      `stage.map` joins the `credits` part's `own` list through the
      merged `shuyi.ron`;
    - 3050's `release.yml` and `export.yml`, the `ci.yml` with the
      Cargo.lock step and `--locked`, and the README and `AGENTS.md`
      "Releasing" sections. The test that keeps the engine's own
      `export.yml` equal to the template's reads the base part's copy;
    - the `app` part in `shuyi.ron` and the generated
      `crates/game/src/app_info.rs` (3048), with `{{crate}}` renamed
      `{{crate_name}}` in both, and a test that a composed game passes
      `cargo shuyi app-info --check`;
    - 2115's bullet under *The layers* in each template's `AGENTS.md`
      (time a rule in turns or steps with `shuyi_core::timer`, never
      with `dt`), in the base part's `AGENTS.md` if 2115 is done. If
      not, nothing to do: 2115 writes it in `base` (its clause says
      so);
    - 2513's `scripts/setup-linux.sh` and
      `.devcontainer/devcontainer.json` (`{{name}}` in the latter). Its
      tests that keep each copy equal to the engine's read the base
      part's copies, and CI's `templates` job runs the composed game's
      script;
  - part `stage-top-down-grid`, when perspective is `top-down-grid`:
    1129's grid screen as a stage, on the short path. Its map, lights
    and seed are `BasicData`'s own part (`BasicGame::with_own`), with
    `()` settings and `run_game`. A `GameTools` of its own keeps what
    the grid game adds today (the seed option on `frame`, the session's
    scene and events, `replay`, `bots` and the map importer) and calls
    `BasicTools` for the rest. The PR gives the composed grid game's
    code count before and after, from `cargo xtask template-lines`
    (2512), or by 2511's counting rule if 2512 isn't done;
  - **From 2015 (done):** the grid template's rules crate has a fence,
    and it moves with the part that holds `crates/rules`
    (`stage-top-down-grid`): `crates/rules/clippy.toml`, the float ban
    in `crates/rules/src/lib.rs` and `crates/rules/tests/layers.rs`.
    The rules crate's dependencies (`shuyi-core`, `shuyi-assets`,
    `serde`), which blank's root `Cargo.toml` lacks, come with the part:
    the merged `Cargo.toml` takes a part's `[workspace.dependencies]`,
    or the rules crate names them itself. Grid's `AGENTS.md` sentences
    on clippy and the layers test (*The layers*) must not be lost when
    `base` writes `AGENTS.md` (two parts can't write one file): base's
    *The layers* says that the grid stage's rules crate comes with
    `crates/rules/clippy.toml` and `crates/rules/tests/layers.rs`, which
    turn these rules into errors, and that a failure is fixed in the
    code, never by loosening them. The paths that name
    `templates/grid/crates/rules/tests/layers.rs` move in the same PR:
    `layers.rs`'s `a_new_games_rules_test_keeps_a_copy_of_the_devices`
    (`crates/xtask`), `lint`'s tests in `crates/cli` (2534), the comment
    on `devices` in `tools/layers.ron` and the device crates' row in
    `docs/README.md`. CI's steps "The rules crate's fence holds" and
    "shuyi lint finds what breaks the game's code" keep running on the
    composed grid game;
  - `blank` is `base` alone;
  - **either order with 3108 and 3109** (parked): if either is done
    first, it wrote template folders (`first-person`, `third-person`,
    `top-down-3d`, `hd-2d`). Turn each into a part the same way, keep
    its name as a short name, and keep 3108's direction menu working
    over the short names. If not, nothing to do: each writes its
    templates as parts when it lands (their clauses say so).
- **Embedded:** the built-in template is embedded in the `shuyi` binary
  (`include_dir`, believed MIT OR Apache-2.0: check its `Cargo.toml`
  and `cargo deny` on the day; or a build step that writes it as
  bytes), so `shuyi new` works with no checkout of the engine.
- **The format's page**, `docs/templates.md`: the fields, `When`,
  placeholders, merged files, with the built-in template as the example.
- **Either order:**
  - **2512** (`cargo xtask template-lines`): if it is done, move it onto
    the composer. It counts each built-in short name (`blank`, `grid`)
    from `compose`'s plan, with the placeholders filled as `shuyi new`
    fills them and no folder written, and `tools/line-budget.ron` names
    the short name `blank`. If not, 2512 does it (its clause says so).
  - **2516** (`GameTools::assets`): if it is done, the grid part's
    `GameTools` writes `assets` as `BasicTools` does, now that the grid
    game loads through `BasicData`. If not, 2516 writes it (its clause
    says so).
  - **2353** (the pseudo-language walk): if it is done, the grid part
    drops the grid template's own `with_pseudo` line, since
    `BasicData::load` adds the pack, and keeps its walk test. If not,
    2353 adds the line to `BasicData::load` only (its clause says so).
  - **1134** (the skill pack): if 1134 is done, the skills `shuyi new`
    copies into a new game join `compose`'s plan, read from the engine's
    `skills/` as `new` reads them (not a second copy in the template),
    so every composed game has them and `new`'s answer lists them. If
    not, 1134 adds them to the plan (its clause says so).
  - **2110** (save migrations): if it is done, `base`'s `AGENTS.md`
    keeps the blank template's "Saves" section, and the grid template's
    settings fixtures (`crates/game/tests/saves/settings/`), its saves
    test and its "Saves" text go with its settings file, since the stage
    takes the `()` settings, which the engine versions and checks. Where
    the kept text points at the grid template's test, it names
    `check_saved_forms` instead. If not, 2110 writes the "Saves" section
    in `base` only (its clause says so).
  - **2374** (an eight-way choice in the grid template): if it is done,
    carry the choice it added to `templates/grid` into the grid part. If
    not, 2374 adds it to the grid part (its clause says so).
  - **2206** (parked: MIDI playback): if it is done, it has added its
    `music-midi` part (a `.mid`, a cue and the bank line, when `music`
    has `midi` picked) to the built-in template's list above; build it
    with the other parts. If not, nothing to do (2206 writes the part).
  - **2517** (format upgrades): if 2517 is done, carry its template
    changes. The grid part's `GameTools` registers `KeymapRon::<Action>`
    in `importers()` with the `KeymapDef<Action>` kind, beside the map
    importer, and the base part's `AGENTS.md` keeps the row for
    `cargo shuyi upgrade` in its table of tools. If not, nothing to do:
    2517 puts them in the parts (its clause says so).
  - **2812** (`{{engine.docs}}`): if 2812 is done, the base part's
    `AGENTS.md` keeps the placeholder, and `compose` fills it from the
    game's `Engine` as `new`'s `files()` did, with its test. If not,
    nothing to do: 2812 adds it to the base part and to `compose` (its
    clause says so).

**Out (do not do):**
- asking questions, and the catalog's questions (2502);
- more than one perspective, and moves between stages (2503);
- targets checks and per-target CI (2504);
- templates from a path or git, `shuyi template check` and `new` (2505);
- any hook or script that runs when a game is made (ADR-0013 §7:
  never). A script the developer runs later, like 2513's
  `scripts/setup-linux.sh`, is a file like any other.

## Implementation steps

1. Write the types, `Template::load` and its checks, with fixture
   templates for each error.
2. Write `compose`, placeholders and the merged files. Use the code 2003
   wrote for `shuyi add` to edit `Cargo.toml`; don't write a second one.
3. Write `apply` and the `new` flags.
4. Move 1129's `templates/grid` and `templates/blank` into
   `templates/shuyi-default` as parts, and embed it. Count the grid
   game's code, move its stage onto the short path, and count it again.
   Move `.github/dependabot.yml`'s github-actions `directories` to the
   parts' workflow folders in the same PR (2011's zizmor job reads `.`,
   which finds every `.github/workflows` folder, so it needs no change). 2011's
   `cargo xtask check-pins` fails until they name them, and it also
   checks that the parts' `[workspace.dependencies]` requirements equal
   the root's. Move the `rust-toolchain` update's `directories` to the
   parts' `rust-toolchain.toml` folders too: 2012's `cargo xtask
   check-toolchain` (rule 5) fails until they name them.
   No gate checks the paths that name the old folders by a glob, and a
   search for `templates/grid` misses them: point the
   `dependency-upkeep` skill's step 3 (`templates/*/Cargo.toml`) and
   the docs map's pair rows (`docs/README.md`: `templates/*/AGENTS.md`,
   `templates/*/.github/workflows/`) at the parts' folders.
5. Point CI's `templates` job (it makes a game from `grid` and from
   `blank` and runs its gates) at the new template.
6. Write `docs/templates.md` and link it from the README. Settle the
   *Either order* clause and the files `base` carries over.

## Acceptance criteria

- [x] `shuyi new demo --template grid` makes a game that builds and
      passes its tests in CI, as 1129's did.
- [x] `shuyi new grid <dir>` and `shuyi new blank <dir>` still work,
      as CI's `templates` job runs them.
- [x] A game composed with `--template grid` has
      `crates/rules/clippy.toml`, the float ban and `tests/layers.rs`,
      its `AGENTS.md` names them, and CI's fence step passes on it.
- [x] `shuyi new demo --answers fixtures/answers.ron` makes the same
      bytes as the equivalent `--set` flags (a test compares the plans).
- [x] Composing the same answers twice gives the same bytes (golden
      test on the plan's file list and hashes).
- [x] Each `Template::load` and `ComposeError` case has a fixture and a
      message naming the file, field or parts.
- [x] Two parts writing one unmerged file fail with both parts named.
- [x] `shuyi new --json` output matches its schema (1129's schema test).
- [x] `docs/templates.md` exists and is linked from the README.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `When` evaluation; placeholder replacement, including unknown
  ones and a `${{ matrix.target }}` expression passing through; each
  load check; each merged file.
- Property: for any answer set over a fixture template, `compose`
  either returns a plan or a `ComposeError`, never panics, and gives the
  same plan twice.
- Fuzz-style: arbitrary bytes into the `template.ron` loader never panic.
- Integration: `shuyi new` with `grid` and `blank` into a temporary
  folder, then `cargo build` and `cargo test` there (CI).

## Completion notes

Done on 2026-10-24.

**What was done:**

- `shuyi_cli::template`: `Template` (with `short_names`, below),
  `Part`, `When`, `Question`, `Pick`, `Answer`, `Answers`, all
  `deny_unknown_fields`, `format_version` 1. `Template::load(dir)` and
  `Template::read(origin, files, engine)` check everything the scope
  lists, each error naming the file and the field (`LoadError`).
  `built_in()` reads the embedded `templates/shuyi-default`.
- The composer, pure: `compose(&Template, &Answers, &Catalog, &Fill) ->
  Result<Plan, ComposeError>`, placeholders (`fill`: `{{name}}`,
  `{{crate_name}}`, `"{{engine.<crate>}}"`, `{{answer.<question>}}`,
  `${{ … }}` passed through), merged files, the generated
  `crates/game/src/stages.rs` (`stages_rs`), and `apply(&Plan, dir,
  force)`. Modules turn on through `catalog::turn_on`, 2003's code.
- `shuyi new <dir>` with `--template`, `--answers`, `--set` (repeated)
  and `--force`; its answer adds `template`, `answers`, `parts`,
  `overwritten`, `modules`, `targets` and `stages`, and the plan's
  limits. The contract is version 17. `new <template> <dir>`, the older
  form, still works.
- `templates/shuyi-default`: part `base` (2511's blank template on the
  short path, with a title that starts the first stage) and part
  `stage-top-down-grid` (1129's grid screen as a stage, on the short
  path: `BasicGame::with_own` with its map, lights and seed as its own
  part, `()` settings, `run_game`, and a `Tools` of its own with the
  seed option, the scene and events, `replay`, `bots` and the map
  importer, calling `BasicTools` for the rest). `blank` and `grid` are
  short names. The rules crate's rules moved into the part's
  `crates/rules/AGENTS.md`.
- CI's `templates` job, `.github/dependabot.yml`, `THIRD_PARTY_ASSETS.md`,
  the docs map, `docs/contract.md`, the README, `docs/templates.md` (new,
  linked from the README) and ADR-0042.
- Tests: `When`, placeholders, each load check, each `ComposeError`,
  each merged file, the RON merge, `apply`, a property test over answer
  sets, fuzz-style tests of the manifest and the RON merge, the
  `--answers`/`--set`/short name plans compared, a golden of both short
  names' plans (`crates/cli/src/template/plans.golden`), and `new`'s
  answer against its schema. Both games made from this checkout pass
  their own gates (fmt, clippy, tests, `check`, `check --target all`,
  `check-keys`, `lint`), and the grid game was played through the MCP
  session tools.

**Code counts** (2511's rule: `crates/game/src` and
`crates/game/examples`, no blank or comment lines, no `#[cfg(test)]`
module): the grid game 674 lines before, 521 after (its rules crate
unchanged, 202); the blank game 75 before, 115 after (the generated
`stages.rs` and the title's line for starting a stage), within 2511's
120.

**Deviations:**

- `compose` takes a fourth argument, `&Fill` (the name, the crate name
  and the engine's dependencies), so it stays pure.
- Every `Cargo.toml` and every `.ron` file merge, not only the root
  `Cargo.toml` and `shuyi.ron`: the grid stage adds its dependencies to
  the game crate's manifest and its text and colours to `en.ron` and
  `palette.ron`, which `BasicData` reads alone (ADR-0042 §1).
- The flow: the title is always the first screen, and its Confirm
  starts the first stage, so the base's tests hold in every game; the
  grid game's scripts start with `Enter`. A game has one stage until
  2503 (ADR-0042 §2).
- `short_names` is a field of `template.ron` (ADR-0042 §3).
- The `perspective` question is the template's own until 2502 asks the
  engine's.
- With no checkout, `new` reads the catalog the binary embeds
  (`Catalog::embedded`).
- `cargo machete` skips the grid part's rules crate (its parent
  `Cargo.toml` is a piece of the workspace, not one) and says so; it
  exits 0, and the made games' own clippy covers them.

**Either order:** 2512, 2516, 2353, 2225, 2223, 2513, 2115, 3039,
3048, 3050, 2808, 1134, 2110, 2374, 2206 and 2517 aren't done, so their
pieces fell to them. 3108 and 3109 haven't landed. Each now says what it
does as "**From 2501 (done):**" (work-ticket §5, steps 5 and 6), with
2117, 2342, 2809 and the parked 2551 and 2771 settled too.

**Handed over** (step 6): the template's own `perspective` question and
the one-stage limit to 2502; the generated `src/stages.rs`, its names
and lifting the one-stage limit to 2503; `short_names` and
`Template::read` to 2505; the one-stage limit to 2771; the tool
program's layers to 2551.

**From 2027 (#99, landed first):** its scope for this ticket is done
here: `--template` beside the older `new <template> <dir>` (CI's
long-name step uses the older form); `docs/contract.md`, the README and
the MCP `new` tool's description show the new form; the grid stage's
fence moved with `stage-top-down-grid`, and the base part's `AGENTS.md`
names `crates/rules/clippy.toml` and `crates/rules/tests/layers.rs` (a
test checks a composed grid game has both, the float ban and those
names); the paths of the device crates' copy (`tools/layers.ron`, the
xtask and `lint` tests, the docs map) and the globs the docs map and
the `dependency-upkeep` skill used now name the parts.

**Docs:** `docs/templates.md` (new), `docs/contract.md`, the README, the
docs map's rows, the crate doc, the base part's `AGENTS.md` and
`README.md`, `crates/rules/AGENTS.md`, ADR-0042.

Breaking: shuyi-cli: `contract::new::Template` and `contract::new::files` are gone; `NewInput::new(template, dir)` is `NewInput::new(dir).template(name)`, `NewInput::template` is an `Option<String>` (a short name), with new fields `answers`, `set` and `force`; `NewOutput` has new fields; the contract's version is 17. The game calls none of them.
Breaking: templates: the placeholder `{{crate}}` is `{{crate_name}}`, and `templates/grid` and `templates/blank` are the short names of `templates/shuyi-default`.

No game PR is needed: Visions of Shuyi doesn't use `shuyi new` or its
types.
