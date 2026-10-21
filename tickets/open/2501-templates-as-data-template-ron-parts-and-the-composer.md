---
id: "2501"
title: "Templates as data: template.ron, parts that apply when an answer is picked, and a pure composer behind shuyi new"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["1129", "2003", "2511"]
nick_input: none
completed:
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
    renamed), 1129's `"{{engine.<crate>}}"`, 2808's `{{engine.docs}}`
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
  - `--template grid` and `--template blank` stay, as short names for
    answer sets of the built-in template;
  - the result (ADR-0005) lists the files made, the modules turned on,
    and `limits` (for example "no part for answer X; nothing added").
- **The built-in template**, `templates/shuyi-default/`, replaces 1129's
  two folders:
  - part `base` (always): 2511's blank template on the short path
    (`BasicGame`, `BasicData`, `run_game`, `BasicTools`), with
    `shuyi.ron`, `AGENTS.md`, the flow screen and CI with the gates. It
    also carries what these tickets add to 1129's two folders, for each
    one that is done. One that isn't done puts its files in `base`
    itself, as 2225, 2223, 3048 and 3050 say in their *Either order*
    clauses (3039 is in the MVP, so it is due first):
    - 2225's menu sounds: in `crates/game/files/`, `audio.ron` and
      `sfx/move.sfx.ron`, `sfx/select.sfx.ron`, `sfx/cancel.sfx.ron` and
      `sfx/cursor.sfx.ron`, with the `sfx` feature on `shuyi-assets` in
      `crates/game/Cargo.toml` and the sound test in `tests/it/main.rs`.
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
  - part `stage-top-down-grid`, when perspective is `top-down-grid`:
    1129's grid screen as a stage, on the short path. Its map, lights
    and seed are `BasicData`'s own part (`BasicGame::with_own`), with
    `()` settings and `run_game`. A `GameTools` of its own keeps what
    the grid game adds today (the seed option on `frame`, the session's
    scene and events, `replay`, `bots` and the map importer) and calls
    `BasicTools` for the rest. The PR gives the composed grid game's
    code count before and after, from `cargo xtask template-lines`
    (2512), or by 2511's counting rule if 2512 isn't done;
  - `blank` is `base` alone;
  - if the parked 3108 or 3109 have landed with template folders
    (`first-person`, `third-person`, `top-down-3d`, `hd-2d`), turn each
    into a part the same way, and keep its name as a short name.
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

**Out (do not do):**
- asking questions, and the catalog's questions (2502);
- more than one perspective, and moves between stages (2503);
- targets checks and per-target CI (2504);
- templates from a path or git, `shuyi template check` and `new` (2505);
- any hook or script in a template (ADR-0013 §7: never).

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
   parts' workflow folders in the same PR, and point 2011's zizmor job
   at them in place of `templates/blank` and `templates/grid`. 2011's
   `cargo xtask check-pins` fails until they name them, and it also
   checks that the parts' `[workspace.dependencies]` requirements equal
   the root's. Move the `rust-toolchain` update's `directories` to the
   parts' `rust-toolchain.toml` folders too: 2012's `cargo xtask
   check-toolchain` (rule 5) fails until they name them.
5. Point CI's `templates` job (it makes a game from `grid` and from
   `blank` and runs its gates) at the new template.
6. Write `docs/templates.md` and link it from the README. Settle the
   *Either order* clause and the files `base` carries over.

## Acceptance criteria

- [ ] `shuyi new demo --template grid` makes a game that builds and
      passes its tests in CI, as 1129's did.
- [ ] `shuyi new demo --answers fixtures/answers.ron` makes the same
      bytes as the equivalent `--set` flags (a test compares the plans).
- [ ] Composing the same answers twice gives the same bytes (golden
      test on the plan's file list and hashes).
- [ ] Each `Template::load` and `ComposeError` case has a fixture and a
      message naming the file, field or parts.
- [ ] Two parts writing one unmerged file fail with both parts named.
- [ ] `shuyi new --json` output matches its schema (1129's schema test).
- [ ] `docs/templates.md` exists and is linked from the README.
- [ ] All gates in the `run-gates` skill pass.

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
