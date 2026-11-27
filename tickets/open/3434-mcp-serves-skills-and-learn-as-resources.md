---
id: "3434"
title: The MCP server serves the skills and the Learn pages as resources
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3435"]
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
(`crates/cli/src/skills.rs`, `Pack::embedded`: each `Skill` has its
`name` and every file of its folder in `files`).

An agent that found a page with `docs` still reads it whole by leaving
the MCP session. MCP has resources for that: documents the client lists
and reads by URI. Today `crates/cli/src/mcp/server.rs`'s `initialize`
declares only `"capabilities": { "tools": { "listChanged": true } }`,
and `crates/cli/src/mcp/server/tests.rs` (in `it_speaks_json_rpc`)
checks that `resources/list` is refused with `-32601`
(`METHOD_NOT_FOUND`).

**Rewritten on 2026-11-26 for MCP's skills extension.** Anthropic's
article *Skills over MCP, tasks and more* (2026-11-24) put skills served
by an MCP server in Claude and Claude Code (public beta). They are MCP's
**skills extension**, `io.modelcontextprotocol/skills` (SEP-2640, Final;
spec at `github.com/modelcontextprotocol/ext-skills`,
`specification/beta/skills.mdx`; overview at
`modelcontextprotocol.io/extensions/skills/overview`). A client that
speaks it lists our skills as its own, loads each only when needed
(`shuyi:shuyi-input` in Claude Code), and always at the engine revision
the server is built at, which for a game's session is the game's pinned
engine (ADR-0035 §2 forwards the call to that workspace's server). The
first plan here, plain resources at `shuyi://skills/<name>/SKILL.md`,
would be read by no host as a skill. What the extension asks, as read on
2026-11-26 (read the spec again when you start):

- It is written against protocol **2026-07-28** and declared in
  `server/discover`'s `capabilities.extensions`, so this ticket is
  blocked by 3435. A server declaring it MUST declare `resources` too.
  `{ "directoryRead": true }` promises `resources/directory/read`;
  `{}` doesn't.
- **`skills/list`** (paginated: `params.cursor`, `nextCursor`) answers
  one entry per skill: `uri` (its `SKILL.md`'s), `frontmatter` (every
  field of its YAML front matter, as it is) and `resources`, the
  complete manifest of its files, each `{ uri, digest, size }` with
  `digest` `sha256:<hex>`, `SKILL.md` among them. **`skills/get`**
  (`params.uri`) answers one in `result.skill`. Both carry
  `resultType: "complete"`, `ttlMs` and `cacheScope`.
- URIs are `skill://<skill-path>/<file-path>`; the last segment of
  `<skill-path>` MUST be the front matter's `name`. Files are read with
  `resources/read`. An unknown skill or file is `-32602`; an internal
  failure `-32603`.
- `SKILL.md` MUST start with front matter holding `name` and
  `description` (the Agent Skills spec, `agentskills.io/specification`,
  which `check-skills` already holds our pack to).
- Limits: servers SHOULD keep under 512 files and 16 MiB per skill.
  Claude Code loads at most 100 skills per server, text files up to
  100 KB and skills up to 400 KB, and no archives. Our pack, measured
  on 2026-11-26: 18 skills, the largest `shuyi-balance-bots` (5 files,
  32,555 bytes), the largest file 22,375 bytes (`shuyi-ship`). A test
  keeps it so.

The Learn pages are no skills. They stay plain resources under
`shuyi://learn/<path>`, on both protocols.

ADR-0154 §6 records resources with no subscriptions and no
`listChanged`, since the text changes only with the engine. That holds:
the skills' `ttlMs` can be long.

**A game holds its own copy of the pack too.** `shuyi new` writes the
pack into the game's `.claude/skills/` (`template::compose::with_skills`),
and 2578 keeps it current (`shuyi skills`). A host then sees
`shuyi-input` twice, the copy and the server's `shuyi:shuyi-input`. The
spec says hosts MUST NOT let same-named skills silently replace each
other, so both are listed; with 2578's `--check` passing they are the
same text. Keep the copies (Codex and harnesses without the extension
read only the folder) and say in the docs that the served ones are the
pinned engine's.

## Nick input

None.

**From 3435 (done):** there is no `Protocol` type. `Server::handled`
picks the generation per request: `of_old` (the old protocol, no
version in `_meta`, and `initialize`) and `of_stateless` (2026-07-28)
each have their own `match method`, and the handlers return the JSON-RPC
message as a `Value`, not a `String`. A 2026-07-28 response goes through
`of_2026(message, "complete")`, which adds `resultType` and `serverInfo`;
`server/discover`'s capabilities are `discover()`'s. On 2026-07-28 a
missing resource is `-32602` (SEP-2164), on the old protocol `-32002`.
`ttlMs` and `cacheScope` are required on `resources/list` and
`resources/read` (ADR-0156 §2 says how `tools/list` picks them).

## Scope

**In:**
- **The skills extension** on a 2026-07-28 connection (3435's
  `of_stateless`): `server/discover` declares `"resources": {}` and
  `"extensions": { "io.modelcontextprotocol/skills": {} }` (no
  `directoryRead`). `skills/list` answers every skill of
  `Pack::embedded()` in name order, in one page (no `nextCursor`; a
  `cursor` given is `-32602`), each with `uri`
  `skill://<name>/SKILL.md`, its `frontmatter` as JSON (the YAML block
  between the `---` lines read by `serde_norway` into a
  `serde_json::Value`, as `Frontmatter::parse` finds it, so every field
  is kept as written, `metadata` included; `Frontmatter` itself is
  `deny_unknown_fields`, which `check-skills` already holds the pack
  to), and its manifest: every file of
  `Skill::files`, `skill://<name>/<path>`, its `sha256:` digest and its
  size in bytes. `skills/get` answers the same entry for a known
  `SKILL.md` URI. `ttlMs` one hour, `cacheScope: "public"`.
- **`resources/read`** answers a skill's file by its manifest URI,
  `{ "contents": [{ "uri", "mimeType", "text" }] }`: `text/markdown`
  for `.md`, `text/plain` for the rest (every file of the pack is
  text: a test says so). The bytes served are the bytes the digest was
  taken of; no `\r\n` rewriting between the two.
- **The Learn pages**, on both protocols: `resources/list` lists each as
  `shuyi://learn/<path>` (its path under `site/content/learn/`, its
  front matter's title), `mimeType: "text/markdown"`, in path order;
  `resources/read` answers `# <title>`, a blank line, then
  `corpus::learn_page`'s cleaned text (its shortcodes read, not the raw
  source).
- **On the old protocol** (`initialize`), `initialize` declares
  `"resources": {}`, and `resources/list` lists the skills'
  `skill://<name>/SKILL.md` files too (so a client without the extension
  can still read them), skills first, then the pages. Readable as
  above.
- **Errors**: a missing `uri` is `-32602`. An unknown URI is `-32602` on
  2026-07-28 (SEP-2164) and `-32002` on the old protocol (its "resource
  not found"), with the URI in `data` both ways.
- **Forwarding**: a server that lists another workspace's server's
  tools (`listing` in `Server`) asks that server first for
  `skills/list`, `skills/get`, `resources/list` and `resources/read`,
  and answers its response when it has a `result`, so the skills are
  the pinned engine's; otherwise its own (a pinned `shuyi` older than
  this answers `-32601`).
- **`INSTRUCTIONS`** names the two URI forms, and that the skills are
  the engine revision the game pins.
- **SHA-256**: `sha2` (MIT OR Apache-2.0, already in `Cargo.lock` for
  `xtask`, `default-features = false`) becomes a dependency of
  `shuyi-cli`. Check it with `cargo deny check licenses` and add its
  line to `tools/layers.ron` if the CLI's outside crates are listed
  there.
- No public type changes shape: the new module's functions return
  `serde_json::Value`.
- **Estimated size:** about 900 lines (code 350, tests 400, docs 150).

**Out (do not do):**
- The 2026-07-28 protocol itself (`server/discover`, `_meta`,
  `resultType`): 3435.
- `resources/directory/read` and `directoryRead: true`; skills nested
  in skills.
- MCP `prompts`, `resources/subscribe`, `subscriptions/listen` for
  resources, resource templates and `listChanged`.
- Serving the engine's own workflows (`.claude/skills/`): they are the
  repository's, not a game's.
- Serving the examples or the problem codes as resources (`docs` and
  `explain` answer them).
- Dropping the copy of the pack in a new game, or changing `shuyi new`:
  2578 keeps it current.
- Reading the corpus from disk in a debug build: 3414's.

## Implementation steps

1. Read the skills extension's spec (its `skills/list` and `skills/get`
   result shapes, the manifest, the URI rules, the errors) and
   `crates/cli/src/skills.rs` (`Skill`, `Frontmatter`, `Pack`).
2. **`crates/cli/src/mcp/resources.rs`**, a new module (`pub mod
   resources;` in `mcp.rs`): `skills() -> Vec<Value>` (the entries),
   `skill(uri) -> Option<Value>`, `list(protocol) -> Vec<Value>` (the
   `resources/list` entries), `read(uri) -> Option<Value>`. A skill file
   URI must be one of its manifest's.
3. **`server.rs`**: `skills/list`, `skills/get`, `resources/list` and
   `resources/read` arms in `Server::handle` before `tools/*`'s, through
   one `Server::resources(&mut self, id, method, params, protocol) ->
   String` that forwards as the Scope says (`self.listing`,
   `self.forwarded`, `Forward::request`), then answers its own; the
   capabilities in `initialize` and in 3435's `server/discover`; a
   `RESOURCE_NOT_FOUND: i64 = -32002` beside the other codes;
   `INSTRUCTIONS`.
4. **The refused-method test**: in `it_speaks_json_rpc`, ask
   `prompts/list` in place of `resources/list`, which stays `-32601`.
5. **Say so**: `crates/cli/src/mcp.rs`'s module docs and `server.rs`'s
   (the methods it answers); `docs/contract.md`'s *The MCP server* (a
   paragraph on skills and resources before *The session's folder*);
   the book's *Tools for agents*, *The MCP server* (one paragraph: each
   skill served as a skill to clients that speak the extension, as
   `shuyi:<name>` in Claude Code, and each page of the book as a
   resource, with an example URI of each); `skills/README.md` (a game's
   agent also gets the pinned pack from the server).
6. **Check by hand** in Claude Code (2026-07-28, v2.1.331 or later): the
   repository's server's skills appear as `shuyi:shuyi-input` and the
   rest, and one loads when asked about input. Write what you saw in
   the completion notes.

## Acceptance criteria

- [ ] On 2026-07-28, `server/discover` declares `resources` and
      `io.modelcontextprotocol/skills`.
- [ ] `skills/list` answers all 18 skills in name order; each entry's
      `uri` ends `/SKILL.md`, its `frontmatter.name` equals the URI's
      skill segment, and its manifest lists every file of the skill with
      a digest that equals the SHA-256 of the bytes `resources/read`
      answers for it.
- [ ] `skills/get` of `skill://shuyi-input/SKILL.md` answers the same
      entry as `skills/list`; of `skill://nope/SKILL.md`, `-32602`.
- [ ] `resources/read` of `skill://shuyi-test/example.ron` answers that
      file's text; of `shuyi://learn/book/actions.md`, text that starts
      `# Actions, never keys` and holds no `{%` and no `{{ snippet`.
- [ ] On 2025-11-25, `initialize` declares `resources`;
      `resources/list` lists the 18 `SKILL.md` files then every Learn
      page (as many as `corpus::learn_pages()`); an unknown URI is
      `-32002` with the URI in `data`.
- [ ] A read with no `uri` is `-32602` on both protocols.
- [ ] A test fails a skill of more than 100 files or 400 KB, or a file
      over 100 KB, or a file that isn't UTF-8.
- [ ] `prompts/list` is still `-32601`.
- [ ] A server following another workspace's server answers that
      server's `skills/list`, and its own when that one answers an
      error.
- [ ] Checked in Claude Code by hand (step 6).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, in `resources.rs`: the entries' order, count and fields; each
  digest against the bytes; the unknown URIs; the size limits.
- `server/tests.rs`: both protocols' capabilities; `skills/list`,
  `skills/get`, `resources/list`, `resources/read` (known, unknown, no
  `uri`) through `handle`; the forwarded case with a fake `Forward` (the
  tests' existing fakes of a forwarded server).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
