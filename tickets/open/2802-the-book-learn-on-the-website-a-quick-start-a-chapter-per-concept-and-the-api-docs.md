---
id: "2802"
title: "The book: Learn on the website, with a quick start, a chapter per concept whose code compiles in CI, and the API docs"
type: infra
milestone: "Engine: public"
model: sonnet-5
effort: high
status: todo
blocked_by: ["2801", "2511", "2513", "2810"]
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
     shuyi-cli` (or from git until 2005), `shuyi new blank` and what
     each of its files is on the short path (2511: `BasicGame`,
     `run_game`, `BasicTools`), `shuyi run`, `shuyi web`. On Linux,
     `bash scripts/setup-linux.sh` in the new game's folder before
     `shuyi run` (2513; `cargo install shuyi-cli` itself needs only a
     C linker). An "Open in GitHub Codespaces" link to the engine's
     repository (`https://codespaces.new/Zafnok/shuyi-engine?quickstart=1`,
     2513's dev container), to try it with nothing installed, billed to
     whoever opens it.
  2. **A game and its screens:** `Game`, `GameData`, `Screen`, the
     screen stack, results (ADR-0018), `Ctx`. The short path first
     (`BasicGame`, `BasicData` with its own part, `UiAction` as the
     action set), then `GameData` by hand for a game that outgrows it.
     Text by key: the game's English, the engine's English beneath it,
     and how a game overrides an engine key, if 2351 is done (if not,
     2351 adds that paragraph, as its clause says).
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
     (`frame-png`), and, if 2112 is done, proving a level winnable with
     `solve` (a snippet in `site/snippets/`; if not, 2112 adds that
     section).
  9. **Targets and export:** target profiles, `shuyi check --target`,
     the web build, and, if 3050 is done, releasing from a tag (the
     template README's "Releasing" section; if not, 3050 adds that
     section).
  10. **Tools for agents:** the tool contract (`docs/contract.md`),
      and the MCP server and skill pack once 1133 and 1134 are done
      (until then, a line saying they are coming). The problem codes
      and `shuyi explain` (2509): a page per code under
      `/learn/errors/`, written from `crates/cli/errors/*.md` into
      `site/content/learn/errors/` before `zola build` (so 2808's
      twins cover them), and this chapter links the list; if
      2509 isn't done, a line saying they are coming.
  11. **Coming from another engine:** a table that maps words from
      Godot, Unity, Bevy, GameMaker and PICO-8 to Shuyi's, each row
      linked to the chapter that explains it. For example: a Godot
      scene, a Unity scene or a GameMaker room is a `Screen` on the
      screen stack; a node, a GameObject, a Bevy entity or a GameMaker
      instance is a plain struct the game owns, in a screen or in the
      state its `TurnRules` changes (the arena of ADR-0003 §4 comes
      with the parked tick path), or an entity of `shuyi-ecs` once 2107
      is done; an input map or input actions are an `ActionSet` and the
      player's keymap; `_process`, `Update()`, a Bevy system or a Step
      event is `Screen::update`, with state changed through commands
      (`TurnRules`); resources and imports are kinds and formats
      through the registry. Check each name against the code when
      writing it; a word with no counterpart says so.
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
  - the book's chapters get their row in the docs map,
    `docs/README.md` (2810), under *People making games*: its home is
    `site/content/learn/book/` and `site/snippets/`, updated by "a
    change that adds or changes something public". Its *Pairs* table
    gains the rustdoc and the chapter of each public concept. There is
    no `work-ticket` step of its own: §5's *Write the docs* (2810)
    reaches the chapter through the map. The *What each task reads*
    table names the chapter for each ticket block it covers;
  - the write-ticket template's acceptance criteria gain the line "The
    book's chapter is updated (2802), or the completion notes say why
    not".
- **2004's guides** (one per direction) go in the book as a part after
  chapter 11 when 2004 is done; 2004 points here.
- **The compiler messages on the engine's traits (2510):** if 2510 is
  done, the links in its `note`s point at source files on GitHub;
  change them to the book's chapters, and the `//~` texts of its
  fixtures (`crates/xtask/compile-messages/`) with them. If 2510 isn't
  done, nothing here (2510 links the chapters itself).

**Out (do not do):**
- the tutorial (2803);
- translations of the book;
- `llms.txt`, `llms-full.txt` and a Markdown twin of each Learn page
  (2808). Any shortcode in Learn other than `snippet` will need an
  expansion in 2808's `cargo xtask llms`;
- chapters for modules that don't exist yet (each module's ticket adds
  its own).

## Implementation steps

1. Read the README, the ADRs named above, and `docs/contract.md`.
   Check which of 2351, 2509, 2510 and 3050 are done.
2. Write the snippets crate and the shortcode, with one chapter's
   snippets.
3. Write the chapters, each with its snippets.
4. Add `cargo doc` to `pages.yml`, and the Learn links.
5. Add the book's row to `docs/README.md`, and its line to the ticket template.
6. Run the gates.

## Acceptance criteria

- [ ] Eleven chapters are live under `/learn/book/`.
- [ ] Every code block in the book comes from `site/snippets/` through
      the shortcode (a CI grep finds no fenced Rust block in
      `site/content/learn/book/` that isn't a shortcode).
- [ ] Renaming a public function a snippet uses fails CI.
- [ ] `/api/` holds each workspace crate's rustdoc.
- [ ] `docs/README.md` has the book's row and pairs, and `tickets/TEMPLATE.md` carries the rule.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The snippets crate's tests run in CI with the workspace's.
- `zola check` passes with the new pages.

## Completion notes

*(Filled in by the session that completes the ticket.)*
