---
id: "2521"
title: "The live link: an ADR, a debug-only listener in shuyi-app, and shuyi attach and MCP reading a running windowed build"
type: feature
milestone: MVP
model: opus-5.5
effort: high
status: in-progress
blocked_by: ["1133", "3040"]
nick_input: none
completed:
---

# 2521 — The live link to a running build

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Each lets a tool reach a game running in its real
window:

- **Bevy:** the Bevy Remote Protocol, `bevy_remote`: JSON-RPC over
  HTTP that queries and watches the world
  (https://docs.rs/bevy_remote). The community crates
  `bevy_brp_extras` (screenshots, keys, gamepad) and `bevy_brp_mcp`
  (an MCP server) are built on it.
- **Godot:** "Deploy with Remote Debug" runs the game connected to the
  editor, and the Remote scene tree reads it while it runs
  (https://docs.godotengine.org/en/stable/tutorials/scripting/debug/overview_of_debugging_tools.html).
- **Unity:** the Profiler and the Console attach to a development build
  of the Player over the Player connection.
- **Unreal:** the Remote Control API reaches a running game, packaged
  builds included.

What Shuyi has today:

- The tool contract runs a game **headless** (ADR-0005 §2, ADR-0022
  §3). `run` and `scene` drive a `contract::session::Session`
  (`crates/cli/src/contract/session.rs`), and the only one is
  `Headless<G>`, over the Harness. 1133 (MCP, in the MVP) keeps one
  between calls. Its Out says "a network transport (stdio only)".
- `frame-png` paints with the CLI's own painter (2332 moves it to
  `shuyi_ui::frame::raster`), never with `shuyi-app`'s renderer
  (`crates/app/src/render.rs`).
- `shuyi-app` has no listener. It reads environment variables only for
  the storage folders (`crates/app/src/storage/native.rs`), `MUSIC_DIR`
  and `VOICE_CLIP` (`crates/app/src/audio/native_music.rs`). Nothing in
  the workspace uses `std::net`.
- **From 3040 (done; ADR-0030):** a running build's log is App's
  `LogRing` (`shuyi_ui::log`), read with `App::log`: the last 500 lines
  (`LOG_LINES`), each a `LogLine` with a frame, a level, a category and
  its text. A line holds no number: the ring numbers the lines pushed
  from 0, `LogRing::since(s)` gives the kept lines numbered `s` or more,
  and `next_seq()` is the number the next line gets, so the kept line
  at index `i` is numbered `next_seq() - kept + i`. `dropped()` counts
  every line ever dropped, those Ctx dropped before they got a number
  included. `run` prints `App::take_new_log` with macroquad's macro
  for each line's level, and `App::frame_number` counts frames.
- 2506 (parked editor research) weighs a native game over a local
  socket, and 2507 (parked) connects the editor to the game by what
  2506 picks. Both take this link as their transport.

So an agent never sees the renderer players see, never presses keys
through the real window, and can't read what a running build logged.
To check a web build today it drives a browser and reads pixels, with
no screen stack and no log.

Why it matters:

- **The game first.** Visions of Shuyi ships a windowed build, and a
  web and a Steam Deck build. Agents working on it judge it by the
  headless raster. The link lets them look at the window players see,
  press its keys the way a player does, and read its log.
- **Other 2D games** get the same. The gap grows when screen effects
  and a game's shaders on wgpu (2326, 2327, 3034, parked) make the
  window and the raster differ.
- **The end goal** (2026-10-19): an agent that makes a game "with a
  web ver" should watch the build it made run. The web build and a
  Steam Deck on the LAN are 2522.
- **Perf traces** (Nick, 2026-10-24, [`docs/decisions.md`](../../docs/decisions.md)):
  "easily agent-readable / streamable perf traces when building in
  debug -- both timings and heap snapshots". He put them in the MVP,
  and moved this ticket into the MVP with them, so an agent reads a
  running build's frame times and heap over the link (2555).

This ticket builds the link on one transport, TCP on loopback, and the
two things that read it: `shuyi attach` and 1133's MCP session.

## Nick input

None.

## Scope

**In:**
- **An ADR** (the `write-adr` skill), with the next free number,
  extending ADR-0005 §2 and ADR-0022 §3:
  - **What it is.** The link lets the contract reach a build that is
    already running. Agents use it through the contract (`shuyi
    attach`) and MCP. The wire between `shuyi` and the build is
    internal and has its own version, `link::VERSION`.
  - **The wire:** JSON lines (one UTF-8 JSON object per line) over
    TCP, one client at a time, requests answered in order. Later
    transports carry the same lines: a WebSocket the web page dials out
    on, and every network interface for a device on the LAN (both
    2522). A USB serial line for the Playdate is a possible later one,
    with no ticket: its Simulator runs on the desktop.
  - **Debug builds only.** The listener is compiled only with the
    backend's `link` cargo feature. It listens only when debug tools are
    compiled in (`shuyi_ui::DEBUG_TOOLS`: a debug build, or the
    `debug-tools` feature) and `SHUYI_LINK` is set. `shuyi export`
    (3004) never turns `link` on, and 3004 tests its cargo arguments
    for that.
  - **Loopback and a token.** It binds `127.0.0.1`. The first line must
    be a `hello` with the token, or the connection is closed. The token
    is `SHUYI_LINK_TOKEN` (16 characters or more) or 128 random bits
    from the OS, logged when it starts. A web page can send an HTTP
    request to a loopback port; its first line is `GET …` or `POST …`,
    never a hello, so it is closed.
  - **Reads and input, nothing else.** It reads the screens, the frame
    as text, the window's pixels and the log. Its input is raw device
    events that the backend adds in `poll_input` after the devices'
    own, so they go where a player's go: the keymap, the screens'
    declared actions, replays and session records (2518). It changes
    nothing any other way: no `Game::act`, no settings, no files. The
    one later exception is 2356's debug commands, which that ticket
    records through the game's own commands.
  - **Actions by name** (`AGENTS.md` rule 8, the `input-actions`
    skill): an `act` step presses the player's primary key for the
    action (`Keymap::primary`), or its primary button
    (`Keymap::primary_button`, one button or two pressed together) when
    it has no key. Key and button steps exist too, as in the Harness's
    test scripts.
  - **A frame never waits on it:** non-blocking sockets, read and
    written between frames; a request line is at most 64 KiB; a client
    whose unsent answers pass 64 MiB is dropped.
  - **Pause and step** are not operations of their own. Once 2316 is
    done, a client sends the game's actions for `UiAction::DebugPause`
    and `UiAction::DebugStep` as `act` steps. Debug commands (2356) get
    a `command` request (either order, below). The editor (2506) uses
    this link as its transport and adds the operations it needs to it.
- **`shuyi_platform::link`** (`crates/platform/src/link.rs`), behind a
  new `link` feature (`std`, and optional `serde` and `serde_json`;
  `serde_json` is MIT OR Apache-2.0 and already in `Cargo.lock`
  through `shuyi-cli`). It touches no device: it turns request lines
  into answer lines over an `App<G>`, so it is tested on the headless
  backend.
  - `VERSION: u32 = 1`.
  - `Request` (serde, tagged by `op`): `Hello { token, link }`,
    `Status`, `Frame`, `Screenshot`, `Play { steps }`, `Log { since }`.
  - `Step`: `Keys(String)` and `Pad(String)` (the names
    `Harness::keys` and `Harness::pad` take, checked with
    `Chord::parse` and `Button::parse`), `Act(String)`, `Wait(u32)`
    (frames).
  - `press_for<A: ActionSet>(keymap: &Keymap<A>, name: &str) ->
    Result<Press, String>`, the one lookup of an action by name:
    `ActionSet::from_name`, then `Keymap::primary`, else
    `Keymap::primary_button`. `Press` is `Keys(Chord)` or
    `Pad(ButtonChord)`. The error names the action: no such action, or
    one with neither a key nor a button. The server gets the keymap
    from `Game::input().keymap()`.
  - `Status`: `frames` (`App::frame_number`, 3040), `phase` (`loading`
    while `App::is_loading`, `running`, or `failed` with
    `App::errors`'s text), `screens` (bottom first, `Game::screens`),
    `top_screen`, `suspended`, `focused` and `quit`
    (`App::is_suspended`, `is_focused`, `has_quit`).
  - `Server`:
    - `Server::new(token: String, game: String)`, where `game` is the
      window's title;
    - `handle(&mut self, line: &str, app: &App<G>) -> Handled`:
      `Handled::Answer(String)` now; `Handled::Later` for `Play`, whose
      answer comes when its last frame has run; `Handled::Screenshot`,
      which the backend answers after the next `present` with
      `screenshot(png, width, height)`; `Handled::Close` for a bad
      first line. A `Play` whose steps don't parse is answered at once
      with an error naming the step. Its `Act` steps are looked up when
      it starts (the game is loaded by then): if `press_for` can't press
      one, the play answers with that error and plays nothing. A
      request after a `Play` waits for the play's answer;
    - `input(&mut self, app: &App<G>, out: &mut Vec<RawInputEvent>)`:
      this frame's events. A key goes `Down(chord)` in one frame and
      `Up(chord.key)` in the next, as `Harness::keys` does; buttons
      joined by `+` go down together on a generic pad
      (`PadKind::default()`), then up, as `Harness::pad` does;
      `Act` is the key or buttons `press_for` gives, played the same
      way; `Wait(n)` is n frames with none. While the App is loading or
      suspended the steps wait;
    - `after_frame(&mut self, app: &App<G>) -> Vec<String>`: the
      answers now due;
    - `Frame` answers `game.buffer().to_snapshot(&game.ctx().palette)`,
      as `Harness::snapshot` does;
    - `Log { since }` answers `App::log().since(since)`: the kept lines
      numbered `since` or more (0, the default, is every kept line),
      each with its number (derived as above, or by a `LogRing` helper
      that yields each kept line with its number), frame, level,
      category and text. It also answers `next`, the ring's
      `next_seq()`, which the next request passes as `since`, and how
      many lines numbered `since` or more the ring no longer keeps
      (`(next_seq - kept).saturating_sub(since)`, not `dropped()`,
      which counts every line ever dropped). The server keeps no log
      of its own;
    - a small base64 (RFC 4648) to carry the picture in a JSON line.
- **The listener in `shuyi-app`** (`crates/app/src/link.rs`), behind a
  new `link` feature (`shuyi-platform/link`, and optional `image` and
  `getrandom`):
  - `SHUYI_LINK=<port>` turns it on (`0` takes a free port). With no
    debug tools it logs why and doesn't listen. No command-line flag:
    a game reads its own command line;
  - one client at a time: a second connection while one is attached
    gets one `busy` answer line and is closed;
  - `run` polls it before `app.frame` (accept, read lines,
    `Server::handle`) and hands `Server::input` to `Macroquad`, whose
    `poll_input` (`crates/app/src/backend.rs`) adds those events after
    the keyboard's and the pads'. After `app.frame` it writes the
    answers due. `run` still prints `App::take_new_log` (3040) as
    before; the link reads `App::log` and takes nothing from it;
  - **the screenshot:** after `app.frame` has drawn the frame and
    before `next_frame().await`, `get_screen_data()`
    (`macroquad-0.4.16/src/texture.rs`) reads the window. Its rows come
    bottom up, so flip them. It is encoded as PNG with the `image`
    crate macroquad already uses (0.24, `default-features = false`,
    `png`; MIT OR Apache-2.0; no new crate in `Cargo.lock`). While the
    loading card or the errors show, the answer is that picture, with a
    limit saying so;
  - **the token:** 16 bytes from `getrandom` 0.4 (MIT OR Apache-2.0,
    already in `Cargo.lock` through `tempfile`; native only, since on
    wasm32 it needs `wasm-bindgen`), written as hex;
  - when it starts, it logs one line with `info!`: `shuyi link:
    listening on 127.0.0.1:<port>, token <token>`;
  - on wasm32 the feature builds and does nothing (2522);
  - `shuyi-app`'s tests turn `link` on through a dev-dependency on
    itself, as `shuyi-platform`'s do with `harness`, so clippy and the
    tests cover it.
- **`shuyi attach`**, an operation of the contract that needs no
  project, like `doctor` (`crates/cli/src/contract/attach.rs`):
  `shuyi attach <host:port> --token <t> [steps] [--frames N]
  [--screenshot <file.png>] [--log-since N] [--json]`.
  - Steps are `run`'s (`--keys` or `--script`, `--pad`, `--wait` in
    seconds, turned into frames at 60 a second, read with
    `Script::parse_step`), and `--act <Action>`. **Either order with
    2524**, which adds `Step::Act` (an action pressed by its primary
    key, or its primary button) to the contract's scripts, so there is
    one parser and one lookup:
    - if 2524 is done, `Script::parse_step` reads `--act` here too,
      and its `Step::Act` becomes the link's `Act`. Its lookup by name
      moves into `press_for`, which `Script::play` then calls;
    - if not, `attach` reads `--act` itself, next to
      `Script::parse_step`, and `run`, `scene` and `frame-png` don't
      take it. 2524 then moves `--act` into `Script::parse_step` and
      plays it with `press_for` (its own clause says it uses this
      ticket's parsing and lookup).
  - It connects, says hello, plays the steps, lets `--frames` frames
    pass, then asks for the status, the frame and the log after
    `--log-since` (and the screenshot, written to the file, when
    asked).
  - `AttachInput` and `AttachOutput` derive `JsonSchema`.
    `AttachOutput`: `link`, `game`, `frames`, `phase`, `top_screen`,
    `screens`, `suspended`, `focused`, `quit`, `frame`, `screenshot`
    (path, width, height), `log`: each line as 3040's `LogEntry` plus
    its `seq` (its number in the ring), and `log_next`: the ring's
    `next_seq()`, which the next `--log-since` passes to read on from
    there (`--log-since` defaults to 0, every kept line). When lines
    numbered `--log-since` or more were dropped before this read, a
    limit says how many, as 3040's does for `run`.
    `attach.input.json` and `attach.output.json` are checked in under
    `crates/cli/schemas/`; raise `contract::VERSION` by one and add its
    line to `versions.json`.
  - Its limits say what it could not see: "the build was suspended: the
    steps waited", "waits are counted in the build's own frames", "the
    build shows its loading card".
  - Its errors say what to do: no build listening at the address, a
    wrong token, a link version that isn't this `shuyi`'s.
  - `shuyi-cli` turns on `shuyi-platform/link` to share `Request`,
    `Step`, `Status`, `press_for` and the base64, so the two ends can't
    drift.
- **MCP** (1133): its `start` tool takes a `link` argument (an address
  and a token), which attaches the session to a running build instead
  of starting the game headless. Then:
  - `step` plays over the link;
  - `frame_png` returns the window's picture. If 2514 is done, its
    `every` and `count` are refused over a link ("one picture at a
    time over the link"). If not, 2514 refuses them;
  - `scene` answers the frame as text, with the limit "no scene over
    the link";
  - `events` answers no events, with the limit "no events over the
    link";
  - `reset` is refused ("the link never changes a running build");
  - `stop` detaches and leaves the game running.

  1133's scripted client test gets a step that attaches to a test host.
- **Opting in:** a `link` feature, off by default, in
  `examples/hello-grid/Cargo.toml` (`link = ["shuyi-app/link"]`) and
  in the game crates of `templates/grid` and `templates/blank`.
- **Docs:**
  - `docs/contract.md`: `attach` in *The operations*, and a section,
    "Attaching to a running build": the feature, `SHUYI_LINK`, the
    token, what the link reads, and what it never does;
  - `docs/platform-backends.md`: a backend may offer the link. It adds
    the server's input after its own in `poll_input`, and answers a
    screenshot after `present`;
  - `crates/app/src/lib.rs`'s module docs: how to run a game with it.
- **Either order:**
  - with 2018 (the capture mode): both read the window with
    `get_screen_data()`, flip its rows and encode PNG with `image`, and
    both turn key, button and wait steps into frames of raw input.
    Whichever lands second reuses the first's code (one helper in
    `crates/app/src/readback.rs`, and `link`'s step frames or
    `capture::Plan::frames`) instead of writing it again;
  - with 2316 (pause and step): if it is done, a test plays the toy
    grid game over the server, pauses with an `act` step of the toy's
    action for `UiAction::DebugPause`, steps with its action for
    `UiAction::DebugStep`, unpauses, and ends with the same screens and
    frame text as a run of the same updates with no pause. If not, 2316
    adds that test (its own clause says so);
  - with 2356 (debug commands): if it is done, `Request::Command {
    line }` runs `Game::command` through `App::command` if 2518 is
    done (so it is recorded), or else `App::game_mut` (so `handle`
    takes `&mut App<G>`; 2518 moves it onto `App::command` when it
    lands, its own clause says so), and answers its reply. If not, 2356
    adds it (its own clause says so);
  - with 2518 (session records): App records the input of its one
    `poll_input` a frame, so the link's input is in a session record
    either way. If 2518 is done, a test shows it; if not, 2518 writes
    that test (its own clause says so);
  - **from 1134 (done)** (the skills): the `shuyi-test` skill
    (`skills/shuyi-test/SKILL.md`) gets a section on attaching to a
    running build: when to use it rather than the headless `run`, the
    `link` feature, `SHUYI_LINK`, the token, and `--screenshot`. Its
    worked example needs a window, so CI can't run it: it is marked
    manual (`manual: Some("needs a window")`). The runner
    (`crates/xtask/src/skills.rs`) reads one `example.ron` per skill,
    and `shuyi-test` already has one, so add a second file to what it
    reads (`example-attach.ron`), listed and not run;
  - with 2509 (done, problem codes, ADR-0028): the three errors above
    are `Answer::failed` with codes. "No build listening" is a setup
    code (`SH1nnn`, exit 3: start the build, then retry); the wrong
    token and the link version that isn't this `shuyi`'s get the code
    of their class. A new code is a row in
    `crates/cli/src/contract/codes.rs`'s `TABLE` with its page in
    `crates/cli/errors/` (the gate in `codes/tests.rs` checks both);
  - with 3041 (log sinks): if it is done, the listener's two lines are
    `log::` calls with the target `"platform"`, as 3041's rule says:
    `log::info!` for the port and the token, and `log::warn!` for why
    it doesn't listen. Its default filter keeps them; a macroquad
    `info!` would carry the module's path as its target and be
    dropped. If not, macroquad's `info!` and `warn!`, and 3041 moves
    them with the backend's other lines (its scope names this ticket);
  - the new public types follow ADR-0039.
- **Size.** If the PR passes about 600 changed lines (fixtures,
  snapshots and the generated schemas not counted), the MCP part goes
  to a follow-up ticket (`write-ticket`), in the MVP, blocked by 2521,
  and the PR says so. That ticket also takes 2555's part that answers
  MCP's `perf` tool over the link (2555 says so).

**Out (do not do):**
- the web build, and a device on the LAN (2522);
- frame times and heap readings over the link: 2555 adds a `Perf`
  request, in the MVP;
- the game's own scene (`Scene::data`), events and sounds over the
  link. The headless `run` and `scene` have them. 2506 decides how the
  editor's tree (2507) gets `data` over the link; write a ticket sooner
  when an agent needs them from a running build;
- changing state, settings or files over the link (2356's debug
  commands aside), and loading assets (hot reload is 2220);
- a listener in a build without debug tools, or in an export (3004);
- TLS, accounts, and several clients at once;
- pause, step and channels as operations of their own (2316's actions;
  2506 decides what the editor adds);
- a Playdate transport (the ADR notes it).

## Implementation steps

1. Read ADR-0005, ADR-0022, ADR-0024 and `docs/platform-backends.md`;
   `crates/platform/src/app.rs`, `harness.rs` and `headless.rs`;
   `crates/app/src/lib.rs` and `backend.rs`;
   `crates/ui/src/input/keymap.rs` (`primary`, `primary_button`);
   `crates/cli/src/contract.rs`, `contract/cli.rs`, `contract/doctor.rs`
   (an operation with no project), `contract/session.rs` and
   `frame_png.rs`; 1133's MCP server; 3040's `LogRing` and `App::log`;
   and the code of 2018 and 2524 if they are done.
2. Write the ADR.
3. Write `shuyi_platform::link` and its tests on the toy game
   (`shuyi_ui::toy`): `App::running` on `HeadlessPlatform`, the
   server's events fed in with `HeadlessPlatform::push_input`, each
   frame run with `App::frame_after(&mut platform, FRAME_DT)`.
4. Write the listener in `shuyi-app`, its socket tests and the
   screenshot. Keep the socket code free of macroquad calls (the
   screenshot aside), so its tests run it over the toy game (a
   dev-dependency on `shuyi-ui` with `toy`) without a window. Run
   hello-grid with it on your desktop.
5. Write `shuyi attach` (with `--act` as the 2524 clause says), its
   schemas, and its tests against a test host.
6. Add MCP's `link` argument and its client test step (or the
   follow-up ticket, under the size rule).
7. Add the `link` features to hello-grid and the templates, write the
   docs, and run the gates.

## Acceptance criteria

- [ ] On a desktop, after `SHUYI_LINK=0 cargo run -p hello-grid
      --features link`, `cargo shuyi attach 127.0.0.1:<port> --token
      <t> --act Confirm --screenshot shot.png` answers with the screen
      after the title and writes the window's picture. The picture is
      in the PR.
- [ ] Over the server, on the toy game, a `Play` of keys, buttons and
      waits gives the same screens and the same frame text as the
      Harness playing the same script.
- [ ] A connection whose first line isn't a hello with the right token
      is closed, and nothing after that line is read: tests with a
      wrong token, an HTTP request line, and a line over 64 KiB.
- [ ] A second client while one is attached gets a `busy` answer and
      is closed.
- [ ] A frame never waits on the socket: with no client, and with a
      client that sends nothing, a poll returns at once (a test).
- [ ] Without the `link` feature `shuyi-app` never reads `SHUYI_LINK`.
      With it but without debug tools, it doesn't listen (a test of the
      decision as a pure function).
- [ ] `act` with an action that has a key, one with only a button, one
      with neither and a name the game doesn't have (the last two an
      error naming the action) all behave as said.
- [ ] `press_for` is the only lookup of an action by name: the server
      calls it, and so does `Script::play` if 2524 is done.
- [ ] `attach.input.json` and `attach.output.json` are checked in, and
      `versions.json` has the new version.
- [ ] `attach` with no build listening answers its setup code and exits
      3; a wrong token and a link version that isn't this `shuyi`'s
      answer their own codes (tests against a test host).
- [ ] 1133's scripted client test attaches to a test host, steps it and
      reads its frame, or the follow-up ticket is written under the
      size rule.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: requests and answers round trip as JSON lines; each `Step`
  into frames of events; `press_for` (a key, a button, a chord of two
  buttons, neither, an unknown name); `attach`'s argument parsing,
  `--act` and `--wait` in frames included; base64 against RFC 4648's
  test vectors; the token rules (length, hex, `SHUYI_LINK_TOKEN`); the
  hello rules; `Log` with `since` over 3040's ring (0 gives every kept
  line with its number; the answer's `next` gives none until a line is
  pushed), dropped lines included.
- Property: for random scripts of keys, buttons and waits on the toy
  game, the server's play and the Harness's end with the same screens
  and the same frame text (the Harness waiting in whole frames of
  `FRAME_DT`).
- Integration: the listener over a real loopback socket in
  `shuyi-app`'s tests (no window); `shuyi attach` against a test host
  that runs the server on the toy game in a thread; the MCP client
  step.
- Manual: hello-grid on a desktop, as in the first criterion.

## Completion notes

*(Filled in by the session that completes the ticket.)*
