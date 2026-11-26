---
id: "2854"
title: "The book's chapter on online play: a tick game served from rooms, with bots in empty seats, walked through on trails"
type: feature
milestone: "Engine: networking"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2790"]
nick_input: none
completed:
---

# 2854 — The book's chapter on online play

Unparked by Nick's decision of 2026-11-25 (`docs/decisions.md`): the
next example game, trails, is played online, in rooms a server keeps,
with bots in the empty seats.

## Context

The engine work for online play lands in 2612–2617, and the game that
uses it in 2788–2790. Each of those code tickets leaves its `Book:` line
to this ticket: none of them writes a chapter. This ticket writes the one
chapter a game maker reads to put a real-time game online the way trails
does.

What it builds on:

- **ADR-0153** (written by 2613): the sync model. The server runs the
  real `TickSim` headless and decides everything: seats, which input
  counts at which step, the bots, and when a round starts and with
  which seed. Each step it broadcasts the step's resolved `TickInput`.
  Clients step their own copy of the same deterministic rules with
  exactly those inputs. Determinism (ADR-0006 §2) is the network
  contract.
- **2618's ADR**: where trails' server runs. This
  chapter only links it; hosting is 2856's page.
- **The book today** (`site/content/learn/book/`) has 10 chapters on
  Moon Garden, the book's small game, in `site/snippets/` (the crate
  `book-snippets`). Chapter 5, *Rules as commands*, already has a
  real-time part: *Rules in fixed steps*, with Moon Garden's `Bee`, a
  `TickSim` (`site/snippets/src/rules.rs`, anchors `tick` and
  `tick-test`). This chapter builds on that bee.
- **The `snippet` shortcode** (`site/templates/shortcodes/snippet.md`)
  reads only `site/snippets/` and `site/tutorial/`. It can't show a file
  of `examples/trails`. So the chapter's code is a small game of its own
  in `site/snippets/`, built on the shapes trails uses. The chapter then
  walks through trails by linking its files.
- **`cargo xtask check-book`** (2852, done) fails a bold term with no
  glossary row. Bold is for terms only, and other stress is in italics
  (`site/README.md`, *The rules*).
- **The Markdown twins** and `llms-full.txt` are written at each build
  from the Learn pages (`site/README.md`: "Change Learn, never a twin").
  A new page gets its twin with no extra work.
- **The home page** says the book goes "to putting a finished game
  online", which means publishing it on the web. This chapter is about
  playing together over a network. Its words keep the two apart.

## Nick input

None.

## Scope

**In:**

- **From 2612 (done):** the chapter teaches that a step holds up to eight
  seats (`shuyi_core::tick::MAX_SEATS`), and the byte form of a step's
  input (`SeatInput::encode` and `decode`, `TickInput::encode` and
  `decode`, `TooManyActions`, in `crates/core/src/tick/bytes.rs`): 2612
  left both to it.
- **The chapter** `site/content/learn/book/online-play.md`, titled
  *Playing online*, with `extra.part = "chapters"`, `extra.chapter = 11`
  and `weight = 110`. That puts it after *Tools for agents* (chapter 10,
  weight 100). The book has no real-time or grid chapter after
  chapter 5 to sit next to, so it goes last among the chapters. Its
  sections:
  1. **Why the server decides.** One copy of the rules is the truth.
     Clients step the same rules from the inputs the server sends. That
     works only because the rules are deterministic (chapter 5's hash).
     It also says why not rollback: it needs no server, but a game with
     bots and drop-in needs one anyway, and the browser has only
     WebSocket. Link ADR-0153.
  2. **A room.** `RoomGame` and its consts (`SEATS` and the others 2614
     defines), on Moon Garden's bees.
  3. **Bots in empty seats.** The game's own bot hook,
     `RoomGame::bot_input`, pure, with a seeded `SimRng`. Bot inputs are
     broadcast and recorded like a player's, so replays stay exact.
  4. **Seats, drop-in and drop-out.** A player takes a bot's seat and a
     bot takes back a player's seat, as 2614 builds them. Rounds go on
     without end.
  5. **Quick play.** The director of 2615: one button, a room with a
     free seat, a cap of rooms, `Busy` when full, and the build check
     that answers `Refused`. Also what it deliberately has not: names,
     chat, accounts and room codes, and why (moderation duties,
     ADR-0006).
  6. **The server program.** A game's own binary behind a `server`
     feature, calling `shuyi_server::run` (2616). It never pulls in
     macroquad.
  7. **The client.** The `net` feature, the connection screen's states
     (2790), and where the address comes from (`?server=`, then
     `server.ron`, then the dev default).
  8. **Testing it all with no network.** Rooms, clients and the director
     over `MemoryNetwork` with seeded faults (2613, 2620), and the hash check
     every 15 steps that finds a desync.
  9. **The whole game: trails.** A walk through `examples/trails`, its
     rules, bot, `impl RoomGame`, `trails-server` and online screen,
     linked by path.
  10. **What isn't here yet.** Prediction of your own rider and rollback
      (1147, 2604), lobbies (1145), accounts and ratings (1148), turn
      games online (1140, 1146). All are parked.
  11. **Where a server runs.** Two sentences and a link to 2618's ADR.
      2856 (blocked by this ticket) writes the page *Hosting your
      server* and changes this link to point at it.
- **The code**, `site/snippets/src/online.rs`, with `// ANCHOR:` marks
  for each snippet. It holds a small `RoomGame` over Moon Garden's
  bees: one bee a seat, and a bot that flies by `SimRng`. It also holds
  the tests the chapter shows: a room with two clients over
  `MemoryNetwork`, a drop-out handed to a bot, and a client that
  catches up and matches the server's hash. The server snippet calls
  `shuyi_server::run` in a function the tests compile but never call
  (no socket in a test).
- **`site/snippets/Cargo.toml`**: the `shuyi` front door with its `net`
  feature on (2613's feature, off by default for games), and
  `shuyi-server` by path for the server snippet. Both are workspace
  crates already, so CI builds nothing new.
- **Glossary rows** in `site/content/learn/book/glossary.md`, section
  *Shuyi* unless the word is a general one. One row for each word the
  chapter bolds, at least: room, seat, bot seat, quick play, catch-up,
  hash check, desync, server-authoritative and transport. Each row links
  this chapter's heading.
- **Links from the earlier chapters**: chapter 5's *Rules in fixed steps*
  gets one sentence pointing here ("the same input, stepped on every
  machine, is how a game plays online: chapter 11").

**Out (do not do):**

- Hosting steps, hosts and costs. 2856 writes that page, and links it
  from this chapter.
- The skill pack's `shuyi-net` skill and `docs/modules.ron`. 2855 does
  those.
- Turn games online (lockstep, the referee). 1140 and 1146 are parked.
- A step of the book's project. The box-pusher is a turn game for one
  player.
- Pictures (`site/tutorial/tries.toml`). An online screen needs a
  server running, and `frame_png` has none.
- Changing the `snippet` shortcode to read `examples/`.

**Estimated size:** about 700 lines (chapter 350, snippets and their
tests 300, glossary and links 50).

## Implementation steps

1. Read ADR-0153, 2614's and 2615's completion notes, and
   `examples/trails` (its README, `src/`, `trails-server`). Write the
   chapter's outline against the names that actually landed. A name in
   this ticket that changed is used as it landed.
2. Write `site/snippets/src/online.rs`: the bees' `RoomGame`, the bot
   hook, a server function, and the tests. Add `mod online;` to
   `site/snippets/src/lib.rs` and the dependencies to its `Cargo.toml`.
   Run `cargo test -p book-snippets`.
3. Write `online-play.md` with the `snippet` shortcodes and the
   `newcomer` notes the site's rules ask for (`site/README.md`, *Marking
   a passage*). A reader new to networks gets a note on what a server,
   a socket and latency are.
4. Add the glossary rows, and the sentence in chapter 5.
5. Run the site's checks as the `run-gates` skill lists them (its site
   job: `cargo xtask site-data --site site`, `zola check
   --skip-external-links`, `cargo xtask check-book`,
   `bash site/check-build.sh`), and `cargo xtask check-links`.
6. In the completion notes, list each code ticket whose `Book:` line
   pointed here and the section that covers it.

## Acceptance criteria

- [ ] The chapter renders on the site as chapter 11, after *Tools for
      agents*, and the book's contents list it.
- [ ] Every snippet it shows comes from `site/snippets/src/online.rs`,
      and `cargo test -p book-snippets` runs its tests in CI's test job.
- [ ] A test shows a client that joins late catching up, then matching
      the server's state hash at the next hash check.
- [ ] A test shows a player's seat handed to a bot after the player
      leaves, with the room still stepping.
- [ ] `cargo xtask check-book` passes, with a glossary row for each
      bold term the chapter uses.
- [ ] The link checks pass, and the chapter links ADR-0153, 2618's ADR
      and trails' files.
- [ ] The chapter never calls the genre or the example by an existing
      game's name.
- [ ] Each code ticket whose `Book:` line named 2854 is covered by a
      section, as the completion notes list.
- [ ] The book's chapter is updated (2802): this is that update.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (in `site/snippets/src/online.rs`): the bees' room steps the same
  on the server and on two clients over `MemoryNetwork`; a seeded fault
  (a dropped connection) hands the seat to a bot; a late client catches
  up and its hash matches.
- Integration: `cargo xtask check-book`, the link check and the site
  build, all in CI.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
