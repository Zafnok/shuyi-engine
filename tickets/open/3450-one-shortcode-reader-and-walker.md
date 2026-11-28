---
id: "3450"
title: "One Learn-page shortcode reader for the docs corpus and cargo xtask llms, and one file walker and copy_tree over shuyi_embed::walk"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3450 — One Learn-page shortcode reader, and one file walker and `copy_tree`

## Context

The redundancy sweep of 2026-11-28 found two parsers of the same page
markup, and hand-written folder walks:

- **Two shortcode readers.** The `docs` operation's corpus
  (`crates/cli/src/contract/docs/corpus.rs`, about 119–415) and `cargo
  xtask llms` (`crates/xtask/src/llms.rs`, about 548–635 and 836–1167)
  read the Learn pages' shortcodes separately:
  - `call` (corpus.rs:282 and llms.rs:908) is the same scan. The corpus
    returns an `Option` and llms a `Result` with messages.
  - `arguments` is the same, with stricter errors in llms.
  - `anchored` is the same, apart from llms trimming `\r`.
  - `snippet` and `shot` read the same keys.
  - `corpus.rs` re-declares `Term` and `Terms`, and hand-writes the table
    header that `crates/xtask/src/api_text.rs`'s `table` writes.

  About 150 lines go from `corpus.rs`. The two `clean` and `expand`
  loops differ in behaviour (`{% end %}`, the newcomer, check and
  exercise blocks, `try_it`), so they stay.
- **Hand-written walkers.** `markdown_under` is byte-identical in
  `crates/xtask/src/book.rs:278` and `llms.rs:460`. About 25 recursive
  `read_dir` walks of 11–18 lines each are in xtask and cli
  (`imports.rs`, `tries.rs`, `catalog.rs`, the `reveal` and `llms`
  tests, and others). There are two `copy_tree` helpers
  (`crates/cli/src/web.rs` about 610, and `tries.rs` about 547).
  `shuyi_embed::walk` (`crates/embed/src/lib.rs` about 117–175) already
  walks, sorts and checks. About 200 lines go.

## Nick input

None.

## Scope

**In:**
- **A no-I/O `shuyi_cli::shortcodes` module** with the strict `call`,
  `arguments` and `anchored`/`whole` (trimming `\r`), the `snippet` and
  `shot` argument structs, and the engine-terms table writer (moved from
  `api_text`).
  - llms and the book use it.
  - The corpus uses it too, treats an `Err` from `call` as plain text,
    and keeps its own wording ("snippet: no `file`", "no anchor z").
  - It must not pull `pulldown-cmark` into the shipped `shuyi` binary.
- **`shuyi_cli::fs_tree`** with `files_under(dir, ext, skip)` over
  `shuyi_embed::walk` and one `copy_tree`. The simple walkers in xtask
  and cli call them.
- **Behaviour:** the corpus's output for every page stays byte-equal.
  `embed::walk` errors on a non-UTF-8 name and on two paths that differ
  only in case, and it sorts by `/`-path. A walker converted to it
  takes those rules: say so in a `Behaviour:` line if any shipped
  command's output order changes.
- **Either order with 2845 and 3434:** if they are done, move what they
  added to either reader into `shortcodes`. If not, nothing to do
  (they extend `shortcodes`, as their clauses say).

**Out (do not do):**
- Walkers that skip unreadable entries silently or record metadata
  (`imports.rs`'s flatten): they keep their own walks.
- Merging the two `clean` and `expand` loops.

**Estimated size:** about 900 lines changed (about 600 removed, 300
added).

## Implementation steps

1. Move llms' `call`, `arguments`, `anchored` and the argument structs
   into `crates/cli/src/shortcodes.rs` with their tests. llms calls
   them.
2. Point `corpus.rs` at them. Delete its `call`, `arguments`,
   `is_mark`, `anchored`, `Term`/`Terms` and `engine_terms`. Its tests
   pass unchanged.
3. Add `fs_tree`. Replace `markdown_under` and the simple walkers, and
   merge the copy helpers.

## Acceptance criteria

- [ ] `grep -n "fn call\b\|fn arguments\b" crates/cli/src/contract/docs crates/xtask/src`
      finds no definition outside `shortcodes`.
- [ ] `shuyi docs`' corpus is byte-equal before and after, for every
      Learn page (a test that compares it with a stored copy, or the
      existing corpus tests).
- [ ] `cargo xtask llms --check` and the book build pass with the same
      output.
- [ ] `cargo tree -p shuyi --edges normal` lists no `pulldown-cmark`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `shortcodes` (a good call, a missing closer, a bad key, a CRLF
  anchor), `files_under` (an extension filter and a skip), and
  `copy_tree` into a scratch folder.
- The corpus and llms tests are the regression suite.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
