---
id: "2729"
title: "Parked: Deductions: a board of linked clues, and sentences with blanks confirmed in groups"
type: feature
milestone: "Engine: detective games"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2727"]
nick_input: none
completed:
---

# 2729 — Deductions: boards and blanks

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0016](../../docs/adr/0016-visual-novels-and-detective-games-a-story-stage-a-case-file-and-a-fair-play-check.md)
§8: a detective game asks the player to join what they found into a
conclusion. Deductions come in two shapes, as data:

- **links**: the player joins clues, people and places on a board. The
  right set unlocks the conclusion; a wrong set gets its own answer or a
  default one;
- **blanks**: sentences with slots, each filled from a list. Answers are
  confirmed in groups of a set size (default 3), and only when the
  whole group is right, so guessing doesn't pay.

Every move is a command in `MysteryRules` (2727), so deductions replay.
This ticket builds on 2727's `CaseState` (its `deductions` set is filled
here), its mode stack and its case file screen in `Choose` mode.

**Either order with 2730** (testimony and penalties): a deduction may
cost a penalty for a wrong answer. If 2730 is done when this lands,
wire `penalty` (below) into its meter. If not, keep the field, read it,
and 2730 wires it.

**Either order with 2728** (investigation): if 2728 is done, its
`Made` and `NotMade` conditions get a check that the deduction id
exists, here. If not, 2728 lands second and adds the check.

## Nick input

None.

## Scope

**In:**
- **Deductions as data** (ADR-0007): `*.deduction.ron`, format
  `deduction-ron`, asset kind `Deduction`:
  - `id`, `title` (a text key), `available`: conditions (2727's facts:
    clues found, people met, deductions made) for the deduction to be
    open at all;
  - `penalty`: what a wrong answer costs (default 0);
  - `then`: an optional dialogue node the story jumps to when the
    deduction is made, and optional `clue` (a clue found when it is
    made: a conclusion can be a clue for the next deduction);
  - `shape: Links { … }` or `shape: Blanks { … }`, below.
  - Errors with file and line: an unknown clue, person, place or word
    list; a links answer of fewer than 2 items; a slot whose answer
    isn't in its list; a group size of 0.
- **Links** (`Links { question, answer, wrong, default_wrong }`):
  - `question`: a text key shown on the board;
  - `answer`: the right set of items (`Clue(id)`, `Person(id)`,
    `Place(id)`), compared as a set, order ignored;
  - `wrong`: a list of `{ items, node }`, each a specific wrong set
    (compared as a set) and the dialogue node that answers it;
  - `default_wrong`: the node for any other set. If it is missing, the
    case-wide `default_wrong` in `mystery.ron` is used, and loading
    fails if neither is there.
  - `MysteryCommand::Link { deduction, items }`. It is refused unless
    the deduction is open and every item is found or met. The right set
    makes the deduction. A wrong set runs its node through 2727's node
    call and costs `penalty`.
- **Blanks** (`Blanks { sentences, group }`):
  - `sentences`: each a text key with named slots (`{who}`, `{how}`),
    and per slot its list and its answer. A list is `People`, `Clues`,
    `Places` (only those found or met are offered) or `Words(id)`, a
    word list in the file, each word a text key;
  - `group`: the group size, default 3, set per deduction or in
    `mystery.ron`;
  - the player's draft is state:
    `MysteryCommand::SetBlank { deduction, slot, choice }` and
    `ClearBlank`, kept in `CaseState`
    so a save keeps it. A confirmed slot can't be changed;
  - `MysteryCommand::ConfirmBlanks { deduction }`:
    1. count the unconfirmed slots whose draft is right: `right`;
    2. let `size` be `group`, or the number of unconfirmed slots if
       fewer remain (so the last ones can be confirmed);
    3. if `right < size`, nothing changes. The event is
       `BlanksNotConfirmed { deduction }`, with no count and nothing
       else that tells how many were right;
    4. otherwise confirm `(right / size) * size` of the right slots,
       the first ones in slot order (sentences in file order, slots in
       text order), and send `BlanksConfirmed { deduction, slots }`;
    5. when every slot is confirmed, the deduction is made.
  - A blanks deduction costs `penalty` on a confirm that confirms
    nothing, only if its `penalty` is above 0 (the default is 0, as
    Return of the Obra Dinn has none).
- **Opening a deduction**:
  - the dialogue command `@deduce <id>` (and its Yarn Spinner and Ink
    forms), a command that stops the story (2727's `stops` flag),
    pushes `Mode::Deduce { deduction }`. The story waits there
    until the deduction is made. 2731 treats this as the moment the
    player is "asked" (ADR-0016 §9);
  - a game may also open any open deduction from its own menu; the
    story doesn't wait then;
  - `MysteryCommand::LeaveBoard` pops a board the story isn't waiting
    on.
- **`CaseState` gains** the blanks drafts and confirmed slots, and adds
  made deductions to its `deductions` set, all in `BTreeMap`s.
- **The board screen**, behind the `view` feature:
  - `BoardScene`, plain data. For links: the question, the items on
    the board, and the last answer shown. For blanks: each sentence
    with its slots' drafts, which slots are confirmed, the open list
    when a slot is being filled, and the group size;
  - `BoardScreen` reacts to `UiAction`s and the pointer (1121) where
    there is one. Adding an item opens 2727's case file in `Choose`
    mode. Filling a slot opens a list. It declares its actions (3002) if
    3002 is done, and fits the `playdate` fixture layout then;
  - `BoardSkin`: text, slots and lists as glyph cells; item pictures
    as sprite items.

**Out (do not do):**
- hints (2731 adds `hints` to this format and the command that shows
  them);
- the fair-play check (2731);
- testimony and the penalty meter (2730);
- free-form links drawn between pairs of items (a set is the answer
  here; a game can add its own shape as commands, ADR-0016
  alternatives);
- the example (2732).

## Implementation steps

1. Write `Deduction`, `DeductionShape`, `Links`, `Blanks`, `Sentence`,
   `Slot`, `WordList` and `Item` in `shuyi_mystery::deduce`, and the
   `deduction-ron` format with good and bad fixtures. Add
   `default_wrong` and `group` to `mystery.ron`.
2. Write the links rule as a pure function:
   `judge_links(&Links, &BTreeSet<Item>) -> LinksAnswer`.
3. Write the blanks rule as a pure function:
   `confirm(&Blanks, &Draft, &Confirmed) -> Confirmation`.
4. Add the commands, `Mode::Deduce`, `@deduce` and the `CaseState`
   fields to `MysteryRules`.
5. Write `BoardScene`, `BoardScreen` and `BoardSkin`.
6. Wire `penalty` to 2730's meter if 2730 is done, and the `Made`
   check of 2728 if 2728 is done.

## Acceptance criteria

- [ ] On a links fixture, the right set makes the deduction, a listed
      wrong set runs its own node, and any other set runs the default.
- [ ] On a blanks fixture with group 3, a confirm with 2 right slots
      leaves `CaseState` unchanged and sends only
      `BlanksNotConfirmed`; a confirm with 0 right sends the same event
      and gives the same `BoardScene` confirmed marks.
- [ ] With 4 right slots and group 3, exactly the first 3 right slots
      in slot order are confirmed.
- [ ] With 2 slots left and group 3, both right confirms them and makes
      the deduction.
- [ ] `@deduce` holds the story until the deduction is made.
- [ ] Replaying a board session's commands gives the same hash at every
      step.
- [ ] Each bad fixture gives an error naming its file and line.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `judge_links` for right, listed wrong and default; `confirm`
  for each step above, including the last smaller group; each command
  refused when the deduction isn't open or an item isn't found.
- Property: for any draft and any confirmed set, a confirm with fewer
  right slots than the group size leaves the state equal to before;
  confirmed slots never become unconfirmed; random bytes into
  `deduction-ron` never panic.
- Fixture and determinism: decode hashes; the replay.
- Snapshot: `BoardScene` dumps for links and blanks; one `frame-png`
  picture of each.

## Completion notes
