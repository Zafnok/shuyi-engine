---
id: "2819"
title: "The book taught from nothing: each chapter for a reader new to programming, with the engine's words and Rust's explained, and before-and-after pictures"
type: infra
milestone: "Engine: public"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2818"]
nick_input: none
completed:
---

# 2819 — The book taught from nothing

## Context

Nick, 2026-11-01 ([`docs/decisions.md`](../../docs/decisions.md), *the
tutorial and the book teach from nothing*): the tutorial's pages were too
sparse, and "same notes for the book, but we can file that as a later
ticket". 2818 rewrote the tutorial for a reader brand new to programming
and to making games, and built the pictures: `site/tutorial/tries.toml`,
`cargo xtask tutorial-tries` and the `try_it` shortcode.

The book (`site/content/learn/book/`, 2802) still reads like a reference:
each chapter states what a concept is and shows its code, and assumes the
reader knows Rust and knows what a frame, a state or a trait is.

## Nick input

None. The audience is decided.

## Scope

**In:**
- Rewrite each chapter of the book for the tutorial's reader: say what
  each idea is for before its name, explain each Rust feature the first
  time a chapter's code uses it (or link the tutorial's explanation and
  its [glossary](../../site/content/learn/tutorial/glossary.md)), and say
  what can go wrong and what the error looks like.
- A chapter keeps its job: one concept, explained on its own, on Moon
  Garden. The tutorial stays the one that builds a game in order.
- Pictures: every chapter whose idea shows on screen has at least one
  what-you-see picture, and a before-and-after where changing something
  teaches it. Moon Garden lives in `site/snippets/`, not `site/tutorial/`.
  Either extend `tutorial-tries` to read a project under `site/snippets/`,
  or show the tutorial's game. Choose one, and say why in the notes.
- **Either order with 2813:** if 2813 is done, its pictures of Hello, Grid
  stay where they are and this ticket adds to them; if not, nothing to do
  (2813 adds its shots beside these, as its text says).

**Out (do not do):**
- changing what the engine does, or a public item;
- the tutorial's pages (2818);
- the playground (2814, 2820);
- pictures of a target's own display, which are 2813's `playdate` shot.

## Implementation steps

1. Read the tutorial's pages and `glossary.md`, so a chapter uses the same
   words for the same things.
2. Rewrite one chapter at a time, in the book's order, with each chapter's
   code still from `site/snippets/`.
3. Add the pictures, and look at each.
4. Check `cargo xtask llms`: the book stays in `llms-full.txt`, under its
   budget. Say in the notes if the budget had to rise, and why.
5. Run the `document` skill, then the gates.

## Acceptance criteria

- [ ] Each chapter explains every Rust feature and engine word its code
      uses, in the chapter or by a link to where the tutorial does.
- [ ] Each chapter whose idea shows on screen has a picture, each made by
      the tools and checked in CI.
- [ ] `cargo xtask llms` passes under its budget.
- [ ] The book's chapter is updated (2802), or the completion notes have a
      `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `book-snippets` and the site's build checks, as they stand.
- `cargo xtask tutorial-tries --check` over any new picture.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
