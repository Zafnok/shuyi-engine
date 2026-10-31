---
id: "2731"
title: "Parked: Fair play, checked: every case solvable from what can be found, no dead ends, and hints as data"
type: feature
milestone: "Engine: detective games"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["2728", "2729", "2730", "2724"]
nick_input: none
completed:
---

# 2731 — Fair play, checked

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0016](../../docs/adr/0016-visual-novels-and-detective-games-a-story-stage-a-case-file-and-a-fair-play-check.md)
§9: what players hate most in a mystery is a case they can't solve from
what they can find, or a game that leaves them stuck. The engine checks
it. `shuyi check` gains a case check, `shuyi_mystery::fair`, that reads
the scripts, hotspots, deductions and testimony as a graph and says:

- **solvable**: every deduction and contradiction needs only clues that
  can be found before the player is asked it, on every path;
- **no dead ends**: from every reachable state the case can still be
  closed, and a penalty that empties the meter has an outcome;
- **every clue matters or says so**: a clue no deduction or testimony
  uses is a warning, unless it is marked as colour.

It reports the path that breaks a rule as lines and choices. **Hints**
are data too: an ordered list per deduction.

It needs everything it reads: investigation (2728), deductions (2729),
testimony and the meter (2730), and 2724's story checks in
`shuyi check`, which it plugs into. The example (2732) runs it in CI.

## Nick input

None.

## Scope

**In:**
- **The graph**, `shuyi_mystery::fair::CaseGraph`, built from the
  loaded assets, never from a running game:
  - the story scripts as nodes, lines, choices, jumps and conditions
    (from 2724's `StoryGraph` in `shuyi_vn::graph`, which the story
    checks and the route chart also read), and
    the mystery commands in them (`@clue`, `@person`, `@investigate`,
    `@deduce`, `@testimony`);
  - each investigation's hotspots, conditions, actions and `exit`
    (2728);
  - each deduction's `available` conditions and answers (2729);
  - each testimony's statements, presses and contradictions (2730);
  - `mystery.ron`: the closing endings, the meter.
- **The abstract state**, so the search is finite and small. A state
  is:
  1. **a story position**, kept only where the player decides
     something: a choice, an investigation, a board the story waits on,
     a testimony statement. A run of lines with no choice between two
     such points is one edge, carrying what its commands do;
  2. **the case facts**: the clues found with their stage, the people
     met with their stage, the deductions made, the contradictions
     found. These only ever grow (2727 moves a clue's stage only
     forward), so along any path the facts form a rising chain;
  3. **the dialogue variables that some condition reads**, each
     abstracted to the literal values the scripts compare it with, plus
     `Other`. A variable set by anything but a literal is `Unknown`,
     and a condition on it takes both branches. This can over-report,
     never under-report, and the report says when `Unknown` was
     involved;
  4. **the mode stack**, at most 4 deep (deeper is an error naming the
     path).

  Left out on purpose, because no rule depends on them: the cursor, the
  hotspots examined (no condition reads them), blanks drafts (a blanks
  deduction is one edge, taken when every slot's answer can be
  offered), wrong answers and the penalty count (checked apart, below),
  hints, and everything ADR-0016 §3 calls the player's.
- **The search**: breadth first from the start, with every state seen
  kept in a `BTreeMap` (the memo) and its parent edge, so each state is
  expanded once and the shortest breaking path can be read back.
  - **A limit**, `fair.max_states` in `mystery.ron` (default 200 000).
    Going past it fails the check loudly: "the case has more than N
    states", with the places and choices that made the most states. It
    never passes on a partial search.
  - The bound: positions are finite, and the facts only grow, so a
    path has at most (number of facts + 1) fact sets on it, and the
    whole search at most (positions) × 2^(number of facts) states. In
    practice it is far fewer, because most facts come in an order the
    story sets. The worst case is many clues that can be found in any
    order: the example's size (about 8 clues, 3 people, 2 deductions)
    is at most 2^13 = 8 192 fact sets per position. Write the measured
    state count of each fixture, and of a case of the example's size
    built by the test (8 clues found in any order, 3 people, 2
    deductions), in the Completion notes. The example itself is 2732's,
    which writes its own count. The README says how to split a case
    that reaches the limit,
    for example into chapters whose clues are all found before the next
    chapter starts.
  - If that case comes near the limit, add one reduction:
    examines whose `when` has only positive conditions, whose node has
    no choice, and that can't make `exit` hold are applied together in
    file order as one edge, when no `NotFound` or `NotMade` condition
    anywhere in the case names what they find. Add it only with a
    property test that the reports are the same with and without it on
    every fixture and on random small cases.
- **The rules**, each a pure function over the explored graph:
  - `solvable`: in every reachable state where the player is asked
    (`@deduce` holds the story, or a testimony waits for a
    contradiction), the answer can be given from the facts of that
    state, plus what the asked mode itself can give (a testimony's
    presses). For links: every item of the right set is found or met.
    For blanks: every slot's answer is offered. For testimony: some
    statement's `contradicted_by` holds a found clue;
  - `no_dead_ends`: every reachable state can reach a closing ending
    (`mystery.ron`'s `closing_endings`), found by searching the graph
    backwards. Endings a writer lists in `other_endings` (a field this
    ticket adds to `mystery.ron`, empty by default: a bad ending by
    choice) are allowed stops, not dead ends;
  - `meter_has_outcome`: if there is a meter, every penalty source
    (each testimony, each deduction with `penalty` above 0) has an
    `on_empty`, and a `GameOver` node reaches an ending;
  - `wrong_answers_find_nothing`: a `wrong` or `default_wrong` node
    that finds a clue or meets a person is an error, because wrong
    answers are left out of the graph and must stay out;
  - `unused_clues`: a warning for each `Evidence` clue that no
    deduction answer, blanks answer or `contradicted_by` names. A
    `Colour` clue (2727) gets none.
- **The report**: each finding has the rule, the file and line of the
  thing that breaks, and for a path rule the path from the start as
  steps a writer reads, for example:
  - `case.dlg:12 choice 2 "ask.knife"`;
  - `examine desk in study`;
  - `case.dlg:40 @deduce motive`.

  Errors fail `shuyi check`; warnings don't. The JSON form follows the
  tool contract (1129), with `limits` saying when `Unknown` variables
  were involved.
- **Plugged into `shuyi check`** beside 2724's `shuyi_vn::check`,
  called by 1129's check operation the same way: it runs when the game
  has the `mystery` feature on, so it runs in every such
  game's CI (1129's templates run `shuyi check`). If 2727's
  `check::references` isn't wired yet, wire it here too.
- **Hints** (ADR-0016 §9):
  - `*.deduction.ron` gains `hints`: an ordered list of text keys,
    empty by default, so old files load unchanged;
  - `MysteryCommand::ShowHint { deduction }` shows the next one,
    refused when none are left. `CaseState` keeps the hints shown and
    the wrong tries per deduction, so a game's own rule ("a hint after
    3 wrong tries") can read them and send the command;
  - `BoardScene` (2729) lists the hints shown; the skin draws them as
    glyph cells;
  - a hint key that doesn't exist is an error in the check.
- **Fixture cases** in `crates/mystery/tests/fair/`, each a tiny case
  that breaks one rule and nothing else:
  - `good`: passes with no warnings;
  - `late-clue`: the clue a `@deduce` needs is found only after it;
  - `one-branch`: the clue is found on one choice branch only; the
    report names that choice;
  - `lost-chance`: a hotspot with `NotMade(x)` holds the only way to a
    clue needed after `x`;
  - `no-contradiction`: a testimony's contradicting clue can't be found
    before it;
  - `dead-end`: a `Go` leads to an investigation with no exit and no way
    back;
  - `game-over-no-ending`: the meter's `GameOver` node reaches no ending;
  - `wrong-node-clue`: a wrong answer's node finds a clue;
  - `unused-clue`: one warning; `colour-clue`: no warning;
  - `too-big`: a case built by the test that passes the limit, which
    fails loudly.

**Out (do not do):**
- story checks that aren't about the case (unknown characters, lines no
  path reaches): 2724 owns them;
- the route chart (2724);
- the rule that decides when a hint is shown: that is each game's;
- checking that a clue is *convincing*: the check is about what can be
  found, not about the writing;
- the example and its CI (2732).

## Implementation steps

1. Write `CaseGraph` and the abstract `State` (with `Ord`, so it can sit
   in a `BTreeMap`), built from the assets and 2724's story graph.
2. Write the search with the memo, the parent edges and the limit.
3. Write each rule and the backward search for `no_dead_ends`.
4. Write the report: steps as lines and choices, text and JSON.
5. Call the check from `shuyi check` beside `shuyi_vn::check`.
6. Add `hints`, `ShowHint`, the tries count and the board's hint lines.
7. Write the fixture cases and their expected reports.

## Acceptance criteria

- [ ] Each fixture case gives exactly its one finding, with the file
      and line, and for path rules the path, matching a checked-in
      report.
- [ ] `good` passes `shuyi check` with no warnings.
- [ ] `too-big` fails with the limit message, and no fixture passes on
      a partial search.
- [ ] The check gives the same report, byte for byte, on two runs and
      on native and the web build's CI job.
- [ ] A hint is shown by `ShowHint`, replays, and a missing hint key is
      reported.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the abstraction of each kind of edge; each rule on hand-built
  graphs; the variable abstraction with `Unknown`; the report's path.
- Property: on random small cases (a few clues, rooms and choices,
  variables set only to literals), `solvable` and `no_dead_ends` give
  the same answer as a brute-force search that plays the real
  `MysteryRules` in every command order (leaving out wrong answers, as
  the check does); in cases with no `NotFound` or
  `NotMade` condition, adding a clue find to a script never turns a
  `solvable` pass into a failure.
- Fixture: the cases above with checked-in reports.
- Integration: `shuyi check --json` on `good` and `late-clue`.

## Completion notes
