---
id: "2290"
title: "A map's look checked against the lights and the tilesets: a light that isn't one, a prop no look draws, each at its place in the header"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2290 — A map's look checked against the lights and the tilesets

## Context

A request from the game.

1. **The game ticket:** game#1409, the game's map checks on these
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1409-map-look-checks-from-the-engine.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** report every map whose header names a
   light the game doesn't have, or places a prop on a tile that its
   tileset's look has no picture for, each error at the place in the
   header that names it.
3. **The friction numbers:** the game's third engine-friction sweep
   (game#1397, 2026-11-27): `crates/content/src/map.rs` lines 162–260
   (`check_lights`, `check_props`, `prop_error`, `light_error`), about
   95 production lines. Every type they read is the engine's:
   `MapLook::light` and `MapLook::props` (`grid_map.rs`), `Lighting`
   (`lighting.rs`), a tileset's `Look::props` (`tileset.rs`), and
   `grid_map::header_lines` and `header_position` for the place. The
   game would keep about 10 (the call with its maps and tilesets): about
   9×. `MapLook::props`' own doc says the names are "checked against its
   tilesets by the game", which is the gap.
4. **The generic shape**, in `crates/assets/src/grid_map.rs`:
   - `MapLook::light_problem(&self, lighting: &Lighting) ->
     Option<String>` and `MapLook::prop_problems(&self, look: &Look) ->
     Vec<((i32, i32), String)>`: the plain checks;
   - `check_looks(maps, lighting, looks) -> Vec<ContentError>`: over
     every map (its file name and source), each problem at its line and
     column in the header (the light's name; the prop's `(x, y)` inside
     `props`, either spacing), else at the file's start. `looks` gives
     the tileset looks a map's `tiles` name; a tileset without that look
     isn't asked.
   - The messages list what exists (`the lights are …`, `its props:
     …`), as the game's do.
5. **Examples from games that aren't tactics:** a farming game's map of
   a village at dusk that names its `dusk` light and places a `lamp`
   and a `well` by hand: a typo in either is caught at its place when
   the map loads.

## Nick input

None.

## Scope

**In:**
- The two methods, `check_looks`, their errors at the header's place,
  and docs; `MapLook::props`' doc points to them.
- Nothing changes for a game that doesn't call them.

**Out (do not do):**
- Checking a map's terrain or legend (already the map format's).
- Lights or props of other formats than the `.map` header.
- The game's use (game#1409).

## Implementation steps

1. **Read** `crates/assets/src/grid_map.rs` (`MapLook`, `header_lines`,
   `header_position`), `lighting.rs` (`Lighting::get`, `names`) and
   `tileset.rs` (`Look::props`); the game's `crates/content/src/map.rs`
   at its pin shows the messages and the placing.
2. The two methods with tests.
3. `check_looks` with the header positions, with tests on a map whose
   `props` spans lines.
4. Docs.

## Acceptance criteria

- [ ] A map naming an unknown light gives one error at the light's
      name in the header, listing the lights (unit test).
- [ ] A map placing `"lamp"` at `(19, 5)` on a look without `lamp`
      gives one error at `(19, 5)` (and at `(19,5)` when written without
      the space), listing the look's props.
- [ ] A tileset without the map's look gives no error for it.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: both methods and `check_looks`, the header positions.
- Property: none.
- Snapshot / integration: none.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
