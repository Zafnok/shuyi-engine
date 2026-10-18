---
id: "2501"
title: "Templates as data: template.ron, parts that apply when an answer is picked, and a pure composer behind shuyi new"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["1129", "2003"]
nick_input: none
completed:
---

# 2501 — Templates as data, and the composer

## Context

Nick, 2026-10-17 (`docs/decisions.md`): a "create-react-app type command
with templates", which is "extensible too". Several perspectives,
physics, music and targets give too many combinations to keep as whole
templates.
[ADR-0011](../../docs/adr/0011-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)
§3 makes a template a folder of data made of **parts**, and a pure
composer that turns a template and a set of answers into files.

This ticket builds the format and the composer, and moves 1129's two
templates (`grid`, `blank`) onto them. Nothing asks questions yet:
answers come from flags and from an answers file. The questions are
2502, several perspectives are 2503, targets are 2504, and templates
from outside the engine are 2505.

It needs `shuyi new` (1129) and `shuyi add` with module metadata (2003),
because a part turns modules on through the same code as `shuyi add`.

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
  - **Placeholders:** `{{name}}`, `{{crate_name}}` and
    `{{answer.<question>}}` in file contents and file names. An unknown
    placeholder is an error naming the file and line. There are no
    conditions or loops.
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
  - part `base` (always): the crates, `shuyi.ron`, `AGENTS.md`, the
    flow screen, CI with the gates;
  - part `stage-top-down-grid`, when perspective is `top-down-grid`:
    1129's grid screen as a stage;
  - `blank` is `base` alone.
- **Embedded:** the built-in template is embedded in the `shuyi` binary
  (`include_dir`, believed MIT OR Apache-2.0: check its `Cargo.toml`
  and `cargo deny` on the day; or a build step that writes it as
  bytes), so `shuyi new` works with no checkout of the engine.
- **The format's page**, `docs/templates.md`: the fields, `When`,
  placeholders, merged files, with the built-in template as the example.

**Out (do not do):**
- asking questions, and the catalog's questions (2502);
- more than one perspective, and moves between stages (2503);
- targets checks and per-target CI (2504);
- templates from a path or git, `shuyi template check` and `new` (2505);
- any hook or script in a template (ADR-0011 §7: never).

## Implementation steps

1. Write the types, `Template::load` and its checks, with fixture
   templates for each error.
2. Write `compose`, placeholders and the merged files. Use the code 2003
   wrote for `shuyi add` to edit `Cargo.toml`; don't write a second one.
3. Write `apply` and the `new` flags.
4. Move 1129's `templates/grid` and `templates/blank` into
   `templates/shuyi-default` as parts, and embed it.
5. Point 1129's CI job (it makes a game with `shuyi new grid` and builds
   it) at the new template, and add `blank`.
6. Write `docs/templates.md` and link it from the README.

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
  ones; each load check; each merged file.
- Property: for any answer set over a fixture template, `compose`
  either returns a plan or a `ComposeError`, never panics, and gives the
  same plan twice.
- Fuzz-style: arbitrary bytes into the `template.ron` loader never panic.
- Integration: `shuyi new` with `grid` and `blank` into a temporary
  folder, then `cargo build` and `cargo test` there (CI).

## Completion notes
