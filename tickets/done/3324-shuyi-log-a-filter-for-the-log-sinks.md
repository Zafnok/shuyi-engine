---
id: "3324"
title: "SHUYI_LOG: a filter for the log's sinks read from the environment on a desktop, by level, category and library"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["3041"]
nick_input: none
completed: 2026-11-22
---

# 3324 — SHUYI_LOG: a filter for the log's sinks

## Context

3041 (done) gave every game on `shuyi-app` its log's sinks
(`crates/app/src/log_sink.rs`, ADR-0030): a file on a desktop
(`<storage folder>/logs/latest.log`), stderr in a debug build, the
browser's console on the web, and a crash report's last lines. Which
lines pass is fixed there: `LogFilter::default()` in `shuyi_ui::log`,
`info` and more serious for the engine's categories and `warn` and more
serious for any other `log` target (`LogFilter::allows_target`, with the
other targets' level `LogFilter::with_others`).

3041's scope also had a `SHUYI_LOG` variable and a parser for it. Its PR
passed its **Size** rule (about 600 changed lines; it measured 1,361),
so that part moved here, as the rule says. A developer chasing a bug in
the audio or the controllers wants `debug` lines of that category, or a
library's (`gilrs`), without a rebuild.

## Nick input

None.

## Scope

**In:**
- **`LogFilter::parse(&str) -> Result<LogFilter, String>`** in
  `shuyi_ui::log` (pure, `no_std`):
  - items split by commas, spaces trimmed, empty items skipped; levels
    `error`, `warn`, `info` and `debug` (`LogLevel::from_name`);
  - a level alone sets the engine's categories' level (`LogFilter`'s
    `level`); `name=level` with a category's name sets that category
    (`LogFilter::with`); any other name sets the level of every other
    target that starts with it (`gilrs` covers `gilrs_core::…` too);
  - anything else is refused with a reason in English: `loud`,
    `audio=loud`, `=debug`;
  - the rest is `LogFilter::default()`'s.
- **Prefixes in `LogFilter`:** a list of `(String, LogLevel)` and
  `with_prefix(&str, LogLevel)` (the same prefix given twice keeps the
  last). `allows_target` reads a category's name as its category, a
  target with prefixes at the level of its longest, and any other at the
  other targets' level. `most_detailed` takes the prefixes' levels into
  account. `LogFilter` is not `#[non_exhaustive]` today and has private
  fields only, so a field is additive (ADR-0039).
- **`shuyi_app::log_sink::install`** takes the filter's text
  (`filter: Option<&str>`) and returns the `LogFilter` in use. A bad
  value is ignored: the default is used, and one `Warn` line, target
  `platform`, says why. `run_app`'s `install_log` (`crates/app/src/lib.rs`)
  reads `SHUYI_LOG` on a desktop (nothing on the web) and hands the
  filter in use to `App::set_log_filter`, so App's ring keeps the same
  lines.
- **stderr in a release build when `SHUYI_LOG` is set** (a developer at a
  terminal), beside the debug build's. A build without debug tools still
  passes warnings and errors alone, whatever the filter says
  (ADR-0094 §4: `kept_in_this_build` in `log_sink.rs`).
- **Docs:** `docs/platform-backends.md`'s *Logs* gets `SHUYI_LOG`'s
  syntax, with an example (`SHUYI_LOG=warn,audio=debug,macroquad=info`),
  in place of its paragraph on the fixed levels.

**Out (do not do):**
- a debug-menu row for the filter, and a filter in the web page's
  address (a ticket if a developer asks);
- the debug view's own filter (2375 shows what App's filter keeps).

**Estimated size:** about 400 lines (code 150, tests 200, docs 50).

## Implementation steps

1. Read `shuyi_ui::log` (`LogFilter`), `crates/app/src/log_sink.rs`
   (`install`, `Sink`) and `install_log` in `crates/app/src/lib.rs`.
2. `LogFilter::parse`, the prefixes and `with_prefix` in `shuyi-ui`, with
   their unit tests (`crates/ui/src/log/tests.rs`).
3. `install`'s `filter` argument, its returned filter and its warning;
   stderr when it is set; `install_log` reading `SHUYI_LOG` and calling
   `App::set_log_filter` after `App::new`. Update the call in
   `crates/app/examples/render_card.rs` and in
   `crates/app/tests/log_sink.rs` (the second install there can give a
   filter and check the one returned).
4. The docs. Run the gates.

## Acceptance criteria

- [x] `LogFilter::parse` takes `info`, `warn,audio=debug` and
      `gilrs=warn`, and refuses `loud` and `audio=loud` with a reason
      (test).
- [x] With `gilrs=info,gilrs_core=debug`, a `Debug` line of
      `gilrs_core::platform` passes and one of `gilrs::gamepad` does not
      (test: the longest prefix).
- [x] A bad `SHUYI_LOG` gives the default filter and one `platform`
      warning naming the reason (test on `install`'s parsing part).
- [x] With `SHUYI_LOG=debug`, a debug build of hello-grid writes its
      `debug` lines of the engine's categories to `latest.log` (the
      completion notes have the lines).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `LogFilter::parse`, `with_prefix`, `allows_target` with
  prefixes, `most_detailed` with prefixes.
- Integration: `crates/app/tests/log_sink.rs` with a filter given.
- Manual: hello-grid with `SHUYI_LOG=debug`.

## Completion notes

What was done:

- `shuyi_ui::log::LogFilter::parse` reads a filter from text, as the
  scope says, with reasons in English for `loud`, `audio=loud` and
  `=debug`. `LogFilter` gained a list of prefixes, `with_prefix` (the
  same prefix twice keeps the last), the longest prefix in
  `allows_target` (a category's name never goes to a prefix), and the
  prefixes in `most_detailed`.
- `shuyi_app::log_sink::install` takes `filter: Option<&str>` and returns
  the `LogFilter` in use; a bad one gives the default and one `WARN`
  line, target `platform`, written after the file opens so it lands in
  `latest.log` (`filter_of` is the tested parsing part). stderr is on in
  a release build when a filter is given. `log_sink::FILTER_VARIABLE` is
  `"SHUYI_LOG"`. `install_log` reads it on a desktop and `run` hands the
  filter to `App::set_log_filter` right after `App::new`.
- Docs: `docs/platform-backends.md`'s *Logs* has the syntax and the
  example in place of the fixed levels' paragraph, and the stderr bullet.

Manual check, hello-grid's debug build at `23ba0a6b` (the tip of `main`
then) with this branch's changes, `latest.log`'s first lines:

```text
SHUYI_LOG=debug
    0.000 f0 INFO  platform: Shuyi Engine 0.1.0, windows x86_64 debug, storage shuyi-hello-grid, game version 0.1.0
    0.015 f2 INFO  storage: smoke check passed
SHUYI_LOG=debug,gilrs=debug,macroquad=debug
    0.017 f2 INFO  storage: smoke check passed
    0.027 f2 DEBUG gilrs::gamepad: Loaded 858 mappings.
SHUYI_LOG=loud
    0.002 f0 WARN  platform: SHUYI_LOG ignored, the default filter used: `loud`: `loud` is not a level (error, warn, info or debug)
```

Deviation: the fourth criterion asks for hello-grid's `debug` lines of
the engine's categories. The engine writes none today (no
`LogLevel::Debug` line outside tests), so `SHUYI_LOG=debug` passes them
but there are none to show; the library's `debug` line above shows a
`debug` line reaching the file through the filter.

Breaking: shuyi-app: `log_sink::install`: it takes a fourth argument, the filter's text (`None` for the default), and returns the `LogFilter` in use

```migration
since: 0.1.0
what: log_sink::install takes the filter's text and returns the filter in use.
old: "pub fn install(storage_name: &str, game_version: &str, dir: Option<PathBuf>)"
new: "pub fn install(storage_name: &str, game_version: &str, dir: Option<PathBuf>, filter: Option<&str>) -> LogFilter"
before: |
  shuyi_app::log_sink::install("my-game", "1.0", dir);
after: |
  shuyi_app::log_sink::install("my-game", "1.0", dir, None);
find: ["install(\"my-game\", \"1.0\", dir);"]
```

Behaviour: shuyi-app: on a desktop `SHUYI_LOG` sets which lines reach the log's file, stderr and App's ring (`app.log()`); unset, nothing changes
Behaviour: shuyi-app: a release build on a desktop writes its log's lines to stderr too when `SHUYI_LOG` is set

Book: `site/content/learn/book/targets-and-export.md`'s paragraph on the
log names `SHUYI_LOG`.

Game: nothing to adopt, the game calls neither `log_sink::install` nor
`LogFilter` (`gh search code --repo Zafnok/visions-of-shuyi 'log_sink'`
and `'LogFilter'`: no hits).

Nick input: none.
