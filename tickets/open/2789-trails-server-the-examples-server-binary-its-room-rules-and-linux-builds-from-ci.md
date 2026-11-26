---
id: "2789"
title: "trails-server: the example's room rules, its server binary, and Linux builds for x86_64 and Arm from CI"
type: feature
milestone: "Engine: examples"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2788", "2616"]
nick_input: none
completed:
---

# 2789 — trails-server: the room rules, the binary and its Linux builds

## Context

Nick's decision of 2026-11-25 (`docs/decisions.md`) puts trails (2788)
online: rooms of eight seats, bots in the empty ones, players dropping
in and out, matched by a black-box director, hosted for free.

- **The engine half** is 2613–2616: `shuyi-net`'s `RoomGame` trait and
  sans-IO `Room` (2614), the quick-play `Director` (2615), and
  `shuyi-server` (2616), a headless **library** that serves a director's
  rooms over WebSocket (`/play`, `/healthz`, `/stats`) through
  `shuyi_server::run(config, game)`. ADR-0154 (2613) gives the sync
  model: the server steps the real `TickSim` and sends each step's
  inputs.
- **Each game owns its server binary.** So trails gives one:
  `trails-server`, which links trails' pure rules and `shuyi-server`,
  and never the window, audio or `shuyi-app`.
- **The free host** (2618) is Oracle Cloud's Always Free tier: an A1
  (Arm, `aarch64`) if Oracle has capacity, else an E2.1.Micro
  (`x86_64`, 1 GB). So the binary is built for both.
- **Nick's machine is Windows with the GNU toolchain**, so the Linux
  release binaries come from CI. GitHub's `ubuntu-24.04-arm` runners
  are free for public repositories, and `export.yml` already uses one
  (`linux-arm64`).
- `ci.yml` runs on pull requests and pushes to `main` only, not on
  tags.

2788's `Round` is the room's game: one step moves every rider one cell,
at 15 steps a second (2615: "trails runs 15 steps a second"), and
`bots::bot_input(&Round, seat, &mut SimRng)` is the bot.

## Nick input

None.

## Scope

**In:** in `examples/trails`.

- **Features** in `Cargo.toml`:
  - `room = ["shuyi/net"]`: the room rules (the `shuyi` front door's
    `net` feature, 2613);
  - `server = ["room", "dep:shuyi-server"]`, which never turns on
    `window`.
  - `default` stays `["window"]`; `room` and `server` are off in it.
- **`src/room.rs`** (behind `room`), pure: `pub struct Trails` (the
  arena and tuning, loaded once) and `impl RoomGame for Trails`, in the
  shape 2614 gives the trait:
  - `type Sim = Round`;
  - `SEATS = 8`;
  - `STEPS_PER_SECOND = 15`, `MAX_ROUND_STEPS = 2700` and
    `PAUSE_STEPS = 45`: consts, so a unit test checks each equals
    `tuning.ron`'s `moves_per_second`, `max_round_steps` and
    `pause_steps`, and fails naming the one that drifted;
  - `new_round(seed, round)` is `Round::new` from the arena, the tuning
    and the seed;
  - `round_winner` maps `Round::winner` onto 2614's `RoundEnd`
    (`Going`, `Won(seat)`, `Draw`);
  - `seat_alive` is whether that seat's rider still rides;
  - `bot_input` delegates to `bots::bot_input`;
  - `made()` is trails' `made()`, as snake's `lib.rs` builds one, so a
    client of another build is refused (2615's `OtherBuild`).
- **`src/bin/trails-server.rs`**, `[[bin]] required-features =
  ["server"]`:
  - `--config <path>` reads the server's settings as RON, in the type
    2616 gives (address, TLS paths, the director's caps, the limits, the
    egress budget); with no `--config`, the dev default: `127.0.0.1:7777`,
    plain WebSocket, 2616's default caps;
  - it builds `Trails` from the embedded `files/` and calls
    `shuyi_server::run(config, Trails)`;
  - `--version` prints the `Made`.
- **`examples/trails/server.ron`**: the dev config, commented field by
  field. The production config is 2618's.
- **A dependency-tree test** (`tests/server_deps.rs`, or an xtask check
  if a test can't run `cargo tree`): `cargo tree -p trails
  --no-default-features --features server -e normal` lists no
  `macroquad`, `miniquad`, `shuyi-app`, `quad-snd` or `shuyi-sound-out`.
- **CI, in `ci.yml`, a job `trails-server`**, a matrix of
  `x86_64-unknown-linux-gnu` on `ubuntu-24.04` and
  `aarch64-unknown-linux-gnu` on `ubuntu-24.04-arm`:
  - `cargo build --release --locked -p trails --no-default-features
    --features server --bin trails-server`;
  - strips the binary, prints its size to the step summary;
  - uploads `trails-server-x86_64` and `trails-server-aarch64` as
    artifacts (2618 fetches them by artifact id);
  - runs on changes under `crates/net`, `crates/server`, `crates/core`
    and `examples/trails` (through `changes.yml`'s outputs), and joins
    `ci-result`'s needs;
  - `permissions: contents: read`, pinned actions, zizmor-clean.
- **A release workflow, `.github/workflows/trails-server-release.yml`**,
  on a pushed tag `trails-server-v*`: the same two builds, attached to a
  GitHub release of that tag. `contents: write` in that job only,
  `persist-credentials: false`, zizmor-clean. CI never deploys (2618:
  no SSH key goes into GitHub's secrets).
- **README**, a section "Run a server": `cargo run -p trails
  --no-default-features --features server --bin trails-server`, what it
  listens on, and that the online client is 2790's.
- `Book:` and `Skills:` lines in the completion notes (2854's chapter
  and 2855's skill teach this).

**Out (do not do):**
- The online client and its screens (2790).
- Deploying, the host, TLS certificates and the production config
  (2618).
- The load test at ten rooms (2791).
- Docker or other container images (a later ticket, if wanted).
- A change to trails' rules beyond what `RoomGame` needs: a rule change
  found here goes back to 2788's code with a test.

**Estimated size:** about 500 lines (code 200, tests 180, CI and data
120).

## Implementation steps

1. Read 2614's `RoomGame`, `Room` and wire, 2615's `Director`, 2616's
   `run` and its config type, and 2788's `Round`, `bots.rs` and
   `made()`.
2. The features and `src/room.rs`, with unit tests.
3. The binary, its `--config`, and `server.ron`.
4. The dependency-tree test.
5. The integration tests (below).
6. The CI job, the release workflow, zizmor.
7. The README section and the completion notes.

## Acceptance criteria

- [ ] A `Room<Trails>` (2614) of eight bots plays 10 rounds with a
      golden hash in `tests/golden/room.ron`, and the rounds it plays
      are the ones `Round` plays from the same seeds.
- [ ] An integration test starts `shuyi_server::run` on `127.0.0.1:0`
      with `Trails`, connects a WebSocket test client that sends
      `Hello` and `Play`, follows 300 steps through `RoomClient`, and
      every hash the server sends matches the client's sim; it runs on
      Linux and on Windows GNU.
- [ ] A client whose `Made` differs is refused with `OtherBuild`.
- [ ] `cargo tree` of the server build lists no window, audio or
      `shuyi-app` crate.
- [ ] CI builds and uploads both Linux artifacts on a pull request that
      touches `examples/trails`.
- [ ] `cargo run -p trails` (the window) builds and plays as before.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`src/room.rs`): the step rate agrees with the tuning; a round
  from a seed equals `Round::new` from it; `bot_input` through the
  trait equals `bots::bot_input`.
- Golden: `tests/room.rs`, eight bots, 10 rounds.
- Integration: `tests/server.rs`, a WebSocket client against `run` on
  `127.0.0.1:0` (a dev-dependency on the WebSocket client crate 2616
  already uses, so no new crate enters the lock; name its license as
  2616 checked it).
- The dependency-tree check.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
