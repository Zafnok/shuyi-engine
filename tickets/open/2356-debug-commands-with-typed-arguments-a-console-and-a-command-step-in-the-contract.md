---
id: "2356"
title: "Debug commands: typed arguments that screens declare, a console to type them, a command step in the contract and MCP, and changes recorded through the game's own commands"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2316"]
nick_input: none
completed:
---

# 2356 — Debug commands, a console, and a command step

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6,
Unreal 5.8 and GameMaker.

- **Unreal** has console commands and console variables built in.
  Commands flagged `ECVF_Cheat` and the `UCheatManager` class are left
  out of Shipping builds.
- **GameMaker** has its `dbg_*` functions: a debug overlay with
  buttons, sliders and text a game declares.
- **Bevy** has the community crate `bevy-console`
  (<https://crates.io/crates/bevy-console>). **Godot** and **Unity**
  users add community consoles too.

So Unreal has it built in, and users of the other engines add it.

What Shuyi has today:

- `crates/ui/src/debug.rs`: `DebugEntry<G>` is a label and a closure
  `FnMut(&mut Ctx<G>) -> Transition<G>`, listed by `DebugMenuScreen`.
  The game opens that menu through `GameHooks::debug`
  (`crates/ui/src/game.rs`), only with debug tools on
  (`Ctx::debug_tools`, on in debug builds and with the `debug-tools`
  feature: `shuyi_ui::DEBUG_TOOLS`). An entry takes no arguments, is
  picked from the menu only, and nothing outside the game reaches it.
  Hello, Grid and the templates wire no debug menu
  (`GameHooks::default()`).
- `GameTools::frame_options` (`crates/cli/src/game.rs`) lets a game
  declare start options, such as Hello, Grid's `--stage` and `--seed N`
  (`examples/hello-grid/src/tools.rs`). They reach `frame-png`, `run`,
  `scene` and `bots` (ADR-0022 §2), but only at the start.
- `frame_png::Step` (`crates/cli/src/frame_png.rs`) is `Keys`, `Pad`
  or `Wait`. A script changes state partway through a run only by input.
- `TurnRules::legal_commands` (`crates/core/src/turn.rs`) lists the
  commands `check` accepts. Its contract lets a game leave some out
  only "when there are too many to list".
- 2315 and 2316 (the debug view) are read-only by design. Editing state
  belongs to the parked editor (2506, 2507).
- 1133's MCP session has `start`, `step`, `scene`, `frame_png`,
  `events`, `reset` and `stop`. It has no tool a game declares.

What is missing: named debug commands that take arguments, a way to
type them in the running game, and a script step that runs one partway
through a run.

**Why it matters.** Reaching a bug's state takes minutes of play: a
piece next to a wall on turn 9, a meter at 1. A command that a screen
declares (`place foe 3 4`) does it in one line, for a person at the
console or an agent through `run --command` and MCP. Visions of Shuyi,
the engine's first user, gets it first. Every other 2D game gets it
too. It also serves the end goal of 2026-10-19 (`docs/decisions.md`):
an agent reaches the state it needs to test without long key scripts.

**The rule that keeps replays true** (ADR-0002 §2–§3): a command that
changes a game's state does it by sending the game's own `TurnRules`
command, which the screen keeps like any other. So the game's record
holds it, and `shuyi replay` reproduces it with the same hashes.

**What stays the game's** (`AGENTS.md` rule 4): which commands exist.
The engine ships the mechanism: declaring, parsing, routing, the
console, the script step and the log. A game that wants a cheat adds a
debug-only variant to its own command enum. That variant is never in
`legal_commands`, so bots, search (2112, 2113) and soak (2527) never
send it. Hello, Grid shows the pattern with one command.

The engine cannot see a game's command type through `Screen<G>`, so it
cannot enforce the rule through types. The ADR states it, and the
Hello, Grid replay test below proves it for the pattern every game
copies.

Nick placed the debug view after the MVP, not parked, and never before
the game's Chapter 1 work, on 2026-10-19 (`docs/decisions.md`). This
ticket joins that group, after 2316. On 2026-10-20 he put the features
most 2D games expect after the MVP too.

## Nick input

None.

## Scope

**In:**
- **An ADR** (the `write-adr` skill, the next free number, listed in
  `docs/adr/README.md`): debug commands. Screens declare them, and
  `Game` runs them between frames. A state change goes only through
  the game's own commands. The built-ins are `help`, `screens` and
  `quit`. The ADR covers the script step, and that commands run only
  with debug tools on. It links 2315's debug view ADR. It updates
  ADR-0022 §3, whose script is "keys, pad buttons, waits", and the
  `TurnRules` contract line below.
- **`shuyi_ui::debug::command`** (`crates/ui/src/debug/command.rs`),
  the data. `shuyi-ui` builds without `std` (3005): use
  `crate::prelude` and `alloc` types, as `debug.rs` does.
  - `ArgKind`: `Int { min: i32, max: i32 }`,
    `OneOf(&'static [&'static str])`, or `Word` (one word, no spaces).
  - `ArgDef { name: &'static str, kind: ArgKind }`.
  - `CommandDef { id: &'static str, help: &'static str, args:
    Vec<ArgDef> }`. The id is `snake_case` ASCII.
  - `DebugArg`: `Int(i32)` or `Word(String)`.
  - `DebugCall { id: String, args: Vec<DebugArg> }`, with
    `int(i) -> Option<i32>` and `word(i) -> Option<&str>` getters.
  - `CommandDef::parse(&self, words: &[&str]) -> Result<DebugCall,
    String>`. It checks the count and each argument, and errors read
    like `place: y is 0 to 9, not 12`.
  - `CommandDef::usage()`, for example
    `place <side: hero|foe> <x: 0-11> <y: 0-9>`.
- **Two defaulted methods on `Screen`** (`crates/ui/src/screen.rs`).
  They break no game.
  - `fn debug_commands(&self) -> Vec<CommandDef>`, which returns none
    by default. The list is built from `&self`, so bounds can come from
    the state (the map's size).
  - `fn debug_command(&mut self, ctx: &mut Ctx<G>, call: &DebugCall)
    -> Result<String, String>`. It answers the reply text or why it
    refused. The default refuses. `Game` never calls it for an id the
    screen didn't declare.
- **`Game::command(&mut self, line: &str) -> Result<String, String>`**
  (`crates/ui/src/game.rs`). It runs between frames, like `Game::act`.
  - With debug tools off, it answers
    `Err("debug tools are off in this build")` and changes nothing.
    After a quit, it answers `Err("the game has quit")`.
  - The first word is the id. The built-ins come first:
    - `help` lists every command now available, one usage line and
      help each;
    - `screens` lists the stack's names, bottom first;
    - `quit` asks the game to quit, as `Transition::Quit` does.
  - Otherwise the call goes to the game's screens, from the top down
    (add a `pub(crate)` walk of `ScreenStack`'s screens from the top:
    its `screens` field is private). Screens named in
    `GameHooks::debug_screens` are skipped. The first screen that
    declares the id parses the words against its `CommandDef` and runs
    it. A bottom screen can therefore declare commands for the whole
    game.
  - A command opens and closes no screen. Debug menu entries do that.
  - The game redraws after the command. Sounds the command asks for
    come out with the next frame.
  - An unknown id answers ``unknown command `x`: `help` lists them``.
  - `Game::debug_commands(&self) -> Vec<CommandDef>` gives the
    built-ins and then the game's screens' commands, top first, without
    duplicates. `help` and the tests use it.
- **`TurnRules`' contract** (`crates/core/src/turn.rs`): a game may also
  leave debug-only commands out of `legal_commands`, and says which.
- **The Harness:** `Harness::command(&mut self, line: &str) -> &mut
  Self` (`crates/platform/src/harness.rs`) works like `Harness::act`:
  the command, then one frame of `FRAME_DT`.
  `Harness::command_log()` gives each command with the frame it ran
  before, its line and its result.
- **The contract** (`crates/cli`):
  - `frame_png::Step::Command(String)` (JSON `{"command": "place foe 3
    4"}`). `Step::check` refuses an empty line, a line over 120
    characters, and anything but printable ASCII. `Script::play` runs
    it with `Harness::command`.
  - `--command "…"` is a step in `frame-png`, `run` and `scene`. Add
    it to `frame_png::parse_args`, `Script::parse_step`, `run::parse_args`
    and both usage texts (`frame_png::usage`, `run::usage`).
  - `session::Event::Command { frame, line, ok, reply }`. `Headless`
    builds it from the Harness's command log, and `run`'s text
    (`event_text`) prints it.
  - `run` and `scene` still pass when they ran. Each failed command adds
    a limit: ``command `x` on frame 3 failed: …``. A `--command` that
    `Step::check` refuses is a usage error, like a bad `--keys`.
  - `contract/fake.rs`: `FakeSession` counts one frame for a
    `Command` step, as `Harness::command` runs.
  - Regenerate the schemas, raise `contract::VERSION` by one, and add
    its line to `crates/cli/schemas/versions.json` (`docs/contract.md`,
    *The schemas*).
  - `docs/contract.md`: the step in *The operations*, the event, and a
    short *Debug commands* section with the rule and Hello, Grid's
    example.
- **Hello, Grid, the worked example** (`examples/hello-grid`):
  - `rules::Command::Place(Side, Pos)` is debug only. It puts that
    side's piece on a tile at once. Whose move it is and the turn stay
    the same. `check` refuses it with `Error::Over` once the stage is
    over, and with `Error::OutOfReach(pos)` for a tile off the map, a
    wall, the goal or the other piece's tile. It adds
    `Event::Placed { side, from, to }`. It is never in
    `legal_commands`, and that method's doc says so.
  - `StageScreen` (`src/screens/stage.rs`) declares
    `place <side: hero|foe> <x> <y>`, with bounds from
    `self.stage.map()`'s width and height (0-11 and 0-9 today). Its
    `debug_command`:
    - refuses while a piece is shown walking (`walk` is set);
    - asks `TurnRules::check` first, since `send` drops the error, and
      replies with the rules' error (`(0, 0) is out of reach`);
    - otherwise runs it through `send`, so `commands` (and so a
      `StageRecord`) holds it, clears `selected` (a picked-up piece),
      and replies `placed foe at 3,4`.
- **MCP, in either order with 1133:**
  - if 1133 is done, add a `command` tool to its session (input:
    `line`; answer: the reply, and whether it ran) and let `step` take a
    script's `command` steps;
  - if 1133 isn't done, 1133 adds them (edit requested there).
- **The skills, in either order with 1134:**
  - if 1134's `shuyi-debug-replay` and `shuyi-test` skills exist, add a
    short section: reach a bug's state with a command the game declares
    (`--command`) rather than a long key script, and keep the command a
    debug-only variant of the game's own enum, never in
    `legal_commands`;
  - if they don't exist yet, 1134 covers it (edit requested there).
- **The log (3040, done):** `Game::command` also logs each line and its
  result with `Ctx::log` (level `Info`, category `Game`: the commands
  are the game's).
- **Session records, in either order with 2518:** if 2518 is done, a
  command run between frames goes through `App::command(&mut self,
  line: &str) -> Result<String, String>` (new), which runs
  `Game::command` and notes the line on the next `FrameRecord`
  (`commands: Vec<String>`); playback runs it before that frame, as
  2518 says. Only calls from outside a frame are recorded:
  a console line (below) is typed input, which the record holds
  already. If 2518 isn't done, 2518 does it (its own clause says so).
- **The live link, in either order with 2521:** if 2521 is done, add
  `Request::Command { line }` to its link, which runs `Game::command`
  and answers its reply. If not, 2521 adds it (its own clause says so).
- **Types, in either order with 2907:** if 2907's ADR is done and
  names a description that `ArgKind` should use, follow it. If not,
  2907's ADR covers `ArgKind` among its consumers.
- **For the game:** additive. `Screen` gains defaulted methods, and
  with the console `Ctx` gains a field (it has private fields, so no
  game builds it field by field). A game that matches `frame_png::Step` or
  `session::Event` exhaustively must add an arm. **Either order with
  2013:** if it is done, the new public types follow its rules; if not,
  2013 covers them. If 2010 is done and its check calls the new
  variants a break, the PR carries its `Breaking:` line. The game's PR
  that moves the pin follows this one.
- **The console** (the last step; step 9 says when it moves to a
  follow-up ticket):
  - **The console's log in the context:** `Ctx::debug_console:
    ConsoleLog`. It holds the last 64 lines: each command as `> line`,
    then its reply lines. Every `Game::command` call writes to it,
    whatever sent the command, so the console shows commands sent from
    the contract too. It also holds a pending line that the console
    screen sets. After the stack's update, `Game::step` takes that line
    and runs it through the same routine as `Game::command`.
  - **The screen:** `shuyi_ui::debug::console::ConsoleScreen`, named
    `debug_console`, an overlay over the bottom rows of the frame.
    - Its input line takes the typed characters (`FrameInput::text`,
      printable ASCII only, at most 120). The text-box keys from
      `input::fixed::text_key` work as in `NameBox::handle`: Done runs
      the line, Delete deletes the last character, Cancel closes the
      console. Actions are ignored while it types, as `NameBox::handle`
      ignores them, so a letter bound to an action types the letter.
    - It shows the log's last lines that fit, then `> ` and the input.
    - Without a keyboard (`Ctx::keyboard`, which 3053 moves to
      `ctx.platform.keyboard`: use whichever exists), it shows one line
      saying it needs a keyboard and that the contract's `--command`
      reaches the same commands. Its Cancel action closes it.
    - It declares `UiAction::Cancel` only.
    - `DebugEntry::console()` is the debug menu entry that opens it.
    - Its name counts as a debug screen whether
      `GameHooks::debug_screens` lists it or not: the Debug action does
      nothing over it, 2316's pause keeps it updating, and commands are
      not routed to it.
    - The look is a skin (rule 9): a `ConsoleView` plain-data struct,
      painted by `debug::glyph::paint_console`.
  - **While paused (2316):** the console still takes typing and runs
    commands. 2316 keeps the debug screens updating while paused (edit
    requested there). If the 2316 that landed does not, make them
    update here.

**Out (do not do):**
- Variables (get and set over settings fields). The options screen and
  settings files cover settings, and live edits are the editor's
  (2506, 2507, parked).
- A typed listing of commands in `run`'s answer, and one MCP tool per
  command with its own schema. `help` answers as text. Write a ticket
  when an agent needs more.
- Commands for tick games. 1119 (parked) adds them as inputs of the
  next step (its clause says so). If 1119 is done first, add them here
  as its clause describes.
- Rewind (2357). It seeks over session records, so it gets commands
  through 2518 and needs nothing here.
- A pad path for the console (a list of commands without arguments,
  chosen with Confirm) on pad-only targets, and the system's own
  keyboard (`Ctx::system_keyboard`, done 3003) for the console on a
  Steam Deck. Write a ticket when a debug build there needs it.
- The log's recent lines (3040) in the console. It shows commands and
  replies only.
- Keeping the game running under the console (2367, parked).
- A command in the grid template. 2511 reshapes the templates, and 2512
  budgets their lines. Hello, Grid shows the pattern.
- Several commands on one line, tab completion, recalling past lines,
  scripting (2101), and writing arbitrary state.

## Implementation steps

1. Read `crates/ui/src/debug.rs`, `debug/glyph.rs`, `screen.rs`,
   `game.rs`, `ctx.rs`, `widgets/name_entry.rs` (`NameBox`, the typing
   pattern) and `input/fixed.rs`. Then read
   `crates/platform/src/harness.rs` (`act`), `crates/core/src/turn.rs`,
   and in `crates/cli/src/` read `frame_png.rs`, `contract/run.rs`,
   `contract/session.rs` and `contract/fake.rs`. Then read Hello,
   Grid's `rules.rs`, `screens/stage.rs`, `tools.rs` and
   `tests/it/determinism.rs`, 2315's and 2316's ADR, ADR-0002 §2–§3,
   ADR-0022 §3, and the `input-actions` skill. Check which of 1133,
   1134, 2010, 2013, 2518, 2521 and 2907 are done.
2. Write the ADR.
3. Add `debug/command.rs` with its unit tests.
4. Add the two `Screen` methods, the stack's walk from the top,
   `Game::command` and `Game::debug_commands`, with tests on test
   screens in `crates/ui/src/game/tests.rs`.
5. Add `Harness::command` and its log.
6. Add `Step::Command`, the flag, the event, the limit and the fake's
   frame. Regenerate the schemas, raise `VERSION`, add the
   `versions.json` line, and update `docs/contract.md`.
7. Change the `TurnRules` contract line. Add Hello, Grid's `Place` and
   the stage's `place` command, with the tests below.
8. Do the either-order items whose other ticket is done (MCP, skills,
   log, session records, live link, types).
9. Count the diff, fixtures, snapshots and the regenerated schemas
   aside.
   - Under about 400 changed lines: add the console (`Ctx::debug_console`,
     `debug/console.rs`, `DebugEntry::console()`, `glyph::paint_console`,
     the pending line in `Game::step`, the paused case). Test it on the
     toy (`crates/ui/src/toy.rs`): a `Game` over a toy screen that
     declares one command, with a debug hook that opens a
     `DebugMenuScreen` holding `DebugEntry::console()` (the toy's own
     `hooks()` open its gallery instead).
   - Otherwise write the console's follow-up ticket (the `write-ticket`
     skill, the next free 23xx number, After MVP, `blocked_by:
     ["2356"]`) with the console's part of this Scope and its tests, and
     land the rest.
10. Run the gates.

## Acceptance criteria

- [ ] An ADR records the design and the rule, and
      `docs/adr/README.md` lists it.
- [ ] Harness, Hello, Grid: from `--stage --seed 0`, `place foe 3 4`
      followed by a key script leaves a stage whose `StageRecord`
      replays through `HelloTools::replay` with the last hash equal to
      `state_hash` of the stage on screen.
- [ ] `shuyi run --stage --command "place foe 3 4" --json` on Hello,
      Grid answers a `command` event with `ok: true` and a scene with
      the foe on 3,4. A bad command (`place foe 99 4`) adds a limit and
      changes nothing.
- [ ] With debug tools off (`Game::with_debug_screens(false)`),
      `Game::command` refuses, and the state hash is unchanged.
- [ ] `Place` is never among `legal_commands`, and the `bots` tests in
      `examples/hello-grid/tests/it/bots.rs` and `contract.rs` pass
      unchanged.
- [ ] Either the console is in: typing a line on the toy and pressing
      the text-box Done key runs it and shows the reply, also on a
      paused game, and its insta snapshots (with and without a
      keyboard) are committed. Or the console's follow-up ticket is in
      `tickets/open/`, After MVP, with `blocked_by: ["2356"]`.
- [ ] No key is named in code beyond the text-box keys of
      `input::fixed` (`check-keys` passes). Debug text carries the
      `check-text` English-only comment.
- [ ] The schemas, `VERSION` and `versions.json` agree (the schema test
      passes).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit:
  - `CommandDef::parse`: each `ArgKind` in and out of bounds, too few
    and too many words, and the error texts;
  - `usage()`;
  - `Game::command`: routing past debug screens and down the stack, a
    built-in winning over a screen's command of the same id, `help`,
    `screens` and `quit`, an unknown id, after a quit, and with debug
    tools off;
  - `Step::check`, `Script::parse_step` and `run::parse_args` for
    `--command`;
  - Hello, Grid's `Place`: refused on a wall (`place foe 0 0`), off the
    map, on the goal, on the other piece and after the stage is over;
    whose move it is and the turn unchanged after it;
  - with the console: the log keeping 64 lines.
- Seeded sweep (as `tests/it/determinism.rs` does, no new dependency):
  for seeds 0 to 15, a Harness from `--stage --seed N` plays a key
  script with `place` commands mixed in at tiles drawn from a `SimRng`
  of seed N. The screen's `StageRecord` replays to the same final hash
  as the stage on screen.
- Snapshot / integration:
  - the contract `run --command` and `scene --command` tests in
    `examples/hello-grid/tests/it/contract.rs`;
  - the regenerated schemas;
  - with the console: insta snapshots in `crates/ui/tests/it/` (a new
    `debug_console.rs`, listed in `main.rs`) of the toy's console with
    and without a keyboard, and a Harness test of it on a paused game.

## Completion notes

*(Filled in by the session that completes the ticket.)*
