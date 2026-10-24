---
id: "2529"
title: "Parked: Several peers in one headless run: shuyi peers with network faults, a desync report, and MCP tools per peer"
type: feature
milestone: "Engine: networking"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1140", "1133"]
nick_input: none
completed:
---

# 2529 — Several peers in one headless run

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need): the game doesn't ask for it, so this waits until after the MVP (`docs/ROADMAP.md`) and for 1140, the networking work it builds on. If the game's ticket 0053 picks an online mode, it is unparked with 1140. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Each of the big three lets a developer run several
copies of a networked game at once, make the network worse on purpose,
and watch what happens:

- Godot 4.7: Debug > Customize Run Instances starts several copies,
  each with its own arguments; the debugger has a network profiler.
- Unity 6: Multiplayer Play Mode runs virtual players beside the
  editor
  (`docs.unity3d.com/Packages/com.unity.multiplayer.playmode@latest/`),
  and the Multiplayer Tools' network simulator adds delay and loss.
- Unreal 5.8: Play In Editor runs a server and several clients with
  network emulation (lag, loss); Gauntlet runs sessions of several
  processes; Networking Insights shows the traffic.

What Shuyi has today:

- The tool contract runs **one** game. `contract::session::Session`
  (`crates/cli/src/contract/session.rs`) is one game headless;
  `Headless<G>` drives one `Harness<G>`. ADR-0022 §3: "The MCP server
  (1133) keeps one between calls." No operation in `docs/contract.md`
  starts more than one game.
- `replay::first_difference` (`crates/cli/src/contract/replay.rs`)
  finds the first step where two runs' state hashes differ, for
  records. Nothing compares live peers.

What is planned:

- 1140 (parked): `MemoryNetwork` with seeded loss, delay, reordering
  and duplicates, `Transport::stats()`, and `LockstepSession` with
  desync detection. Only Rust tests reach them.
- 1145 (parked): two Harness `hello-grid` games and a relay in one
  process, as a Rust integration test. ADR-0006's Consequences: "Two
  Harness games play a match against each other in CI."
- 1133: an MCP server over stdio whose session keeps "one headless
  game running".
- 1134: the `shuyi-net` skill waits for 1140, this ticket and 1145:
  before this ticket, it has nothing an agent can run.

So once networking lands, an agent asked for an online turn game cannot
play both sides, add loss, or see a desync without writing Rust.

Why it matters:

- **The game first.** Visions of Shuyi's ticket 0053 (game#0053)
  decides whether it gets an online mode. If it does, ADR-0006 makes it
  lockstep plus a relay, and the first check of that mode is two copies
  playing each other over a bad network. This ticket makes that one
  command.
- **Other 2D games.** Any turn game played online (a card battle, a
  board game, tactics) gets the same check.
- **The end goal** (`docs/decisions.md`, 2026-10-19): the most
  AI-friendly 2D engine. An agent drives multiplayer through the same
  contract and MCP tools it already uses.

This is parked with networking. It is unparked with 1140 or right
after it. It is best done before 1145, so 1145's online mode and the
`shuyi-net` skill can use it, but either order works (the `hello-grid`
bullet below).

### The design, in short

- **A new operation, `peers`.** `run`'s answer is one game's
  (`RunOutput`), and each operation has one input and one answer
  schema (`contract::schemas()` pairs them per `Op`). So several peers
  are an operation of their own whose answer holds one `RunOutput` per
  peer. Whatever `run`'s answer carries (3040's log lines, 2356's
  command events), each peer's answer carries too.
- **`shuyi-cli` never depends on `shuyi-net`.** ADR-0009 §4: "No
  default crate depends on a module." The contract defines plain data
  (`NetConditions`, `NetStats`) and a trait, `Peers`. `shuyi-net`
  implements it behind its own `tools` feature (ADR-0009 §4: a part
  that pulls in a heavy layer sits behind the module's own feature), the
  way `hello-grid`'s `tools` feature pulls in `shuyi-cli` today
  (`examples/hello-grid/Cargo.toml`). A game turns `shuyi-net/tools` on
  only from its own `tools` feature, so `shuyi-cli` is never linked into
  a game's binary (ADR-0002 §1).
- **Peers run in rounds.** The peer whose step it is plays one key,
  one button group or one frame of waiting. Every other peer then runs
  as many frames with no input, and the network moves on that many
  steps. Peers stay within two frames of each other, and the same input
  gives the same answer every time.
- **A desync reuses `replay`'s form.** Each peer's turn hashes are
  compared with peer 0's over the turns both have applied (both lists
  cut to the shorter), by the comparison `replay::first_difference`
  makes. The answer names the peer, the first differing turn and both
  hashes.

## Nick input

None.

## Scope

**In:**
- **`replay::first_differing(a: &[String], b: &[String]) ->
  Option<Difference>`**, factored out of `replay::first_difference`,
  which calls it with each run's step hashes. Its tests stay as they
  are.
- **`crates/cli/src/contract/peers.rs`**, the operation:
  - `PeersInput { start: Vec<GameArg>, peers: u32, net: NetConditions,
    steps: Vec<PeerStep>, frames: u32 }`, with `serde(default)` and
    `deny_unknown_fields` as `RunInput` has. Every peer starts with the
    same `start` options.
  - `PeerStep { peer: u32, step: Step }` (`frame_png::Step`).
  - `NetConditions { seed: u64, loss: u8, delay: u16, reorder: u8,
    duplicate: u8 }`: `loss`, `reorder` and `duplicate` in whole
    percent (0 to 100), `delay` in frames (one network step a frame, at
    most 600). The default is a perfect network with seed 0. No floats.
  - `NetStats`: what 1140's `Transport::stats()` gives, copied field by
    field into a contract type with `JsonSchema`. Whole numbers only.
  - `Seat { peer: u32, count: u32 }`: which peer a game is starting.
  - `PeersInput::check`: 2 to `MAX_PEERS` (8) peers, every step's
    `peer` below `peers`, every step passes `Step::check`, and `frames`
    at most `run::MAX_IDLE_FRAMES / peers`.
  - `PeersOutput { peers: Vec<PeerOutput>, desync: Option<Desync> }`;
    `PeerOutput { peer: u32, run: RunOutput, net: NetStats, turns: u32
    }`; `Desync { peer: u32, at: Difference }`, where `at.step` is the
    turn and `at.hashes` are peer 0's and that peer's, each as
    `StateHash`'s text (sixteen hex digits, as `ReplayStep::hash` has
    it).
- **The `Peers` trait**, in the same module, object-safe:
  - `count(&self) -> u32`;
  - `session(&self, peer: u32) -> &dyn Session` and
    `session_mut(&mut self, peer: u32) -> &mut dyn Session`;
  - `advance(&mut self, steps: u32)`: the network moves on;
  - `set_net(&mut self, net: &NetConditions)`: new fault rates from the
    next step on;
  - `stats(&self, peer: u32) -> NetStats`;
  - `turn_hashes(&self, peer: u32) -> Vec<StateHash>`: the state's hash
    before the first command, then after each command, in
    `replay::turns`' form, so turn numbers match `shuyi replay`'s
    steps.
- **`GameTools::peers(&self, options: &[GameArg], count: u32, net:
  &NetConditions) -> Result<Box<dyn Peers>, String>`** in
  `crates/cli/src/game.rs`, defaulting to `unsupported("peers",
  "peers")`: "the game's tools don't offer peers (GameTools::peers)".
- **The driver**, three functions, so MCP reuses the first two:
  - `peers::play(peers: &mut dyn Peers, steps: &[PeerStep], frames:
    u32)`:
    - each step is cut into pieces: a `Keys` or `Pad` step token by
      token; a `Wait` into frames of `min(left, FRAME_DT)` as
      `Harness::wait` plays them (at least one frame); a step of any
      other kind (2356's command step, if it exists) whole;
    - each piece is one round: its peer plays it (`Session::play` of a
      one-step `Script`). The frames it took are that peer's
      `frame_count` after less before, never computed from the step.
      Every other peer idles that many frames (`Session::idle`), then
      `Peers::advance` by that many;
    - then `frames` rounds of one idle frame each and `advance(1)`;
  - `peers::compare(peers: &dyn Peers) -> (Option<Desync>,
    Vec<String>)`: each peer's turn hashes against peer 0's, by
    `first_differing` over both lists cut to the shorter. The first
    peer that differs is the desync. Its limits: "peer N is K turn(s)
    behind peer 0: the turns after T weren't compared";
  - `peers::peers(game: &dyn GameTools, input: &PeersInput) ->
    Answer<PeersOutput>`: checks the input, starts the peers, plays,
    compares. It is `ok` when there is no desync. Its limits: each
    peer's `run::limits_of`, prefixed `peer N: `; `compare`'s; and
    always "the network is in memory: real sockets and their timing
    aren't tried".
- **The CLI:** `shuyi peers <n> [--net
  seed=7,loss=20,delay=3,reorder=10,duplicate=1] [--peer <i>] [steps]
  [--frames N] [the game's options] [--json]`. `--peer i` sends the
  steps after it to peer `i` (peer 0 until the first one). `--net`
  takes any of `NetConditions`' five fields, comma-separated, by the
  same names as in JSON. `peers::parse_args`, `peers::usage` and
  `peers::text` follow `run`'s (`crates/cli/src/contract/run.rs`).
- **Problem codes** (2509 is an MVP ticket, so it is done first): the
  codes `run`'s and `bots`' sites have. A refused input
  (`PeersInput::check`, `parse_args`, a bad `--net`) is `SH0001`;
  `GameTools::peers` left at its default is `SH3001`; any other error
  of the game's is `SH3002`. A desync is a verdict (`ok: false`), not
  an error, and has no code.
- **Wiring:** `Op::Peers` (`peers`) in `Op::ALL`, `Op::name`, and
  `contract::schemas()`; `cli::runs_the_game` and `cli::with_game`;
  the ops listed in `crates/cli/src/lib.rs`'s usage and in
  `contract.rs`'s module docs.
- **`FakePeers`** in `crates/cli/src/contract/fake.rs`: N
  `FakeSession`s, a count of network steps, and hashes the test sets.
- **The schemas:** `peers.input.json` and `peers.output.json`
  generated, `contract::VERSION` raised by one, and its line added to
  `crates/cli/schemas/versions.json` (`docs/contract.md`, *The
  schemas*).
- **`shuyi-net`'s `tools` feature** (`crates/net/Cargo.toml`: optional
  `shuyi-cli`, `shuyi-ui` and `shuyi-platform` with `harness`; a
  dev-dependency on `shuyi-ui` with `toy` for the integration test),
  with `shuyi_net::tools::MemoryPeers<G: GameData>`:
  - `MemoryPeers::start(count, net, peer, hashes)`, where `peer: impl
    Fn(Seat, MemoryTransport) -> Result<Headless<G>, String>` starts
    one peer's game with its transport (the game decides from the seat
    who hosts), and `hashes: impl Fn(&Harness<G>) -> Vec<StateHash>`
    reads the game's `LockstepSession` hashes (for example through
    `Harness::screen`);
  - it builds one `MemoryNetwork` from `NetConditions` (1140's fault
    settings) and implements `Peers` over it. Nothing in `tools` uses
    floats;
  - `tools/layers.ron` (2015's `check-layers`): `shuyi-net`'s `engine`
    list gains `ui`, `platform` and `cli`;
  - `crates/net/README.md` names the feature.
- **MCP, over 1133's session** (`shuyi mcp`; the last step, see *Size
  and the split*):
  - the `peers` operation is an MCP tool like every operation (1133),
    with no code of its own here;
  - `start` takes an optional `peers` (2 to 8) and `net`; with them the
    session holds the game's `Peers` instead of one `Session`. `reset`
    starts the same peers again with the same conditions;
  - `step`, `scene`, `frame_png` and `events` take an optional `peer`
    (0 when absent). `step` plays through `peers::play`, so every peer
    keeps up;
  - a new `net` tool: with a `NetConditions`, it changes the fault
    rates (`Peers::set_net`). Its `seed` must be the session's, since
    the faults' random stream keeps going; another seed is an error. It
    always answers the conditions, each peer's `NetStats` and turns,
    and `peers::compare`'s desync if there is one;
  - on a session of one game, a `peer` other than 0 and the `net` tool
    answer an error that names `start`'s `peers`. MCP's refusals are
    `SH0001`.
- **`hello-grid`, in either order with 1145:** if 1145 is done (its
  online mode exists), `examples/hello-grid/src/tools.rs` offers
  `GameTools::peers` through `MemoryPeers`, its `tools` feature turns
  on `shuyi-net/tools`, and a CI test runs `cargo shuyi peers 2 --net
  seed=1,loss=10,delay=2` on it. If 1145 isn't done, 1145 does that
  (edit requested there).
- **The `shuyi-net` skill, either order with 1134:** if 1134 is done,
  drop 2529 from the `waits_for` of `skills/shuyi-net/SKILL.md`
  (`check-skills` fails on an id no longer in `tickets/open/`), and add
  a section on `peers` (`--net`, `--peer`, the desync answer, and the
  MCP tools if step 9 keeps them) and `MemoryPeers`. If 1145 is done
  too, `waits_for` is then empty: drop it, and write the skill's worked
  example as 1134's scope says (two `hello-grid` peers through `peers`,
  with a seeded loss, which CI runs). If 1145 isn't done, 1145 writes
  it. If 1134 isn't done, nothing to do (1134 writes the skill from
  what exists).
- **Docs:**
  - `docs/contract.md`: the `peers` row in *The operations*, the
    `peers` row in *What a game gives*, and a short *Peers* section
    (rounds, the network's faults, the desync, the limits, and the MCP
    tools if step 9 adds them);
  - `docs/net.md` (1140's page): how a game offers peers, with
    `MemoryPeers`.
- **For the game:** additive. `GameTools` gains a defaulted method, so
  the game's tools still build. `Op` gains a variant and `Op::ALL`
  grows to ten, so a game that matches `contract::Op` exhaustively adds
  an arm. 2010 is done, and 2013 is an MVP ticket, so it comes first:
  the new public types follow 2013's ADR, and if 2010's check calls
  `Op::Peers` a break, the completion notes carry its `Breaking:` line
  for `shuyi-cli`. The game's PR that moves the pin follows this one.
- **Size and the split:** the operation, its driver, the wiring and
  their tests are about 450 changed lines; `MemoryPeers` and its
  integration test about 200; the docs about 60; MCP about 150. With
  MCP that is more than one PR. So MCP is the last code step (step 9).
  It stays in this PR only if the diff, fixtures and snapshots aside,
  is under 600 changed lines when step 9 starts. Otherwise step 9
  writes a follow-up ticket with the `write-ticket` skill instead:
  parked like this one, `blocked_by: ["2529"]`, holding the MCP
  bullets above. Agents can still run `peers` through MCP as an
  operation tool.

**Out (do not do):**
- Real sockets, the relay, the LAN, Steam and lobbies (1141–1145). A
  relay or a referee inside the run (1141, 1146) is 2550's, after
  this ticket.
- Tick games: rollback (1147) and command turns (2720) build their own
  sessions, and neither ticket adds them to `peers`. `peers` drives
  turn games' lockstep only; a tick game in `peers` is a follow-up if
  a game asks.
- The `net` channel drawn in the debug view (round trip, loss, bytes
  per channel, the lockstep turn, hash agreement). It comes with the
  networking module, over 2315 (edit requested in 1140 and 2315).
- Several windows at once, or a local relay for them. 1145's
  in-process test covers loopback; windows on a running game are the
  editor's (2506) and `shuyi attach`'s (2521).
- More than 8 peers, and load tests: scale is 1150's research.
- Recording each peer's session (2518) or rewinding it (2357): a
  follow-up. Step 10 writes it with the `write-ticket` skill, parked
  like this one, blocked by this ticket and by whichever of 2518 and
  2357 is still open, and the completion notes name it.
- A template part with online play: 2551, after 1145. Until then a
  fresh `shuyi new` game offers no peers.
- Local multiplayer on one machine: input seats (2406, parked).

## Implementation steps

1. Read `crates/cli/src/contract.rs`, `contract/run.rs`,
   `contract/session.rs`, `contract/replay.rs`, `contract/cli.rs`,
   `contract/fake.rs`, `crates/cli/src/game.rs` and
   `crates/cli/src/frame_png.rs` (`Script`, `Step`), `docs/contract.md`
   (with 2509's problem codes), ADR-0022, ADR-0009 §4 and
   `tools/layers.ron`. Then read 1140's `crates/net` (`MemoryNetwork`,
   its faults and its step, `Transport::stats`, `LockstepSession` and
   its hashes) and 1133's MCP session.
2. Factor `replay::first_differing` out of `first_difference`.
3. Write `contract/peers.rs`: the types, `Peers`, `NetConditions`
   parsing, `PeersInput::check`, `play`, `compare`, `peers`, `text`,
   `usage`, `parse_args` and the problem codes, with unit tests over
   `FakePeers`.
4. Add `GameTools::peers`, `Op::Peers`, the CLI wiring and the usage
   lines.
5. Regenerate the schemas, raise `contract::VERSION`, add its line to
   `versions.json`.
6. Add `shuyi-net`'s `tools` feature, `MemoryPeers` and its line in
   `tools/layers.ron`, with the integration test below.
7. Do `hello-grid`'s part if 1145 is done, and the `shuyi-net` skill's
   part if 1134 is done.
8. Write the docs.
9. Measure the diff, fixtures and snapshots aside. Under 600 changed
   lines: add `peers`, `net`, `peer` and `reset` to the MCP session and
   the `net` tool, with its scripted client test, and their lines in
   `docs/contract.md`'s *Peers* section. Otherwise write the follow-up
   ticket (*Size and the split*) and name it in the completion notes.
10. Write the follow-up for recording and rewinding each peer's
    session (*Out*), and name it in the completion notes.
11. Run the gates.

## Acceptance criteria

- [ ] `shuyi-cli` has no dependency on `shuyi-net`, not even an optional
      one: `cargo metadata` lists no `shuyi-net` among `shuyi-cli`'s
      dependencies.
- [ ] Two peers of the test game below, with `loss=20, delay=3,
      reorder=10, seed=7`, end with equal turn hashes, the answer is
      `ok`, and a second run gives the same JSON.
- [ ] A desync forced on peer 1 at turn 3 answers `ok: false`, with
      `desync` naming peer 1, turn 3 and both hashes, equal to
      `replay::first_differing` over the two lists cut to the shorter.
- [ ] A peer one turn behind is no desync: the answer is `ok` and its
      limits say how far behind it is.
- [ ] After every round, every peer's `frame_count` is the same, and the
      network has advanced by the frames the round took (a `FakePeers`
      test).
- [ ] A game whose tools don't offer peers answers "the game's tools
      don't offer peers (GameTools::peers)" with code `SH3001`; one peer
      or an unknown `--net` field answers `SH0001`.
- [ ] `peers.input.json` and `peers.output.json` are checked in,
      `contract::VERSION` is raised, and `versions.json` has its line.
- [ ] `peers` is one of `shuyi mcp`'s tools, with its schemas.
- [ ] Either the MCP scripted client starts two peers, steps peer 1,
      reads peer 1's scene, sets `loss` with the `net` tool, and reads
      every peer's stats; or step 9 moved MCP out, and the follow-up
      ticket is in `tickets/open/`, parked, and named in the completion
      notes.
- [ ] If 1145 is done: `hello-grid`'s CI test of `cargo shuyi peers 2
      --net seed=1,loss=10,delay=2` answers `ok`.
- [ ] The test game's screen reacts to `Action::Confirm` and names no
      `Key`, `Chord` or `Button`; only the tests' Harness scripts name
      keys.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit:
  - `NetConditions` parsing: each field, an unknown field, a value that
    isn't a whole number, a percent over 100, a delay over 600;
  - `parse_args`: `--peer` sends the steps after it to that peer;
  - `PeersInput::check`: 1 or 9 peers, a step for a peer that doesn't
    exist, too many frames;
  - `play` over `FakePeers`: a `Keys` step `"A B"` of peer 1 plays as
    two rounds; a wait plays one frame a round; frame counts stay
    equal; the network advances by the frames run;
  - `compare` over set hashes: equal, differing at turn 3, a peer
    behind;
  - the problem codes of a refused input and of a game with no peers;
  - `text` of an answer that agrees and of one with a desync.
- Property: for any `NetConditions`, two peers of the test game agree
  at every turn both have applied, and every peer's frame count is
  equal after each round.
- Integration (`crates/net/tests/`, with the `tools` feature): a
  test-only lockstep game: one screen over the toy's `GameData`
  (`Game::new(shuyi_ui::toy::ctx(), Box::new(screen),
  GameHooks::default())`) that sends the next command of a small
  counter `TurnRules` through 1140's `LockstepSession` when
  `Action::Confirm` comes, played by `MemoryPeers`. A test-only start
  option breaks determinism on seat 1 at turn 3 for the desync test.
- Snapshot: the schemas test; the MCP client test, if step 9 keeps MCP
  here.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
