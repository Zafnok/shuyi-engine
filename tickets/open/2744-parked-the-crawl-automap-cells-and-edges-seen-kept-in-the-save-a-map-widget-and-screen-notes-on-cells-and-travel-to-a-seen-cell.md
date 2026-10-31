---
id: "2744"
title: "Parked: The crawl automap: cells and edges seen kept in the save, a map widget and screen, notes on cells, and travel to a seen cell"
type: feature
milestone: "Engine: dungeon crawlers"
model: implementation
effort: high
status: blocked
blocked_by: ["2742"]
nick_input: none
completed:
---

# 2744 — The crawl automap and travel

> **Parked** (Nick, 2026-10-22: "if we can have tickets to add support for this type of game too"): like the other kinds of game, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
§6. Modern crawlers draw the map as the party walks; Wizardry hid it
behind a spell; Daphne lets the player mark a spot and walk there. The
game picks which (it opens the map screen, or doesn't).

It builds on 2742 (the view, which says what was seen) and 2741 (the
explorer). Paths use `shuyi_core::path`, which today walks cells with a
cost per cell; walls between cells need a neighbour function that reads
edges.

**Who uses it** (ADR-0033 §17's bar for engine work): crawler RPGs and labyrinth puzzle games; travel and `frontier` also serve the explorer bot (2755). A game that hides the map (Wizardry's spell) simply doesn't open the screen.

**The map screen's cursor** walks the floor grid as the tactics map's
cursor walks its map: the grid kit's `shuyi_ui::grid::Cursor` (one cell
a step, never off the map), or one of its own if that one doesn't fit.

**Either order with 2743:** if 2743 is done, the widget and the map
screen go behind its `draw` feature, which already pulls in
`shuyi-ui`, and travel sends its next command when a step's motion
ends (each frame when `motion_ms` is 0). If not, make `draw` pull in
`shuyi-ui` here, with its line in `tools/layers.ron`, and send one
command a frame; 2743 makes travel wait for its motion when it lands.

**Either order with 2336:** if 2336 is done, `MapWidget` draws walls,
doors, one-way arrows and the explorer's arrow with its line shapes; if
not, with glyph cells and Overlay rectangles, and nothing is left for
2336.

## Nick input

None.

## Scope

**In:**
- **What was seen**, in `shuyi_crawl::seen`:
  - `Seen { cells: per floor a bit grid, edges: per floor a bit grid,
    walked: per floor a bit grid }`, kept in `Explorer` (2741), so it is
    in the hash and the save;
  - after each command that moves or turns, `CrawlRules` (and
    `explore::apply`) marks every slot of the new view and the edges it
    showed as seen, and the cell walked as walked. A found secret is
    seen as a door;
  - `Seen` serialises compactly (bits, run-length), with a test that a
    50×50 floor fully seen stays under 1 KiB.
- **Notes**: `Notes { by cell: (mark: u8, text: String) }` per floor,
  the player's own. Commands `Note(pos, mark, text)` and `Unnote(pos)`
  in the explorer's rules, so they replay and save.
- **Paths through edges**: in `shuyi_core::path`, a general search over
  a neighbour function (`search_with(start, goal, neighbours: impl
  Fn(Pos) -> impl Iterator<Item = (Pos, u32)>)`), if the module has no
  such entry yet. `shuyi_crawl::travel::route(map, explorer, goal) ->
  Option<Vec<CrawlCommand>>`: a path over seen cells and seen, passable
  edges, as the turns and forward steps that walk it.
- **Travel**: `Travel` in `shuyi_crawl::travel` holds a route and gives
  the next command; the stage sends one command a frame, or one per
  step's motion (the clause on 2743 says which), and drops the route
  when an event other than `Moved`/`Turned` comes back (a spinner, an
  encounter, a message), or when a command is refused. Travel is never
  a command of its own, so a record of a run stays plain steps.
- **The widget** (behind `draw`): `MapWidget` paints a floor's seen
  cells and edges into a rectangle as glyph cells and Overlay
  rectangles (a grid map's walls run straight across or down), at a
  scale of whole pixels per cell: walls, doors, one-way arrows,
  specials met (stairs, a teleporter's mark), notes' marks, the
  explorer's arrow; unseen cells blank. Colours by palette role.
- **The map screen**: a screen with a cursor on the map (the grid
  kit's map cursor, or one of its own, on the floor grid), the
  floor list, the note under the cursor, actions to add a note and to
  travel to the cursor. Actions only (`input-actions` skill). It pops
  with `Travel` or nothing (ADR-0018).
- **A corner map**: the same widget at a small scale around the
  explorer, for a stage to draw beside the view.
- **Settings** in `crawl.ron`: `map: always | never` for the corner map,
  the cell scale.
- **A search for the bot**: `frontier(map, explorer) -> Vec<Pos>`, the
  seen cells beside unseen ones that a route reaches, nearest first.
  2755's explorer bot uses it; travel to the nearest is an "auto-explore"
  a game may offer.

**Out (do not do):**
- what opens the map (a spell, an item, always): the game's;
- map pictures beyond lines and glyphs;
- field of view for top-down games (2120);
- the explorer bot (2755);
- 2354's `GridCursor` for the map's cursor: it moves over a
  widget's rows, and a map cursor walks a map (2354's Out says so of
  the tactics map's).

## Implementation steps

1. `Seen`, marking from the view, its compact form and tests.
2. Notes and their commands.
3. `search_with` (if needed) and `route`; `Travel`; `frontier`.
4. `MapWidget`, the corner map, the map screen.
5. Snapshots.

## Acceptance criteria

- [ ] Walking a test floor's corridor marks exactly the cells and edges
      the views showed (a snapshot of `Seen` as text).
- [ ] `route` never crosses an unseen cell or edge, never passes a wall
      (a property test over random maps), and returns `None` when no
      seen way exists.
- [ ] Travel stops at a spinner and at an encounter mark (tests).
- [ ] The map screen pops with a travel to the cursor's cell (a Harness
      test).
- [ ] `frame-png` snapshots of the map screen and the corner map.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: marking; notes; `route`; `frontier`; `Seen`'s size.
- Property: routes are legal walks over seen cells.
- Integration: a Harness test of the map screen and of travel.
- Snapshot: the widget and screen.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
