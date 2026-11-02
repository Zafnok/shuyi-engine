---
id: "2819"
title: "The book taught from nothing: each chapter for a reader new to programming, with the engine's words and Rust's explained, and before-and-after pictures"
type: infra
milestone: "Engine: public"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2818"]
nick_input: none
completed: 2026-11-01
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

- [x] Each chapter explains every Rust feature and engine word its code
      uses, in the chapter or by a link to where the tutorial does.
- [x] Each chapter whose idea shows on screen has a picture, each made by
      the tools and checked in CI.
- [x] `cargo xtask llms` passes under its budget.
- [x] The book's chapter is updated (2802), or the completion notes have a
      `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass (those this PR can move here, the rest in CI, as the notes say).

## Tests required

- `book-snippets` and the site's build checks, as they stand.
- `cargo xtask tutorial-tries --check` over any new picture.

## Completion notes

Every chapter is rewritten for the tutorial's reader: it says what its idea is
for before its name, explains the Rust in its code in notes marked **Rust**, or
links the tutorial, and has a section on what goes wrong. The two appendices
get the same treatment, less of it: their readers are narrower. The site's
README now says the rule.

**The pictures.** `tutorial-tries` now reads Moon Garden too, a try with
`step = "book"`, rather than showing the tutorial's game. The book's reader
reads about Moon Garden, and a picture of a box puzzle beside the garden's code
would teach less. It costs little: `book_manifest` makes a copy of
`site/snippets` a one-crate workspace of its own, and `site/snippets/examples/tools.rs`
draws its frames. There are 11 tries, 8 of them before-and-afters, each looked at.

Chapters 1 to 6 and 8 have pictures. **Chapters 7, 9 and 10 have none:** a save, a
target's check and the tool contract don't show on screen, and a target's own
display is 2813's.

**The errors are real.** Each message in *When it goes wrong* came from running
that mistake, on Moon Garden or a tutorial step. Some are cut to their first
lines. They are written by hand, so they can drift as messages improve.

**Found doing it:** `{Confirn}` in a language file passes `check` and shows on
screen, so 2575. A missing text key passes `check` too and panics in a debug build:
that's the design, and chapter 2 says so. A `blank` game's `lint` runs no text
check and no rules fence: chapters 2 and 6 say so.

**Verified here:** the xtask's and the book's tests, their clippy, `cargo fmt
--check`, `cargo xtask tutorial-tries --check`, `zola check`, `check-build.sh`,
`cargo xtask llms` (about 177 KB, under the budget of 400 KB, not raised), the
ticket lint, typos, the docs and links checks, and the pages in a browser at
desktop width and at 375 px. **Not run here:** the rest of the workspace's gates,
as only `xtask` and `book-snippets` changed among the crates. CI runs them.

**Deviations:**

- **Size.** About 1,600 lines of prose, over the ticket size of 1,500: one voice,
  read as one.
- **The glossary** gains no row, as the tutorial's pages are out of scope. A word
  only the book uses is explained where the book uses it.
- **The job's cost:** `tutorial-tries` builds a copy per before-and-after, so 8 more.

**Follow-ups:** 2575.

Book: this is the book. No public item changed.
