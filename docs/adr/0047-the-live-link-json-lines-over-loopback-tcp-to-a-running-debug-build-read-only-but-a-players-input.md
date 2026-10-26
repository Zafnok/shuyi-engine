# ADR-0047: The live link: JSON lines over loopback TCP to a running debug build, read-only but a player's input

- **Status:** Accepted
- **Date:** 2026-10-25
- **Related tickets:** 2521 (this: the wire, the listener in `shuyi-app`,
  `shuyi attach`), 2560 (MCP's session over the link), 2522 (the web
  build and a device on the LAN), 2555 (perf over the link), 2356 (debug
  commands), 2316 (pause and step), 2018 (the capture mode), 2524 (`act`
  in the contract's scripts), 2506 and 2507 (parked: the editor), 3004
  (`shuyi export`)
- **Extends:** ADR-0005 §2 (the contract's surfaces), ADR-0022 §3 (the
  session), ADR-0024 (the platform boundary), ADR-0028 (problem codes),
  ADR-0030 (the log)

## Context

- **The contract runs a game headless** (ADR-0005 §2, ADR-0022 §3): `run`,
  `scene`, `frame-png` and the MCP session drive a `Headless<G>` over the
  Harness, and `frame-png` paints with the engine's raster, never with
  `shuyi-app`'s renderer. An agent never sees the window players see,
  never presses keys through it, and can't read a running build's log.
- **Other engines reach a running game.** Bevy's Remote Protocol is
  JSON-RPC over HTTP (`bevy_remote`); Godot's "Deploy with Remote Debug"
  connects the game to the editor; Unity's Profiler and Console attach to
  a development Player; Unreal's Remote Control API reaches a running
  game. Each is a development feature.
- **App already keeps what a reader wants** (ADR-0030, ADR-0046): the log
  and the frames' records in rings numbered from 0, read `since` a
  number; `App::frame_number`; the game's screens and its frame buffer.
- **A backend owns the loop and the devices** (ADR-0024, `AGENTS.md`
  rule 5). `shuyi-platform` touches none, and is `no_std` without its
  `std` feature.
- **A web page can reach a loopback port**: a browser lets any page send
  an HTTP request to `127.0.0.1`. A listener that trusts loopback alone
  can be driven by a page the developer happens to have open.
- **Agents read text** (the contract answers JSON; MCP is JSON-RPC; the
  perf trace is JSON lines, ADR-0046 §7).

## Decision

1. **What it is.** The live link lets the contract reach a build that is
   already running. Agents use it through the contract, `shuyi attach`
   (an operation that needs no project, like `doctor`), and MCP's
   session (2560). The wire between `shuyi` and the build is internal,
   versioned by `shuyi_platform::link::VERSION` (1): the build answers a
   hello with its version, and closes the link to a client of another.
2. **The wire is JSON lines**: one UTF-8 JSON object per line, a
   `Request` tagged by `op` (`hello`, `status`, `frame`, `screenshot`,
   `play`, `log`) and an `Answer` tagged by `answer`, in order, one
   client at a time. The first transport is TCP on `127.0.0.1`. Later
   ones carry the same lines: a WebSocket the web page dials out on, and
   every network interface for a device on the LAN (2522). A USB serial
   line for the Playdate is a possible later one; its Simulator runs on
   the desktop, so the TCP link serves it first.
3. **The wire and the server live in `shuyi_platform::link`**, behind its
   `link` feature (`std`, `serde`, `serde_json`; the one outside crates
   `tools/layers.ron` gives `shuyi-platform`). They touch no device:
   `Server` turns request lines into answer lines over an `App<G>`, and
   gives the backend a play's input frame by frame, so the whole protocol
   is tested on the headless backend. `shuyi-cli` turns the same feature
   on and uses the same types, so the two ends can't drift.
4. **Debug builds only.** A backend's listener is compiled only with its
   own `link` cargo feature (`shuyi-app/link`), off by default. It
   listens only when debug tools are compiled in (`shuyi_ui::DEBUG_TOOLS`)
   and `SHUYI_LINK=<port>` is set (`0` takes a free one); otherwise it
   logs why and doesn't. No command-line flag: a game reads its own
   command line. `shuyi export` (3004) never turns `link` on.
5. **Loopback and a token.** The listener binds `127.0.0.1`. The first
   line must be a `hello` with the build's token, or the connection is
   closed and nothing after that line is read: a browser's request line
   is `GET …` or `POST …`, never a hello. The token is
   `SHUYI_LINK_TOKEN` (16 characters or more) or 128 random bits from the
   system (`getrandom`), as hex, logged in one line when the build starts
   listening: `shuyi link: listening on 127.0.0.1:<port>, token <token>`.
6. **Reads and a player's input, nothing else.** The link reads the
   screens and where the build is (`Status`), the frame as text, the
   window's picture and the log by number. Its only input is raw device
   events the backend adds in `Platform::poll_input` after the devices'
   own, so they go where a player's go: the keymap, the screens' declared
   actions, replays and session records (2518). It changes nothing any
   other way: no `Game::act`, no settings, no files, no reset. The one
   later exception is 2356's debug commands, which that ticket records
   through the game's own commands.
7. **Steps as the Harness has them**: keys and buttons by the Harness's
   names (`Chord::parse`, `Button::parse`), pressed in one frame and let
   go in the next; waits in frames, the build's own (a client turns
   seconds into frames at 60 a second, and says so in a limit); and an
   action by name (`AGENTS.md` rule 8). `link::press_for` is the one
   lookup of an action by name: `ActionSet::from_name`, then
   `Keymap::primary`, else `Keymap::primary_button`. An action is looked
   up when its play starts, the game loaded; one that can't be pressed
   fails the play with nothing played. The steps wait while the game
   loads or is suspended, and the answer says they did. A request after
   a play waits for the play's answer.
8. **A frame never waits on the link.** The sockets are non-blocking,
   read before `App::frame` and written after it; a request line is at
   most 64 KiB; a client whose unread answers pass 64 MiB is dropped. A
   second client hears one `busy` line, and the listener half-closes it
   and reads it until it hangs up, so closing never resets the
   connection before it reads the answer. The picture is read after
   `App::frame` has drawn and before the backend shows it
   (`get_screen_data()` in `shuyi-app`, its rows flipped, PNG by the
   `image` crate macroquad already uses), carried in base64.
9. **Pause and step are not operations of their own.** Once 2316 is
   done, a client sends the game's actions for `UiAction::DebugPause` and
   `UiAction::DebugStep` as `act` steps. Debug commands get a `command`
   request (2356), frame times a `perf` request (2555), and the editor
   (2506) adds the operations it needs to the same wire.
10. **Errors have codes** (ADR-0028): no build listening is a setup to
    fix (`SH1005`, exit 3), a wrong token a wrong argument (`SH0004`),
    another link version a setup (`SH1006`: attach with the build's own
    engine), and a link that is busy, closed or unreadable an operation
    that failed (`SH3006`).

## Consequences

- An agent checks the window players see, presses its keys as a player
  does, and reads the build's log, with the same contract and codes as
  the headless operations. The web build and a Steam Deck (2522), perf
  (2555), MCP (2560) and the editor (2507) build on one wire.
- The protocol is tested without a window: the server on the headless
  backend, the listener over a real loopback socket on the toy game, and
  `attach` against a test host in a thread. A property test holds that a
  play over the server ends as the Harness's.
- The link adds no new crate to `Cargo.lock`: `serde_json`, `image` 0.24
  and `getrandom` 0.4 were there.
- Each backend that offers the link writes its own listener (a socket, a
  WebSocket, a serial line) and its own picture; the server is shared.
  `docs/platform-backends.md` says what one must do.
- A debug build with the feature and `SHUYI_LINK` set accepts input from
  any local process that has its token: that is a developer's machine,
  and the token is in the build's own log only.
- The game's own scene, events and sounds are not on the link yet; the
  headless `run` and `scene` keep them.

## Alternatives considered

- **HTTP or JSON-RPC (as Bevy's Remote Protocol)**: a request-response
  framing over a stream gains nothing for one client, and an HTTP
  listener is exactly what a web page can talk to. JSON lines are read
  and written with `serde_json` alone, and carry over a WebSocket and a
  serial line unchanged.
- **A binary protocol (postcard)**: smaller, but an agent or a person
  can't read or type it, and the picture dominates the bytes anyway.
- **Trusting loopback without a token**: any web page could press keys
  in the build. The token costs one log line.
- **A listener in every build, turned on at run time**: a shipped game
  would carry a socket a player's machine could expose. Behind a cargo
  feature and `DEBUG_TOOLS`, a release or an export has no code for it.
- **The socket code in `shuyi-platform`**: it would touch the network
  from the boundary crate, which touches no device, and would not carry
  over to the web's WebSocket or a console's own sockets. The server is
  there; each backend owns its transport.
- **`Game::act` for actions**: it skips the keymap, the screens' declared
  actions and the records, so a check over the link would pass where a
  player's keys fail. Raw events after the devices' go the player's way.
- **A pause and a step of the link's own**: the game already has its
  debug actions (2316); a second way to pause would not be recorded the
  same way.
