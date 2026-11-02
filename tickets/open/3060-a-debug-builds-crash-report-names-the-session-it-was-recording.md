---
id: "3060"
title: "A debug build's crash report names the session it was recording, written at the panic"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: ["3046", "2519"]
nick_input: none
completed:
---

# 3060 — A debug build's crash report names its session

## Context

3046 and 2519 each promised this ticket in their **Out** lines: a
crash report from a debug build that names the session it was
recording. 2518's and 2520's **Out** lines point here too. The
2026-10-24 backlog audit found it unwritten.

What is there once the blockers land (2518 blocks 2519, so it is done
first too):

- **3046's crash reports.** `shuyi_platform::crash::CrashReport` is
  plain data, `#[non_exhaustive]`, at `version` 1. Every field but
  `version` and `message` is `#[serde(default)]`, so fields can be
  added and older reports still read. `CrashReport::parse` refuses
  text over 2 MiB. In `shuyi-app` (`crates/app/src/crash.rs`), the
  panic hook writes the report at `crash-<when>` through the game's
  storage, and keeps the report and its key in a static. On native,
  `run` calls `app.frame` inside `catch_unwind`. When it catches a
  panic, it stops the sound, takes the game's record
  (`App::crash_record`) inside `crash::quiet` and a second
  `catch_unwind`, adds it with `crash::add_record` (which writes the
  report again under the same key), and ends the process with exit
  code 101. Before each `app.frame`, `run` calls `crash::set_frame`
  with the frame about to run (`app.frame_number() + 1`). On the web a
  panic aborts, so nothing runs after the hook.
- **2518's session record** (`shuyi_platform::session`): what the
  platform handed the game, one `FrameRecord` per `App::frame` from
  the first frame, the store as it was handed over, and a hash per
  frame. `App::take_session` gives the record so far. A record plays
  back only from its first frame, and only as far as its frames are
  whole.
- **2519's recording.** With debug tools on (`shuyi_ui::DEBUG_TOOLS`:
  debug builds, or the `debug-tools` feature), `run` calls
  `App::record` before the first frame. The record is written to
  `<data dir>/sessions/<unix ms of its start>.session.ron` when the
  game quits and when the debug menu asks, through a temporary file
  and a rename. All but the newest ten (`KEEP_SESSIONS`) are then
  deleted. The pure parts are in `crates/app/src/sessions.rs`.
  `shuyi run --session <file>` and `scene --session` play a record
  headless and name the first frame that plays differently. MCP's
  `start` takes a session too, and `step` goes on from where it
  stopped.
- **2520** (open; either order, below) reads a report with
  `shuyi crash` and replays its rules record with `replay --crash`. A
  rules replay never runs screens, skins or the platform (its
  **Out**).

The gap: a caught panic ends the process with 101 before 2519's write
at quit. So a debug build loses the session that led to its panic.
That session is the one record that plays the whole game up to the
panic: menus, screens, skins, the clock and the music. A rules record
holds only the commands.

**Why the report names the file and doesn't hold it.** `parse` refuses
a report over 2 MiB, and an hour's session runs to 16 MB (2518). A
session also holds the whole store, saves and settings included, which
3046's ADR keeps out of reports.

**Why the file ends before the frame that panicked.** A panic cuts its
frame off part-way. The `FrameRecord` that `App` began for it lacks
what `App` notes after the panic's point (its hash at least), so it
can't play back the same. Playing that frame's input again would also
panic the game's tool program, and `run --session` doesn't catch
panics. So the file keeps the frames that finished, and it plays with
no difference to the frame before the panic. The report's message,
place, top screen and log say what the last frame did. MCP's `start`
from the session lets an agent try that frame again with `step`.

Why it matters:

- **The game first.** A playtest of the game's debug build panics. The
  report in the data folder names the session, which sits beside it
  under `sessions/`. An agent plays it with `shuyi run --session`,
  reads the scene the player last saw, fixes the bug, and keeps the
  file as a golden session (2519).
- **Every game** on `shuyi-app` gets it with no code of its own: the
  backend records and writes the session.
- **The end goal** (2026-10-19, `docs/decisions.md`): an agent goes
  from a crash to the state that led to it without asking the player
  anything.

## Nick input

None. Only builds with debug tools record a session (2519), so a
shipped build's report names none. Recording in shipped builds, and
sending a session with a report, hold the player's input and saves: a
privacy question for Nick (parked 3047 asks it for crash reports).

## Scope

**In:**
- **The report names the session** (`shuyi_platform::crash`):
  - `CrashReport` gains `session: Option<String>`,
    `#[serde(default)]`: the session file's name as 2519 names it
    (`1761300000000.session.ron`). Never a path: a path holds the
    player's account name. `version` stays 1. `CrashReport` is
    `#[non_exhaustive]`, so the field is additive;
  - `to_text` writes it as it writes the other optional fields (left
    out when `None`, if they are), and `parse` reads it. A report
    without it still reads, with `None`.
- **The session cut at the panic** (`crates/app/src/sessions.rs`,
  2519's pure parts):
  - `cut_at_panic(record: &mut SessionRecord, frame: u32)` keeps the
    record's first `frame - 1` frames. `frame` is the frame that
    panicked, numbered as `App::frame_number` numbers frames (from 1).
    Frame 1 keeps none, and a record already that short keeps all. The
    record's other fields stay as they are. `App` pushes a frame's
    `FrameRecord` as the frame ends (2518): say it in the doc comment;
  - `write_at_panic(dir: Option<&Path>, record: Option<SessionRecord>,
    frame: u32) -> Option<String>` cuts the record and writes it with
    2519's writer into `dir`, the data folder (the writer adds
    `sessions/`). So it gets the temporary file, the rename, the ring
    of ten and the `info` line naming the file. It gives the file's
    name. It gives `None` with no record (debug tools off), with no
    data folder (2519's `warn`), or when the write fails (a `warn`
    naming the error). If 2519's writer doesn't give the file's name
    back, make it.
- **`crash::add_session(name: String)`** (`crates/app/src/crash.rs`),
  like `add_record`: it sets `session` on the report the hook kept, and
  writes the report again under the same key. With no kept report (the
  hook's write failed), it does nothing. A failed write is dropped, and
  it never panics.
- **`run`'s catch path** (native) gains a step after 3046's
  `crash::add_record` and before the exit with 101:
  1. take the record with `app.take_session()`;
  2. call `write_at_panic` with `storage::data_dir(storage_name)` and
     the frame `run` last passed to `crash::set_frame`, inside
     `crash::quiet` and a `catch_unwind`. So a panic while writing
     writes no second report, and the process still ends with 101;
  3. pass a name it gives to `crash::add_session`.

  With debug tools off, `App` records nothing, `take_session` gives
  `None`, and nothing is written.
- **Either order with 2520** (`shuyi crash` and `replay --crash`): if
  2520 is done:
  - `CrashOutput` (`crates/cli/src/contract/crash.rs`) gains
    `session: Option<String>`, the report's, and `crash::text` shows
    it. `CrashOutput` is `#[non_exhaustive]` (ADR-0039, as 2520 makes
    it), so the field is additive;
  - when the report names a session, `replay --crash`'s refusal of a
    report with no record, and its rules-only line in `limits`, add
    "its session plays the whole game: `shuyi run --session <file>`";
  - the `crash` output schema is regenerated, `VERSION` goes up by one,
    and `crates/cli/schemas/versions.json` gets its line (the four
    steps in `docs/contract.md`, *The schemas*);
  - `docs/crash-reports.md`'s paragraph on `shuyi crash` names the
    field.

  If not, nothing to do (2520 adds them, its clause says so).
- **From 1134 (done)** (the skill pack): its `shuyi-debug-replay`
  skill (`skills/shuyi-debug-replay/SKILL.md`) gains a paragraph. A
  debug build's crash report names its session in `session`, a file in
  `<data dir>/sessions/`. `shuyi run --session <file>` plays the whole
  game to the frame before the panic, screens and skins included,
  where a rules replay (`shuyi replay`) runs only the rules. MCP's
  `start` from the session lets the agent `step` on from there.
  `check-skills` passes.
- **Docs** (the `document` skill):
  - `docs/crash-reports.md` (3046) gains a section, *A debug build's
    session*. A debug build's report names its session in `session`.
    The file is in the `sessions` folder beside the reports
    (`<data dir>/sessions/`), and it ends on the frame before the
    panic. Play it with `shuyi run --session <file>` after copying it
    into the game's project, or start MCP from it and `step` on. The
    ring of ten deletes it after ten more sessions, so keep a copy
    with the report. A session holds the player's saves, settings and
    every input, so it stays with the developer. Shipped builds write
    none, and a panic on another thread or on the web names none;
  - 2519's page on where debug builds keep their sessions
    (`docs/sessions.md`, or the book's testing chapter it moved to)
    gains a line: a panic writes the session so far, cut before the
    frame that panicked, and the crash report names it;
  - `docs/platform-backends.md`'s *Panics* section (3046) gains a
    line: a backend that records sessions writes the record so far
    after a caught panic, cut before the frame that panicked, and
    names it in the report's `session`.
- **No new ADR.** The report names the file and holds none of it, as
  3046's ADR keeps saves and settings out of reports. The record is
  2518's; the file and its ring are 2519's.
- **For the game:** `CrashReport` gains a field, additively. The
  game's debug builds now write their session at a panic and name it
  in the report. Release builds without debug tools don't change, and
  nothing in the game's code changes. If the part for 2520 is done
  here, the contract's version goes up. Say so in the PR; the game's
  PR that moves the pin follows.
- **Size:** about 400 changed lines. The field and its tests about 40;
  `cut_at_panic` and `write_at_panic` with their tests about 150;
  `add_session` and the hook test about 60; `run` about 20; the part
  for 2520 about 70 (schemas aside); the docs about 50.

**Out (do not do):**
- Recording in shipped builds or by players, and sending a session
  with a report: Nick's privacy question (parked 3047).
- The session's frames inside the report: a report names the file
  (above).
- Playing the frame that panicked, and catching a panic in
  `run --session`: the file ends before that frame. Write a ticket if
  agents need it.
- A panic on another thread (the music worker): `run` doesn't catch
  it, so the game goes on, 2519 writes the session at quit as always,
  and the report names none.
- The web: a panic aborts there, and its sessions are downloads the
  debug menu asks for (2519).
- `Setup::last_crash` and the game's notice: a player's notice doesn't
  need the session's name.
- Keeping a crash's session out of the ring of ten: write a ticket if
  developers lose them.
- The record's format and playback (2518); the writer, the ring and
  `run --session` (2519); the rules replay (2520).

## Implementation steps

1. Read 3046's `crates/app/src/crash.rs` (the hook, `add_record`, the
   catch path in `run`), `shuyi_platform::crash` and its ADR. Read
   2519's `crates/app/src/sessions.rs`, its writer and ring, and its
   write at quit in `run`. Read 2518's `App::record`,
   `App::take_session`, and where `App` pushes each `FrameRecord`.
   Check whether 2520 is done.
2. Add `CrashReport::session`, with its unit tests, and add the field
   to 3046's property test's strategy.
3. Write `cut_at_panic` and `write_at_panic`, with their tests (below).
   The toy test needs `shuyi-platform`'s `harness` feature and
   `shuyi-ui`'s `toy` feature in `shuyi-app`'s dev-dependencies: add
   them unless 2519 did.
4. Write `crash::add_session`, and extend 3046's
   `crates/app/tests/crash_hook.rs`.
5. Add the step to `run`'s catch path.
6. Do 2520's part if it applies, and the `shuyi-debug-replay`
   paragraph (from 1134).
7. On the machine at hand, run hello-grid's debug build with
   `SHUYI_PANIC_AT_FRAME=30` (3046). Check that the report names a
   file in `<data dir>/sessions/`. Copy it into `examples/hello-grid/`
   (not committed) and play it with `shuyi run --session <file>`: 29
   frames of 29, no difference. Run the release build the same way:
   its report has no `session`. Put both in the PR description.
8. Write the docs, and run the gates.

## Acceptance criteria

- [ ] `CrashReport::parse` reads a report written without `session`
      (3046's text) with `session: None`, and a report with `session`
      round-trips through `to_text`.
- [ ] `cut_at_panic` keeps the first `frame - 1` frames; frame 1 keeps
      none; a shorter record keeps all.
- [ ] A toy session recorded on `App` whose frame 12 panics (caught
      with `catch_unwind`), passed to `write_at_panic` with a temporary
      folder, gives a name. The file is under `sessions/`, reads with
      `SessionRecord::from_ron`, and `Harness::from_session` then
      `play_all` answers 11 frames of 11 with no `first_difference`.
- [ ] `write_at_panic` with no record, or with no folder, writes
      nothing and gives `None`.
- [ ] In `crash_hook.rs`: after a caught panic, the stored report has
      `session: None`. After `crash::add_session(name)`, one report
      under the same key holds `name`, and the `record` that
      `add_record` added is still there.
- [ ] The PR description has step 7's two runs: the debug build's
      report names a session that plays 29 frames with no difference,
      and the release build's report names none.
- [ ] With 2520's part done here: `shuyi crash` on a report with
      `session` answers it and `crash::text` shows it; `replay
      --crash` on a report with a session and no record names the
      session in its reason; the schemas, `VERSION` and
      `versions.json` agree (the schema test passes).
- [ ] The `shuyi-debug-replay` skill has the paragraph, and
      `check-skills` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-platform`): `session` read, written and left out; an
  old report without it.
- Property (`shuyi-platform`): 3046's round trip, with `session` in its
  strategy.
- Unit (`shuyi-app`, `sessions.rs`): `cut_at_panic`'s three cases;
  `write_at_panic` with no record and with no folder; the toy session
  that panics, written and played back. One way to make the toy panic
  at frame 12: a toy `Start` whose `Start::state_hash` hook (2518)
  panics while a thread-local flag the test sets is on, and gives
  `None` otherwise. The test drives `App::frame` itself with a
  `HeadlessPlatform`, sets the flag before the twelfth frame, and
  clears it before the playback.
- Integration (`shuyi-app`, `crates/app/tests/crash_hook.rs`):
  `add_session`, inside its one `#[test]`, since the hook is global.
- Integration (`shuyi-cli`, with 2520's part done here): `crash` and
  `replay --crash` on a report with a session, through the command
  line and the fake game.
- Manual: step 7.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
