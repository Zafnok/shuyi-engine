---
id: "2552"
title: "shuyi module new: a scaffold for an outside module, with its metadata, README, skill and a demo game that installs it"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["2526"]
nick_input: none
completed:
---

# 2552 — `shuyi module new`

## Context

2526 lets a game add a module from outside this repository (`shuyi add
<crate>@<version>`, `--git` at a revision, or `--path`), and writes
`docs/writing-a-module.md`. Its *Out* leaves a scaffold for a module,
like `shuyi template new`, to a ticket of its own. Nothing was watching
for it, so the 2026-10-24 backlog audit wrote it now (the
`write-ticket` skill: a follow-up you can name now is written now).

What a module ships with, and where it is said:

- [ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
  §4: a crate with `[package.metadata.shuyi]`; a README in a fixed
  order (what it is for, what it is not for, what it costs, how to
  start, how it sits beside its alternatives); an example game driven
  by the Harness, with its test; and a skill.
- [ADR-0041](../../docs/adr/0041-the-module-catalog-concerns-as-data-metadata-answers-as-pairs-and-modules-and-add-reading-the-engine-the-game-builds-with.md):
  - §2, the metadata's shape: `answers` as a list of `{ concern,
    answer }` pairs, `serves`, and the optional `alternatives`,
    `works_with` and `formats`. `shuyi_cli::catalog::ModuleInfo` reads
    it with `deny_unknown_fields`;
  - §5, the skill in the package's own `skill/` folder;
  - §6, the README's fifth section is `## Beside its alternatives`.
- 2526's ADR, for a module outside this repository: its engine version
  is its dependency on the engine; its skill is `skill/SKILL.md` in its
  `include` list, named after its crate; and the naming rule Nick
  picks (`docs/decisions.md`: `<name>-shuyi`, or `shuyi-<name>` too).
- [ADR-0040](../../docs/adr/0040-modules-join-a-running-game-a-library-first-a-module-trait-with-declared-hooks-and-state-by-type-on-the-context.md)
  and 2358 (done): the `Module` trait a game installs with
  `Game::with_module`, or on the short path with `BasicGame::modules`
  (ADR-0041 §7).

Getting all of that right by hand costs an author, or their agent,
many lines and failed checks before the module's own code: the
metadata's keys, the README's order, a game to try the module in, and
that game's test. A game starts with one command (`shuyi new`,
[ADR-0013](../../docs/adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)).
A module should too.

**The code today:**

- `crates/cli/src/contract/new.rs`:
  - `Engine` (`Main`, `Rev`, `Path`, and its default), `NewInput`,
    `NewOutput`;
  - `check_name`, `crate_name`, `is_empty_dir`;
  - `dependency(engine, root, krate)`, the text of an engine crate's
    dependency behind `{{engine.shuyi}}`;
  - `TEMPLATES`, the embedded `templates/`.
- `templates/blank/`:
  - its root `Cargo.toml` is a virtual workspace (`members =
    ["crates/*"]`) with the engine as `shuyi` and `shuyi-cli` in
    `[workspace.dependencies]`;
  - `crates/game/src/lib.rs`: `game()` ends with
    `.modules(shuyi::enabled_modules)`;
  - `crates/game/tests/it/main.rs` drives the game with
    `game::game().harness()`.
- `crates/cli/src/catalog.rs`: `ModuleInfo`, `Answered`,
  `engine_root` and `Catalog::read`.
- `crates/ui/src/module.rs`: `Module` (`name`, `install`,
  `before_update`, `draw_over`), `Install::hook`, `Install::state`.
  `Ctx::module_state` and `module_state_mut` read a module's state
  (`crates/ui/src/ctx.rs`).
- CI's `templates` job (`.github/workflows/ci.yml`) and the `run-gates`
  skill's template section make a game from each template and run its
  gates. 2548 (done) added a game with a long name.

**Why it matters:**

- **The game** needs nothing here. It uses the engine directly.
- **Other 2D games:** ADR-0009 §7 leaves most modules to other people.
  This is how one starts.
- **The end goal:** Nick wants "a wide variety of game types"
  (2026-10-16) and the "#1 AI friendly" engine (2026-10-19). An agent
  starts a module with one command, and every check passes before it
  writes the module's own code.

## Nick input

None. The naming rule is 2526's question, answered before this ticket
starts.

## Scope

**In:**

- **`shuyi module new`**, a contract operation with `--json`:

  ```text
  shuyi module new <dir> --concern <id> --answer <id> --serves <ids>
      [--name <name>] [--crate <crate>] [--license <spdx>]
      [--engine-rev <commit> | --engine-path <dir>] [--json]
  ```

  - `--name`: the module's name: lowercase ASCII letters, digits and
    `-`, starting with a letter; the folder's name if none. From it
    come:
    - its crate, `<name>-shuyi` unless `--crate` names another;
    - its folder, `crates/<name>`;
    - its Rust type (`foo-bar` gives `FooBar`);
    - the name its `Module::name` gives (`foo_bar`).
  - `--crate`: if Nick's answer keeps `shuyi-` for the engine, a crate
    starting with `shuyi-` or `shuyi_` is refused, naming the rule. If
    anyone may use it, the README gets the line saying the crate isn't
    the engine's, as his answer asks.
  - `--concern`, `--answer`: one pair of `answers`. More pairs are
    added in `Cargo.toml` by hand.
  - `--serves`: one or more directions, joined by `,`. It can't be
    empty (`cargo xtask check-modules` fails an empty one for the
    engine's own modules, and the same rule holds here).
  - `--license`: `MIT OR Apache-2.0` if none. One that 2526's
    `credits::license_allowed` rejects over `credits::CODE_LICENSES`
    is a warning, not a refusal, as in `shuyi add`.
  - The engine flags are `shuyi new`'s, with its default. The module
    and its demo game take the engine from the same place.
  - A `<dir>` that exists and isn't empty is refused, as `new` refuses
    one.
- **What it writes:** a game made by `new`'s own code from the blank
  template, with the module as a second member of its workspace:

  ```text
  <dir>/              the game "<name> demo", from the blank template
    Cargo.toml        + the module and shuyi-ui as workspace deps
    crates/game/      the demo game, which installs the module
    crates/<name>/    the module
      Cargo.toml
      README.md
      src/lib.rs
      skill/SKILL.md
  ```

  - **The root `Cargo.toml`** gains two lines in
    `[workspace.dependencies]`, its layout kept (`toml_edit`):
    - `shuyi-ui`, written by `new`'s `dependency`, so it comes from
      the same engine as `shuyi`;
    - `<crate> = { path = "crates/<name>" }`.
  - **`crates/<name>/Cargo.toml`**, built in code (a `Cargo.toml` in a
    folder of `shuyi-cli` would be a package of its own, which `cargo
    package` leaves out):
    - `name`, `version = "0.1.0"`, `description`, `license`,
      `keywords = ["shuyi", "gamedev"]`;
    - `edition` and `rust-version` from the workspace, and `[lints]
      workspace = true`;
    - `include` naming `src`, `skill` and `README.md`;
    - `shuyi-ui.workspace = true`, its one dependency. It gives
      `Module` and `GameData`, and keeps the module free of the
      window, so a headless server can use it (ADR-0009 §4);
    - `[package.metadata.shuyi]` with `answers` and `serves` from the
      flags, and `alternatives = []` and `works_with = []`;
    - a comment over each key, naming `docs/writing-a-module.md`.
  - **`src/lib.rs`**: the smallest module that does something a test
    can see. Its type is the `Module` of any `G: GameData`. Its
    `install` declares `Hook::BeforeUpdate` and puts a `Frames(u64)`
    state on the context, and its `before_update` adds one to it. The
    doc comments say what to replace. The names sit in their own
    `const` items, so rustfmt lays the file out the same for any name
    (as 2548 did for the templates).
  - **`README.md`**: ADR-0009 §4's five sections, in order, with the
    headings `docs/writing-a-module.md` gives, the fifth `## Beside its
    alternatives`.
    - The first paragraph names the module, its concern and answer,
      and what it serves. `shuyi add` prints it.
    - "How to start" shows `shuyi add <crate>@<version>` and the code
      that installs the module, as the demo game has it.
    - "What it costs" says it depends on `shuyi-ui` only, and that its
      size is measured once it has code.
  - **`skill/SKILL.md`**: frontmatter `name: <crate>` and a
    `description`, then short sections: what the module is for, how a
    game installs it, its state, and where its README is.
  - **The demo game** (`crates/game`):
    - its `Cargo.toml` depends on the module (`<crate>.workspace =
      true`);
    - in `src/lib.rs`, `.modules(shuyi::enabled_modules)` becomes
      `.modules(modules)`, and a function `modules` returns
      `shuyi::enabled_modules()` with the module pushed last;
    - a test, `tests/it/module.rs` with its `mod module;` line in
      `tests/it/main.rs`, runs a few frames with
      `game::game().harness()` and reads the module's `Frames` on the
      context.

    The demo game is the module's example (ADR-0009 §4): a game it
    serves, driven by the Harness.
  - The other files are the blank game's, as `new` writes them.
- **The data:** `lib.rs`, the README, the skill and the demo's test are
  files with `{{...}}` placeholders in `crates/cli/module/`. They are
  embedded with `include_str!` and filled as `new` fills the
  templates. The folder sits outside `templates/`, so `shuyi new`
  never lists it. `cargo package --list -p shuyi-cli` names its files.
- **The catalog:** with the folder written, the answer reads the
  engine's catalog as `shuyi modules` does (`catalog::engine_root`,
  then `Catalog::read`). It says whether the catalog has the concern
  and the answer. One it doesn't have is still written, and the answer
  says it is outside the catalog, as 2526's `shuyi modules` then shows
  it. If the catalog can't be read (a git engine and no network), a
  `limits` line says the ids weren't checked.
- **The answer:** `NewOutput`'s fields for the game, then:
  - the crate's name and the module's folder;
  - whether the concern and answer are in the catalog (`null` when it
    couldn't be read);
  - the license warning, if any.

  Its `limits` say what is the author's to write: the README's
  sections, the skill and the module's code. They also say that
  `shuyi add <crate> --path <dir>/crates/<name>` adds the module to
  another game.
- **The contract:**
  - `ModuleNewInput` and `ModuleNewOutput` are new public types.
    Each is `#[non_exhaustive]` with a constructor, as `NewInput` is
    ([ADR-0039](../../docs/adr/0039-public-types-that-can-grow-non-exhaustive-config-and-views-with-constructors-plain-values-left-open.md)).
  - Raise `contract::VERSION`, regenerate `crates/cli/schemas/`, and
    add the line to `versions.json` (`docs/contract.md`, *The
    schemas*).
  - Its MCP tool takes no `project`, like `new`'s.
- **Either order with 2505:** if 2505 is done, name `module new` as it
  named `template new` and `template check`: the `Op`, the schema
  files, the MCP tool, and how `cli.rs` reads a two-word command. If
  not, this ticket sets the form, and 2505 follows it:
  - `Op::ModuleNew`, named `module_new`;
  - its schemas, `module_new.input.json` and `module_new.output.json`;
  - its MCP tool, `module_new`;
  - its command, `shuyi module new`.
- **CI:** the `blank` leg of the `templates` job also:
  - makes a module with a long name (`--engine-path
    "$GITHUB_WORKSPACE"`);
  - runs `cargo fmt --all --check`, `cargo clippy --workspace
    --all-targets -- -D warnings` and `cargo test --workspace` in its
    folder;
  - adds the module to the leg's `demo` game with `shuyi add <crate>
    --path <its folder> --yes`, and runs `cargo build` there.

  The `run-gates` skill's template section gets the same lines.
  **Either order with 2033:** if 2033 is done, its test reads the
  `templates` job's `run: |` blocks that have a `cargo shuyi check`
  line, and the first code block of `run-gates`' *A new game from each
  template*, as copies of the game's gates: put the module's lines in a
  step and a code block of their own, never in "Run its gates" or the
  game's block; if not, nothing to do (2033 checks that its rules skip
  those lines).
- **Docs** (the `document` skill):
  - `docs/writing-a-module.md` starts from the command: what it
    writes, and what to fill in first;
  - the operation's row in `docs/contract.md`'s table;
  - the skill-pack skill that 2526 taught `shuyi add`'s outside
    sources learns `shuyi module new`. It starts a module with it,
    then fills the README's sections and the skill.
- **For the game:** nothing changes. A new operation is no API break.

**Out (do not do):**

- The in-house form: a module under this repository's `crates/`. Each
  engine module's ticket writes its crate, its catalog row, its
  `tools/layers.ron` entry, its feature of `shuyi` and its line in
  `enabled_modules` by hand (ADR-0041, *Consequences*).
- Templates: `shuyi template new` is 2505's.
- Publishing the module (`cargo publish -p <crate>`, which
  `docs/writing-a-module.md` already says), or listing it anywhere.
  The website's Assets page is 2809's.
- A CI workflow for the module beyond the one the blank game brings.
- More than one concern, or `alternatives`, `works_with` and `formats`,
  from flags: the author edits `Cargo.toml`, as its comments say.
- The module's own code beyond the frame counter.
- Changing what `shuyi add` reads or refuses, except where step 1
  says.

## Implementation steps

1. Read 2526's ADR, `docs/writing-a-module.md` and its `shuyi add`:
   how it reads an outside module's README, skill and example, and
   what it reports missing. The demo game is this scaffold's example.
   If 2526's `shuyi add` would report the example missing for a module
   made here (for example, it looks only inside the package), make the
   two agree in this PR. Change the rule or the layout, whichever
   2526's ADR allows, and say which in the PR.
2. Read Nick's naming answer in `docs/decisions.md`.
3. Write `crates/cli/src/contract/module_new.rs`: the flags, the names,
   the refusals, the module's `Cargo.toml` text, and the call to
   `new`'s code for the blank game. Then write the module's files from
   `crates/cli/module/`, and edit the demo game (with `toml_edit` for
   the two `Cargo.toml` files).
4. Read the catalog and build the answer. Add the license warning.
5. Settle the clause with 2505. Then add the `Op`, the schemas,
   `contract::VERSION` and `versions.json`.
6. Add the CI steps and the `run-gates` lines. Write the docs and the
   skill's lines.
7. Run the gates.

## Acceptance criteria

- [ ] `shuyi module new <tmp>/foo --concern bodies --answer procedural
      --serves side-view --engine-path <checkout>` writes the layout
      above. In it, `cargo fmt --all --check`, `cargo clippy
      --workspace --all-targets -- -D warnings` and `cargo test
      --workspace` pass, and the demo's test reads the module's
      `Frames` after the frames it ran.
- [ ] A module with a long name passes `cargo fmt --all --check` as
      well.
- [ ] From a game made by `shuyi new blank --engine-path <checkout>`,
      `shuyi add foo-shuyi --path <tmp>/foo/crates/foo --yes` adds the
      module with no missing part in `limits`, and the game builds.
- [ ] `catalog::ModuleInfo` reads the written metadata, and `shuyi
      modules --json` in the made folder lists `foo-shuyi` with
      `origin: outside`.
- [ ] A concern outside the catalog is written, and the answer says it
      is outside the catalog.
- [ ] A `<dir>` that isn't empty, and a bad `--name`, are refused. If
      Nick kept `shuyi-` for the engine, `--crate shuyi-foo` is
      refused, naming the rule.
- [ ] `--license GPL-3.0-only` writes the module, with the warning.
- [ ] `shuyi module new --json` matches its schema, and the schemas
      and `versions.json` match `contract::VERSION`.
- [ ] CI's `templates` job and the `run-gates` skill make and check a
      module.
- [ ] `docs/writing-a-module.md` and `docs/contract.md` name the
      command.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`module_new.rs`):
  - the module's `Cargo.toml` as a golden string, for `--engine-path`
    and for the default engine;
  - the names: crate, folder, type and module name from `foo` and
    `foo-bar`;
  - the refusals: a folder that isn't empty, a bad name, and the
    `shuyi-` prefix if Nick's answer refuses it;
  - the blank template's `crates/game/src/lib.rs` holds
    `.modules(shuyi::enabled_modules)` exactly once, so a change to
    the template fails here and not in a made game;
  - the license warning.
- Integration (`crates/cli/tests/it/`): a module made in a temporary
  folder with `--engine-path` to this checkout. Check:
  - the files;
  - `ModuleInfo` from the written `Cargo.toml`;
  - `shuyi modules` in the folder.
- CI: the `templates` job's steps above, which build and test the made
  module and add it to another game.
- Schema: the contract's schema test.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
