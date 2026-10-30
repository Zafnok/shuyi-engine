---
id: "2737"
title: "Parked: Example: a card battle on the turn path, with a deck, a hand, energy and foes that show their intent"
type: feature
milestone: "Engine: examples"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2511", "2114"]
nick_input: none
completed:
---

# 2737 — Example: a card battle on the turn path

> **Parked** (Nick, 2026-10-20, choosing a gallery of small games as a kind of game the engine serves; Visions of Shuyi is "the first game I want to be launched with the engine"): like real-time strategy, visual novels and detective games, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6,
Unreal Engine 5.8 and the 2D engines GameMaker, Defold and Construct 3.
Several of them teach kinds of game through small working games:
Bevy's `examples/showcase` (`breakout.rs`, `mines.rs`), Godot's demo
projects (https://github.com/godotengine/godot-demo-projects/tree/master/2d),
Unity's 2D samples and Construct 3's Example Browser. Neither Bevy's
showcase nor Godot's 2D demos has a card game, and Shuyi has none. A
deck, a hand and an opponent who shows what it will do next make a turn
game with hidden information: the shape of card battles, board games
with a bag of tiles, and duels.

What Shuyi has today:

- one example, `examples/hello-grid`, a turn game on a grid map;
- the turn path: `TurnRules` (`crates/core/src/turn.rs`); `Sides`,
  `Bot`, `play` and `RandomBot` (`crates/core/src/bots.rs`); and
  `SimRng` with `roll_below`, the `RandomSource` trait and
  `ScriptedRng` (`crates/core/src/rng.rs`). There is no shuffle and no
  stream per purpose, so a deck would be shuffled by hand;
- widgets for a game with no map: `Menu` and `ListCursor`
  (`crates/ui/src/widgets/menu.rs`, `list.rs`), and
  `GlyphBuffer::panel` (a box with a title in its border) and
  `GlyphBuffer::meter` (`crates/ui/src/frame/draw.rs`);
- a game's own data kinds through `shuyi_assets::ron::Ron<T>`, as
  hello-grid reads `en.ron` (`formats` in
  `examples/hello-grid/src/lib.rs`), and their errors through
  `GameTools::check` (`crates/cli/src/game.rs`);
- the short path a small game is written on (2511, ADR-0036):
  `BasicGame::with_own` for a game with data of its own, read by an
  `Own<T>`, with `.console`, `.screens` and the default
  `BASIC_CONSOLE` (48 × 24) (`crates/platform/src/basic.rs`);
  `BasicData` and `BASIC_GLYPHS` (`crates/ui/src/basic.rs`);
  `BasicTools`, with `harness` and `picture` for a game's own
  `GameTools` (`crates/cli/src/basic.rs`); and `shuyi::prelude` with
  `run_game`. `UiAction`'s six actions are an `ActionSet`. It plays no
  sounds (2511's Out).

What is planned:

- 2114 (After MVP): `SimRng::stream`, and `shuffle`, `pick` and
  `pick_weighted` on `RandomSource`. Its docs deal a shuffled deck;
- 2113 (After MVP): `Minimax` and `Mcts` bots over `TurnRules`, which
  search only copies a game's `Determinize` makes, so they never see
  the luck to come. Its Out names this ticket;
- 2115 (After MVP): `Countdown` in `shuyi_core::timer`, for a status
  that lasts some turns;
- 2225 (After MVP): menu sounds from `.sfx.ron` files, which the short
  path reads with no code;
- 2736 (parked): the match-3, the gallery's other turn game.

Why it matters:

- **The game first:** Visions of Shuyi needs none of this, so it waits
  for the launch.
- **Other 2D games:** Nick said on 2026-10-14 that the engine serves
  games "not constrained to tactical rpgs", and on 2026-10-20 he picked
  a gallery of small games. This is the first example with no map at
  all: rules, menus and panels. It is also the first whose screen must
  not show the whole state. The draw pile's order is hidden, and a test
  holds the view to that.
- **The end goal** (2026-10-19): an agent asked for a card game reads
  this example first, and an eval task grades against it.

Nothing comes from an existing card game (no names, cards, numbers or
art), and nothing from Visions of Shuyi (`AGENTS.md` rule 4).

## Nick input

None.

## Scope

**In:**
- **`examples/card-battle`**, a workspace member (the root
  `Cargo.toml` takes `examples/*`) on 2511's short path:
  - `src/lib.rs`: `pub fn game() -> BasicGame<UiAction, Tables>`, made
    with `BasicGame::with_own` (the name `Card Battle`, the storage
    name `card-battle`, the embedded `files/` layer, `Tables::load` and
    the title as its first screen), then `.console(CONSOLE)` and
    `.screens(screens::all)`;
  - `src/main.rs`: the `windows_subsystem` line and
    `fn main() { shuyi_app::run_game(card_battle::game()); }`;
  - `examples/tools.rs`, the tool program behind a `tools` feature, as
    hello-grid's `Cargo.toml` declares it;
  - imports from `shuyi::prelude`, and a `Cargo.toml` naming the
    engine as `shuyi` alone, as `examples/hello-grid/Cargo.toml` does.
- **The data**, the game's own part `Tables` (2511's `own`), read by
  `Tables::load(&Bundle)`:
  - two kinds, `Cards` (`files/cards.ron`) and `Foes`
    (`files/foes.ron`), each read through a `Ron<T>` format that the
    game's `formats()` adds to `BasicData::formats()` (rule 10);
  - `cards.ron`: the hero (health 40, energy 3 a turn, a hand of 5);
    about eight cards by id, each with a `cost` (0 to 3) and a list of
    effects: `Damage(n)`, `Block(n)`, `Draw(n)`, `Energy(n)` and
    `Weaken(turns)`; the starting deck (ten ids); and the pool rewards
    come from. A card's name and effect lines are text keys in
    `files/en.ron` (`card.<id>`, `effect.damage` with an `{n}`
    argument, and so on), read with `Ctx::text_with`;
  - `foes.ron`: two kinds of foe, each with its health and its moves,
    as a `Cycle` of moves or as `Weighted` moves with their weights. A
    move is `Attack(n)`, `Guard(n)` or `Hex(turns)`, which weakens the
    hero. Then the run, as lists of foe ids: fight 1 against two foes,
    fight 2 against one stronger foe. Ids are plain words of the
    session's choosing;
  - `Tables::load` reports, each as a `ContentError` naming the file
    and the id: a deck, pool or fight naming an id that isn't in its
    table; a pool naming a card twice; a cost above the hero's energy;
    a foe with no moves; and `Weighted` moves whose weights add up
    to 0. `BasicTools::check` gives these to `cargo shuyi check`.
- **The rules** (`src/rules.rs`, pure: no I/O, no clock, no float):
  `Run`, with `Serialize` and `Deserialize`, which implements
  `TurnRules` and `Sides` (`enum Side { Hero, Foes }`):
  - **the state:** the hero's health, block, energy and weak turns;
    the foes, each with its health, block, next move (its intent) and
    weak turns; the deck, the draw pile, the hand and the discard
    pile; the fight number and any reward offer; and three streams,
    `SimRng::stream(seed, "deck")` for shuffles,
    `SimRng::stream(seed, "foes")` for weighted moves and
    `SimRng::stream(seed, "rewards")` for offers, so one never moves
    another. The helpers that draw take `&mut impl RandomSource`, so
    unit tests script them with `ScriptedRng`;
  - **a fight starts** with the deck `shuffle`d into the draw pile on
    the "deck" stream, a hand drawn, and each foe's first intent picked
    (the first of its cycle, or `pick_weighted` on the "foes" stream).
    When a card must be drawn and the draw pile is empty, the discard
    pile is shuffled into it. If both are empty, nothing is drawn;
  - **commands:** `Play { slot: u8, target: Option<u8> }` (a card in
    the hand, and a foe for a card with a `Damage` or `Weaken` effect,
    `None` for any other), `EndTurn`, `FoesAct`, `Take(u8)` and
    `Skip`. `Command` derives `Debug`, `Clone`, `PartialEq`, `Eq`,
    `Serialize` and `Deserialize`;
  - **`legal_commands`, in this order:** on the hero's turn, each
    `Play` the hero can pay for, by slot and then by living foe (one
    with `None` for a card with no target), then `EndTurn`; during an
    offer, `Take(0)` up to the last card, then `Skip`; on the foes'
    turn, `FoesAct` alone; nothing once the run has ended;
  - **the hero's turn:** a card costs its energy and does its effects
    in order. Damage goes through the target's block first. A weakened
    attacker deals three quarters, rounded down. `EndTurn` discards the
    hand and counts the hero's weak turns down by one;
  - **the foes' turn:** `FoesAct` sets each living foe's block to 0,
    makes each one do its intent in order, picks each one's next
    intent, and counts the foes' weak turns down. Then the hero's block
    falls to 0, energy is set to the hero's energy again and a new hand
    is drawn. Weak turns are a `shuyi_core::timer::Countdown`;
  - **sides:** `Hero` moves on its turn and during an offer; `Foes`
    moves from `EndTurn` until `FoesAct`. `turn()` counts the hero's
    turns from the run's start.
    `Intents`, a `Bot`, returns `FoesAct`, as hello-grid's `Chase`
    gives its foe's commands;
  - **a won fight** (every foe at 0 health) ends at once: the piles are
    emptied. If it isn't the last fight, it offers three cards, the
    first three of the pool `shuffle`d on the "rewards" stream (all of
    them if the pool is smaller). `Take` adds one to the deck and `Skip`
    adds none, and the next fight starts with the hero's health
    carried over;
  - **outcome:** `Won` when the last fight is won, `Lost` when the
    hero's health reaches 0;
  - **events:** `Drew`, `Shuffled`, `Played`, `Hit { who, blocked, lost }`,
    `Guarded`, `Weakened`, `Intent { foe, intent }`, `Defeated`,
    `Offered`, `FightWon` and `RunEnded`;
  - cards are never lost or made: while a fight is on, the draw pile,
    the hand and the discard pile together hold the deck's cards.
- **Hidden things stay hidden** (rule 9). `FightView::of(&Run)` is the
  plain data the player may know: everything but the streams and the
  draw pile's order, of which it holds only the count. The discard pile is in it
  in full, as the genre shows it. The fight screen paints a
  `FightView`, and the tools' scene is built from one. A test builds
  two `Run`s that differ only in the draw pile's order and gets equal
  views and equal scenes from both.
- **The screens** (`src/screens.rs`), glyph skin only, all listed in
  `screens::all`:
  - a title with a `Menu`: Play, Options (the engine's
    `OptionsScreen` from `shuyi_ui::screens::options`, as hello-grid
    opens it) and Quit; the fight; the reward; and an end screen that
    says won or lost and goes back to the title on `Confirm`;
  - **the fight:** the foes across the top, each a `panel` with its name
    in the border, its health as a `meter`, its block and its intent as
    a label ("Attack 7"); the hero's health, block and energy; the hand
    along the bottom, each card a `panel` with its name, cost and
    effect lines, the focused one raised a row, a card the hero can't
    pay for drawn dim; and the two piles' counts. Every word comes from
    `files/en.ron`, as in hello-grid;
  - **choosing:** a `ListCursor` over the hand and an "End turn" entry
    after the last card, scrolled by its `window` when the hand holds
    more cards than fit. `Left` and `Right` move it, and `Confirm` plays
    the card or ends the turn (on a card the hero can't pay for, it
    does nothing). A card with a target while two foes live asks for
    one with the same actions (a second `ListCursor` over the living
    foes), and `Cancel` takes the card back;
  - the foes' turn plays back from the events, one event every 400 ms,
    timed by summing `FrameInput::dt` as hello-grid's stage times its
    walk; `Confirm` skips to the end of it;
  - `Cancel` with nothing picked asks whether to leave, on a screen that
    pops with its answer, as hello-grid's `Leave` does. Yes goes back
    to the title.
- **The actions:** `UiAction`'s six (`Up`, `Down`, `Left`, `Right`,
  `Confirm` and `Cancel`), as 2511's short path gives them.
  `files/keymap.ron` is the blank template's, plus a `targets` table
  for `playdate` and one for `steam-deck` as in hello-grid's. Nothing
  names a key (the `input-actions` skill).
- **The frame:** `ConsoleSpec::text(50, 15)` (400 × 240 at the font's
  8 × 16 cells) through `BasicGame::console`, if the fight fits it.
  `shuyi.ron` then lists `playdate` (frame `(400, 240)`, scale
  `integer`, text 16 px) beside `desktop`, `web` and `steam-deck`, and
  the palette's roles stay apart in 1-bit (3006's check). If it doesn't fit, use `ConsoleSpec::text(48, 24)` as hello-grid does,
  and the README says why the Playdate is left out, as hello-grid's
  does.
- **The font:** the screens draw only glyphs of 2511's `BASIC_GLYPHS`
  (`crates/ui/src/basic.rs`), so `files/font` is a copy
  of `templates/blank/crates/game/files/font`: Hello, Grid's atlas and
  the Terminus license (OFL-1.1). The templates' Terminus row in
  `THIRD_PARTY_ASSETS.md` gains its path.
- **Sounds:** if 2225 is done, the engine's four menu cues come from
  `files/sfx/*.sfx.ron` and `files/audio.ron`, which the short path
  reads with no code, as 2225 gives the blank template. If not, the
  example is silent, as the short path is (2511's Out), its README
  says so, and 2225's PR adds the sounds (its item on the gallery
  examples says so).
- **`shuyi.ron`** in hello-grid's form: `game`, `targets`, `web`, and a
  `check_keys` section as the blank template's (`src`, `files`, with
  `files/keymap.ron` as `not_text`), so `cargo shuyi check-keys` runs in
  the example's folder.
- **The tools** (`src/tools.rs`): a `GameTools` of its own that calls
  `BasicTools`' `frame`, `lang`, `check` and `controls` (2511), and
  adds:
  - `session`: `Headless` over the harness `with_scene`, the scene built
    from the top screen's `FightView` (data: `fight`, `health`, `block`,
    `energy`, `hand` as card ids, `draw_pile` as a count, `discard`,
    `foes` and `outcome`);
  - `replay`: a `RunRecord { seed, commands }` read with `ron`, re-run
    through `shuyi_cli::contract::replay::turns`, as hello-grid's
    `StageRecord`;
  - `bots`: a `RandomBot` hero against `Intents`, summed by `tally`, as
    hello-grid's `bots`.
- **Search bots, either order with 2113:** if it is done, `Run`
  implements `Determinize`. Whatever the side, the copy keeps
  everything but the draw pile's order and the streams: the draw pile
  is sorted by card id, then `shuffle`d on a new
  `SimRng::stream(seed, "deck")`, and the "foes" and "rewards" streams
  become `SimRng::stream(seed, "foes")` and
  `SimRng::stream(seed, "rewards")`. Sorting first makes the copy
  independent of the hidden order. The score (`Evaluate`) is the
  hero's health minus the foes', and a finished run scores 1000 if won
  and -1000 if lost. Two tests:
  - seeds 0 to 19 are played by an `Mcts` hero and by a `RandomBot`
    hero, each against `Intents`. The `Mcts` hero wins at least as many
    runs. The Completion notes give both win counts and the
    `MctsSettings` used;
  - two states that differ only in the draw pile's order get the same
    first command from the same `Mcts`.

  If 2113 isn't done, nothing here: 2113's clause with 2737 adds all
  of this when it lands.
- **Golden table, from 2116 (done):** the example gets a golden table
  as Hello, Grid's (`tests/golden/card-battle.ron`, a record
  `tests/records/seed-11.ron` from `cargo shuyi bots 1 --seed 11 --json`,
  checked by `tests/golden.rs`), its window behind a default `window`
  feature, and its own `cargo test --target wasm32-wasip1 -p card-battle
  --no-default-features --test golden` step in the `determinism` job,
  as 2116 says a new example does.
- **Soak, either order with 2527:** if it is done, the tools implement
  `soak` through `shuyi_cli::contract::soak::turns` with `RonCodec` and
  `Run::invariants` (the deck whole while a fight is on, energy and
  health in range), and `cargo shuyi soak 20` passes in the example's
  folder. If not, nothing.
- **CI and Pages,** either order with the other examples (2735, 2736,
  2738, 2739 and 2740):
  - if the `wasm` job of `.github/workflows/ci.yml` and
    `.github/workflows/pages.yml` still name each example by hand
    (today `examples/hello-grid` alone), change both to a loop over
    every `examples/*/shuyi.ron` that has a `web` section:
    `cargo shuyi check --target all` and `cargo shuyi web --release` in
    each folder, an uploaded artifact per example, and in `pages.yml`
    each `dist/web` copied to a folder named after the example, with a
    line on the index page from its `web.title`. The `run-gates` skill's
    web-build step loops the same way;
  - if an earlier example made the loop, this one needs no workflow
    change;
  - the example gets its card in the Examples section,
    `site/content/examples/<name>.md` (2801, done).
    2019's `web-render` job (done) runs `check.py boot` on this build
    too: it loops over the examples as `pages.yml` does.
- **A README** in hello-grid's form, with the lines of its own code
  without tests and data (`cargo xtask template-lines` if 2512 is done;
  counted, and said how, if not). The engine's `README.md` names the
  example beside hello-grid.
- **The eval, either order with 2524:** if it is done, write
  `evals/tasks/card-battle.ron`: template `blank`, name `Card Battle`,
  targets `desktop` and `web`, reference
  `Project("examples/card-battle")`, and the checks `Gates`, `Check`,
  `Targets`, `Web`, `Responds { steps: [Act("Down")] }` (the title's
  menu moves), `Replay`, and
  a `Scene` check on data its prompt names (after `Act("Confirm")` from
  the title: `/fight` is `1` and `/energy` is `3`). Its prompt asks for
  this game in the words of 2524's tic-tac-toe prompt, without the
  example's cards or numbers beyond those the checks read.
  `cargo xtask eval-reference card-battle` passes. If 2524 isn't done,
  add a line to its *first tasks* in this PR naming the card-battle
  task and this example as its reference.

**Out (do not do):**
- a map between fights, a longer run, shops, rests, healing, relics,
  upgrades, card art, and animation beyond a beat per event: follow-ups
  if a game asks;
- statuses beyond weakness, and cards that make, copy or remove cards;
- a search bot as the foes' mind: the foes follow the intents they
  show, as the genre does;
- the search bots (2113), the RNG helpers (2114), timers (2115), golden
  tables (2116), soak (2527) and the eval (2524) themselves: this
  ticket only uses them;
- saves: a run ends when the game closes;
- the mouse: 2405 serves menus, and nothing here needs more;
- sounds beyond the engine's menu cues (2225);
- a `kind` answer in `docs/modules.ron` (2003), a guide (2004) or a
  template part (2501): the example proves the default engine, not a
  module (ADR-0009 §7);
- the other examples: the match-3 (2736), the roguelike (2735), the
  tick-path examples (2738, 2739) and the tutorial's box-pushing puzzle
  (2803);
- anything from an existing card game or from Visions of Shuyi;
- new engine features. A gap the example shows becomes a ticket (the
  `write-ticket` skill), named in the Completion notes.

## Implementation steps

This is large for one PR. Work the steps in order. If the diff passes
about 600 changed lines (fixtures and snapshots aside) at the end of
step 6, stop there: write a follow-up ticket for steps 7 to 9, parked
with this one, carrying their scope, criteria and tests, and say so in
the PR. The run then has one fight.

1. Read `examples/hello-grid` (its README, `src/lib.rs`,
   `src/rules.rs`, `src/screens.rs`, `src/screens/stage.rs`,
   `src/tools.rs` and `tests/it/`), `crates/ui/src/widgets/list.rs`
   and `menu.rs`, `crates/ui/src/frame/draw.rs`, 2511's
   `BasicGame`, `BasicData` and `BasicTools`, and 2114's helpers.
   Check which either-order tickets are done.
2. Make the crate on the short path: `Cargo.toml`, `src/lib.rs`,
   `src/main.rs`, `examples/tools.rs` and `files/`. Write `Cards`,
   `Foes`, `Tables::load` and their checks, with unit tests.
3. Write the rules of one fight and `Intents`, with the unit and
   property tests.
4. Write `FightView`, the title, fight, end and leave screens, the
   keymap, the text and `shuyi.ron`. Write the hidden-pile test, the
   Harness playthrough of fight 1 and the pictures of fight 1.
5. Write the tools, the 50-seed test, the replay test and the `check`
   tests.
6. Make the CI and Pages loop, or check that the example is picked up.
   Write the README and the engine `README.md` line. Run the gates.
7. Add the offer, the reward screen and fight 2: the rules, their unit
   tests, the playthrough carried on to fight 2, and the reward
   picture.
8. Add the golden table (2116), and the parts of 2113 and 2527 that
   are done.
9. Add the eval task, or the line in 2524. Run the gates.

## Acceptance criteria

- [ ] `examples/card-battle` is on 2511's short path: `src/main.rs`
      calls `shuyi_app::run_game`, and the tools call `BasicTools`.
- [ ] The Harness playthrough of seed 1 (`Harness::act` only), from the
      title, wins fight 1 by playing every card it can pay for on the
      first living foe and then ending the turn. With step 7, it takes
      a reward and starts fight 2. The starting deck and fight 1 are
      tuned so this plan wins seed 1.
- [ ] A `RandomBot` hero against `Intents` plays seeds 0 to 49 with no
      rules error, each run ending within 400 turns.
- [ ] A run's commands replay to the same events and `state_hash`
      twice, and `cargo shuyi replay` passes on a record from
      `cargo shuyi bots 1 --json`.
- [ ] While a fight is on, at every step of random legal play, the draw
      pile, the hand and the discard pile together hold the deck
      (property test).
- [ ] Two runs that differ only in the draw pile's order give equal
      `FightView`s and equal scenes.
- [ ] `cargo shuyi check` fails on a copy of `cards.ron` whose deck
      names an unknown card, and passes on the example's files, as do
      `cargo shuyi check --target all` and `cargo shuyi check-keys` in
      the example's folder.
- [ ] The `frame-png` pictures are checked in as snapshots.
- [ ] CI builds the example for the web, and Pages publishes it.
- [ ] The README states the example's own lines, and the engine's
      `README.md` names the example.
- [ ] If 2113 is done, `Run` implements `Determinize`, both search tests
      pass, and the win counts are in the Completion notes.
- [ ] The golden table above is in, and if 2527 is done, its part
      above is in.
- [ ] `evals/tasks/card-battle.ron` grades this example with
      `cargo xtask eval-reference card-battle`, or 2524's scope names
      the task.
- [ ] If the PR stopped after step 6, a parked follow-up ticket holds
      steps 7 to 9, and the PR says so.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`src/rules/tests.rs`): each effect; damage through block; three
  quarters when weak, rounded down; energy and its refill; a draw that
  shuffles the discard pile in (a `ScriptedRng`); a draw with both
  piles empty; a cycle intent and a weighted one (a scripted draw); a
  foe's block cleared when the foes act; a defeated foe skipped and not
  a target; each refused command (too little energy, a dead or missing
  target, a slot past the hand, any command after the end); the
  `legal_commands` order; an offer with no card twice; a broken
  `cards.ron` and `foes.ron`, one per check.
- Property (proptest): the deck stays whole while a fight is on;
  energy is never below 0; health never above its maximum.
- Integration (`tests/it/`): the playthrough; 50 seeds; replay; the
  hidden pile; `check`, `check --target all` and `check-keys`; the
  search tests if 2113 is done; soak if 2527 is.
- Golden: the record's hashes, natively and on wasm32 (2116).
- Snapshot: `frame-png` pictures of fight 1's first turn, a target
  being chosen in fight 1 and the reward, as insta binary snapshots as
  in `examples/hello-grid/tests/it/snapshot.rs`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
