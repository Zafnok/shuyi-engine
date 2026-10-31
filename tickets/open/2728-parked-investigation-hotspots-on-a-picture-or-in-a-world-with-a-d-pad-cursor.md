---
id: "2728"
title: "Parked: Investigation: hotspots on a picture or in a world, with a cursor that works with only a d-pad"
type: feature
milestone: "Engine: detective games"
model: reasoning
effort: high
status: blocked
blocked_by: ["2727", "2405"]
nick_input: none
completed:
---

# 2728 — Investigation: hotspots and a d-pad cursor

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0016](../../docs/adr/0016-visual-novels-and-detective-games-a-story-stage-a-case-file-and-a-fair-play-check.md)
§7: a detective game is investigated by examining things, on a picture
or in a world. Each thing is a **hotspot** with conditions and an
action. The player points at it with a pointer where there is one
(2405), and with only a d-pad the cursor jumps between hotspots, so
investigation fits every target (3002, ADR-0008 §3). Pixel hunting is
not a feature: the scene says which hotspots are found.

This ticket builds on 2727 (`CaseState`, `MysteryRules`, its mode stack
and node call) and 2405 (pointer input in console pixels). Every choice
the player makes is a command in `MysteryRules`, so an investigation
replays.

**World hotspots, either order with 1122** (collision, movers and
triggers):
- if 1122 is done, this ticket also builds world hotspots on its
  `Trigger`s (Scope, last item);
- if 1122 isn't done, do pictures only. Write a follow-up parked ticket
  (the next free `27xx` id, `blocked_by` this ticket and 1122, with the
  `Parked:` prefix and the same parked note) for world hotspots, and add
  a "World hotspots" section to this ticket's Completion notes that
  names it.

## Nick input

None.

## Scope

**In:**
- **Investigations as data** (ADR-0007): `*.investigation.ron`, format
  `investigation-ron`, asset kind `Investigation`:
  - `place`: the place to go it belongs to (2727's `Place`);
  - `picture`: the background, in picture pixels;
  - `hotspots`, in file order, each with:
    - `id`;
    - `shape`: `Rect { x, y, w, h }` or `Polygon { points }`, in
      integer picture pixels (3 to 32 points);
    - `label`: a text key, shown when the hotspot is selected;
    - optional `anchor`: the point the d-pad cursor sits on (default:
      the centre of the shape's bounding box, rounded down);
    - optional `order`: a number that overrides reading order;
    - `when`: conditions that must all hold for the hotspot to be there:
      `Found(clue)`, `NotFound(clue)`, `Met(person)`, `Made(deduction)`,
      `NotMade(deduction)`, `Var(name, value)` (a dialogue variable);
    - `action`: `Examine { node }` (run a dialogue node),
      `Talk { person, node }` (run a node and meet the person), or
      `Go { place }`;
  - `exit`: optional `{ when, node }`: when its conditions hold after an
    action, the investigation ends and the story jumps to `node`.
  - Errors with file and line: an empty or self-crossing polygon, a
    shape outside the picture, two hotspots with one id, an unknown
    clue, person or place. A `Made` or `NotMade` naming an unknown
    deduction is checked if 2729 is done; if not, 2729 adds that check.
- **Hit testing in integers**, `shuyi_mystery::investigate::hit`:
  - a rectangle holds `(px, py)` when `x <= px < x + w` and
    `y <= py < y + h`;
  - a polygon by the crossing-number rule with `i64` cross products and
    a half-open rule per edge (as a rectangle is half-open), so a point
    on an edge shared by two polygons belongs to at most one of them.
    No floats;
  - when hotspots overlap, the later one in the file is on top and wins.
- **Entering an investigation**: the dialogue command
  `@investigate <place>` (and its Yarn Spinner and Ink forms), a command
  that stops the story (2727's `stops` flag), pushes
  `Mode::Investigate { place }` on 2727's mode stack. The story waits
  until `exit` holds or a `Go` leaves for another place's
  investigation.
- **Commands in `MysteryRules`** (ADR-0016 §8, last point, applied to
  §7):
  - `MysteryCommand::Examine { hotspot }`: legal only in
    `Mode::Investigate` and only for a hotspot whose `when` holds. It
    runs its action through the node call (2727), marks the hotspot
    examined in `CaseState`, and checks `exit` when the node ends;
  - `MysteryCommand::Go { place }` through a `Go` hotspot.
  - The cursor and the pointer are never in a command; only the
    hotspot chosen is. So a run played with a pointer and one played
    with a d-pad give the same record.
- **`CaseState` gains** `examined: BTreeSet<(PlaceId, HotspotId)>`. The
  scene marks those hotspots as found.
- **Pointer picking** (2405): `FrameInput::pointer()`, whose `at` is in
  console pixels. The screen maps `at` to picture pixels through the
  picture's place in the scene (integer offset and scale), and `hit`
  picks the top hotspot there. Hover selects it on a frame the pointer
  `moved`, and a click examines it, as 2405's menus do. A still pointer
  never takes the selection back from the d-pad. A click here is a
  release whose press began on the same hotspot (`hit` gives the same
  one at `down_at` and at `at`), as 2405's grid stage clicks a tile:
  `PointerInput::clicked` knows only the rectangles a painter records,
  and a hotspot may be a polygon. The confirm input examines the
  selected hotspot too.
- **The d-pad cursor**: the hotspots there now, in reading order. Sort
  by anchor `y`; anchors within `row_band` pixels of the first in a row
  (default 8, set in `mystery.ron`) form one row, sorted by `x`; `order`
  overrides. Down and Right move to the next, Up and Left to the
  previous, wrapping. Confirm examines, Cancel opens the screen's menu.
  When the hotspots change after an action, the cursor stays on the
  same hotspot if it is still there, otherwise on the next one in order.
- **`InvestigationScene`**, plain data: the background, each hotspot
  there now (id, bounding box, found or not, selected or not, label key
  when selected), and the place's name. `InvestigationScreen` builds it
  and reacts to `UiAction`s and the pointer only. It declares its
  actions (3002). Its skin draws the marks as sprite
  items and the label as glyph cells; what a mark looks like is the
  skin's.
- **A Playdate fit test**: a fixture keymap layout named `playdate`
  passes `controls_fit` (3002) for the investigation screen.
- **World hotspots** (only if 1122 is done): `WorldHotspots` maps a
  1122 `Trigger` on a thing in a tick stage to a hotspot's `when` and
  `action`. Shapes are the trigger's box; polygons are pictures only.
  The tick stage holds `MysteryRules` and sends it `Examine` from its
  `step` when the game's examine action is pressed inside a trigger, so
  the stage's `TickRecord` replays the investigation too. The scene of
  the world stage gets the same found marks.

**Out (do not do):**
- the board, blanks and `@deduce` (2729);
- testimony (2730);
- the fair-play check (2731), which reads these files later;
- the example (2732);
- pointer input itself (2405) and triggers themselves (1122);
- moving or animated hotspots (write a ticket if a game asks);
- hiding hotspots until the pointer is over them: the scene always
  lists every hotspot there now (ADR-0016 §7).

## Implementation steps

1. Write `Investigation`, `Hotspot`, `Shape`, `Condition` and
   `HotspotAction` in `shuyi_mystery::investigate`, and the
   `investigation-ron` format with good and bad fixtures.
2. Write `hit` and its property tests.
3. Write `Condition::holds(&CaseState, &VnState)`.
4. Add `Mode::Investigate`, `@investigate`, `Examine`, `Go` and `exit`
   to `MysteryRules`, and `examined` to `CaseState`.
5. Write the reading order, the cursor, the pointer mapping,
   `InvestigationScene`, its screen and its skin.
6. If 1122 is done, write `WorldHotspots` and a small tick stage
   fixture. If not, write the follow-up parked ticket.

## Acceptance criteria

- [ ] On a fixture picture, a Harness test examines every hotspot with
      the d-pad cursor only, and another does it with the pointer only;
      both records end in the same `CaseState` hash.
- [ ] A hotspot whose `when` doesn't hold is not in the scene, not in
      the cursor's ring, and `Examine` on it is refused and changes
      nothing.
- [ ] `exit` jumps the story to its node once its conditions hold.
- [ ] Replaying an investigation's commands gives the same hash at
      every step.
- [ ] Each bad fixture gives an error naming its file and line.
- [ ] World hotspots work in a tick stage fixture, or the follow-up
      parked ticket exists and the Completion notes name it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: rectangle edges; a concave polygon; overlap order; reading
  order with rows, `row_band` and `order`; the cursor after hotspots
  change; each condition.
- Property: a point is inside a rectangle exactly when it is inside
  the same rectangle written as a 4-point polygon; for two polygons
  that share an edge, no point is in both; random bytes into
  `investigation-ron` never panic.
- Fixture and determinism: a decode hash; the investigation replay.
- Snapshot: the `InvestigationScene` dump with found and unfound
  hotspots; one `frame-png` picture of the skin's marks.

## Completion notes
