---
id: "2852"
title: "cargo xtask check-book fails a term the book defines that has no glossary row"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] `site/README.md` says how a term is marked.
- [ ] `cargo xtask check-book` fails a marked term with no glossary
      row, and passes the book as it is after this ticket.
- [ ] The Markdown twins show a term as text (if a shortcode).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a page with a term in the glossary, one not, a plural, and an
  emphasis that isn't a term.

## Completion notes

*(Filled in by the session that completes the ticket.)*
