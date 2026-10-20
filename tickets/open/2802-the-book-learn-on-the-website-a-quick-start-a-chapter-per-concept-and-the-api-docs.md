---
id: "2802"
title: "The book: Learn on the website, with a quick start, a chapter per concept whose code compiles in CI, and the API docs"
type: infra
milestone: "Engine: public"
model: sonnet-5
effort: high
status: todo
blocked_by: ["2801"]
nick_input: none
completed:
---

# 2802 — The book

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): "extensive
documentation". Bevy has its Quick Start, its unofficial book and
docs.rs. Shuyi has a README, ADRs and rustdoc with `missing_docs` on
(`Cargo.toml`), but no guide that teaches the engine to someone who
didn't build it.

2801 gives the website a Learn section. This ticket writes the book in
it, and the rule that keeps it current: a ticket that adds or changes a
public feature updates its chapter.

Docs rot when their code isn't built, so every code block in the book
comes from a file CI compiles.

## Nick input

None.

## Scope

**In:**
- **`site/content/learn/book/`**, chapters in this order, each a page
  of plain text with short examples:
  1. **Quick start:** install Rust (with the GNU toolchain note for
     Windows from `AGENTS.md`, *Environment*), `cargo install --locked
     shuyi-cli` (or from git until 2005), `shuyi new`, `shuyi run`,
     `shuyi web`.
  2. **A game and its screens:** `Game`, `GameData`, `Screen`, the
     screen stack, results (ADR-0018), `Ctx`.
  3. **Actions, never keys:** `ActionSet`, keymaps, rebinding, how text
     names a key (the `input-actions` skill, for people).
  4. **The frame and skins:** views as plain data, glyph and sprite
     skins, why tests read the scene (`AGENTS.md` rule 9).
  5. **Assets and formats:** kinds, formats, the registry, bundles and
     layers, packs (ADR-0007, ADR-0019, ADR-0023).
  6. **Rules as commands:** turn games, `shuyi_core::turn`, history and
     undo, determinism and the hash.
  7. **Saves and settings:** `Storage`, profiles, settings.
  8. **Testing:** the harness, replays, bots, snapshot tests of frames
     (`frame-png`).
  9. **Targets and export:** target profiles, `shuyi check --target`,
     the web build.
  10. **Tools for agents:** the tool contract (`docs/contract.md`),
      and the MCP server and skill pack once 1133 and 1134 are done
      (until then, a line saying they are coming).
- **Code that compiles:** `site/snippets/`, a crate in the workspace
  (`publish = false`) whose `src/` holds each chapter's code as
  functions and tests, marked with `// ANCHOR: name` and `// ANCHOR_END:
  name` lines. A Zola shortcode `{{ snippet(file="...", anchor="...") }}`
  includes the lines between them. CI builds and tests the crate with
  the rest of the workspace, so a breaking change breaks the book's
  build.
- **The API docs:** `cargo doc --no-deps --workspace` in `pages.yml`,
  copied to `/api/`. Learn links each crate's page. Once crates are on
  crates.io (2005), docs.rs links are added beside them.
- **Keeping it current:**
  - the `work-ticket` skill gains a step: a ticket that adds or changes
    something public updates its chapter in the same PR, or says in
    its completion notes why none is needed;
  - the write-ticket template's acceptance criteria gain the line "The
    book's chapter is updated (2802), or the completion notes say why
    not".
- **2004's guides** (one per direction) go in the book as a part after
  chapter 10 when 2004 is done; 2004 points here.

**Out (do not do):**
- the tutorial (2803);
- translations of the book;
- chapters for modules that don't exist yet (each module's ticket adds
  its own).

## Implementation steps

1. Read the README, the ADRs named above, and `docs/contract.md`.
2. Write the snippets crate and the shortcode, with one chapter's
   snippets.
3. Write the chapters, each with its snippets.
4. Add `cargo doc` to `pages.yml`, and the Learn links.
5. Change the `work-ticket` skill and the ticket template.
6. Run the gates.

## Acceptance criteria

- [ ] Ten chapters are live under `/learn/book/`.
- [ ] Every code block in the book comes from `site/snippets/` through
      the shortcode (a CI grep finds no fenced Rust block in
      `site/content/learn/book/` that isn't a shortcode).
- [ ] Renaming a public function a snippet uses fails CI.
- [ ] `/api/` holds each workspace crate's rustdoc.
- [ ] The `work-ticket` skill and `tickets/TEMPLATE.md` carry the rule.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The snippets crate's tests run in CI with the workspace's.
- `zola check` passes with the new pages.

## Completion notes

*(Filled in by the session that completes the ticket.)*
