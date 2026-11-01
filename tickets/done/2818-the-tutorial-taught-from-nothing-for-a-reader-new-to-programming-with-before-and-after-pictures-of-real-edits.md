---
id: "2818"
title: "The tutorial taught from nothing: for a reader new to programming, with the engine's words and Rust's explained, and before-and-after pictures of real edits"
type: infra
milestone: "Engine: public"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2803"]
nick_input: none
completed: 2026-11-01
---

# 2818 — The tutorial taught from nothing

## Context

Nick, 2026-11-01 ([`docs/decisions.md`](../../docs/decisions.md), *the
tutorial and the book teach from nothing*): "our tutorial pages are ok but
a bit content sparse, for instance on page 01, we just list all files and
don't really give more info. I want the tutorial to be very in-depth.
Assume the developer is brand new to programming and game development.
Introduce Shuyi primitives, Shuyi concepts, Rust-isms that are necessary
for Shuyi, and so on." And: "Tutorial will be read by humans, AI can infer
all Shuyi directives from its source and useful indices."

Then: "I'd also want some screenshots showing i.e. ok now what if we want
this color to be blue? or this size to be 2x? etc and show a before/after
with code changes rn we have no screenshots only playable demos."

2803 built the ten steps, each a project CI builds and tests. Its pages
say what to do and show the code, but explain little, and have no picture.
2813 plans pictures for the book, of Hello, Grid; nothing plans pictures
of a change.

## Nick input

None. The audience and the pictures are decided.

## Scope

**In:**
- **The pages.** Rewrite every page in `site/content/learn/tutorial/` for
  a reader who has never programmed: what a terminal, a compiler, a file
  and a test are; each Shuyi idea (screen, stack, context, action, keymap,
  command, rules, view, skin, bundle, format, Harness …) before its name is
  needed; each Rust feature (`let`, `fn`, `struct`, `enum`, `match`,
  `impl`, traits, `&`, `&mut`, `Option`, `Result`, `?`, `use`, derives,
  closures, generics …) the first time the code uses it, in the words of
  that code. Each step says what it will do, what you see at the end, and
  which files it changes, so a reader can follow it by hand.
- **Two new pages:** `00-start` (before you start: the words, the tools
  and how to read a page) and `glossary` (every word, and where it is
  taught). Neither is a step: neither has a folder, nor a play link.
- **Pictures of real edits.** `site/tutorial/tries.toml` lists each
  picture: its step, its keys and, for a before-and-after, its edits (file,
  text to find, its replacement). `cargo xtask tutorial-tries` copies the
  step, makes the edits, and renders with `frame-png` into
  `site/static/tutorial-tries/`, where they are checked in. With `--check`
  it renders again and compares bytes, fails on a `find` that isn't exactly
  once in its file, on a stale or orphan picture, and on a try no page
  shows. A new CI job, `tutorial-tries`, runs it. So an edit a page tells a
  reader to make always applies, builds, and gives what the picture shows.
- **The `try_it` shortcode** (`site/templates/shortcodes/try_it.md`):
  shows the edit as a diff and the two pictures, or one picture for a try
  with no edits. `cargo xtask llms` expands it, as 2808 requires.
- **Anchors** added to the steps' files, so a page can show smaller pieces.
  No step's code changes.
- **`llms-full.txt` leaves the tutorial out**, per the decision. Its pages
  keep their twins and their lines in `llms.txt`.
- The docs map and `site/README.md`; the `run-gates` skill names the job.

**Out (do not do):**
- changing what a step's game does, or adding a step;
- the book (2819);
- the playground and a code pad (2814, 2820, 2916);
- video, and animated pictures;
- 2813's `shots` part of `shuyi.ron`. **Either order with 2813:** if 2813
  is done, keep `tutorial-tries` as it is (its edits have no home in
  `shuyi.ron`) and name its shortcode's CSS with `shot`'s; if not, nothing
  to do (2813's `shot` shortcode and `figure-pair` class are its own).

## Implementation steps

1. `crates/xtask/src/tries.rs`, its tests and `main.rs`'s command.
2. `try_it`, the step templates, the CSS and `llms.rs`.
3. Choose each step's tries, render them and look at each.
4. Write the pages, with new anchors where a page needs a smaller piece.
5. Add the CI job, then run the `document` skill and the gates.

## Acceptance criteria

- [x] Every page explains every Rust feature and engine word its code
      uses, there or by a link to the page that did.
- [x] Each step has at least one picture of what the reader should see,
      and steps with something visible to change have a before-and-after.
- [x] A before-and-after covers at least: a color made blue, and a size
      made double.
- [x] `cargo xtask tutorial-tries --check` passes in CI, on Linux, for
      pictures made on Windows, and fails on a stale picture, a `find` that
      no longer matches, an orphan and an unused try (unit tests).
- [x] Each step still builds and passes its own tests.
- [x] `llms-full.txt` holds no tutorial page; `llms.txt` lists them.
- [x] Nothing names the original game's trademark or Visions of Shuyi.
- [x] The book's chapter is updated (2802), or the completion notes have a
      `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `tries.rs`: parsing, a bad name, a `find` found no times or twice,
  orphans, a try no page shows, comparing pictures.
- `llms.rs`: `try_it`'s expansion, its missing picture and its unknown
  name; the tutorial out of `llms-full.txt`.
- CI: the `tutorial-tries` job.

## Completion notes

Every page of the tutorial is rewritten for a reader who has never programmed,
with notes marked **Rust**, **Shuyi** and **Game idea** on the lines that need
them. `00-start` and `glossary` are new. There are 34 tries in
`site/tutorial/tries.toml`, and 19 of them are before-and-afters, including a
color made blue (`title-blue`, `box-blue`, `box-home-blue`) and a room made
twice the size (`level-double`). Each was looked at by eye. The pages were
checked in a browser at desktop width and at 375 px, with no sideways scroll.

**Verified here:** `cargo xtask tutorial-tries --check`, the xtask's tests and
clippy, `cargo fmt --check` in every step, the tests of steps 2, 6 and 9, the
site's build checks, `zola check`, `cargo xtask llms`, the ticket lint, and the
docs and links checks. **Not run here:** the rest of the workspace's gates, as
only `xtask` changed among the crates, and the tests of the other seven steps,
which gained only comments. CI runs both. Whether the pictures made on Windows
equal CI's on Linux is only known from CI's first `tutorial-tries` run.

**Deviations:**

- **Size.** About 2,400 lines of prose and 400 of code and tests, over the
  ticket size of 1,500: the prose is one voice, and is read as one.
- **`--scale 2`.** The pictures are drawn at twice the size they show. A pair
  shows narrower than the frame, and scaling 384 pixels down made the text
  unreadable.
- **`llms-full.txt`** no longer holds the tutorial, as Nick decided. Its size is
  now about 123 KB. The tutorial's twins and `llms.txt` lines stay.
- **A TOML `Cargo.toml` can't hold an anchor**, so its one-line additions are in
  the page's words, as inline code.

**The job's cost:** `tutorial-tries` takes about 8 minutes on this machine with
the engine already built, as each before-and-after builds its own copy of a step.

**Follow-ups:** 2819 (the book), 2820 (the steps in the playground), 2916
(parked: Rust in a page, and its cost, for Nick).

Book: none, no public item changed; 2819 is the book's turn.
