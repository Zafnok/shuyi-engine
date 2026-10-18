---
id: "2730"
title: "Parked: Testimony: press a statement, present a clue at a contradiction, and penalties as data"
type: feature
milestone: "Engine: detective games"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["2727"]
nick_input: none
completed:
---

# 2730 — Testimony and penalties

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0016](../../docs/adr/0016-visual-novels-and-detective-games-a-story-stage-a-case-file-and-a-fair-play-check.md)
§8: a witness gives **testimony**, statements in order. The player can
**press** a statement to hear more, or **present** a clue at the
statement it contradicts. The right clue at the right statement moves
the story on; a wrong one costs a **penalty**. Penalties are data: a
meter that runs down, and what happens when it is empty (start the
testimony again, a game over, or nothing). All of it is commands in
`MysteryRules` (2727), so it replays.

Testimony is told the way a visual novel is (ADR-0016 §1): each
statement is drawn through the visual novel's text box (2722), with
the witness as the speaker. This ticket builds on 2727's `CaseState`
(its penalty count), its mode stack and node call, and its case file
screen in `Choose` mode.

**Either order with 2729** (deductions): a wrong deduction may cost a
penalty too. If 2729 is done when this lands, wire its `penalty` field
into the meter here. If not, 2729 wires it.

## Nick input

None.

## Scope

**In:**
- **Testimony as data** (ADR-0007): `*.testimony.ron`, format
  `testimony-ron`, asset kind `Testimony`:
  - `id`, `witness` (a person id, 2727), optional `title` (a text key);
  - `statements`, in order, each with:
    - `id` and `line`: the statement's text, by text key and line id,
      so translation and voices work on it as on any line. Voices are
      2725's job; this ticket only keeps the line id;
    - `press`: an optional dialogue node, run when the statement is
      pressed;
    - `contradicted_by`: the clues that contradict it (empty for most
      statements), and `then`: the node the story jumps to when one is
      presented here;
    - `wrong`: an optional node for a wrong clue at this statement;
  - `wrong`: the testimony's node for a wrong clue, used when the
    statement has none; else `mystery.ron`'s `default_wrong_present`.
    Loading fails if none of the three is there;
  - `penalty`: what a wrong presentation costs (default 1);
  - `on_empty`: an optional override of the meter's outcome (below)
    for this testimony.
  - Errors with file and line: an unknown person, clue, node or text
    key; a statement with `contradicted_by` and no `then`; a testimony
    in which no statement has `contradicted_by` (it could never end).
- **The penalty meter**, in `mystery.ron`:
  `meter: { max, on_empty }`, with `on_empty` one of:
  - `RestartTestimony`: the meter fills again and the testimony starts
    from its first statement. Outside a testimony (a wrong deduction,
    2729) it only fills the meter again;
  - `GameOver { node }`: the story jumps to `node`, which must reach an
    `ending` (2721). The game's own flow decides what follows (for
    example, loading a save);
  - `Nothing`: the meter stays empty and play goes on.
  - No `meter` at all means penalties are counted and do nothing else.
  - The meter is `max` minus 2727's penalty count, never below 0.
    Refilling resets the count. Penalties taken in all are also kept,
    for a game that wants to grade the player.
- **Running a testimony**:
  - the dialogue command `@testimony <id>` (and its Yarn Spinner and
    Ink forms), a command that stops the story (2727's `stops` flag),
    pushes `Mode::Testimony { id, statement }` at the first
    statement. The story waits until a contradiction is presented;
  - `MysteryCommand::NextStatement` and `PreviousStatement`; after the
    last statement it goes back to the first, as testimony loops;
  - `MysteryCommand::Press`: runs `press` through 2727's node call and
    comes back to the same statement. Refused when the statement has no
    `press`;
  - `MysteryCommand::Present { clue }`: refused unless the clue is
    found. If the clue is in the statement's `contradicted_by`, the
    event is `Contradiction { testimony, statement, clue }`, the mode
    pops, and the story jumps to `then`. Otherwise the event is
    `WrongPresent`, the penalty is taken, the `wrong` node runs through
    the node call, and the meter's outcome applies if it is empty;
  - `CaseState` gains the testimonies finished and each one's
    contradictions found.
- **The testimony screen**, behind the `view` feature:
  - `TestimonyScene`, plain data: the visual novel's `VnScene` (2722)
    for the stage and the text box, plus the statement number and
    count, whether it can be pressed, and the meter (value and max);
  - `TestimonyScreen` reacts to `UiAction`s: Left and Right move
    between statements, Confirm presses, and present opens 2727's case
    file in `Choose` mode. Present is `MysteryAction::Present`, added
    here to 2727's action set, which a game maps from its own
    `ActionSet` (the `input-actions` skill); the help line names its
    input through `HelpKeys`. It is required on this screen. The
    screen declares its actions (3002) if 3002 is done, and fits the
    `playdate` fixture layout then;
  - the skin draws the meter and the statement number as glyph cells,
    over the visual novel skin's text box.

**Out (do not do):**
- deductions (2729), except wiring its `penalty` when 2729 is done;
- statements that change or appear after a press (write a ticket if a
  game asks);
- presenting people or places (ADR-0016 §8 names clues; a ticket can
  add them);
- the fair-play check (2731), which reads these files later;
- the example (2732).

## Implementation steps

1. Write `Testimony`, `Statement`, `Meter` and `OnEmpty` in
   `shuyi_mystery::testimony`, and the `testimony-ron` format with good
   and bad fixtures. Add `meter` and `default_wrong_present` to
   `mystery.ron`.
2. Write the meter as a pure type: `take(cost) -> MeterOutcome`.
3. Add `Mode::Testimony`, `@testimony` and the commands to
   `MysteryRules`, and the `CaseState` fields.
4. Write `TestimonyScene`, `TestimonyScreen` and its skin.
5. Wire 2729's `penalty` into the meter if 2729 is done.

## Acceptance criteria

- [ ] On a fixture testimony, pressing statement 2 runs its node and
      returns to statement 2.
- [ ] Presenting the right clue at the right statement jumps the story
      to `then` and ends the testimony.
- [ ] Presenting it at another statement, or another clue, takes the
      penalty and runs the right `wrong` node.
- [ ] Each `on_empty` outcome does what this ticket says, with a fixture
      each, and a testimony's `on_empty` overrides the case's.
- [ ] Presenting a clue not found is refused and changes nothing.
- [ ] Replaying a testimony's commands, wrong ones included, gives the
      same hash at every step.
- [ ] Each bad fixture gives an error naming its file and line.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the meter (take, empty, refill, no meter); statement moves and
  looping; each command, legal and refused.
- Property: the meter never goes below 0 or above `max`; random bytes
  into `testimony-ron` never panic.
- Fixture and determinism: decode hashes; the replay.
- Snapshot: the `TestimonyScene` dump; one `frame-png` picture of a
  statement with the meter.

## Completion notes
