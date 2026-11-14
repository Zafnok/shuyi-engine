# ADR-0115: Exercises: a task in a step's playground, checked by checks kept beside the step and compiled into the copy of it the site plays

- **Status:** Accepted
- **Date:** 2026-11-13
- **Related tickets:** 2828, 2820, 2827, 3069, 2823
- **Extends:** ADR-0090 §8 (what an exercise is, past where it sits)

## Context

The book is "a unified learning resource and highly interactive" (Nick,
2026-11-03, `docs/decisions.md`). A quick check (2827) asks whether the
reader followed; an exercise asks them to do something, in the step's
playground (2820), and the game says whether it is done, as Exercism's
tasks and Rustlings' exercises are each checked by a test. ADR-0090 §8
says only where an exercise sits: in the step's `## Check yourself`,
after its quick checks. The playground edits data files, never Rust
(decision of 2026-10-28), so an exercise's task is a change to a data
file.

What there is:

- **3069:** a web build lays the files its page holds in
  `sessionStorage` (`shuyi-page-file:<path>`) over its own, through
  `shuyi_app::page::page_layer`, and `page::report` sends the page its
  problems, `{ kind: "shuyi-report", text }`, through our own miniquad
  plugin, `crates/cli/web/page-files.js`.
- **2820:** every step of the book's project has a playground of its data
  files beside its game. `shuyi_app::run_game` lays the page's files and
  reports, so a step's game takes them with no code of its own. Every step
  has data files, so every step gets exercises.
- **2827:** a quick check is the body shortcode `check`, read by
  `book.rs` (`cargo xtask check-book`) and written as text in the twins by
  `llms.rs`.

A check is code that knows the step: its levels' format, its rules, its
own part of the data. The step's project is what the page asks the reader
to type (`site/tutorial/<step>/`, linked as *Its whole project*, built and
tested by CI's `tutorial` job, `git diff --exit-code` after), so a check
written there would put code in the reader's project that the page never
asks for.

Tried on step 2 by hand, in a browser, against a local build of the site:
the copy's binary, built with `cargo shuyi web`, read `shuyi-page-check`,
laid the page's edited `levels/1.map`, checked it and posted the result to
the page; a broken map posted its problems and no result; and a message
posted by the page itself, not its frame, was ignored.

## Decision

### 1. Where the checks live: `site/exercises/`, spliced into a copy

The checks are plain Rust beside the steps, never in them:

- `site/exercises/<step>.rs`, one file a step, named as its folder
  (`02-board.rs`): the step's checks, each a function taking what the
  step's game loaded (its `BasicData`: its levels in its own part, its
  palette, its text, its keys) and returning `Ok(why it passed)` or
  `Err(why not)`, a reason a reader can act on ("level 1 has 1 box: add a
  second, and a target for it"). It ends with `pub const CHECKS`, each
  check's name and function, and tests: each check passes on the step's
  own files with its exercise's edit laid over them, and fails without.
- `site/exercises/main.rs`: the binary every copy runs, the step's
  `main.rs` but for calling `run_game_checked(game::game(), checks::CHECKS)`
  where the step calls `run_game(game::game())`.
- `site/exercises/<helper>.rs`: what several steps share (a search that
  plays a level through the step's rules, the palette's colors read as a
  person names them), each file a module a step's checks name with
  `#[path]`.

`cargo xtask exercises` (`crates/xtask/src/exercises.rs`) makes, for each
step, a copy under `target/exercises/<step>/` (three folders down, so the
step's path dependencies on the engine still resolve, as `tutorial-tries`
does), leaving out its `target/` and `dist/`, then:

- puts `main.rs` in the copy's `crates/game/src/main.rs`, and the step's
  file and every helper in `crates/game/src/exercises/`, the step's as
  `checks.rs`;
- with no flag, checks the copies: `rustfmt --check` on `site/exercises/`,
  and in each copy `cargo clippy --locked --bin box-pusher --tests -- -D
  warnings` and `cargo test --locked --bin box-pusher`, one target folder
  for them all (`target/exercises-build`);
- with `--copy`, only makes the copies, for the web builds.

The copy keeps the step's `Cargo.toml` and `Cargo.lock` unchanged: the
checks add no dependency, so `--locked` holds and no lock is added. The
binary is the step's own crate's (`box-pusher`), so the checks use the
step's types (`game::level::Level`, `game::rules::Puzzle`) by their paths.

The xtask also checks the pages against the files: every step with
`extra.files` has a `site/exercises/<step>.rs` and one or two exercises,
each exercise's `check` is a name its step's `CHECKS` holds (as a string
`"name"` in the file), and each name there is asked for by an exercise.

### 2. The site plays the copy

`pages.yml` builds each step's `play/` from its copy (`cargo xtask
exercises --copy`, then `cargo shuyi web --release` in
`target/exercises/<step>/`), and CI's `web-render` job boots those builds.
The step's own folder is still what the page shows and CI's `tutorial`
job tests: the copy differs from it by `main.rs` and the folder of
checks, which a reader never sees. CI's `exercises` job runs `cargo xtask
exercises`.

### 3. The game: `run_game_checked`, `page_check`, `CheckReport`, `report_check`

In `shuyi_app::page`:

- `page_check() -> Option<String>`: the name of the check the page asks
  for, `sessionStorage`'s `shuyi-page-check`; `None` when it holds none,
  and on every target but the web.
- `CheckReport { name, passed, text }`, `#[non_exhaustive]`, built with
  `CheckReport::new` (ADR-0039).
- `report_check(&CheckReport)`: on `wasm32` sends the parent frame
  `{ kind: "shuyi-check", name, passed, text }` at the page's origin (the
  console when there is no such frame), as `report` does; elsewhere
  nothing.

The JS is two more functions of `page-files.js`, `shuyi_page_check` and
`shuyi_page_report_check`, and its version moves to 2 with
`PLUGIN_VERSION`; `shuyi-cli`'s tests keep the lists and the versions
equal.

`shuyi_app::run_game_checked(game, checks)` is `run_game` with checks, a
slice of `(name, fn(&BasicData) -> Result<String, String>)` (the type
`PageCheck`). After the page's files are laid and reported as `run_game`
does, when the page asks for a check: if its files have problems, the
page has them and gets no check result; otherwise the game's data is read
once more, the named check runs on it and its result is reported; a name
the game has no check for is reported failed, saying so. Then the game
plays as usual. A check that plays runs a list of commands through the
step's rules (`TurnRules::apply`), headless: the Harness is a test
feature, not in a web build.

### 4. The page: the `exercise` shortcode and `playground.js`

```text
{% exercise(check="second-box", file="levels/1.map", to="games") %}
Add a second box to level 1, and a target for it.

Hint: a box is a feature, `(x, y): Box`. A target is an `o` under `---`.
{% end %}
```

- A body shortcode, placed as a check is (ADR-0090 §8): in `## Check
  yourself`, after the checks, never inside a mark. The lines before the
  paragraph starting `Hint: ` are the task, the rest the hint, both
  needed. `check` is the name, lowercase letters, digits and `-`; `file`
  one of the page's `extra.files`; `to` optional, an audience as
  ADR-0090 §5's. `book.rs`'s `kind()` opens it (`Opened::Exercise`), so
  its `{% end %}` closes it.
- The page shows the task, *Do it in `files/<file>`*, the hint in a
  `<details>`, and two buttons: **Open `files/<file>`** (unfolds the
  playground, selects the file's tab, scrolls to it and focuses its
  editor) and **Check my work**. The result goes in the exercise's
  `role="status"` line: *Done.* or *Not yet.* and the check's text. With
  `to`, it is an `aside.exercise.newcomer` with a `.newcomer-label`, so the
  reader's filter folds it and the bar counts it as an exercise.
- **Check my work** writes the name to `shuyi-page-check`, then runs the
  playground as **Run** does. The listener takes `shuyi-check` messages
  only from the playground's frame at the page's origin, as it takes
  `shuyi-report`. The key is removed when a result or a report of
  problems comes, by **Run**, and as the page goes (`pagehide`), so a game
  played on its own page, or run again, checks nothing. A report of
  problems shows under Problems as before, and the exercise says to fix
  them first.
- Buttons show only once the game's text is fetched and the playground
  holds the exercise's file. With no JavaScript the task and the hint
  still show.
- **The twin:** `**Exercise.**`, the task, where it is done ("in this
  step's playground, in `files/<file>`, where the game checks it"), and
  the hint as text.

Nothing is kept past the tab: no score, no progress, no account.

## Consequences

- The reader's project stays exactly as the page says, and every step's
  page has the same three files of machinery behind it: a main, its
  checks, the helpers.
- A check is tested against the step's own files and its exercise's edit
  in CI, so an exercise can't go stale when a step's files change.
- The site's `play/` is now a copy's build. It is the step's game, and a
  change to a step reaches it the same way, but a bug only in the copy's
  `main.rs` would show only there: it is one line, and `web-render` boots
  it.
- Each step's crate is built twice in CI (the `tutorial` job and the
  `exercises` job), on top of one shared build of the engine.
- `run_game_checked` is public: any game on the short path with a page
  can have checks, not only the book's.

## Alternatives considered

- **Checks in each step's project** (a module of its crate, or an
  `examples/` binary) — puts code in the reader's project the page never
  asks them to type, and the *whole project* link would show it.
- **A cargo project of checks per step, depending on the step's crate by
  path** — ten more workspaces with ten more locks to keep current; one
  workspace holding all ten can't, as the steps' crates share a name and
  version (Cargo's lock refuses two path packages so).
- **A hook on `BasicGame`** (`.checks(…)`) — the step's `lib.rs` would
  have to call it, which is code the page asks the reader to type.
- **Checks as data the engine interprets** (a scenario of key presses and
  a condition on the scene) — reasons like "level 1 has two boxes but one
  target" need the step's types, and the ticket asks for checks in plain
  Rust.
- **The page checks in JavaScript, or in a second WebAssembly module** —
  the page would need its own parsers of the step's formats, and couldn't
  play the step's rules; the game already knows its files.
