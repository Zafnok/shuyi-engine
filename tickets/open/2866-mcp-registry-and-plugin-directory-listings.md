---
id: "2866"
title: "Agent directories: the MCP server in the official MCP Registry from the release job, and the skill pack ready for Anthropic's plugin directory"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2859", "2005"]
nick_input: setup
completed:
---

# 2866 — The MCP Registry and the plugin directory

## Context

The engine's first audience is people who drive a coding agent, and they
find tools in MCP directories and in their agent's plugin directory. A
listing keeps working long after launch posts fade.

- **The official MCP Registry** takes a server through its CLI,
  `mcp-publisher`, signed in with GitHub (OIDC in Actions), with no PR on
  any repository, so rule 12 isn't touched
  (<https://modelcontextprotocol.io/registry/about>,
  <https://modelcontextprotocol.io/registry/github-actions>). A cargo
  package proves it owns the name by an `mcp-name:` line in its README as
  crates.io shows it
  (<https://modelcontextprotocol.io/registry/package-types>). Other
  directories read from the registry.
- **The skill pack** is already a plugin marketplace
  (`.claude-plugin/marketplace.json`, the plugin `shuyi` from `skills/`).
  Anthropic's plugin directory takes one through a web form the owner
  submits (<https://claude.com/docs/plugins/submit>): not a PR, flagged
  here because it is Nick's submission. Only if 2859 says yes.

The heat check of 2859: a listing's text can't be changed without a new
version, so it says "grid games", never "make games" or "2D games" alone.
The marketplace's description today says "making 2D games", and changes
here.

## Nick input

**Setup**, after the release, if 2859 said yes (about 30 minutes):

1. Sign in at the directory's submission page the PR links
   (<https://claude.com/docs/plugins/submit> says where).
2. Submit the plugin: repository `Zafnok/shuyi-engine`, the marketplace at
   its root, the plugin `shuyi`, at the release tag the PR names (not
   `main`).
3. Data handling: no personal data, no outside service; it runs cargo on
   the user's machine (as `skills/README.md` says).
4. Read each compliance box before ticking it, then submit.
5. Answer a reviewer's request yourself, or hand it to an agent session.

## Scope

**In:**
- `server.json` at the root: the name the registry gives the `Zafnok`
  GitHub namespace; package type cargo, the crate that installs the
  `shuyi` binary (`shuyi-cli`), its version, stdio transport and the
  arguments that start the MCP server; the description "Make, run, step,
  screenshot and test 2D grid games with no window, from an agent.
  Deterministic, text-first. Rust, MIT or Apache-2.0."
- The `mcp-name:` line, visible, in `shuyi-cli`'s README.
- A step in the release workflow (2005's) after `cargo publish`: install
  `mcp-publisher` at a pinned version, checked by its hash, license
  checked (`AGENTS.md`, rule 7); sign in by GitHub OIDC; publish. On every
  release, since the registry is in preview and may reset.
- A test that `server.json`'s version equals `shuyi-cli`'s.
- `server.json` checked against the registry's schema in CI.
- `.claude-plugin/marketplace.json`'s descriptions say "grid games
  (turn-based and in real time)"; the plugin's version follows the
  release.
- `skills/README.md` says what runs locally (cargo builds the game) and
  that nothing is sent anywhere.
- A per-release line in the release's checklist: look (read only) at the
  listing on the registry and on any directory that copies it; a wrong
  description is fixed here, at the source.
- **Either order with 2865:** if 2865 is done, add a row for Nick's
  plugin form to `docs/launch/checklist.md`; if not, nothing to do (2865
  adds it).

**Out (do not do):**
- Claiming a listing on any directory by signing in, a `glama.json`, a
  paid boost, or a form that asks for an email.
- Awesome lists, arewegameyet.rs and any submission by PR or issue
  (rule 12).
- The Codex plugin directory (later, once this listing shows what
  reviewers ask).

## Implementation steps

1. Write `server.json`, the README line and the version test.
2. Add the schema check to CI.
3. Add the release step: least permissions (`id-token: write` for OIDC
   only in that job), pinned, zizmor clean.
4. Fix the marketplace's descriptions; update `skills/README.md`.
5. After the release, check the registry's API lists the server at the
   released version; note it.

## Acceptance criteria

- [ ] `server.json` exists and passes the registry's schema in CI.
- [ ] The version test fails a `server.json` whose version differs from `shuyi-cli`'s.
- [ ] `shuyi-cli`'s README shows the `mcp-name:` line as text.
- [ ] `server.json`'s and the marketplace's descriptions say "grid" and have no word from 2861's keep-out list.
- [ ] zizmor reports no finding on the release job.
- [ ] After the release, the registry's API returns the server at the released version (completion notes).
- [ ] The skill pack (`skills/`) is updated, or the completion notes have a `Skills:` line saying why not.
- [ ] The completion notes have a `Book:` line.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the version check, both ways.
- CI: the schema check of `server.json`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
