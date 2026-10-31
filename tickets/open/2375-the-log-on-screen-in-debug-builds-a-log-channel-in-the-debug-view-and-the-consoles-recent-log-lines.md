---
id: "2375"
title: "The log on screen in debug builds: a `log` channel in the debug view, and the newest log lines in the debug console"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: ["2315", "2356"]
nick_input: none
completed:
---

# 2375 — The log on screen: a `log` channel and the console's log lines

## Context

3040 (done,
[ADR-0030](../../docs/adr/0030-the-log-plain-lines-kept-per-app-output-only-and-handed-to-the-backends-sinks.md))
gave the engine one log. `Ctx::log` keeps a game's lines until `App`
takes them at the end of every frame (`Ctx::take_log`, called from
`App::take_game_log` in `crates/platform/src/app.rs`). `App` stamps
each line with its frame, keeps the last 500 that its `LogFilter` lets
in, and hands them to tests (`Harness::log`), to the contract (`run`'s
and `scene`'s answers) and to the backend's sinks
(`App::take_new_log`).

3040's **Out** left two pieces for later:

- "a log channel in the debug view (a follow-up once 2315's channels
  exist)";
- "the log in the debug console (2356)".

Nobody wrote them. This ticket is that follow-up. 2315 lists it among
its later channels, and 2356's **Out** leaves the console's log lines
here.

**Why it matters.** A developer plays a debug build and sees a screen
go wrong. The line that says why (a settings file it couldn't read, a
voice manifest it refused, a warning the game logged itself) goes to a
terminal they may not have open, and on a handheld there is no
terminal at all. Godot shows a running game's output in its editor's
Output panel. Here the newest lines show over the frame, turned on from
the debug menu, and in the console where commands are typed.

**The gap under both pieces.** A screen sees only `&Ctx`. The kept
lines live in `App`, in `shuyi-platform`, which `shuyi-ui` can't
depend on. So the first part of this ticket is a way back: with debug
tools on, `App` hands the lines it kept in each frame back to the
game's context, which keeps the newest few for the debug tools.

The rules this keeps:

- **Output only** (ADR-0030 §4). The debug tools show the lines, and no
  engine or game code reads them to decide anything. A replay with the
  channel on has the same state hash as one with it off.
- **Debug tools only** (`Ctx::debug_tools`, `shuyi_ui::DEBUG_TOOLS`). A
  shipped build keeps no copy and pays one branch per frame.
- **Graphics are a skin** (`AGENTS.md` rule 9). The channel is 2315's
  gizmos, and its tests read the gizmo list and the snapshot lines.
- **No key is hard-coded** (rule 8). The channel is turned on from
  2315's "Debug view" menu, and the console is 2356's.

2356's step 9 may move its console to a follow-up ticket. Then 2356's
PR adds that ticket to this one's `blocked_by`, and part 3 below works
on that ticket's console.

## Nick input

None.

## Scope

**In:**
- **An ADR** (the `write-adr` skill, the next free number, listed in
  `docs/adr/README.md`). Its `Extends:` line names ADR-0030 §2 and §4,
  and ADR-0030 itself is not edited. It records:
  - with debug tools on, `App` hands the lines it kept in each frame
    back to the game's context, which keeps the newest
    `RECENT_LOG_LINES`;
  - the debug tools show them, and nothing decides on them;
  - a shipped build keeps none;
  - a line logged in frame N shows from frame N+1, since App takes it
    after frame N is drawn.

  The alternatives it weighs: `Ctx` copying each line as it is logged
  (no frame, no filter, and none of App's own lines), and the debug
  tools reading `App` (a screen sees only `Ctx`).
- **The way back** (`shuyi-ui` and `shuyi-platform`):
  - `shuyi_ui::log::RECENT_LOG_LINES: usize = 16`;
  - `LogLine::ascii_line(&self) -> String`: `"{level} {category}:
    {text}"`, with the names `LogLevel::name` and `LogCategory::name`
    give (`warn storage: settings: unreadable: …`). Every character
    that isn't printable ASCII (`' '..='~'`) becomes `?`, since a label
    and the console print ASCII only;
  - `Ctx` gains a private `recent_log: LogRing` of
    `RECENT_LOG_LINES`, and two methods:
    - `Ctx::push_recent_log(&mut self, line: LogLine)`, for the
      platform's `App` or a front end of its own: keeps the line,
      dropping the oldest past 16. With `debug_tools` off it does
      nothing;
    - `Ctx::recent_log(&self)`: the kept lines, oldest first (the
      `vec_deque::Iter` that `LogRing::lines` returns). Its doc says a
      line shows from the frame after it was logged;
  - `App` gains a private `log_shown: u64`, the sequence number of the
    first kept line not yet handed back, and a private
    `fn show_new_log(&mut self)`. When the game runs and its context
    has `debug_tools` on, it hands each line of
    `self.log.since(self.log_shown)` to `Ctx::push_recent_log`, in
    order. Either way it then sets `log_shown` to
    `self.log.next_seq()`. `App::end` calls it after `take_game_log`,
    and `App::running` after its own take. So the context gets exactly
    the lines App's filter kept, stamped with their frame, App's own
    included, each once. With debug tools off nothing is cloned.
- **The `log` channel** (`shuyi_ui::debug::log_channel`,
  `crates/ui/src/debug/log_channel.rs`; `no_std` + `alloc` through
  `crate::prelude`, as the rest of `shuyi-ui`):
  - the channel: id `log`, menu label `Log`, with the
    `// check-text: not player text (debug tools are English only)`
    marker that `debug.rs` uses;
  - `LOG_VIEW_LINES: usize = 6`;
  - `log_gizmos<G: GameData>(ctx: &Ctx<G>, spec: ConsoleSpec, out:
    &mut Gizmos)`. When `out.channel_on("log")`, it pushes the newest
    lines of `ctx.recent_log()` as `Label` gizmos at the top left of
    the frame, oldest at the top: line `i` at `(0, i * cell_h)` in
    console pixels. It shows at most
    `min(LOG_VIEW_LINES, max(1, spec.rows / 3))` lines, each line's
    `ascii_line` cut to `spec.cols` characters;
  - the roles: `Warn` and `Error` lines get a new role,
    `GizmoRole::Alert` (snapshot name `alert`), painted in
    `EngineColor::Warning`. `Info` and `Debug` lines get `Note`.
    `GizmoRole` is `#[non_exhaustive]` (2315), so the role is additive;
  - **offered by the engine, not by a screen.** `Game` offers `log`
    whenever debug tools are on, the way 2315 lets a screen offer a
    channel, so the "Debug view" menu lists it over every screen. If
    2315's list is filled only from `Screen::debug_draw`, add a way for
    `Game` to offer a channel. After the stack's `debug_draw` pass,
    `Game` calls `log_gizmos`, so the log's labels come after every
    screen's gizmos;
  - 2315's `frame-png --debug log` and `--debug all` draw it with no
    change.
  - **Either order with 2316** (the top screen's readout): its
    `readout` channel prints `Label` lines from the top-left corner
    too. The readout keeps rows 0 to n - 1 (n its lines, `paused`
    included), and the log's lines start on row n, cut to the rows
    left. If 2316 is done, `log_gizmos` takes the first row
    (`first_row: usize`), `Game` passes the readout's line count (0
    with the `readout` channel off), and a test with both channels on
    finds no two labels on one row; if not, draw from row 0 as above,
    and 2316 adds the first row.
- **The console's log lines** (2356's `Ctx::debug_console:
  ConsoleLog`):
  - `Ctx::push_recent_log` also writes each `Warn` and `Error` line to
    the console's log as its `ascii_line`, after the commands and
    replies already there. `Info` and `Debug` lines stay out: 2356's
    `Game::command` logs every command at `Info`, and the console
    already shows each command and its reply;
  - a log line in `ConsoleLog` is marked as one (2356's lines are
    commands and replies). `ConsoleView` passes the mark on, and
    `paint_console` paints those lines in `EngineColor::Warning` (the
    look is a skin);
  - they count toward the console log's last 64 lines like the rest.
- **Docs** (the `document` skill):
  - rustdoc on every new item;
  - the `log` channel in `shuyi_ui::debug`'s module docs, beside
    `grid`;
  - `docs/platform-backends.md`: with debug tools on, `App` hands each
    frame's kept lines back to the context. A front end of its own over
    `Game` (no `App`) calls `Ctx::push_recent_log` to fill the view, or
    the view shows nothing.
- **For the game's pin bump:** additive. `Ctx` gains a private field
  and two methods, `LogLine` a method, and `GizmoRole` a variant of a
  `#[non_exhaustive]` enum. If marking a log line in `ConsoleLog`
  changes the shape of a public type of 2356's, CI's `api` job finds
  it, and the completion notes carry its `Breaking:` line.

**Out (do not do):**
- A filter of the view's own, by level or category, chosen from the
  menu. The view shows what App's `LogFilter` keeps, and 3041 reads
  that filter from `SHUYI_LOG`. Write a ticket when someone needs more.
- Scrolling back through older lines. The 500 kept lines are read
  through the contract's `run` and `scene` answers (3040, done), and
  from a running windowed build through the live link (2521).
- The backend's own lines that never reach `App` (render, audio, pads):
  3041 sends them to its sinks.
- Timing spans and the frame profiler: 2325 (parked).
- A message log for players: 2766 (parked). This view is for
  developers, and English only.
- New keys or actions. The channel is turned on from the debug menu,
  and the console keeps 2356's keys.

## Implementation steps

1. Read ADR-0030 and the ADRs of 2315 and 2356. Read
   `crates/ui/src/log.rs`, `ctx.rs`, `debug.rs`, `debug/gizmo.rs`,
   `debug/console.rs`, `debug/glyph.rs` and `game.rs` (where it asks
   the stack for gizmos). Read `crates/platform/src/app.rs` (`end`,
   `running`, `take_game_log`) and its tests. Check whether 2356 moved
   its console to a follow-up ticket: if so, that ticket is in this
   one's `blocked_by`, done, and holds `debug/console.rs`.
2. Write the ADR.
3. Add `RECENT_LOG_LINES` and `LogLine::ascii_line`, with unit tests in
   `crates/ui/src/log/tests.rs`.
4. Add `Ctx::push_recent_log`, `Ctx::recent_log` and the console's
   write, with tests in `crates/ui/src/ctx/tests.rs`.
5. Add `App::show_new_log` and its two calls, with tests in
   `crates/platform/src/app/tests.rs`. Clone the lines out of the ring
   before borrowing the running game mutably, as `take_game_log` takes
   the game's lines before it logs them.
6. Add `GizmoRole::Alert`, its snapshot name and its color. Add
   `debug/log_channel.rs` with its tests, and `Game`'s offer and call.
7. Mark log lines in `ConsoleLog`, and paint them.
8. Write the Harness tests below and the docs.
9. Run the gates.

## Acceptance criteria

- [ ] An ADR records the way back and extends ADR-0030, and
      `docs/adr/README.md` lists it.
- [ ] A Harness over the toy, with debug tools on and the `log` channel
      on: a line logged with `harness.ctx_mut().log(LogLevel::Warn,
      LogCategory::Screens, "a test warning")` after frame 2 is in
      `Ctx::recent_log` after frame 3, carrying frame 3. The snapshot
      after frame 4 lists it in the overlays as
      `over  gizmo log alert label 0,0 "warn screens: a test warning"`.
- [ ] `Ctx::recent_log` holds the newest 16 lines App kept, each once,
      in order, with their frames. With App's filter at `Warn`, an
      `Info` line never reaches it.
- [ ] With debug tools off (`Game::with_debug_screens(false)`),
      `Ctx::recent_log` stays empty, `log` is not offered, and the
      snapshot has no `log` gizmo.
- [ ] A Harness replay of the toy's grid game (2315's replay-hash test)
      has the same state hash with `log` on and a line logged every
      frame as with no channel on.
- [ ] A non-ASCII character in a line shows as `?`, and a line longer
      than the frame is wide is cut at its width.
- [ ] The console shows a `Warn` line after the command and the reply
      logged before it, and never an `Info` line. Its insta snapshot is
      committed.
- [ ] No key is named in code (`check-keys` passes), and the channel's
      label carries the `check-text` English-only comment.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit:
  - `LogLine::ascii_line`: each level's and category's name, and
    non-ASCII and control characters as `?`
    (`crates/ui/src/log/tests.rs`);
  - `Ctx::push_recent_log`: the newest 16 kept, nothing with debug
    tools off, and `Warn` and `Error` lines in the console's log while
    `Info` and `Debug` lines are not (`crates/ui/src/ctx/tests.rs`);
  - `log_gizmos`: nothing with the channel off; the newest lines,
    oldest at the top, at `(0, i * cell_h)`; `Alert` for `Warn` and
    `Error` and `Note` for the rest; cut at `spec.cols`; one line on a
    frame of 2 rows (`crates/ui/src/debug/log_channel/tests.rs`);
  - App: each kept line handed back once, in order, with its frame,
    across frames and from `App::running`; a line the filter dropped
    never handed back; nothing with debug tools off
    (`crates/platform/src/app/tests.rs`).
- Property (`proptest`, already a dev-dependency of `shuyi-ui`): for
  any lines pushed and any `ConsoleSpec` of at least one row and one
  column, `log_gizmos` never yields a label outside the buffer's
  `pixel_bounds` or longer than `spec.cols`.
- Snapshot / integration:
  - the Harness test above and its twin with debug tools off
    (`crates/platform/tests/it/`);
  - 2315's replay-hash test of the toy's grid game, with `log` on;
  - an insta snapshot of 2356's toy console
    (`crates/ui/tests/it/debug_console.rs`) with a command, its reply,
    and a `Warn` line pushed after them.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
