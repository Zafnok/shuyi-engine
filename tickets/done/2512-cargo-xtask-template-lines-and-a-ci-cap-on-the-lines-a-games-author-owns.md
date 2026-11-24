---
id: "2512"
title: "Lines a game's author owns: cargo xtask template-lines over the templates and examples, and a CI cap on the minimal template"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2511"]
nick_input: none
completed: 2026-11-23
---

# 2512 — cargo xtask template-lines, and a CI cap on the lines a game's author owns

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. The gap also cites PICO-8, which caps a cartridge's code
at 8192 tokens
(<https://www.lexaloffle.com/dl/docs/pico-8_manual.html>), so its games
stay small by rule.

**What Shuyi has today:**

- Nick's end goal (2026-10-19,
  [`docs/decisions.md`](../../docs/decisions.md)): an agent makes snake
  "in not too many LOC", and every new API is judged against it.
  Nothing measures that, and nothing enforces it.
- `cargo xtask` (`crates/xtask/src/main.rs`) has `ticket-lint`,
  `check-links` and `check-no-std`, and depends only on `serde_norway`
  and `pulldown-cmark` (`crates/xtask/Cargo.toml`). `no_std.rs` already
  reads Rust line by line and drops `//` comments (`code_of`), but it
  knows no strings and no `/* */` blocks.
- `shuyi-cli` already has the scanner this ticket needs (re-read
  2026-10-28, found by an analysis of the engine for areas that do the
  same job twice): `check_keys::lex` (`crates/cli/src/check_keys.rs`,
  `pub(crate)`) splits Rust or RON into `Line { code, comment, strings }`
  and knows `//` and nested `/* */` comments, strings with escapes, raw
  strings, and chars apart from lifetimes. `exempt_lines` and
  `mark_item` find each `#[cfg(test)]` item by counting brackets on
  those lines. `check_text.rs` uses all three.
- CI's `tickets` job (`.github/workflows/ci.yml`) builds the xtask and
  runs `ticket-lint` and `check-links` on every PR, docs-only ones too.
- 2015 added `serde`, `serde_json` and `ron` to the xtask, and the
  first RON file under `tools/`, `tools/layers.ron`.
- **From 2009 (done):** `tools/compile-budget.ron` holds a new game's
  rebuild time and its dependency count, which `cargo xtask
  compile-times` (`crates/xtask/src/compile_times.rs`) checks in CI's
  `compile-times` job. It doesn't count lines. This ticket's budget is a
  second file beside it; nothing else is shared.
- Planned, not done: 2524 (the agent eval) counts the lines an agent
  adds, with `git diff`, not the lines a template hands its author.
- The counts on 2026-10-20, made with a short script by the rule below
  (the tool may differ by a few lines):

  | Folder | Code | Tests | Data | Setup |
  | ------ | ---- | ----- | ---- | ----- |
  | `templates/blank` | 437 | 129 | 53 | 131 |
  | `templates/grid` | 916 | 314 | 81 | 149 |
  | `examples/hello-grid` | 1871 | 1799 | about 226 | 66 |

  2511 (MVP, done first) brings the blank template's code to 120 lines
  or fewer.
- **The template has drifted past 2511's cap** (re-counted 2026-11-23,
  found by an audit of the engine's blind spots). 2511's notes counted
  71 lines after it, and 2501's 115. Today, by the rule below, over
  `templates/shuyi-default/base` and the `stages.rs` that `stages_rs`
  (`crates/cli/src/template/compose.rs`) writes for `blank`:
  `lib.rs` 61 (outside its `#[cfg(test)]` item), `title.rs` 73,
  `main.rs` 5, `examples/tools.rs` 5, the written `stages.rs` about 22,
  and `build.rs` 45. That is about 165 lines without `build.rs`, which
  2511 didn't have, and about 210 with it. `app_info.rs` is generated
  and left out. The lines came with later tickets, none of which had a
  cap to meet: 2111 (the stamp), 3048 (the identity), 3094 (the crash
  record), 3082 (editions, and `build.rs`), 3300 (the `packed`
  variants of `files()`), 2590 (the private files) and 2087.
- **When the cap lands** is Nick's call in decision 3528 (the release's
  guards): whether it moves ahead of the first release (2005). This
  ticket stays After MVP until then.

**Why it matters:**

- **The game first.** Nothing here touches Visions of Shuyi, which isn't
  counted. Its own friction review (game ADR-0092) measures its screens.
- **Other games** start from a template that stays small. Without a
  gate the lines creep back. Several after-MVP tickets each planned a
  few lines in the templates: 2225's menu sounds, 3043's background cap,
  3046's crash reports and 3048's identity. On 2511's short path their
  parts go in the engine (`BasicData`, `run_game`, the `()` settings),
  and the cap keeps it so: a part every new game needs goes in the
  engine, where every game gets it.
- **The end goal.** "Few lines" becomes a number CI checks. The parked
  examples (2736 to 2739, snake among them) quote their own counts from
  this command.

## Nick input

None to do the work. The cap is the minimal template's count when the
ticket is worked, a technical choice, and it only goes down. Whether
the cap comes before the first release (2005) is asked in decision 3528,
not here.

## Scope

**In:**

- **`cargo xtask template-lines [<folder>...] [--json] [--check]
  [--summary <file>]`** (`crates/xtask/src/template_lines.rs`, with its
  line in `main.rs`'s module docs, `USAGE` and `match`, and in the
  `description` of `crates/xtask/Cargo.toml`):
  - **The folders:** each folder under `templates/` and `examples/`,
    sorted, or the folders given. A folder may be any game's root, such
    as one made by `shuyi new`. Paths in the output use `/`, on Windows
    too.
  - **Every file gets a kind.** The left-out rules come first, so a
    license text under `files/` is left out, not data.
    - **left out:**
      - the folders `.git/`, `target/` and `dist/` (the templates'
        `.gitignore` names the last two), and `Cargo.lock`;
      - Markdown;
      - files that aren't UTF-8 (pictures, packs, `atlas_1bit.bin`);
      - license texts (a file name with `LICENSE`, `LICENCE`, `NOTICE`
        or `COPYING` in it);
      - generated files, whose first line says `Generated by` and `Do
        not edit`. `font-atlas` writes that header
        (`crates/cli/src/font_atlas.rs`, so every `font/atlas.ron`), and
        so does `app-info` (`crates/cli/src/app_info.rs`, so every
        `crates/game/src/app_info.rs`, 3048);
    - **code:** `.rs` files outside `tests/` folders, without their
      `#[cfg(test)]` items. `build.rs` and `examples/*.rs` are code;
    - **tests:** `.rs` files under a `tests/` folder or named
      `tests.rs`, and the `#[cfg(test)]` items of code files;
    - **data:** text files under a `files/` folder;
    - **setup:** `Cargo.toml`, `shuyi.ron`, `rust-toolchain.toml`,
      `rustfmt.toml`, `clippy.toml`, `.cargo/config.toml` and
      `.github/**/*.yml`;
    - **any other file** is left out too: `.gitignore`, a font's source
      under `files-src/`, test snapshots.
  - **A line counts** when it isn't blank and isn't only a comment. The
    comment marker goes by the file's extension: `//` for `.rs`, `.ron`
    and `.map`, and `#` for `.dlg`, `.toml` and `.yml`. In `.rs` and
    `.ron`, a `/* … */` block is a comment too, and in `.rs` blocks
    nest. A code line that ends in a comment counts. A text file with
    any other extension has no comment marker: every line that isn't
    blank counts.
  - **One scanner for `.rs` and `.ron`, the one `check-keys` uses.**
    `check_keys`'s `lex`, `Line`, `exempt_lines` and `mark_item` move to
    a public module of their own, `shuyi_cli::rust_lines`
    (`crates/cli/src/rust_lines.rs`), and `check_keys.rs` and
    `check_text.rs` call them from there. The xtask calls the same
    module through its dependency on `shuyi-cli` (the 2501 clause
    below). `tools/layers.ron` needs no line: it lists only `shuyi`
    and `shuyi-*` crates, and the xtask is neither. No second lexer is
    written, so a `//`, a `/*` or a brace inside a string or a char
    changes nothing, as in `check-keys` today. A line counts when its
    `code` isn't blank or it is inside a string that spans lines. The
    move is additive for the game (new public items, nothing removed).
  - **A `#[cfg(test)]` item** is what `exempt_lines` marks: from the
    attribute to the end of the item after it, its `;` (`mod tests;`),
    or the `}` that closes its first `{`. If the counter needs a rule
    `check-keys` lacks, change it in `rust_lines` with a test, and
    `check-keys`' and `check-text`'s tests still pass unchanged.
  - **The output:**
    - by default, a table: folder, code, tests, data, setup;
    - `--json`: the same, with every file's path, kind and lines. A
      left-out file has the kind `left-out` and 0 lines;
    - `--summary <file>`: the table appended as Markdown (CI passes
      `$GITHUB_STEP_SUMMARY`).
- **The cap**, in `tools/line-budget.ron`:
  - one record, `(template: "blank", code: N)`. `N` is the blank
    template's code count when this ticket is worked, rounded up to a
    multiple of 10 (110 stays 110, 111 becomes 120). That is about 210
    today, not 2511's 120 (*Context*): the PR says so, and the cap only
    goes down from there;
  - a comment above it says what it is, and that it is lowered, never
    raised (`AGENTS.md` rule 6): a part every new game needs goes in the
    engine (2511's ADR).
- **`--check`** reads the cap and counts the template it names
  (`templates/blank`), whatever folders are given. It fails:
  - when the template's code is over the cap. The message names its five
    biggest code files, and says that a part every new game needs goes
    in the engine;
  - when the cap is 10 or more above the count. The message says "lower
    `code` to M in tools/line-budget.ron", with M the count rounded up
    to a multiple of 10, so the cap follows every shrink.

  Only that template's code is capped. The other templates and
  examples, and all tests, data and setup, are reported only: they grow
  with features, and a cap on tests would discourage them.
- **CI:** a step in the `tickets` job of `.github/workflows/ci.yml`,
  after the link check: `cargo xtask template-lines --check --summary
  "$GITHUB_STEP_SUMMARY"`. That job already builds `xtask`, and it runs
  on every PR.
- **The skills and docs:**
  - `run-gates`: the command, in *The repository's own checks*;
  - `write-ticket`: one line. A ticket that would add code to the
    minimal template puts the part in the engine instead (this gate);
  - `docs/contract.md`, *The templates*: what counts, the cap, and the
    command.
- **`xtask`'s dependencies:** `serde` (with `derive`) and `ron` to read
  the cap, and `serde_json` for `--json`. `serde` and `ron` come from the
  workspace with `features = ["std"]`, as `crates/cli/Cargo.toml` has
  them. All three are under MIT OR Apache-2.0, already in `Cargo.lock`
  through `shuyi-cli`, and allowed by `deny.toml`'s `[licenses] allow`
  list. `cargo deny` sees no new crate.
- **From 2015:** the xtask already has the three crates, and
  `tools/line-budget.ron` follows the comment style of
  `tools/layers.ron`.
- **From 2511 (done):** ADR-0036 §9 leaves a derive for `ActionSet` to
  these counts. The PR description gives the code lines of each game's
  own `ActionSet` enum (`action.rs` in the grid template and in Hello,
  Grid; the blank template uses `UiAction`). If games pay for their own
  enum, write the derive's ticket (the `write-ticket` skill); if not,
  the completion notes say so.
- **Either order:**
  - **From 2501 (done):** `templates/blank` and `templates/grid` are
    gone. The counter counts each built-in short name (`blank`, `grid`)
    from `compose`'s plan (`shuyi_cli::template::compose`, with the
    placeholders filled as `shuyi new` fills them:
    `shuyi_cli::contract::new::fill_for`). `compose` is pure, so no
    folder is written. The xtask then depends on `shuyi-cli` (a path
    dependency, which links no device crate), and the cap names the
    short name. 2501's PR counted by 2511's rule: the grid game 674
    lines before, 521 after; the blank game 75 before, 115 after (the
    generated `src/stages.rs` and the title's line for a stage).
  - **The parked examples, either order with 2735, 2736, 2737, 2738
    and 2739:** each one's README gives the lines of its own code. If
    one is done, its folder is already in the table (every folder
    under `examples/` is counted): replace the count its README made
    by hand with this command's, and say so. If not, nothing to do:
    its README quotes this command (its clause says so). For 2738
    (the parked brick-breaker), the line to change is where its README
    counted `examples/bricks` by hand and said how: it becomes this
    command's code count.
  - **From 3094 (done):** the grid template's crash record is in; its
    lines in the grid game are counted like the rest, and the cap is set
    from the count with them.

**Out (do not do):**

- changing any template or example to lower its count (2511, and 2501
  for the grid template);
- counts on the website's Examples cards (2801): its page may read
  `--json` later;
- feeding the agent eval (2524) or its scheduled runs (2525): they may
  read `--json` later;
- a cap on any folder but the minimal template;
- counting tokens, as PICO-8 does: lines are what people and agents
  read;
- a contract operation for games (`shuyi lines`): a follow-up if a game
  asks.

## Implementation steps

1. Read `crates/xtask/src/main.rs`, `no_std.rs` (`code_of`,
   `rust_files`), `crates/xtask/Cargo.toml`, `crates/cli/src/check_keys.rs`
   (`lex`, `exempt_lines`, `mark_item`) and `check_text.rs`, the
   `tickets` job in `ci.yml`, and the three folders, and
   `tools/compile-budget.ron` beside the budget this ticket adds.
2. Move the scanner to `shuyi_cli::rust_lines`, with `check-keys`' and
   `check-text`'s tests passing unchanged. Then the line rule in
   `template_lines.rs` on it, with fixtures as strings.
3. Write the line rule for the other extensions, the kinds, and the
   folder walk.
4. Write the table, `--json` and `--summary`. Wire the command into
   `main.rs`, `USAGE` and the crate's `description`.
5. Run it. Write `tools/line-budget.ron` from the blank template's code
   count, then `--check` and its tests.
6. Add the CI step, the two skills' lines and the contract's paragraph.
7. Run it on the repository, and put the table in the PR description.
   Run the gates.

## Acceptance criteria

- [x] `cargo xtask template-lines` prints code, tests, data and setup
      for each built-in template (`blank`, `grid`) and
      `examples/hello-grid`. The PR description has the table. The
      blank template's code is within a few lines of the 2026-11-23
      count in *Context* (or the change is explained), and any other count that
      differs from *Context* by more than a few lines is explained.
- [x] `--check` passes on the repository. With the cap one lower it
      fails and names the files; with the cap 10 higher it fails and
      asks to lower it. Unit tests cover both, and the PR shows a run.
- [x] CI's `tickets` job runs the step and writes the table to the
      run's summary.
- [x] `--json` parses, with every file's path, kind and lines.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit, the line rule: blank lines; `//`, `///` and `//!`; a `/* */`
  block over several lines; a nested `/* /* */ */` block; a `//` or a
  `/*` inside a string on a code line, which counts.
- Unit, `#[cfg(test)]` items: `mod tests { … }` with `"}"` and `'{'`
  inside it; a lifetime `'a` beside a `'{'`; nested braces; `mod
  tests;`; a raw string holding `}`.
- Unit, the kinds of a fixture folder's files: a license text under
  `files/`, a generated header, a picture, Markdown, a file under
  `target/`, a `files/x.dlg` with `#` comments, a `files/x.txt` with no
  comment marker, and `.gitignore`.
- Unit, the cap: under, equal, over, and 10 or more above (with M in
  the message).
- Unit: the summary's Markdown for fixed counts, and `--json` paths with
  `/`.
- Integration: the run on the repository, in CI.

## Completion notes

Done 2026-11-23.

- **The scanner moved.** `check_keys`'s `lex`, `Line`, `exempt_lines`,
  `mark_item` and `is_ident` are now `shuyi_cli::rust_lines` (public),
  and `check_keys.rs` and `check_text.rs` import them from there. Their
  tests pass unchanged; the lexer's own tests stay in `check_keys.rs`.
  One rule was added for the counter: `Line::in_string`, true on a line
  that begins inside a string started above, so the lines of a string
  over several lines count. `Line` is `#[non_exhaustive]` (a view the
  lexer builds, ADR-0039).
- **`cargo xtask template-lines`** (`crates/xtask/src/template_lines.rs`)
  counts each short name of `templates/shuyi-default` (`blank`, `grid`,
  composed with `shuyi_cli::contract::new::plan`, which fills the
  placeholders as `shuyi new` does and writes nothing) and each folder
  under `examples/`, or the short names or folders given. `--json`,
  `--summary` and `--check` as the ticket says. CI's `tickets` job runs
  `--check --summary "$GITHUB_STEP_SUMMARY"` after the link check.
- **The counts on 2026-11-23:**

  | Folder | Code | Tests | Data | Setup |
  | ------ | ---- | ----- | ---- | ----- |
  | `blank` | 211 | 219 | 55 | 592 |
  | `grid` | 889 | 659 | 86 | 673 |
  | `examples/hello-grid` | 3570 | 4036 | 285 | 136 |

  - `blank`: 211, as *Context* counted (about 210): `title.rs` 73,
    `lib.rs` 61, `build.rs` 45, `stages.rs` 22, `main.rs` 5,
    `examples/tools.rs` 5. The cap is 220, 211 rounded up.
  - `grid`: 889 against 916 for the old `templates/grid` (2026-10-20).
    2501's 521 was by 2511's rule, over fewer files.
  - `examples/hello-grid`: 3570 code and 4036 tests against 1871 and
    1799 on 2026-10-20. It grew over 48 commits since. Its biggest code
    files are `src/tools.rs` (579) and `examples/asset_memory.rs` (569).
  - Setup is large in the templates because each game has
    `release.yml` (252) and `export.yml` (179). It is reported only.
- **`--check` shown:** with `code: 220` it passes. With `code: 210`
  (under the count) it fails: "blank: 211 code lines, over the cap of
  210 in tools/line-budget.ron; its biggest code files:
  crates/game/src/title.rs (73), crates/game/src/lib.rs (61),
  crates/game/build.rs (45), crates/game/src/stages.rs (22),
  crates/game/examples/tools.rs (5). Don't raise the cap: a part every
  new game needs goes in the engine, where every game gets it
  (ADR-0036)". With `code: 230` it fails: "blank: 211 code lines, 19
  under the cap of 230: lower `code` to 220 in tools/line-budget.ron".
- **Deviations:**
  - The engine's skill pack in a game (`.claude/` and `.agents/`) is
    left out, with the builds. Its worked examples are `.rs` files
    (`.claude/skills/*/example/*.rs`, 646 lines in a `blank` game), so
    by the ticket's rule they counted as code, at 857 lines. They are
    the engine's, and the pack's updater replaces them, so they aren't
    the author's.
  - Data counts a target's own data folder (`files@playdate/`) as well
    as `files/`.
  - A code file's `#[cfg(test)]` lines show in `--json` as the file's
    `test_lines`, beside its `lines` of code.
  - `tools/sound-parity/footprint.ron` names the xtask's `description`
    by its text, for the sound give-up: its `Replace` now holds the new
    words, so `sound-parity give-up` still finds them once.
- **From 2511, the `ActionSet` derive (ADR-0036 §9):** neither built-in
  template has its own enum; both use `UiAction`, so a new game pays
  nothing. Only Hello, Grid writes one: `src/action.rs`, 61 code lines,
  27 of tests, kept in the long form on purpose (ADR-0036 §10). No
  derive ticket, since no new game pays for an enum.
- **The parked examples (2735 to 2739):** none is done; each one's
  README clause now names this command's code count.
- **Decision 3528:** its list of guards no longer offers 2512, which is
  done, and its *Context* says the cap is in CI.
- **Docs:** `docs/contract.md`, *The templates*, says what counts, the
  cap and the command; `run-gates` and `write-ticket` have their lines.
  `rust_lines` is a public module documented in its own doc comments; no
  book page covers the lint's scanner.

Book: none, `shuyi_cli::rust_lines` is the scanner behind `check-keys`, `check-text` and the repository's line count, which no Learn chapter covers.

No break: `rust_lines` is new, and the items it holds were `pub(crate)`
in `check_keys`.
