---
id: "2824"
title: "One book: the tutorial's pages become the book's project, with one glossary, one Learn way in, and the old addresses sending readers on"
type: feature
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: ["2823"]
nick_input: none
completed:
---

# 2824 — One book: the tutorial becomes its project

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md), *one
book, read at the reader's level*): "the Book will contain everything. The
Tutorial can be an example game or project". 2823's ADR decides the parts,
the Zola layout, the templates and the new address of every page. This
ticket moves the pages there. It changes no page's teaching: marking who
a passage is for is 2825's, and the filter is 2826's.

Today Learn has two ways in (`site/content/learn/_index.md`): the book
(`site/content/learn/book/`) and the tutorial
(`site/content/learn/tutorial/`), each with its own landing page, its own
templates and its own glossary-like words. The book's landing page sends a
new programmer to the tutorial's *Before you start*. The tutorial's code
stays in `site/tutorial/<step>/` unless the ADR moved it.

## Nick input

None.

## Scope

**In:**
- **The pages move** to the ADR's places: *Before you start*, the ten
  steps and the glossary into the book, as `git mv`, so their history
  follows. Each gets the front matter its new section needs, and an
  `aliases` entry for its old address.
- **One glossary** for the whole book: the tutorial's `glossary.md` takes
  the words the book's chapters explain, and each chapter's link to a word
  goes there.
- **The landing pages:** the book's `_index.md` says what the book holds,
  its parts, and where each reader starts (2826 turns that into the
  welcome page). Learn's `_index.md` has one way in, the book, beside the
  API docs and the problem codes. The tutorial's `_index.md` goes, its text
  moved into the project part's own landing page.
- **The templates** as the ADR says: the project's steps keep their
  `play/` link and their *step N of 10* place, and previous and next run
  through the whole book.
- **Every link** to a moved page (`@/learn/tutorial/…`) in `site/content/`,
  the templates, `site/data/features.toml`, `site/README.md` and the rest
  of the repository's Markdown, found with `cargo xtask check-links` and
  `zola check`. Open tickets that name a moved page's path in backticks
  (2112, 2225, 2528 and 2777 on 2026-11-03; grep `tickets/open` again)
  get the new path in this PR.
- **What reads the old paths:** `pages.yml` (the steps' web builds copied
  to each step page's `play/`), CI's `web-render` job, `check-build.sh`
  (its rule that a tutorial page has only `bash` code blocks follows the
  project's pages), and `crates/xtask/src/llms.rs` (its rule leaving the
  tutorial out of `llms-full.txt` follows the project, as the ADR says),
  with their tests.
- **`site/README.md`**: *What is where* and *The rules* for one book, and
  the `document` skill for the docs map.

**Out (do not do):**
- changing what a page teaches, beyond the links and the sentences that
  name "the tutorial" or "the book" as two things;
- marking passages by audience (2825), the welcome page and the bar
  (2826), quick checks (2827), exercises (2828);
- moving `site/tutorial/` or renaming its step folders, unless the ADR
  says to;
- the guides (2004).

**Either order with 2814:** if 2814 is done, the playground keeps its own
page, and the new Learn landing page and the book's landing page link it;
if not, nothing to do (2814 links it from the landing pages as this ticket
leaves them).

**Either order with 2820:** if 2820 is done, the steps' playground moves
with their pages and template, and its copied files keep their paths; if
not, nothing to do (2820 builds it on the steps' pages where this ticket
puts them).

**Either order with 2817:** if 2817 is done, nothing to do; if not, nothing
to do here either (2817 plays each step from its page in the book, and the
old addresses redirect there).

## Implementation steps

1. Read 2823's ADR and completion notes, `site/README.md`, the templates,
   `check-build.sh`, `llms.rs`, `pages.yml` and the CI jobs that read
   `site/`.
2. Move the pages with `git mv`, give each its front matter and
   `aliases`, and write the landing pages.
3. Merge or adapt the templates, and fix the links.
4. Change `pages.yml`, the CI jobs, `check-build.sh` and `llms.rs`, with
   their tests.
5. Build the site, serve it, and in the built-in browser read the
   contents, follow previous and next through all of it, open a step's
   `play/`, and open three old addresses.
6. Run `cargo xtask llms` on the built site, the `document` skill, then
   the gates.

## Acceptance criteria

- [ ] Every page the ADR's table of contents names is at its new address,
      and Learn has one way in, the book.
- [ ] Every old `/learn/tutorial/…` address reaches its page's new address.
- [ ] Previous and next run through the whole book in the ADR's order.
- [ ] Each step's page links its web build, and `web-render` boots each.
- [ ] One glossary; no page links the old one.
- [ ] `zola check`, `check-build.sh` and `cargo xtask check-links` pass,
      and `cargo xtask llms` writes a twin of every page, with
      `llms-full.txt` as the ADR says.
- [ ] `git log --follow` on a moved page shows its history.
- [ ] The book's chapter is updated (2802): this ticket is that change.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `llms.rs`: its unit tests updated for the project's new path, and one
  that an aliased page gets one twin, at its new address.
- Site: `check-build.sh` fails a non-`bash` code block on a project page
  (tried once by hand, said in the notes) and passes the site.
- By hand in the browser: step 5, listed in the notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
