---
id: "2827"
title: "Quick checks in the book: a question, its answers and why, at the end of each step and chapter, answered in the page with no server"
type: feature
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2826"]
nick_input: none
completed:
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

2823's ADR sketches the check's shape. 2825 marked every passage with its
audience, and 2826's filter hides what a reader knows; a check carries an
audience too, so the filter hides a check on Rust from a reader who knows
Rust.

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
- **The checks themselves:** two or three at the end of each project step
  and each concept chapter, on what that page taught, each answer's reason
  pointing back at the passage that explains it. A check of code asks what
  a line does, never to recall a name.

**Out (do not do):**
- exercises done in the playground (2828);
- a score, progress kept between visits, or an account;
- checks on the appendices.

## Implementation steps

1. Read 2823's ADR, 2825's mark and its `llms.rs` expansion, 2826's
   script, and the pages.
2. Write the shortcode, its styles, its script (beside 2826's
   `book-filter.js`), its expansion and the check of the checks.
3. Write the checks, a page at a time.
4. Try three pages in the built-in browser: a right and a wrong answer,
   the keyboard alone, JavaScript off, 375 px.
5. Run `cargo xtask llms`, the `document` skill, then the gates.

## Acceptance criteria

- [ ] Every project step and concept chapter ends with two or three
      checks.
- [ ] Choosing an answer says whether it is right and why, and with no
      JavaScript the answers are in a `<details>`.
- [ ] A check carries an audience, and 2826's filter hides it, with its
      stub, when the reader knows that audience.
- [ ] A malformed check fails the build checks.
- [ ] The twin of a page shows its checks as text.
- [ ] The book's chapter is updated (2802): this ticket is that change.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`llms.rs`): the check's expansion.
- The check of the checks: a fixture page for each failure above.
- By hand: step 4, in the notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
