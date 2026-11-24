---
id: "3344"
title: "Native targets run a game's sliced work on a worker pool, and the result is the same as on one thread"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3009"]
nick_input: none
completed:
---

# 3344 — Native targets run sliced work on a worker pool, with the same result as one thread

## Context

The engine runs all of a game's work on the main thread, on every
target, by policy:

- [ADR-0128](../../docs/adr/0128-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md)
  §4 says "No threads and no async asset server" (after ADR-0007,
  *Alternatives*). Loads run in steps on the main thread
  ([ADR-0097](../../docs/adr/0097-a-load-in-steps-a-slice-a-frame-from-the-platform-and-the-one-shot-load-kept-beside-it.md)).
- 3009 slices a bot's search over frames by work units, never time, so
  the frame rate holds and the decision is the same on every device. Its
  Out says "threads", because the Playdate has one core and the web build
  has no threads we use.
- [ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
  §10 allows threads only in the parked RTS module (2717's `rayon`
  feature), and 2108 plans a parallel ECS schedule. Both split one step
  across threads and wait for it to finish.

So a desktop, a Steam Deck or a phone with eight cores runs a bot's
search, a path search or a level generator on one of them, sliced into
the frame. The other cores stay idle, and a slow search takes many frames
to finish. Godot ships a `WorkerThreadPool`, and Bevy ships an
`AsyncComputeTaskPool`.

The engine already has the two halves:

- **Work that can be cut anywhere.** 3009's `Think` (`step(units) ->
  Progress`) gives the same decision whether it is stepped by 1 unit or
  by `u32::MAX`. That is what lets a worker run it to the end while the
  main thread runs it in slices, with one answer.
- **Threads in a backend.** `shuyi-app` already runs closures on threads
  of its own: `Pending` (`crates/app/src/audio/native_music.rs`) decodes
  music off the frame, and the in-house mixer has a worker
  (`crates/app/src/audio/in_house.rs`). `shuyi-cli`'s
  `crates/cli/src/pool.rs` runs a work list on `n` threads and gives the
  answers back in the list's order, resuming a panic on the calling
  thread, so its result never depends on the thread count.

This ticket joins them. A game hands a `Think` to the engine as a job. On
a target whose backend has a pool, a worker runs the job. Anywhere else,
`App` steps it on the main thread as 3009 does. Either way the result is
the same value, and it reaches the game at a frame boundary. It amends
ADR-0128 §4 for native targets only. The web build, the Playdate and the
other small targets keep running everything on one thread.

The time a result arrives does depend on the device and on how busy the
pool is. That is already true of 3009's slices on a slow device, and it
is safe for the same reason: a result reaches the simulation only through
a command (turn games) or a step's input (tick games), which the record
keeps (ADR-0002 §2). A session record notes the frame each result was
handed over, as it notes the system keyboard's answer (ADR-0072), so a
recorded run plays back the same on the headless backend.

From the engine blind-spot audit of 2026-11-23 (its proposed ticket 60).

## Nick input

None.

## Scope

**In:**
- **An ADR** (the `write-adr` skill) that amends ADR-0128 §4: on a
  target whose backend says it has a pool, a game's jobs run on worker
  threads. Each job is a `Think`, so its result never depends on the
  thread count, the slice size or the timing. A result enters the
  simulation only through a command or a step's input. Loads, decoding
  and the simulation's own steps stay on the main thread. The web build
  and the small targets have no pool. Name 3009's `Think` as the job's
  contract and `crates/cli/src/pool.rs` as the precedent for giving
  answers back in a fixed order.
- **A job mailbox in `shuyi-ui`** (`crates/ui/src/jobs.rs`), shaped like
  the system keyboard's (`crates/ui/src/system_keyboard.rs`), on `Ctx` as
  `ctx.jobs`:
  - `start<J>(&mut self, job: J) -> JobTicket` where `J: Think + Send +
    'static` and `J::Decision: Send + 'static`. The mailbox boxes it
    behind an object-safe trait of its own, so `App` and the backend
    never see the game's types;
  - `take<D: 'static>(&mut self, ticket: JobTicket) -> Option<D>`: the
    result once it has been handed over, `None` before. A wrong `D`
    panics with a message that names the ticket;
  - `cancel(&mut self, ticket: JobTicket)`: the job's result is dropped
    when it arrives, and a worker stops it at its next chunk;
  - `JobTicket` is a `Copy` number that counts up from 0 per run, so a
    recorded run and its playback number jobs alike.
  - **If 3009 already gave `App` a way to take a `Think` from a screen**
    (read its completion notes first), the pool is a second branch of
    that path, and no second mailbox is added: rename nothing 3009 made
    public.
- **A seam in `shuyi-platform`:** a `Jobs` trait (`crates/platform/src/jobs.rs`)
  with `start(&mut self, ticket, job)`, `cancel(&mut self, ticket)` and
  `poll(&mut self, done: &mut Vec<(JobTicket, Finished)>)`, and
  `Platform::jobs(&mut self) -> Option<&mut dyn Jobs>` with a default of
  `None`, as `Platform::text_input` has. A default method keeps backends
  outside this repository building.
- **`App` routes the jobs** (`crates/platform/src/app.rs`):
  - after the update it moves the mailbox's new jobs to
    `Platform::jobs()` when that is `Some`;
  - when it is `None` it steps each job itself, oldest first, within
    3009's `budgets.think_units_per_frame`, exactly as 3009 steps a
    thinking bot;
  - before the next update it hands every finished result back to the
    mailbox in ticket order, never in finishing order;
  - a panic in a job on a worker is resumed on the main thread at hand
    over, so the crash path (ADR-0079) and the panic message are the ones
    one thread would give.
- **Records.** `FrameRecord` (`crates/platform/src/session.rs`) gains
  `jobs_done: Vec<u32>`, the tickets handed over that frame (`#[serde(default,
  skip_serializing_if = "Vec::is_empty")]`, so older records still read
  and `FORMAT_VERSION` needn't move). Playing a record back, `App` runs
  each job to its end on the main thread and hands it over at the frame
  the record names. The Harness keeps 3009's rule: a job runs to
  completion in one call, so tests stay fast.
- **The pool in `shuyi-app`** (`crates/app/src/jobs.rs`), native only:
  - `std::thread` workers, no new dependency. The count is
    `available_parallelism() - 1`, at least 1 and at most 8, made once
    when the backend starts;
  - a worker takes the oldest waiting job and steps it in chunks of
    `think_units_per_frame` (or 4 096 units if the profile has none),
    checking its cancel flag between chunks;
  - finished results go back through a channel that `poll` drains; the
    backend's `Drop` sets every cancel flag and joins the workers;
  - the web build of `shuyi-app` returns `None` from `Platform::jobs`.
    A backend may branch on its own target; screens and rules never do
    (ADR-0060).
- **The engine's searches become jobs.** A job must own what it reads,
  and today `RolloutSearch<'_, R, O, P, E>`
  (`crates/core/src/bots/rollout.rs`) and `PlanSearch<'_, R, O, E>`
  (`crates/core/src/bots/plan.rs`) borrow their bot (`bot: &'a mut
  Rollout<…>`). Give each an owned form: the bot moves into the search
  (for example `Rollout::into_search(self, &rules)`) and comes back with
  the decision (`into_bot`), its luck where one thread would leave it.
  The borrowed form stays. Do the same for 2112's `Solver` if it borrows
  what it searches.
- New public types follow ADR-0039: `JobTicket` is a plain `Copy`
  newtype; `Finished` is opaque; `Jobs` is a trait whose added default
  method is additive. `FrameRecord`'s new field is additive.
- **Either order with 2113:** if 2113 is done, give its minimax and MCTS
  searches the same owned form and run each fixture through the pool in
  the equivalence test; if not, nothing to do (2113 does it, its scope
  says so).

**Out (do not do):**
- Splitting one step across threads and waiting for it: 2108's parallel
  ECS schedule and 2717's `rayon` passes. Those run inside a step; a job
  runs beside the frame.
- Loads and asset decoding on the pool. ADR-0097's load steps build one
  game in order, and 2270 measures large files first; if its numbers
  show a hitch, that is a ticket of its own.
- Threads on the web build (macroquad's loader, game ADR-0034), the
  Playdate, the GBA or the DS.
- Changing `Pending` or the mixer's worker.
- Time limits of any kind on a job (3009's rule: units, never time).
- Making any search smarter or cheaper.

**Estimated size:** about 1,700 lines (code 800, tests 650, docs 250).

**Split:** past 1,000 lines, split off the owned forms of the searches
(the step "the engine's searches become jobs") into a new ticket; this
one then ships the pool with a test job of its own and 2112's `Solver`.

## Implementation steps

1. Read 3009's completion notes: where `Think` and `think::Progress`
   live, and how `App` steps a thinking bot. Fit the mailbox to that
   path.
2. Write the ADR.
3. Add the mailbox in `shuyi-ui` and `ctx.jobs`, with its type-erased job
   trait, and unit tests for `start`, `take`, `cancel` and a wrong type.
4. Add the `Jobs` trait and `Platform::jobs` in `shuyi-platform`, and the
   routing in `App`: to the pool when there is one, stepped within the
   budget when there isn't, handed over in ticket order, panics resumed.
5. Add `jobs_done` to `FrameRecord`, note it while recording, and hand
   results over at the recorded frame in playback.
6. Add the pool in `shuyi-app` for native, with cancel and shutdown, and
   `None` on the web.
7. Give the rollout bot and the turn planner (and 2112's `Solver` if it
   needs it) their owned search forms.
8. Write the equivalence tests, and the book's passage on bots (the one
   3009 wrote, or `site/content/learn/book/rules-as-commands.md`): when a
   job runs on the pool, when on the main thread, and why its result is
   the same.

## Acceptance criteria

- [ ] On every fixture of the rollout bot (2125), the turn planner (2128)
      and the level solver (2112), a job run on the pool with 1, 2 and 7
      workers gives the same decision as `Bot::choose` on one thread, and
      the bot's state hash after it is the same.
- [ ] The same jobs stepped by `App` with no pool (the headless backend)
      give the same decisions.
- [ ] Results reach the mailbox in ticket order when they finish in
      another order (a test job that finishes later for a smaller ticket).
- [ ] A job that panics on a worker panics on the main thread at hand
      over, with the job's message.
- [ ] A cancelled job's result never reaches the mailbox, and dropping
      the backend with jobs running joins every worker (no thread left).
- [ ] A session recorded with the pool on (results handed over at
      frames the pool decided) plays back on the headless backend with
      the same hash every frame.
- [ ] A record made before this change reads and plays back the same.
- [ ] The web build of an example that starts a job runs it on the main
      thread, and `cargo tree` of `shuyi-app` for `wasm32-unknown-unknown`
      gains no dependency.
- [ ] `ctx.jobs` and `Platform::jobs` have doc comments with an example.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the mailbox (start, take, cancel, wrong type); the pool's order,
  cancel and shutdown; `jobs_done` read and written.
- Property: for random fixtures and worker counts 1 to 7, the pool, the
  main thread in random slices and one call give the same decision.
- Integration: a recorded session with jobs plays back the same; a job
  that panics; old session fixtures still replay.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
