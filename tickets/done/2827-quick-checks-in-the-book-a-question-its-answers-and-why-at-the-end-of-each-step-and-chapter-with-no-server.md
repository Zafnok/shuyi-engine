---
id: "2827"
title: "Quick checks in the book: a question, its answers and why, at the end of each step and chapter, answered in the page with no server"
type: feature
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2826"]
nick_input: none
completed: 2026-11-11
---

# 2827 — Quick checks in the book

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md), *one
book, read at the reader's level*): the book is "a unified learning
resource and highly interactive". Its pictures (2813, 2818) and its
playground (2814, 2820) show a reader what the engine does. Nothing yet
asks the reader whether they followed. A quick check does: a short
question at the end of a step or a chapter, answered in the page, with
why each answer is right or wrong. The Rust book's quizzes and Exercism's
concept checks do the same.

**From 2823 (done):** [ADR-0090](../../docs/adr/0090-one-book-the-project-folded-into-the-chapters-a-newcomer-mark-around-a-passage-and-a-readers-filter-kept-in-the-browser.md) §8 sketches the check.

- **Where:** a step or a concept chapter ends with a `## Check yourself`
  section holding its checks. A step's exercises (2828) follow them there.
  A chapter with project steps after it ends with its checks, then the
  line naming its steps (ADR-0090 §2).
- **The shape:** a body shortcode, `{% check(to="rust") %}` …
  `{% end %}`. Its first paragraph is the question. Then comes a list,
  one item per answer: `- [x]` marks a right answer and `- [ ]` a wrong
  one, and ` — ` comes before why. `to` is optional.
- **Not yet tried:** parsing that list in Tera is this ticket's to try.
  If it can't be done cleanly, the checks go in a TOML file the
  shortcode names, as the tries do. The ADR lists that as the
  alternative, and this ticket's notes say which was chosen.

2825 marked every passage with its
audience, and 2826's filter hides what a reader knows; a check carries an
audience too, so the filter hides a check on Rust from a reader who knows
Rust.

**From 2825 (done):** `cargo xtask check-book` (`crates/xtask/src/book.rs`,
CI's `site` job) reads the body shortcodes: it takes every `{% end %}` as
the close of a `newcomer` mark and fails one with no mark open. A `check`
shortcode with a body adds its opener to `kind()` there, and its own
rules (no `newcomer` inside a `check`, say) to `parts()`, with tests.
`llms.rs`'s `twin` splits a page with `book::parts`, so a check's twin goes
in the same place.

**From 2826 (done):** the reader's filter, `site/static/book-filter.js`
(`site/README.md`, *The reader's filter*), leaves out whole every
`aside.newcomer[data-newcomer]` of an audience the reader knows, through
`style.css`'s `[data-book-fold~="…"]` rules, with nothing in its place
(ADR-0098). Every passage shown gets " · hide" on its `.newcomer-label`
paragraph, which folds it to a stub made from that paragraph. A check
behaves the same way if its HTML is such an aside with such a label.
Otherwise `stubs()`, the bar's count and the fold rules learn the check's
own element. The bar's count says "notes", so a check counted there may
want its own word.

## Nick input

None.

## Scope

**In:**
- **The check**, a shortcode in `site/templates/shortcodes/`: a question,
  two to four answers, which are right, and a line on each saying why.
  It takes an audience, as 2825's marks do. In the page, choosing an
  answer shows whether it is right and why; with no JavaScript, the
  answers show in a `<details>` the reader opens. Nothing is kept or sent
  anywhere.
- **Its look** in `style.css`, light and dark, at 375 px, with the
  keyboard (the answers are buttons or radio inputs, and the result is
  announced, `role="status"`).
- **The twin:** its expansion in `crates/xtask/src/llms.rs`: the question,
  the answers, and which is right and why, as text.
- **The check of the checks:** a check with no right answer, an answer
  with no reason, or an unknown audience fails the build checks.
- **The checks themselves:** two or three in the `## Check yourself`
  section at the end of each project step and each concept chapter, on
  what that page taught, each answer's reason
  pointing back at the passage that explains it. A check of code asks what
  a line does, never to recall a name.

**Out (do not do):**
- exercises done in the playground (2828);
- a score, progress kept between visits, or an account;
- checks on the appendices.

## Implementation steps

1. Read ADR-0090, 2825's mark and its `llms.rs` expansion, 2826's
   script, and the pages.
2. Write the shortcode, its styles, its script (beside 2826's
   `book-filter.js`), its expansion and the check of the checks.
3. Write the checks, a page at a time.
4. Try three pages in the built-in browser: a right and a wrong answer,
   the keyboard alone, JavaScript off, 375 px.
5. Run `cargo xtask llms`, the `document` skill, then the gates.

## Acceptance criteria

- [x] Every project step and concept chapter ends with two or three
      checks.
- [x] Choosing an answer says whether it is right and why, and with no
      JavaScript the answers are in a `<details>`.
- [x] A check carries an audience, and 2826's filter hides it, with its
      stub, when the reader knows that audience.
- [x] A malformed check fails the build checks.
- [x] The twin of a page shows its checks as text.
- [x] The book's chapter is updated (2802): this ticket is that change.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`llms.rs`): the check's expansion.
- The check of the checks: a fixture page for each failure above.
- By hand: step 4, in the notes.

## Completion notes

**Done.**

- **The check:** `site/templates/shortcodes/check.html`, the body
  shortcode of ADR-0090 §8. **The list parses in Tera**, so the checks
  stay in the pages and no TOML file was needed: the body splits at each
  `\n- [`, and each answer at its first ` — `. It fails the build itself on
  no question, fewer than two or more than four answers, an answer with
  no reason, no right answer, or an unknown `to`.
- **In the page:** the answers are buttons (`static/book-check.js`, loaded
  by `book::script()` with `defer`). Choosing one says "Right." or "Not
  quite." and why in the check's `role="status"` line. With no JavaScript
  the buttons stay hidden and the answers and why are in a `<details>`.
  Nothing is kept or sent.
- **The filter:** a check with `to` is an `aside.check.newcomer` with
  `data-newcomer` and a `.newcomer-label` ("New to Rust? a quick check"),
  so 2826's fold rules, stubs and hide work on it unchanged. The bar
  counts it as a check, not a note: "6 notes and 1 check hidden". A
  check with no `to` is for everyone and doesn't fold.
- **The build checks:** `cargo xtask check-book` (`crates/xtask/src/book.rs`)
  reads the `check` opener as a mark's (no nesting either way), and
  `read_check` fails what the shortcode fails, plus an answer that isn't
  one line or holds a link (it becomes a button), text after the answers,
  and a shortcode inside. It also fails a concept chapter (`[extra]
  chapter`) or a step (`[extra] step`) whose last `## ` section isn't `##
  Check yourself` holding two or three checks, a check outside it, and
  a check on any other page. `site/check-build.sh` fails a built page with
  a check that doesn't load `book-check.js`.
- **The twin:** `llms.rs` writes a check as `**Quick check.**`, the
  question, and each answer as `- Right: …` or `- Wrong: …` and why. One
  with `to` is a blockquote under its label, and `llms-full.txt` leaves out
  the `programming`, `games` and `rust` ones, as it does their passages.
- **The checks:** three in the `## Check yourself` section at the end of
  each of the 10 concept chapters and 10 steps, 60 in all. Each reason
  links the passage that explains it. Three carry `to="rust"` (steps 8 and 9,
  and Testing). *Before you start* is a whole page for one
  newcomer, not a concept chapter, and has none, as the appendices don't.
- **Docs:** `site/README.md`, a new section *Quick checks*, and the
  layout table.

**Tried by hand** (`zola serve`, the built-in browser):

- Step 5: a wrong answer shows "Not quite." and its reason with the
  answer's border in red; then the right one, "Right." in green, and the
  first answer's state cleared. Dark mode.
- Keyboard alone on step 5: Tab reaches each answer, Enter chooses it,
  `aria-pressed` follows, and the result is in the `role="status"` line.
- Testing: "New to Rust" off leaves its Rust check out whole, the bar
  says "6 notes and 1 check hidden"; on again, its "hide" folds it to
  the stub "New to Rust? a quick check · show", focus on the stub, and
  "show" opens it.
- Rules as commands at 375 px, light mode: no sideways scroll, answers
  full width at least 44 px tall. As served, with no JavaScript, the
  buttons are `hidden` and the `<details>` isn't; its list reads "Right:
  … — why" for each answer.

**Deviations:** none from the scope. The ADR's example writes "Yes:" or
"No:" at the start of why; the page adds "Right." or "Not quite." itself,
so a reason starts with the reason.

No break: `xtask` isn't published, and no public item changed.
