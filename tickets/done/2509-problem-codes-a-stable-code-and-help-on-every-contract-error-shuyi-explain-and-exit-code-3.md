---
id: "2509"
title: "Problem codes: a stable code and a help line on every error the contract gives, shuyi explain to read its page, and exit code 3 for fix the setup, then retry"
type: feature
milestone: MVP
model: opus-5.5
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-10-21
---

# 2509 — Problem codes, shuyi explain and exit code 3

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. The benchmarks give an error a key that a tool can
branch on:

- Unity 6's command-line tool (its CLI reference on docs.unity.com)
  answers in a JSON envelope whose errors carry a stable code
  (`INVALID_COMMAND_ARGS`), and its exit codes give the class: 2 usage,
  3 authentication, 4 configuration, 6 operation failed, 7 service
  unreachable (the one class it calls retryable), 8 tests failed.
- Bevy numbers its errors B0001 to B0006, each with a page
  (https://bevy.org/learn/errors/).
- Rust gives each compiler error a code, and `rustc --explain E0277`
  prints its page.

What Shuyi has today, in `crates/cli/src/contract.rs`:

- `Answer<T>` (line 116) says why an operation could not run in
  `error: Option<String>`: prose only.
- `Problem` (line 200) has `file`, `line`, `column` and `message`: no
  code and no help. `shuyi_assets::ContentError`
  (`crates/assets/src/error.rs:9`), which it is made from, has the same
  four fields.
- The exit codes are 0 passed, 1 failed, 2 wrong arguments
  (`Answer::exit_code`, line 165; `contract::cli::wrong`,
  `crates/cli/src/contract/cli.rs:43`). A folder with no `shuyi.ron`, a
  `shuyi.ron` that lacks the part a command needs, and a project with
  no game tool program also exit 2 (`main_with` and `run` in
  `crates/cli/src/lib.rs:84-181`; `cli::no_game`, `cli.rs:54`). So
  "your command is wrong" and "your setup is wrong" look the same.
- `doctor`'s checks carry a `fix` line each (`Check`,
  `crates/cli/src/contract/doctor.rs:34`): the only help the contract
  gives, and proof that the pattern works here.
- `docs/contract.md`'s example answer still says `"contract": 3`, and
  `VERSION` is 4 (`contract.rs:52`).

No ticket plans codes. The MCP server (1133) and the skills (1134)
would match prose, and the first release (2005) would freeze the
answer's shape as it is.

Why it matters:

- **The game first.** The game's sessions and CI run `cargo shuyi
  check` and the other operations. A code lets a skill or a CI step say
  what to do without reading the message, and lets the message's words
  change.
- **The end goal** (Nick, 2026-10-19, [`docs/decisions.md`](../../docs/decisions.md)):
  the most AI-friendly engine, where an agent makes a small game in
  under an hour. An agent fixes faster what it can classify, and a help
  line saves it attempts.
- **Before 2005.** Codes and exit codes are a public promise. Made
  before the first release, they are part of the first public contract
  instead of a break after it.

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md), *a
second comparison*): "logs and error codes agents can read" are among
the protections in the MVP. 3040 is the logs; this ticket is the codes.

## Nick input

None.

## Scope

**In:**

- **Codes.** `SH` and four digits. The first digit is the class, and
  the class sets the exit code:

  | Class | Codes | Exit code | Means |
  | ----- | ----- | --------- | ----- |
  | usage | `SH0nnn` | 2 | the arguments or the input are wrong: change the command |
  | setup | `SH1nnn` | 3 | the project file or the toolchain needs fixing: fix it, then run the same command again |
  | content | `SH2nnn` | 1 | a file of the game's content has a problem |
  | operation | `SH3nnn` | 1 | the operation failed on something else: the game's tools, a target, a file it reads or writes |

  A code is never reused or renumbered. A retired code keeps its row
  (`retired: true`) and its page, which says when and why it retired.
  Each code also has a kebab-case `name` (`missing-file`), as stable,
  for the docs and for tickets that name a code before it has a number
  (2517's `old-name`).

- **The first codes.** Each has a row, a page and at least one site:

  | Code | Name | Given where (today's code) |
  | ---- | ---- | -------------------------- |
  | `SH0001` | `wrong-arguments` | `cli::wrong` (`cli.rs:43`); "unknown command" (`lib.rs:172`); `schema`'s usage; what an op refuses in its input: `bots`' games and seed (`bots.rs:137`, `:143`), `run`'s script check (`run.rs:89`), `import --kind` with no such kind (`import.rs:302`), `new`'s refusals (a bad name, a folder that isn't empty, `--engine-rev` that isn't a commit, `--engine-path` that is no checkout of the engine: `check_name`, `crate_name`, `engine_root`, `make` and `parse_args` in `new.rs`), `check` called in the wrong function (`check.rs:58`, `:98`) |
  | `SH1001` | `no-project` | `Project::find` finds no `shuyi.ron` (`crates/cli/src/project.rs:46`), seen by `main_with` (`lib.rs:106-110`) |
  | `SH1002` | `project-file` | `shuyi.ron` doesn't read (`project.rs:32-33`), lacks the part a command needs (`Project::part`, `project.rs:59`, printed at `lib.rs:177-180`), or names no game tool program (`cli::no_game`; `game::run_in_game_program`, `crates/cli/src/game.rs:163`) |
  | `SH1003` | `toolchain` | cargo doesn't start (`game.rs:181`); a `doctor` with a failed check |
  | `SH2001` | `parse` | `error` in `crates/assets/src/ron.rs:119`: every RON format's syntax error |
  | `SH2002` | `not-text` | `format::text` (`crates/assets/src/format.rs:296`); `NotText` into `ContentError` (`crates/assets/src/bundle.rs:376`) |
  | `SH2003` | `missing-file` | `Formats::find` (`format.rs:660-661`, its own message with `with_code`); the language formats (`crates/assets/src/lang/formats.rs:126`, `:132`), the font (`crates/assets/src/font.rs:136`) and the language pack (`crates/assets/src/lang.rs:560`) through one new helper, `ContentError::not_found(file)`, which uses `NOT_FOUND` (`format.rs:56`) and the code |
  | `SH2004` | `ambiguous-file` | `Formats::find` when several files could be the asset (`format.rs:662`) |
  | `SH2005` | `no-format` | `Formats::unread` (`format.rs:626`); `import` when no kind claims the file (the `Problem` built at `import.rs:364`) |
  | `SH2006` | `too-big` | `decode_with` (`format.rs:309-317`) |
  | `SH2007` | `pack-differs` | `Bake::check` (`crates/assets/src/pack/bake.rs:392`, `:406`) |
  | `SH2008` | `unknown-name` | the keymap's unknown action, key chord or button (`validate_bindings`, `crates/assets/src/keymap.rs:1236`, `:1261`); a map's unknown terrain or legend character (`crates/assets/src/grid_map.rs:327`, `:460`) |
  | `SH2009` | `duplicate` | a binding listed twice or on two actions (`keymap.rs:1273`, `:1276`); a layout listed twice (`keymap.rs:1061`); a text key used twice (`lang.rs:323`, `:428`; `crates/assets/src/lang/lines.rs:252`) |
  | `SH2010` | `out-of-range` | the keymap's repeat interval and crank notch (`keymap.rs:982`, `:1002`); a map too wide or too tall (`grid_map.rs:426`, `:435`) |
  | `SH3001` | `not-offered` | a `GameTools` method the game left at its default (`game::unsupported`, `game.rs:156`). A new `game::is_unsupported(&str)` tells it from the game's own errors by the text `unsupported` starts with, a constant both share |
  | `SH3002` | `game-error` | any other error a `GameTools` method returns: `check` (`check.rs:84`), the controls (`check.rs:119`), `session` (`run.rs:92`), `replay` (`replay.rs:169`), `bots` (`bots.rs:160`), `bake` (`bake.rs:178`) |
  | `SH3003` | `target` | a target the project or the profiles don't have, or profiles that don't load (`check.rs:102`, `:108`, `:140`; `unknown_target`, `bake.rs:146`) |
  | `SH3004` | `file` | a file the op reads or writes can't be: `import`'s file (`import.rs:292`), a record (`read`, `replay.rs:149`), the pack (`bake.rs:201`), `new`'s files (`write`, `new.rs:278-280`) and a template that doesn't fill (`files`, `fill`) |

  `replay`'s closure (`replay.rs:159-167`) mixes reading the records
  with the game's replay: the reads get `SH3004`, the game's errors
  `SH3001` or `SH3002`. `new`'s `make` returns its error with its code,
  so `new` (`new.rs:289`) passes the right one on.

  **Either order with 2527:** if the `soak` op exists, its sites get
  codes as `bots`' do: its refused games and seed `SH0001`, the game's
  errors `SH3001` or `SH3002` through `game::code_of`, and a record
  `--save` can't write `SH3004`. A failure the soak finds is a verdict
  (`ok: false`), not an error, and gets no code. If 2527 isn't done,
  2527 adds these sites.

- **Types.**
  - `shuyi_assets::error::Code(pub &'static str)`, `no_std` like the
    rest of the crate (3005), and `shuyi_assets::error::codes` with the
    ten content codes as constants (`codes::PARSE`, …).
  - `ContentError` gains `pub code: Option<Code>` and `#[must_use] pub
    fn with_code(self, code: Code) -> Self`, and `not_found(file)`.
    Its `Display` doesn't change, so the tests that compare its text
    stand. The game builds no `ContentError` by struct literal (checked
    2026-10-20 with `gh search code`: its `crates/content/src/map.rs`
    and `crates/xtask/src/check_script.rs` use `ContentError::new`), so
    its pin bump builds unchanged. The PR says the field is new.
  - `crates/cli/src/contract/codes.rs`:
    - `Class` (`Usage`, `Setup`, `Content`, `Operation`) with
      `exit_code()`;
    - `Entry { code, name, class, title, help, page, retired }`, its
      `page` an `include_str!` of its file, so a missing page fails the
      build;
    - `TABLE: &[Entry]` and `entry(code: &str) -> Option<&'static
      Entry>`;
    - the cli's codes as constants of type `Code` (`codes::NO_PROJECT`,
      …), each named as its row's `name` in upper snake case; the
      content rows use `shuyi_assets::error::codes`.
  - In `game.rs`, next to `unsupported`: `is_unsupported(&str)` (the
    `SH3001` row) and `code_of(e: &str) -> Code`, `SH3001` when
    `is_unsupported(e)`, else `SH3002`. Every site that passes on a
    `GameTools` error uses it.
  - `Answer<T>` gains `code: Option<String>` and `help:
    Option<String>`:
    - `Answer::failed(op, code: Code, error)` takes the code, so no
      failure can be written without one, and the compiler lists every
      site;
    - `#[must_use] fn with_code(self, code: Code) -> Self` codes an
      answer that ran and failed on the setup (`doctor`);
    - its help is the row's unless `with_help(…)` gives a sharper one
      (a usage text, `doctor`'s checks);
    - `exit_code()` is 0 when `ok`, else its code's class's, and 1
      with no code.
  - `Problem` gains `code: Option<String>` and `help: Option<String>`;
    `From<&ContentError>` copies the code and takes the help from its
    row.
  - `doctor::Check`'s `fix` becomes `help`, with the same text, and
    `doctor::text` prints it as `help:`. A `doctor` with a failed check
    answers `SH1003` (exit 3), its help: fix each failed check as its
    help says, then run `doctor` again.
  - `code` and `help` serialize as `null` when absent, like `error`.

- **Exit codes.**
  - No `shuyi.ron` (`SH1001`), a `shuyi.ron` that doesn't read or
    lacks a part, or no game tool program (`SH1002`), and cargo that
    doesn't start (`SH1003`) exit 3, from `main_with`, `run`,
    `cli::no_game` and `game::run_in_game_program`.
  - `Project::find` (`project.rs:42`) tells a missing file (`SH1001`)
    from one that doesn't read (`SH1002`), returning the code with the
    message.
  - With no project, `main_with` (`lib.rs:104-111`) prints a failed
    `Answer` when the command is a contract op (`Op::named`) and
    `--json` is given, as `cli::no_game` does; otherwise the text, then
    the usage, as today.
  - The refusals in the `SH0001` row that are `Answer::failed` today
    exit 2, not 1. The `cli.rs` tests change with them: `new` into a
    folder that isn't empty (`cli.rs:417-420`) expects 2, `no_game`
    3 (`cli.rs:377-381`), `doctor` 0 or 3 (`cli.rs:409-412`), and the
    schema count 20 (`cli.rs:402`). So do the binary's tests in
    `crates/cli/tests/it/binary.rs`: no project
    (`outside_a_project_it_says_so_and_exits_2`, renamed `…_exits_3`),
    a missing part (`a_project_is_found_from_a_folder_below_it`), no
    game tool program (`the_commands_that_run_the_game_need_its_tool_program`,
    `the_ops_that_run_the_game_say_they_need_its_tool_program`) expect
    3; `new` into a folder that isn't empty 2, and the schema count 20
    (`new_doctor_and_schema_need_no_project`).
  - Exit code 3 means "fix the setup, then retry", nothing else. No
    exit code means "retry as it is": nothing in the CLI calls a
    network yet (Unity's 7). A later error that waits on something
    outside the project, like 2521's "no build listening", uses 3:
    starting the build is fixing the setup.

- **What people see.**
  - A failed answer: `check: error[SH1002]: shuyi.ron names no game
    tool program …`, then `  help: …`, then `  see: shuyi explain
    SH1002`.
  - A problem with a code: `files/stage.map:3:4: error[SH2001]:
    expected ','`. A problem without one prints as today. After the
    problems, one `help[SHnnnn]: …` line per code they used, written by
    one helper in `codes.rs` that `check::text`, `import::text` and
    `bake::text` share.
  - The commands outside the contract (`check-keys`, `frame-png`, …)
    print the setup errors they share with the contract (the paths in
    `lib.rs` and `game.rs`) the same way.

- **`explain`, a contract operation**
  (`crates/cli/src/contract/explain.rs`), so the MCP server (1133)
  gets it as a tool:
  - `shuyi explain` lists every code: code, name, class, title, and
    whether it is retired;
  - `shuyi explain SH2003` prints its page;
  - its input is `ExplainInput { code: Option<String> }`;
  - `--json` answers `ExplainOutput { entries: Vec<Explained> }`, one
    entry per code asked for (every code when none is), with `page:
    Option<String>` given when one code is asked for;
  - an unknown code fails with `SH0001`, its help naming `shuyi
    explain`;
  - it needs no project (`cli::without_project`), like `doctor` and
    `schema`;
  - `Op::Explain` joins `Op::ALL`, and its schemas join `schemas()`
    (20 files).

- **Pages.** `crates/cli/errors/SHnnnn.md`, one per code: a title
  (`# SH2003: A file isn't there`), the class and exit code, what it
  means, how to fix it, and one example of the message, in about ten
  lines. They live inside the crate's folder so `cargo package` takes
  them for 2005 (the templates at the root already need a fix there,
  ADR-0022's *Consequences*).

- **The gate**, a test in `crates/cli/src/contract/codes.rs`:
  - each code matches `SH[0-9]{4}`, its first digit matches its class,
    and no code or name is in the table twice;
  - every file in `crates/cli/errors/` is a row's page, and each page's
    first line names its code;
  - every row that isn't retired is used: its constant, which every
    site writes with its module (`codes::PARSE`, never a bare `PARSE`
    or `FILE`), appears in `crates/*/src/**/*.rs` outside the two
    files that declare the codes and outside tests (a file named
    `tests.rs`, or the text after `#[cfg(test)]`). The checker is a
    function over a table and file texts, tested on fake ones.

- **The contract's version.** The schemas regenerated, `VERSION`
  raised by one (to 5, or the next number if another ticket raised it
  first), and its line in `crates/cli/schemas/versions.json`
  (`docs/contract.md`, *The schemas*). **Either order with 3040:** both
  change the answers, and whichever lands second raises the version
  again.

- **Docs.**
  - An ADR (the `write-adr` skill, the next free number) extending
    ADR-0022 §1: codes and classes, exit codes 0 to 3, a code never
    reused, the help line, the pages, `explain`, and why the code is a
    field and not part of the message. Listed in `docs/adr/README.md`.
  - `docs/contract.md`: *The operations* gains `explain`; *The answer*
    shows `code`, `help` and the current version; a new section,
    *Problem codes*, gives the classes, the exit codes, `shuyi
    explain`, and the rule for a new error: a new code, its row and its
    page in the same PR.
  - `explain` in `lib.rs`'s usage text and in the op table of
    `contract.rs`'s module doc. The exit codes in `lib.rs`'s usage
    text and `main_with`'s doc comment, and in `cli.rs`'s module doc.
  - `templates/grid/AGENTS.md` and `templates/blank/AGENTS.md`, under
    *The tools*: a row for `cargo shuyi explain <code>`, and a line
    saying a failed command has a code and exit code 3 means fix the
    setup (`cargo shuyi doctor`), then run it again.

- **Either order with these tickets:**
  - **2802** (the book): if it is done, the book gets a page per code
    under *Learn*, made from `crates/cli/errors/`, and chapter 10 links
    them. If not, 2802 does it (edit requested there).
  - **2808** (`llms.txt`): nothing here. 2808 twins every page under
    `site/content/learn/`, so the code pages reach `llms.txt` and
    `llms-full.txt` once 2802 puts them there.
  - **2013** (constructors and `#[non_exhaustive]`): if it is done,
    `Code`, `ContentError`'s new field and `Entry` follow its rules. If
    not, 2013 covers them.
  - **2010** (public API diffs): if it is done, its job reports
    `ContentError`'s new field, `Answer::failed`'s new argument,
    `Answer`'s and `Problem`'s new fields and `Check`'s renamed one.
    The completion notes then carry a `Breaking:` line for
    `shuyi-assets` and one for `shuyi-cli`, each saying the game builds
    unchanged.
  - **2513** (`doctor` on Linux): its checks' `fix` lines are `help`
    lines. If 2513 is done, its rows and its `text` test move to
    `help` here. If not, 2513 writes `help` where it says `fix`.
  - **2017** (`contract.md` tested): if it is done, its test in
    `crates/cli/tests/it/docs.rs` parses *The answer*'s example as an
    `Answer<SceneOutput>` and fails until the example has this ticket's
    fields and version: keep it a `scene` answer. If not, the example
    is right when 2017 starts.
  - **2111, 2515, 2516, 2517, 2519, 2520, 2521**, and any other ticket
    done before this one that adds a contract error or a problem: their
    `Answer::failed` sites get codes here too (the new signature makes
    the compiler list them), and their problems get the content code
    of their class. 2517's `old-name` and `upgrade-by-hand` become
    rows.

- **Size.** If the PR passes about 600 changed lines (the pages, the
  generated schemas and snapshots not counted), the codes inside the
  keymap, language and map validators (`SH2008` to `SH2010`: their
  rows, pages, sites and tests) go to a follow-up ticket
  (`write-ticket`), MVP, and the PR says so. Adding a code to an error
  that had none is no break: `code` is in the shape already.

**Out (do not do):**
- a code for every message of every decoder: codes are by class, at
  the sites above;
- codes from a game's own validators, and a range of codes for a game
  (a follow-up if a game asks);
- codes on `TargetProblem` (`check --target`, whose `check` and
  `level` already key it) and on `FitProblem`;
- codes for the commands outside the contract, beyond the setup
  errors they share with it;
- a code for a game tool program that doesn't build: cargo's own exit
  code passes through `run_in_game_program` as today;
- translated messages or pages;
- an exit code for "retry as it is";
- compiler messages on the traits a game implements (2510);
- the log in the answers (3040);
- the pages on the website (2802), `llms.txt` (2808), the MCP server's
  tools carrying `code` and offering `explain` (1133), the skills
  branching on codes (1134), and the release that waits on this ticket
  (2005).

## Implementation steps

1. Read ADR-0022, `docs/contract.md`, `crates/cli/src/contract.rs`,
   `cli.rs`, `lib.rs`, `game.rs`, `project.rs` and `doctor.rs`, and in
   `shuyi-assets` `error.rs` and `format.rs`.
2. Write `Code`, the content constants, `with_code` and `not_found` in
   `shuyi-assets`, and code the sites in the table. Run the assets
   tests and the `no_std` build.
3. Write `contract/codes.rs`, the table, the pages and the gate test.
4. Change `Answer` and `Problem`; give `Answer::failed` its code
   argument, and code each site the compiler lists (`game::code_of`
   for the game's errors, `make` returning its code); `exit_code` by
   class.
5. Code the setup paths in `lib.rs` (with `main_with`'s JSON answer),
   `game.rs`, `project.rs` and `cli.rs`, and `doctor`'s answer and
   help. Change the exit codes the tests in `cli.rs` and
   `tests/it/binary.rs` expect, as *Exit codes* lists.
6. Write the `explain` operation and wire it. Regenerate the schemas,
   raise `VERSION`, add the line to `versions.json`.
7. Write the text output, the docs, the ADR and the templates'
   `AGENTS.md` lines.
8. Run the gates. Count the changed lines; past about 600, do *Size*.

## Acceptance criteria

- [x] In a folder with no `shuyi.ron`, `shuyi check --json` prints an
      answer with `"code": "SH1001"` and a help, and exits 3.
- [x] In a project whose `shuyi.ron` names no game tool program,
      `shuyi check` exits 3 with `SH1002`; wrong arguments exit 2 with
      `SH0001`; `replay missing.ron` exits 1 with `SH3004`; a fake game
      without `bots` answers `SH3001`; a `doctor` with a failed check
      answers `SH1003` and exits 3.
- [x] `shuyi explain` lists every code, `shuyi explain SH2003` prints
      its page, and `shuyi explain SH9999` exits 2 with `SH0001`.
- [x] A RON file with a syntax error, read by `import`, gives a problem
      with `"code": "SH2001"` and its row's help.
- [x] `Answer::failed` can't be called without a code.
- [x] The gate test passes, and its checker fails on fake tables with a
      page and no row, a code twice, and a row nothing uses.
- [x] The schemas, `VERSION` and `versions.json` agree (the schema
      test).
- [x] `docs/contract.md` shows `code`, `help`, the current version and
      the exit codes 0 to 3.
- [x] `shuyi-assets` still builds for `thumbv7em-none-eabihf`.
- [x] CI's `templates` job passes with the templates' new `AGENTS.md`
      lines.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-cli`): `Class::exit_code`; `Answer::exit_code` by class;
  `Answer` and `Problem` as JSON with and without a code; a problem's
  text with a code and the `help[…]` lines; `explain` with no code, one
  code and an unknown one; `Project::find`'s two errors;
  `game::is_unsupported` and `game::code_of`; `new` into a folder that
  isn't empty answers `SH0001`; `doctor` on a fake machine with a
  failed check answers `SH1003`.
- Unit (`shuyi-assets`): each coded site gives its code: a RON syntax
  error, a file that isn't UTF-8, a missing file through
  `Formats::find`, two files that could be one asset, a file no format
  reads, a file too big, a pack that lacks a file, a keymap's unknown
  action, a text key used twice, a crank notch out of range.
- Integration: `cli.rs`'s exit-code tests with the new codes, and the
  schema count of `schema --out` (20 files); in
  `crates/cli/tests/it/binary.rs`, the tests *Exit codes* lists, and
  `outside_a_project_it_says_so_and_exits_3` also runs `check --json`
  in the empty folder and reads `"code": "SH1001"`.
- The gate test.

## Completion notes

**Done.**

- `shuyi_assets::error::Code` and `codes` (`PARSE` … `PACK_DIFFERS`,
  `SH2001`–`SH2007`); `ContentError` gains `code`, `with_code` and
  `not_found`. Coded at the format registry (`text`, `decode_with`,
  `unread`, `find`), `NotText`, the RON reader (`parse_ron`'s error and
  `load_ron`'s missing file), the language formats, the font, the
  language pack, the dialogue reader's non-UTF-8 file, and
  `Bake::check` (its three messages and `same`'s "differs").
- `crates/cli/src/contract/codes.rs`: `Class`, `Entry`, `TABLE` (15
  rows), `entry`, `help_of`, `exit_code`, `said` (the
  `op: error[CODE]` block) and `help_lines`; the pages in
  `crates/cli/errors/`; the gate in `codes/tests.rs`, its checker tested
  on fake tables.
- `Answer` gains `code` and `help`; `Answer::failed(op, code, error)`;
  `with_code`, `with_help`; `exit_code` by class. `Problem` gains `code`
  and `help`, and prints `error[CODE]`. `check`, `import` and `bake`
  print one `help[CODE]` line per code after their problems.
- `game::is_unsupported` and `game::code_of`; every site that passes on
  a `GameTools` error uses `code_of` (`check`, its controls and one-bit
  skin, `run`/`scene`, `replay`, `bots`, `playtest`, `bake`).
- Setup: `Project::find` returns `(Code, String)` (`SH1001` or
  `SH1002`); `contract::cli::setup_failed` says a setup error once for
  `main_with` (JSON answer for an op with `--json`, else the text and the
  usage), `no_game`, `run_in_game_program` (no program, or cargo that
  doesn't start: `SH1003`) and the commands whose part `shuyi.ron`
  lacks. All exit 3.
- `doctor`: `Check::fix` is `help`; a failed check answers `SH1003`
  (exit 3) with its help.
- `explain` (`contract/explain.rs`), an op needing no project, by code
  or name; schemas regenerated (22 files), `VERSION` 7 and its
  `versions.json` line.
- ADR-0028; `docs/contract.md` (`explain`, the answer's `code` and
  `help`, version 7, *Problem codes*); the usage text and doc comments;
  the templates' `AGENTS.md`.

**Deviations.**

- **Size: SH2008–SH2010 moved to ticket 2530**, as *Size* says: the PR
  is about 1,500 changed lines of Rust (half of them tests) without the
  pages and schemas. The keymap's, language packs' and maps' problems
  keep `code: null` until 2530.
- The ticket's counts were written before `playtest` (2126): the schema
  count is **22**, not 20, and `VERSION` went from 6 to **7**, not 5.
  `playtest`'s refusals and the game's errors got codes like `bots`'.
- `Project::part` lost its `command` argument and says
  "shuyi.ron has no `web`"; the command's name comes first in the coded
  line instead (`web: error[SH1002]: …`).
- Also coded, beyond the table's sites, the same classes where the same
  message is given: `load_ron`'s missing file, the dialogue reader's
  non-UTF-8 file, `Bake::check`'s "the pack has this file and the
  sources don't" and its kinds' "read from the pack differs"; `check`'s
  one-bit skin that doesn't load (the game's error: `code_of`); and
  `schema`'s two refusals print `error[SH0001]`.
- `explain` takes a code's name too (`shuyi explain missing-file`), and
  its JSON entries carry `exit_code` and `help` beside the fields the
  ticket lists.
- A failed answer's text indents a multi-line help (a usage) under
  `help:`.

**Either-order tickets.** None of 2010, 2013, 2017, 2111, 2513,
2515–2521, 2527, 2802 or 3040 is done, so each picks up its part as its
own text says.

**For the game.** `ContentError` has a new field, `code`; the game
builds `ContentError` with `new` only, so its pin bump builds
unchanged. `Answer::failed` takes a code, `Answer` and `Problem` have
`code` and `help`, `doctor::Check::fix` is `help`, `Project::find`'s
error is `(Code, String)` and `Project::part` takes no command: none of
these is used by the game's code. Its CI or scripts that expected exit
2 for a missing `shuyi.ron`, part or game tool program get 3 now.
