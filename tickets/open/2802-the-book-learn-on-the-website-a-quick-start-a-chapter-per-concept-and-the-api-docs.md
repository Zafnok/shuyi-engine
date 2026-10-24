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
     --git https://github.com/Zafnok/shuyi-engine shuyi-cli` (crates.io
     has only placeholders before 2005), `shuyi new blank` and what
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
     skins, why tests read the scene (`AGENTS.md` rule 9), and, if 2109
     is done, ease curves and tweens drawn from the time alone
     (`shuyi_core::ease`, a snippet in `site/snippets/`; if not, 2109
     adds that section, as its clause says), with `Sequence` and
     `Points` if 2135 is done (if not, 2135 adds them, as its clause
     says). If 2362 is done, the camera file (`camera.ron`); if not,
     2362 adds that section, as its clause says.
     If 2329 is done, a section on laying widgets out with
     `shuyi_ui::layout`; if not, 2329 adds it, as its clause says.
     If 2334 is done, a short section on text runs
     (`GlyphBuffer::add_text`, `TextStyle`, fonts by id); if not, 2334
     adds that section, as its clause says.
     If 2336 is done, a short section on drawn shapes (lines, circles
     and polygons); if 2337 is done, one with a tinted sprite and an
     additive one (if not, each adds its section, as its clause says).
  5. **Assets and formats:** kinds, formats, the registry, bundles and
     layers, packs (ADR-0007, ADR-0019, ADR-0023), and, if 2220 is done,
     hot reload (`Hot<T>`, `shuyi run --watch`; if not, 2220 adds that
     section, as its clause says). If 2232 is done, a link to its
     section "Levels from Tiled and LDtk" of `docs/formats.md` (if
     not, 2232 adds that link, as its clause says).
  6. **Rules as commands:** turn games, `shuyi_core::turn`, history and
     undo, determinism and the hash, and, if 2115 is done, time in steps
     or turns (`shuyi_core::timer`; if not, 2115 adds that section).
  7. **Saves and settings:** `Storage`, profiles, settings, and, if
     2110 is done, save versions: `VERSION`, the `Migrate` steps and
     the `v<N>_*.ron` fixtures `check_saved_forms` checks, written from
     2110's ADR (if not, 2110 adds that section, as its clause says).
     If 2221 is done, scenes: a world saved to a `*.scene.ron` file,
     loaded back with its ids for a save, and instanced with new ids
     (if not, 2221 adds that section, as its clause says).
  8. **Testing:** the harness, replays, bots, snapshot tests of frames
     (`frame-png`), and, if 2112 is done, proving a level winnable with
     `solve` (a snippet in `site/snippets/`; if not, 2112 adds that
     section). **Either order with 2519:** if it is done, the chapter
     covers session records from its `docs/sessions.md` (where debug
     builds keep them, saving one from the debug menu, playing one
     with `shuyi run --session`, a golden session); that page and its
     row in `docs/README.md` go, and links to it point at the chapter.
     If not, nothing to do (2519 writes that section of the chapter).
  9. **Targets and export:** target profiles, `shuyi check --target`,
     the web build, and, if 3050 is done, releasing from a tag (the
     template README's "Releasing" section; if not, 3050 adds that
     section), and, if 3051 is done, a short "Signing" section that
     links `docs/signing.md` (if not, 3051 adds it).
  10. **Tools for agents:** the tool contract (`docs/contract.md`),
      and the MCP server (1133, done): `shuyi mcp`, set up with
      `shuyi mcp install`, and its session tools (`start`, `step`,
      `scene`, `frame_png`, `events`, `reset`, `stop`), from
      `docs/contract.md`'s *The MCP server* and ADR-0031.
      **From 1134 (done):** the chapter teaches its skill pack: the
      Claude Code plugin (`.claude-plugin/marketplace.json`, whose
      plugin is `skills/`), the common skill installers, and the skills
      `shuyi new` copies into a game's `.claude/skills/` (`shuyi add`
      copies a module's). The problem codes
      and `shuyi explain` (2509): a page per code under
      `/learn/errors/`, written from `crates/cli/errors/*.md` into
      `site/content/learn/errors/` before `zola build` (so 2808's
      twins cover them), and this chapter links the list.
  11. **Coming from another engine:** a table that maps words from
      Godot, Unity, Bevy, GameMaker and PICO-8 to Shuyi's, each row
      linked to the chapter that explains it. For example: a Godot
      scene, a Unity scene or a GameMaker room is a `Screen` on the
      screen stack; a node, a GameObject, a Bevy entity or a GameMaker
      instance is a plain struct the game owns, in a screen or in the
      state its `TurnRules` changes (the arena of ADR-0003 §4 comes
      with the parked tick path), or an entity of `shuyi-ecs` if 2108
      is done (if not, 2108 adds it with its chapter "An optional
      ECS"); an input map or input actions are an `ActionSet` and the
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
  copied to `/api/`. Learn links each crate's page (docs.rs links come
  with 2005, under Out).
- **Learn and the home page (2801):** 2801's Learn page names the
  book and the API docs without links, and its "Get started" button
  goes to Learn. Link the book and `/api/` from the Learn page, and
  point the button at the quick start.
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
- **Fast compiles (2009):** if 2009 is done, move its "Fast compiles"
  section from `README.md` into a page of the book (the numbers, the
  setup and the nightly-only options), link it from the quick start,
  and leave a link in the README in its place. If not, nothing here
  (2009 writes the page in the book).
- **Either order with 2108:** if 2108 is done, add a twelfth chapter,
  "An optional ECS", after chapter 11 and before 2004's guides:
  `shuyi-ecs`, `EcsRules`, the parallel schedule and `ecs-garden`, with
  its snippets; if not, nothing to do (2108 writes it).
- **Either order with 2338:** if 2338 is done, chapter 4 gets a short
  section on nine-slice and tiled pictures and picture frames; if not,
  nothing to do (2338 adds it).
- **Either order with 2528:** if 2528 is done, where the book tells
  the reader to try the web build (the quick start's `shuyi web` and
  chapter 9), it builds, serves and opens it in one step with `cargo
  shuyi web --serve --open`. If not, nothing to do (2528 changes those
  lines when it lands, as its clause says).

**Out (do not do):**
- the tutorial (2803);
- translations of the book;
- `llms.txt`, `llms-full.txt` and a Markdown twin of each Learn page
  (2808). Any shortcode in Learn other than `snippet` will need an
  expansion in 2808's `cargo xtask llms`;
- the quick start's crates.io install and Learn's docs.rs links: 2005
  makes both when it publishes, after this ticket (2005 is blocked by
  2808, which is blocked by this);
- chapters for modules that don't exist yet (each module's ticket adds
  its own).

## Implementation steps

1. Read the README, the ADRs named above, and `docs/contract.md`.
   Check which of 2009, 2109, 2334, 2351, 2510 and 3050 are done.
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
