---
id: "2825"
title: "Every passage says who it is for: new to programming, to making games, to Rust or to Shuyi, marked on every page of the book and checked in CI"
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

**From 2823 (done):** [ADR-0090](../../docs/adr/0090-one-book-the-project-folded-into-the-chapters-a-newcomer-mark-around-a-passage-and-a-readers-filter-kept-in-the-browser.md) §4 and §5 decide the mark:

- **The audiences:** exactly four, an unmarked passage being for
  everyone:
  - `programming` ("New to programming?"): what a program, a terminal,
    a file or a variable is;
  - `games` ("New to making games?"): how games are made whatever the
    engine, such as the game loop and frames, state, sprites, data kept
    apart from code, and ECS. Nick added it on 2026-11-08
    (`docs/decisions.md`, *the book's project folds into its chapters,
    and a fourth kind of reader*);
  - `rust` ("New to Rust?");
  - `shuyi` ("New to Shuyi?").
- **A `games` passage says how games are made, not how Shuyi makes
  them.** "A game draws a frame many times a second" is `games`; "a
  screen draws into the glyph buffer" is `shuyi` or unmarked.
- **A `shuyi` mark is narrow.** A passage about the engine that a reader
  who knows Shuyi still needs stays unmarked: a line to type, a rule
  the engine checks, a gotcha, or what an operation does that the page
  relies on. Only an introduction to an idea a veteran already has is
  `shuyi`.
- **The shortcode:** `` {% newcomer(to="rust", topic="`match`") %} `` …
  `{% end %}`.
  - `topic` is required: a few words, which name the stub 2826 leaves.
  - A mark holds whole blocks only (paragraphs, lists, code,
    `snippet`s, `try_it`s, headings), on its own lines.
  - Marks don't nest.
- **Its HTML:** `<aside class="newcomer" data-newcomer="…">`, then a
  `newcomer-label` paragraph with the label and the topic. The topic goes
  through `markdown(inline=true)`.
- **A page that is all one audience:** `[extra] newcomer = "…"`. 2824 set
  it on *Before you start*, and this ticket makes its top line say so.
- **The twin:** a blockquote. Its first line is `` > **New to Rust?**
  `match` ``, and the passage follows with `> ` in front of each line.
- **`llms-full.txt`:**
  - it leaves out the `programming`, `games` and `rust` passages, and
    the pages marked whole for any of them;
  - it keeps the `shuyi` passages;
  - it still leaves out the project's pages;
  - its first lines say what it leaves out.

The ADR's trial ran the body shortcode around a `snippet` on Zola 0.21.
2824 put every page in the one book.

The pages mark some passages already, by hand, as blockquotes:
`> **Rust:**`, `> **Shuyi:**` and `> **Game idea:**` (104 on
2026-11-08: 75, 21 and 8, most of them in the project's steps). They
become `rust`, `shuyi` and `games`. Many more explain a word in a sentence of
an ordinary paragraph ("A **crate** is Rust's word for one package of
code"), and those are unmarked.

## Nick input

None.

## Scope

**In:**
- **The mark:** the ADR's shortcode, `site/templates/shortcodes/newcomer.md`,
  and its look in `site/static/style.css`: a label naming the audience
  and the topic, and a rule at the passage's side, light and dark,
  readable at 375 px. The page-level key also goes in `chapter.html`'s
  top line. With no JavaScript every passage shows, labelled.
- **The twins:** its expansion in `crates/xtask/src/llms.rs` (a shortcode
  added to Learn needs one, `site/README.md`, *The docs for agents*), and
  `llms-full.txt` as the ADR says.
- **The check:** a page that uses a mark with an audience not among the
  ADR's four, an unclosed mark, a mark inside a mark, or a mark with no
  `topic`, fails the site's build checks (`check-build.sh`, or a `cargo xtask`
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
- ***Before you start*'s legend** of the three kinds of note becomes a
  few lines on the four marks: their labels, and that the bar folds them
  (2826).
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

1. Read ADR-0090, 2824's completion notes, `site/README.md`, the
   shortcodes and `llms.rs`'s shortcode expansions.
2. Write the shortcode, its styles, its expansion in `llms.rs` and the
   check, with their tests.
3. Mark the pages in the book's order: *Before you start*, then each
   chapter with the steps after it, then the appendices. For each page,
   read it once with each audience left out, and fix what no longer
   reads.
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
- [ ] For three pages (a chapter, a step and an appendix), the completion
      notes quote what a Rust programmer from another engine would read
      with the `programming`, `games` and `rust` passages left out, and it
      reads as whole sentences.
- [ ] The book's chapter is updated (2802): this ticket is that change.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`llms.rs`): the mark's expansion in a twin, and in
  `llms-full.txt`: a `programming` or `rust` passage left out, and a
  `shuyi` one kept.
- The check: a fixture page for each failure above, and the site passing.
- By hand: step 4, and step 3's read-throughs, in the notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
