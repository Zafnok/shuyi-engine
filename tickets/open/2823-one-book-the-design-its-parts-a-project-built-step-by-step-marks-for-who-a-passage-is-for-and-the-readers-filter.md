---
id: "2823"
title: "One book, the design: its parts, the tutorial as its first project, marks for who a passage is for, and the reader's filter, in an ADR"
type: infra
milestone: "Engine: public"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2817"]
nick_input: sign-off
completed:
---

# 2823 — One book, the design

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md), *one
book, read at the reader's level*): the book and the tutorial fold into
one book that holds everything. The tutorial becomes the book's project, a
whole game built step by step, as programming books carry project
walkthroughs (the Rust book's guessing game, then its later projects).
Every passage says who it is for, so a reader new to programming, a reader
new to Rust and a developer who only wants to see how Shuyi differs each
read the same book at their own level. A reader who knew an earlier Shuyi
sees what changed since. And it is interactive. A first-time visitor gets
a short welcome page with a skip, and a bar at the top of every page
changes the answers later. All of it but the version view comes before the
engine goes public.

What there is today (`site/README.md`, *What is where*):

- **The book**, `site/content/learn/book/`: a quick start and ten concept
  chapters (2802, taught from nothing by 2819), one page a chapter in
  `weight` order, each numbered by `extra.chapter`, appendices without one
  (*Coming from another engine*, *Fast compiles*). Their code is
  `site/snippets/` (Moon Garden), shown with `{{ snippet() }}`.
- **The tutorial**, `site/content/learn/tutorial/`: *Before you start*
  (`00-start`), ten steps of a box-pushing puzzle (2803, 2818), each
  numbered by `extra.step`, and a `glossary`. Each step's code is a whole
  project in `site/tutorial/<step>/`, built and tested by CI's `tutorial`
  job; its web build is at the page's `play/` (`pages.yml`), booted by
  `web-render`.
- **Pictures** of both are tries in `site/tutorial/tries.toml`, shown with
  `{{ try_it() }}` (2813, 2818).
- **The pages already mark their readers by hand**, as blockquotes:
  `> **Rust:**` (69 of them), `> **Shuyi:**` (21) and `> **Game idea:**`
  (10), mostly in the tutorial.
- **The twins and `llms.txt`** (2808, `crates/xtask/src/llms.rs`): a
  Markdown twin of each Learn page, and `llms-full.txt` of every twin but
  the tutorial's (decision of 2026-11-01: the tutorial is written for
  people).
- **Released versions keep a frozen copy** of the site, which `llms.txt`
  lists (`llms.rs`, *Released versions*).
- **Done:** 2817 played each step from its page; its evidence rows are
  in its completion notes.
- **Open tickets on these pages:** 2814 (the playground page), 2820 (the steps in the playground),
  2822 (`tutorial-tries` on Windows) and 2004 (the guides the book
  publishes).

This ticket decides the shapes, so that 2824–2829 are built to one plan.

Nick, 2026-11-05 (`docs/decisions.md`, *the first release waits for
everything before going public*): the tutorial is finished before it is
redesigned, so 2817, which plays every step in the browser at its
current address, comes first. The exercises' design moved to 2828,
which comes after the playground (2820) they are done in.

## Nick input

**Sign-off:** read the book's new table of contents and the welcome
page's three questions, written out in the ADR, and say whether a reader
new to programming, a Rust programmer and someone coming from another
engine would each know where to start.

## Scope

**In:** an ADR (the `write-adr` skill) that decides:

1. **The parts and their order.** For example: *Start here* (the welcome
   page, *Before you start*, the quick start), *Project: a box-pushing
   puzzle* (the ten steps), *The ideas* (the concept chapters, on Moon
   Garden), *Appendices* (coming from another engine, fast compiles, the
   one glossary, the problem codes). How pages are numbered across parts
   (a step keeps a step number inside its project). That the book can take
   more projects later, each a part of its own, and what a second project
   adds (its folder of steps, its tries, its CI).
2. **The Zola layout:** the folders under `site/content/learn/book/`,
   which become sections, their templates (whether `tutorial.html` and
   `tutorial-step.html` merge into `book.html` and `chapter.html` or stay
   as a project's), the page-to-page links (previous, next, the part), and
   whether `site/tutorial/` (the steps' code) keeps its path. Keeping it is
   the default: `tries.toml`, the `snippet` shortcode's `step`, three CI
   jobs and `pages.yml` read it.
3. **The old addresses:** every `/learn/tutorial/…` page keeps working as
   a redirect (Zola's `aliases`), and what that does to the twins and
   `llms.txt` (`llms.rs` refuses a page that sets `slug` or `path`).
4. **The audiences:** exactly three, named in the ADR (for example
   `programming` for programming and game making, `rust` and `shuyi`).
   An unmarked passage is for everyone. What each existing callout becomes
   (`> **Game idea:**` → `programming`, `> **Rust:**` → `rust`,
   `> **Shuyi:**` → `shuyi`), and how a passage that a Shuyi veteran
   needs differs from one that only teaches a newcomer the engine.
5. **The mark's syntax:** a Zola body shortcode (a name and its
   arguments) around a passage, and a page-level front-matter key for a
   page that is all one audience (*Before you start* is all
   `programming`); whether marks nest; how one renders with no JavaScript
   (everything shown, each mark labelled: "New to Rust?"); what its twin
   says; and whether `llms-full.txt`, which leaves the project out today,
   keeps doing so or takes the project's unmarked and `shuyi` passages.
6. **The filter:** the three questions, the answers each takes, where
   they are kept (`localStorage`, a reader's own convenience; the page
   works when it throws), the welcome page with its skip, the bar on every
   page, what a hidden passage leaves behind (a one-line stub to show it,
   so nothing disappears silently), and what "skip ahead" means: the
   first page, and the first passage of a page, that the reader's answers
   don't hide.
7. **The version marks**, sketched for 2829: how a passage says the
   version it arrived or changed in, and how a ticket's `migration` block
   (`tickets/README.md`, *Breaking changes*) names the book's page it
   changes.
8. **The quick checks**, sketched for 2827: a question, its answers and
   why, in the page. Only where an exercise sits in a page is named (beside
   the step's quick check); its design is 2828's.
- **The table of contents** and **the welcome page's text**, written out
  in the ADR, for Nick's sign-off.
- **Each later ticket checked against the ADR:** 2824–2829, and the open
  tickets above. Where the ADR changes what one of them says, change the
  ticket in this PR.

**Out (do not do):**
- moving or changing any page (2824), marking passages (2825), any
  script or template (2826–2828);
- the code pad (2916);
- the exercises' design: where a check lives and how the game reports it
  (2828, once the playground exists);
- the guides (2004): the ADR says only where in the book they go.

## Implementation steps

1. Read `site/README.md`, `site/templates/` (`book.html`, `chapter.html`,
   `tutorial.html`, `tutorial-step.html`, `base.html`, the shortcodes),
   `site/check-build.sh`, `crates/xtask/src/llms.rs`, `pages.yml` and the
   CI jobs that read `site/`, 2802, 2803, 2808, 2818 and 2819, and the
   open tickets in *Context*.
2. Read how two or three books do it: the Rust book (a project between
   concept chapters), Rust by Example's settings, and a site with a
   reader's level (MDN's *Beginner* vs reference, for example). Write
   what each does in a line of the ADR.
3. Try the Zola parts that carry risk on a scratch copy of the site:
   a nested section with its own template, a body shortcode around
   Markdown with a `snippet` inside it, `aliases` on a moved page, and
   `cargo xtask llms` over the result.
4. Write the ADR, the table of contents and the welcome page's text.
5. Update 2824–2829 and the open tickets where the ADR differs from them.
6. Ask Nick for his sign-off, record it in the completion notes, and run
   the gates.

## Acceptance criteria

- [ ] The ADR decides points 1–8 above, each with what was tried in step 3
      where it applies.
- [ ] The table of contents names every existing book and tutorial page
      in its new place, and nothing is left out.
- [ ] Every old `/learn/tutorial/…` address has a planned new address.
- [ ] 2824–2829 match the ADR, and the open tickets it changes are
      updated in this PR.
- [ ] Nick's sign-off is in the completion notes.
- [ ] The book's chapter is updated (2802), or the completion notes have a
      `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None: no code changes. The scratch trials of step 3 are described in
  the ADR, and the tickets that build each part carry its tests.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
