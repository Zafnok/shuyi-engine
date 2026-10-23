---
id: "3054"
title: "Parked: Leaderboards in the store services: local high-score tables, a high-score screen, and submissions that carry their run's record for a replay to check"
type: feature
milestone: "Engine: store services"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["3036"]
nick_input: none
completed:
---

# 3054 — Leaderboards in the store services

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need): the game doesn't ask for it, so this waits until after the MVP (`docs/ROADMAP.md`), and after 3036, whose shape it follows. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. What they have:

- **Steam** gives leaderboards for free: a board is found or made by
  name, a score is uploaded with "keep best", and entries are read for
  the whole world, around the player or among friends
  (https://partner.steamgames.com/doc/features/leaderboards).
- **Unreal** has a leaderboards interface in its Online Subsystem, and
  Epic Online Services' leaderboards are free.
- **Unity** has Unity Gaming Services Leaderboards, where server code
  can check a score before it counts.
- **Godot and Bevy** ship none. GodotSteam and `bevy_steamworks` reach
  Steam's boards through plugins, and a local high-score table is game
  code over the saves.

What Shuyi has (checked 2026-10-20):

- No score, high score or leaderboard anywhere in `crates/`,
  `templates/` or `examples/`.
- The pieces a table needs:
  - `Storage` (`crates/ui/src/storage.rs`): small named text blobs,
    keys `^[a-z0-9_-]{1,64}$` (`is_valid_key`);
  - profiles (`crates/ui/src/profiles.rs`): `ProfileStore` reads and
    writes the profile in use's keys, and `ProfileStore::computer_mut`
    reaches what stays with the computer. Each `Profile` has a `name`;
  - the platform hands the storage over once (`Platform::storage`,
    `crates/platform/src/platform.rs:209`), and `Setup::ctx`
    (`crates/platform/src/app.rs:140`) puts it in the context with
    `Ctx::with_storage` (`crates/ui/src/ctx.rs:769`);
  - a game's record re-run by `shuyi replay`
    (`crates/cli/src/contract/replay.rs`). What a record is belongs to
    the game: for hello-grid, `StageRecord` (its seed and commands,
    `examples/hello-grid/src/tools.rs`), read by `GameTools::replay`
    (`crates/cli/src/game.rs:117`).
- What the tickets planned before this one, and the hole in it:
  - 3036 (parked) builds achievements in the shape this ticket copies: a
    trait the platform hands in as it hands in `Storage`, calls on
    `Ctx`, a local backend kept through `Storage`, a memory backend for
    tests, replays and bots, an unlock notice and an engine screen. It
    leaves leaderboards out and sends them to 1148.
  - 3037 (Steam achievements) sends them to 1144 and 1148, and 1144
    (Steam networking) sends them to 1148.
  - 1148 builds leaderboards only on our own Postgres server, "read from
    the same verified results". It has no client trait, no local table
    and no Steam boards.
  - ADR-0006 §1 lists leaderboards among the services, "Steam where it
    fits". So nobody owns Steam's boards or a local table.
  - 1146's result record (seed, commands, outcome, hash), which a
    checker re-simulates with `shuyi replay`, is the idea a checked
    score reuses.

Why it matters:

- **The game first.** Visions of Shuyi doesn't ask for leaderboards, so
  this is parked with the other store services.
- **Other 2D games** expect a high-score table: arcade games, puzzles
  scored by fewest moves, time trials and daily runs. One that ships on
  Steam expects Steam's boards, which had no ticket before this one.
- **The end goal** (Nick, 2026-10-19, `docs/decisions.md`) names snake
  and Mario, games that usually keep a high score. They wait for real
  time (2026-10-20), but a turn-based puzzle can use this before them.
- **Determinism's edge.** A submission can carry the record of the run
  that made it, so a checker re-simulates the run instead of trusting
  the number. The check itself is 1148's, on its server; this ticket
  makes sure every backend receives the record.

## Nick input

None.

## Scope

**In:**
- **`LeaderboardsDef`**, an asset kind read from RON (ADR-0007,
  ADR-0019), in `shuyi_assets::leaderboards`
  (`crates/assets/src/leaderboards.rs`), built like
  `crates/assets/src/palette.rs` (`PaletteRon`, `from_source`,
  `line_of_key`):
  - `boards: Vec<BoardDef>`; each `BoardDef` has `id` (a `BoardId`),
    `name` (a text key), `best` (`Highest` or `Lowest`), `shown`
    (`Number`, `Seconds` or `Milliseconds`, Steam's three display
    types) and `kept` (how many entries a local table keeps, 1 to 100,
    10 if left out);
  - scores are `i32`, as Steam's are;
  - the format `LeaderboardsRon`: id `leaderboards`, extension `ron`,
    no magic bytes, the 4 MiB text limit; a row in `docs/formats.md`'s
    built-in table;
  - checks with file and line (`line_of_key`): a duplicate id; an id
    whose storage key `board-<id>` fails `is_valid_key`; `kept` of 0 or
    above 100;
  - a `name` key the game's English lacks. A format can't see the
    language, so this is a function of its own,
    `LeaderboardsDef::missing_text(&self, file, source, lang: &Lang) ->
    Vec<ContentError>`, over `Lang::has` (`crates/assets/src/lang.rs:201`),
    as `palette::theme_problems` is a check of its own. The toy's tests
    call it, and a game calls it from its `GameTools::check`;
  - a property test that random bytes never panic it. If 2020 is done,
    add the format to its fuzz targets.
- **The service**, in `shuyi_ui::leaderboards`
  (`crates/ui/src/leaderboards.rs`). It builds without `std` (3005):
  `use crate::prelude::*` and `alloc`, as `storage.rs` does. Its types
  follow ADR-0037 on `#[non_exhaustive]`:
  - `Submission { board: BoardId, score: i32, record: Option<String>,
    by: Player }`. `record` is the run's record as the game's
    `GameTools::replay` reads it, stamped as 2111 stamps records if 2111
    is done. `Player { profile: ProfileId, name: String }` is filled in
    by `Ctx`; a store backend uses the store's signed-in player instead;
  - `Range::Top { count: u16 }` and `Range::AroundMe { before: u16,
    after: u16 }`;
  - `Entry { rank: u32, name: String, score: i32, mine: bool }`;
  - `BoardEvent`:
    - `Submitted { board, rank: Option<u32>, new_best: bool }`: `rank`
      is the score's place, `None` if it didn't make the table;
      `new_best` is true when it beats this player's own best on the
      board;
    - `Scores { board, range, entries: Vec<Entry>, total: u32 }`;
    - `Failed { board, why: BoardError }`, with `BoardError::Storage`,
      `BoardError::Offline` and `BoardError::Backend(String)`;
  - `trait Leaderboards: fmt::Debug`:
    - `fn available(&self) -> bool`: whether the backend takes calls
      now, as 3036's ADR asks of every store service. Both
      backends here always say true; a store that isn't running says
      false, and the screen then shows its `failed` line;
    - `fn submit(&mut self, board: &BoardDef, submission: Submission,
      storage: &mut dyn Storage)`;
    - `fn request(&mut self, board: &BoardDef, range: Range, me:
      ProfileId, storage: &dyn Storage)`;
    - `fn poll(&mut self, out: &mut Vec<BoardEvent>)`.

    A backend answers on a later `poll`, never inside `submit` or
    `request`, because Steam's calls answer later. `storage` is the
    computer's (`ProfileStore::computer_mut`). The local backend keeps
    its tables there; a store backend may keep a queue of unsent scores
    there. If 3036 settled another way for `LocalAchievements` to reach
    the storage, do the same here.
  - `fn show_score(shown: Shown, score: i32) -> String`: digits for
    `Number`, `m:ss` for `Seconds` (`h:mm:ss` from an hour), `m:ss.mmm`
    for `Milliseconds`, a leading `-` for a negative score. Digits stay
    plain in every language: 2348 leaves numbers by language to a
    follow-up of its own (its Out).
- **The backends:**
  - `LocalLeaderboards`, for builds with no store. Each board is one
    computer key, `board-<id>`, so the table is the computer's: the
    people who share it (2310's profiles) see each other's scores, as on
    an arcade machine. Each entry keeps the profile and the name it was
    made with. The file holds the kept entries, each profile's best, and
    a counter that numbers submissions:
    - entries are ordered by score (by `best`), then by that number, so
      the earlier of two equal scores stays above. No clock is read;
    - only the top `kept` entries stay; a score below them updates the
      profile's best and is answered with `rank: None`;
    - `AroundMe` answers the window of the kept table around the
      player's best kept entry, or the top of the table if none is kept;
    - a removed profile's entries stay under the name they were made
      with, as on an arcade machine (profile ids are never reused,
      `Profiles::add`);
    - the file implements `SaveFormat` (`"Leaderboard"`, 1;
      `crates/core/src/save.rs`), and 2110's `Migrate` with a golden
      fixture if 2110 is done;
    - an unreadable file answers `Failed` and is never written over;
    - records are not kept.
  - `MemoryLeaderboards`, for tests, the Harness, replays, bots and
    `frame-png`: the same tables in memory, never the player's storage,
    and `submitted() -> &[Submission]`, records included, for tests.
- **The context and the platform:**
  - `GameData::leaderboards(&self) -> Option<&LeaderboardsDef>`,
    defaulted to `None`, beside wherever 3036 put the achievements'
    definition;
  - `Ctx` holds a `Box<dyn Leaderboards>`: `MemoryLeaderboards` from
    `Ctx::new`, swapped with `Ctx::with_leaderboards`;
  - `Ctx::submit_score(board, score)` and `Ctx::submit_score_with(board,
    score, record)`. An unknown board id fails a `debug_assert!` in debug
    builds and is a warning in release (`Ctx`'s warnings,
    `Ctx::take_warnings`, or 3040's `shuyi_ui::log` if it is done). A
    submission is an output, like a sound cue: the rules never read a
    board;
  - `Ctx::request_scores(board, range)`;
  - `Game::step` (`crates/ui/src/game.rs:293`) polls the service before
    the screens update, and the frame's answers are in
    `Ctx::board_events() -> &[BoardEvent]` for that frame only;
  - **handed in as 3036 hands in achievements.** If 3036 made the
    `StoreServices` set that `Platform::store_services()` hands in
    (every member `None` by default), add
    `leaderboards: Option<Box<dyn Leaderboards>>` to it. If it handed
    achievements in another way, do the same. Either way:
    - `Setup::ctx` (`crates/platform/src/app.rs:140`) puts in the
      platform's backend, or `LocalLeaderboards` when there is none;
    - the headless backend (`impl Platform for HeadlessPlatform`,
      `crates/platform/src/headless.rs:332`), which the Harness and
      `frame-png` run on, hands in `MemoryLeaderboards`, so no run of
      theirs writes a `board-` key;
    - the macroquad backend (`crates/app/src/backend.rs`) needs no change
      until a store backend exists.
  - The API only grows (defaulted methods, a new field in a
    `#[non_exhaustive]` struct), so the game's next pin bump breaks
    nothing.
- **The high-score screen**, `shuyi_ui::screens::high_scores`, an engine
  screen like the profiles screen (`crates/ui/src/screens/profiles.rs`):
  - `HighScoresScreen::new(ctx, board)` asks for the top `kept` entries
    when it opens. Left and Right switch between the top and around the
    player; Cancel leaves. It declares those actions (`Screen::actions`,
    ADR-0025);
  - it shows the board's name, then rank, name and score
    (`show_score`) per entry, the player's own entries marked; "loading"
    until the answer comes, "no scores yet" for an empty board, and a
    line saying the scores can't be shown on `Failed`;
  - **the look is a skin** (`AGENTS.md` rule 9): `HighScoresView` is
    plain data, `glyph::paint` draws it, and `painted_by` lets a game
    give its own painter;
  - its help line names keys through `Ctx::text_with`'s key placeholders
    (`AGENTS.md` rule 8);
  - text keys under `high_scores.`: `top`, `around_me`, `loading`,
    `empty`, `failed`, `new_best` (with `{board}`) and `help`. **Either
    order with 2351:** if 2351 is done, their English goes in
    `crates/ui/lang/en/ui.ron`; if not, in the toy's
    `crates/ui/src/toy/en.ron`, and 2351 moves them with the rest;
  - it describes itself for screen readers if 2322 is done.
- **"New best":** on `Submitted { new_best: true }`, `Game` shows 3036's
  unlock notice with `high_scores.new_best` and the board's name. The
  game turns it on with a switch of its own (`Ctx::new_best_notice`, a
  `bool`, off by default), apart from the achievements notice: a Steam
  build turns that one off because Steam's overlay shows achievements,
  but Steam shows nothing for a new best.
- **The toy game** (`crates/ui/src/toy.rs`) has one board (highest
  first, `Number`, 5 kept). Its `Lobby` (line 706) gets a "High scores"
  item that opens the screen, and its tests submit through `Ctx`.
- **Either order with 3037 (Steam):** whichever of 3037 and this ticket
  is done second adds `SteamLeaderboards`.
  - If 3037 is done and has no `SteamLeaderboards`, add it here, in
    `shuyi-app` behind 3037's `steam` feature, over 3037's fake of the
    Steam calls:
    - each board is found or made once (`find_or_create_leaderboard`,
      its sort from `best` and its display from `shown`); calls wait in a
      queue until Steam answers, as 3037's achievements do;
    - `submit` uploads with "keep best"; Steam's answer becomes
      `Submitted` (`new_best` from whether the score changed, `rank` from
      the new global rank);
    - `Top` and `AroundMe` read the global entries and those around the
      player. Add a `Range::Friends` then, read from the player's
      friends; the local backend answers it with `Failed`
      (`BoardError::Backend`);
    - without Steam running, the build keeps `LocalLeaderboards` and
      logs why, as 3037's fallback does;
    - check the method names in the `steamworks` version 3037 pinned.
    This part is about 150 lines. If it takes the PR well past 600
    lines, split it first into a parked ticket of its own, blocked by
    this one (the `write-ticket` skill).
  - If 3037 is not done, leave Steam out: 3037 adds `SteamLeaderboards`
    (its own either-order clause with this ticket).
- **An ADR** (the `write-adr` skill), with its row in
  `docs/adr/README.md`: the leaderboards kind and its format, local
  tables that belong to the computer, ties broken by order of arrival
  with no clock read, and the record every submission carries for a
  server to check later. If 3036's ADR on store services already says
  how a service answers (requests through `Ctx`, answers as events on a
  later frame, results that reach screens and views, never the rules),
  this ADR refers to it and doesn't restate it; an accepted ADR is never
  rewritten (`write-adr`, step 5). If 3036 wrote none, this ADR says it.

**Out (do not do):**
- checking a submission's record by re-simulating it: 1148's server
  does it (its Leaderboards bullet), with `History`
  (`crates/core/src/history.rs`) for turn games; the record of a tick
  game waits for 1119's `TickRecord`. No `shuyi replay --verify-score`
  here: offline a player can edit the local save anyway;
- our own server's boards, and the client backend over them under this
  trait (1148);
- online tables over HTTP, such as a web service's high scores: they
  need 2601 (HTTP requests as a platform capability), and become a
  backend under this trait that sends through it, never a client of
  their own;
- typing a name for each entry, as arcade machines ask for initials:
  entries take the profile's name, and a store can't say whether a
  score makes the table until it answers. A follow-up if a game asks;
- dates on entries;
- attaching the record to a Steam entry as a shared file, and watching a
  kept run: follow-ups when a game wants to show other players' runs;
- other stores (GOG Galaxy, Epic, consoles): a ticket each under this
  trait, when a game ships there and Nick has said yes to its SDK
  (ADR-0006 §5 allows Steam's alone);
- anti-cheat beyond re-simulating records;
- anything in Visions of Shuyi: nothing turns on by itself.

## Implementation steps

1. Read 3036's code and ADR (its trait, how the platform hands it in,
   how its backends reach the storage, its notice and its toy changes),
   `storage.rs`, `profiles.rs` (`ProfileStore`), `Setup` and `App::load`
   in `crates/platform/src/app.rs`, `HeadlessPlatform` in
   `crates/platform/src/headless.rs`, `palette.rs` for the format, and
   the profiles screen for the split between view and skin.
2. Write the ADR and its row in `docs/adr/README.md`.
3. Write `LeaderboardsDef`, `LeaderboardsRon`, the checks and
   `missing_text`, with the property test and bad fixtures; add the row
   to `docs/formats.md`.
4. Write the service types, `show_score`, `LocalLeaderboards` and
   `MemoryLeaderboards`, with unit and property tests.
5. Add `GameData::leaderboards`, the `Ctx` calls, the poll in
   `Game::step`, the `leaderboards` member beside 3036's achievements,
   `Setup::ctx`'s fallback and the headless backend's
   `MemoryLeaderboards`. Add the toy's board and a toy test that submits
   through `Ctx`.
6. Count the changed lines so far, fixtures and snapshots aside. If the
   screen and the notice would take the PR past about 600, stop here:
   move steps 7 and 8 to a new parked ticket blocked by this one (the
   `write-ticket` skill), and say so in the Completion notes.
7. Write the high-score screen, its view and its glyph painter, with
   snapshot tests; wire the "new best" notice.
8. Add the lobby item that opens the screen.
9. If 3037 is done, add `SteamLeaderboards` as the either-order clause
   says, or split it out as that clause says.
10. Run the gates.

## Acceptance criteria

- [ ] Of two equal scores, the one submitted first ranks higher, on
      every run.
- [ ] A local table keeps only the top `kept` entries; a score below
      them is answered with `rank: None` and still updates the
      profile's best.
- [ ] `new_best` is true only when a score beats the same player's
      earlier best.
- [ ] No answer arrives inside `submit` or `request`; each comes on a
      later `poll`, and `Ctx::board_events` holds it for one frame.
- [ ] `LocalLeaderboards` keeps two profiles' scores in one table across
      a restart (through `MemoryStorage` in a test), each entry named by
      the profile that made it.
- [ ] An unreadable table answers `Failed` and its file is unchanged.
- [ ] A Harness run of the toy that submits scores leaves the player's
      storage without any `board-` key, and `MemoryLeaderboards`
      receives each submission's record unchanged.
- [ ] `show_score` gives the expected text for each `Shown`, negatives
      and an hour or more included.
- [ ] The headless backend hands in `MemoryLeaderboards`, and a build
      with no store backend gets `LocalLeaderboards` from `Setup::ctx`.
- [ ] Unless step 6 moved them: the high-score screen's snapshots (top,
      around the player, loading, empty, failed) are committed and
      reviewed with `frame-png`, and the toy's lobby opens the screen.
- [ ] Random bytes never panic `LeaderboardsRon`; bad fixtures give
      errors with file and line, and `missing_text` names a board whose
      `name` key the English lacks, with its line.
- [ ] If `SteamLeaderboards` is written here: with 3037's fake, a score
      submitted before Steam found the board is uploaded once it does,
      and the gates pass with and without the `steam` feature.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: ordering and ties; trimming to `kept`; `new_best`; `AroundMe`
  with and without a kept entry; a removed profile's entries; an
  unknown board id; an unreadable table; `show_score`; answers only on
  `poll`; each check of the format and `missing_text`.
- Property: any order of submissions gives a table sorted by score then
  arrival, no longer than `kept`, holding the best `kept` of them;
  random bytes never panic the format.
- Snapshot / integration: a Harness run of the toy that submits and
  reads the answers; unless step 6 moved them, the screen's five
  states, the toy's lobby opening it and the "new best" notice.

## Completion notes

*(Filled in by the session that completes the ticket.)*
