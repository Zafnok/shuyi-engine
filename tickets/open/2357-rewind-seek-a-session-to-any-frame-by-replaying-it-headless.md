---
id: "2357"
title: "Rewind: seek a session to any frame by replaying it headless, in the contract first and the debug view second"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["1133", "2316", "2511", "2518", "2519", "2575", "3042"]
nick_input: none
completed:
---

# 2357 — Rewind: seek a session to any frame by replaying it headless

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Nick put the features most 2D games expect after the
MVP, not parked ([`docs/decisions.md`](../../docs/decisions.md),
2026-10-20, "Split by group"). This one comes late in that group,
because it builds on the debug view (2315, 2316) and the session record
(2518, 2519).

What the benchmarks have:

- Unreal 5.8's Rewind Debugger records a play session (animation,
  transforms, traces) and scrubs the recording on a timeline in the
  editor
  (dev.epicgames.com/documentation/en-us/unreal-engine/rewind-debugger-in-unreal-engine).
  It shows the past. It does not put the game back in that state to
  play on from there.
- Godot 4.7's Game view and Unity 6's editor pause a running game and
  advance it one frame at a time. Neither goes back.
- Bevy 0.20 has nothing built in. The community crate `bevy_save` takes
  snapshots of the world and rolls back to them (crates.io/crates/bevy_save).

So "go back and play on" is less a gap next to them than an edge the
engine's determinism makes cheap: going back is replaying the session
from its start, headless, to the frame wanted.

What Shuyi has today (checked 2026-10-21):

- **A start that runs once.** A game starts from `Start<G>`
  (`crates/platform/src/app.rs`). Its `load` is a `Load<G>`, a
  `Box<dyn FnOnce(Setup) -> Result<Game<G>, String>>` (line 155). `App`
  takes it once (line 438) and calls it in `App::load` (line 475), which
  gives the game `seed: mix(now.0)`, a seed from the clock (line 479).
  `App` keeps the start's look and system hook, but its card only until
  the load. So a running game can't build itself again from its start.
  `Start::before_load` (line 206) wraps the load with a `FnOnce`, and
  `shuyi-app`'s `run` uses it to hand in the voice manifest it read
  (`crates/app/src/lib.rs:160`). The starts that games write are
  closures that could run twice:
  - hello-grid's and the grid template's, today in their binaries
    (`examples/hello-grid/src/main.rs:62`,
    `templates/grid/crates/game/src/main.rs:84`). 2519 moves them into
    their `lib.rs` (`hello_grid::start()`), so the tools can build them;
  - `BasicGame::start` (`crates/platform/src/basic.rs`, 2511), which the
    blank template's start now is.
- **Going back in the rules only.** `shuyi_core::history`
  (`crates/core/src/history.rs`) has `History::state_at` (line 114) and
  `History::rewind_to` (line 146). They go back in a turn game's rules,
  by command. They don't rebuild the screens, menus and UI state around
  the rules.
- **The contract plays forward.** `run` and `scene` play a `Script` of
  keys, pad buttons and waits in a fresh session (`played`,
  `crates/cli/src/contract/run.rs:84`), which `GameTools::session`
  (`crates/cli/src/game.rs:110`) starts anew each time. `replay`
  (`contract/replay.rs`) re-runs a turn record and names the first step
  whose hash differs; it doesn't play on from a step. One Harness wait
  is at most `MAX_FRAMES` (2 000) frames
  (`crates/platform/src/harness.rs:45`).
- **MCP can only start over.** 1133 (MVP) gives the session tools
  `start`, `step`, `scene`, `frame_png`, `events`, `reset` and `stop`.
  An agent goes back by `reset` and stepping again by hand.
- **The plans leave it to this ticket.** 2316 pauses and steps
  forward, and its Out leaves stepping backward and rewinding here. The
  editor research (2506, parked) sends pause and step as 2316's actions
  and plans going back on this ticket's `--until`, `seek` and
  `DebugBack` (its clause says so).

What is planned under it:

- 2518 records a whole session in `App` (`App::record`; its start:
  display, seed, storage; then each frame's time, input, system events,
  text answers and music position) and plays it back through
  `session::Playback` (no `harness` feature), with a hash per frame; 2575
  gives the Harness the same (`Harness::from_session`,
  `Harness::play_to`). It
  records only drivers of `App::frame`: `frame_after` stops a record and
  sets `truncated`, and so does its cap of `MAX_SESSION_FRAMES`.
- 2519 makes the windowed and web builds record their sessions in debug
  builds, and plays records in `shuyi run`, `scene` and MCP's `start`
  (`--session <file>`, `--any-stamp`, `GameTools::session_from`,
  `Session::play_record`). It leaves `frame-png` without `--session`.
  Its ring keeps whole session files, so a record always holds its
  first frame.
- 2905 (research) plans to rebuild the game and replay "the record so
  far". The replay and hand-over in this ticket is that mechanism, built
  once.

Why it matters:

- **The game first.** By the time 2316's pause stops the game, the frame
  where a bug started has usually passed: a menu that took the wrong
  row, a dialogue line skipped, a cursor that jumped. A developer of
  Visions of Shuyi, or an agent working on it, goes back ten seconds and
  watches it happen again with the debug view's channels on. The turn
  history already goes back in the rules. This goes back in everything
  around them.
- **Other 2D games.** In a tick game (once 2137 is done), the jump
  that went wrong is one second back.
- **The end goal** (2026-10-19): an agent bisects a session with
  `run --session <file> --until N` for a few N, finds the first frame
  where the scene is wrong, and plays on from there with other input to
  try a fix.

Rules it keeps (`AGENTS.md`):

- **Determinism** (rule 5). Going back is replaying the record from its
  start, never a snapshot of `App`'s state. The replayed frame's hash is
  checked against the record's before anything changes.
- **Only backends touch devices** (rule 5). The replay runs on the
  `HeadlessPlatform`, in silence, with its storage in memory.
- **Actions, not keys** (rule 8, the `input-actions` skill).
  `UiAction::DebugBack` is mapped through the keymap, like `Debug` and
  2316's `DebugPause` and `DebugStep`.
- **Debug tools only.** Nothing here acts unless `Ctx::debug_tools` is
  on.

## Nick input

None.

## Scope

A frame number here counts `App::frame` calls from 1, as 2518's
`Played::frames` does.

**In:**

**Part 1, the contract (do it first).**

- **`until` wherever 2519 plays a session record:** `run` and `scene`,
  MCP's `start`, and `frame-png` (below). The input gains
  `until: Option<u32>` (`--until <frame>` on the command line):
  - the record plays up to and including that frame through 2518's
    `Harness::play_to`, frame by frame (so `MAX_FRAMES` doesn't bound
    it; the record's length does). 2519's `Session` gains
    `play_record_to(&mut self, frame: u32) -> Option<Played>`, defaulted
    to `None` like `play_record`; `Headless` calls `Harness::play_to`;
  - each frame's hash is checked against the record's, as 2519's
    playback does for a whole record, and the first difference fails
    the answer and names that frame;
  - then the input's `script` and `frames` play on from there: a branch,
    headless;
  - `until` without `--session` is an error. `until` of 0, or past the
    record's last frame, is an error that gives the record's length
    (read from the parsed file before anything plays).
- **`frame-png --session <file> [--any-stamp] --until <frame>`** writes
  that frame, then plays any `--keys`, `--pad` and `--wait` steps after
  it. 2519 leaves `frame-png` without `--session`, so add the flags
  here, parsed the way 2519 did it for `run`. Paint the frame the way
  1133's MCP `frame_png` paints a session's frame (read it in step 1).
  If that path can't start from a record, add a defaulted
  `GameTools::frame_from_session(&self, record: &SessionRecord, stamps:
  Stamps, until: Option<u32>, script: &Script, scale: Option<u32>) ->
  Result<Shot, String>` (default: `unsupported("frame-png --session",
  "frame_from_session")`), implemented beside 2519's `session_from` by
  hello-grid's tools, the grid template's and 2511's `BasicTools`.
- **Either order with 2514** (frames over time): if 2514 is done,
  `frame-png --session <file> --until N` also takes its `--every` and
  `--count`, with its output flags: capture i is frame N + i × every of
  the record, played on with the record's own input (a capture past
  the record's end is an error that gives its length), painted as
  above, and the output is 2514's sheet, APNG or GIF. With steps after
  `--until`, the captures come after them, as 2514's do. A test on
  hello-grid's golden session: with `--count 4`, the sheet's first
  cell equals the one frame `--until` alone writes. If not, 2514 adds
  it (its clause says so).
- **MCP `seek { frame }`** in 1133's session. **From 1133 (done):**
  the server runs no game. It forwards each session tool to the host
  (`mcp::host::Host`, in the game's tool program), which keeps the
  game in `Live`, and its `start` launches that program with cargo, so
  starting again rebuilds the game. So `seek` is an `Op` in
  `Op::SESSION` (its arms in `Host::call` and `mcp::description`, its
  schemas `session.seek.*.json`), the server forwards it as it does
  `step`, and the host starts the game again as `reset` does (from
  `Live::start`, no rebuild), plays it to `frame`, and answers as
  `step` does:
  - for a session started from a record, it replays that record
    (2575's `Harness::from_session`, then `Harness::play_to`). A frame
    past the record's end replays the whole record, then the steps run
    after it, by the rule below;
  - for a session started from options, which runs on `Harness::new`
    and `App::frame_after` and is never recorded (2518 records only
    `App::frame` drivers), it replays the steps the session ran, which
    the host keeps in order in `Live` (each `step`'s script and idle
    frames).
    `frame` must be a frame where an earlier `start` or `step` answer
    ended; another frame is an error that lists those frames;
  - the next `step` plays on from there. The steps after `frame` are
    dropped from what the host keeps: a branch. A frame past the last
    one run is an error.
- The schemas regenerated, `contract::VERSION` raised
  (`crates/cli/src/contract.rs:52`), `crates/cli/schemas/versions.json`
  updated, and `docs/contract.md` says what `--until` and `seek` do.

**Part 2, going back in a running game (second).**

- **`Start::rerunnable(card, look, load: impl Fn(Setup) ->
  Result<Game<G>, String> + 'static)`**, beside `Start::new`:
  - it keeps the load as a `Rerun<G>`
    (`alloc::rc::Rc<dyn Fn(Setup) -> Result<Game<G>, String>>`) in a new
    field `rerun: Option<Rerun<G>>`, and its `load` calls it;
  - `Start::before_each_load(self, before: impl Fn(&mut Setup) +
    'static)` wraps the load and the rerun alike. `before_load` keeps its
    `FnOnce`; on a rerunnable start it wraps the first load only and
    drops `rerun`, as its doc comment then says;
  - `shuyi-app`'s `run` moves to `before_each_load` (cloning the voice
    manifest);
  - `hello_grid::start()` and the grid template's start (in their
    `lib.rs` since 2519) move to `Start::rerunnable`. `BasicGame::start`
    (2511) builds a rerunnable start, so the blank template and every
    game on the short path get it with no change of their own;
  - `App::new` keeps a copy of a rerunnable start to build the replay
    from: its card, look, system hook, 2518's `made` and `state_hash`,
    and the rerun;
  - **From 2518 (done):** `Start` has `made` and `state_hash` and is
    `#[non_exhaustive]` (ADR-0039), so `rerun`, set by a builder like them,
    is additive.
- **`UiAction::DebugBack`** in `shuyi_assets::keymap::action_set`,
  beside `Debug` (line 29):
  - like 2316's `DebugPause` and `DebugStep`, it acts only with debug
    tools on. The sample `ActionSet` (`keymap/sample.rs`, `sample.ron`)
    gains it, not rebindable, and the sample and the toy
    (`shuyi_ui::toy`) map it;
  - it gets its arms in `UiAction`'s own `ActionSet` impl (2511),
    neither required nor rebindable, and stays out of `ALL`, as 2511's
    rule says for every debug action, so the blank template's
    `files/keymap.ron` doesn't change;
  - every `match` on `UiAction` in this repository gains it where 2316's
    actions are (`crates/ui/src/widgets/ask.rs`,
    `widgets/name_entry.rs`, `toy/grid.rs`,
    `examples/hello-grid/src/screens/stage.rs`);
  - each press asks to go back one second.
- **The request.** 2315's `DebugView` (`Ctx::debug_view`) gains
  `rewind: Option<Rewind>`, with `Rewind::Back { seconds: u8 }` and
  `Rewind::To { frame: u32 }`. `DebugBack` sets `Back { seconds: 1 }`.
  `Game::take_rewind()` hands it out once. `DebugView` also gains two
  fields `App` writes each frame: `session_frame: Option<u32>` (the
  frame number while `App` records, else `None`) and `rewind_note:
  Option<String>` (why the game can't go back, or why the last request
  was refused).
- **The "Debug view" screen** (2315, with 2316's rows) gains:
  - a line `frame N` from `session_frame`;
  - rows "Back 1 s" and "Back 10 s";
  - a row "Go to frame N", which starts at the current frame; `Left`
    and `Right` move N by 60 frames, and `Confirm` asks for it;
  - when `rewind_note` is set, a line with it, and the rows do nothing
    while the game can't go back.

  Its labels are English, like every debug tool, with the line
  `// check-text: not player text (debug tools are English only)` as
  `crates/ui/src/debug.rs` has.
- **`App` goes back** (`crates/platform/src/app/rewind.rs`, `no_std`),
  inside `App::frame`, after a frame where `Game::take_rewind` gave a
  request. 2518 keeps the recorder in `App` (`App::record`), so
  `App::frame` does all of it, and the web build gets it with the
  desktop, since both run `App::frame` in `shuyi-app`'s loop:
  1. **The target.** `To` is that frame. `Back` is the latest frame at
     least `seconds` of game time before the current one. Frames on
     which the stack didn't update (2316's pause and slow motion) count
     no time, so `App` keeps, per recorded frame, the game time it
     handed the stack (a new `Game::last_frame_updated() -> bool` says
     whether it did). The target is never before the first frame after
     the load.
  2. **What is refused,** changing nothing, with the reason in
     `rewind_note` and in App's log (3040: a `Warn` line, category
     `Platform`):
     - a start made with `Start::new` (no rerun);
     - an `App` that isn't recording;
     - a record that stopped early (`truncated`: 2518's cap of
       `MAX_SESSION_FRAMES`, or a `frame_after`) and ends before the
       target;
     - a `To` frame after the current one;
     - a target frame on which the replayed game's system keyboard is
       open (`system_keyboard.is_waiting()`);
     - a load that fails;
     - a replay whose hash differs from the record's (it names the first
       such frame: a determinism bug, worth a ticket of its own).
  3. **The replay.** A new `App` built from the kept copy of the start
     (the rerun as its one-time load) plays the record from its first
     frame to the target through 2518's `session::Playback`, on
     `HeadlessPlatform::from_session` (`crates/platform/src/headless.rs:233`).
     That gives it the record's display, its store in a `MemoryStorage`
     (`crates/ui/src/storage.rs:73`) and its frame times, so `App::load`
     gives the same seed. It is silent and touches no device.
  4. **The swap.** Only the game moves over. `App`'s own state
     (suspended, focused, quit, the recorder) stays the device's:
     - the replayed game replaces the running one;
     - `App::branch_here` (new, shared with step 6) restarts the clock,
       as after a load (line 498), so the jump counts no time; forgets
       held input (`Game::forget_held_input`, 3042), so a key held at
       the target in the old run doesn't stick; and pauses the game
       (2316's `DebugView::paused`);
     - the device's music and voice stop (`Audio::stop_all`,
       `crates/platform/src/audio.rs:97`). Then the replayed game's
       track (`Game::music().target()`, `crates/ui/src/game.rs:436`) is
       loaded and started from its top: `MusicCommand::Load`, then
       `Start` (`crates/ui/src/audio.rs:320`), through `Audio::play`
       with an `AudioFrame` of those two commands and the game's
       volumes, as `step` builds one;
     - the new game takes the old one's debug channels (view only,
       2315).
  5. **Storage stays in memory.** After going back, the game keeps the
     replay's in-memory storage for the rest of the run, and its saves
     never reach the device. The device holds saves from after the
     target: writing over them would lose them, and reading them would
     break the branch's replay. The debug view says "saves stay in
     memory after going back".
  6. **The record branches.** The frames after the target are dropped,
     and the frames played next are recorded after it. 2518's
     `FrameRecord` gains `branch: bool` (skipped when false; follow 2518's
     ADR on the record's version), set on the first frame of the branch.
     `Playback` calls `App::branch_here` before it runs such a frame, so
     the branch replays to the same hashes.
- **A request in a playback is dropped,** with a warning: `App` acts on
  a request only when it records and its start is rerunnable.
  `Playback`, `Harness::from_session` and the replay itself don't
  record, and the contract's sessions from options have no rerun. A
  request that went through left no trace in the record (its frame came
  after the target and was dropped), and one that was refused replays
  the same.
- **A measure.** A test marked `#[ignore]` times the replay of a
  10-minute session of the toy's grid game (36 000 frames recorded with
  `Harness::start_recording`, a key every second), headless. Run it once
  with `cargo test -p shuyi-platform --release -- --ignored`, and write
  the number in the PR and the `rewind` module's docs. If it is over
  1 s, write a ticket for keyframes; don't build them here.
- **The ADR.** A short new ADR (the `write-adr` skill, the next free
  number, with its row in `docs/adr/README.md`) that links 2315's
  debug-view ADR, 2518's session-record ADR and ADR-0024 §6. It records:
  replay from the start, not snapshots; what is refused; storage in
  memory after a jump; the music restarted; the branch and its mark.
- **For the game:** `Start` gains a field and two constructors
  (above). `UiAction` gains a variant: a game that matches it
  exhaustively (hello-grid's `stage.rs` shows one) adds an arm, unless
  2316 made `UiAction` `#[non_exhaustive]` (2013 left it exhaustive).
  `DebugView`, `Game` and `Session` gain items; `RunInput` gains a
  field (a new contract version). Say so in the PR; the game's PR that
  moves the pin follows.

**Out (do not do):**
- snapshots of `App`'s state, and keyframes (a ticket only if the
  measure asks for one);
- going back in a turn game's rules: `shuyi_core::history` does it;
- rewinding sound that was playing: music restarts from its top, voices
  stop, and short sounds play out;
- writing a branch's saves to the device;
- a history of gizmos to scrub through: the target frame's gizmos are
  built again, which is enough;
- captures of a record's own frames in MCP's `frame_png`: after `seek`,
  2514's `every` and `count` idle on the branch;
- the editor's timeline over `seek`: 2506's research decides whether
  the live view has one (its panels say so);
- seeking a running windowed build over 2521's link;
- debug commands that go back (`back` and `goto` on 2356's commands):
  not planned, a ticket when someone needs them. Commands 2356 records
  replay with the frames through 2518's playback, with nothing to do
  here;
- networked sessions: recording and rewinding each peer's session in
  2529's `peers` is a follow-up that 2529 writes (its *Out*), and tick
  games' rollback (1147) goes back its own way;
- recording in builds without debug tools (2519 decides that).

## Implementation steps

1. Read 2518's and 2519's code and ADR (`shuyi_platform::session`,
   `Playback`, `HeadlessPlatform::from_session`, `Harness::from_session`
   and `play_to`, `App::record`, `Session::play_record`,
   `GameTools::session_from`), 1133's server (how its `frame_png`
   paints a session's frame, and its session state), 2315's and 2316's
   code (`DebugView`, the "Debug view" screen, the pause), 2511's
   `BasicGame` and `UiAction`'s `ActionSet` impl, 3042's
   `forget_held_input`, ADR-0022, ADR-0024 §6 and the `input-actions`
   skill.
2. Add `Session::play_record_to` and `until` to `run` and `scene`, with
   the contract's tests.
3. Add `--session`, `--any-stamp` and `--until` to `frame-png`, with
   `GameTools::frame_from_session` if step 1 found it is needed.
4. Add MCP `seek` and its stdio test.
5. Regenerate the schemas, raise `VERSION`, update
   `crates/cli/schemas/versions.json` and `docs/contract.md`.
6. Add `Start::rerunnable` and `before_each_load`. Move `shuyi-app`,
   `hello_grid::start()`, the grid template and `BasicGame::start` onto
   them. Format the templates as CI's template job checks them
   (`cargo fmt --all` doesn't reach `templates/`).
7. Add `UiAction::DebugBack` to `UiAction`'s `ActionSet` impl, the
   sample keymap, the toy and every match.
8. Add `Rewind`, `Game::take_rewind`, `Game::last_frame_updated`, the
   two `DebugView` fields and the "Debug view" rows.
9. Write `app/rewind.rs`: the target, the refusals, the replay, the
   swap with `App::branch_here`, and the branch with its mark in 2518's
   `FrameRecord` and `Playback`. Call it from `App::frame`.
10. Time the replay and write the number down.
11. Write the ADR.
12. Run the gates. Parts 1 and 2 together may pass about 600 changed
    lines (schemas and snapshots aside). If the diff will, open the PR
    after step 5 with Part 1 only, write steps 6 to 11 and Part 2's
    criteria and tests as a ticket of their own (the `write-ticket`
    skill: After MVP, blocked by this ticket's blockers), and say so in
    the completion notes.

## Acceptance criteria

Part 1:

- [ ] `shuyi run --session <file> --until N` on hello-grid answers with
      frame N's snapshot, the same as the Harness playing the record's
      first N frames.
- [ ] `run --session <file> --until N --keys <script>` plays the keys
      after frame N, and answers as the same keys played after a
      playback to N.
- [ ] A record changed at frame K below N fails `--until N` and names
      frame K.
- [ ] `--until` without `--session` is an error; `--until` of 0 or past
      the record's end is an error that gives the record's length.
- [ ] `frame-png --session <file> --until N` writes frame N.
- [ ] MCP, a session from options: `start`, step a key script to frame
      300 and another to frame 600, `seek 300`, then the second script
      again gives the snapshot seen at 600; other input gives a branch;
      a seek to a frame no answer ended on, or past the last frame, is
      an error.
- [ ] MCP, a session from Hello, Grid's golden session (2519): `seek N`
      for an N inside it answers with the snapshot that
      `run --session <file> --until N` gives.
- [ ] The schemas and `contract::VERSION` changed together (the schemas'
      test passes).

Part 2:

- [ ] An `App` test on the `HeadlessPlatform`, recording with
      `App::record` from a rerunnable start: the toy's grid game runs
      600 frames, then `DebugBack` lands on the frame one second of game
      time back, paused, with the record's hash for that frame and the
      snapshot the straight run showed then.
- [ ] Going back across a debug pause counts only the frames that
      updated.
- [ ] After going back and playing on, the branch's record replays
      through 2518's `Playback` to the same hash on every frame, its
      `branch` mark included.
- [ ] During the replay, no storage write and no audio call reaches the
      device (a counting fake platform). At the swap the device gets
      `stop_all`, then `Load` and `Start` of the replayed game's track.
- [ ] Each refusal changes nothing and says why in `rewind_note`: a
      `Start::new` start, an `App` that isn't recording, a truncated
      record that ends before the target, a `To` frame after the current
      one, an open system keyboard, a replay made to diverge on purpose.
- [ ] With debug tools off, `DebugBack` and the rows do nothing.
- [ ] `check-keys` passes, and help text names `DebugBack` through the
      keymap.
- [ ] hello-grid, the grid template and a game from `shuyi new blank`
      build on rerunnable starts (CI's template job).
- [ ] The replay's time for 36 000 frames is in the PR and the module
      docs.
- [ ] The ADR is written and listed in `docs/adr/README.md`.

Both parts:

- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit:
  - the target: `Back` over paused frames, never before the first
    frame after the load, `To` after the current frame refused;
  - the request taken once;
  - a rerunnable start, and `BasicGame::start()`, loaded twice from
    equal setups gives games with the same hash;
  - `before_each_load` runs before the first load and before each
    rerun; `before_load` on a rerunnable start drops the rerun;
  - `until`'s checks on `RunInput`; MCP `seek`'s frame rule for a
    session from options.
- Property: for any script of N frames and any K of at most N, going
  back to K and replaying the input of frames K+1 to N ends with the
  straight run's hash.
- Snapshot / integration:
  - the contract's `run`, `scene` and `frame-png` with `--until`, and
    the MCP stdio test with `seek`;
  - the "Debug view" screen with the new rows, and with `rewind_note`
    when the game can't go back;
  - hello-grid and the templates build on rerunnable starts (CI's
    template job).

## Completion notes

*(Filled in by the session that completes the ticket.)*
