---
id: "3434"
title: The MCP server serves the skills and the Learn pages as resources
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["3412"]
nick_input: none
completed:
---

# 3434 — The MCP server serves the skills and the Learn pages as resources

## Context

Split from 3412 (its steps 7–8), which ran past its **Split:** point.
3412 gave the contract a `docs` operation: lexical BM25 over a corpus
`shuyi` embeds at the engine's revision (ADR-0154), with the Learn pages
read as their twins read them (`crates/cli/src/contract/docs/corpus.rs`,
`learn_page` and `learn_pages`) and the skill pack embedded
(`crates/cli/src/skills.rs`, `Pack::embedded`).

An agent that found a page with `docs` still reads it whole by leaving
the MCP session. MCP has resources for that: documents the client lists
and reads by URI. Today `crates/cli/src/mcp/server.rs`'s `initialize`
declares only `"capabilities": { "tools": { "listChanged": true } }`,
and `crates/cli/src/mcp/server/tests.rs` (in `it_speaks_json_rpc`)
checks that `resources/list` is refused with `-32601`
(`METHOD_NOT_FOUND`).

ADR-0154 §6 records the decision this ticket builds: resources with no
subscriptions and no `listChanged`, since the text changes only with the
engine.

## Nick input

None.

## Scope

**In:**
- `initialize` declares `"resources": {}` beside `tools`.
- `resources/list` lists each skill's `SKILL.md` as
  `shuyi://skills/<name>/SKILL.md` (with its front matter's description
  and a title) and each Learn page as `shuyi://learn/<path>` (its path
  under `site/content/learn/`, its front matter's title), all
  `mimeType: "text/markdown"`: skills in name order, then pages in path
  order.
- `resources/read` answers
  `{ "contents": [{ "uri", "mimeType": "text/markdown", "text" }] }`: a
  skill's `SKILL.md` as it is (`Skill::text`), a Learn page as
  `# <title>`, a blank line, then `corpus::learn_page`'s cleaned text
  (its shortcodes read, not the raw source). A missing `uri` is
  `-32602`; an unknown URI is `-32002` (MCP's "resource not found") with
  the URI in `data`.
- A server that lists another workspace's server's tools (`listing` in
  `Server`) asks that server first and answers its response when it has
  a `result`, so the resources are the pinned engine's; otherwise its
  own.
- `INSTRUCTIONS` names the resources' two URI forms.
- No public type changes shape: the new module's functions return
  `serde_json::Value`.

**Out (do not do):**
- MCP `prompts`, `resources/subscribe`, resource templates and
  `listChanged`.
- Serving the examples or the problem codes as resources (`docs` and
  `explain` answer them).
- Reading the corpus from disk in a debug build: 3414's.

**Estimated size:** about 400 lines (code 150, tests 150, docs 100).

## Implementation steps

1. **`crates/cli/src/mcp/resources.rs`**, a new module (`pub mod
   resources;` in `mcp.rs`): `list() -> Vec<Value>`, `text(uri) ->
   Option<String>` and `read(uri) -> Option<Value>`, as the Scope says.
   A skill URI's file part must be `SKILL.md` (`skills::FILE`).
2. **`server.rs`**: a `RESOURCE_NOT_FOUND: i64 = -32002` beside the
   other codes; a `"resources/list" | "resources/read"` arm in
   `Server::handle` before `tools/*`'s, calling a method
   `Server::resources(&mut self, id, method, params) -> String` that
   forwards as the Scope says (`self.listing`, `self.forwarded`,
   `Forward::request`), then answers its own; `initialize`'s
   capabilities; `INSTRUCTIONS`.
3. **The refused-method test**: in `it_speaks_json_rpc`, ask
   `prompts/list` in place of `resources/list`, which stays `-32601`.
4. **Say so**: `crates/cli/src/mcp.rs`'s module docs and `server.rs`'s
   (the methods it answers); `docs/contract.md`'s *The MCP server* (a
   paragraph on the resources before *The session's folder*); the book's
   *Tools for agents*, *The MCP server* (one paragraph: each skill and
   each page of the book as a resource, with an example URI of each).

## Acceptance criteria

- [ ] `initialize` declares `resources` (`{}`).
- [ ] `resources/list` lists all 18 skills' `SKILL.md` and every Learn
      page (as many as `corpus::learn_pages()`), each `text/markdown`.
- [ ] `resources/read` of `shuyi://skills/shuyi-input/SKILL.md` returns
      that file's text; of `shuyi://learn/book/actions.md`, text that
      starts `# Actions, never keys` and holds no `{%` and no
      `{{ snippet`.
- [ ] An unknown URI (`shuyi://learn/nope.md`,
      `shuyi://skills/shuyi-input/example.ron`) is `-32002` with the URI
      in `data`; a read with no `uri` is `-32602`.
- [ ] `prompts/list` is still `-32601`.
- [ ] A server following another workspace's server answers that
      server's `resources/read`, and its own when that one answers an
      error.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, in `resources.rs`: the list's order and count, a skill's and a
  page's text, the unknown URIs.
- `server/tests.rs`: `initialize`, `resources/list`, `resources/read`
  (known, unknown, no `uri`) through `handle`; the forwarded case with a
  fake `Forward` (the tests' existing fakes of a forwarded server).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
