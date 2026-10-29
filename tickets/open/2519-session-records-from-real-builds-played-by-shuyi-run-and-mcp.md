---
id: "2519"
title: "Session records from real builds: desktop debug builds keep their last sessions, a debug menu entry saves one (a download on the web), and shuyi run, scene and MCP play them back"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2518", "1133", "2511"]
nick_input: none
completed:
---

# 2519 — Session records from real builds, played by shuyi run and MCP

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Unreal's Replay System records a running game's session
and plays it back; Unity's Input System records input events and
replays them. Bevy's remote protocol (BRP) gives agents a running game,
but it records nothing of the session a person played.

2518 builds the session record (`shuyi_platform::session`): `App`
records every value the platform hands the game, and the Harness plays
a record back from the game's real `Start`, with a hash per frame. But
only tests make records there, and only Rust tests play them. What is
missing (read on 2026-10-21):

- **The real builds don't record.** `shuyi_app::run`
  (`crates/app/src/lib.rs:148-183`) drives `App::frame` with
  `Millis::from_secs_f64(miniquad::date::now())` (`lib.rs:164`) and
  keeps nothing.
- **A developer can't ask for a record** while playing. The debug menu
  (`DebugMenuScreen`, `crates/ui/src/debug.rs:111`) lists only the
  game's entries. Hello, Grid and the templates have no debug menu at
  all: their actions have no `Debug` (`examples/hello-grid/src/action.rs`)
  and they pass `GameHooks::default()` (`examples/hello-grid/src/lib.rs:316`).
- **The contract can't play one.** `run` and `scene` start a game from
  its start options and a script (`RunInput`,
  `crates/cli/src/contract/run.rs:23`; `GameTools::session`,
  `crates/cli/src/game.rs:110`). 1133's MCP `start` does the same.
- **A game's tools can't build its real `Start`.** Hello, Grid's start
  lives in its binary (`examples/hello-grid/src/main.rs:50` and `:62`,
  where the stages' luck is seeded from the clock), and so does the grid
  template's (`templates/grid/crates/game/src/main.rs:72` and `:84`).
  2511 gives the blank template `BasicGame::start()` in
  `shuyi-platform`, which its tools (`BasicTools`) can already call.

Why it matters:

- **The game first.** A playtester hits a softlock in a debug build of
  Visions of Shuyi, quits, and the session is already on disk. An agent
  runs `shuyi run --session <file>`, reads the scene where the session
  ends (or where this build first plays it differently), fixes the bug,
  and keeps the file as a test.
- **Every game** made with `shuyi new` gets the same, since the
  templates share their `Start` with their tools.
- **The end goal** (2026-10-19, `docs/decisions.md`): agents make and
  fix games. MCP's `start` from a record puts an agent where a player's
  session ended, to step on from there.

**What "a ring" means here.** A ring of the last minutes of one session
can't be played back: a record must start at the first frame, since
nothing stores the state where a later stretch begins (rewind, 2357,
also replays from the first frame). So the ring is of whole sessions:
the last ten files are kept, older ones deleted.

## Nick input

None. Only debug builds record (debug tools on). Recording in shipped
builds or by players, and sending a record as a bug report, are out of
scope: a record holds the player's input and saves, a privacy question
for Nick (parked 3047 asks it for crash reports).

## Scope

**In, part A: playing records** (2357 and 2116 build on it):

- **The starts move into the libraries:**
  - `hello_grid::start() -> Start<Hello>` in `src/lib.rs`, moved from
    `main.rs` with `look` and the card's title (a `TITLE` const the
    window uses too). `lib.rs` names nothing of `shuyi-app`, so 2116's
    build without the window still works. It sets 2518's builders:
    - `Start::made`: `hello_grid::made()` (2111's stamp: `PACK_STAMP`
      and the fingerprint of `Bundle::new(vec![layer()])`), so a
      session record and a `StageRecord` of one build carry one stamp;
    - `Start::state_hash`: the stage's rules state through
      `shuyi_core::state_hash`, found with `Game::screen`
      (`crates/ui/src/game.rs:426`); `None` on other screens.

    `main.rs` calls it.
  - The grid template does the same in its `lib.rs`, `main.rs` and
    `tools.rs`, its `Start::made` being its own `made()` (2111),
    formatted as CI's `templates` job checks them
    (`templates/` is outside the workspace's `cargo fmt`).
  - `BasicGame::start()` (2511) sets `Start::made` from its files'
    fingerprint, with its name as the game's word. So a game on the
    short path, the blank template among them, stamps its records with
    no code of its own, and the blank template's files don't change
    (2512's cap is untouched).
- **`shuyi run` and `scene` play a record:**
  - `RunInput` gains `session: Option<SessionInput>`, with
    `SessionInput { file: String, any_stamp: bool }`. The file is read
    from the project's root, as `replay`'s record is. `run::run` and
    `run::scene` take the root as `replay::replay` does (their calls are
    at `crates/cli/src/contract/cli.rs:197-198`).
  - The command line gains `--session <file>` and `--any-stamp`
    (`run::parse_args` and its usage line). `RunInput::check` refuses
    start options with a session: the session's start is the game's
    real one.
  - **From 2568 (done):** `run::played` and `Host::start` call
    `game::check_start` on the start options first: keep that call ahead
    of the record path (a session has no start options, so it passes).
  - `GameTools::session_from(&self, record: &SessionRecord, stamps:
    Stamps) -> Result<Box<dyn Session>, String>`, with 2518's `Stamps`,
    defaulted to `Err(unsupported("run --session and scene --session",
    "session_from"))`. Most simply it is 2518's
    `Harness::from_session(start, record, stamps)`, its error as text,
    in a `Headless` with the game's scene and events, as its `session`
    builds one. Hello, Grid's tools, the grid template's and 2511's
    `BasicTools` implement it.
  - The engine reads the file and parses it with
    `SessionRecord::from_ron` (an error names the file). It hands the
    record to the game's `session_from` and plays it to its end through
    a new `Session::play_record(&mut self) -> Option<Played>`, defaulted
    to `None` (a session started from options); `Headless` calls
    `Harness::play_all`. Then the script and `frames` play as today.
  - `RunOutput` and `SceneOutput` gain `session:
    Option<SessionPlayed>`: the file, the frames played and in the
    record, and the first differing frame with both hashes as hex text
    (`shuyi-core` has no `schemars`). 2518's `Played::limits` join the
    answer's `limits`.
  - A frame that differs gives `ok: false` with the result, as `replay`
    does when its runs disagree. The script and `frames` don't play
    then, so `frames`, `top_screen`, `frame` and `scene` are the
    differing frame's. A refused stamp is `Answer::failed` with 2111's
    `Mismatch` text.
  - The contract's `VERSION` (`crates/cli/src/contract.rs:52`) goes up,
    the schemas and `crates/cli/schemas/versions.json` are regenerated
    (ADR-0022 §5, `docs/contract.md`'s *The schemas*), and
    `docs/contract.md` gets the flags. A refused stamp answers the code
    2111 gives `replay`'s refused stamp, so one `Mismatch` has one code
    (2111 is done first: it blocks 2518, which blocks this ticket). A
    differing frame is a verdict: `ok: false`, no code, exit 1, as
    `replay`'s differing runs are (`docs/contract.md`, *Problem
    codes*).
- **MCP:** 1133's `start` takes `session` and `any_stamp` as `run`
  does, and `step`, `scene` and `frame_png` go on from the frame it
  stopped on. `Server::start` (`crates/cli/src/mcp/server.rs`) sends
  the host only `start` today: it forwards the new fields too, and the
  host's `Live` (`crates/cli/src/mcp/host.rs`) keeps them beside
  `start`, so `reset` starts the same record again, not a fresh game
  (2357's `seek` starts a session again the same way). Its scripted
  client test gains a start from Hello, Grid's golden session, and a
  `reset` of it that ends on the same frame.
- **A golden session for Hello, Grid only:**
  - In `shuyi_platform::harness` (with `std`):
    - `check_session(start, text: &str) -> Result<Played, String>`
      plays a record's text and fails naming the first differing frame,
      or both stamps (2111's `Mismatch` text);
    - `bless_session(start, text: &str) -> Result<String, String>`
      drives 2518's `Playback` over every frame, differences and all,
      with `App::record` on, and gives that new record's text: the same
      inputs, this build's stamp and hashes. It fails if the new
      record's inputs differ from the old ("this build reads the
      platform differently: record the session again");
    - `assert_session(start, path)` reads the file and calls
      `check_session`. With the environment variable `SHUYI_BLESS=1` it
      calls `bless_session` and writes the file back.

    Tests call the first two directly: setting an environment variable
    is `unsafe` in Rust 2024, and the workspace forbids `unsafe`.
  - `examples/hello-grid/tests/sessions/stage.session.ron`, recorded
    with `Harness::start_recording(hello_grid::start())` and a script
    (the title, the options screen, the stage, a few moves on it). A
    test marked `#[ignore = "writes the golden session"]` beside the
    check makes it, so it can be made again.
  - `examples/hello-grid/tests/it/session.rs` plays it with
    `assert_session`, the path built from `env!("CARGO_MANIFEST_DIR")`,
    in a test with `golden` in its name. The PR says how to bless it.
    - **Either order with 2116:** if it is done, the session test lives
      in Hello, Grid's `tests/golden.rs` target (not
      `tests/it/session.rs`), with `golden` in its name. CI's
      `determinism` job then plays the session on wasm32 (its runner
      maps the workspace, so `assert_session` opens the file), and
      `cargo xtask golden --bless`, which sets `SHUYI_BLESS=1`,
      blesses it with the tables. If not, 2116 moves it.
  - The templates get no session files (they stay small, and a new
    game's author doesn't bless files they didn't make); the docs say
    how to add one.

**In, part B: recording in the real builds:**

- **`shuyi-app` records with debug tools on** (`shuyi_ui::DEBUG_TOOLS`,
  `crates/ui/src/lib.rs:77`: debug builds, or the `debug-tools`
  feature), and never without:
  - `run` calls `App::record` before the first frame, and puts a clone
    of the voice manifest's text it read into the record with
    `App::session_mut` (the manifest reaches the game through
    `Start::before_load`, `lib.rs:160`, out of `App`'s sight);
  - a new `crates/app/src/sessions.rs` holds the pure parts (whether to
    record, the file name, which files to delete), tested without a
    window.
- **The desktop keeps its last sessions:**
  - in `<data dir>/sessions/` (`storage::data_dir(storage_name)`,
    `crates/app/src/storage/native.rs:47`), never through the game's
    `Storage`: a record must not land in the next record's store.
    `FileStorage::list` (`native.rs:98-117`) reads only `.ron` files at
    the top of the data folder, so the subfolder stays out of the
    game's keys; a test checks it. With no data folder, a `warn!` and
    nothing written;
  - a record is written as `<unix ms of its start>.session.ron` when the
    game quits (before `run` leaves its loop) and when the debug menu
    asks (below), through a temporary file and a rename, as
    `FileStorage::write` does. A later write of the same session
    replaces its file;
  - after each write, all but the newest `KEEP_SESSIONS` (10), by the
    start time in their names, are deleted;
  - a line names the file, at `info`. **Either order with 3041:** if
    it is done, this line and the `warn` with no data folder are
    `log::info!` and `log::warn!` with `target: "storage"` (3040's
    category), as its sink and its filter expect; if not, macroquad's
    `info!` and `warn!`, as the backend's other lines are, and 3041
    moves them to the `log` crate (its scope names this ticket).
- **"Save session" on the debug menu:**
  - `Ctx::session_recording: bool`, which `App` sets while it records
    and while it hashes frames for a playback (2518's
    `App::hash_frames`). So a session and its playback show the same
    menu, and 2518's rule (a value from the platform gets a field in the
    record) doesn't apply: the value is the same in both;
  - `DebugMenuScreen::new` adds an engine entry after the game's (beside
    2315's "Debug view" entry, if 2315 is done) while
    `ctx.session_recording` is true. Choosing it calls
    `Ctx::ask_session_save()` and keeps the menu open;
    `App::take_session_save() -> bool` hands the request to the backend
    after the frame, which writes the record so far and goes on
    recording;
  - no key: the Debug action already opens the menu (`AGENTS.md` rule
    8). The label is English, like every debug tool (`check-text` note
    on the line).
- **Hello, Grid gets a debug menu**, so the entry can be seen and the
  web build can save:
  - a `Debug` action in `src/action.rs`, its `ui()` being
    `UiAction::Debug`, neither required nor rebindable (as
    `shuyi_ui::debug::hint` expects), and the module's docs updated;
  - `files/keymap.ron` binds it to `F2` in the layout's keys (as
    `crates/assets/src/keymap/sample.ron` does), and lists
    `"Debug": []` in any other table that must list every action;
  - `hello_grid::game` passes `GameHooks` whose `debug` opens a
    `DebugMenuScreen` with no entries of its own, and whose
    `debug_screens` is `&["debug_menu"]`.
- **The web offers a download.** The web build has no files and allows
  no `wasm-bindgen` (`AGENTS.md`, *Environment*), so:
  - `crates/cli/web/download.js`, our own miniquad plugin
    `shuyi_download` with a `version`, in the style of `gamepad.js`:
    `shuyi_download(name_ptr, name_len, text_ptr, text_len)` reads the
    two strings from the wasm memory, makes a `Blob` and clicks a
    temporary link;
  - `crates/app/src/sessions/web.rs` declares it in an `unsafe extern
    "C"` block, under `#[allow(unsafe_code)]` as
    `crates/app/src/pads/web.rs:17` is, with a `PLUGIN_VERSION` and its
    `shuyi_download_crate_version()` export. `crates/app/Cargo.toml`'s
    comment on `unsafe_code` and `lib.rs`'s docs (line 31) name every
    plugin block (with 3043's `lifecycle.js` too, if it is done);
  - `cargo shuyi web`'s `SHELL` (`crates/cli/src/web.rs:25`) gains
    `download.js`, `index.html` loads it after `mq_js_bundle.js` and
    before the game, and `crates/cli/web/README.md` gets a section. The
    tests that pin `gamepad.js` (`web.rs`, from line 783) get the same
    for it: the functions it provides are those the app declares, and
    its version matches. `download.js` goes in 3039's notices table
    (`SHELL_NOTICES` in `web.rs`) as the engine's own, beside
    `gamepad.js`;
  - **Either order with 3052:** both change `index.html` and `SHELL`.
    If 3052 is done, `download.js` loads between the bundle and the
    page's last call, `if (shuyiShell.start(…)) load("%WASM%");`, and
    `shell.js` stays first; if not, nothing to do (3052 keeps
    `download.js` when it changes the page). Whichever lands second
    sets `SHELL`'s length;
  - the web saves only when the debug menu asks (it never quits, and a
    download per hidden tab would be noise).

**In, both parts:**

- **Docs:** where debug builds keep their sessions, how to save one
  from the debug menu, how to play one with `shuyi run --session`, and
  how to keep and bless a golden session. **From 2802 (done):** this
  goes in the book's chapter 8 (*Testing*,
  `site/content/learn/book/testing.md`), its code in `site/snippets/`. A line in `docs/platform-backends.md`'s *Session
  records* (2518) says what `shuyi-app` writes. **From 1134 (done):**
  its `shuyi-debug-replay` skill (`skills/shuyi-debug-replay/SKILL.md`)
  gains a section: reproduce a bug from a session record with `run
  --session`, read `first_difference`, then fix and keep the file as a
  golden session; `check-skills` passes.
- **For the game:** its debug builds start writing sessions to
  `<data dir>/sessions/`; `GameTools` gains a defaulted method; `Ctx`
  gains a field; `run::run` and `run::scene` take the project's root;
  `RunInput`, `RunOutput` and `SceneOutput` gain fields (a new contract
  version). To offer `session_from`, the game's tools need its `Start`,
  so it moves out of its binary as Hello, Grid's does. Say so in the
  PR; the game's PR that moves the pin follows.
- **Size.** About 950 changed lines, not counting the schemas, the
  golden session file and snapshots: the starts about 150, the contract
  about 230, MCP about 40, the golden session about 90, the desktop
  recording and its ring about 140, the debug menu entry and Hello,
  Grid's menu about 110, the web download about 120, the docs about 60.
  Part A and the desktop recording come first (steps 2 to 5). Then
  count (`git diff --stat main`, the same files aside). Past about 600
  lines, the debug menu entry, Hello, Grid's debug menu and the web
  download go to a follow-up ticket (the `write-ticket` skill, After
  MVP, blocked by this one) with their acceptance lines, and the
  completion notes name it. The lines of 3043 and 3052 that name this
  ticket's `download.js`, and its row in `SHELL_NOTICES`, then go to the
  follow-up.

**Out (do not do):**
- Recording in shipped builds or by players, and sending a record as a
  bug report: Nick's decision on privacy (parked 3047).
- A crash report naming the session a debug build was recording:
  3060 (blocked by this ticket and 3046). Rewind (2357) and the live
  link to a running build (2521, 2522) are their own.
- Stopping a session at a chosen frame (`--until`), `frame-png
  --session`, MCP's `seek`, and going back in a running game: rewind,
  2357, which builds on this ticket.
- Captures of a range of a session (`--every` and `--count` with
  `frame-png --session`): whichever of 2514 and 2357 lands second adds
  them (their clauses say so).
- Writing records on the Playdate, GBA or a console: their backends'
  follow-ups, if a playtest there needs one.
- A switch to turn recording off in a debug build: write a ticket if a
  developer asks.
- Golden session files, and a debug menu, in the templates.
- The record's format and playback themselves (2518).

## Implementation steps

1. Read 2518's ADR and `shuyi_platform::session`, `shuyi_app::run`,
   `storage/native.rs`, `pads/web.rs`, `gamepad.js` and its tests in
   `crates/cli/src/web.rs`, `debug.rs`, `contract/run.rs`,
   `contract/session.rs`, `contract/cli.rs`, `game.rs`, 2511's
   `BasicGame` and `BasicTools`, and 1133's server.
2. Move Hello, Grid's and the grid template's starts into their
   libraries, and stamp `BasicGame::start()`. Check the binaries, and
   the template job's gates on a game made from each template (as
   `.github/workflows/ci.yml`'s `templates` job runs them).
3. Add `--session` to `run` and `scene`, `GameTools::session_from`,
   `Session::play_record`, the schemas and the version line; then MCP's
   `start`.
4. Write `check_session`, `bless_session` and `assert_session`, record
   Hello, Grid's golden session, and write its test. Bless it with
   `SHUYI_BLESS=1 cargo test -p hello-grid golden`, or with `cargo xtask
   golden --bless` if 2116 is done.
5. Write `sessions.rs` and the desktop recording, writes and ring.
6. Count the diff (**Size**). If it fits, add `Ctx::session_recording`,
   the debug menu entry, `App::take_session_save` and Hello, Grid's
   debug menu; then `download.js`, `sessions/web.rs` and the shell's
   tests. If not, write the follow-up ticket.
7. Write the docs and the skill section, and run the gates. Check by hand: quit Hello, Grid's debug build on the desktop,
   and play the newest file with `shuyi run --session`. If step 6 kept
   them, also save one from the debug menu on the desktop, and in the
   web build (`cargo shuyi web`, a debug build). Say so in the
   completion notes.

## Acceptance criteria

- [ ] `sessions.rs`: the file name, keeping the newest ten, and a later
      write of one session replacing its file are tested on a temporary
      folder; a record written there is not among
      `FileStorage::list`'s keys.
- [ ] With debug tools off, `shuyi-app` never calls `App::record` (a
      test of the pure decision in `sessions.rs`).
- [ ] `shuyi run --session examples/hello-grid/tests/sessions/stage.session.ron`
      plays it with no difference and answers the stage's scene. A copy
      with one input removed answers `ok: false`, its
      `session.first_difference` names the frame, and its `scene` is
      that frame's.
- [ ] Start options with `--session` are refused; a file that isn't a
      record is refused naming the file; a record with another stamp is
      refused unless `--any-stamp`; a game whose tools lack
      `session_from` gets the default's message (the fake game).
- [ ] The contract's schemas and version line are regenerated, and the
      schema test passes.
- [ ] MCP's scripted client starts Hello, Grid from the golden session
      and reads the scene, and `reset` starts it from the record again
      (the same frame and scene).
- [ ] Hello, Grid's golden session test passes. `bless_session` on the
      golden text keeps every input and gives this build's stamp and
      hashes (a test on the text, with no file); bless runs through
      `SHUYI_BLESS=1`, or `cargo xtask golden --bless` once 2116 is
      done.
- [ ] A new game from each template plays a session: the grid
      template's `tests/it/main.rs` and `BasicTools`' tests in
      `shuyi-cli` record one with `Harness::start_recording` and play it
      through `session_from` with no difference, and CI's `templates`
      job passes.
- [ ] Unless **Size** moved them out:
  - the debug menu shows "Save session" only while `App` records or
    plays a record; choosing it makes `App::take_session_save` true
    once; and a recorded session that chooses it plays back with no
    difference (Harness tests on the toy, its debug hook opening a
    `DebugMenuScreen`);
  - Hello, Grid's Debug action opens its debug menu with debug tools
    on, and does nothing with them off (Harness test);
  - `download.js` provides exactly the functions
    `crates/app/src/sessions/web.rs` declares, its version matches, and
    `index.html` loads it between the bundle and the game (the
    `shuyi-cli` tests).
- [ ] If **Size** moved them out, the completion notes name the
      follow-up ticket.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `sessions.rs` (the decision to record, names, the ring); the
  `RunInput` checks (a session with start options, a file that isn't a
  record); `check_session` and `bless_session` on a toy record's text
  (a difference named, the inputs kept by a bless).
- Integration: the contract's `run` and `scene` with a session (the
  fake game for the refusals, Hello, Grid in
  `examples/hello-grid/tests/it/contract.rs` for a play and a
  difference); the MCP client test; Hello, Grid's golden session; the
  grid template's and `BasicTools`' sessions; unless **Size** moved
  them out, the debug menu entry, Hello, Grid's debug menu and the web
  shell's plugin tests.
- Manual: the desktop and web checks in step 7.

## Completion notes

*(Filled in by the session that completes the ticket.)*
