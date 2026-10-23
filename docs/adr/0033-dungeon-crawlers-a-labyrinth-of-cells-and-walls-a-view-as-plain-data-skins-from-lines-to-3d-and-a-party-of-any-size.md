# ADR-0033: Dungeon crawlers: a labyrinth of cells and walls, a view as plain data, skins from lines to 3D, and a party of any size

- **Status:** Accepted
- **Date:** 2026-10-22
- **Related tickets:** 2741–2755, all parked until after Visions of
  Shuyi launches; 2003 (modules and the catalog), 2109 (tweens), 2114
  (RNG helpers), 2116 (determinism fixtures), 2218 (writers), 2336
  (drawn shapes), 2340 (reduced motion), 2501 (template parts), 3102–3104
  (glTF, the 3D view, the GPU painter)
- **Extends:** ADR-0003 §1–§2 (stages, turn rules), ADR-0007 (formats
  are plugins), ADR-0009 (modules, perspective markers), ADR-0011 §2
  and §5 (the *World drawing* concern, the 3D view), ADR-0015 §1 (the
  *Kind of game* concern)

## Context

Nick, 2026-10-22 (`docs/decisions.md`): "another game type I like a lot
is Wizardry first person dungeon crawler, especially the intro to
Variant Daphne... what I would want most is a single player party
blobber (i.e. 1 character only not 4 or 6 party) but anyway support
for any character amount is ok".

What these games are made of, from Wizardry I–V, Wizardry Variants
Daphne, the Etrian Odyssey series, Eye of the Beholder and Labyrinth of
Refrain:

- **A labyrinth on a square grid whose walls sit between cells**, not
  on them. Doors, one-way doors, locked doors and secret doors are
  edges too. The party faces one of four ways, steps, strafes and
  turns.
- **Special cells**: spinners that turn the party without telling it,
  teleporters, pits and chutes, darkness, zones where magic fails,
  stairs and elevators, messages, fixed fights, one-time events.
- **A first-person view** of that grid. Three looks are classic:
  Wizardry I's wireframe of lines; pictures painted per wall position
  (Eye of the Beholder, the 2D Etrian games); and real 3D with smooth
  steps and turns (Daphne).
- **A map** the game draws as the party walks (or that only a spell
  shows), and in Daphne, travel to a marked spot.
- **A party** in rows (front and back), usually of up to six. Nick wants
  one character most of all; Daphne has a protagonist who never leaves
  and up to five others.
- **Characters made from rolled bonus points**, with callings open only
  above some numbers, kept in a roster in town.
- **Round-based battles seen from the front**: enemy groups at ranges,
  each member chooses an action, then everything resolves in an order
  from the numbers and luck. Enemies are unknown until identified.
- **Spells by level**, items to identify, cursed equipment, chests with
  traps to name and disarm.
- **A town of menus**: a tavern, an inn, a shop, a temple, the way down.
- **An opening.** Daphne's is a story scene (the protagonist is put to
  death), then waking in the labyrinth to a voice that speaks before
  the first step.

What the engine has: the grid kit (`shuyi_core::grid`: `Pos`, `Dir`,
`Grid<T>`, paths in `shuyi_core::path`), turn rules (ADR-0003 §2), the
bots, playtests and autobalance (ADR-0027, 2125–2130), menus, list
cursors, the question widget, name entry and profiles (2307–2310),
dialogue and cinematics, text by key, storage. Planned: drawn lines
(2336), tweens (2109), weighted picks (2114), the 3D view and its
painters (3102–3104, parked).

What it lacks: walls between cells, a view from inside the grid, and
everything of a party RPG. The engine's rules hold: deterministic turn
rules, a scene of plain data and a skin that paints it (`AGENTS.md`
rule 9), nothing of a game's content in the engine (rule 4), actions
not keys (rule 8), text as the source of truth (rule 10).

## Decision

### 1. Two modules, neither on top of the other

- **`shuyi-crawl`** (`crates/crawl`): the labyrinth. Its map, moving
  through it, the view from inside, the skins that paint the view, and
  the automap.
- **`shuyi-rpg`** (`crates/rpg`): a party RPG's parts. A party of
  any size, members' numbers as data, items, round-based battles,
  abilities, encounters and chests, characters made and kept, and the
  town hub.
- **Neither depends on the other.** A top-down RPG in the style of the
  early Dragon Quest games uses `shuyi-rpg` on the grid kit. A
  crawler uses both, and its game's rules join them (§9). A puzzle
  labyrinth with no fights uses `shuyi-crawl` alone. The party's
  sheets, items and abilities don't need its battles, so an action RPG
  (ADR-0034) uses them with fights of its own in real time.
- Both carry every module duty of ADR-0009 §4: metadata, a README in
  the fixed order, a feature of the `shuyi` crate (`crawl`, `rpg`), a
  catalog row, an example, a skill, a line in `tools/layers.ron`
  (ADR-0032), and `needs` (ADR-0012 §1).
- **Concerns** (ADR-0009 §2, ADR-0015 §1, ADR-0011 §2):

  | Concern | New answer | Module |
  | ------- | ---------- | ------ |
  | Kind of game | `dungeon-crawler` | `shuyi-crawl` (with `shuyi-rpg` in its example) |
  | Kind of game | `party-rpg` | `shuyi-rpg` |
  | Perspective | first-person grid, marker `FirstPersonGrid` in `shuyi_crawl::perspective` | `shuyi-crawl` |
  | World drawing | lines; pictures per slot; 3D labyrinth | `shuyi-crawl`'s `draw` and `3d` features |

- **First-person grid is a perspective of its own.** Its rules work in
  grid tiles, so the grid kit's paths work unchanged, but its input is
  relative to the facing (forward, turn left) and its picture is drawn
  from inside. It isn't ADR-0011's `FirstPerson`, which moves freely in
  3D.

### 2. The labyrinth is cells with edges

- **`CrawlMap`** in `shuyi_crawl::map`: floors, each a `Grid<Cell>` and
  the edges between cells. A cell holds its look (an index into the
  skin's table), an area name (for encounters, §11) and its specials.
  An edge holds its kind and its look:

  | Edge kind | Passes | Seen as |
  | --------- | ------ | ------- |
  | `Open` | both ways | nothing |
  | `Wall` | neither | a wall |
  | `Door { lock }` | both ways, once opened; a lock names a key the game checks | a door |
  | `Secret { lock }` | as a door, once found | a wall until found |
  | `OneWay(Dir)` | in that direction only | a door from the side that passes, a wall from the other |

- **Specials** as data, per cell. The module carries out the ones that
  move the party, and reports the rest as events:
  - moves: spinner (a facing from the seeded RNG, and no event says
    so), turn to a facing, teleporter (to a floor, cell and facing),
    chute (to the floor below), stairs and elevators (offered as
    commands, §3);
  - events: pit (the game decides the harm), darkness (the view sees
    nothing, §4), a zone the game names (for example where magic
    fails), a message (a text key, once or always), the game's own
    event by name, a fixed encounter mark (§11).
- **The format**: `*.crawl.ron`, asset kind `CrawlMap`, format
  `crawl-ron` (ADR-0007). Each floor's layout is drawn as text that
  reads as the map:

  ```
  +--+--+--+
  |A       |
  +  +dd+--+
  |  >  |sp|
  +--+--+--+
  ```

  Horizontal edges are two characters between `+` corners (`--` wall,
  two spaces open, `dd` door, `ss` secret, `^^` or `vv` one-way),
  vertical edges one character (`|`, space, `d`, `s`, `>` or `<`), and
  a cell's two characters name its specials and area through the
  file's legend. A diff of a map shows what moved, and an agent reads
  and writes it. 2218's writer gives the same text back. Tiled and LDtk
  don't hold walls between cells well, so they aren't formats of this
  kind.

### 3. Moving is turn rules

- **`Explorer`** in `shuyi_crawl::explore`: the floor, the cell, the
  facing, the explorer's effects with the steps they last (light, float
  over pits, a compass, the game's own by name), the steps taken, and
  what has been seen (§6). It hashes and dumps as text, one fact a line.
- **`CrawlRules: TurnRules`** over a map and an explorer. Commands:
  `Forward`, `Back`, `StrafeLeft`, `StrafeRight`, `TurnLeft`,
  `TurnRight`, `TurnAround`, `Open`, `Search`, `Take(stairs or
  elevator choice)`, `Wait`.
  - A step into a wall is refused (`CrawlError::Blocked`) and changes
    nothing; the view plays its bump from the refusal.
  - Each step returns events: `Moved`, `Turned`, `Spun` (for the log
    and tests; the view never shows it), `Teleported`, `Fell`,
    `Entered { area, specials }`, `Message`, `Event { name }`, `Found`
    (a secret edge).
  - `Search` finds a secret edge in front with a chance from the game
    (a trait method), on the seeded RNG.
- **A game joins the labyrinth with its own rules.** `shuyi_crawl::explore`
  also exposes the step as a function over `(&CrawlMap, &mut Explorer,
  &mut SimRng, Command)`, so a game's `TurnRules` holds the explorer
  beside its party and battles and calls it (§9).

### 4. The view is plain data

- **`CrawlView`** in `shuyi_crawl::view`, built from a map and an
  explorer: **slots** by depth (0 is the explorer's own cell) and
  lateral offset, relative to the facing. Each visible slot holds the
  cell's floor and ceiling looks and its three visible edges (in front,
  left, right) as they are *seen*: a secret edge not yet found is a
  wall, a one-way edge from its closed side is a wall.
- **What is visible**: a slot is drawn when the eye reaches it through
  open edges (and opened doors) inside the cone. A wall hides what is
  behind it. Darkness stops the view at its cell; a light effect sets
  how deep the view goes. The depth and width are settings (Wizardry's
  three deep and one to each side by default).
- **Things in cells** (a monster group, a chest, a person) are the
  game's: it hands the view a list of `(cell, look)` and each lands in
  its slot.
- The view's snapshot is text, one line per slot. Tests of what the
  player sees read it, never pixels (`AGENTS.md` rule 9). No floats.

### 5. Skins paint the view

- **Behind `shuyi-crawl`'s `draw` feature** (on `shuyi-ui`):
  - **lines**: Wizardry I's wireframe, drawn with 2336's line shapes in
    a viewport rectangle, from integer rectangles per depth. It needs no
    art, so a new game shows a labyrinth on its first run, in one bit,
    on the Playdate too;
  - **pictures**: a `*.crawlskin.ron` maps each (depth, side, face,
    look) to a picture, with left and right mirrored when the skin says
    so. A check lists every piece the view can ask for and has no
    picture.
- **Steps and turns may move.** A skin animates a step or a turn over a
  few frames as a view-only tween (2109): a slide or a zoom for
  pictures. The rules have already moved. The player's speed setting
  and reduced motion (2340) make it instant.
- **Behind the `3d` feature** (on `shuyi-3d`, ADR-0011): the labyrinth
  built as meshes from the map once (a wall, door, floor and ceiling
  piece per look, from glTF), the camera at eye height in the
  explorer's cell, steps and turns as camera tweens. That is Daphne's
  look. `frame-png` draws it with the software rasterizer (ADR-0011
  §5).
- All three read the same `CrawlView` and explorer, so a game changes
  look by changing skin, and its tests don't change.

### 6. The automap

- **What has been seen**: each floor's cells and edges, seen in a view
  or walked, kept in the explorer and so in the hash and the save. A
  secret edge, once found, is a door on the map.
- A map widget draws it from the map and the seen set: walls, doors,
  specials met, the explorer's arrow, and the player's notes (a mark
  and a text per cell). Whether the map is always there or opened by a
  spell is the game's choice.
- **Travel**: a path to a seen cell over seen cells and known edges.
  It is sent as ordinary commands, one step at a time, so a spinner, an
  encounter or a message stops it, and a record of the run stays plain
  commands.

### 7. A party of any size

- **`party.ron`**: the most members (1 to 8), how many stand in the
  front row (a party of one has no rows), and whether one member is
  fixed (Daphne's protagonist, who never leaves). Every part of
  `shuyi-rpg` is tested with a party of one and a full party.
- **Members' numbers are data.** `sheet.ron` names the game's stats,
  pools (a current and a most, such as the game's hit points), and
  conditions with their rules (can't act, can't be targeted, ends after
  some rounds, ends at rest, counts as down). A level comes from an
  experience table. The engine names no stat, pool, calling or lineage
  (`AGENTS.md` rule 4).
- **Formulas are the game's**: a `PartyMath` trait (to hit, harm,
  initiative, chances) that the game implements. The module holds the
  order of things, never the numbers.
- When no member can act, the party is beaten, and the game decides
  what that means.

### 8. Items are data

`items.ron`: each kind of item with the slot it is worn in, its stack
size, tags, what a member needs to use it, a name for when it is
unknown, whether it is cursed (it can be put on and not taken off until
a cure), and its price. Items are kept per member or shared, with
limits, as `party.ron` says. Equip, take off, use, give, drop and
identify are commands.

### 9. Battles are rounds of turn rules

- **`BattleRules<M: PartyMath>: TurnRules`**: the party against up to
  four enemy groups, each a kind and a count at a range. Each round:
  - **choosing**: every member who can act, in order, picks an action
    (fight a group, defend, an ability with a target, an item with a
    target, run, or the game's own); `Back` undoes the last pick;
  - **resolving**: in an order from `PartyMath::initiative` and the
    seeded RNG, each action and each enemy's action (from the game's
    `EnemyMind`, by default a random legal one) resolves, as events.
- Reach from rows: by default the front row reaches the first groups
  and the back row reaches nothing in melee; an item or ability can say
  otherwise.
- Surprise rounds, running away, and enemies known only by their
  unknown names until identified.
- **One history for a whole run.** A game holds exploring, battles and
  chests in one `TurnRules` (an enum of modes), so a run is one seed and
  one list of commands and the bots and playtests (ADR-0027) play all of
  it. The example shows this. A game may instead push a battle stage and
  hand back the result (ADR-0003 §1, ADR-0018); the module works either
  way.

### 10. Abilities are data

`abilities.ron`: each ability with its cost (charges per level as in
Wizardry, a pool, or nothing), how it is learned (a level table or a
choice), its target (self, a member, the party, an enemy, a group,
every enemy, the explorer), and where it works (battles, the
labyrinth, the town). What it does is the game's (`PartyMath`), apart
from the explorer's effects `shuyi-crawl` knows (light, float, compass),
which the game passes on.

### 11. Encounters and chests

- `encounters.ron`, per area name: a chance per step, a weighted table
  of groups (2114) with counts and ranges, and fixed encounters by mark
  (once, or until beaten). The game asks it after each step's events.
- After a victory, drops from a drop table: items, money, or the
  game's own drop by name.
- **Trapped chests are the template's, not the engine's** (§17).
  Wizardry's chest (inspect, name the trap, perhaps wrongly, disarm,
  open) is its own habit. The crawler template copies it into a new
  game as code the game owns (2754).

### 12. Characters made and kept

`make.ron`: the steps of making a character, as choices the game names
(for example a lineage and a calling), each option with its numbers and
what it needs, and bonus points rolled from the seed with a reroll as a
command. Making a character is turn rules, so it replays and a bot can
make one. The roster is kept in the player's profile, and the party is
formed from it in the hub.

### 13. Shops and paid services, not a town

- **A shop** (stock, buy, sell) and **a paid service** (restore pools,
  end a condition, refill charges, apply level-ups, identify, uncurse,
  or the game's own by name) are engine: nearly every RPG has them.
  Money is a pool.
- **A place list** is a plain menu over the game's list of places, each
  opening a shop, the roster, or the game's own screen.
- **Which places a town has is the game's** (§17). Wizardry's tavern,
  inn and temple are the crawler template's `places.ron`: a shop with
  the services of an inn, another with a temple's. The engine has no
  inn type.

### 14. Openings need nothing new

An opening like Daphne's is a cinematic (game ADR-0060), then dialogue
over the labyrinth's view before the first command. The example shows
it. A pre-rendered video opening waits for 2909.

### 15. The example and the template

`examples/crawler` and a template part `kind-dungeon-crawler` (2501).
The template starts **a party of one** (`max: 1`), as Nick prefers.
Raising `max` gives a full party, rows and all, with no other change.
The example plays both.

### 16. What every target runs

Both modules' rules are pure and use no floats. The pictures skin runs
everywhere 2D runs: the web, the Playdate (in one bit, ADR-0026), the
GBA and DS (indexed colour, ADR-0012). The lines skin needs 2336. The 3D
skin runs where ADR-0011 §8 says 3D runs.

### 17. What is engine, what is template, what is example

Nick, 2026-10-22: "ensure these are all 'engine-shaped' tickets and not
'game-shaped' tickets", and "certainly we can build one of each genre
as a demo of the engine". Every piece of a kind of game lands in one of
three places, by this bar:

| Where | The bar | In this ADR |
| ----- | ------- | ----------- |
| **Engine** (a module crate) | At least two different kinds of game would use it as it is, and it names no game's content: no stat, class, item, monster, place or rule a game would call its own. What varies is data or a trait the game implements | the map and its edges, moving, the view, the skins, the automap, a party of any size, sheets as data, items, round-based battles, abilities, encounter and drop tables, shops and services, making characters, the roster |
| **Template** (a part of `shuyi new`) | A genre's habit that a new game usually wants and often changes. It is copied into the new game as code and data the game owns, edits or deletes, and is never a type in an engine crate | trapped chests; a town of a tavern, an inn, a temple and a shop; a starter floor, enemy and table |
| **Example** (`examples/`, a demo) | A small complete game of the genre that shows the engine off and is its integration test. Its story, numbers, art and rules are its own, made for it, and copy no other game | the crawler with its opening, two floors and its formulas (2754) |

- A ticket that adds to an engine crate names, in its Context, the
  kinds of game that would use the piece. One that can't name two moves
  the piece to the template or the example.
- When unsure, it is the template's or the example's (`AGENTS.md`
  rule 4: "When unsure whether something is engine or game, it is
  game").
- ADR-0034 (action RPGs) uses the same bar.

## Consequences

- A crawler is a small game to write: a map drawn in text, the data
  files of §7–§13, the game's formulas and a skin. The wireframe needs
  no art at all.
- `shuyi-rpg` serves more than crawlers: any party RPG with menus and
  round-based fights uses it without the labyrinth.
- Walls between cells are a second kind of map beside the grid kit's
  cells. Paths and travel need a neighbour function that reads edges;
  2744 adds one.
- The number of slots a pictures skin needs grows with the view's depth
  and width. The check (§5) keeps it honest, and the lines skin is the
  fallback.
- Real-time crawlers (Dungeon Master, Legend of Grimrock) aren't
  covered. Their map and view are this ADR's, but stepping in real time
  waits for the tick simulation (1119) and a ticket of its own.

## Alternatives considered

- **One module for everything** — a top-down party RPG would pull in a
  labyrinth it never draws, and a puzzle labyrinth a battle system. Two
  modules cost one more catalog row.
- **Walls as cells** (the roguelike way, 2119) — a crawler's thin
  walls would take a grid twice the size, its maps would stop looking
  like the genre's, and doors between two rooms would be cells to stand
  in.
- **A raycaster as the 2D skin** — smooth turning on any angle, but the
  genre turns in quarters, and pictures per slot are its classic look
  and cheaper on the Playdate and the GBA. A raycasting skin can be
  added later beside the others, from the same view.
- **Stats in the engine** (hit points, strength, magic points as
  fields) — every game names and counts its own, and `AGENTS.md` rule 4
  keeps them out. Names as data cost a lookup when a game is built.
- **Battles only as a separate stage** — a run would be two histories
  that the bots can't play as one. §9 allows both and the example shows
  one history.
