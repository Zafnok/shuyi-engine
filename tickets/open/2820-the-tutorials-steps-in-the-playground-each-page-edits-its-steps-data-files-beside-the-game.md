---
id: "2820"
title: "The tutorial's steps in the playground: each step's page edits that step's data files beside its game, and each before-and-after can be tried there"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: ["2814", "2818", "3069"]
nick_input: none
completed:
---

# 2820 — The tutorial's steps in the playground

## Context

Nick, 2026-11-01 ([`docs/decisions.md`](../../docs/decisions.md), *the
tutorial and the book teach from nothing*) asked that the playground wire
to the tutorial. 2814 builds the playground on Hello, Grid alone, and its
Out leaves "other examples" to a later ticket: this is that ticket for the
tutorial. 3069 gives a web build the files its page holds.

2818 gave each step's page tested before-and-after pictures, from
`site/tutorial/tries.toml`. Many of them edit a data file: a color in
`palette.ron`, a line of `en.ron`, a level's `.map`. Those a reader can
then try on the page, with no install.

This edits data files, not Rust. Hot reload (2905) doesn't change that: it
rebuilds on the developer's own machine, and a browser has no compiler.
Rust in the page is [2916](2916-parked-research-editing-a-games-rust-code-in-a-web-page.md)'s.

## Nick input

None.

## Scope

**In:**
- A page's playground for each step that has data files worth changing,
  reusing 2814's template and `playground.js`, with the step's web build
  at `play/` in the frame: the step's `palette.ron`, `en.ron` and, from
  step 2, its levels, from `site/tutorial/<step>/crates/game/files/`.
- A step's web build gets 3069's file layer, as Hello, Grid's does.
- **An "Try this here" button on a `try_it`** whose edits are all in files
  under `files/`: it opens the page's playground with the edit made, so
  the reader sees the "after" live. A `try_it` that edits Rust keeps its
  picture and says it needs a rebuild on the reader's own machine.
- `site/README.md` and the step pages' wording on it.

**Out (do not do):**
- editing or compiling Rust in the browser (2916);
- changing 2814's Hello, Grid page;
- sharing edits by link, or saving them past the tab (2814's Out).

## Implementation steps

1. Read 2814's and 3069's completion notes, `site/templates/tutorial-step.html`,
   the `try_it` shortcode and `tries.toml`.
2. Give the steps' web builds the file layer, and copy each step's files
   for the page as 2814's `copy-playground.sh` does.
3. Add the playground to the step template, and the button to `try_it`.
4. Try each step in the built-in browser.
5. Run the `document` skill, then the gates.

## Acceptance criteria

- [ ] On each step's page with data files, a reader changes a file, runs
      it, and sees the game change, with its problems shown when broken.
- [ ] A `try_it` of a data file offers its edit in the playground, and
      running it gives what its "after" picture shows.
- [ ] A `try_it` of Rust says it needs a rebuild, and offers no button.
- [ ] The page works at 375 px with no sideways scroll, and loads nothing
      from another host.
- [ ] The book's chapter is updated (2802), or the completion notes have a
      `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Site: the site's CI job checks each step's copied files equal its own.
- By hand in the browser, the criteria above, listed in the notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
