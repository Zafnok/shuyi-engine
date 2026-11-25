---
id: "2852"
title: "cargo xtask check-book fails a term the book defines that has no glossary row"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-25
---

# 2852 — `cargo xtask check-book` fails a term the book defines that has no glossary row

## Context

`site/README.md` says each word of Rust or of the engine is explained
the first time a page uses it, "and its row goes in the one glossary,
`book/glossary.md`". Nothing checks it. The docs audit of 2026-11-25
found the chapters had bolded new terms (**stream**, **sound effect**,
**template**) with no glossary row, and no commit in the last 60 had
touched the glossary. 2851 adds those rows. This ticket stops the next
one from slipping.

The pages use bold for two things. One is a term being defined
(`**seed**`, `**hash**`, `**stream**` in `rules-as-commands.md`). The
other is emphasis (`**only through commands**`, `**All or nothing.**`).
So the check needs a way to tell them apart.

## Nick input

None.

## Scope

**In:**

- **Mark a defined term.** Choose one, and write it in `site/README.md`
  beside the glossary rule:
  - a shortcode, `{{ term(word="stream") }}`, rendered as bold today
    (and expanded by `crates/xtask/src/llms.rs` for the Markdown twins,
    as the docs map's llms row asks of a new shortcode);
  - or bold kept for terms only, with emphasis written another way
    (italics) and the chapters changed to match.

  Pick by how many emphasis bolds the chapters have (count them) and
  say why in the completion notes.
- **The check**, in `cargo xtask check-book` (`crates/xtask/src/book.rs`):
  every marked term in a page of the book (chapters, appendices, the
  project's steps) is a word of a glossary row (case and plural aside;
  a row of "Update, draw" covers both). A term with no row fails,
  naming the page, the line and the term. Every row's links resolve (as
  `check-links` already checks).
- The chapters' terms marked as chosen; each one without a row gets a
  row or stops being marked a term (the session decides per word, and
  2851's rows count).

**Out (do not do):**

- writing the rows 2851 writes (**Either order with 2851:** if 2851 is
  done, its rows are there and this ticket only marks and checks; if
  not, write the rows the check asks for here, and 2851 writes none of
  them that exist);
- checking that a row's *New to* or its links are the right ones.

## Implementation steps

1. Read `crates/xtask/src/book.rs`, `site/README.md`'s glossary rule and
   the glossary's table.
2. Count the bolds in the book that are emphasis and those that are
   terms; choose the mark.
3. Write the check and its tests.
4. Mark the terms; add the missing rows.
5. `cargo xtask llms` if a shortcode was added; run the gates.

## Acceptance criteria

- [x] `site/README.md` says how a term is marked.
- [x] `cargo xtask check-book` fails a marked term with no glossary
      row, and passes the book as it is after this ticket.
- [x] The Markdown twins show a term as text (if a shortcode): no shortcode, so a term is bold in the twins as in the page.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a page with a term in the glossary, one not, a plural, and an
  emphasis that isn't a term.

## Completion notes

**The mark: bold is for terms, emphasis is in italics.** The book's pages
(chapters, appendices, *Before you start* and the project's steps; not the
front pages or the glossary) had 372 bolds. 70 end with `.`, `:`, `?` or
`!`: run-in headings (*When it goes wrong*'s `**A bad key.**`), which the
check leaves alone. Of the other 302, 61 were emphasis, a button's name
(**Ports**, **Key bindings**) or a list's label (`**`Cargo.toml`**`, the
gates in step 10), and about 241 were terms. A shortcode would have
changed the 241 and every term written from now on, and a term written
in plain bold would slip past it; italics changed the 61 and leave bold
as the one way to mark a term, so a new bold is checked by default. No
shortcode means nothing for `llms.rs` to expand.

- `site/README.md`, *The rules*: bold is a term, which has a glossary row;
  a bold ending in punctuation is a run-in heading; other stress is in
  italics. *The checks* says `check-book` checks it.
- `crates/xtask/src/book/terms.rs`: the bolds of a page, outside fences
  and code spans, a bold running over lines within a paragraph; the
  glossary's words, each row's first cell split at its commas; compared
  without backticks, case or a plural ending (`-s`, `-es`, `-ies`).
  `check_pages` runs it when the glossary is among the pages, so a term
  with no row fails `check-book` with its page, line and word. Unit tests
  in `book/terms/tests.rs` (a term with a row, one without, plurals, a
  run-in heading, italics, a bold over two lines, code skipped) and one
  through `check_pages` in `book/tests.rs`.
- Every row's links: `check-links` already checks the glossary's files,
  and `zola check` their anchors; no check was added for them. Each anchor
  this ticket links was checked against its page's headings.
- 137 bolds failed the first run. 2 were reworded (`its **key**` became
  `its **text key**`, step 1 and chapter 6, the row there); the 61 above
  went to italics, among them *Moon Garden*, *Shuyi Engine*, the other
  engines' *scene tree* and *entity system* (appendix A), the font
  *set* in chapter 3, *revision*, *dialogue preview* and the closed
  build's *code*; the rest got a row or a word in a row.
- Rows added: Game engine, Stream, Pixel, Sound effect, Dither, Test,
  Flake, Debug and release, Static site, Draft, Digest, Closed build, CI,
  Template, Tool program, Folder, Message, Save bundle, Reveal kit,
  Agent, Exit code, JSON Schema and JSON Pointer, Program and source
  code, Rust, Terminal, rust-analyzer, Container, Commit and repository,
  Data and code, Reverse-DNS, Comment, serde, JSON, Global, Debug info,
  Nightly. Words added to rows: deterministic, property, exporting, push
  and pop, console and glyph buffer, embedded, contract, compiling,
  function, call and return (for `fn`, calling, returning), module and
  public, variable, borrowed, loop, set, chaining, vector, guard,
  let-else, linking.
- **Deviation, order with 2851:** 2851 is not done, so this ticket wrote
  the rows its check asked for, among them 2851's **stream**, **sound
  effect** and **template**. 2851's other words (**pointer**, **graphics
  tier**, **cue**) aren't bolded in the book, so the check asks nothing
  of them: 2851's clause now says the rows above exist and that it bolds
  and writes those three if they are terms.

No break: only the xtask (not published) and the site changed.
Book: none, no public API changed; the book's pages changed only in
which words are bold.
Game: nothing to adopt, the check and the book are the engine's site.
