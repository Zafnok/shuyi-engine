---
id: "2855"
title: "The skill pack teaches online tick games: shuyi-net rewritten for rooms and quick play, and modules.ron gains a server answer"
type: feature
milestone: "Engine: networking"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2789", "2787"]
nick_input: none
completed:
---

# 2855 — The skill pack teaches online tick games

Unparked by Nick's decision of 2026-11-25 (`docs/decisions.md`): the
next example game, trails, is played online, in rooms a server keeps,
with bots in the empty seats.

## Context

The engine gains online play for tick games in 2612–2617, and trails
uses it in 2788–2790. Each of those code tickets leaves its `Skills:`
line to this ticket. An agent making a game reads the skill pack
(`skills/`, ADR-0043), and today the pack says online play doesn't
exist.

What is there today:

- **`skills/shuyi-net/SKILL.md`** (89 lines, version `1.0`) starts with
  "**Not built yet.**". It teaches only how a turn game stays ready for
  lockstep (`## How lockstep will work`, `## Steps, meanwhile`). Its
  `metadata.waits_for` is `"1140 2529 1145"`. It has no `example.ron`:
  its `## Worked example` section says it waits.
- **`cargo shuyi check-skills skills --tickets tickets/open`** fails a
  `waits_for` id that is no open ticket (`crates/cli/src/skills.rs`,
  check 5).
- **The worked examples** (`skills/<skill>/example.ron`, run by
  `crates/xtask/src/skills.rs` in CI's `skills` job, and by
  `cargo xtask skill-examples <skill>`) run on a game fresh from
  `shuyi new`. A real-time game comes from `--template arcade`, the
  template part 2787 adds. So this ticket is blocked by 2787 too.
- **`docs/modules.ron`**, concern `online` ("How do players play
  online?"): `none` (ready), and `lockstep`, `rollback` and `referee`
  (all parked, `by: "shuyi-net"`). It has no answer for a
  server-authoritative real-time game. Its header says "a ticket that
  adds an answer adds it here, and one that finishes an answer sets it
  `ready`". `docs/modules.md`, the catalog, has one `## Online play`
  row: `shuyi-net`, parked, 1140–1150. `docs/guides/README.md` line 22
  says "None, or lockstep, rollback or a referee server."
  `cargo xtask check-modules` keeps the three in step.
- **`shuyi new`** asks the `online` concern once two of its answers are
  ready or planned (`crates/cli/src/contract/new/wizard.rs`, around
  line 236). So a `ready` `server` answer makes every new game get the
  question.
- **The game** (`Zafnok/visions-of-shuyi`, pinned at engine revision
  `b69585eee640ba3ba4963bc62ca5f6ec3db189d7`) copies the skill pack at
  its pin (game ADR-0097). `gh search code --repo Zafnok/visions-of-shuyi
  'shuyi-net'` finds only that copy (`.claude/skills/shuyi-net/SKILL.md`,
  `.agents/skills/shuyi-net/SKILL.md`) and the tickets about copying it.
  The game gets the new text at its next pin bump, and nothing in it
  must change.

## Nick input

None.

## Scope

**In:**

- **Rewrite `skills/shuyi-net/SKILL.md`** for what exists, and set its
  `version` to `"2.0"`. Its description names online play for real-time
  games through rooms and quick play. It teaches, with the names that
  landed in 2613–2617 and 2789:
  - when to use it: a tick game (`TickSim`) played online with others,
    with bots in empty seats; and when not to (a turn game, which is
    still parked; local play, `shuyi-stage-arcade` or
    `shuyi-stage-grid`);
  - the model in a paragraph: the server decides, and broadcasts each
    step's inputs; clients step the same rules (ADR-0150);
  - the steps: `impl RoomGame` beside the game's `TickSim`, with the bot
    hook kept pure in the rules (no floats, a seeded `SimRng`); a server
    binary behind a `server` feature calling `shuyi_server::run`; the
    client's `net` feature and its connection screen; the director's
    config (rooms, warm rooms, seats);
  - the client transports behind `net`, and the bytes each adds to a
    shipped game, copied from 2617's and 2790's completion notes;
  - testing over `MemoryNetwork` with seeded faults, and finding a
    desync from a hash check;
  - the rules a game keeps: no floats in the rules, never a key named,
    no chat, names or accounts without Nick (moderation duties,
    ADR-0006), and the server logs no addresses;
  - lockstep, rollback and the referee kept as short "parked" notes,
    each with its ticket.
- **`metadata.waits_for`** lists only the still-open ids the skill
  names as parked (1140, 1145, 1146, 1147 and 2529, as many as are open
  when this lands), so `check-skills` passes.
- **`skills/shuyi-net/example.ron`**, its worked example, on
  `template: "arcade"`: it turns on the `shuyi` front door's `net`
  feature, writes a small `RoomGame` and bot for the template's rules
  (files under `skills/shuyi-net/example/`), writes a test that runs a
  room with two clients over `MemoryNetwork`, drops one and checks a
  bot takes its seat, then runs `cargo test`, `cargo fmt --all --check`
  and `cargo clippy --workspace --all-targets -- -D warnings`, as the
  other examples do. It never opens a socket.
- **Cross-links**: one line each from `skills/shuyi-stage-grid`,
  `skills/shuyi-stage-arcade` (2787), `skills/shuyi-test` (testing a
  room over `MemoryNetwork`) and `skills/shuyi-ship` (shipping the
  server binary and a client with `net`) to `shuyi-net`. The row in
  `skills/README.md` (line 72, "online play") says rooms and quick play.
- **`docs/modules.ron`**: an answer under `online`,
  `(id: "server", name: "a game server: rooms, bots in empty seats, quick play", by: "shuyi-net", status: …, tickets: ["2614", "2615", "2616"])`.
  2613 adds this answer as `planned` (its crate's module metadata
  needs it for `check-modules`); keep that row and fix its words to
  these.
  - **Its status.** `shuyi new` already asks the `online` question once
    `server` is `planned` (2613's `Behaviour:` line). `ready` says a new
    game made with `--online server` gets it working, so it becomes
    `ready` only if such a game gets the `net` feature turned on and
    passes its gates. If no template part turns it on (2551, the online
    template part, is parked), it stays `planned`, and the completion
    notes say why.
  - `docs/modules.md`: the `shuyi-net` row says what is built (rooms,
    quick play), and a row for `shuyi-server`, the headless server
    library.
  - `docs/guides/README.md` line 22, and each guide that lists online
    answers, say "a game server" too.
  - `cargo xtask check-modules` passes.
- **The docs map, `docs/README.md`**:
  - the `26xx` row's "read first" gains ADR-0150, ADR-0151 and the
    book's online chapter (2854, linked by path once it exists);
  - a row for how this repository's own servers are deployed and
    watched: `deploy/trails/README.md` (2618) and the monitor workflow
    (2619), if no row covers them yet. **Either order with 2856:** if
    2856 is done, that row links its book page *Hosting your server* as
    the game maker's version; if not, nothing to do (2856 links it);
  - `cargo xtask check-docs` passes.

**Out (do not do):**

- The book's chapter. 2854 writes it.
- The hosting page. 2856 writes it.
- An online template part. 2551 is parked.
- The lockstep steps and the two-peer `hello-grid` worked example that
  1140, 1145 and 2529 plan for this skill. They stay parked. When one
  of them lands, it adds its example beside this one as
  `skills/shuyi-net/example-lockstep.ron` (the runner reads every
  `example-<what>.ron`, `crates/xtask/src/skills.rs`), not in place of
  `example.ron`.
- `.claude/skills/` and `.agents/skills/`. This ticket changes the
  shipped pack in `skills/`, not this repository's own workflows, so
  `pwsh -File scripts/sync-agent-skills.ps1` has nothing to do.

**Estimated size:** about 600 lines (the skill 250, the worked example
and its files 250, modules, guides and the docs map 100).

## Implementation steps

1. Read ADR-0150, the completion notes of 2613–2617, 2789 and 2790, and
   `examples/trails`. Note the names that landed and the byte costs of
   the `net` feature.
2. Rewrite `skills/shuyi-net/SKILL.md` and set `version` and
   `waits_for`.
3. Write `skills/shuyi-net/example.ron` and its files in
   `skills/shuyi-net/example/`. Run
   `cargo xtask skill-examples shuyi-net`.
4. Add the cross-links and the `skills/README.md` row.
5. Add or fix the `server` answer in `docs/modules.ron`, settle its
   status (Scope), and update `docs/modules.md` and the guides. Run
   `cargo xtask check-modules` and `cargo run -p shuyi-cli -- modules`.
6. Update `docs/README.md`. Run `cargo xtask check-docs`.
7. Run `cargo shuyi check-skills skills --tickets tickets/open` and the
   `run-gates` skill.

## Acceptance criteria

- [ ] `skills/shuyi-net/SKILL.md` no longer says "Not built yet", and
      every type, function and feature it names exists in the code.
- [ ] `cargo xtask skill-examples shuyi-net` passes, and CI's `skills`
      job runs it.
- [ ] The worked example's test shows a bot taking a dropped player's
      seat.
- [ ] `cargo shuyi check-skills skills --tickets tickets/open` passes.
- [ ] `shuyi modules` lists the `server` answer under *Online play*.
- [ ] A new game made with `--online server` builds and passes its
      gates, or the answer is not `ready` and the completion notes say
      why.
- [ ] `cargo xtask check-modules` and `cargo xtask check-docs` pass.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code: this is that update.
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the worked example through the skills runner (CI's
  `skills` job), with its own test of a room over `MemoryNetwork`.
- `cargo xtask check-modules`, `cargo xtask check-docs` and
  `check-skills`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
