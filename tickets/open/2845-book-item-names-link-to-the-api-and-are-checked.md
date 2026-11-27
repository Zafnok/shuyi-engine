---
id: "2845"
title: Item names in the book link to the API, and the build fails on an unknown name
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2845 — Item names in the book link to the API, and the build fails on an unknown name

## Context

The book (`site/content/learn/`, ticket 2802) names the engine's types and
functions in backticks all through its prose: `Screen::update`,
`TurnRules`, `Tween::new`, `shuyi::ui::PlatformFacts`. None of them is a
link. `grep -rn '/api/' site/content/learn` finds nothing, though `cargo
doc` puts every engine crate at `/api/` (pages.yml, the crates of
`site/data/api.toml`). A reader who wants the reference has to search
for it.

Nothing checks those names either. 2815 (done) made `cargo xtask api`
ask for a change to the book, or a `Book:` line, when the public API
changes. It doesn't check that what the prose names exists. The drift
was already there. Appendix A's table
(`site/content/learn/book/from-another-engine.md`) had the row, until
3413 drew the table from data and fixed it,
`| none yet | a tween | a coroutine | — | an animation curve | — |`,
but `shuyi_core::Tween` is public (`crates/core/src/ease.rs`,
re-exported at `crates/core/src/lib.rs:72`, ticket 2109). Chapter 2
(`frame-and-skins.md:203`) teaches it. The page was last changed on
2026-11-09, after the tween landed.

The counts, today: Learn has 247 distinct backticked names that start
with a capital (`Screen`, `UiAction::Up`, `Option`, `FixedUpdate`), and 44
distinct backticked `a::b` paths. Many aren't the engine's: Rust's own
(`Option`, `Vec`, `Self`), other crates' (`Serialize`, `ron::from_str`),
other engines' in appendix A (`FixedUpdate`, `NextState`, `PlayerPrefs`),
the book's own game (`Puzzle`, `Garden`, `stages::Own`) and problem
codes (`SH2001`). So the check can't fail every name that isn't in the
API. It needs a list of the names that aren't the engine's, and that
list is checked too.

3413 (done) built an index of the public items from rustdoc's JSON,
`crates/xtask/src/rustdoc_index.rs`, for its `engine-terms.toml`. This
ticket reads the same index (below). The `api` job already installs the nightly
that rustdoc's JSON needs (`api::NIGHTLY`, `nightly-2026-08-16`). It
only runs when the crates change, though (`changes.yml`'s `crates`), and
a PR that only edits the book must be checked as well. So the check gets
its own job.

## Nick input

None.

## Scope

**In:**
- **The index of public items.** For each crate of `site/data/api.toml`,
  rustdoc's JSON on `api::NIGHTLY`, built with the features `/api/` is
  built with (pages.yml's `cargo doc`: the default features). It reads
  each item's path, kind and members: methods, required trait methods,
  variants, fields, associated constants and types. It follows `pub use`
  (globs too) across the crates, so `shuyi::ui::Screen` and
  `shuyi::prelude::UiAction::Up` reach the item where it is defined. An
  item's link is its page under `/api/<crate>/`, by rustdoc's file names
  (`struct.Tween.html`, `<module>/index.html`) and anchors
  (`#method.new`, `#tymethod.update`, `#variant.Up`, `#structfield.x`,
  `#associatedconstant.X`, `#associatedtype.X`).
- **What counts as a name.** Inline code in a Learn page's Markdown
  (`pulldown-cmark`, already a dependency of `xtask`), not in a code
  block and not already a link's text, whose whole text is one of these:
  - a name starting with a capital that holds a lower-case letter
    (`Screen`; all-caps text such as `PATH`, `VERSION` or `SH2001` is
    skipped);
  - such a name and a member (`Screen::update`, `UiAction::Up`);
  - a path (`shuyi::ui::widgets`, `ron::from_str`).

  Each may end in `()` or one `<…>` (`Tween<i32>`), which is dropped
  before the name is resolved. Anything else (`update`, `ctx.game`,
  `cargo shuyi web`) is not a name.
- **How a name resolves:**
  - a path starting with an engine crate (`shuyi`, `shuyi_*`) must reach
    an item of the index, or the check fails;
  - a bare name, or a name and a member, links when exactly one item of
    the index has that name. If no item has it, it must be in the list
    of other names below, or the check fails. If two different items
    have it (a re-export counts as the same item), the check fails and
    asks for a path, unless the list names this page for it;
  - an engine type with a member it doesn't have (`Tween::start` when
    there is no such method) fails;
  - a path that starts with something else (`std::env::var`,
    `ron::to_string`, `stages::own`) must have its first part in the
    list.
- **The list of other names**, `site/data/book-names.toml`: tables
  `rust` (the language and `std`), `crates` (the outside crates' names
  and their crate names), `engines` (other engines' names, for appendix
  A), and `[[own]]` entries for the book's own game, each with the
  `pages` it is used on. The check fails on an entry that no page uses,
  on an `[[own]]` name used on a page it doesn't list, and on an entry
  in `rust`, `crates` or `engines` that is also an engine item. Such a
  name is written as a path, or as an `[[own]]` entry with its pages.
- **`cargo xtask book-api --check`** (`crates/xtask/src/book_api.rs`)
  reads Learn and the index, and prints each failure as
  `<page>:<line>: <name>: <why>`, with what to do (write a path, add the
  name to the list, or fix the name). It writes nothing.
- **`cargo xtask book-api --public <dir> --base-url <url>`** runs after
  the API docs are copied into the built site (pages.yml). In each Learn
  page's HTML, it wraps each `<code>` that resolved, outside a `<pre>`
  and an `<a>`, in a link to its page. It fails when a link it writes
  points at a file that isn't in `<dir>/api/`. That catches a wrong file
  name rule before readers find it.
- **CI:** a new job `book-api` in `ci.yml`. It runs on every event, like
  `site`, installs the nightly as the `api` job does, and runs
  `cargo xtask book-api --check`. It goes in `ci-result`'s `needs` and
  in the gates table (`crates/xtask/src/gates.rs`, beside `api`), as
  `gate("book-api", "book-api", Filter::Always).ci_only("needs a
  nightly")`.
- **pages.yml:** the nightly is installed, and `cargo xtask book-api
  --public site/public --base-url "$BASE_URL"` runs after the step that
  copies the API docs in and before `cargo xtask llms`.
- **The book made true:** every unknown or drifted name the first run
  finds is fixed in the page, or listed in `book-names.toml` if it isn't
  the engine's. The completion notes list the drifts that were fixed.
- **The docs:** `site/README.md` gets a short section, *Names of the
  API*: a backticked engine name links itself, how to write a name that
  is two items, and when a name goes in `book-names.toml`. The docs
  map's row for the book (`docs/README.md`) says what now checks the
  book's names. The `run-gates` skill's table gets the `book-api` row.
- **Public types** (ADR-0039): none. All of this is in `xtask`, which
  isn't published.

**From 3413 (done):** read the index with `rustdoc_index::build`
(`crates/xtask/src/rustdoc_index.rs`), giving it the features `/api/` is
built with (pages.yml's `cargo doc`, the default features: no feature
argument); `Index::get` finds an item at any public path. The tween row
is fixed in `site/data/engine-terms.toml`, and `cargo xtask api-text
--check` checks the "none yet" rows' tickets.

**Out (do not do):**
- Links in the Markdown twins and `llms.txt` (2808). Agents read the
  API from 3413's plain-text listing.
- Links from the problem codes (`SH2001`) to their pages.
- Checking names in code blocks or in snippets. The snippets compile
  in CI already (2802).
- Making the book's own game's names from its code (`site/snippets/`,
  `site/tutorial/`). They are listed by hand, with their pages.
- Checking what the prose says about an item. This only checks that the
  item exists and links it.
- Moving the table of appendix A into data, and doc aliases: 3413 (done).
- Search over the site, and the docs and API search in the MCP server:
  not this ticket.

**Estimated size:** about 1,400 lines (code 600, tests 450, data 250,
docs 100), counted from the steps and doubled. 3413 built the index,
so about 250 of them are written.

**Split:** past 1,000 lines, split off step 5 (the links in the built
pages and pages.yml) into a new ticket, and keep the check here.

## Implementation steps

1. Read `crates/xtask/src/api.rs` (`NIGHTLY`, how it installs and runs
   the nightly, `listing`'s features), `crates/xtask/src/llms.rs` (how it
   walks Learn with `pulldown-cmark` and skips drafts and shortcodes),
   `crates/xtask/src/book.rs` (the Learn checks' shape and messages),
   `site/data/api.toml`, the `docs`, `api`, `site` and `ci-result` jobs
   of `ci.yml`, and `pages.yml`.
2. **The index**, as the clause above says, with tests over a small
   fixture crate's JSON made by the pinned nightly: an item, a
   re-export, a glob, a crate re-exported under a short name (as the
   `shuyi` facade does), a trait's required and provided methods, an
   enum's variants and a struct's fields.
3. **The scanner and the resolver** in `book_api.rs`: the names of a
   page with their lines, and each name's link or failure, as the Scope
   says. The resolver takes the index and the list as values, so its
   tests need no nightly.
4. **`book-names.toml`** from the first run over Learn, and the book's
   drifts fixed in the pages (the tween row as the clause says).
5. **The links:** the `--public` mode, its check that each link's file
   exists, and its step in `pages.yml`.
6. **The `book-api` job**, its `ci-result` need, its gates row and the
   `run-gates` table row. The command's lines go in `main.rs`'s usage and
   `match`.
7. **Docs** (the `document` skill): `site/README.md`, the docs map's
   book row.
8. Run the gates, and after the merge, open a chapter of the live site
   and follow two links into `/api/`.

## Acceptance criteria

- [ ] `cargo xtask book-api --check` passes on the book, with
      `book-names.toml` written, and fails on each case of the fixture
      pages: an engine path that reaches nothing, a bare name in neither
      the index nor the list, a name of two items with no path, an
      engine type with a member it lacks, an unused list entry, an
      `[[own]]` name on a page it doesn't list, and a list entry that is
      also an engine item. Each failure names the page, the line and the
      name.
- [ ] `shuyi::ui::PlatformFacts`, `shuyi::prelude::UiAction::Up`,
      `Screen::update` and `Tween<i32>` resolve to the page and anchor
      where each is defined (unit tests over the fixture index).
- [ ] Appendix A no longer says Shuyi has no tween, and the completion
      notes list every drift the first run found.
- [ ] In the built site, a chapter's `Tween::new` is a link to
      `/api/shuyi_core/ease/struct.Tween.html#method.new`, a name in a
      code block is not a link, and `--public` fails on a link whose
      file isn't in `api/` (a test over a fixture site).
- [ ] The `book-api` job runs on a PR that changes only a Learn page,
      and it is in `ci-result`'s `needs` and in the gates table.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (the index): each item shape of step 2, over the fixture JSON.
- Unit (`book_api.rs`): what is a name and what isn't, the `()` and
  `<…>` endings, each way of resolving and each failure of the Scope,
  and the messages.
- Integration: `--public` over a fixture site with a small `api/`
  folder: links written outside `<pre>` and `<a>` only, and the missing
  file failure.
- CI: the `book-api` job over the real book.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
