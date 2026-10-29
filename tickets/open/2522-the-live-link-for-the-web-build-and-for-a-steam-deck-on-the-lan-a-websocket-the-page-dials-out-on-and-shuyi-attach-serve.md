---
id: "2522"
title: "The live link for the web build and for a Steam Deck on the LAN: a WebSocket the page dials out on through a plugin of our own, shuyi attach --serve, and a LAN setting"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2521"]
nick_input: none
completed:
---

# 2522 — The live link for the web build and for a Steam Deck on the LAN

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. The engines that let a tool reach a running game (2521
lists them) also reach one on another device. Godot's remote debugger
connects to a game deployed to another device over the network, and a
web export connects back to it over a WebSocket. Unity's Profiler
connects to a development build on another device by its address.

2521 builds the live link for a native build: `shuyi_platform::link`'s
`Server` answers JSON lines over TCP on `127.0.0.1`, in builds with
debug tools and the `link` feature, and `shuyi attach` and MCP read it.
Its ADR names the two transports this ticket adds. Two builds the game
ships can't be reached yet:

**From 2521 (done):** its ADR is
[ADR-0047](../../docs/adr/0047-the-live-link-json-lines-over-loopback-tcp-to-a-running-debug-build-read-only-but-a-players-input.md).
`Server::handle` returns `Handled::Close(Option<String>)`: a line to
send before closing (the build's hello, for a client of another link
version), so a bridge closes a client the same way. The listener's token
rules are `crates/app/src/link.rs`'s `token`, `hex` and `random_bits`;
the picture is `crates/app/src/readback.rs`'s `window_png`. The
listener and `readback` are compiled only with
`all(feature = "link", not(target_arch = "wasm32"))`, and `getrandom` and
`image` are native-only dependencies, so `link` builds on wasm32 and does
nothing there. A second client is told `Answer::Busy`, half-closed and
read until it hangs up; a client that hangs up just before a new one
connects makes way (`hung_up`). MCP's session over the link is 2560.

**From 2555 (done):** `link::VERSION` is 2. `Request::Perf { since,
max, snapshot }` reads `App::perf()` by number and answers
`Answer::Perf(PerfAnswer)`. With `snapshot`, `Server::handle` returns
`Handled::Snapshot`, and after the frame the backend checks
`Server::wants_snapshot` and sends `Server::snapshot(&app,
app.heap_snapshot(&mut platform))`, as `shuyi-app`'s `answer_link` does
beside the screenshot; the web backend and the bridge do the same.


- **The web build.** A page can't listen on a socket. It can only dial
  out, with a WebSocket. The web build has no `wasm-bindgen` (game
  ADR-0034), so it needs a miniquad JS plugin of our own, as
  controllers have (`crates/cli/web/gamepad.js`, read by
  `crates/app/src/pads/web.rs`). The vendored loader
  (`crates/cli/web/mq_js_bundle.js`) carries macroquad's `quad_net`
  plugin, which no Rust code uses (`crates/cli/web/README.md`). 1142
  (parked, networking) plans our own `web/net.js` instead. This ticket
  writes it so 1142 extends it.
- **A Steam Deck, or any second machine, on the LAN.** 2521 binds
  loopback only. The Deck (3012) runs a Linux build. Testing it at a
  desk (3013) today means the SteamOS Devkit Client and MangoHud, with
  no screen stack, frame text or engine log.

Why it matters:

- **The game first.** Nick wants Visions of Shuyi on the Steam Deck
  (2026-10-15), and it has a web build. An agent, or Nick at the
  computer, can then read the Deck build's screens and log, take its
  real picture and press its keys, and do the same with the web build
  in a browser.
- **Other 2D games:** any game whose `shuyi.ron` has a `web` part, as
  Hello, Grid's does, gets the same. The templates have no `web` part
  today (2528's Out notes it).
- **The end goal** (2026-10-19): an agent that makes a game "with a web
  ver" should watch the web build it made run.

## Nick input

None.

## Scope

**In:**
- **An ADR** (the `write-adr` skill, the next free number) that extends
  2521's for its two later transports. 2521's ADR's `Status:` line
  gains "Extended by ADR-NNNN (the web page and the LAN)", as
  ADR-0003's names ADR-0011. It decides:
  - the page dials out over a WebSocket to a bridge in `shuyi`, through
    our own JS plugin, and carries 2521's JSON lines unchanged, one line
    a message;
  - the bridge binds loopback only, checks the page's `Origin` and both
    ends' token, and uses `tungstenite` (a new dependency, below);
  - the LAN is plain TCP on every IPv4 interface, opt-in, with the
    token as the only guard, debug builds only, for a home or office
    network. No TLS. A firewall on the machine running the build may
    ask, or need a rule.
- **The LAN, for native builds** (`crates/app/src/link.rs`):
  - `SHUYI_LINK_LAN=<port>` binds `0.0.0.0:<port>` (every IPv4
    interface) instead of loopback. It is never the default. With both
    `SHUYI_LINK` and `SHUYI_LINK_LAN` set, it logs an error and listens
    on neither. The choice is a pure function, tested;
  - when it starts, it logs the port and the token, and a warning: any
    machine on the network that has the token can read this debug
    build's frames and send it input;
  - `SHUYI_LINK_TOKEN` (2521's rule: 16 characters or more) is how a
    developer sets the token in the Deck's launch options, where the
    log is hard to read: `SHUYI_LINK_LAN=7878
    SHUYI_LINK_TOKEN=<token> %command%`.
- **The Deck's steps:** if 3012 is done, `docs/targets/steam-deck.md`
  gains "Attaching from your computer": the launch options above, how
  to find the Deck's address, and `shuyi attach <address>:7878 --token
  <token>`. If not, the steps go in `docs/contract.md`'s "Attaching to
  a running build" (2521), and 3012 links them (edit requested there).
- **The page dials out:**
  - **`crates/cli/web/net.js`**, a miniquad plugin of our own named
    `shuyi_net`, like `gamepad.js`. It reads and writes the bundle's
    `wasm_memory`:
    - `shuyi_net_open(url_ptr, url_len) -> id`;
    - `shuyi_net_state(id)`: 0 connecting, 1 open, 2 closed;
    - `shuyi_net_send(id, ptr, len)`: one text message;
    - `shuyi_net_next_len(id)`: the next received message's length in
      bytes, or -1 when there is none;
    - `shuyi_net_take(id, ptr, len)`: copies that message into the
      wasm memory and drops it from the queue;
    - `shuyi_net_close(id)`;
    - `shuyi_net_hash_len()` and `shuyi_net_hash(ptr, len)`: the page's
      `location.hash`, as UTF-8;
    - received messages are queued up to 64 MiB, past which the socket
      is closed;
    - text messages only; 1142 adds binary ones (edit requested
      there);
    - its `VERSION` equals the Rust side's `PLUGIN_VERSION`, and
      `shuyi_net_crate_version()` returns it.
  - `crates/cli/src/web.rs`'s `SHELL` gets `net.js`.
    `crates/cli/web/index.html` loads it after `mq_js_bundle.js` and
    before `load(...)`, as it loads `gamepad.js`.
    `crates/cli/web/README.md` gets a section on it, like
    `gamepad.js`'s.
  - **Either order with 3052** (the web shell): if it is done,
    `index.html` loads `net.js` between the bundle and its last line,
    `if (shuyiShell.start(…)) load("%WASM%");`, so the order test
    holds as written, and the later of the two sets `SHELL`'s length.
    If not, nothing to do: 3052 keeps `net.js` where this ticket put
    it (its clause says so).
  - **From 3039 (done)** (the notices): `net.js` goes in
    `SHELL_NOTICES` (`crates/cli/src/web.rs`), the table of the shell's
    files, as the engine's own, beside `gamepad.js` (its test fails
    until it does).
  - **`crates/app/src/web_socket.rs`** (wasm32 only): the `extern "C"`
    declarations, the second file with `#[allow(unsafe_code)]` after
    `pads/web.rs`. The functions that take a pointer are `unsafe fn`,
    each wrapped in a safe function that takes a slice, with a `SAFETY`
    comment. A safe `WebSocket` (`open`, `state`, `send(&str)`,
    `recv() -> Option<String>`, `close`) sits over them, and 1142
    extends it. The comments that call `pads/web.rs` the only such file
    (in `crates/app/Cargo.toml` and at the top of `pads/web.rs`) name
    every plugin `extern` block instead, with 3043's, 2519's and
    2601's where those landed first.
  - **The link on wasm32** (`crates/app/src/link.rs`): with the `link`
    feature and debug tools, when `run` starts it reads the page's
    hash. If the hash holds `link=<host:port>&token=<token>`, it opens
    `ws://<host:port>/` and sends the page's hello with the token. Then
    it runs 2521's `Server` over the socket's messages, one JSON line a
    message, read and written between frames as on native. Without the
    hash, or with a bad one, it opens nothing. Without debug tools it
    logs why, as on native. If the socket fails or closes, it logs why
    and doesn't dial again: reload the page. Reading the hash is a pure
    function compiled on every target, so native tests cover it.
  - **`shuyi_platform::link` gains** (shared by the page and the
    bridge, so they can't drift):
    - `Page`, the page's own messages to the bridge, tagged by `page`:
      `Page::Hello { token, link }` (its first message) and
      `Page::Close` (its server closed the client: `Handled::Close`).
      Every other page message is an answer line for the client;
    - `Request::Detached`, which the bridge sends when its client goes.
      The server then waits for a new hello, as the native listener
      does when its client closes.

    A native build never sees either, so `link::VERSION` stays 2.
  - **The screenshot** reads the canvas as 2521's native screenshot
    reads the window (`get_screen_data()`, rows flipped, PNG). **From
    2019 (done):** it works in WebGL: the capture mode reads the canvas
    so in headless Chromium, pixel for pixel equal to the raster
    (`crates/app/src/readback.rs`'s `window`, compiled on the web with
    `capture`; its `window_png` is native only until this ticket turns
    it on there).
  - **Hello, Grid:** `examples/hello-grid/shuyi.ron`'s `web` part gets
    `features: ["link"]` (2521 adds the `link` feature to its
    `Cargo.toml`), so `cargo shuyi web --link` builds it.
  - **The wasm32 build is linted:** 2019's (done) `cargo clippy
    --target wasm32-unknown-unknown -p shuyi-app --features capture
    --lib --examples` step in CI's `wasm` job gets `link` in its
    `--features` (`capture,link`). This proves `link` builds on wasm32
    without `getrandom`. The `run-gates` skill's line (its *The web
    build in a browser* section) changes the same way.
- **`shuyi attach --serve [--port P] [--origin <url>]`**
  (`crates/cli/src/contract/attach/serve.rs`): a bridge between the
  page and `shuyi attach`. It answers no contract operation: it prints
  lines until it is stopped, so `AttachInput`, the schemas and
  `contract::VERSION` don't change. Threads and blocking sockets are
  fine here: it is a tool, not a frame.
  - It listens on `127.0.0.1:P` (by default port 0, a free one) and
    makes a token as 2521's listener does: 16 bytes from `getrandom`
    0.4, written as hex (MIT OR Apache-2.0, already in `Cargo.lock`).
    Its first line of output is the hash alone, to open the page with:
    `#link=127.0.0.1:<port>&token=<token>`.
  - A connection whose first line is an HTTP `GET` asking to upgrade is
    the page. `tungstenite` serves it as a WebSocket (MIT OR
    Apache-2.0; `default-features = false` with its `handshake`
    feature, no TLS; its dependencies are MIT or Apache-2.0 crates that
    `deny.toml` allows, and `cargo deny check` proves it). Its `Origin`
    must be `http://127.0.0.1:<any port>` or `http://localhost:<any
    port>`, or the one `--origin` names; any other gets 403. Its first
    message must be a `Page::Hello` with the token and this `shuyi`'s
    `link::VERSION`, or it is closed.
  - Any other connection is a `shuyi attach` client speaking 2521's
    JSON lines. The bridge checks its hello itself (the token and
    `link::VERSION`) and closes it with 2521's error otherwise. Then
    its lines, hello first, go to the page, and the page's answers come
    back. When the client goes, the page gets `Request::Detached`. On
    `Page::Close` the bridge closes the client.
  - One page and one client at a time. A second of either gets 2521's
    `busy` answer. A client that comes while no page is attached gets
    an error that says to open the page with the hash, and is closed.
    When the page goes, the attached client is closed with an error
    saying so.
  - It prints a line when the page or a client comes or goes.
  - So `shuyi attach 127.0.0.1:<port> --token <token> …` and MCP's
    `link` argument work on a web build exactly as on a native one.
- **Docs:**
  - `docs/contract.md`: `attach --serve` in *The operations*, and
    "Attaching to a running build" (2521) gains the web build (the
    bridge, the hash, `cargo shuyi web --link`) and the LAN
    (`SHUYI_LINK_LAN`, the warning);
  - `crates/app/src/lib.rs`'s module docs: how to run a web build with
    the link.
- **Either order:**
  - from 2019 (the web build in CI, done):
    `tools/web-check/check.py` gains `link <dir>`. It starts `shuyi
    attach --serve`, opens Hello, Grid's link build with the hash it
    prints, runs `shuyi attach … --act Confirm --json`, and checks that
    the screens changed. `web-render` runs it;
  - with 2528 (`shuyi web --serve`): if it is done, `shuyi web --serve
    --link` also starts the bridge and opens the page with its hash.
    If not, 2528 adds that (its clause says so);
  - with 1142 (parked): if 1142 is done, `net.js` and `web_socket.rs`
    exist already, with binary messages and the page's query string.
    Add only what this list has and they lack (the hash functions,
    `SHELL`, `index.html`'s order, the tests), keep their binary
    messages, and write no second plugin. If not, write them here, and
    1142 extends them (its own clause says so);
  - with 2601 (parked, HTTP requests): its web backend's `http.js`
    follows `net.js`'s queue and its limit on queued bytes, or the
    `net.js` list above if `net.js` isn't written yet. So if 2601 is
    done, nothing to do; if not, nothing to do (2601's clause says so);
  - with 3041 (log sinks): if it is done, the lines this ticket adds
    to `crates/app/src/link.rs` (the LAN's port, token and warning,
    and why the page opens no link) are `log::` calls with the target
    `"platform"`, as 3041's rule says, so its default filter keeps
    them. If not, macroquad's macros, and 3041 moves them with the
    backend's other lines (its scope names this ticket);
  - with 3012: as in *The Deck's steps* above.
- **Size.** If the PR passes about 600 changed lines (fixtures and
  snapshots aside), the LAN setting and the Deck's steps go to a
  follow-up ticket (`write-ticket`, After MVP, blocked by this one),
  and the PR says so. If it is still over, `check.py`'s `link` mode
  (2019's checker) and the either-order clause with 2528 go to that
  follow-up too, and the PR points them at it.

**Out (do not do):**
- a page on a public host (GitHub Pages, itch.io) dialing the
  developer's machine: browsers ask the player's permission for that,
  and the link is for local debug builds;
- the bridge on the LAN: it binds loopback only, so a page opened from
  another machine has no link (2528's `--lan` says so);
- a link in a web build without debug tools, or in `shuyi export web`
  (3004);
- TLS (`wss://`), and the link across the internet;
- finding builds on the LAN by themselves (mDNS, broadcast). 1143
  (parked) finds games to play, not builds;
- binary messages and `WsTransport` (1141 and 1142, parked);
- a `web` part in the templates (2777);
- the Playdate (2521's ADR notes it);
- phones' browsers (2407, parked).

## Implementation steps

1. Read 2521's ADR and code (`shuyi_platform::link`,
   `crates/app/src/link.rs`, `crates/cli/src/contract/attach.rs` and
   its test host); `crates/cli/web/` and its README,
   `crates/cli/src/web.rs` and its `gamepad.js` tests,
   `crates/app/src/pads/web.rs`, and 1141 and 1142. Read 2019's
   findings on reading the canvas (`crates/cli/web/README.md`, *Checked
   in CI*).
2. Write the ADR, and the line in 2521's ADR's `Status:`.
3. Add the LAN setting with its pure decision function and tests, and
   the Deck's steps.
4. Add `Page` and `Request::Detached` to `shuyi_platform::link`, with
   a test that `Detached` makes the server wait for a new hello.
5. Write `net.js`, `web_socket.rs` and the link on wasm32, with the
   hash reader's tests, the plugin's tests in `crates/cli/src/web.rs`
   (the function list reads `safe fn` and `unsafe fn` lines alike), and
   `index.html`'s order test.
6. Write the bridge and its tests.
7. Add `link` to Hello, Grid's `web` part and the wasm32 clippy step.
   Run `cargo shuyi web --link` in `examples/hello-grid`, serve
   `dist/web` on `127.0.0.1`, and try it in the browser pane. Keep the
   pane shown: a hidden pane may not run frames.
8. Attach to a native build with `SHUYI_LINK_LAN` from a second machine
   or the Deck, or else through this machine's LAN address.
9. Write the docs and run the gates.

## Acceptance criteria

- [ ] In the browser pane, Hello, Grid's link build, opened with the
      hash `shuyi attach --serve` printed, answers `shuyi attach
      127.0.0.1:<port> --token <token> --act Confirm --screenshot
      shot.png` with the screen after the title, and the file is the
      canvas's picture. Both are in the PR.
- [ ] A native build with `SHUYI_LINK_LAN` is attached from a second
      machine or a Deck on the same network, noted in the PR. With no
      second machine, it is attached from the same machine through its
      LAN address, and the PR says so.
- [ ] `SHUYI_LINK` and `SHUYI_LINK_LAN` together listen on neither (a
      test).
- [ ] The bridge refuses a page from another origin, a page whose hello
      has the wrong token, a client whose hello has the wrong token, a
      second page, a second client, and a client while no page is
      attached (tests).
- [ ] Through the bridge, `shuyi attach` gets the same answers as over
      a direct link to the same game (a test).
- [ ] `net.js` and `web_socket.rs` list the same functions and the same
      version, and `index.html` loads `net.js` between the bundle and
      `load(...)` (`shuyi-cli` tests).
- [ ] Without the hash, or with a bad one, a link build's page opens no
      socket: the hash reader's tests, and the browser pane's network
      list in the PR.
- [ ] `shuyi-app` with `link` passes clippy for
      `wasm32-unknown-unknown` in CI's `wasm` job.
- [ ] The ADR is in `docs/adr/` with its row in the index, and 2521's
      ADR's `Status:` line names it.
- [ ] `cargo deny check` passes with `tungstenite`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (native): reading the hash (`link=` and `token=`, missing or bad
  fields, other fields around them); the bridge's `Origin` rule; the
  LAN decision (`SHUYI_LINK`, `SHUYI_LINK_LAN`, both, neither, no debug
  tools); `Page` round trips as JSON; `Request::Detached` makes the
  server wait for a new hello.
- Unit (`shuyi-cli`): `net.js` and `web_socket.rs` list the same
  functions and version; `index.html` loads `net.js` between the bundle
  and `load(...)`; `SHELL` writes `net.js`.
- Integration (native): the bridge in a test thread, with a
  `tungstenite` client playing the page (its `Page::Hello`, then 2521's
  `Server` over the toy game on `HeadlessPlatform`, as 2521's test host
  runs it) and `shuyi attach` talking to it through the bridge; each
  refusal in the criteria; the LAN listener's address is `0.0.0.0`, and
  a client reaches it.
- Manual: the browser pane, as in the first criterion; a second machine
  or a Deck on the LAN.

## Completion notes

*(Filled in by the session that completes the ticket.)*
