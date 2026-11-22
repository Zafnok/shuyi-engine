---
id: "3324"
title: "SHUYI_LOG: a filter for the log's sinks read from the environment on a desktop, by level, category and library"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["3041"]
nick_input: none
completed:
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

- [ ] `LogFilter::parse` takes `info`, `warn,audio=debug` and
      `gilrs=warn`, and refuses `loud` and `audio=loud` with a reason
      (test).
- [ ] With `gilrs=info,gilrs_core=debug`, a `Debug` line of
      `gilrs_core::platform` passes and one of `gilrs::gamepad` does not
      (test: the longest prefix).
- [ ] A bad `SHUYI_LOG` gives the default filter and one `platform`
      warning naming the reason (test on `install`'s parsing part).
- [ ] With `SHUYI_LOG=debug`, a debug build of hello-grid writes its
      `debug` lines of the engine's categories to `latest.log` (the
      completion notes have the lines).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `LogFilter::parse`, `with_prefix`, `allows_target` with
  prefixes, `most_detailed` with prefixes.
- Integration: `crates/app/tests/log_sink.rs` with a filter given.
- Manual: hello-grid with `SHUYI_LOG=debug`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
