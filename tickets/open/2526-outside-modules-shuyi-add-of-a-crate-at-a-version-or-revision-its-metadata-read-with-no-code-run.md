---
id: "2526"
title: "Outside modules: shuyi add of a crate from crates.io at a version, git at a revision or a folder, its metadata read with no code run, and an ADR extending ADR-0009"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2003", "2005"]
nick_input: decision
completed:
---

# 2526 — Outside modules in `shuyi add`

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engine Defold. In each of them most add-ons
come from people outside the engine's team:

- Bevy's plugins are ordinary crates on crates.io. Its plugin guide lets
  anyone use a `bevy_xxx` name "but please be reasonable", and asks
  people to check first before taking a generic one
  (<https://bevy.org/learn/quick-start/plugin-development/>).
- Defold adds a library to a game by the URL of a release zip in
  `game.project` (<https://defold.com/manuals/libraries/>).
- Godot installs add-ons from its Asset Library. Unity's Package Manager
  takes packages by git URL.

**What Shuyi has, and plans:**

- [ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
  §4 says a module is a crate `shuyi-<name>` in `crates/<name>` of this
  repository, with `[package.metadata.shuyi]`, a README in a fixed
  order, an example and a skill. §6 says `shuyi add <module>` turns on a
  feature of the `shuyi` crate.
- 2003 builds §4 and §6: `ModuleInfo` in `shuyi-cli`, the catalog
  (`docs/modules.ron`, `docs/modules.md`), `cargo xtask check-modules`
  over `crates/`, and the contract operations `shuyi modules` and
  `shuyi add`. `shuyi add` only flips a feature of `shuyi`, copies the
  module's skill (its package's `skill/`, else the skill pack's
  `skills/<name>` beside it, ADR-0041 §5) and prints the
  README's first paragraph. It knows nothing outside this repository.
  The `shuyi` crate's features turn on only the engine's own modules.
- §4 already lets heavy or experimental work live in a repository of
  its own (2901 is the first). Nick, 2026-10-19 (`docs/decisions.md`),
  also wants the in-house platform pieces carved into "their own repos
  which we pull into engine" one day. `shuyi add` can't reach either.
- §7 writes a module only when a kind of game needs it. So the modules
  for most kinds of game will be written by other people.
- [ADR-0013](../../docs/adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)
  §7 and 2505 set the pattern for templates from anywhere: a folder or
  git at a required revision, nothing runs, the source, revision and
  license shown before anything is written, `--yes`, a refusal under
  `--no-input`, and a warning (not a refusal) for a license outside
  ADR-0002 §8.
- 2005 waits for the MCP server (1133) and the skill pack (1134), so
  both exist when this ticket starts.
- The code today:
  - `crates/cli/src/contract/new.rs`: `ENGINE_GIT`, `ENGINE_CRATES`
    (`shuyi`, then `core`, `assets`, `ui`, `platform`, `app`, `cli`,
    by folder under `crates/`), and the `--engine-rev` check (7 to 40
    hex digits);
  - a game from `shuyi new` is a virtual workspace (`members =
    ["crates/*"]`) with the engine in `[workspace.dependencies]`
    (`templates/grid/Cargo.toml`);
  - `crates/cli/src/game.rs` (`run_in_game_program`) and
    `crates/cli/src/web.rs` (`target_dir`) run cargo through the `CARGO`
    variable, or `cargo`. `target_dir` finds the workspace with `cargo
    locate-project --workspace`;
  - **From 2003 (done):** `shuyi_cli::catalog` runs `cargo metadata
    --format-version 1` through the `CARGO` variable and reads its
    JSON with serde (`resolved_shuyi`, private, behind `engine_root`).
    Make that runner one shared helper, and use it for steps 1 and 3
    of the probe. 2015's reader is in the xtask, which is never
    published;
  - `crates/cli/src/contract.rs`: `Answer::new` gives a `result`, and
    `Answer::failed` gives an `error` with no `result` (an operation that
    couldn't run);
  - `crates/assets/src/credits.rs` line 84: `credits::LICENSES`, the
    ADR-0002 §8 code list plus `OFL-1.1`.
- `docs/decisions.md` (2026-10-14) names the engine's crates "plus
  later crates under `shuyi-*`", and says nothing about crates by other
  people.

**Why it matters:**

- **The game** doesn't need this. It uses the engine directly, and
  `shuyi add` of the engine's own modules doesn't change.
- **Other 2D games** get the modules ADR-0009 §7 leaves to others,
  added the same way as the engine's own, with the same check that two
  alternatives aren't both on.
- **The end goal:** Nick wants "a wide variety of game types"
  (2026-10-16) and the "#1 AI friendly" engine (2026-10-19). An agent
  adds a stranger's module through the contract, without running the
  stranger's code, and without copying text its later sessions would
  follow unless the user said yes.

## Nick input

**Decision**, with the `ask-nick` skill, before writing the ADR. Ask it
only if `docs/decisions.md` doesn't have the answer yet. 2809, which
waits for this ticket, reads the answer.

May other people name their crates `shuyi-<something>`?

- **A. Anyone may**, with a line in their README saying the crate isn't
  official. This is Bevy's way. The cost: a stranger's `shuyi-physics`
  would look official on crates.io, and the catalog's planned names
  (`shuyi-physics`, `shuyi-net`, …) could be taken before the engine
  publishes them.
- **B. `shuyi-` stays for the engine's own crates.** Others put the word
  after the name, such as `foo-shuyi`, and add `shuyi` to their
  crates.io keywords. **Recommended.** It is only a request: the name
  isn't registered (`docs/decisions.md`, 2026-10-14), so nothing
  enforces it.

`shuyi_foo` is not a third choice. crates.io treats `-` and `_` as the
same, so `shuyi_foo` and `shuyi-foo` are one name.

Record the answer in `docs/decisions.md`, and add a short "Naming your
crate" paragraph to the README's *The name* section.

## Scope

**In:**
- **An ADR** (the `write-adr` skill), extending ADR-0009 §4, §6 and §7.
  It says:
  - an **outside module** is a crate from crates.io, a git repository or
    a folder, with the same `[package.metadata.shuyi]` as §4. Its
    concern and answer come from the catalog (`docs/modules.ron`) or are
    new ones; a new one is shown as outside the catalog;
  - **its engine version is its dependency on the engine**, with no
    second field: `shuyi add` checks that the game and the module
    resolve to one copy of each engine crate;
  - it should ship what §4 asks for: a README in §4's order, an example,
    and a skill, at `skill/SKILL.md` inside its package (in its
    `include` list), where the engine's own modules keep theirs
    (ADR-0041 §5). For an outside crate `shuyi add` reads only that
    folder, never `catalog::skill_of`'s fallback two folders up, which
    would be outside the package (in cargo's cache, or above a
    `--path` folder). The engine's CI doesn't check it. `shuyi add`
    says in `limits` what is missing;
  - the naming rule Nick picks. A module's skill is named after its
    crate. `shuyi add` never copies a skill whose name is one of the
    engine's own skills (1134's `skills/`), whatever the naming rule;
  - **how its code joins the game:** through the game's own code, as its
    README's "how to start" section shows. The `shuyi` crate's features
    (2003) cover only the engine's modules;
  - **trust:** adding one runs none of its code. Before writing, the
    developer sees its source, version or revision, license, and what it
    answers, and says yes. Its skill is a second yes, because a skill is
    text the game's agent follows in every later session;
  - a license outside ADR-0002 §8 is a warning, not a refusal (the game
    is its developer's), as in ADR-0013 §7;
  - no registry and no search service. The website's directory (2809)
    and the module's own docs are how people find one;
  - the alternatives considered:
    - a registry service;
    - an `engine` field in the metadata (a second place that can
      disagree with the dependency);
    - asking the crate itself, through a `build.rs` or a binary (runs
      its code);
    - downloading `.crate` files over HTTP (a new dependency, when cargo
      already does it).
- **The sources.** These mirror `cargo add`'s own flags, because a crate
  has a name and a template folder doesn't:
  - `shuyi add <crate>@<version>`: crates.io. The version is required,
    and its absence is an error that says why: the developer reads the
    license of the version they adopt;
  - `shuyi add <crate> --git <url> --rev <commit>` or `--tag <tag>`. A
    commit is checked like `--engine-rev` (7 to 40 hex digits).
    `--branch`, or no revision, is refused with the reason, as in 2505:
    the same command must always add the same code;
  - `shuyi add <crate> --path <dir>`: a folder, for a module's author
    trying it in a game, and for the tests;
  - `--package <member>` picks the workspace member to change, as
    `cargo add -p` does. 2003's `shuyi add` changes no member: it edits
    the root `Cargo.toml`'s `shuyi` entry (under
    `[workspace.dependencies]`, else `[dependencies]`). So the default
    is the one member that depends on `shuyi` (`crates/game` in a game
    from `shuyi new`). With several, or none, `shuyi add` refuses and
    names them, asking for `--package`;
  - a bare name with no version or flag is 2003's catalog path,
    unchanged. If it isn't in the catalog, the error adds: "for an
    outside crate, name its version: `shuyi add <crate>@<version>`". A
    crate of the engine's own named with a version gets "this is the
    engine's own module: `shuyi add <name>`".
- **Reading it with no code run** (`crates/cli/src/contract/add/outside.rs`
  or beside 2003's `add`):
  1. `cargo metadata --format-version 1 --no-deps` on the game gives the
     member's dependencies on the engine: `shuyi`, the `shuyi-<folder>`
     crates of `ENGINE_CRATES`' other six entries, and the catalog's
     module crates.
  2. A **probe** package is written in a temporary folder. It holds:
     - an empty `[workspace]`;
     - those dependencies exactly as reported (source, version
       requirement, absolute path, features, default features);
     - the outside crate from its source;
     - an empty `src/lib.rs`;
     - a copy of the game's workspace `Cargo.lock` (found as
       `web::target_dir` finds the workspace), if it has one, so it
       resolves as the game would.
  3. `cargo metadata --format-version 1` on the probe, run through the
     `CARGO` variable as `game.rs` does. It downloads and reads
     manifests. It compiles nothing, and runs no build script and no
     procedural macro.
  4. From its answer:
     - the crate's `metadata.shuyi`, read into 2003's `ModuleInfo`
       through serde from the JSON. A crate without it is refused: "not
       a Shuyi module; use `cargo add` for a plain crate";
     - its `license` (or `license_file`);
     - its folder (from `manifest_path`), for the README's first
       paragraph and the skill's frontmatter (`name`, `description`),
       its files and its line count.
- **The engine check:** in the probe's resolve, each engine crate
  appears at one version from one source. If not, `shuyi add` refuses
  and names both: "foo-shuyi 0.2.1 needs shuyi-core 0.2 from crates.io;
  the game has shuyi-core 0.3: pick a version of foo-shuyi built for
  this engine".
- **The license check,** in `shuyi_assets::credits` beside the lists,
  so the website's check (2809) can call it too:
  - `credits::CODE_LICENSES`, ADR-0002 §8's 14 code ids. If 2223 is
    done, it is there. If not, add it here under that name, and 2223
    keeps it;
  - **Either order with 2032:** if 2032 is done and this ticket writes
    `CODE_LICENSES` (2223 didn't), extend 2032's test
    `licenses_are_deny_tomls_allow_list` (`crates/assets/src/credits.rs`'s
    tests) so `CODE_LICENSES` plus `OFL-1.1` is also checked against
    `deny.toml`'s allow list; if not, nothing to do (2032, or 2223,
    covers it);
  - `credits::license_allowed(expression: &str, list: &[&str]) ->
    Result<bool, String>`: a small evaluator of SPDX expressions (`OR`,
    `AND`, `WITH`, parentheses, and the `/` older crates use for `OR`).
    It is true when some `OR` branch has every part in the list, and
    `X WITH Y` is one part. It uses `core` only, so `shuyi-assets` still
    builds without `std`. An expression it can't read is an `Err`;
  - 2809, which waits for this ticket, calls this one;
  - a license outside `CODE_LICENSES`, one that can't be read, or only a
    `license_file` adds a warning line. A `limits` line says that the
    module's own dependencies are for the game's `cargo deny`, if it
    runs one.
- **The question, and the rules for it:**
  - Shown before anything changes:
    - the source;
    - the version, or the revision and URL;
    - the license, with any warning;
    - the concern and answer;
    - the alternative the game already has on (2003's warning, for
      outside modules too);
    - the README's first paragraph.
  - Interactive: "Add foo-shuyi 0.2.1?" and then, if it ships a skill,
    "Copy its skill `foo-shuyi` (3 files, 140 lines) into the game's
    skills?".
  - `--yes` answers the first question. The skill is copied only with
    `--with-skill` as well. `--no-input`, or stdin not being a terminal
    (`std::io::IsTerminal`), without `--yes` refuses.
  - With `--json` and no `--yes`, the answer is `ok: false` and
    `changed: false`, through `Answer::new` (not `Answer::failed`, which
    drops the `result`). Its `result` holds everything above, and a
    `limits` line says: "nothing was added: an outside module needs
    --yes; show the user its source, license and skill first".
  - **Either order with 2505:** if 2505 is done, call its question (the
    function that shows a source, revision and license and reads
    `--yes` and `--no-input`). If not, write it here as one function in
    `shuyi_cli::contract`, and 2505 calls it.
- **Writing:** after the yes:
  - if the member already has this dependency from the same source,
    nothing runs and the answer is `changed: false`;
  - otherwise `cargo add` with the same source flags (and `-p`). It
    edits `Cargo.toml` keeping its layout, and builds nothing;
  - the skill goes through 2003's copy function into the game's skills
    folder, regular files only (a symbolic link is skipped and named in
    `limits`). It never goes over an existing folder of that name, and
    never under the name of one of the engine's skills;
  - a `limits` line says the game uses the module from its own code, as
    its README's "how to start" section shows.
- **`shuyi modules`:** from the game's full `cargo metadata`, every
  package with `metadata.shuyi` that isn't the engine's is listed under
  its concern with `origin: outside`, its crate, version or revision,
  and license. A concern outside the catalog goes in a group "Outside
  the catalog".
- **The contract:**
  - `add`'s input gains `version`, `git`, `rev`, `tag`, `path`,
    `package`, `yes` and `with_skill`;
  - its answer gains `source`, `license`, `license_warning`, `skill`
    and `changed`;
  - `modules`' answer gains `origin`;
  - raise `contract::VERSION`, regenerate `crates/cli/schemas/`, and add
    the line to `versions.json` (`docs/contract.md`, *The schemas*);
  - the doc comments of `yes` and `with_skill` say to ask the user
    first. They become the schema's descriptions, which the MCP server's
    `add` tool (1133) shows.
- **Docs:**
  - `docs/writing-a-module.md`, short, for a module written here or
    outside:
    - the metadata;
    - the README order;
    - the example and the skill (and its `include` line);
    - depending on the engine version it was tested with;
    - the naming rule;
    - how a game adds it, and installs its `Module` (2358) with
      `Game::with_module`;
    - one line recommending a CI job that builds it against the latest
      engine release.

    It links `docs/formats.md#writing-a-format-plugin` and
    `docs/platform-backends.md` instead of repeating them.
  - The `add` row of `docs/contract.md`.
  - 2809, which waits for this ticket, adds the line `shuyi add
    <crate>@<version>` and a link to `docs/writing-a-module.md` to its
    Assets page's Modules group.
- **The skill pack:** the skill that runs `shuyi add`,
  `skills/shuyi-new-game/SKILL.md` (1134, done), learns the outside
  sources. It must show the user the source, license and skill before
  passing `--yes` or `--with-skill`. `check-skills` passes.

**Out (do not do):**
- `shuyi module new`, a scaffold for a module like `shuyi template new`
  (2505): 2552, which waits for this ticket;
- the website's directory of modules (2809), a snapshot of it in
  `shuyi modules`, or any search;
- `shuyi remove` (as in 2003);
- building or testing outside modules in the engine's CI;
- code that installs an outside module in the game: its code joins a
  running game through 2358's `Module`, which the game installs itself
  (`shuyi::enabled_modules`, 2003, lists only the engine's modules);
- outside modules as wizard answers or in template parts: 2502's
  questions and 2501's parts take only the catalog's modules
  (ADR-0013 §1), and neither adds them; a follow-up ticket when a
  template asks for one;
- checking the licenses of a module's dependencies (the game's own
  `cargo deny`);
- a `deny.toml` for new games.

## Implementation steps

1. If `docs/decisions.md` has no naming answer, ask Nick (above) and
   record it.
2. Write the ADR.
3. Write `CODE_LICENSES` (unless 2223 did) and `license_allowed` in
   `crates/assets/src/credits.rs`, with unit tests.
4. Write the source parsing, the probe, the `cargo metadata` read and
   the engine check, with unit tests on fixture JSON.
5. Write or reuse the question (either order with 2505), then the
   `cargo add` call and the skill copy.
6. Mark outside modules in `shuyi modules`.
7. Regenerate the schemas and raise the contract version. Then write
   `docs/writing-a-module.md`, the contract row, the README paragraph
   and the skill's lines.
8. Run the gates. If the diff passes about 600 lines, move the
   `shuyi modules` marking (step 6) to a follow-up ticket written with
   the `write-ticket` skill, and say so in the PR.

## Acceptance criteria

- [ ] `shuyi add outside-module --path <fixture> --yes` on a game from
      `shuyi new grid --engine-path <checkout>` adds the dependency, and
      a second run reports `changed: false`.
- [ ] The fixture module's `build.rs` would write a marker file. After
      `shuyi add`, no marker exists: nothing of the module ran.
- [ ] `shuyi add outside-module --git file://<repo> --rev <hash> --yes`
      adds a module from a git repository made in the test. Without
      `--rev`, or with `--branch`, it fails and says why.
- [ ] The fixture committed with its engine path dependency is refused
      by the engine check, naming the second `shuyi-core`.
- [ ] `shuyi add foo` (no version, not in the catalog) fails and shows
      the `<crate>@<version>` form.
- [ ] Without `--with-skill`, no skill is copied. With it, the skill's
      folder is in the game's skills folder. An existing folder of that
      name is never overwritten.
- [ ] `--no-input` without `--yes` refuses, and `--json` without `--yes`
      answers `ok: false` with a `result` holding the source, license
      and skill.
- [ ] A crate with no `[package.metadata.shuyi]` is refused with the
      `cargo add` hint.
- [ ] The engine check refuses a resolve with two copies of an engine
      crate, naming both (unit test on fixture metadata).
- [ ] A copy of the fixture under `GPL-3.0-only` gets the warning and is
      still added with `--yes`.
- [ ] `credits::license_allowed` passes its unit tests, and
      `shuyi-assets` still builds without `std` (the `no-std` CI job).
- [ ] `shuyi modules --json` lists the added module with
      `origin: outside`.
- [ ] The schemas and `versions.json` match `contract::VERSION` (1129's
      schema test).
- [ ] The ADR is in `docs/adr/` with its row in `docs/adr/README.md`,
      and its header says it extends ADR-0009 §4, §6 and §7.
- [ ] Nick's naming answer is in `docs/decisions.md` and the README.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit:
  - source parsing (a version required, `--branch` refused, the commit
    check);
  - the probe's `Cargo.toml` for each source, as golden strings;
  - `credits::license_allowed`: `MIT OR Apache-2.0`, `MIT/Apache-2.0`,
    `GPL-3.0-only`, `MIT AND GPL-3.0-only`, `(MIT OR GPL-3.0-only) AND
    Zlib`, `Apache-2.0 WITH LLVM-exception`, an empty string, and an
    unclosed parenthesis;
  - the engine check on fixture `cargo metadata` JSON with one and two
    copies of `shuyi-core`;
  - `ModuleInfo` from outside metadata with a concern outside the
    catalog.
- Integration (`crates/cli/tests/it/add.rs`), on a fixture module
  (`crates/cli/tests/fixtures/outside-module/`, depending on the engine
  by path, with metadata, a README, a skill and a `build.rs` that would
  write a marker at the path in an environment variable the test sets).
  - Its `Cargo.toml` has an empty `[workspace]` of its own, because it
    sits inside the engine's workspace (`members = ["crates/*",
    "examples/*"]`) without being a member.
  - Its `src/lib.rs` uses its engine dependency, because `cargo
    machete` reads every `Cargo.toml` in the repository.
  - Copy the engine's `Cargo.lock` into the game first, so the resolve
    picks versions cargo already has.
  - The cases:
    - the folder source;
    - git: a copy of the fixture without its engine dependency,
      committed in a repository made in the test (`git init`, commit,
      use the hash). Checked with cargo 1.98.1: a path dependency inside
      a git package resolves under the git source, so the fixture
      committed as it is brings a second `shuyi-core`, which is the
      engine check's refusal case;
    - the marker test;
    - `--yes` and `--with-skill`;
    - a second run;
    - the `GPL-3.0-only` copy;
    - `shuyi modules`.
- Schema: the contract's schema test.

## Completion notes

*(Filled in by the session that completes the ticket.)*
