---
id: "2269"
title: "BasicGame reloads its text, palette, keymap and its own part (maps, dialogue) under run --watch with no game code"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2269 — BasicGame reloads its own files under run --watch

## Context

2220 (done, [ADR-0071](../../docs/adr/0071-hot-reload-a-disk-layers-bytes-replaced-values-that-remember-what-they-read-and-a-module-hook.md))
built hot reload for any asset kind: `shuyi_assets::hot::Hot<T>`
(`crates/assets/src/hot.rs`) remembers what each decode read, and a
module that declares `Hook::Reloaded` refreshes it in
`Module::reloaded` and reports with `Ctx::note_reload`. 2242 (done)
redraws a changed PNG on its own. But a value reloads only if a module
the game writes keeps it as a `Hot` and refreshes it.

A search of the repository (`grep -rn "Hot<" crates examples
templates`) finds one such module: `examples/hello-grid/src/hot_palette.rs`
(and the dialogue preview's `DialogueFolder`, which reloads only a
preview's folder). The short path (ADR-0036) has none:

- `BasicData::load` (`crates/ui/src/basic.rs`) reads `en.ron`,
  `keymap.ron`, `palette.ron`, the font atlas, the picture table and the
  game's own part (`Own<T>`, a `fn(&Bundle)`) once, with
  `Formats::load`, and keeps plain values;
- `BasicGame` (`crates/platform/src/basic.rs`) lays the watched folder
  over the files (`BasicGame::over`), but installs no reload module;
- the template (`templates/shuyi-default`) keeps its stage's map in its
  own part (`stages::own` reads `stage.map`), and its `AGENTS.md` says
  "a module of the game's reloads what it keeps as a `Hot` value".

So under `cargo shuyi run --watch` a new picture colour shows, but a
changed line of text, colour, key binding or map shows nothing until
the game restarts. Every game made with `shuyi new` would write the same
glue. This ticket makes the short path reload its own data with no game
code. It came from the engine blind-spot audit of 2026-11-23 (item 28).

The game (pinned at engine revision `50472bbc`) isn't on `BasicData`
(`gh search code --repo Zafnok/visions-of-shuyi 'BasicData'` finds only
its copies of the engine's skills and `docs/engine/pins.md`, which says
"the game isn't on `BasicData`"), so nothing in the game changes.

## Nick input

None.

## Scope

**In:**
- `BasicData::reload(&mut self, changed: &[String]) -> Vec<(&'static str, Reload)>`
  in `crates/ui/src/basic.rs`: for each of `en.ron`, `keymap.ron`,
  `palette.ron` and the own part whose files are among `changed`, decode
  again with `BasicData::formats()`, with the same checks as `load`
  (`check_key_placeholders` for the English, the engine's colours for
  the palette). A good decode replaces the value. A failed one keeps the
  old value and gives `Reload::Failed` with its errors. It needs the
  paths each value read: record them at `load` with
  `shuyi_assets::bundle::recording` (public), or keep the three standard
  files as `Hot` values; the own part, read by a plain `fn(&Bundle)`,
  is recorded with `recording`.
- A built-in module, `BasicReload`, in `crates/ui/src/basic.rs` (it
  needs `BasicData`'s private fields and the crate's `Ctx::set_widths`):
  it declares `Hook::Reloaded`, calls `BasicData::reload`, and applies
  each good reload to the context:
  - the English: the language widths again (as `Ctx::set_widths` does
    with the widths it has), then a redraw;
  - the palette: `ctx.palette = Palette::new::<EngineColor>(…)`; a
    palette that lacks a colour is a failed reload, old colours kept;
  - the keymap: `ctx.refresh_keymap()`, so the player's own bindings and
    chosen layout stay;
  - every outcome goes to `Ctx::note_reload` under its file's path, so a
    broken file shows its error over the frame (2220's display).
- **The rule for the own part:** when it reloads, the game starts over
  at its first screen on the same context. Settings, profile, layout
  and player bindings stay; every screen's state is dropped. That is
  the one rule a map or a dialogue file can always obey: a screen built
  from the old map (the template's `Stage`) can't be patched in place.
  It needs a new request on the context, additive:
  `Ctx::restart(first: fn(&mut Ctx<G>) -> Box<dyn Screen<G>>)`, which
  `Game::reloaded` honours after every module's hook has run (it drops
  the stack and opens the screen `first` makes). `BasicReload` holds the
  game's `First<A, T>` and asks for it.
- `BasicGame::game` installs `BasicReload` before the game's own modules
  (`BasicGame::modules`), in a build with debug tools
  (`shuyi_ui::DEBUG_TOOLS`), so a shipped build lists no new module.
  `Game::reloaded` is only called there, or when a platform watches
  files (`tell_changed`, `crates/platform/src/app.rs`).
- The template's `AGENTS.md` row for `cargo shuyi run --watch`, the
  *Hot reload* section of `docs/formats.md`, the short path in
  `docs/templates.md` and the book's *Reloading while the game runs*
  (`site/content/learn/book/assets-and-formats.md`) say what reloads on
  its own and the restart rule.
- The new public items: `BasicData::reload` and `Ctx::restart` are
  methods; `BasicReload` is a struct with private fields and
  `BasicReload::new(first)`. No public type gains a field (ADR-0039).
- **Either order with 3414:** if 3414 is done, the `BasicReload` module
  also runs in an MCP session over 3414's disk layer: add a test that a
  text edit shows in the next `step`'s frame and is listed in its
  `reloads`; if not, nothing to do (3414 adds that test when it lands).

**Out (do not do):**
- The font atlas (`font/atlas.ron`) and the picture table: a changed
  picture already redraws (2242); a new picture or a new atlas
  description still needs a restart. Say so in the docs.
- Keeping a screen's place across an own-part reload (rebuild and
  replay to the same moment): 2905's research.
- `.ui.ron` documents (2330 reloads them with its own `Hot<UiDoc>`) and
  scene files (2221).
- Reload on the web (2528 rebuilds and reloads the page) and in the
  headless tool program and MCP session, which read embedded bytes
  (2270 adds a disk path there, and 3414 the session's; this ticket
  needs only `run --watch`'s layer).
- A game's own `GameData` off the short path: it keeps writing its own
  module, as Hello, Grid does.

**Estimated size:** about 900 lines (code 350, tests 400, docs 150).

## Implementation steps

1. Read ADR-0071, `crates/assets/src/hot.rs`, `crates/ui/src/basic.rs`,
   `crates/ui/src/ctx.rs` (`Ctx::new`, `refresh_keymap`, `set_widths`,
   `note_reload`), `crates/ui/src/game.rs` (`Game::reloaded`),
   `crates/platform/src/basic.rs` and `examples/hello-grid/src/hot_palette.rs`.
2. Record the paths each of `BasicData`'s values read at `load`, and
   add `BasicData::reload`.
3. Add `Ctx::restart` and have `Game::reloaded` honour it after the
   hooks (a request made outside a reload is honoured at the next
   frame's start, or refused; pick one and document it).
4. Add `BasicReload` and install it in `BasicGame::game` with debug
   tools on.
5. Record the restart rule with the `write-adr` skill (a short ADR that
   amends ADR-0071 for the short path), update the docs listed in the
   Scope, and run the gates.

## Acceptance criteria

- [ ] Through `BasicGame::harness()` over an owned layer
      (`BasicGame::over`, `Harness::change_file`): a changed `en.ron`
      line shows in the next frame's buffer; a changed `palette.ron`
      colour is the context's palette on the next frame; a changed
      `keymap.ron` binding works on the next frame while a binding the
      player changed stays.
- [ ] A changed file the own part read (the template stage's
      `stage.map`) reloads the own part, and the game is at its first
      screen on the next frame with the same layout and settings.
- [ ] A changed file nothing read reloads nothing, and the screens keep
      their state (the stack's state hash is unchanged).
- [ ] A broken `en.ron`, `palette.ron` (one lacking an engine colour),
      `keymap.ron` or map keeps the old value, the game keeps running
      on the same screen, and `Ctx::reload_errors` names the file and
      line; a good save clears it.
- [ ] A build with debug tools off installs no `BasicReload`
      (`Game::modules` lists only the game's).
- [ ] A game made by `shuyi new` reloads its text and map under
      `cargo shuyi run --watch` with no code of its own (checked by
      hand; say so in the completion notes).
- [ ] The completion notes have a `Behaviour:` line: a short-path game
      with debug tools lists the `basic_reload` module and reloads its
      files under `run --watch`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `BasicData::reload` for each file, each outcome (unchanged,
  reloaded, failed), and the own part's recorded reads.
- Unit: `Ctx::restart` honoured after every module's hook, once.
- Integration: the harness tests above, in `crates/platform/tests/it/`
  beside `reload.rs`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
