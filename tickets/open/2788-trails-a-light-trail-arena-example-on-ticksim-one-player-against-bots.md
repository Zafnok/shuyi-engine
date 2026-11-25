---
id: "2788"
title: "trails: a light-trail arena example on TickSim, one player against bots, for desktop, web and the Playdate"
type: feature
milestone: "Engine: examples"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2612"]
nick_input: none
completed:
---

# 2788 — trails: a light-trail arena example on TickSim

## Context

Nick's decision of 2026-11-25 (`docs/decisions.md`) makes the next
example game an original light-trail arena that later goes online:
rooms with capped seats, bots in the empty ones, players dropping in and
out. It is grid based, top down and real time. This ticket is its
offline half: the rules, the bots and the screens, one player against
seven bots. 2789 puts the same rules in a server's rooms, and 2790 adds
the online client.

**Original, never a copy.** The game is called *trails*. It is never
named or styled after a film or another game: no film or product names
in its text, code or files, no logos, no borrowed fonts, and no glowing
"neon grid" look. It uses flat colours of its own from `palette.ron`
and a one-bit palette for the Playdate. The genre (riders that leave a
solid trail, last one riding wins) is old and common; the look and
words are ours.

Its nearest relative is `examples/snake` (2739, done): a real-time grid
on `TickSim` with a title, a stage, a pause menu and an end screen,
goldens, a record, a long run, a web build deployed to Pages by
`.github/workflows/pages.yml`'s loop over `examples/*/shuyi.ron`, and a
site card (`site/content/examples/snake.md`). Trails copies that
layout and those tests.

What the code gives, checked on this commit:

- `TickSim` and `TickInput` (`crates/core/src/tick.rs`); 2612 raises
  `MAX_SEATS` to 8, which eight riders need.
- `TickDriver` (`crates/ui/src/tick.rs`) steps a sim at the platform's
  60 steps a second (`shuyi_core::timer::STEPS_PER_SECOND`) and builds
  **seat 0 alone** for each step (`TickInput::one`).
- Snake paints glyph cells of 8 × 16 pixels (`snake::CONSOLE`, 50 × 15
  cells, a tile two cells across). A 64 × 36 arena needs smaller tiles:
  the frame's pictures (`GlyphBuffer::add_sprite`, a `Sprite` with
  `dest` in console pixels and `Paint::Solid(Rgb)`) can paint a 6 × 6
  pixel tile in any colour.
- `shuyi_core::bots` is for turn games (`TurnRules`); nothing in the
  core plans bots for a tick game, and none is wanted there (below).

### Two speeds, one rule set

Online, a room steps 15 times a second, and each step sends every
client one frame (2614, 2615: "trails runs 15 steps a second"). Offline,
`TickDriver` steps 60 times a second. So the rules are split in two:

- **`Round`**: one round of the arena, a `TickSim` in which **each step
  moves every living rider one cell**. The server and the online client
  step this (2789, 2790).
- **`Offline`**: a `TickSim` for one machine, over a series of `Round`s.
  At each of the platform's steps it latches seat 0's turns, and every
  `60 / moves_per_second` steps it fills seats 1–7 with `bot_input` and
  steps the `Round` once. It also runs the pause between rounds and
  counts the wins. `TickDriver` drives it as it drives snake, and its
  `TickRecord` (seat 0 alone, at 60 a second) replays exactly, because
  the bots are a pure function of the `Round` and `Offline`'s own seeded
  `SimRng`.

This keeps the engine's `TickDriver` unchanged: no hook for extra seats
is needed.

## Nick input

None.

## Scope

**In:** a new workspace member, `examples/trails` (the workspace's
`examples/*` glob already takes it), laid out as `examples/snake` is.

- **`files/arena.map`** (the engine's `.map` format, as snake's
  `board.map`): 64 × 36 cells, a border of walls, open floor, and 8
  spawn marks as the game's own feature type (`Spawn(u8)`, seat 0–7),
  spread round the arena, each facing inwards.
- **`files/tuning.ron`** (`trails::rules::Tuning`, read and checked as
  snake's is):
  - `moves_per_second: 15`, which must divide 60 (5, 6, 10, 12, 15, 20,
    30 or 60), else an error;
  - `max_round_steps: 2700` (three minutes at 15 a second), after which
    the round ends with no winner; it caps a catch-up's size online
    (2614);
  - `pause_steps: 45` (three seconds) between rounds;
  - `clear_on_crash: true`: a crashed rider's trail vanishes at once;
  - `bot_look: 128`, the most cells a bot's flood fill counts;
  - `bot_mistake_per_mille: 30`, how often a bot picks at random among
    its safe moves instead of the best.
- **`src/rules.rs`**, pure, with `#![deny(clippy::float_arithmetic)]`
  as snake's has, no I/O and no clock:
  - `Round::new(arena: &Arena, tuning: &Tuning, seed: u64) -> Round`,
    the riders on their spawns. `Round` implements `TickSim`.
  - **Actions** are absolute directions (`Up`, `Down`, `Left`, `Right`
    in the game's `ActionSet`); a press turns the rider. A press
    straight back is ignored. Two presses in one step: the last wins.
  - **Each step**, every living rider moves one cell, leaving its old
    cell as its trail. Then, all at once:
    - a rider whose target is a wall, any trail (its own included) or a
      living rider's head crashes;
    - two riders that target the same cell both crash;
    - two riders that swap cells (head on) both crash.
  - `Event`: `Crashed { seat }`, `RoundOver { winner: Option<u8> }`.
  - The round is over when at most one rider is riding (that one wins;
    none left is a draw), or at `max_round_steps` (a draw).
  - `Round::winner(&self) -> Option<Option<u8>>`: `None` while riding,
    `Some(winner)` when over. 2789's `RoomGame::round_winner` calls it.
  - `hash()` covers every cell, every rider and the step count, through
    `shuyi::core`'s state hash as snake's does.
  - **`Offline`**: `Offline::new(arena, tuning, seed, players: u8)`;
    seats `0..players` read the input, the rest are bots (`players` 0
    is all bots, for the goldens and the long run). It steps the
    `Round` every `60 / moves_per_second` of its own steps, holding the
    last direction pressed in between as that step's seat-0 press.
    After a round ends it waits `pause_steps` steps of its own (in
    moves), then starts the next `Round` with a seed drawn from its
    `SimRng`. It keeps the wins of each seat, and its `Event` adds
    `RoundStart { round: u32 }`.
- **`src/bots.rs`**, pure, no floats:
  `pub fn bot_input(round: &Round, seat: usize, rng: &mut SimRng) ->
  SeatInput`.
  - It looks at the three moves that aren't straight back. A move is
    safe when its target cell is free and not a cell any other living
    head could also move into this step.
  - For each safe move, a flood fill from the target over free cells
    counts the room there, up to `bot_look` cells.
  - It picks the move with the most room; ties, and the
    `bot_mistake_per_mille` cases, are picked among the safe moves with
    the `rng`. With no safe move it keeps straight.
  - It presses the move's direction only when it differs from the
    heading, so a bot's input is empty most steps (and costs little on
    the wire, 2612).
  - The same function serves offline play here and the server's bots
    (2789); offline, `Offline`'s `SimRng` is its `rng`.
- **The screens** (`src/screens.rs`), as snake's:
  - **Title**: the name, a line on how to play (keys named through the
    player's keymap, never in text), Confirm to ride.
  - **Arena**: `TickDriver<Action>` over `Offline` with `players: 1`,
    recording; the HUD line shows each seat's wins in its colour, the
    player's marked "you". **Pause** over it, as snake's.
  - **Round over**: an overlay over the arena during the pause, "You
    win", "Seat N wins" or "Nobody wins", then the next round starts on
    its own. Cancel leaves to the title.
  - The scene (`scene()`, a plain-data view of the arena, the riders,
    their trails and the seat colours) is what tests read; the skin
    paints it.
- **The skin:** each cell is a 6 × 6 pixel picture
  (`GlyphBuffer::add_sprite`) cut from a tiny white image of the game's
  own (`files/tile.png`, made here, ours, no third-party asset) and
  painted `Paint::Solid` in the seat's colour; trails a darker shade of
  the head's colour; walls and floor from the palette. The arena is
  384 × 216 pixels under a 16-pixel HUD line, inside the 400 × 240
  frame. If `add_sprite` can't do this, use glyph cells instead (an
  8 × 8 half of a cell per tile, a 50 × 28 arena) and say which in the
  completion notes; the rules don't change.
- **Files:** `en.ron` (every player text), `keymap.ron` (the directions,
  Confirm, Cancel, Pause; never a key in code), `palette.ron` (a colour
  for each seat, picked to be told apart, and checked against the
  palette's contrast rule if one exists), `palette@onebit.ron` for the
  Playdate: the player's rider solid, the bots in the one-bit palette's
  patterns as snake's `palette@onebit.ron` does, `audio.ron` and two
  sounds in `files/sfx/` (a crash and a round's start) written as a few
  numbers, as snake's are.
- **`shuyi.ron`**: as snake's, with targets `desktop`, `web`,
  `steam-deck` and `playdate` at 400 × 240, a `web` section (so
  `pages.yml` builds and deploys it with the other examples), a
  `preview` (an APNG of a round of bots), `check_keys`, `notices`.
- **CI:** a step `Trails' golden table on wasm32` in `ci.yml`'s
  `determinism` job beside snake's, and trails in `long-run.yml`'s loop
  beside snake (its long run below).
- **README** (`examples/trails/README.md`): what it is, how to play,
  how it is built, its line count from `cargo xtask template-lines
  examples/trails`.
- **Website card:** `site/content/examples/trails.md`, as snake's.
- **Either order with 3536:** if 3536 is done, add `trails` to CI's
  `export` matrix with a `packed` feature and a `disclosure` part as
  3536 gave snake; if not, nothing to do here (3536 then adds trails
  beside snake).

**Out (do not do):**
- Networking: the room rules and server (2789), the online client
  (2790).
- Split-screen play on one machine (2774, parked).
- The arcade template part (2787): trails follows snake's layout, and
  2787 changes nothing here.
- Power-ups, boosts, speed changes, scores beyond wins.
- A generic bot module for tick games in `shuyi-core`: trails' bot is
  game code and stays in the game (`AGENTS.md` rule 4's spirit for
  examples: the engine supplies hooks, 2614's `RoomGame::bot_input`).
- A change to `TickDriver`: `Offline` fills the bots' seats itself.

**Estimated size:** about 1,300 lines (code 750, tests 400, data and
docs 150).

**Split:** past 1,000 lines, split off the flood-fill bot (`bots.rs`'s
look-ahead and its tests) into a follow-up ticket written then, which
2789 then also waits for; here the bots keep straight and turn to a
random safe move only when blocked.

## Implementation steps

1. Read `examples/snake` whole (its `Cargo.toml` features, `lib.rs`,
   `rules.rs`, `screens.rs`, `tools.rs`, its tests and `shuyi.ron`),
   2739's completion notes, and `GlyphBuffer::add_sprite`. Copy snake's
   skeleton to `examples/trails` and rename.
2. `rules.rs`: `Tuning`, the map's `Spawn` feature, `Round` and its
   tests (every crash case).
3. `Offline`: pacing, the latch of seat 0's turns, rounds, the pause,
   the wins; its tests.
4. `bots.rs` and its tests.
5. The scene, the skin and the screens; the files.
6. Goldens, the record, the long run, the integration tests, the
   preview.
7. `shuyi.ron`, the CI steps, the README and the site card.
8. Completion notes, with `Book:` and `Skills:` lines (the book's
   online chapter, 2854, walks through trails once it is online; the
   skill pack is taught by 2855).

## Acceptance criteria

- [ ] `cargo run -p trails` plays on desktop; `cargo shuyi web
      --release` in its folder builds, and the page plays in the
      browser pane; `cargo shuyi check --target all` passes, the
      Playdate's frame included.
- [ ] Rules tests cover every crash case: a wall, another's trail, its
      own trail, another's head, two riders into one cell, a head-on
      swap; and a press straight back is ignored.
- [ ] A round ends with one rider left (that seat wins), none (a draw)
      and at `max_round_steps` (a draw); a tuning with
      `moves_per_second: 7` is refused with an error.
- [ ] A bot never moves into an immediate crash when a safe move exists
      (a property over seeded arenas).
- [ ] `Offline` with 8 bots plays 10 rounds to a golden hash, in
      `tests/golden/trails.ron`; CI's `determinism` job runs it on
      wasm32 too.
- [ ] A `TickRecord` of a played round (seat 0 at 60 steps a second)
      replays to the same hash through `Offline`.
- [ ] Screens are tested through the scene and the state, never cells:
      title → arena → round over → next round, and the pause menu.
- [ ] `cargo shuyi check-keys` and `check-text` pass: no key named in
      code or text.
- [ ] Nothing in the game's name, text, files or look names or imitates
      a film or another game (a reviewer reads `en.ron`, the README, the
      site card and the preview).
- [ ] `cargo xtask check-layers` passes, and `rules.rs` and `bots.rs`
      deny float arithmetic.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass, `wasm` and `web-render`
      included.

## Tests required

- Unit: `src/rules/tests.rs` (each crash case, the round's end, tuning
  checks, `Offline`'s pacing: a press between two moves turns at the
  next move, never lost and never twice); `src/bots/tests.rs` (no
  immediate crash with a safe move; the bigger room is picked; ties use
  the seed).
- Property (proptest): random presses over 500 moves never panic, and
  the same seed and presses give the same hash.
- Golden: `tests/golden.rs` and `tests/golden/trails.ron` (8 bots, 10
  rounds), run on wasm32 in CI.
- Record: `tests/records/` with a record of a player's round.
- Integration: `tests/it/` as snake's (screens, playthrough, files,
  snapshot of the scene, the tools' contract).
- Long run: `long/bots-play-rounds.long.ron`, for `cargo shuyi bench
  --long --twice`: the heap and sounds stay flat.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
