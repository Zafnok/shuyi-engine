---
id: "2616"
title: "shuyi-server: a headless library that runs a director's rooms over WebSocket, with TLS, limits, health and a cost guard"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: todo
blocked_by: ["2615"]
nick_input: setup
completed:
---

# 2616 — shuyi-server: rooms over WebSocket, headless

## Context

- Nick's decision of 2026-11-25 (`docs/decisions.md`): trails is played
  online, in up to 10 rooms on an always-free host.
- ADR-0155 (2613) makes the server a **headless platform backend**
  (ADR-0008 §2): tokio, sockets and the clock, never macroquad, never
  the rules. The rooms (2614) and the director (2615) are sans-IO in
  `shuyi-net`; this ticket is the I/O around them.
- 1146 (parked) planned `shuyi-server` as a turn-game referee binary.
  This ticket creates the crate for tick rooms instead, as a library;
  1146 is amended to add a referee room kind to it later. 1141 (parked)
  is amended to run its relay on this crate's listener.
- **The host** (2618's ADR): Oracle Cloud Always Free, on an
  A1.Flex if capacity allows, otherwise an E2.1.Micro (x86_64, 1 GB of
  RAM, 1/8 OCPU burstable, 50 Mbps). So the server is built to fit the
  Micro: **under 256 MB** of memory (2618's systemd unit sets
  `MemoryMax=384M`). TLS is mandatory in production: the web client is
  served over HTTPS (GitHub Pages), and browsers block a plain `ws://`
  from such a page. The certificate is a Let's Encrypt IP certificate
  on the 6-day shortlived profile, renewed by certbot, whose deploy hook
  copies the PEM files and runs `systemctl reload` (SIGHUP).
- **The exposure:** the server is reached by a raw IP, so its own limits
  on connections, addresses and messages are the defence (2618's caveat
  (i)).
- What exists, checked on 2026-11-25: no tokio, tungstenite or rustls
  in `Cargo.lock`; `deny.toml` allows MIT, Apache-2.0, ISC, BSD and the
  rest (ring is Apache-2.0 AND ISC; rustls and tokio-rustls are
  Apache-2.0/ISC/MIT; tokio and tokio-tungstenite are MIT), `[bans]
  deny` is empty, and `[graph]` checks default features only.
  `docs/modules.md`'s *Platform backends* table lists `shuyi-app` and
  `shuyi-playdate`; `cargo xtask check-modules` asks a backend with a
  row there for its `README.md`.
- `grep -rnw` over `tickets/open`: `shuyi-server` is named by 1146
  (which created it), 2550 (`--via referee` says its binary isn't tried)
  and 2606 (its `Accepts` list); all three are parked and amended or
  read against this crate. `ServerConfig` and `ServerError` are named by
  no other ticket.

## Nick input

**Setup:** hold the name `shuyi-server` on crates.io as an empty 0.0.0
placeholder before this PR merges: CI's `layers` job runs `cargo xtask
check-publish --index`, which fails an engine crate whose name isn't
held. Nick decided on 2026-11-26 (`docs/decisions.md`) to hold it when
this ticket makes the crate real; ask him with the `ask-nick` skill when
the PR opens. Running a public instance is 2618's, with Nick's account
steps.

## Scope

**In:** a library crate `crates/server` (package `shuyi-server`):
`pub fn run<G: RoomGame>(config: ServerConfig, game: G) ->
Result<(), ServerError>`, which blocks until shutdown. Games own their
binary (2789's `trails-server`); the library parses no command line.

- **Runtime:** tokio with a `current_thread` runtime, features `rt`,
  `net`, `time`, `io-util`, `macros`, `sync`, and `signal` on Unix. One
  task owns the `Director` (2615); each connection is a task joined to
  it by bounded `tokio::sync::mpsc` channels.
- **One port, two kinds of request.** The accept loop reads the request
  head (at most 4 KiB, 5 s) with `httparse` (MIT/Apache-2.0, already a
  dependency of tungstenite):
  - `GET /play` with an upgrade: the server answers `101` itself
    (`tungstenite::handshake::derive_accept_key`) and wraps the stream
    with `tokio_tungstenite::WebSocketStream::from_raw_socket(stream,
    Role::Server, Some(config))`, with `max_message_size` and
    `max_frame_size` set to `max_msg_bytes`;
  - `GET /healthz`: `200 ok`, or `503` with the reason `budget` or
    `shutting-down`;
  - `GET /stats`: JSON (serde_json): 2615's `DirectorStats`, the uptime
    in seconds, the engine version and the game's `Made`, bytes out this
    month, the egress budget, the steps skipped by the loop, the
    connection count, and the time of the last certificate load (unix
    seconds). 2619 reads it and documents nothing of its own: these
    field names are the contract, written in the crate's docs;
  - anything else: `404`, then close.
- **WebSocket:** `tokio-tungstenite` with `default-features = false`
  (features `handshake` only). Binary messages are 2614's and 2615's
  wire frames; a text message is a protocol error and closes.
- **TLS**, optional (`tls: Option<TlsConfig>` with certificate and key
  PEM paths):
  - `tokio-rustls` with `default-features = false` and rustls with
    `default-features = false, features = ["ring", "std", "tls12"]`.
    Never aws-lc-rs (it needs NASM on the GNU toolchain), never
    `webpki-roots` or `webpki-root-certs` (CDLA-Permissive-2.0, not on
    the allow list). `deny.toml`'s `[bans] deny` lists `aws-lc-rs`,
    `aws-lc-sys`, `webpki-roots` and `webpki-root-certs`, so a later
    dependency can't bring them back.
    **Either order with 2617:** if 2617 is done, the four entries are
    there already: keep them. If not, add them here.
  - Plain without it: for local development, the tests, and running
    behind a Cloudflare Tunnel (2618's fallback).
  - The certificate and key are read with `rustls-pki-types`' PEM
    reader and swapped into a `ResolvesServerCert` on SIGHUP (Unix
    only; on Windows only at start). A file that doesn't parse keeps the
    old certificate and logs one line.
- **The tick loop:** a `tokio::time::interval` at `G::STEPS_PER_SECOND`
  with `MissedTickBehavior::Skip`. Each wake it computes how many steps
  are due since the start, runs at most 3, and counts the rest as
  skipped (in `/stats`). Before each step it hands the director the
  `NetEvent`s from the connection tasks; after it, it sends the outbox's
  frames and closes the outbox's `drop` peers. The transport the
  director sees is a `shuyi_net::Transport` implemented over the
  connection channels, so the director never knows about sockets.
- **Each connection:** an outbound queue capped at `queue_max` frames
  (default 64); a client whose queue is full is cut and its seat becomes
  a bot (`Disconnected { reason: TimedOut }`). A WebSocket ping every 5
  s, and a 15 s timeout without a pong or a message. Idle sockets keep
  the 30 s ping a proxy such as Cloudflare needs.
- **Limits**, in `ServerConfig` (`#[non_exhaustive]`, `ServerConfig::new`
  and `with_*` setters, ADR-0039), read from RON with
  `ServerConfig::from_ron(&str)`. The file's name and path are the
  game's (2618 uses `trails-server.ron`, kept apart from the client's
  `server.ron`):
  - `listen` (address and port), `tls`;
  - `max_conns` 200; `per_ip` 4; `max_msg_bytes` 512;
  - the director's settings (2615's `DirectorConfig`: `rooms_max` 10,
    `rooms_warm` 1, messages a second, and the rest), passed through;
  - `behind_proxy: bool` (default false): when true, `per_ip` counts the
    first address in `CF-Connecting-IP` or `X-Forwarded-For` instead of
    the socket's, which is the tunnel's own;
  - addresses are kept in memory only, for the count, and never logged
    or written.
- **The cost guard:**
  - an egress counter of every byte the server writes, plus a fixed 74 B
    a message for the TLS record and TCP/IP headers (ADR-0155's
    numbers), so it tracks what the host bills;
  - saved to a state file (`state_dir` in the config; 2618's
    `StateDirectory=trails-server`) every minute and at shutdown, and
    reset at the start of each calendar month, UTC. The month is
    computed from unix seconds by a pure function (days from civil);
    the clock is read in the I/O layer and passed in, never read in the
    logic;
  - `monthly_egress_budget`, default 1 TB (Oracle's free egress is 10
    TB): past it, new players get 2615's `Busy`, `/healthz` is `503
    budget`, and the players already in keep playing. If 2615's
    director has no switch to refuse `Play`, add
    `Director::set_open(bool)` to it here.
- **SIGTERM** and Ctrl-C: send 2614's `Closing` frame to every client,
  flush for up to 2 s, save the state file, exit 0. `/healthz` says
  `shutting-down` meanwhile.
- **Logs:** to stdout (journald), through the `log` facade with a small
  stdout logger of our own: start, stop, certificate loads, and a count
  line a minute (connections, rooms, humans, bytes). Never an address,
  never a message's content.
- **The layers:** `tools/layers.ron` gains `"shuyi-server": (engine:
  ["shuyi-core", "shuyi-net"], outside: Any, devices: false, floats:
  false)`. `check-layers` then refuses an edge to `shuyi-app`,
  `shuyi-ui` or macroquad.
- **The catalog:** a row in `docs/modules.md`'s *Platform backends*
  table ("`shuyi-server` | a game's rooms on a server, headless (tokio)
  | ready | 2616") and `crates/server/README.md`, as `check-modules`
  asks of a backend.
- **Rule 13:** this crate never reaches a game's client build. A game
  depends on it only from its own `server` feature, off by default
  (2789); the crate's README says so.
- **`cargo deny`:** `shuyi-server` is a workspace member with its
  dependencies on by default, so `[graph]` sees them; check that
  `cargo deny check` lists rustls, ring and tokio and passes.
- The completion notes get a `Book:` line (2856's hosting page and
  2854's chapter) and a `Skills:` line (2855).

**Out (do not do):**
- Game binaries and the trails rules: 2789.
- Deployment, certbot, systemd and the firewall: 2618. Watching it from
  outside: 2619.
- The native client: 2617. The web client: 1142.
- The relay with room codes: 1141. A referee room kind for turn games:
  1146.
- HTTP/2, UDP, WebTransport, compression (permessage-deflate).
- Reading a certificate's expiry: 2619 reads it from the TLS handshake.

**Estimated size:** about 1,300 lines (code 750, tests 450, docs 100).

**Split:** past 1,000 lines, split steps 5 and 6 (TLS with its reload,
and the cost guard with its state file) into a new ticket written then
with the `write-ticket` skill, blocked by this one, and add it to the
`blocked_by` of 2618. This ticket keeps plain WebSocket, `/healthz` and
`/stats`, the limits, the loop and the signals.

## Implementation steps

1. `crates/server`: `Cargo.toml` (the dependencies above with their
   features audited, `version.workspace = true`), the `layers.ron`
   entry, the catalog row and the README; `ServerConfig` and its RON
   parse.
2. The accept loop: read the head, route to `/play`, `/healthz`,
   `/stats` or `404`.
3. Connection tasks, their channels, the `Transport` over them, and the
   director task with the tick loop.
4. Limits: `max_conns`, `per_ip` (with `behind_proxy`), message size,
   the outbound cap, pings and timeouts.
5. TLS: load, serve, reload on SIGHUP; the `[bans]` entries.
6. The cost guard: the counter, the state file, the monthly reset, the
   refusal.
7. SIGTERM and the `Closing` frame; the logs.
8. The tests below.
9. The `Book:` and `Skills:` lines.

## Acceptance criteria

- [ ] Two test clients (`tokio-tungstenite` as a dev-dependency)
      against a server of `room::testing::Dots` on `127.0.0.1:0` are
      seated in the same room and see equal hashes at every check over
      300 steps.
- [ ] Closing one client's socket makes its seat a bot at the next step
      (read from the other client's `Roster`).
- [ ] The 5th connection from one address is refused; the 201st
      connection overall is refused.
- [ ] A message over `max_msg_bytes` and a peer over the message budget
      are cut, and the others play on.
- [ ] A client that stops reading is cut once its queue is full, and its
      seat becomes a bot.
- [ ] `/healthz` is `200 ok`; `/stats` parses and holds the fields the
      docs list; an unknown path is `404`.
- [ ] A TLS handshake works with an `rcgen` self-signed certificate
      (`rcgen` as a dev-dependency, MIT/Apache-2.0, on ring), and on
      Unix a SIGHUP after rewriting the files serves the new
      certificate to the next connection.
- [ ] With the egress budget set below what was sent, `Play` gets
      `Busy` and `/healthz` is `503 budget`.
- [ ] The state file survives a restart, and a new month (an injected
      time) resets it.
- [ ] SIGTERM sends `Closing` to every client and exits 0 (Unix).
- [ ] On Linux, resident memory stays under 256 MB with 200 idle
      connections and 10 warm rooms (read from `/proc/self/status`).
- [ ] `cargo tree -p shuyi-server -i aws-lc-rs` and `-i webpki-roots`
      find nothing; `cargo deny check` passes with the new `[bans]`.
- [ ] `cargo xtask check-layers` and `check-modules` pass.
- [ ] It builds and its tests pass on Linux and Windows GNU; the signal
      tests run on Unix only.
- [ ] The completion notes have a `Book:` line naming 2854 and 2856 and
      a `Skills:` line naming 2855.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `ServerConfig` parse (defaults, every field, an unknown field
  refused); the month function over leap years and the turn of a year;
  the egress counter's arithmetic; the request-head router.
- Integration, in `crates/server/tests/`: one test per criterion above,
  each on `127.0.0.1:0`.
- Memory: the 200-connection test on Linux only.

## Completion notes
