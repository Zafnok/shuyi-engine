---
id: "3439"
title: "shuyi docs searches the API too: the plain-text listing as Api hits, and the doc aliases as their terms"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3439 — shuyi docs searches the API too: the plain-text listing as Api hits, and the doc aliases as their terms

## Context

Split off 3413 (done), whose *From 3412 (done)* clause this was: 3413
landed the plain-text API per crate (`cargo xtask api-text --out <dir>`,
`crates/xtask/src/api_text.rs`, written into `site/public/api/` by
pages.yml) and the table of other engines' words
(`site/data/engine-terms.toml`), but left them out of 3412's docs index
to stay inside its size.

3412 (done) built `shuyi docs`: BM25 over a corpus embedded in `shuyi`
at its revision ([ADR-0154](../../docs/adr/0154-the-docs-search-bm25-over-an-embedded-corpus-at-the-pinned-revision.md)).
Its §7 says how a source joins: a `shuyi_embed` table in
`crates/cli/build.rs` (or an `include_str!`, as `how-do-i.toml` is), a
variant of `HitKind` (`#[non_exhaustive]`, so additive), and a function
in `crates/cli/src/contract/docs/corpus.rs` that cuts it into chunks,
added to `corpus::chunks`. Today an agent that asks `docs` for
`draw_sprite` finds the appendix's table (3413 reads it into the corpus
as text) but no API item.

The plain-text API is built from rustdoc JSON on the pinned nightly
(`api::NIGHTLY`), which `cargo build` of `shuyi` can't run. So `shuyi`
embeds a tracked copy, which a check keeps current.

## Nick input

None.

## Scope

**In:**

- **A tracked copy** of the listing, `crates/cli/api/<crate>.txt`, one
  per crate of `site/data/api.toml`, written by `cargo xtask api-text
  --out crates/cli/api` and in the shape 3413's files have. To keep it
  small it holds the item lines only (no `impl` line, no line without an
  item in the index), so a change to a trait impl doesn't move it.
  `cargo xtask pr-size` counts it as generated (`is_generated`).
- **The check:** `cargo xtask api-text --check` (CI's `api` job) also
  fails when the tracked copy differs from what it would write, naming
  the crate and the command that rewrites it.
- **The corpus:** `HitKind::Api`; `corpus::api_chunks` cuts the copy
  into one chunk per item (title: its path; text: its signature, its
  first doc sentence and its aliases, so an alias is a term of the item
  it names); `corpus::chunks` adds them.
- **The answer** names an `Api` hit's URL in rustdoc's HTML on the site
  (`api/<crate>/…`), or none when the path doesn't map to a page.

**Out (do not do):**

- Changing the ranking (ADR-0154) beyond what a new kind needs.
- Rustdoc's own search, the aliases and the site's files: 3413 (done).

**Estimated size:** about 900 lines (code 350, tests 300, docs 250),
counted from the steps and doubled; the tracked copy is left out of the
count as generated.

## Implementation steps

1. `--items-only` (or the tracked copy's own writer) in `api_text.rs`,
   and `--check`'s comparison with `crates/cli/api/`.
2. The copy written, and `pr_size::is_generated` naming it.
3. `HitKind::Api`, `corpus::api_chunks` over a `shuyi_embed` table of
   `crates/cli/api`, and `corpus::chunks`.
4. The contract's schema for the new kind (a version rise only if
   ADR-0151's rule says a client breaks: an added enum value of an
   answer).
5. Docs (the `document` skill): the book's *Tools for agents* says
   `docs` finds API items and their aliases.

## Acceptance criteria

- [ ] `shuyi docs --query draw_sprite` answers
      `shuyi_ui::frame::glyph_buffer::GlyphBuffer::add_sprite` among its
      first three hits, as an `Api` hit.
- [ ] `cargo xtask api-text --check` fails, naming the crate, when
      `crates/cli/api/` is out of date, and passes on this branch.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `api_chunks` on a small fixture copy (an item, its aliases);
  the stale-copy check on a fixture.
- Snapshot: the `docs` answer for `draw_sprite`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
