# ADR-0030: The log: plain lines with a level, a category and a frame, kept per App, output only, and handed to the backend's sinks

- **Status:** Accepted
- **Date:** 2026-10-22
- **Related tickets:** 3040 (this), 3041 (the shipped build's sinks), 1133
  (the MCP server), 2325 (timing spans, parked), 3010 (the Playdate
  backend)
- **Extends:** ADR-0022 (the contract's answers), ADR-0024 (the platform
  boundary)

## Context

- Bevy sends every message through `tracing` with a level and a filter
  per module; Godot writes `user://logs/` and lets a game add a
  `Logger`; Unity writes `Player.log`; Unreal has log categories with a
  verbosity each. Each has one log with levels, most with categories.
- The engine had strings with no level: `Ctx` gathered what went wrong
  reading saved data in a `Vec<String>`, `App` copied it once after the
  game's load, and `shuyi-app` printed it with macroquad's `warn!`. A
  line given later, and every line in a Harness (which starts with
  `App::running` and never loads), reached nobody.
- ADR-0005 names logs among what agents need, and the contract's `run`
  and `scene` answers carried none: an agent saw that a run went wrong,
  not why.
- Tests run many Harnesses at once, one per test thread. A global log
  mixes their lines.

## Decision

1. **A log line is plain data**: `shuyi_ui::log::LogLine { frame, level,
   category, text }`. `LogLevel` is `Error`, `Warn`, `Info`, `Debug`,
   ordered from the most serious; `LogCategory` is `Assets`, `Audio`,
   `Input`, `Screens`, `Storage`, `Platform` and `Game` (a game's own
   lines). Both are `#[non_exhaustive]` and have a lowercase `name()` and
   `from_name`. The module is `no_std` + `alloc`.
2. **Kept per `App`, never in a global.** `Ctx` keeps the lines it is
   given (`Ctx::log`) in a `LogRing` until `App` takes them
   (`Ctx::take_log`). `App` counts its frames (`App::frame_number`: 0
   before the first, one more per `frame` or `frame_after`) and, at the
   end of every frame (a suspended one too) and once in `App::running`,
   takes the game's waiting lines and stamps them with that number. A
   line given between two frames carries the next. `App` keeps the last
   `LOG_LINES` (500) in its own `LogRing`, counting the dropped, and
   filters what enters it with a `LogFilter` (a level for every category,
   `Info` by default, and one per category over it).
3. **Who logs.** `shuyi-core` never logs, and `shuyi-assets` returns
   errors and never logs. `shuyi-ui` logs through `Ctx`, and
   `shuyi-platform` through `App` (a failed load: one `Error` line,
   category `Platform`).
4. **Output only.** No engine or game code reads the log to decide
   anything. Tests, tools and backends read it: `App::log` (every kept
   line), `App::take_new_log` (the lines since the last call, by the
   ring's sequence numbers), `Harness::log`, and the contract's
   `Session::log` and `log_dropped`.
5. **Not player text.** A line is English and never translated;
   `check-text` counts no literal inside a `log(…)` call.
6. **The contract answers with it.** `run`'s and `scene`'s answers carry
   `log`, a list of `LogEntry { frame, level, category, text }` with the
   level's and category's names. Lines dropped past 500 are a limit of
   the answer.
7. **A backend hands App's lines to its own sinks.** `shuyi-app` prints
   them with macroquad's macros for now; 3041 moves it to the `log` crate
   (MIT OR Apache-2.0, `no_std`, already in `Cargo.lock` through gilrs,
   and what macroquad's `log-rs` feature turns macroquad's own macros
   into), and adds a log file, the browser console and a filter read from
   text.

## Consequences

- Two Harnesses on two test threads never see each other's lines, and
  every line knows its frame, so an agent reads "frame 0: warn storage:
  settings: unreadable" beside the screens and events of the same run.
- A Harness's log is complete from frame 0: what the game's context
  logged before the Harness started is taken by `App::running`.
- The game keeps `Ctx::take_warnings` (its own front ends and tests call
  it): it takes every waiting line and returns the text of the `Warn`
  and `Error` ones. `App::take_warnings` is gone; a backend calls
  `App::take_new_log`.
- A line logged outside a game's `Ctx` (the backend's own: render, audio,
  pads) doesn't reach `App`'s ring. Those stay on the backend's sink
  until 3041.
- The contract's version rises, and every `run` and `scene` answer grows
  a `log` field.

## Alternatives considered

- **The `log` crate's global facade in the engine's crates** — one logger
  per process: two Harnesses on two test threads mix their lines, and a
  line knows no frame. It stays a backend's sink (§7).
- **`tracing`** — the same global problem, a larger dependency tree, and
  its spans are timing, which is 2325's job.
- **A global facade of our own** — dropped for §2: anything global mixes
  test threads.
- **Keep `Vec<String>` warnings and add a level** — no category to
  filter by, no frame, and still taken once after the load.
- **An unbounded log** — a game that logs every frame grows without end
  on a small device; 500 lines with a count of the dropped is enough to
  say why a run went wrong.
