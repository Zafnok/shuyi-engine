---
id: "2567"
title: "check-skills reads links with a Markdown parser, as check-links does"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2567 — `check-skills` reads links with a Markdown parser, as `check-links` does

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-28). Two checks find the links in Markdown, each its own way:

- **`shuyi_cli::skills::links_in`** (`crates/cli/src/skills.rs:625`):
  `shuyi check-skills` reads each skill file's links with it, and
  `link_problem` (`skills.rs:656`) says what is wrong with each target (a
  file not in the pack, a link that leaves the pack, a fixed page on the
  engine's `main`). It scans the prose of each line (from `split_code`,
  `skills.rs:535`) for `](` and takes everything up to the next `)`
  (`skills.rs:627`–`637`). That misreads three kinds of link:
  - a link with a title, `[a](b.md "A title")`: the target read is
    `b.md "A title"`, and `check-skills` reports "no such file in the
    skill pack" for a file that is there;
  - a target with parentheses, `[a](notes(1).md)`: the target read is
    `notes(1`, the same wrong report;
  - a reference link, `[a][r]` with `[r]: ../../docs/x.md` on a line of
    its own: never read, so a link that leaves the pack passes, though a
    game gets only the pack (1134).
  A second loop (`skills.rs:638`–`650`) adds every URL of the engine's
  repository on a line, in code too; it is right as it is.
- **`cargo xtask check-links`' `relative_links`**
  (`crates/xtask/src/links.rs:79`): reads the same links with
  `pulldown-cmark` (`Parser::new(text).into_offset_iter()`, every
  `Tag::Link` and `Tag::Image`, the line counted from the event's offset),
  so titles, parentheses, angle brackets and reference links are read as
  Markdown reads them.

`pulldown-cmark` is already in `Cargo.lock` (0.13.4, through
`crates/xtask/Cargo.toml:18`, with `default-features = false`), and its
own dependencies there are `bitflags`, `memchr` and `unicase`. Its
license is `MIT` (its `Cargo.toml`), which ADR-0002 §8 allows and
`deny.toml`'s `allow` list has. Its default features are `getopts` (the
binary's options) and `html` (the renderer, `pulldown-cmark-escape`);
neither is needed. `tools/layers.ron` gives `shuyi-cli` `outside: Any`,
so no line there. A game's `tools` feature builds `shuyi-cli`
(`templates/shuyi-default/base/crates/game/Cargo.toml`), so the parser
compiles into each game's tool build: CI's `compile-times` job measures
that.

## Nick input

None.

## Scope

**In:**
- `crates/cli/Cargo.toml` depends on `pulldown-cmark = { version =
  "0.13", default-features = false }`, the same requirement as xtask's,
  so `Cargo.lock` keeps one copy, with a comment line saying what it is
  for (`check-skills`' links).
- `links_in`'s first loop reads the links of the whole text with
  `pulldown_cmark::Parser`: the `dest_url` of every `Tag::Link` and
  `Tag::Image` (inline, reference and autolinks), trimmed, with its line
  counted from the event's offset as xtask does. Links in code are not
  links, as before.
- The second loop (the engine's URLs anywhere on a line) and its rule of
  not adding a URL already found on that line stay as they are.
- `links_in`'s doc comment says it reads the links as Markdown does.
- `split_code` stays: `commands_in` still uses it.
- **For the game:** no API change (`links_in`, `split_code` and
  `link_problem` are private). `check-skills` stops reporting a link
  with a title or with parentheses that is fine, and starts reporting a
  reference link that leaves the pack or names a missing file; a game
  that checks its own skills can see the second kind.

**Out (do not do):**
- `crates/xtask/src/links.rs`: it checks the repository's files on
  disk, and percent-decodes targets; `shuyi-cli` doesn't depend on
  xtask, and the two calls are a few lines each. Leave it.
- Percent-decoding targets or checking `#anchors` in `link_problem`.
- Links written in HTML (`<a href>`), which neither check reads.
- Moving `commands_in` or `split_code` onto the parser.

## Implementation steps

1. Add the dependency to `crates/cli/Cargo.toml` (above), and check
   `cargo tree -p shuyi-cli -e normal -i pulldown-cmark` shows 0.13.4
   with no `getopts` and no `pulldown-cmark-escape`.
2. In `crates/cli/src/skills.rs`, replace the `split_code(text).1` loop
   at the top of `links_in` with a loop over
   `Parser::new(text).into_offset_iter()` matching
   `Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. })`,
   pushing `(text[..range.start].matches('\n').count() + 1,
   dest_url.trim().to_owned())`. Keep the second loop.
3. Update `links_in`'s doc comment.
4. Add the tests below in `crates/cli/src/skills/tests.rs`.
5. Run `cargo shuyi check-skills skills --tickets tickets/open`; fix any
   link in `skills/` it now reports.
6. Run the gates.

## Acceptance criteria

- [ ] `grep -nF 'find("](")' crates/cli/src/skills.rs` finds nothing.
- [ ] `cargo tree -p shuyi-cli -e normal -i pulldown-cmark` lists one
      `pulldown-cmark` 0.13.4, and `cargo tree -p shuyi-cli -e normal`
      has no `getopts` and no `pulldown-cmark-escape`.
- [ ] The tests below pass, and every existing test in
      `crates/cli/src/skills/tests.rs` and `crates/cli/tests/it/skills.rs`
      passes unchanged.
- [ ] All gates in the `run-gates` skill pass, `cargo deny check` and
      `cargo xtask compile-times --budget` among them.

## Tests required

- Unit (`crates/cli/src/skills/tests.rs`, with its `good`, `pack` and
  `found` helpers): a skill whose body links an existing
  `references/a.md` as `[t](references/a.md "A title")` and an existing
  `references/b(1).md` as `[p](references/b(1).md)` reports nothing; a
  reference link `[r][out]` with `[out]: ../../docs/contract.md` reports
  "the link leaves the skill pack" on the line of `[r][out]`; a link in a
  code span or a fenced block is still not read.
- The existing `a_link_must_name_a_file_of_the_pack_and_never_main` and
  `a_code_span_that_wraps_is_still_code_and_a_lone_backtick_ends_with_its_paragraph`
  pass unchanged.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
