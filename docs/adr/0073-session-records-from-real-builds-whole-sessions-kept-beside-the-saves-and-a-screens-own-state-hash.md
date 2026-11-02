# ADR-0073: Session records from real builds: whole sessions kept beside the saves, and a screen's own state hash

- **Status:** Accepted
- **Date:** 2026-11-02
- **Related tickets:** 2519 (this), 2518 and 2575 (the record and the Harness's
  part), 2357 (rewind), 3060 (a crash report naming its session), 3047 (parked:
  sending a report)
- **Extends:** ADR-0072 (session records), ADR-0036 (the short path), ADR-0022
  (the contract), ADR-0051 (stamps)

## Context

ADR-0072 made a record, and only tests made or played one. A playtester's
softlock in a debug build was lost when they quit. A record can't be cut to its
last minutes: it must start at the first frame, since nothing stores the state a
later stretch starts from.

A playback needs the game's real `Start`, and it needs the `Start` the build
that recorded used: a different stamp refuses the record, and a different state
hash makes every frame differ. Hello, Grid's `Start` lived in its binary. A game
on the short path has no `Start` of its own to change: `run_game` calls
`BasicGame::start`, and the base template's `lib.rs` must stay small for the
blank game (ADR-0036).

## Decision

1. **Builds with debug tools record, and no others.** `shuyi-app` turns recording
   on before the first frame when `DEBUG_TOOLS` is on. A switch, and anything in
   a shipped build, is out: a record holds the player's input and saves, a privacy
   question that is Nick's (parked 3047).
2. **The ring is of whole sessions.** A desktop build writes
   `<unix ms of its start>.session.ron` to `sessions/` under the game's data
   folder when the game quits, and when the debug menu asks, through a temporary
   file and a rename; the newest ten stay. The name's time is the system's: the
   loop's own clock (`pacing::seconds`) starts near 0 each launch. The folder is
   beside the saves, never in `Storage`, so a record doesn't land in the next
   record's store.
3. **The web saves when asked, as a download**, through a miniquad plugin of our
   own (`download.js`): it has no files and never quits.
4. **"Save session" is the engine's entry on the debug menu**, shown while `App`
   records or plays a record (`Ctx::session_recording`). The same menu shows in a
   session and its playback, so the value needs no field in the record, and asking
   (`Ctx::ask_session_save`) changes nothing the frame's hash reads.
5. **A screen can hash its own state**: `Screen::state_hash`, defaulted to
   `None`. `BasicGame::start` hands `Game::state_hash`, the top-most screen's that
   has one, to `Start::state_hash`. So a stage on the short path gets its rules
   state in each frame's hash from a method on its screen, with no change to the
   base template's files. A game with its own `Start` may still pass any function.
6. **`BasicGame::made`'s word is its id and version.** That is what the base
   template's `made()` already writes, so a record from `run_game` and the tools'
   `GameTools::made` agree, and a new version's records are refused as another
   build's.
7. **The tools check the stamp first, by `GameTools::made`**, so a refusal has
   `replay`'s code, `SH3007`. `session_from` answers text, which can't carry a
   code; `Harness::from_session` still checks, for a game whose `Start` is stamped
   and whose tools give no `made`.
8. **A record that plays differently stands on that frame.** `run` and `scene`
   answer `ok: false` with the result, and the script and `frames` don't play, as
   `replay`'s differing runs are a verdict. MCP's `start` answers the same but
   keeps the game: looking at that frame is why an agent started it.

## Consequences

- Hello, Grid's `Start` is `hello_grid::start`, in its library, and a game that
  wants `run --session` moves its `Start` out of its binary the same way.
- A session of a Hello, Grid golden file changes when its files change, so a
  change to them is blessed (`cargo xtask golden --bless`).
- `Headless` no longer assumes the game has loaded: a record stopped before the
  load, or a load that failed, answers with no game and a limit that says so.
- A frame's hash reads what happened, the screens and the rules state. A cursor
  moved on a screen that holds no state in its hash isn't a difference until it
  shows as a sound, a screen or a rule.

## Alternatives considered

- **A ring of the last minutes of a session**: can't be played back (see Context).
- **Builder methods on `BasicGame` for the stamp and the hash**: each changes the
  base template's `lib.rs`, a blank game's file, for a part only a stage has.
- **A prefix on the text `session_from` answers, to find a stamp's refusal**: a
  second parsed string where `GameTools::made` already says what is needed.
