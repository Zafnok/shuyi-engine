---
id: "2829"
title: "What changed since your version: a reader who knew an earlier Shuyi sees each page's changes since then, from the tickets' migration blocks and the passages' version marks"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2826", "2005"]
nick_input: none
completed:
---

# 2829 — What changed since your version

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md), *one
book, read at the reader's level*): "someone that used Shuyi in v1.0 […]
and now wants to check out Shuyi in v12.0 can see how it's changed". It
has nothing to show until there are two releases, so it comes after the
MVP; 2005 makes the first.

What there is:

- **The migration blocks** (`tickets/README.md`, *Breaking changes*): a
  ticket that breaks the public API writes a `Breaking:` line and a
  `migration` block (`since`, `what`, `old`, `new`, `before`, `after`,
  `find`) in its completion notes, and `cargo xtask changelog`
  (`crates/xtask/src/changelog.rs`, 2804) makes each release's migration
  guide from them.
- **The `Book:` rule** (2815): CI's `api` job fails a change to the public
  API that changes no file of the book and has no `Book:` line.
- **Released versions keep a frozen copy** of the site, which `llms.txt`
  lists (`crates/xtask/src/llms.rs`, *Released versions*).
- **From 2826 (done):** the reader's filter, the popup on a first visit
  and the bar (`site/README.md`, *The reader's filter*). "Have you made a
  game before?", answered "Yes, with Shuyi", is how a reader says they
  know Shuyi.
  - The popup's and the bar's HTML is `filter()` in
    `site/templates/book-macros.html`; the questions' logic is `answer()`
    and `finish()` in `site/static/book-filter.js`.
  - `read()` there keeps only `knows` and `engine`, and `write()` writes
    `v`, `knows` and `engine` back. A `"version"` field must go through
    both, or a bar toggle drops it.
- **From 2823 (done):** [ADR-0090](../../docs/adr/0090-one-book-the-project-folded-into-the-chapters-a-newcomer-mark-around-a-passage-and-a-readers-filter-kept-in-the-browser.md) §7 sketches the version marks.
  - A passage is wrapped in `{% since(version="0.4.0") %}` …
    `{% end %}`. The version is as Cargo writes it, and the compare
    link goes to its `x.y` frozen copy.
  - A `since` may hold `newcomer` marks, and none of its own kind.
  - It is block-level, as the `newcomer` mark is.
  - A whole page new in a version has `[extra] since = "0.4.0"`.
  - A migration block's optional `book` field is an `@/` path with an
    optional anchor.
  - The reader's version is a `"version"` field added to the filter's
    `localStorage` entry, `shuyi-book` (§6).

**From 2825 (done):** `cargo xtask check-book` (`crates/xtask/src/book.rs`)
reads `newcomer` marks only, takes every `{% end %}` as a mark's close, and
fails marks that nest. A `since` holds `newcomer` marks (ADR-0090 §7), so
the parser there learns a `since` around marks, with tests, and
`llms.rs`'s `twin` (which splits a page with `book::parts`) writes its
twin.

## Nick input

None.

## Scope

**In:**
- **A migration block names its page:** an optional `book` field, the
  book page (and anchor) the change touches, as an `@/` path. `changelog`
  checks the page exists. The `Book:` rule's check accepts a migration
  block with `book` as the book's change.
- **A passage says when it arrived or changed:** the ADR's version mark
  (for example `since="0.4.0"`), checked by the build checks against the
  released versions, and written by the PR that adds or changes the
  passage after the first release (the `document` skill says so).
- **The page data:** `cargo xtask changelog` (or a sibling command)
  writes, for each book page, the migration blocks that name it with
  their versions, for the site's build (`site/data/`, never committed).
- **The reader's version:** the bar and the popup take "the version
  I knew", from the released versions. Each book page then shows a
  **Changed since X** box at its top, listing each migration block that
  names it from after X (its `what`, its `before` and `after`), and marks
  the passages whose version is after X. **Only what changed** folds the
  rest, with 2826's stubs.
- **Compare:** each page links the same page in the frozen copy of the
  version the reader knew, when it existed then.
- **The twins:** each page's twin lists its changes by version, so an
  agent upgrading a game reads them too.

**Out (do not do):**
- writing version marks on passages before the first release: everything
  then is "since the first release";
- a diff of a page's text between versions, beyond the link to the old
  copy;
- changes to the migration guide's own format beyond `book`.

## Implementation steps

1. Read ADR-0090, 2826's completion notes, `changelog.rs` and its
   migration parser, the `Book:` check, and `llms.rs`'s *Released
   versions*.
2. Add `book` to the migration block, with its checks and tests, and the
   page data.
3. Write the version mark's check, and add the reader's version to the
   bar, the popup and the script.
4. Write the *Changed since X* box, the passage marks and the compare
   link, and the twins' change lists.
5. Try it on a built site with two fixture releases (the tests' tags or a
   scratch clone), in the built-in browser.
6. Run the `document` skill, then the gates.

## Acceptance criteria

- [ ] A migration block with a `book` field naming a missing page fails
      `changelog`.
- [ ] A reader who knew version X sees, on each page a later migration
      block names, that block's change, and no box on a page nothing
      changed.
- [ ] Passages marked after X are marked as changed, and *Only what
      changed* folds the rest.
- [ ] A page links its copy in X's frozen site when it existed then.
- [ ] A twin lists its page's changes by version.
- [ ] The book's chapter is updated (2802): this ticket is that change.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`changelog.rs`): `book` parsed, checked, and grouped by page
  across fixture releases.
- Unit (`llms.rs`): a twin's change list.
- The version mark's check: a fixture page with a mark of an unreleased
  or malformed version.
- By hand: step 5, in the notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
