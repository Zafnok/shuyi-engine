---
id: "2766"
title: "Parked: A message log widget: event lines with colour roles, repeats folded, a few lines on screen, and a history screen"
type: feature
milestone: "Engine: roguelikes"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: blocked
blocked_by: []
nick_input: none
completed:
---

# 2766 — A message log widget

> **Parked** (Nick, 2026-10-20, choosing roguelikes as a kind of game the engine serves, and 2026-10-22: "I also want support for roguelikes, and Diablo-likes"): like the other roguelike tickets, this waits until after Visions of Shuyi launches, unless the game asks for it sooner through its `engine-change` skill. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found on 2026-10-22 when the backlog was checked for roguelike gaps
([ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
§17's bar). A roguelike tells the player what happened in a few lines
under the map ("The rat bites you. ×3"), with older lines in a history
screen. The engine's `shuyi_ui::log` is the developer's log
(ADR-0030), not the player's; 2749's battle log paces one battle's lines
and doesn't keep a history.

**Who uses it**: roguelikes, crawlers outside battle, tactics games'
battle logs, survival and colony games' event feeds, text adventures.

It builds on text by key and styled text in lines (2347, if done; see
below), and the engine's text box and list cursor (done).

**Either order with 2347:** if 2347 is done, a line holds a
`StyledText`, so it may hold colour roles inside it, and this ticket
wraps a line over its spans, each span keeping its role (the wrapping
of a `StyledText` that 2347's Out leaves to the box that needs it). If
not, a whole line takes one role, and 2347 turns the lines into
`StyledText` and wraps them over their spans, as its clause says.

**A candidate from the game** (the game/engine boundary audit of
2026-11-03): the game's dialogue screen keeps a backlog of a talk, about
60 production lines with nothing of the game in them but the player
type (game `crates/ui/src/screens/dialogue/history.rs`): each line seen
as a copy of `DialoguePlayer<H>` there, with the reply picked and the
`MusicLine` playing; jumping back to a line shows the scene as it was,
keeps the lines after it, and cuts them from the first reply on, so a
reply is asked again. As `shuyi_ui::dialogue::History<H: DialogueHooks>`
(`reached`, `replied`, `jump`) it could feed this ticket's history screen
for a game's talk, as `MessageLog` does for events. Whoever unparks this
decides whether it comes in; if it does, write the game ticket that
swaps the game's copy for it. The game ticket game#1218, which trims that screen's
tests, leaves `history.rs` in place for this. Parked 2723's visual novel backlog is its
own, over the story's events.

## Nick input

None.

## Scope

**In:**
- **`MessageLog`** in `shuyi_ui::message_log`: lines of a text key with
  arguments (formatted through the language pack, so a log stays
  translatable), a colour role, and the turn or step it came on; a cap
  on lines kept (a setting); repeats of the same line in a row folded
  into one with a count (`×3`); serialisable, so a save keeps it if the
  game wants.
- **The widget**: the last N lines in a rectangle, newest at the
  bottom, word-wrapped (2209), lines from this turn bright and older
  ones dimmed by a role; scrolls by whole lines.
- **The history screen**: every kept line, with the list cursor's
  scroll window, pops back (ADR-0018). Actions only.
- Harness test and `frame-png` snapshots.

**Out (do not do):**
- 2749's paced battle log (it may later push its lines here too);
- speech bubbles in the world;
- 2723's visual novel backlog.

## Implementation steps

1. `MessageLog` and folding.
2. The widget and the history screen.
3. Tests and snapshots.

## Acceptance criteria

- [ ] Repeats fold with a count; a different line between them breaks
      the fold (tests).
- [ ] The log keeps at most its cap.
- [ ] A Harness test opens the history and scrolls to the oldest line.
- [ ] `frame-png` snapshots of the widget and the screen.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: folding, the cap, formatting by key.
- Integration: the Harness test.
- Snapshot: widget and screen.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
