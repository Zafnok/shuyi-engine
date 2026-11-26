# ADR-0154: The docs search is BM25 over a corpus `shuyi` embeds, at the pinned revision

- **Status:** Accepted
- **Date:** 2026-11-26
- **Related tickets:** 3412 (this), 3434 (the MCP resources), 3413 (the
  API's listing, a source to join), 3414 (reading the corpus from disk in a debug build), 2808 (the
  Markdown twins), 2843 (search on the website), 2525 (the eval that
  measures it)
- **Builds on:** [ADR-0138](0138-a-games-files-embedded-as-a-table-of-our-own-shuyi-embed.md)
  (`shuyi_embed`), [ADR-0043](0043-the-skill-pack-skills-at-the-root-worked-examples-run-by-the-xtask-check-skills-in-the-cli-and-the-pack-folder-as-the-plugin.md)
  (the skill pack), [ADR-0039](0039-public-types-that-can-grow-non-exhaustive-config-and-views-with-constructors-plain-values-left-open.md)
  (public types), [ADR-0150](0150-the-contracts-version-rises-only-on-a-break.md)
  (the contract's version)

## Context

An agent with no training on Shuyi fails first at finding things: "how
do I play a sound", "what is the type for a sprite". The MCP server ran
and checked a game, but nothing in its session answered that: `explain`
covers problem codes, `modules` the catalog, and the Learn pages' twins
(2808) are on the website, so an agent had to leave the session and
fetch them over HTTP, at the site's revision rather than the engine the
game pins.

The answer has to be the same on every machine (an answer is compared
and tested, as every other operation's), cost nothing to run, and need
no network: a game's tools run offline and in CI.

## Decision

1. **An operation, `docs <query>`** (`Op::Docs`, the MCP tool `docs`),
   which needs no project. It answers ranked hits (kind, title, source
   path in the engine's repository, section anchor, the Learn page's twin
   URL, an integer score, an excerpt) and the rows of a "how do I" table
   whose task the query asks.
2. **The corpus is text `shuyi` embeds**, so it is the engine's revision
   the game pins: `crates/cli/build.rs` writes `shuyi_embed` tables of
   `site/content/learn`, `site/snippets`, the tutorial's steps' files a
   snippet shows (`site/tutorial`, its builds, art and fonts left out) and
   `examples` (its `.rs` and `.md` files); not the pages the site's build
   generates into `site/content/learn` (`book/guides/`, `errors/`, ignored
   by git), so the corpus doesn't depend on whether the site was built; the skill pack and the problem
   codes' pages were embedded already. Together about 1.45 MB of a binary
   that is no game's: a test holds the three planned tables under 1 MB and
   the tutorial's under 600 KB more.
3. **Chunks** (`contract::docs::corpus`): a Learn page or a skill's
   `SKILL.md` cut at its `##` and `###` headings outside fenced code, the
   anchor Zola's slug of the heading; a Learn page read as its twin reads
   it (`snippet` is its code, the bodies of `newcomer`, `check` and
   `exercise` kept, a `shot` as its picture's alt text and caption,
   `try_it` dropped); an example's Rust file as its `//!`
   comment and its public items' names; a problem code's page whole. An
   xtask test holds each Learn page's headings and snippet code equal to
   its twin's, so a new shortcode fails the twins and the search alike.
4. **Lexical BM25** (`contract::docs::index`; k1 = 1.2, b = 0.75), a
   chunk's title terms counted twice. Words are lowercased ASCII, split at
   anything but a letter, a digit or `_`; a `snake_case` or `CamelCase`
   word is kept whole and split, since an agent asks for `add_sprite` and
   for "add sprite"; eight stop words are dropped. The index is built on
   first use, once per process (`OnceLock`, about 0.2 s in a debug
   build). Scores are floats, as a tool's may be: no simulation reads them
   (ADR-0002's float ban is the rules'). They are answered in thousandths,
   rounded, ties broken by path then position; a test pins the scores of a
   fixed corpus, so a platform that rounds otherwise fails in CI.
5. **The "how do I" table** is data, `site/data/how-do-i.toml`: rows of
   task phrases and one target each (`learn/<path>#anchor`,
   `skill/<name>`, `example/<path>`). A test fails a row whose target the
   corpus doesn't hold.
6. **MCP resources** (3434): the server will declare `"resources": {}`
   and list each skill's `SKILL.md` (`shuyi://skills/<name>/SKILL.md`)
   and each Learn page (`shuyi://learn/<path>`, the text `docs` reads), as
   Markdown; an unknown URI is `-32002`. No subscriptions, no
   `listChanged`: the text changes only with the engine. A server
   following another workspace's server asks it first, so the resources
   are that engine's.
7. **How a source joins**: a table in `build.rs` if its files aren't
   embedded yet, a `HitKind` (`#[non_exhaustive]`, so adding one breaks no
   client), and a function in `corpus` that cuts it into chunks, added to
   `corpus::chunks`. The API's plain-text listing (3413) joins this way as
   `api`.

## Consequences

- An agent finds a page, a skill or a name from inside the session, for
  the engine its game pins, and the MCP server's instructions say to call
  `docs` before guessing a name.
- `shuyi` grows by the corpus, about 1.45 MB; a game ships none of it.
- An edited Learn page is found only after `shuyi` is rebuilt (`build.rs`
  watches the folders); 3414 reads the tables from disk in a debug build.
- A lexical search misses a synonym the docs never use. The "how do I"
  table answers the common tasks in an agent's words, and the eval (2525)
  measures what is left.

## Alternatives considered

- **Embeddings and a vector store**: a model to ship or a service to
  call, answers that differ by model version, and a network call;
  refused (the ticket's Scope).
- **Searching the website** (2843's job): the site's revision, not the
  game's pin, and a network call.
- **Serving `llms-full.txt` as one resource**: 300 KB an agent reads
  whole, with no ranking; the twins stay the site's.
- **A crate for full-text search** (`tantivy`): a large dependency for a
  corpus of a few thousand chunks that BM25 in 200 lines ranks well.
