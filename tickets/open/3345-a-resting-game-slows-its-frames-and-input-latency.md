---
id: "3345"
title: A resting game slows to a rest rate until its picture or input changes, and each frame's input latency is recorded
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3345 — A resting game slows to a rest rate until its picture or input changes, and each frame's input latency is recorded

## Context

The engine's first games are turn and grid games. They sit still most of
the time: a map waiting for the player's move, a menu, a dialogue line.
Today they still run, draw and paint 60 frames a second while nothing on
screen changes. On a laptop, a Steam Deck or a phone that is wasted
battery and heat. 3013 names it as where "a careless engine shows", and
its budget is 8 W on an idle battle map. Godot has a low-processor mode
for this.

What exists now:

- **The frame cap** (2321, done; [ADR-0050](../../docs/adr/0050-a-setting-the-platform-applies-a-request-with-a-serial-confirmed-by-a-report-before-the-frame.md)).
  `shuyi_ui::frame_cap::FrameCap` is only `Off` or `Fps(u16)`
  (`crates/ui/src/frame_cap.rs`). The pacer in `crates/app/src/pacing.rs`
  waits to it. Nothing lowers the rate when the game is at rest.
- **The background** ([ADR-0077](../../docs/adr/0077-a-background-policy-in-the-games-settings-paused-or-muted-without-focus-applied-by-app.md), `GameSettings::BACKGROUND`,
  `crates/ui/src/settings.rs`) says what a game does without the focus.
  3043 adds fewer frames in a background web tab. Neither covers a
  focused window that shows the same picture frame after frame; 3043's
  own Out says so.
- **The web already skips frames.** Under a cap the browser's
  `requestAnimationFrame` sets the rate, so `run` in
  `crates/app/src/lib.rs` skips the game on an early frame and calls
  `App::present_again` (`crates/platform/src/app.rs`), holding its input
  for the next frame that runs.
- **The frame hash is not the picture.** `App::frame_hash` hashes "what
  happened in it, never its picture", and only while recording
  ([ADR-0072](../../docs/adr/0072-session-records-app-notes-what-the-platform-gives-a-hash-per-frame-of-what-happened-and-ron-never-migrated.md)).
  The picture is `Show::Game`'s `GlyphBuffer`
  (`crates/platform/src/platform.rs`), which derives `PartialEq`
  (`crates/ui/src/frame/glyph_buffer.rs`).
- **The perf record** (`shuyi_ui::perf::FrameRecord`, `FrameTimes` in
  `crates/ui/src/perf.rs`; [ADR-0046](../../docs/adr/0046-perf-traces-measured-by-the-backend-kept-by-app-beside-the-log-and-read-by-agents-as-json-lines.md))
  has `update_us`, `draw_us`, `paint_us`, `present_us` and `frame_us`.
  Nothing records how long a key press takes to reach the screen. A grep
  for `input latency|input lag|latency from` in `tickets/open` and `docs`
  finds nothing that plans it.

Why not "redraw only when the picture changes" outright: macroquad clears
the window at the start of every frame (`Context::begin_frame` in
macroquad 0.4.16 calls `clear`), so a frame that is shown must be painted
again, and the loop never waits on the system's events. So at rest this
ticket shows frames less often, at a rest rate, instead of never. Waking
at once on an event is a follow-up (Out).

The latency number matters for the real-time games (1119) and for the
Deck: once frames slow at rest, the first key press after a rest waits
for the next rested frame, and this number shows how long that is.

**Either order with 3013:** if 3013 is done, read the idle draw it
recorded in `docs/targets/steam-deck.md`; if it was over 8 W, say in the
completion notes that this is the ticket its bullet asked for, and add a
line to that page saying the rest rate is how a resting game lowers it
(the number is measured again by a later Deck run, not here). If not,
nothing to do here (3013 measures the idle draw with the rest on and off).

## Nick input

None.

## Scope

**In:**
- A `Rest` setting the game chooses: after how many frames of the same
  picture with no input the game rests, and the frames a second it shows
  while resting. A `GameSettings::REST` constant, beside `BACKGROUND`,
  with a default that rests (about 1 s, then 20 frames a second; both
  *Tunable*), and `Rest::NEVER` for a game that never rests.
- A screen keeps the game awake for a frame
  (`Ctx::keep_awake`): for a timer that must fire on time, or a
  real-time game whose picture holds still while its world moves.
- `App` decides each frame whether it is at rest, and says so on the
  `Frame` it presents. A frame is still when its picture
  (`Show::Game`'s buffer, background and look; a card's or the errors'
  frame), its debug overlay and its fullscreen flag equal the last
  frame's, it read no input and no system event, and nothing kept it
  awake. Any change ends the rest on the next frame.
- The native pacer paces to the rest rate while at rest, and back to the
  cap at once. On the web the same rest skips the game on the browser's
  early frames, as the cap already does.
- At rest the game still runs a frame at the rest rate: its time, its
  tick steps and its timers go on, so a session record replays the same
  (only `dt` differs, which the record already holds).
- The `Pacing` report says whether it is resting, so a game and the
  debug readout can see it.
- A perf record field with the input latency: from the start of the
  frame that read the input to the end of the swap that showed its
  picture. It is in the trace file (`SHUYI_PERF`) and in `run --perf`'s
  summary as a spread.
- New public items follow ADR-0039: `Rest` is a `#[non_exhaustive]`
  struct with constructors; the new fields on `Frame`, `Pacing` and
  `FrameRecord` (each already `#[non_exhaustive]`) are additive, read
  with `#[serde(default)]` so old records still load. Nothing breaks for
  the game; its PR that moves the pin may set `REST` if the default
  doesn't suit it.

**Out (do not do):**
- Waking at once on input while resting (miniquad's
  `blocking_event_loop` with `schedule_update`, which macroquad 0.4.16
  calls on its input events): a follow-up if the latency recorded here
  shows the first press after a rest is late.
- An options row for the player to turn the rest off: a follow-up if a
  game asks.
- The background frame rate of a web tab without the focus (3043).
- Measuring the Deck's power draw (3013).
- The Playdate's fixed 30 frames a second: its backend
  (`crates/playdate`) ignores the rest field, as it ignores the cap.
- The event's own timestamp from the system (macroquad doesn't give
  one): the latency starts when the frame reads the input. The time the
  event waited before that is at most the interval before the frame,
  which `frame_us` already holds; the docs say so.

**Estimated size:** about 1,400 lines (code 600, tests 600, docs 200).

**Split:** past 900 lines, split off steps 6–7 (the input latency in the
perf record and the summary) into a new ticket.

## Implementation steps

1. **`Rest` in `shuyi-ui`.** In `crates/ui/src/frame_cap.rs` add `Rest`
   (`after_frames: u16`, `fps: u16`, held to `MIN_FPS` to the cap),
   `Rest::DEFAULT`, `Rest::NEVER` and `Rest::new`. Add
   `const REST: Rest = Rest::DEFAULT;` to `GameSettings`
   (`crates/ui/src/settings.rs`) with a doc comment like `BACKGROUND`'s.
   Add `resting: bool` to `Pacing`.
2. **Keep awake.** `Ctx::keep_awake(&mut self)` sets a flag for this
   frame; `Game` takes and clears it each frame. Document it on `Ctx`
   with the two uses in Scope.
3. **`App` finds rest.** In `crates/platform/src/app.rs` keep what the
   last presented frame showed. Reuse the allocation
   (`GlyphBuffer::clone_from`), or keep a hash of it if the bench shows
   the compare costs more; say which in the completion notes. Count
   still frames as Scope defines them; at `REST.after_frames` the frame
   is at rest. Add `rest: Option<u16>` to `Frame`
   (`crates/platform/src/platform.rs`): the rest rate while at rest,
   `None` otherwise.
4. **The native pacer.** In `crates/app/src/pacing.rs`, `Pacer` takes
   the rest rate with the cap (`set_cap` gains it, or a `set_rest`), and
   paces to the lower of the two while it is set. The rules of the file
   still hold (deadlines, never wait twice). Leaving rest starts the
   deadlines again from the next frame, so the first frame after a rest
   is not delayed. `pace` in `crates/app/src/lib.rs` reads it from the
   backend (`Macroquad::frame_cap` and its new rest), and the report
   says `resting`.
5. **The web.** `Pacer::web_frame` skips the game on browser frames
   that come before the rest period, with the input held as now.
6. **The input latency.** In `crates/ui/src/perf.rs` add
   `input_us: Option<u32>` to `FrameRecord`: on the record of the frame
   after one that read input, from that frame's start to the end of the
   swap that showed it (its `frame_us` minus its own wait before it,
   plus this frame's `present_us`); `None` when the last frame read no
   input. `App` fills it in `end_perf`, where it has both times.
7. **The summary.** `PerfSummary` in `crates/cli/src/contract/perf.rs`
   gets an `input` spread, as `present` has, and the line for people
   names it.
8. **Docs.** `docs/platform-backends.md`, its "The frame cap" section: the
   rest field and how a backend applies it. ADR-0050 gets an
   "Amended by 3345" note: rest is a second rate on the same report.
   ADR-0046 §7 lists `input_us` and says what it leaves out. The book's
   saves-and-settings chapter
   (`site/content/learn/book/saves-and-settings.md`) gets a paragraph on
   `REST` and `keep_awake`.

## Acceptance criteria

- [ ] A headless test: a game whose picture holds still and reads no
  input is at rest from frame `after_frames` on (`Frame::rest` is the
  rest rate), and not on the frame before.
- [ ] A key press, a mouse move, a system event (focus, resize), a change
  of one cell, of the background, of the look, of the overlay's reload
  errors, or `Ctx::keep_awake` each end the rest on that frame, and the
  count starts again.
- [ ] `Rest::NEVER` never rests; `Rest::new` with a rate over the cap
  is held to the cap, and one under `MIN_FPS` to `MIN_FPS`.
- [ ] Pacer tests on the clock-free `Pacer`: at rest with a cap of 60
  and a rest of 20, frames start 50 ms apart; the frame after the rest
  ends starts at most one cap period (16.7 ms) after it, not 50 ms; with
  vsync limiting at 60 and no rest it still never waits.
- [ ] The web pacer at rest runs the game on one browser frame in three
  at 60 Hz with a rest of 20, and the held input reaches the next frame
  that runs.
- [ ] A session recorded across a rest replays to the same frame hashes.
- [ ] `FrameRecord::input_us` is set on the frame after one that read
  input and `None` otherwise; a record without the field loads.
- [ ] `run --perf` on an example with an input script reports the
  input spread.
- [ ] The bench's instruction budgets (3008) still pass with the
  picture compare in each frame.
- [ ] The book's chapter is updated (2802), or the completion notes have
  a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Rest`'s constructor and holding; `App`'s still-frame count for
  each thing that ends a rest (the list in the criteria); the pacer's
  rest pacing on native and the web; `input_us` filled and empty.
- Property: for any sequence of still and changed frames, the frame is
  at rest exactly when the last `after_frames` frames were all still.
- Snapshot / integration: a harness run of a grid example that rests,
  then takes a key, with its frames' `rest` and the record replayed;
  the perf summary's JSON with the input spread.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
