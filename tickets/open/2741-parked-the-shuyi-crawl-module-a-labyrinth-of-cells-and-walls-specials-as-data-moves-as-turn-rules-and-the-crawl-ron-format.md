---
id: "2741"
title: "Parked: The shuyi-crawl module: a labyrinth of cells and walls, specials as data, moves as turn rules, and the crawl-ron format"
type: feature
milestone: "Engine: dungeon crawlers"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["2003", "2116"]
nick_input: none
completed:
---

# 2741 — The shuyi-crawl module: a labyrinth of cells and walls

> **Parked** (Nick, 2026-10-22: "if we can have tickets to add support for this type of game too"): like the other kinds of game, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-22 (`docs/decisions.md`): first-person dungeon crawlers
in the style of Wizardry and Wizardry Variants Daphne, a party of one
most of all, any size allowed.
[ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
sets the design. This ticket is its §1 (the module), §2 (the map) and
§3 (moving).

What it builds on:

- **The grid kit**: `shuyi_core::grid` (`Pos`, `Dir`, `Grid<T>`). A
  crawl floor is a `Grid<Cell>` plus its edges.
- **Turn rules** (ADR-0003 §2, `shuyi_core::turn::TurnRules`) and the
  seeded RNG (`shuyi_core::rng::SimRng`).
- **Formats are plugins** ([ADR-0007](../../docs/adr/0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md),
  [ADR-0019](../../docs/adr/0019-the-format-registry-kinds-are-types-a-format-reads-one-file.md)):
  every new file kind is a `Format`, never panics, and is deterministic.
  Its determinism fixture is a row in a golden table (2116's
  `shuyi_assets::golden`).
- **Module duties** ([ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
  §3–§4), the catalog and the `shuyi` crate (2003), and the layers
  file ([ADR-0032](../../docs/adr/0032-the-layers-as-data-checked-over-cargo-metadata-a-float-ban-in-the-simulations-and-a-new-games-rules-crate-kept-pure.md)).

**Either order with 2218:** if 2218 is done, write the `crawl-ron`
writer here, and its round-trip test (read, write, read gives the same
map and the same text). If not, 2218 adds it with the other writers.

The view is 2742, the 2D skins 2743, the automap 2744, the 3D skin 2753,
the example 2754, the explorer bot 2755. The party, battles and items
are `shuyi-rpg` (2745–2752).

**Who uses it** (ADR-0033 §17's bar for engine work): first-person crawler RPGs (Wizardry, Etrian Odyssey, Labyrinth of Refrain), first-person labyrinth puzzle games with no fights, and escape-room and horror games that step through rooms on a grid. A real-time crawler later reuses the map (ADR-0033, *Consequences*).

## Nick input

None.

## Scope

**In:**
- **The crate** `crates/crawl` (`shuyi-crawl`), with ADR-0009 §4's
  duties:
  - it depends on `shuyi-core` and `shuyi-assets` only. Empty `draw`
    and `3d` features are filled by 2743 and 2753, so a headless tool
    uses the rules without drawing;
  - metadata: `concern`/`answer` pairs `kind`/`dungeon-crawler` and
    `perspective`/`first-person grid`; `serves` the first-person grid
    perspective; `alternatives` empty; `works_with` empty for now;
  - its line in `tools/layers.ron` (pure: no devices, no floats);
  - the `crawl` feature of the `shuyi` crate, and the catalog row in
    `docs/modules.md` set to building with tickets 2741–2744, 2753–2755;
  - in `docs/modules.ron`: the `dungeon-crawler` answer of `kind` and
    the `first-person grid` answer of the Perspective concern, from
    parked to building;
  - the README's first two sections in ADR-0009 §4's order. 2754
    finishes it;
  - the `shuyi-crawl` skill: if 1134 is done, a short skill saying
    what this ticket built and what 2742–2755 add. If not, add
    `shuyi-crawl` to 1134's list;
  - if 2004's guide check is in place, a planned page
    `docs/guides/dungeon-crawler.md` naming 2742–2755. 2754 writes it.
- **The marker** `FirstPersonGrid` in `shuyi_crawl::perspective`,
  implementing `shuyi_core::perspective::Perspective` (ADR-0009 §3). If
  the trait doesn't exist yet (1120 is parked), add the trait with this
  one marker in `shuyi_core::perspective`, as ADR-0009 §3 describes it,
  and say so in the Completion notes.
- **The map** in `shuyi_crawl::map`:
  - `Facing`: the four ways, with `left()`, `right()`, `back()`, and
    `to_dir()`/`from_dir()` to the grid kit's `Dir` (north is `Dir::Up`);
  - `EdgeKind::{Open, Wall, Door { lock: Option<KeyId> }, Secret { lock:
    Option<KeyId> }, OneWay(Facing)}`, and `Edge { kind, look: u16 }`.
    `KeyId` is a string id the game checks;
  - `Cell { look: u16, area: AreaId, specials: Vec<Special> }`;
  - `Special::{Spinner, TurnTo(Facing), Teleport { floor, pos, facing },
    Chute, Stairs { to: Vec<(FloorId, Pos)> }, Pit, Dark, Zone(ZoneId),
    Message { text: TextKey, once: bool }, Event(EventId),
    Encounter(MarkId)}`;
  - `Floor { id, cells: Grid<Cell>, edges }`: the edges between cells
    and around the border, stored as a horizontal and a vertical grid of
    `Edge`. `floor.edge(pos, facing) -> Edge` reads the edge on that side
    of a cell; the edge between two cells is one value, seen from both;
  - `CrawlMap { floors }`, built by `CrawlMap::build(...) -> Result<Self,
    Vec<ContentError>>`, which checks every teleport and stairs target
    exists and is inside its floor;
  - `hash()` over stable bytes for each type.
- **The format** `*.crawl.ron`, asset kind `CrawlMap`, format
  `crawl-ron`:
  - per floor: `id`, `layout` (a multi-line string as ADR-0033 §2
    draws it), `legend` (a map from a two-character cell code to its
    specials and area), and optional `looks` (a map from a cell code or
    an edge character to a look index; default look 0);
  - the layout's grammar: corners `+`; horizontal edges two characters
    (`--` wall, two spaces open, `dd` door, `ss` secret, `^^` or `vv`
    one-way toward that side); vertical edges one character (`|`, space,
    `d`, `s`, `>`, `<`); a cell's two inside characters a legend code,
    or two spaces for a plain cell. A locked door names its key in the
    legend of a cell beside it (`lock: "east": "iron-key"`);
  - errors name the file, the line and the column: a ragged row, a
    missing corner, an unknown code, an edge character in the wrong
    place, a teleport out of bounds;
  - `deny_unknown_fields`; a fuzz-style test (random bytes and random
    layouts never panic); a determinism fixture as a row of
    `crates/crawl/tests/golden/formats.ron` (2116);
  - registered by the module's `register(&mut Formats)`;
  - a page `docs/formats/crawl.md` (or a section of `docs/formats.md`,
    where the docs map says format pages go) with the grammar and one
    small map.
- **The explorer** in `shuyi_crawl::explore`:
  - `Explorer { floor, pos, facing, effects: Vec<(EffectId, u32)>,
    steps: u64, opened: BTreeSet<EdgeRef>, found: BTreeSet<EdgeRef>,
    flags: BTreeSet<EventId> }` (the steps an effect lasts, doors
    opened, secrets found, one-time messages and events done). The seen
    set is 2744's;
  - effects the module knows: `light` (2742 reads it), `float` (pits
    and chutes do nothing), `compass` (a flag the skin shows); a game's
    own by name;
  - a text dump, one fact per line, and `hash()`.
- **`CrawlRules: TurnRules`** in `shuyi_crawl::rules`, over a map, an
  explorer and a `SimRng`:
  - commands `Forward`, `Back`, `StrafeLeft`, `StrafeRight`,
    `TurnLeft`, `TurnRight`, `TurnAround`, `Open`, `Search`,
    `Take(index)` (a stairs or elevator choice), `Wait`;
  - `check` refuses a step through a wall, a closed door, an unfound
    secret or a one-way edge against its way (`CrawlError::Blocked`),
    `Open` with no door in front or a lock the game says is shut (a
    `CrawlHooks::has_key(&KeyId) -> bool`), `Take` with no stairs or out
    of range. An error changes nothing;
  - `apply` returns `CrawlEvent`s: `Moved`, `Turned`, `Spun { to }`,
    `Teleported`, `Fell { to_floor }`, `Entered { area, specials }`,
    `Message(TextKey)`, `Event(EventId)`, `Encounter(MarkId)`, `Pit`,
    `DoorOpened`, `Found(EdgeRef)`, `Ended(EffectId)`;
  - each step or turn counts down the effects;
  - `Search` finds an unfound secret edge in front with the chance from
    `CrawlHooks::search_chance()` (a `u8` out of 256), on the RNG;
  - spinners pick a facing from the RNG and say so only in `Spun`;
  - `legal_commands()` lists exactly the commands `check` accepts, in a
    fixed order; `outcome()` is `None` (a labyrinth has no end of its
    own; a game's rules decide);
  - `CrawlHooks` is a trait with defaults (no keys, a chance of 64), so
    a test needs no game.
- **The step as a function**: `explore::apply(map, &mut explorer, &mut
  rng, &hooks, command) -> Result<Vec<CrawlEvent>, CrawlError>`, which
  `CrawlRules` calls, so a game's own `TurnRules` holds the explorer
  beside its party and battles (ADR-0033 §9).
- **CI**: `-p shuyi-crawl` joins the wasm32 `cargo test` of the
  `determinism` job (2116).

**Out (do not do):**
- the view (2742), the skins (2743, 2753), the automap and travel
  (2744), the example (2754), the explorer bot (2755);
- the party, battles, encounters and items (`shuyi-rpg`, 2745–2752):
  this module only reports `Encounter(mark)` and `Entered { area }`;
- importing Tiled or LDtk maps (ADR-0033 §2);
- generating floors (2119 makes grids of cells; turning them into a
  crawl floor is a ticket of its own if a game asks);
- real-time movement (ADR-0033, *Consequences*).

## Implementation steps

1. Make the crate, its metadata, its layers line, the `shuyi` feature,
   the catalog rows and the README's first sections.
2. Add the marker (and the trait, if missing).
3. Write `map`: the types, `edge()`, `build()`, hashes, with unit tests
   of `edge()` from both sides.
4. Write the layout parser and the format, its errors, the fuzz-style
   test and the golden row; the docs page.
5. Write `explore` and `rules`, with tests for every special and every
   refusal.
6. Add the crate to the wasm32 determinism job.

## Acceptance criteria

- [ ] A three-floor test map in `crawl-ron` loads and every special and
      edge kind in it behaves as ADR-0033 §2–§3 says (one test each).
- [ ] A step into a wall, a closed door, an unfound secret and a
      one-way edge the wrong way is refused and changes nothing (the
      hash is the same).
- [ ] `legal_commands()` equals the commands `check` accepts, on every
      cell of the test map and every facing (a property test).
- [ ] A spinner's facing comes from the seed: the same seed and
      commands give the same explorer hash.
- [ ] Format errors name file, line and column (one test per error).
- [ ] The golden row passes on x64 and wasm32.
- [ ] `cargo xtask check-layers` passes with the new line.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Facing`, `edge()` from both sides, the layout parser's errors,
  each special, each refusal, effects counting down.
- Property: `legal_commands` agrees with `check`; random layouts never
  panic; a parsed map written and read again (if 2218 is done) is equal.
- Integration: the golden table on native and wasm32.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
