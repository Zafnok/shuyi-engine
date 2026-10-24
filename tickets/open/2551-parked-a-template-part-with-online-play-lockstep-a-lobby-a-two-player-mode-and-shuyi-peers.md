---
id: "2551"
title: "Parked: A template part with online play: a new game that picks lockstep gets a lobby, a two-player mode over the relay and the LAN, and shuyi peers"
type: feature
milestone: "Engine: networking"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2501", "2502", "2529", "1145"]
nick_input: none
completed:
---

# 2551 — A template part with online play

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need, 2529 among them): a follow-up of the parked peers run (2529) and of the parked sessions and lobbies (1145), and a follow-up of parked work is parked. The game doesn't ask for it, so it waits until after the MVP (`docs/ROADMAP.md`) and until both are unparked and done. If the game's ticket 0053 picks an online mode, it may be unparked with them. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0013](../../docs/adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)
makes `shuyi new` a wizard over the catalog, and a template a folder
of parts (2501). Online play is one of the catalog's concerns
(`docs/modules.ron`, `online`), asked with one pick (ADR-0013 §1;
2502's `--online`). Its answers are none, lockstep, rollback and a
referee server. ADR-0013's Consequences: "A module's ticket adds a part
only when its answer needs starter files." Lockstep needs them. Picked
today, it turns on the `net` module and nothing more: the new game has
`shuyi-net` and no way to play online.

What the parked networking work gives `hello-grid` by the time this
starts:

- 1145: a lobby screen over `SessionBackend` (create a room or join by
  code, a player list, ready toggles, start), the relay's and the
  LAN's backends, and a two-player online mode through the lobby,
  tested by two Harness games in one process.
- 2529, or 1145 if it landed later: `GameTools::peers` through
  `shuyi_net::tools::MemoryPeers`, a `tools` feature that turns on
  `shuyi-net/tools`, and a CI test of `cargo shuyi peers 2 --net
  seed=1,loss=10,delay=2`.

A new game gets none of it. 2529's **Out** leaves it here ("A template
part with online play ... A fresh `shuyi new` game offers no peers"),
and 1134's `shuyi-net` skill says the same of its worked example.

Why it matters:

- **Other 2D games.** Any turn game played online between friends (a
  card battle, a board game, tactics) starts the same way.
  [ADR-0006](../../docs/adr/0006-networking-in-layers-transports-sessions-sync-models-services.md)
  makes that lockstep plus a relay, and §4 says lockstep between peers
  is for friends.
- **The end goal** (`docs/decisions.md`, 2026-10-19: the most
  AI-friendly 2D engine). An agent asked for an online turn game makes
  it with one command and checks it with `peers`, as it checks
  `hello-grid`.
- **The game first.** Visions of Shuyi isn't made from the template,
  so nothing changes for it.

### The design, in short

- **A part, `online-lockstep`,** of the built-in template
  (`templates/shuyi-default/`, 2501), with `when: All([Picked("online",
  "lockstep"), Picked("perspective", "top-down-grid")])` and `modules:
  ["net"]`. 1140's lockstep runs over `TurnRules`, and the top-down
  grid is the template's turn stage. With lockstep and no grid stage,
  no part applies: the game gets the `net` module and a `limits` line,
  as 2501's composer gives for an answer no part adds files for.
- **Two players share the grid stage's piece, turn about.** The grid
  part's rules have one side: your piece walks to the goal while the
  wanderer steps by luck. Online, seat 0 commands on even turns and
  seat 1 on odd ones. The wanderer's luck is rolled from the seed both
  peers share, so a desync shows at once. The rules don't change.
- **The part keeps to its own files.** Two parts never write one file
  (2501's `SameFile`), and files hold no conditions (ADR-0013 §3).
  Where the part meets a file another part writes (the first screen,
  the tool program, the text, the game's `tools` feature), it goes
  through what the composer merges, and adds a merge where none fits.
- **`WithPeers`, in `shuyi-cli`:** a layer over a game's tools that
  answers `peers` and passes every other method on. The grid part's
  `tools.rs` stays the grid part's.

## Nick input

None.

## Scope

**In:**
- **`shuyi_cli::contract::peers::WithPeers<T, F>`**, beside 2529's
  `Peers`:
  - `WithPeers::new(tools: T, peers: F)`, where `T: GameTools` and
    `F: Fn(&[GameArg], u32, &NetConditions) -> Result<Box<dyn Peers>,
    String>`. Its fields are private and `new` is its constructor
    (ADR-0039);
  - its `GameTools` answers `peers` through `F` and passes every other
    method on to `tools`. The impl carries
    `#[deny(clippy::missing_trait_methods)]`, so a method `GameTools`
    gains later fails the build until it is passed on. A defaulted
    method left out would answer "unsupported" though the tools
    offer it;
  - it names `shuyi-cli`'s types only, so `shuyi-cli` still never
    depends on `shuyi-net` (ADR-0009 §4).
- **The part**, in the folder 2501's layout gives parts (for example
  `templates/shuyi-default/online/lockstep/`), and its `Part` in
  `template.ron` as above:
  - **`crates/game/src/online.rs`** (a folder `src/online/` if it
    grows):
    - `Match`, a screen holding 1140's `LockstepSession` over the grid
      part's `Stage` (`crates/rules`), its seat, and the transport the
      session runs on. On its seat's turn an action becomes a command
      as on the grid stage, and is sent through the session; on the
      other seat's turn actions send nothing. Every command the session
      hands back is applied in order. It draws the grid stage's view of
      the state, a line saying whose turn it is, and the outcome at the
      end. A desync the session reports shows as a `Note` and ends the
      match. Where the grid part's screen keeps its action-to-command
      step or its view private, make them public in the grid part and
      call them from both screens; never copy them. If `Stage` or
      `Command` lacks a derive the session needs (`Serialize`), add it
      in the grid part.
    - The lobby: 1145's lobby screen over the relay's and the LAN's
      backends, with the relay's address from `files/online.ron`. When
      the lobby starts the match, `Match` begins at the seat the
      session gives, with the seed the handshake hands over.
    - `online::start(seat, transport)`: the game started straight into
      `Match` at that seat, with no lobby, for the tools and the tests.
    - `online::peers(options, count, net)`, in `src/online/tools.rs`
      behind the game's `tools` feature: `MemoryPeers::start(count,
      net, peer, hashes)`. `peer` starts a headless game through
      `online::start` with its seat and its `MemoryTransport`, the grid
      stage's `--seed` read from `options`. `hashes` reads the session
      hashes `Match` keeps. A count other than 2 is the game's error:
      "the online mode is for two players". `shuyi.ron`'s `check_text`
      skips this file as it skips `src/tools.rs`, through the merged
      manifest.
  - **`crates/game/files/online.ron`:** the relay's address
    (`ws://localhost:<port>`, the port 1141's relay listens on by
    default). Its comments say how to run the relay
    (`docs/net-relay.md`) and that two copies on one LAN find each
    other with no relay.
  - **`crates/game/tests/online.rs`,** a test target of its own, so no
    other part's test file changes: two Harness games over
    `MemorySession` (1145's in-memory backend; if it is test-only in
    `shuyi-net`, make it public behind the `tools` feature) meet in
    the lobby, ready up, and play the stage to its end through actions
    only, with equal hashes at every turn. A third test checks that an
    action on the other seat's turn sends nothing.
  - **On the web,** the part offers what `hello-grid`'s online mode
    offers there after 1145, and the made game's web build still
    builds.
  - **Lockstep with no grid stage** (`--template blank --online
    lockstep`): the part doesn't apply, and the game still gets the
    `net` module, with a `limits` line naming the answer. If 2502 turns
    on an answer's module only when no part names it, make it also do
    so when no part that names it applies.
- **Where the part meets other parts' files.** For each, do (1) if it
  works; if not, (2). A merge added to the composer is written in
  `docs/templates.md`, with a fixture template and its test, and a
  composed game without the part stays the same bytes (2501's golden
  plan test):
  - **The way in, and the module.** The first screen is the base
    part's flow screen, and `src/lib.rs` is not this part's either.
    (1) If the flow lets the player pick among the stages
    `src/stages.rs` lists, the part declares `stage: Some("online")`,
    named by its own text key, and its module joins the crate as a
    stage part's does. (2) If the flow starts its first stage with no
    choice, a part may declare `entry: Some("<text key>")`: the
    composer lists the entries in `src/stages.rs` (declaring their
    modules there), and the flow offers them through the `UiAction`s
    before its first stage when there is one.
  - **The tool program.** `crates/game/examples/tools.rs` runs the
    picked parts' tools. (1) If 2501 builds it from the parts in a way
    that takes one more layer (as it joins the grid part's `GameTools`
    to the base part's), the part's layer is `WithPeers::new(tools,
    online::peers)`. (2) If one part writes it as a fixed file,
    generate it as `src/stages.rs` is: a part may declare `tools:
    Some("<path>")`, a function that takes the tools before it and
    returns them wrapped, applied in the parts' order.
  - **The text.** The lobby's keys are 1145's. The part's own keys
    (its entry's name, whose turn it is, waiting for the other player,
    the outcome lines) join the game's English as the grid part's own
    keys join the base part's in 2501.
  - **The game's `tools` feature.** `modules: ["net"]` turns on
    `shuyi`'s `net` feature through the merged `Cargo.toml` (2003's
    code). The game's `tools` feature, which the base part writes, must
    also turn on `shuyi-net`'s `tools`. (1) If `hello-grid` reaches
    `shuyi-net/tools` through a feature of the `shuyi` crate, the base
    part's `tools` feature names that feature. (2) If not, add one:
    `crates/shuyi/Cargo.toml` gains `tools = ["shuyi-net?/tools"]`, a
    weak feature that turns on `shuyi-net`'s `tools` only in a game
    with `net` on. The base part's `tools` feature turns on
    `shuyi/tools`, `hello-grid` moves onto it, and 2003's
    `check-modules` learns that `tools` is no module's feature. Record
    the pattern (a module's `tools` part reached through `shuyi`'s one
    `tools` feature) with the `write-adr` skill. Either way a game's
    binary never links `shuyi-cli` (ADR-0002 §1).
- **The catalog:** if `lockstep` is still `parked` in
  `docs/modules.ron`, set it `ready` with no tickets, as ready answers
  have, so the wizard offers it (ADR-0013 §6). `cargo xtask
  check-modules` passes.
- **CI:** 2501's `templates` job gains a leg. It runs `shuyi new demo
  --no-input --online lockstep`, then the made game's gates (as for
  every made game, `tests/online.rs` among its tests), then `cargo
  shuyi peers 2 --net seed=1,loss=10,delay=2`, which must answer `ok`.
  It runs `peers` again with one step from each seat, in turn, and a
  wait longer than the delay between them (in the form 2529's
  `peers::usage` gives): it must answer `ok` with every peer's `turns`
  at 2.
- **Peers through a relay, either order with 2550:** if 2550 is done,
  the part's tools also offer `GameTools::peers_via` for `Direct` and
  `Relay`, through `MemoryPeers::through_relay` for the relay, as
  `hello-grid`'s tools do. `WithPeers` then answers `peers_via` itself
  (`Direct` through the `peers` it is given), never passing it on to
  the tools it wraps, and the CI leg also runs `cargo shuyi peers 2
  --via relay --net seed=1,loss=10,delay=2`, which must answer `ok`. If
  not, nothing to do here (2550 adds it to this part when it lands; its
  clause says so).
- **The `shuyi-net` skill, either order with 1134:** if 1134 is done,
  `skills/shuyi-net/SKILL.md` says how a new game starts with online
  play (`shuyi new <dir> --online lockstep`) and what the part gives,
  and shows `cargo shuyi peers` on such a game. Its worked example
  stays on `hello-grid`. If not, nothing to do (1134 writes the skill
  from what exists).
- **Docs** (the `document` skill): `docs/templates.md` (the part, its
  `when`, what it gives, and any merge added here); `docs/net.md`
  (starting a new game with online play); `docs/contract.md`'s *Peers*
  section (`WithPeers`, for a game's own tools).
- **For the game:** additive: a new type in `shuyi-cli` and, if (2)
  above adds it, a feature of `shuyi`. If CI's `api` job calls any of
  it a break, the completion notes carry its `Breaking:` line. The
  game's PR that moves the pin follows this one.
- **Size:** `WithPeers` and its tests are about 120 changed lines; the
  part's code and its tests about 300; CI, the catalog and the docs
  about 80; each merge the composer needs about 80 more. If both (2)s
  of the way in and the tool program are needed and the diff,
  fixtures and snapshots aside, passes 600 changed lines, the merges
  go first in a PR of their own: write their ticket with the
  `write-ticket` skill, add it to this ticket's `blocked_by`, and work
  it first.

**Out (do not do):**
- Parts for the catalog's other online answers, `rollback` and
  `referee`. Nothing plans them; a game that asks gets a ticket then.
- Online play over any other stage. The part applies with the top-down
  grid stage only; another turn stage that wants it writes its own
  part.
- Two sides in the grid part's rules. The two players share the piece.
- More than two players.
- Steam's lobbies, accounts, matchmaking and chat. The part offers the
  relay and the LAN, as `hello-grid`'s online mode does.
- Running a relay for a game's players: the developer's, as
  `docs/net-relay.md` says.
- Whether a target can play online: the `net` module's `needs` and the
  wizard's check of them (ADR-0013 §5).
- Recording or replaying an online match.

## Implementation steps

1. Read 2501's `templates/shuyi-default/template.ron` and composer
   (the merged files, how the grid part's module, tools and text join
   the base part's, the flow screen, the golden plan test); 2502's
   `online` question and `--online`; 1145's lobby screen,
   `SessionBackend`, `MemorySession` and `hello-grid`'s online mode and
   its tests; 2529's `contract/peers.rs`, `MemoryPeers` and
   `hello-grid`'s `peers` in `src/tools.rs` and `Cargo.toml`;
   `docs/templates.md`, `docs/net.md`, `docs/net-relay.md`, ADR-0006,
   ADR-0009 §4 and ADR-0013. Check whether 2550 and 1134 are done.
2. Write `WithPeers` and its unit tests.
3. Settle the game's `tools` feature: (1), or (2) with its ADR.
4. Write the part: `online.rs` (`Match`, the lobby, `start`),
   `online/tools.rs` (`peers`), `files/online.ron`, the text and
   `tests/online.rs`. Make public in the grid part what `Match` needs
   from its screen.
5. Settle the way in and the tool program: (1), or (2) with their
   fixtures. Measure the diff here (*Size*).
6. Set the catalog's `lockstep` answer.
7. Add the CI leg.
8. Do 2550's part if it is done, and the skill's if 1134 is done.
9. Write the docs.
10. Run the gates.

## Acceptance criteria

- [ ] `shuyi new demo --no-input --online lockstep` makes a game that
      builds and passes its tests in CI: in its `tests/online.rs`, two
      Harness copies meet in the lobby, ready up and play the stage to
      its end through actions only, with equal hashes at every turn.
- [ ] On that game, CI's `cargo shuyi peers 2 --net
      seed=1,loss=10,delay=2` answers `ok`, and with one step from each
      seat in turn it answers `ok` with every peer's `turns` at 2.
- [ ] `cargo shuyi peers 3` on that game answers the game's error that
      the online mode is for two players, with code `SH3002`.
- [ ] `shuyi new demo --no-input` (online play `none`) makes the same
      bytes as before this ticket: 2501's golden plan test passes
      unchanged.
- [ ] `shuyi new demo --no-input --template blank --online lockstep`
      makes a game with the `net` module on, no online files and a
      `limits` line that names the answer, and it builds.
- [ ] In the made game, `shuyi-cli` is reached only through its
      `tools` feature: `cargo tree -e normal` on the game crate without
      `--features tools` lists no `shuyi-cli`.
- [ ] `WithPeers` passes every other `GameTools` method on, and its
      impl denies `clippy::missing_trait_methods`.
- [ ] `docs/modules.ron` has `lockstep` ready, `shuyi new demo
      --questions --json` offers it, and `cargo xtask check-modules`
      passes.
- [ ] `cargo shuyi check-keys` and `check-text` pass on the made game;
      the part's screens react to actions and name no key.
- [ ] If 2550 is done: CI's `cargo shuyi peers 2 --via relay --net
      seed=1,loss=10,delay=2` on the made game answers `ok`.
- [ ] `docs/templates.md` describes the part and any merge added here.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-cli`): `WithPeers` over a fake `GameTools` and a fake
  `Peers` maker: `peers` comes from the function, its error passes
  through, and every other method answers what the wrapped tools
  answer.
- Unit (the composer): the plan for `--online lockstep` holds the
  part's files and the `net` module; with online play `none` it holds
  neither; with `--template blank` it holds the `net` module, none of
  the part's files, and a `limits` line. Each merge added here has a
  fixture template and a test.
- Integration (the made game, in CI): `tests/online.rs` as above, and
  `online::peers` refusing a count other than 2.
- CI: the template leg's `cargo shuyi peers` runs.
- Snapshot: the golden plan for the lockstep answers, in 2501's form.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
