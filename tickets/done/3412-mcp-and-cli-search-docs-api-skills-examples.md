---
id: "3412"
title: The MCP server and the CLI search the docs, the API, the skills and the examples, and serve the skills and Learn pages as resources
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-26
---

# 3412 — The MCP server and the CLI search the docs, the API, the skills and the examples, and serve the skills and Learn pages as resources

## Context

Nick's end goal is for Shuyi to be the most AI-friendly Rust engine
(`docs/decisions.md`, *2026-10-19: the platform layer goes in-house, and
the end goal*). Today the MCP server runs and checks a game well, but it
cannot help an agent find things. An agent with no training data on
Shuyi fails first at discovery ("how do I play a sound", "what is the
type for a sprite"), and nothing in the MCP session answers that.

What exists, checked on 2026-11-23:

- The contract's operations are `Op` in `crates/cli/src/contract.rs`
  (`new`, `check`, `lint`, `modules`, `add`, `run`, `scene`, `replay`,
  `playtest`, `autobalance`, `import`, `bake`, `doctor`, `explain`,
  `attach`, `export`, and the session's `start` … `stop`). None searches
  anything. `explain` covers problem codes only, and `modules` the
  module catalog only.
- `crates/cli/src/mcp/server.rs:780` declares only
  `"capabilities": { "tools": { "listChanged": true } }`: no `resources`
  and no `prompts`. `crates/cli/src/mcp/server/tests.rs:111` checks that
  `resources/list` is refused with `-32601` (`METHOD_NOT_FOUND`,
  `server.rs:103`). The `INSTRUCTIONS` constant (`server.rs:35`) never
  mentions the docs.
- `cargo xtask llms` (ticket 2808, `crates/xtask/src/llms.rs`) writes
  `llms.txt`, `llms-full.txt` and a Markdown twin of every Learn page,
  but only into the site's output. An agent inside the MCP session must
  leave it and fetch them over HTTP. `llms.txt` links the API as
  "rustdoc's HTML" only (`llms.rs:1504`).
- `shuyi` already embeds the skill pack (`skills.rs`, `Pack::embedded`),
  the template and the targets through `shuyi_embed::Table` in
  `crates/cli/build.rs` (ADR-0138). The Learn pages
  (`site/content/learn/`, about 300 KB, 28 pages), the book's snippets
  (`site/snippets/`, about 110 KB) and `examples/` are not embedded.
- The Learn pages use five shortcodes: `snippet` (203 uses), `newcomer`
  (236), `check` (61), `try_it` (47) and `exercise` (12).
- A grep of `tickets/`, `docs/adr/` and `docs/decisions.md` for "docs
  search", "search_docs", "resources/list", "BM25" and "how do I" found
  no plan. 2808's Scope leaves "hosted search, a chatbot" out.

The blind-spot audit of 2026-11-23 (its proposal 62) found this gap.
This ticket gives the contract a `docs` operation, so the CLI and the
MCP server answer the same, and serves the skills and Learn pages as
MCP resources. The search is lexical (BM25 over chunks) with no
embeddings and no network: the index is built in process from text the
binary embeds, at the engine revision the game pins.

Builds on: 2808 (the twins), 2509 (`explain`), ADR-0043 (the skill
pack), ADR-0138 (`shuyi_embed`), ADR-0039 (public types).

## Nick input

None.

## Scope

**In:**
- A `docs` contract operation (`Op::Docs`, `shuyi docs <query>`, the MCP
  tool `docs`). It needs no project, like `explain` and `doctor`. It
  takes a query and answers ranked hits. Each hit has a kind, a title, a
  source path in the engine repository, a section anchor, a URL for a
  Learn page (the twin's, as `llms.rs`'s `url_of` makes it), an integer
  score and a short excerpt. It also answers the rows of the "how do I"
  table that match.
- The corpus, embedded with `shuyi_embed::Table` in `crates/cli/build.rs`:
  the Learn pages, the book's snippets (so a `snippet` shortcode
  becomes its code in the index), the skill pack (already embedded), the
  problem-code pages `explain` serves, and each example's `.rs` and
  `.md` files under `examples/`.
- Chunks: a Learn page or a skill is cut at its `##` and `###`
  headings. An example file is one chunk of its `//!` doc comment and
  its public item names. A problem code page is one chunk.
- A "how do I" table, `site/data/how-do-i.toml`. Each row has a few task
  phrases ("play a sound", "save the game") and one target: a Learn page
  and anchor, a skill, or an example file. A test fails a row whose
  target the embedded corpus doesn't hold.
- MCP resources: `initialize` declares `"resources": {}` (no subscribe,
  no `listChanged`). `resources/list` lists each skill's `SKILL.md` as
  `shuyi://skills/<name>/SKILL.md` and each Learn page as
  `shuyi://learn/<path>`, with `mimeType: "text/markdown"`.
  `resources/read` returns the text. An unknown URI is error `-32002`
  (the MCP spec's "resource not found").
- `INSTRUCTIONS` says to call `docs` before guessing a name. The new
  game's `AGENTS.md` (`templates/shuyi-default/base/AGENTS.md`, the
  table around line 95) gets a `cargo shuyi docs <query>` row.
- The new schemas (`crates/cli/schemas/docs.input.json`,
  `docs.output.json`) get their fingerprint in the schemas'
  `versions.json`. A new operation breaks no client, so the contract's
  `VERSION` stays (ADR-0150).
- Public types: `DocsInput`, `DocsOutput`, `Hit`, `HitKind` and `HowRow`
  follow `explain.rs`'s pattern for contract types (`deny_unknown_fields`,
  `JsonSchema`, versioned by `VERSION`). `HitKind` is `#[non_exhaustive]`
  (ADR-0039): an `api` kind comes with 3413. No existing type changes
  shape; `Op` is already `#[non_exhaustive]`, so the new variant is
  additive.
- An ADR (the `write-adr` skill): the docs index is lexical BM25 over an
  embedded corpus at the pinned revision; no embeddings, no network, no
  hosted service. It names what is indexed and how a new source joins.

**Out (do not do):**
- A plain-text API listing per crate, doc aliases and the names agents
  guess: 3413 builds them. **Either order with 3413:** if 3413 is done,
  index its plain-text API files (`site/public/api/<crate>.txt`, written
  by `cargo xtask api-text`) as the API source: `HitKind::Api` hits, one
  chunk per item line (signature, first doc sentence), its aliases as
  extra terms. `site/public/` is ignored by git and `api-text` needs the
  nightly, so embed a tracked copy of the files that
  `cargo xtask api-text --check` keeps current. If not, nothing to do
  (3413 adds its `.txt` files to this index when it lands).
- New cookbook examples: 2786 writes them. **Either order with 2786:** if
  2786 is done, add a "how do I" row for each cookbook example; if not,
  nothing to do (2786 adds its rows to `site/data/how-do-i.toml`).
  `examples/` is indexed by walking it, so a new example is found with no
  change here.
- Search on the website: 2843's job.
- **Either order with 3414:** if 3414 is done, read the corpus tables
  through 3414's `embedded::bytes` helper (the table's `<NAME>_ROOT`
  folder, read from disk in a debug build), with a test that an edited
  Learn page is found without a rebuild; if not, nothing to do (3414
  wires the corpus tables to its helper when it lands).
- Embeddings, a vector store, any network call, MCP `prompts`, and
  `resources/subscribe`.
- Measuring a cold agent's success with the search: the eval (2525).
- Changing the twins `cargo xtask llms` writes.

**Estimated size:** about 1,400 lines (code 650, tests 500, docs 120,
data 130).

**Split:** past 900 lines, split off steps 7–8 (the MCP resources) into a
new ticket, blocked by this one.

## Implementation steps

1. **Embed the corpus.** In `crates/cli/build.rs`, add tables `learn`
   (`../../site/content/learn`), `snippets` (`../../site/snippets`) and
   `examples` (`../../examples`, with `Table::skip` patterns leaving out
   everything but `.rs` and `.md`: `files*`, `capture`, `bench`, images,
   `.ron`). Add the corpus's size to the test that checks the binary's
   embedded tables, if one exists; else assert the three tables together
   stay under 1 MB.
2. **The chunker,** `crates/cli/src/contract/docs/corpus.rs`. For a
   Learn page: drop the front matter; replace `{{ snippet(...) }}` with
   the snippet's text (read the `path` and optional `anchor` arguments as
   `llms.rs`'s `snippet` does); keep the body of `{% newcomer %}`,
   `{% check %}` and `{% exercise %}` blocks and drop their markers; drop
   `{{ try_it(...) }}`. Cut at `##` and `###` headings; the anchor is
   Zola's slug of the heading. For a skill: cut `SKILL.md` the same way;
   its other `.md` files are one chunk each. For an example file: one
   chunk of its `//!` lines plus the names after `pub fn`, `pub struct`,
   `pub enum` and `impl`. For a problem code: its page from `codes`.
   Order chunks by source path, then position.
3. **The index,** `crates/cli/src/contract/docs/index.rs`. Tokenize:
   lowercase ASCII, split on anything not alphanumeric, and also split
   `snake_case` and `CamelCase` words into their parts while keeping the
   whole word (`add_sprite` gives `add_sprite`, `add`, `sprite`). A short
   stop list (`the`, `a`, `of`, `to`, `is`, `and`, `in`, `it`). Score
   with BM25 (k1 = 1.2, b = 0.75), with a chunk's title terms counted
   twice. Floats are fine here: this is a tool, not a simulation. Report
   the score as an integer, thousandths rounded, and break ties by path,
   then position, so the answer is the same on every machine. Build it on
   first use, once per process (`OnceLock`).
4. **The "how do I" table,** `site/data/how-do-i.toml`, about 40 rows:
   ```toml
   [[row]]
   ask = ["play a sound", "sound effect", "play audio"]
   go = "learn/book/games-and-screens.md#the-context"
   ```
   `go` is `learn/<path>[#anchor]`, `skill/<name>` or
   `example/<path>`. Embed it (one more table, or `include_str!`). A
   row matches when every term of one `ask` phrase is in the query's
   tokens. Write the rows from the Learn pages and skills as they stand;
   check each target by hand as well as by the test.
5. **The operation,** `crates/cli/src/contract/docs.rs`, after
   `explain.rs`:
   - `DocsInput { query: String, limit: Option<u32>, kinds: Option<Vec<HitKind>> }`:
     `limit` defaults to 8 and is clamped to 1–50.
   - `DocsOutput { how: Vec<HowRow>, hits: Vec<Hit> }`.
   - `Hit { kind, title, path, anchor, url, score, excerpt }`; `excerpt`
     is the chunk's first 400 bytes cut at a character boundary, ending
     in `…` when cut.
   - An empty or all-stop-word query is `SH0001` (wrong arguments). No
     hit is a passing answer with empty lists and a limit line ("nothing
     matched; try fewer words, or `shuyi docs <a type's name>`").
   - Wire it into `Op`, `Op::ALL`, `Op::name`, the usage table at the top
     of `contract.rs`, the `pair::<…>` dispatch (as `Op::Explain` at
     `contract.rs:468`), `takes_project` (no) and `mcp.rs`'s
     `description`. CLI: `shuyi docs <words…> [--limit N] [--kind K]`.
6. **Schemas.** Write the two schemas and add the fingerprint to
   `versions.json` (the schemas' test says what).
7. **MCP resources.** In `server.rs`, declare `"resources": {}` in
   `initialize` (line 780) and handle `resources/list` and
   `resources/read` before the `tools/*` arms (line 222). URIs:
   `shuyi://skills/<name>/SKILL.md` and `shuyi://learn/<path>` (the
   source path under `site/content/learn/`). `resources/read` answers
   `{ "contents": [{ "uri", "mimeType": "text/markdown", "text" }] }`.
   The Learn text is the chunker's cleaned page (step 2), not the raw
   source with shortcodes. An unknown URI is error `-32002` with the URI
   in `data`.
8. **Change the refused-method test** at `server/tests.rs:111` to
   `prompts/list`, which stays refused.
9. **Say so.** Update `INSTRUCTIONS`, the template's `AGENTS.md` row,
   the book's *Tools for agents* chapter
   (`site/content/learn/book/tools-for-agents.md`, its *The tool
   contract* and *The MCP server* sections) and `crates/cli/src/mcp.rs`'s
   module docs. Write the ADR.
10. **Parity with the twins.** In `crates/xtask/src/llms/tests.rs` (xtask
    already depends on `shuyi-cli`), a test that for every Learn page the
    chunker's headings are the twin's `##`/`###` headings, in order, and
    the chunker's text holds every snippet's code the twin shows. A new
    shortcode added to Learn then fails both `llms.rs` and this test.

## Acceptance criteria

- [x] `shuyi docs play a sound --json` passes, and its first hit or a
      `how` row names the page or skill that says how to play a sound.
- [x] `shuyi docs add_sprite` has a hit whose excerpt contains
      `add_sprite` (from a snippet or an example).
- [x] `shuyi docs SH2003` has the problem code's page as a hit.
- [x] `shuyi docs ""` fails with `SH0001` and exit code 2.
- [x] `shuyi docs zzzqqq` passes with no hits and a limit line.
- [x] `--limit 0` gives 1 hit and `--limit 500` at most 50 (the clamp's
      two ends).
- [x] The same query gives byte-identical JSON on two runs and on Linux
      and Windows CI.
- [x] The MCP `tools/list` lists `docs`; `tools/call` of `docs` answers
      what the CLI answers.
- [x] ~~`initialize` declares `resources`; `resources/list` lists all 18
      skills' `SKILL.md` and every Learn page; `resources/read` of
      `shuyi://skills/shuyi-input/SKILL.md` returns that file's text;
      an unknown URI is `-32002`; `prompts/list` is still `-32601`.~~
      (not done: split off to 3434 at the ticket's Split: point)
- [x] Every row of `site/data/how-do-i.toml` resolves to a chunk in the
      corpus (test), and there are at least 30 rows.
- [x] The schemas have their fingerprint in `versions.json`, and the
      contract's `VERSION` is the base's (`cargo xtask contract`
      passes).
- [x] The ADR is in `docs/adr/` and indexed in its README.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the tokenizer (snake and camel splits, stop words, non-ASCII
  kept as one token); the chunker on a page with each of the five
  shortcodes; BM25 ranks a chunk whose title has the term above one whose
  body has it once; tie order; the `limit` clamp at 0, 1, 50 and 51; the
  excerpt cut on a multi-byte character; an empty query is `SH0001`.
- Property: for any query, the hits are sorted by score then path, and
  their number is at most the limit.
- Integration: `crates/cli/tests/` runs `shuyi docs` with `--json` on the
  queries of the acceptance criteria; `server/tests.rs` drives
  `resources/list`, `resources/read` (known and unknown) and `docs`
  through `handle`; the how-do-I table test; the xtask parity test of
  step 10.
- Snapshot: the `docs` schemas, by the schemas' existing test.

## Completion notes

What was done:

- `shuyi docs <words…> [--limit N] [--kind K]` and the MCP tool `docs`
  (`Op::Docs`, `crates/cli/src/contract/docs.rs`), with no project: the
  best hits (8; 1 to 50) of BM25 over the embedded corpus, each with its
  kind, title, path, anchor, twin URL, score in thousandths and excerpt,
  and the "how do I" rows the query asks. `DocsInput`, `DocsOutput`,
  `Hit`, `HitKind` (`#[non_exhaustive]`: `learn`, `skill`, `example`,
  `code`) and `HowRow`.
- The corpus (`docs/corpus.rs`): `crates/cli/build.rs` embeds
  `site/content/learn`, `site/snippets`, `site/tutorial` (the files a
  snippet shows) and `examples` (its `.rs` and `.md` files), beside the
  skill pack and the problem codes' pages already embedded. The index
  (`docs/index.rs`) is built once per process.
- `site/data/how-do-i.toml`, 50 rows, each target checked by a test and
  by hand against the headings as they stand.
- `INSTRUCTIONS` says to call `docs` before guessing a name; the
  template's `AGENTS.md` has a `cargo shuyi docs <query>` row; the
  `shuyi-new-game` skill's *When not to* says to search the docs for a
  task no skill names; `docs/contract.md` has the operation's row and a
  *Searching the docs* section; the book's *Tools for agents* has it in
  *The tool contract* and *The MCP server*.
- ADR-0153; the schemas `docs.input.json` and `docs.output.json`, with
  their fingerprint under contract 62 (no break: a new operation).
- The xtask parity test
  (`the_docs_search_reads_each_learn_page_as_its_twin_does`): every Learn
  page's headings are its twin's, in order, and every code block its twin
  shows (over 200, `try_it`'s diffs left out) is in the search's text.

Deviations:

- **Split at the ticket's Split: point.** Steps 7–8, the MCP resources,
  are ticket 3434 (blocked by this one), as the **Split:** line says past
  900 lines. Its acceptance criteria are 3434's.
- **A fourth table, `tutorial`.** 137 of the Learn pages' `snippet`
  calls read `site/tutorial/<step>/<file>` (`step=`), which the ticket
  didn't plan to embed. The table holds the tutorial's code, data, maps
  and tests, without its builds, fonts, art or `Cargo.toml`s. The size
  test holds the three planned tables (learn, snippets, examples: about
  960 KB) under 1 MB and the tutorial's under 600 KB more.
- **The generated Learn pages are left out.** The site's build writes
  `site/content/learn/book/guides/` and `site/content/learn/errors/`
  (ignored by git); embedding them made the corpus depend on whether the
  site was built first (the size test failed after the site gate ran).
  `build.rs` skips both, and the parity test leaves them out; the problem
  codes' pages are in the corpus from their table.
- **A game's tools' output is left out** (`**/dist/**`). CI's `bench`
  job failed: its first run wrote `examples/hello-grid/dist/bench/run-1/bench-report.md`,
  the `examples` table embedded it, and the second run's rebuilt `shuyi`
  counted every frame 1,702 instructions higher.
- **An excerpt starts where the match is.** The ticket's excerpt was
  the chunk's first 400 bytes. A long section that holds `add_sprite`
  in its third paragraph then shows no `add_sprite`, so the excerpt
  starts at the line where the chunk first holds the query's rarest term
  (by IDF) when that is past the first 400 bytes, with a `…` before it.
  `add_sprite` is found in the `shuyi-screen` and `shuyi-animate`
  skills: `examples/hello-grid/src/map.rs` calls it, and an example's
  chunk holds only its `//!` lines and its public items' names, as the
  ticket says.
- `kinds` filters the `how` rows too, not only the hits.
- Byte-identical JSON on Linux and Windows: a test pins the exact scores
  of a fixed corpus and query (`the_scores_are_the_same_numbers_on_every_machine`),
  so CI fails on either OS if a float rounds otherwise; the real
  corpus's answer is checked byte for byte between two runs and against
  a freshly built index, not snapshotted (a snapshot would change with
  every edit of the book).
- 3413 (API text) and 3414 (disk reading) aren't done, and 2786's
  cookbook isn't: nothing of theirs here. Their clauses now say what
  holds (**From 3412 (done):** in each).

Follow-up tickets: 3434 (the MCP resources, split off). 3413 adds the
API's listing as `HitKind::Api`,
3414 reads the corpus from disk in a debug build, 2786 adds the
cookbook's "how do I" rows, 2525 measures the search with a cold agent.

Behaviour: shuyi-cli: the MCP server's `instructions` say to call
`docs` before guessing a name, and its `tools/list` lists `docs`.

No break: `Op::Docs` is a new variant of a `#[non_exhaustive]` enum and
the other types are new; the contract's `VERSION` stays 62. The game
keeps no list of operations (searched `Zafnok/visions-of-shuyi` for
`Op::ALL` and `resources/list`: only `docs/engine/pins.md`'s notes).

Game: nothing to adopt, the game gets `cargo shuyi docs` with its next
pin move, and its code uses none of it.

Size: 2,629 counted lines, the chunker, the index and the operation (about
1,900 of them, with their tests) answer nothing apart: an index with no
operation is dead code, and an operation with no corpus finds nothing.
The rest, the "how do I" table and the twins' parity test, are this
operation's acceptance; split off, this PR stays over 1,500. The MCP
resources were split off (3434).

Nick input: none.
