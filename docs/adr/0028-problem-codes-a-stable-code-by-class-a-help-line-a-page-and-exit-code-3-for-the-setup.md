# ADR-0028: Problem codes: a stable code by class, a help line, a page, and exit code 3 for the setup

- **Status:** Accepted
- **Date:** 2026-10-21
- **Related tickets:** 2509 (this), 1133 (the MCP server), 1134 (the
  skills), 2005 (the first release), 2802 (the book), 3040 (the log in
  the answers)
- **Extends:** ADR-0022 §1 (every answer is an `Answer<T>`)

## Context

ADR-0022 §1 gives every operation one answer shape, whose `error` says
in prose why an operation could not run, and three exit codes: 0 passed,
1 failed, 2 wrong arguments. A content problem (`Problem`, made from
`shuyi_assets::ContentError`) has a file, a place and a message.

- **Prose is all a tool can match.** A skill, the MCP server or a CI
  step that wants to say what to do next has to read the message, and
  then the message's words can't change.
- **"Your command is wrong" and "your setup is wrong" looked the same.**
  No `shuyi.ron`, a `shuyi.ron` that lacks a part, and no game tool
  program all exited 2, as wrong arguments do. An agent can't tell
  "change the command" from "fix the project, then run it again".
- **The benchmarks key their errors.** Unity 6's command-line tool
  answers with a stable code (`INVALID_COMMAND_ARGS`) and exit codes by
  class (2 usage, 3 authentication, 4 configuration, 6 failed, 7
  retryable). Bevy numbers its errors with a page each (B0001 to B0006).
  `rustc --explain E0277` prints a code's page.
- **Before the first release.** Codes and exit codes are a public
  promise. Made before 2005, they are part of the first contract instead
  of a break after it. Nick asked for "logs and error codes agents can
  read" among the MVP's protections (`docs/decisions.md`, 2026-10-20).

## Decision

### 1. A code is `SH` and four digits; the first digit is its class

| Class | Codes | Exit code | Means |
| ----- | ----- | --------- | ----- |
| usage | `SH0nnn` | 2 | the arguments or the input are wrong: change the command |
| setup | `SH1nnn` | 3 | the project file or the toolchain needs fixing: fix it, then run the same command again |
| content | `SH2nnn` | 1 | a file of the game's content has a problem |
| operation | `SH3nnn` | 1 | the operation failed on something else: the game's tools, a target, a file it reads or writes |

- A code is **never reused or renumbered**. A retired code keeps its row
  (`retired: true`) and its page, which says when and why.
- Each code has a kebab-case **name** (`missing-file`), as stable as the
  number, for docs and for tickets that name a code before it has one.
- Codes are **by class at their sites**, not one per message: "a file
  isn't there" is `SH2003` whichever loader says it.
- Exit code 3 means only "fix the setup, then run it again". **No exit
  code means "retry as it is"**: nothing in the CLI calls a network. An
  error that waits on something outside the project (a build not
  running yet) is a setup error: starting it is fixing the setup.

### 2. The code is a field, not part of the message

- `shuyi_assets::error::Code(pub &'static str)` (`no_std`, like the
  crate), with the content codes in `shuyi_assets::error::codes`.
  `ContentError` gains `code: Option<Code>`, set by `with_code`, and
  `not_found(file)` for the missing file. Its `Display` doesn't change.
- `shuyi_cli::contract::codes` holds the table: `Class` (with
  `exit_code`), `Entry { code, name, class, title, help, page, retired }`
  and `TABLE`, each page an `include_str!` of `crates/cli/errors/<code>.md`
  so a missing page fails the build. The tools' own codes are constants
  there (`codes::NO_PROJECT`), named as their row's `name` in upper snake
  case; every site writes them with the module (`codes::FILE`).
- `Answer<T>` gains `code` and `help` (`null` when absent).
  `Answer::failed(op, code, error)` takes the code, so no failure is
  written without one; `with_code` codes an answer that ran and failed
  on the setup (`doctor`), and `with_help` gives a sharper help than the
  row's (a usage). `exit_code` is 0 when `ok`, else its code's class's,
  1 with no code.
- `Problem` gains `code` and `help`, copied from the `ContentError` and
  its row.
- The errors a game's `GameTools` method returns are `SH3001` when the
  method is left at its default (`game::is_unsupported`, by the text
  `unsupported` starts with) and `SH3002` otherwise (`game::code_of`).

A field lets the message's words change, and lets a tool branch without
parsing text.

### 3. People see the code, the help, and where to read more

```text
check: error[SH1002]: shuyi.ron names no game tool program (`game`), and check runs the game
  help: fix shuyi.ron as the message says, then run the same command again
  see: shuyi explain SH1002
```

A problem with a code reads `files/stage.ron:3:4: error[SH2001]:
Expected comma`, and after a list of problems comes one
`help[SHnnnn]: …` line per code used (`codes::help_lines`). The setup
errors of the commands outside the contract (`check-keys`, `frame-png`)
print the same way.

### 4. `shuyi explain`, an operation of the contract

`explain` lists every code (code, class, name, title, retired), and
`explain <code>` (or its name) prints its page. It needs no project.
With `--json` it answers `ExplainOutput { entries }`; an unknown code is
`SH0001`. As an operation, the MCP server (1133) gets it as a tool.

### 5. A new error is a new code, its row and its page in the same change

A test (`contract/codes/tests.rs`) gates the table: every code is `SH`
and four digits with its class's first digit, no code or name twice,
every file in `crates/cli/errors/` is a row's page whose first line names
its code, and every row that isn't retired is used: its constant appears
in `crates/*/src` outside the files that declare the codes and outside
tests.

## Consequences

- An agent classifies a failure by its exit code alone, and by its code
  knows what to do without reading the message; `shuyi explain` saves it
  a guess.
- The exit code of no project, a project file that lacks a part, and no
  game tool program changes from 2 to 3, and `new`'s refusals from 1 to
  2: a script that tested for 2 there must test for 3.
- `ContentError`, `Answer`, `Problem` and `doctor::Check` (whose `fix`
  is now `help`) change shape: `contract::VERSION` 7. A game that builds
  `ContentError` with `new` builds unchanged.
- Every new error costs a row and a page. That is the point: the gate
  makes an error without its help impossible to merge.
- The pages live inside `crates/cli/`, so `cargo package` takes them for
  the release (2005); the website's pages (2802) are made from them.
- The validators' own problems (the keymap, the language packs, the
  maps) have no code yet: a follow-up ticket gives them `SH2008` to
  `SH2010`. Adding a code where there was none is no break.

## Alternatives considered

- **The code inside the message** (`"SH2003: file not found"`) — a tool
  would still parse text, every test that compares a message would
  change, and `ContentError`'s text is the game's too.
- **A string key like Unity's** (`INVALID_COMMAND_ARGS`) — readable, but
  long in a line of text and with no class in it; the kebab-case name
  gives the readable form beside a short number.
- **A code per message** — hundreds of codes for the decoders, most
  saying "this file's content is wrong" in a new way; an agent gains
  nothing from telling them apart, and the table would never be done.
- **More exit codes** (Unity's 4 configuration, 7 retryable) — the CLI
  calls no network, so "retry as it is" means nothing yet, and one
  setup code covers the project file and the toolchain alike: both are
  "fix it, then run it again".
- **Pages on the website only** — `shuyi explain` works offline and on
  the version installed, and the website takes its pages from the same
  files.
