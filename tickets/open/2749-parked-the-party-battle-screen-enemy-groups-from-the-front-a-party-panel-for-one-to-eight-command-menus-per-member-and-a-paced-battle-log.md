---
id: "2749"
title: "Parked: The party battle screen: enemy groups from the front, a party panel for one to eight, command menus per member, and a paced battle log"
type: feature
milestone: "Engine: dungeon crawlers"
model: implementation
effort: high
status: blocked
blocked_by: ["2748"]
nick_input: none
completed:
---

# 2749 — The party battle screen

> **Parked** (Nick, 2026-10-22: "if we can have tickets to add support for this type of game too"): like the other kinds of game, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
§9, and `AGENTS.md` rule 9: logic builds a plain-data scene, a skin
paints it. 2748's `BattleRules` give the state and events; this ticket
is the stage that shows them and sends the player's picks.

The look, from Wizardry and Daphne: the enemies drawn from the front
over the labyrinth's view (or a background picture), a list of groups
with counts, the party's panel with each member's pools and condition,
a menu of commands for the member choosing, and a log that tells the
round line by line. With a party of one the panel is one line and
there is one menu a round; it must still look full and read well.

It builds on 2748, the engine's menus (2307, 2308, 2309, all done), the
text box and the hurry clock (2305, 2312, done) and 2311's meter.

**Either order with 2340:** if 2340 is done, a hit shakes and flashes
through it, within the player's settings. If not, 2340 wires this
screen when it lands.

`shuyi-rpg` never depends on `shuyi-crawl` (ADR-0033 §1), so the
labyrinth behind the enemies is drawn by the game: `Backdrop::Picture`
is a picture, and `Backdrop::Below` leaves the frame's world layers as
the stage below drew them (a crawler's frozen view, a top-down RPG's
field).

**Who uses it** (ADR-0033 §17's bar for engine work): the same games as 2748; it draws over whatever the stage below drew (a labyrinth, a field, a picture).

## Nick input

None.

## Scope

**In:**
- **`BattleScene`** in `shuyi_rpg::battle::scene` (behind `draw`):
  plain data built from `BattleState`: the groups (look, count alive,
  name or unknown name as a text key, range), the party (name, pools as
  meters, conditions, row), whose turn it is to choose, the commands
  open to them (from `legal_commands()`, as menu items with values,
  ADR-0020), the target list when a command needs one, and the log
  lines of this round. Its text snapshot is what tests read.
- **`BattleSkin`**: paints the scene:
  - enemies: the group's picture once, or up to N side by side, from
    the game's look table, scaled down by range, over the backdrop;
  - the group list with counts and names;
  - the party panel: one row per member in party order, a meter per
    pool the game marks `shown` in `battle.ron`, the worst condition as
    a word, rows split by a line when the party has rows;
  - colours by palette role; the one-bit renderer keeps them apart
    (ADR-0026).
- **The stage** `BattleStage`: a screen that holds `BattleRules`, sends
  the picks, and on resolution plays the events one log line at a time
  through the hurry clock (a held key speeds it up, as it does today),
  with a per-event cue for the skin (hit, miss, fall). It pops with the
  outcome (ADR-0018). Actions only, and the controls fit every target
  (ADR-0025): the screen declares its actions.
- **Settings** in `battle.ron` (2748's file): which pools show, the log's
  speed, whether the log waits for a press at the end of a round.
- **Snapshots**: `frame-png` at choosing, at targeting and mid-log, for
  a party of one and of six.

**Out (do not do):**
- what any command does (2748, 2750);
- enemy animation beyond the cue (a game's skin may animate);
- the hub (2752) and the chest screen (the game's own code: the crawler
  example's, 2754).

## Implementation steps

1. `BattleScene` from `BattleState`, with its text snapshot.
2. `BattleSkin` with both backdrops (or one, as above).
3. `BattleStage`: picks, targets, `Back`, the paced log.
4. Harness tests and snapshots at sizes one and six.

## Acceptance criteria

- [ ] A Harness test plays a battle to the end with a party of one and
      one with a party of six, reading the scene, never the pixels.
- [ ] `Back` returns to the previous member's menu (a Harness test).
- [ ] The log plays one line per event and a held action speeds it.
- [ ] `frame-png` snapshots at the six points above.
- [ ] `check-target` passes for the screen's declared actions on every
      target profile (ADR-0025).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the scene from a state; the menu items from `legal_commands()`.
- Integration: Harness battles at sizes one and six.
- Snapshot: `frame-png` and scene text.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
