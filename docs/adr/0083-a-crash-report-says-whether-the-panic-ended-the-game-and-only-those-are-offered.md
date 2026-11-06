# ADR-0083: A crash report says whether the panic ended the game, and only those are offered

- **Status:** Accepted; extends ADR-0079 §6
- **Date:** 2026-11-05
- **Related tickets:** 3095 (this), 3046 (ADR-0079)

## Context

ADR-0079's hook writes a report of every panic not run in `crash::quiet`,
on any thread, and `App::load` offers the newest report to the game once
(`Setup::last_crash`, §6). CI's `render` job showed where that misleads: on
a Linux machine with no sound card, quad-snd 0.2.8's own audio threads
panic ("Can't open PCM device") as the game starts. The game runs on,
silent, but each start writes reports, and the next start tells the game
it crashed, so a game that shows a notice shows a false one every time.

Such a report is still worth keeping: it explains why the game had no
sound. What is wrong is offering it as a crash.

## Decision

1. `shuyi_platform::crash::CrashReport` gains `fatal: bool`: whether the
   panic ended the game. A report's text without it, older than it, reads
   as `true` (`#[serde(default = …)]`), since such a report was offered as
   a crash. `CrashReport::new` sets it `true`.
2. In `shuyi-app`, `crash::install` remembers the thread it is called on:
   `run` calls it from the thread that runs the game. The hook writes
   `fatal: true` for a panic on that thread, and `false` for one on any
   other. On that thread, a panic not in `crash::quiet` ends the game: on
   native, `run` catches the frame's and exits with 101, and one outside
   that catch ends the process itself; on the web every panic aborts. So
   the thread is the whole test, and `crash::add_record`'s rewrite needs
   no flag of its own.
3. A panic the engine catches on purpose on the game's thread runs in
   `crash::quiet`, as before; the sound effects' decode in `audio.rs`,
   which caught quad-snd's panic without it, now does too, so a broken
   effect is neither a crash nor a report.
4. `offer_last_crash` offers the newest fatal report newer than the one
   `crash-offered` names, skipping reports that aren't fatal or don't read,
   and then names the newest report there, offered or not. Each report is
   offered or skipped once.

## Consequences

- A game on a machine without sound hears of no crash it didn't have; the
  reports of quad-snd's threads stay in its data folder, with their
  `thread`, for a developer to read.
- A panic on another thread that does end the game (none today: a worker
  that panics leaves the game running) would not be offered. A backend
  that adds such a thread marks its report itself.
- A game that catches a panic on the game's thread itself, without
  `crash::quiet`, gets a fatal report that isn't true; the crash docs say
  to use `quiet`.
- The offer reads every report newer than the last offered, at most the
  five kept: no cost to speak of.

## Alternatives considered

- **Mark the report fatal in `add_record`'s rewrite only** — the ticket's
  first idea; but a panic on the game's thread outside `run`'s catch (in
  the load) never reaches it, and on the web nothing does. The thread
  covers all three cases with one rule.
- **Write no report for a panic on another thread** — loses why the game
  had no sound.
- **Offer every report, and let the game check `thread`** — what the docs
  asked before; every game then repeats the same check, and the offer's
  "once" covers the wrong report.
