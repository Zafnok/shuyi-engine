---
id: "2528"
title: "shuyi web --serve: the web build served on localhost or the LAN, rebuilt and reloaded on change, with the browser's log sent back to the terminal"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2528 — `shuyi web --serve`

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and Construct 3 and Defold 1.14. Each of them runs a
build on its target in one step:

- the Bevy CLI's `bevy run web --open` builds the game for the web,
  serves it and opens the browser
  (<https://github.com/TheBevyFlock/bevy_cli>);
- Godot's one-click deploy exports the web preset and runs it on a
  local server
  (<https://docs.godotengine.org/en/stable/tutorials/export/one-click_deploy.html>);
- Construct 3's Remote Preview serves the project to other devices on
  the LAN by a URL, with "Reload all"
  (<https://www.construct.net/en/make-games/manuals/construct-3/overview/testing-projects>);
- Defold reloads changed resources in a running dev build
  (<https://defold.com/manuals/hot-reload/>);
- Unity's Build and Run and Unreal's Launch do the same for their
  targets.

What Shuyi has today (checked 2026-10-20):

- **`shuyi web`** (`crates/cli/src/web.rs`): `run` (line 146) runs
  `cargo build` for `wasm32-unknown-unknown`, then `package` (line 163)
  copies the wasm and writes the shell files of `SHELL` (line 25) into
  the `web.dist` folder `shuyi.ron` names. It is not a contract op and
  has no `--json`. `crates/cli/src/lib.rs` dispatches it (line 169,
  `fn web` at line 341), and its usage line is line 71.
- **Then the developer finds a server.** `README.md:181` says "serve
  that folder over HTTP", and `crates/cli/web/README.md:130` says any
  static file server will do. Nothing in the repository serves
  anything: no `TcpListener` outside the tickets.
- **The page's errors stay in the browser.** The engine and macroquad
  write to the browser's console (`console_log` to `console_error` in
  the bundle's `importObject`, from miniquad's `js/gl.js`). A panic
  goes through the hook miniquad 0.4.11 sets in its
  `src/native/wasm.rs` (`panic::set_hook`, line 60), which logs the
  `Debug` form of `PanicHookInfo` with `console.log`: the file and line,
  but not the message, since the payload prints as `Any { .. }`. Then
  the wasm stops with a `WebAssembly.RuntimeError`. A terminal, and so
  an agent, sees none of it.

What is planned:

- 2220 (After MVP): `shuyi run --watch` reloads assets in a running
  desktop build. Its Out leaves the web aside ("no files to watch").
- 2905 (research): reloading code, by a rebuild that replays to the
  same moment.
- 2019 (After MVP): CI opens the web build in headless Chromium, served
  by Python's `http.server`, and fails on console errors. That is CI,
  not a developer's loop.
- 2522 (After MVP): the live link for the web build, `shuyi attach
  --serve`. Its either-order clause with this ticket: `shuyi web
  --serve --link` starts its bridge.
- 3052 (After MVP): the shell's loading states (`data-shuyi` on
  `<html>`) and readable errors. Its `file` error names `shuyi web
  --serve` once this ticket is done.
- 3041 (After MVP): log sinks; on the web, the browser's console by
  level, and a panic hook that logs the message.
- 3004 (After MVP): `shuyi export web` packages the folder and runs
  nothing.
- 3039 (done): `web::run` and `package` also write `NOTICES.txt` into
  `web.dist`, and `package` takes every work for it as an argument.

Why it matters:

- **The game first.** Visions of Shuyi has a web build, made by `cargo
  shuyi web` from its `web` part. Each try today is a build, a server
  started by hand, a browser, and its developer console. At its next
  pin bump it gets one command, with no code change.
- **Other 2D games:** every game with a `web` part, Hello, Grid, and
  the book's and the tutorial's web steps (2803).
- **Agents.** An agent can't read a browser's console. With `--json`,
  it reads the page's errors in its own terminal and knows when the
  page has loaded the new build. The agent check (2524) grades a web
  build. The end goal itself (2026-10-19, a game "with a web ver")
  waits for real time and the game's launch (2026-10-20); this loop
  will be there for it.

## Nick input

None.

## Scope

**In:**
- **The flags** (`web::parse_args`, `crates/cli/src/web.rs`):
  `shuyi web [--release] [--<feature>...] --serve [--port P] [--lan]
  [--open] [--json]`:
  - `--port`, `--lan`, `--open` and `--json` need `--serve`, each at
    most once; otherwise the usage line and exit 2;
  - `--port P`: 4000 by default; 0 asks the system for a free port;
  - a `web.features` entry named `serve`, `port`, `lan`, `open` or
    `json` makes every `shuyi web` fail with exit 2 and a line that
    names the entry in `shuyi.ron` (the game's features, checked
    2026-10-20, use none of these names);
  - `web::usage` and the usage text in `crates/cli/src/lib.rs` show
    them.
- **The loop** (`crates/cli/src/web/serve.rs`, a child module of `web`,
  so it can call the private `package`):
  1. it builds once, as `shuyi web` does with the same flags. If that
     build fails, it exits 1 with the error, as `shuyi web` does;
  2. it serves `web.dist` and prints the URL (below);
  3. it polls the watched files once a second (below). On a change it
     builds again. Changes made during a build start one more build
     after it;
  4. a good build bumps the build number, and the page reloads. A failed
     one prints the error, keeps the last good build served, and the
     page stays as it is;
  5. it runs until the process is stopped. It writes nothing outside
     `web.dist` (and the target folder, through cargo).
  - The builder is a small trait (`build(&mut self) -> Result<String,
    String>`), so tests drive the loop with a fake. The real one runs
    `cargo build` and `package` as `web::run` does, and holds the
    server's lock while `package` writes, so no request reads a
    half-written file.
- **The server**, on `std::net` alone, with no new dependency:
  - it takes the folder to serve and a `dev` switch. With `dev` on, it
    injects the dev script and answers the two `/__shuyi/` endpoints
    (below); with it off, it serves the folder as it is, which 3004's
    `web` run step uses;
  - it binds `127.0.0.1:P`; with `--lan`, `0.0.0.0:P` (IPv4);
  - one thread accepts. Each connection gets a thread, a read timeout
    of 10 s, one request and `Connection: close`;
  - **requests:** the request line and headers up to 16 KiB, else 431.
    `GET` and `HEAD` serve files, `POST` only `/__shuyi/log`, and
    anything else gets 405;
  - **files:** the path is percent-decoded and `/` is `index.html`. A
    path with a `..` segment, a backslash, a drive letter or a NUL is
    404, and so is a path whose canonical form is not under the
    canonical `web.dist` (a link out of it);
  - **types** by extension: `.html` (`text/html; charset=utf-8`), `.js`
    (`text/javascript`), `.wasm` (`application/wasm`), `.ogg`
    (`audio/ogg`), `.ron` and `.txt` (`text/plain; charset=utf-8`),
    `.json`, `.png` and `.ico`; any other is
    `application/octet-stream`;
  - every answer carries `Cache-Control: no-store`, so a reload fetches
    the new wasm;
  - **Host:** the `Host` must be `localhost:P` or an IPv4 address with
    `:P`, and without `--lan` only `127.0.0.1:P` or `localhost:P`. Any
    other gets 403. So a page on a domain name that points at this
    machine (DNS rebinding) is refused, with `--lan` too;
  - **Origin:** a `POST` whose `Origin` is missing or is not `http://`
    and its own `Host` gets 403, so another site open in the browser
    can't write to the terminal;
  - **a port in use:** exit 1 with "port P is in use (an earlier `shuyi
    web --serve` may still be running); pass `--port 0` for a free
    one".
- **The page gets a dev script; the folder never does:**
  - when the server answers `/` or `/index.html`, it inserts `<script
    src="/__shuyi/dev.js"></script>` right after `<head>` in what it
    sends. The file on disk is untouched;
  - `crates/cli/web/dev.js` is embedded with `include_str!` and served
    at `/__shuyi/dev.js`, with the current build number written in for
    `%BUILD%`. It is not in `SHELL`, so neither `shuyi web` nor 3004's
    export ever writes it;
  - it is one function expression run at once that declares no global
    (the bundle's strict-mode globals; the note in `index.html`), and
    it runs before every other script. It keeps the `fetch` it finds
    then and sends with that one, so a later wrapper of `fetch` (3052
    wraps it) never sees its requests;
  - **reload:** it asks `GET /__shuyi/build` every 500 ms. When the
    number differs from its own, it calls `location.reload()`. While the
    server is gone, it asks every 2 s and logs nothing;
  - **the log:** it wraps `console.debug`, `log`, `info`, `warn` and
    `error`, each still calling the one it wrapped (so the console and
    3052's wrapper keep working), and listens to `window`'s `error`
    and `unhandledrejection`. Each becomes `{ level, source, text }`,
    with `source` one of `console`, `error` and `rejection`; an `Error`
    gives its name, message and stack. It cuts each text at 4 KiB.
    Lines are queued and sent as one JSON array by
    `fetch("/__shuyi/log", { method: "POST", keepalive: true })` every
    250 ms, each body under 60 KiB (a `keepalive` body may not pass
    64 KiB), so a long queue goes in several. Past 500 queued lines it
    drops lines and sends their count;
  - on load it sends `{ source: "page" }` with its build number;
  - its own failures are neither logged nor sent (no loop).
- **The log endpoint:** a body over 64 KiB gets 413. Each text is cut at
  4 KiB. Control characters other than tab and newline become `?`, so
  a page can't send escape sequences to the terminal.
- **What it prints**, on stdout:
  - by default, one line each: `serving dist/web at
    http://127.0.0.1:4000/`, `build 2: ok (… bytes)` or `build 3
    failed: …`, `page: build 2 loaded`, `page error: …`;
  - with `--json`, one JSON object per line and nothing else on stdout
    (cargo's own messages stay on stderr, as with `shuyi web`), from
    one `serde` enum tagged by `event`:
    - `{"event":"serving","url":…,"lan_url":…,"folder":…}`;
    - `{"event":"build","build":2,"ok":true,"summary":…}`, or with
      `"ok":false` and `"error":…`;
    - `{"event":"page","build":2}`;
    - `{"event":"log","build":2,"level":"error","source":"console","text":…}`;
    - `{"event":"dropped","count":…}`;
  - so an agent starts it in the background, waits for `serving`,
    edits, waits for `page` with the new build, reads the `log` lines,
    and stops it by ending the process.
- **What it watches:** the folder of every package `cargo metadata
  --format-version 1` lists with no `source` (the workspace's members
  and its path dependencies, so an engine checkout a game builds
  against by path counts), recursively, and the project's `shuyi.ron`.
  It reads the list once, after the first build. It skips the target
  folder (`web::target_dir`), any folder holding a `CACHEDIR.TAG` (cargo
  writes one in every target folder), `web.dist`, and names that start
  with `.`, so a build's own writes never start a build. If `cargo
  metadata` fails, it watches the project's root. Modification times
  come from `fs::metadata` behind a small trait, so tests use fake
  times. No `notify`-style crate: polling, as 2220 does.
- **`--open`:** after the first good build it opens the URL in the
  default browser: `rundll32 url.dll,FileProtocolHandler <url>` on
  Windows (no shell, so the `&` and `#` of 2522's hash pass through),
  `open` on macOS, `xdg-open` elsewhere. If that fails, it prints the
  URL and goes on.
- **`--lan`:**
  - it finds the machine's LAN address with `std` alone: a `UdpSocket`
    bound to `0.0.0.0:0` and connected to `192.0.2.1:9`, a
    documentation address (connecting sends no packet), then
    `local_addr`. It prints `http://<address>:P/`. If that fails, it
    says to look the address up in the system's network settings;
  - it prints a warning: while it runs, any machine on this network can
    open the game's files and send lines to this terminal. A firewall
    may ask, or need a rule (Windows asks the first time);
  - it is never the default.
- **Either order:**
  - **with 2522:** if it is done, a build with its `link` feature on
    (`--link`) also starts 2522's bridge in this process, and the URL
    printed and opened carries its hash
    (`#link=127.0.0.1:<port>&token=<token>`). With `--lan`, the bridge
    stays on loopback, and a printed note says a page opened from
    another machine has no link. If not, 2522 adds this (its text says
    so);
  - **with 3052:** if it is done, `dev.js` also watches `<html>`'s
    `data-shuyi` with a `MutationObserver` and sends each state, which
    prints as `{"event":"state","build":2,"state":"running"}` (with
    `"error":<code>` from `data-shuyi-error` under `error`), and 3052's
    `file` message names `shuyi web --serve`, as its table says. If not,
    3052 does both (its own clause says so);
  - **with 3004:** if it is done, this ticket gives its `web` exporter
    the run step (`shuyi export web --run`, on the `Exporter` trait's
    `run`): it serves the exported folder through this server on
    `127.0.0.1` with `dev` off, so no dev script and no watching, and
    prints the URL. If not, 3004 adds it (its own clause says so);
  - **from 2802 (done):** where the book tells the reader to serve the
    web build's folder (chapter 9, `site/content/learn/book/targets-and-export.md`,
    *The web build*), it names `cargo shuyi web --serve --open`.
    **Either order with 2777:** if 2777 is done, the quick start's
    `cargo shuyi web` step on a new game becomes `cargo shuyi web
    --serve --open` too; if not, nothing to do (2777 writes it so);
  - **with 2803** (in the MVP's list in `docs/ROADMAP.md`, so most
    likely done): where the tutorial's `10-ship` step tells the reader
    to serve the folder, it names `cargo shuyi web --serve --open`. If
    not, it is written with it (its own clause says so).
- **Docs:**
  - `README.md:181` ("serve that folder over HTTP"): build, serve and
    open in one step with `cargo shuyi web --serve --open`;
  - `crates/cli/web/README.md`: *Building and running* (line 120) names
    `--serve` where it says to serve the folder (line 130), and a new
    section after it, "Serving while you work", gives the flags, the
    events, the reload, `dev.js` and why it is never in the folder, and
    the Host, Origin and `--lan` rules;
  - `examples/hello-grid/README.md:14`: a line for `cargo shuyi web
    --serve --open`;
  - `web.rs`'s module comment.
- **For the game:** no API change. `cargo shuyi web --serve` works on
  its `web` part at its next pin bump.
- **Size.** If the PR passes about 600 changed lines (fixtures and
  snapshots not counted), `--lan` and `--open` go to a follow-up ticket
  (`write-ticket`), After MVP, blocked by this one, and the PR says so.
  The PR then points the lines that need `--lan` at that follow-up:
  2407's QR code (its third ticket's blocker) and 2522's Out.
  If it is still over, the either-order items with 2522 and 3004 go to
  that follow-up too, and its text takes over this ticket's side of
  their clauses.

**Out (do not do):**
- a QR code of the LAN address: it is for phones, whose browsers are
  parked (2407 plans it in its `web-touch` ticket);
- running on any other target: `shuyi export <target> --run` and the
  desktop's run (3004), the Playdate's Simulator (3010) and device
  (3011), and a Steam Deck over SSH (3013), each in its own ticket;
- the `run` contract op: it is headless (`docs/contract.md`), and this
  ticket gives it no second meaning;
- an MCP tool: MCP answers one request at a time, and an agent runs
  `--json` in the background;
- keeping the game's state across a reload, and reloading code without
  one (2905); assets reloaded in a running page (2220 is the desktop's);
- HTTPS, HTTP/2, range requests, compression and folder listings;
- watching with the system's file events (a follow-up if polling costs
  too much, as in 2220);
- 2019's CI check, which serves with Python's `http.server`;
- a `web` part in the templates (2777, which also changes the quick
  start's web step to `--serve --open` if this ticket is done first);
- phones' browsers (2407, parked), and Android and iOS apps (3015,
  parked);
- uploads to a store or a host (3050) and signed packages (3051,
  parked).

## Implementation steps

1. Read `crates/cli/src/web.rs` and its tests, `fn web` in
   `crates/cli/src/lib.rs`, `crates/cli/web/index.html` and
   `crates/cli/web/README.md`. Check whether 2522, 3004, 3052, 2802 and
   2803 are done, and read `package` as 3039 left it.
2. Add the flags and the feature-name check, with their tests.
3. Write the server: requests, files, types, Host and Origin, the
   `dev` switch, the injection, and the two endpoints. Test it on port
   0 over a temporary folder.
4. Write `dev.js`.
5. Write the watcher, the loop over the builder trait, then the real
   builder and the lock.
6. Add the events, `--json`, `--open` and `--lan`. List the function
   that opens the browser, the one that finds the LAN address and the
   real builder in `.cargo/mutants.toml`'s `exclude_re`, each with its
   reason, as `<impl WasmOpt for RealWasmOpt>::optimize` is.
7. The either-order items and the docs.
8. Try it on Hello, Grid in the browser pane, and run the gates. If the
   pane is hidden and the page draws no frames, drive them with
   `wasm_exports.frame()` from the pane's JavaScript tool.

## Acceptance criteria

- [ ] In `examples/hello-grid`, `cargo shuyi web --serve --open` builds,
      prints `http://127.0.0.1:4000/` and opens the game (if **Size**
      moved `--open` out: `--serve`, and the URL opened by hand). A changed
      colour in `files/palette.ron` shows in the page after the rebuild
      with no reload by hand (in the browser pane; screenshots before
      and after in the PR).
- [ ] With `--json`, stdout holds only JSON lines. `console.error("x")`
      run in the page gives a `log` event with `"level":"error"` and
      `"text":"x"`. An `Error` thrown in a `setTimeout`, and a
      dispatched `ErrorEvent` carrying a `WebAssembly.RuntimeError`,
      give `"source":"error"` (by hand; the lines in the PR).
- [ ] After `--serve` builds, every file in `web.dist` is byte for byte
      what `shuyi web` writes, and none contains `__shuyi` (test).
- [ ] The server answers 404 to `/../Cargo.toml`, `/%2e%2e/Cargo.toml`
      and `/..\Cargo.toml`; 403 to a `Host` of `example.com:P`, with
      and without `--lan`; 403 to a `POST` from another `Origin` or with
      none; 413 to a 65 KiB body; and 405 to `PUT` (tests).
- [ ] With `dev` on, the served `index.html` has the dev script once,
      right after `<head>`; with it off, it is the file on disk and
      `/__shuyi/build` is 404. A `.wasm` comes as `application/wasm`
      (test).
- [ ] With a fake builder, a good build bumps `/__shuyi/build`, and a
      failed one keeps the number and the files and prints the error
      (test).
- [ ] With fake file times, a change in a member's folder starts a
      build, and a change in the target folder, a folder holding a
      `CACHEDIR.TAG`, `web.dist` or a dot-folder doesn't (test).
- [ ] A log line holding an escape character prints with `?` in its
      place (test).
- [ ] `--lan` prints the LAN address and the warning, and a page opened
      by that address loads, from a second machine or, failing one,
      from this one (by hand; the PR says which).
- [ ] On Windows, stopping `cargo shuyi web --serve` leaves no `shuyi`
      process holding the port (by hand; Completion notes). If one is
      left, the Completion notes say so and a follow-up ticket is
      written.
- [ ] `Cargo.lock` gains no package.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the flags; the feature-name check; request parsing and its
  limits; path resolution; types; the Host rule with and without
  `--lan`, and the Origin rule; the injection and the `dev` switch;
  the log body (levels, the cut, control characters, the
  size); each event's JSON line (one snapshot each); the watcher's
  skips and changes with fake times; the LAN address's fallback text.
- Integration (`std` alone): the server on port 0 in a test thread over
  a temporary folder, with a `TcpStream` client: the page, the wasm,
  the refusals, `/__shuyi/build` before and after a fake build, and a
  `POST /__shuyi/log` arriving as events through a channel.
- A test that `SHELL` has no `dev.js`, and that `dev.js` uses the
  endpoint paths the server defines (one Rust constant each, found in
  the file's text).
- Manual: the browser pane, as in the first two criteria; `--lan`;
  stopping on Windows.

## Completion notes

*(Filled in by the session that completes the ticket.)*
