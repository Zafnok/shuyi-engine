---
id: "2825"
title: "Every passage says who it is for: new to programming, new to Rust or new to Shuyi, marked on every page of the book and checked in CI"
type: feature
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: ["2823", "2824"]
nick_input: none
completed:
---

# 2825 — Every passage says who it is for

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md), *one
book, read at the reader's level*): the book filters by "familiarity with
general programming, familiarity with Rust, familiarity with Shuyi", so
that a reader new to programming, a reader new to Rust and an experienced
developer who wants "to see how Shuyi Engine differs" each read it at their
level. Before a page can be filtered (2826), each passage must say who it
is for.

2823's ADR names the three audiences, the shortcode that marks a passage,
the page-level front-matter key, how a mark renders with no JavaScript,
what its twin says, and what `llms-full.txt` does with marked passages.
2824 put every page in the one book.

The pages mark some passages already, by hand, as blockquotes:
`> **Rust:**`, `> **Shuyi:**` and `> **Game idea:**` (about 100, most of
them in the project's steps). Many more explain a word in a sentence of
an ordinary paragraph ("A **crate** is Rust's word for one package of
code"), and those are unmarked.

## Nick input

None.

## Scope

**In:**
- **The mark:** the ADR's shortcode in `site/templates/shortcodes/`, its
  look in `site/static/style.css` (a label naming the audience, light and
  dark, readable at 375 px), and the page-level key in the templates.
  With no JavaScript every passage shows, labelled.
- **The twins:** its expansion in `crates/xtask/src/llms.rs` (a shortcode
  added to Learn needs one, `site/README.md`, *The docs for agents*), and
  `llms-full.txt` as the ADR says.
- **The check:** a page that uses a mark with an audience the ADR doesn't
  name, an unclosed mark, or a mark inside a mark when the ADR forbids it,
  fails the site's build checks (`check-build.sh`, or a `cargo xtask`
  command if it reads the Markdown better). So does a hand-written callout
  left in the old form (`> **Rust:**`).
- **Every page marked:** each existing callout becomes the ADR's mark, and
  every passage that only a newcomer needs is marked: the sentence that
  explains a word of Rust, the paragraph that says what a frame is, the
  part of the quick start on what a terminal is. A passage that both
  teaches a newcomer and says something everyone needs is split, so the
  part everyone needs stays unmarked. Read each page with each audience
  hidden in mind: what is left must still read as whole sentences, and
  the code it shows must still be explained for that reader.
- **The glossary:** each word names its audience.
- **The rules** in `site/README.md`: how to mark a passage, what to leave
  unmarked, and that a new passage that teaches a newcomer is marked
  (the `document` skill).

**Out (do not do):**
- hiding anything: the filter is 2826's, and this ticket's pages show
  every passage;
- new teaching beyond the splits and the joins the marks need;
- the version marks (2829);
- quick checks and exercises (2827, 2828).

## Implementation steps

1. Read 2823's ADR, 2824's completion notes, `site/README.md`, the
   shortcodes and `llms.rs`'s shortcode expansions.
2. Write the shortcode, its styles, its expansion in `llms.rs` and the
   check, with their tests.
3. Mark the pages, a part at a time: *Start here*, the project, the ideas,
   the appendices. For each page, read it once with each audience left
   out, and fix what no longer reads.
4. Build the site and look at three pages in the built-in browser, light
   and dark, at 375 px.
5. Run `cargo xtask llms`, read a twin with marks, then the `document`
   skill and the gates.

## Acceptance criteria

- [ ] Every page of the book uses only the ADR's marks; no callout of the
      old form is left, and the check fails one put back.
- [ ] A page with a mark of an unknown audience, or an unclosed mark, fails
      the build checks.
- [ ] Each marked passage shows its label with no JavaScript, in light and
      dark.
- [ ] A twin shows each marked passage with its audience, and
      `llms-full.txt` is as the ADR says.
- [ ] For three pages, one from each part, the completion notes quote what
      a Rust programmer would read with the `programming` and `rust`
      passages left out, and it reads as whole sentences.
- [ ] The book's chapter is updated (2802): this ticket is that change.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`llms.rs`): the mark's expansion in a twin, and in
  `llms-full.txt`.
- The check: a fixture page for each failure above, and the site passing.
- By hand: step 4, and step 3's read-throughs, in the notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
