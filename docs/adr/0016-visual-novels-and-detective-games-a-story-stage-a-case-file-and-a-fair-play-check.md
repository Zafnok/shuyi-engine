# ADR-0016: Visual novels and detective games: a story stage, a case file, and a fair-play check

- **Status:** Accepted
- **Date:** 2026-10-18
- **Related tickets:** 2721–2726 (visual novels), 2727–2732 (detective
  games), all parked until after Visions of Shuyi launches; 1121
  (pointer input), 1122 (triggers), 1123 (animation clips), 1129 (the
  tool contract), 2202 (Yarn Spinner and Ink), 2501 (templates), 3002
  (controls fit)
- **Extends:** ADR-0003 §1–§2 (stages, turn rules), ADR-0007 (formats
  are plugins), ADR-0009 (modules), ADR-0015 §1 (the *Kind of game*
  concern)

## Context

Nick, 2026-10-18 (`docs/decisions.md`): "I wanna be able to support ...
visual novel style games and detective games". Parked until after
Visions of Shuyi launches.

What the engine has after the carve (game 1103–1117):

- **Dialogue**: `.dlg` scenes (game ADR-0011, game ADR-0055) with
  speakers, choices, `@if` and commands such as `@music`, loaded as the
  `DialogueScene` asset kind (ADR-0007). 2202 adds Yarn Spinner and Ink
  as formats of the same kind.
- **Portraits**: PNG pictures named by RON sidecars (ADR-0004).
- **Cinematics**: timed shots on a timeline (game ADR-0060).
- **Text by key**, language packs and line ids for voice and
  translation (game ADR-0045, game ADR-0046).
- **Turn rules** (ADR-0003 §2): state changes only by commands, and a
  run is its seed plus its commands.
- **Storage** by key for save slots and settings (ADR-0008 §2).

What visual novels need beyond that, from Ren'Py, TyranoBuilder and
the games players compare them to:

- characters with poses and expressions, placed and moved on screen;
  backgrounds and full-screen event pictures; transitions;
- a text box with a name box, text that appears at a speed the player
  sets, and choices;
- the **backlog** (the text so far), **skip** (only text already read,
  unless the player says otherwise), **auto** (advance after a pause),
  **going back** to an earlier line, and **saves at any line** with a
  thumbnail;
- a **gallery** of event pictures and endings, kept across playthroughs;
  a **route chart**;
- voice lines tied to lines.

What detective games need, from games such as the Ace Attorney series,
Return of the Obra Dinn, Her Story, the Danganronpa series and The
Case of the Golden Idol:

- **clues** found by examining places and talking to people, kept in a
  case file;
- **investigation scenes**: hotspots on a picture or things in a world
  to examine, people to question, places to go;
- **deductions**: joining clues into a conclusion, or filling the blanks
  of a sentence ("___ was killed by ___ with ___"), confirmed a few at a
  time so guessing doesn't pay;
- **testimony**: statements a witness makes, where pressing one asks
  more and presenting a clue at the wrong statement shows a
  contradiction, with a penalty for wrong guesses;
- and the thing players hate most when it fails: **fair play**. A case
  must be solvable from what the player can find before they are asked,
  and the game must never leave them stuck.

The engine's rules hold: deterministic turn rules, a scene that is plain
data and a skin that paints it (ADR-0002 §4), menus and text boxes as
glyph cells (ADR-0003 §5), actions not keys (`AGENTS.md` rule 8), and
text as the source of truth for every format (ADR-0007).

## Decision

### 1. Two modules, one on top of the other

- **`shuyi-vn`** (`crates/vn`): the visual novel module.
- **`shuyi-mystery`** (`crates/mystery`): the detective module. It
  depends on `shuyi-vn`, because questioning and testimony are told
  the same way a visual novel is. Its README says so (ADR-0009 §4).
- Both carry every module duty of ADR-0009 §4: metadata, a README in
  the fixed order, a feature of the `shuyi` crate (`vn`, `mystery`), a
  catalog row, an example, a skill (`shuyi-vn`, `shuyi-mystery`), and
  `needs` (ADR-0012 §1).
- They answer the *Kind of game* concern (ADR-0015 §1):
  `visual-novel` and `mystery`. Either may be the first to add the
  concern if ADR-0015's tickets haven't.
- **They serve every perspective.** A visual novel is drawn in screen
  space over any stage or alone. A detective game can investigate on
  pictures (a visual novel's screen) or in a world (a top-down town,
  1125), and both use the same case file.

### 2. A story is turn rules over the dialogue the engine already has

- **`VnState`**: where the story is (the scene and line), its variables
  (the dialogue variables `.dlg` already has), and the *stage*: the
  background, the characters on screen with their pose, expression and
  place, the music, and the event picture if one is showing.
- **`VnRules: TurnRules`** (ADR-0003 §2). The commands are `Advance`,
  `Choose(index)` and `Jump(label)` (for a game's own menus). A story is
  its commands, so:
  - **going back** is replaying the history minus its last commands,
    as a battle is rewound (game ADR-0039);
  - **a save** is the history (with a snapshot every N lines, so loading
    is quick). Each saved command records the line id it was sent at, and
    a choice records its option's line id, so a save stays valid when a
    patch fixes, adds or reorders lines. Loading starts from the latest
    snapshot whose line still exists;
  - tests play a story headlessly and read `VnState`.
- **Stage directions are dialogue commands.** The dialogue runner
  already passes commands through; `shuyi-vn` reads these:

  | Command | Meaning |
  | ------- | ------- |
  | `show <who> [pose] [expression] [at <place>] [with <transition>]` | put a character on screen, or change them |
  | `hide <who> [with <transition>]` | take them off |
  | `move <who> to <place> [over <ms>]` | move them |
  | `bg <name> [with <transition>]` | change the background |
  | `event <name>` / `event off` | show a full-screen event picture (and unlock it in the gallery) |
  | `music`, `sound` | music and sounds by cue id, as `@music` already works in `.dlg` |
  | `voice <id>` | play this voice instead of the line's own; a line's voice is otherwise found by its line id |
  | `wait <ms>`, `shake`, `flash` | timing and effects, as data for the skin |
  | `ending <id>` | record an ending reached |

  They are written `@show …` in `.dlg`, `<<show …>>` in Yarn Spinner and
  `# show …` as an Ink tag. 2202's mapping table gains these rows. A
  game lists its own commands in `vn.ron`, so an unknown command is an
  error with its file and line, never ignored. In Ink, where tags also
  carry metadata, a tag is a command only when its first word is a stage
  direction or one of the game's commands; any other tag is left alone.
- **Events, not state, for what the player keeps.** `event` and
  `ending` change the story's state only by what is on screen. `VnRules`
  emits `EventPicture` and `Ending` events, and the screen writes them
  to the player's profile (§3).
- **Characters are data**: `*.character.ron` names a character, their
  name text key, their colour role, and their pictures as **layers**
  (body per pose, face per expression, extras), composed at draw time
  as Ren'Py's layered images are. Screen places (`left`, `center`, `right`,
  and a game's own) are data in `vn.ron`.

### 3. The player's conveniences live outside the story

Some things a visual novel does are the player's, not the story's. They
never enter `VnState`, so they never change a replay:

- **The text speed**, **auto** mode and its pause, and **skip** mode.
  They change how fast `Advance` is sent, nothing else.
- **What has been read**: a set of line ids per player profile, saved
  with the settings (ADR-0008 §2's storage), not with a save. Skip stops
  at the first unread line unless the player chose "skip all".
- **The backlog**: a view built from the history, with each line's
  voice replayable.
- **The gallery and endings reached**: in the profile, across
  playthroughs.
- Each is an action in the player's keymap, and every screen fits each
  target's controls (3002), so a visual novel runs on a gamepad, a
  Steam Deck and a Playdate. The required actions are few: advance, move
  between choices, and a quick menu. Everything else (the backlog, skip,
  auto, going back, saving) is also reachable from that menu, so it fits
  the Playdate's controls.

### 4. The scene and the skin

- **`VnScene`** is plain data: the background, each character's layers
  and place, the event picture, the text box (speaker, the text so far
  as revealed, whether the line is read), the choices, and the effects
  in play. Tests read it.
- **Text, the name box and choices are glyph cells** (ADR-0003 §5), so
  they use the engine's text by key, fonts and languages. Pictures are
  sprite items in screen space.
- **Transitions** (cut, fade, dissolve, slide, a game's own) are data;
  the skin draws them, the Harness can skip them.
- **Saves show a thumbnail** drawn from the scene with the same
  frame-to-pixels code `frame-png` uses, moved into the crate that owns
  the frame if it isn't there yet, so it needs no window and no
  `shuyi-cli`.

### 5. Checking a story

`shuyi check` (1129) reads every story script and reports, with file and
line:

- a character, pose, expression, background, event picture or voice that
  doesn't exist;
- a variable read before any line sets it;
- a line no path reaches, and a choice that leads nowhere;
- an ending no path reaches.

`shuyi route-chart` writes the story's graph (scenes, choices, endings)
as Mermaid text, and the visual novel's route chart screen shows the
parts the player has reached.

### 6. The case file

`shuyi-mystery` keeps what the player knows as data:

- **Clues** (`*.clue.ron`): an id, a name and description by text key,
  a picture, and how it can change (a clue can be updated when more is
  learned).
- **People** and **places**, the same way.
- **`CaseState`**: the clues found, the people met, the deductions made,
  the penalties taken. It sits beside `VnState` in one turn rules value,
  `MysteryRules`, so a case is still its commands.
- Clues are found by dialogue commands (`@clue <id>`, `@person <id>`)
  and by examining (§7). The case file screen reads `CaseState`.
- `shuyi-mystery` adds its commands to `shuyi-vn`'s list the way a game
  does (§2), and `shuyi-vn` gives modules two things they need: a
  command that holds the story as a choice does, and running a dialogue
  node and coming back (for examining and pressing).

### 7. Investigating

- **On a picture**: hotspots (rectangles or polygons in picture pixels)
  on a background, each with conditions (only after a clue, only before
  a deduction) and an action: examine (runs a dialogue node), talk, go
  to a place.
- **In a world**: the same hotspots as triggers (1122, boxes only) on
  things in a tick stage, so a top-down town can be investigated. The
  tick stage holds the `MysteryRules` value and sends it `Examine` from
  its step when the player acts on a trigger, so the stage's
  `TickRecord` replays the investigation too.
- **Pointing and choosing**: the pointer (1121) where there is one. With
  only a d-pad, the cursor jumps between hotspots in reading order, so
  investigation fits every target (3002). Pixel hunting is not a
  feature: the README tells writers to mark hotspots, and the scene
  says which ones are found.

### 8. Deductions and testimony

- **Deductions** (`*.deduction.ron`) come in two shapes:
  - **links**: a set of clues (and people, places) the player joins on a
    board to reach a conclusion. The right set unlocks it; a wrong set
    can have its own answer, or a default one;
  - **blanks**: one or more sentences with slots, each filled from a
    list. Answers are confirmed in groups of a set size (default 3, as
    Return of the Obra Dinn does), and only when the whole group is
    right: any N right slots confirm, first in slot order, and a wrong
    group reveals nothing. When fewer than N slots are left, the group
    shrinks to what is left.
- **Testimony** (`*.testimony.ron`): a witness's statements in order.
  Each can be **pressed** (runs a dialogue node) and has the clues that
  contradict it. Presenting the right clue at the right statement moves
  the story on; a wrong one costs a penalty.
- **Penalties are data**: one meter per case (a testimony may have its
  own), and what happens when it is empty: start the testimony again (or
  only refill the meter, outside a testimony), a game over, or nothing.
- All of these are commands in `MysteryRules`, so they replay.

### 9. Fair play, checked

`shuyi check` gains a case check (`shuyi_mystery::fair`), which reads
the scripts, hotspots, deductions and testimony as a graph:

- **Solvable**: every deduction and contradiction needs only clues that
  can be found before the player is asked it, on every path. The player
  is *asked* when the story waits on it: a `@deduce` command that holds
  the story, or a testimony that waits for a contradiction. A board
  deduction the story never waits on is optional and only needs to be
  solvable at all.
- **No dead ends**: from every reachable state, the case can still be
  closed. Endings are listed in `mystery.ron` as *closing* (the case is
  solved) or *other* (a bad ending, a game over); reaching an *other*
  ending on purpose is not a dead end. Wrong answers and penalties are
  outside the graph, except that a penalty that empties the meter must
  have an outcome.
- **Bounded**: the check searches an abstract state (the story position,
  the clues found and the deductions made), breadth first and
  memoized, up to `fair.max_states` (a setting). Past the limit it fails
  loudly, never passes.
- **Every clue matters or says so**: a clue that no deduction or
  testimony uses is a warning, unless marked as colour.

It reports the path that breaks a rule, as lines and choices. It runs
in CI on the example and on every game that uses the module. **Hints**
are data too: an ordered list per deduction, shown when a game's own
rule says so.

## Consequences

- A writer makes a visual novel with text files they may already know
  (Yarn Spinner, Ink, or `.dlg`), characters and backgrounds as data,
  and no code. A game with code can still add its own screens.
- Every visual novel and mystery made with the engine replays, runs
  headless, and gets its saves, backlog and going back from the turn
  rules it already has.
- The fair-play check is a feature few engines give: the engine says
  when a case can't be solved, before a player finds out.
- Stage directions become part of the dialogue formats' contract, so
  2202's mapping and any later format must handle them.
- Two more modules to document and keep, parked until after the game
  launches. None touches the game; its own dialogue keeps working as
  before, because these commands are only read with `shuyi-vn` on.

## Alternatives considered

- **Import Ren'Py scripts** — Ren'Py is the most used visual novel
  engine, but its scripts are Python. Running them would mean a Python
  interpreter or a partial translator that breaks on real games. Writers
  get Yarn Spinner and Ink instead; a later research ticket can look at
  a subset if people ask.
- **A visual novel as a tick simulation** — text would advance by time,
  and going back and saving at a line would need step logs. A story is
  commands; turn rules fit it.
- **The player's conveniences in the story's state** — read-text and
  auto mode would change replays and saves for no reason. They are the
  player's (§3).
- **The detective kit inside `shuyi-vn`** — every visual novel would pay
  for clues, boards and checks it doesn't use (ADR-0009 §1).
- **One deduction shape** — games differ: boards of links, fill the
  blanks, contradictions. Three small shapes as data cover the games
  players know; a game can add its own as commands.
- **No fair-play check, only guidance for writers** — the failure it
  catches is the one that ruins a mystery, and the scripts already hold
  the graph it needs.
