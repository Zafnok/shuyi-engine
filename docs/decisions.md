# Decisions

Nick's product decisions for Shuyi Engine. Each records his words verbatim,
then the rule that follows from them. Technical decisions are ADRs in
[`adr/`](adr/README.md).

## 2026-10-14: the split, the plan, the repository, the name, the license

Source: game ticket 1101 (the feasibility study,
`github.com/Zafnok/visions-of-shuyi`, `docs/engine/feasibility.md`).

### The goal

> "determine if we can rearchitect the project into 2 separate repos /
> goals: 1. base layer - our engine, which we can add functionality to,
> which is more general purpose. We can ship this eventually with UI /
> graphical coding (like Blueprints from Unreal) 2. game layer - our game
> coded on the engine as an external dependency. engine can become
> free/open source. game remain paid. The engine should serve 2 goals 1.
> make it easy for us to make our current game 2. make it easy for
> prospecting new devs to make their games, not constrained to tactical
> rpgs"

**Rule:** the engine serves the game first, and other people's 2D games
second. It never holds anything of the game (`AGENTS.md`, rule 4).

### The plan: B, with parked work written down

The study offered three plans:

- **A**: carve the engine out and stop.
- **B**: carve, publish with the AI tooling, and build other styles and
  networking only when needed.
- **C**: build everything in parallel with the game.

> "anyway I think B is ok, so long as we document the parked (i.e. after
> MVP/parity) tickets for networking, styles for later. I'll probably
> need a new github repo for the engine I guess. Let's do the very
> initial setup in this session (new github repo, push up tickets and
> ADRs etc, push up useful dev skills like the ones we have in this repo
> for dev of the engine). No actual engine work to be done in this
> session."

**Rules:**

- **The MVP is parity.** The engine reaches the MVP when it does
  everything the game needs today, from its own repository, and is
  usable by agents. [`ROADMAP.md`](ROADMAP.md) lists the tickets.
- **What is parked.** Other game styles (1119–1128), networking
  (1140–1150) and visual scripting (1135) are written down as `Parked:`
  tickets. They start after the MVP, or earlier only when the game needs
  them:
  - 1119–1125 if the game's ticket 0060 says the army walks around freely;
  - 1140–1150 if the game's ticket 0053 picks an online mode.
- **The repository.** The engine has its own repository from 2026-10-14
  (`github.com/Zafnok/shuyi-engine`). It holds plans, tickets and skills.
  The code arrives with the game's ticket 1131, after the carve.

### Networking, if it doesn't cost the game

> "if it turns out feasible without jeopardizing our own velocity, I also
> want to look at adding easy, abstracted networking to the game, support
> for P2P (LAN or WAN), support for sessions, support for hosted
> multiplayer (i.e. hosted on one box and connected by others OR by
> another company), support for matchmaking / ELO, and support for MMO,
> and global chat. These can be phased easy to hard. And we can expose
> both abstracted primitives/interfaces as well as the lower level stuff
> if a certain quirk needs to be used."

**Rule:** [ADR-0006](adr/0006-networking-in-layers-transports-sessions-sync-models-services.md).
The networking tickets are parked as above. Global chat and MMO scale
need Nick's go-ahead after ticket 1150 shows their running costs and
legal duties.

### Visibility and license

Asked "public or private for now?", Nick chose **public now**. Asked
which license, he chose **MIT OR Apache-2.0**, the recommended option.

**Rule:**

- The repository is public.
- The engine is dual-licensed MIT OR Apache-2.0, at the user's option
  (`LICENSE-MIT`, `LICENSE-APACHE`).
- The game's own `LICENSE` is unchanged.

### The name: Shuyi Engine

Of the first shortlist (Glyphwright, Gridwyrm, Cellwright, Tilewright):

> "dont like any of these, they are too confined to the tile idea which we
> will expand past at some point, and glyph as well is underselling our 2D
> capabilities (we should still be able to support glyphs). Think more
> creatively, look for inspiration in existing engine names without
> outright copying or parodying them"

Of the second (Planiverse, Lusory, Emaki, Tomte):

> "can I just name after my wife the same as the game? Shuyi Engine. Or
> just "Shuyi". I won't copyright it so she can still use her name lol.
> Does it affect my Visions of Shuyi game title in any way if Shuyi Engine
> is open license and Visions of Shuyi is closed license?"

Claude answered, not as legal advice:

- The licenses cover code, not names. Apache-2.0 grants no trademark
  rights.
- "Visions of Shuyi" can still be registered for the game (game ticket
  0906), and a lawyer should confirm the engine doesn't weaken it.
- People will connect the two names.

Nick then chose **"Shuyi Engine"** over "Shuyi".

**Rules:**

- **Names:**
  - display name: Shuyi Engine;
  - repository: `Zafnok/shuyi-engine`;
  - crates: `shuyi-core`, `shuyi-assets`, `shuyi-ui`, `shuyi-app` and
    `shuyi-cli`, plus later crates under `shuyi-*`;
  - command-line binary: `shuyi`.
- **No registration.** The name is not registered as a trademark.
- **Forks.** The README asks forks to choose their own name. It welcomes
  "made with Shuyi Engine".

### Formats as plugins

> "we should also think about, as much as possible, making certain parts
> modular, such as - if we want to introduce a highly optimized file
> format, this should be easier than ever. But also maintain broad
> compatibility with normal, widely used file formats. I'm thinking like
> our dialogue scripts, asset files, etc. If you can find a way to make
> png/jpegs better using AI black magic then we can ship a separate repo
> for that, and support their loading too."

**Rules:**

- [ADR-0007](adr/0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md)
  makes formats plugins.
- Tickets:
  - 2201: the registry;
  - 2202: dialogue formats (Yarn Spinner, Ink);
  - 2203: more image and audio formats;
  - 2204: an optimized pack format for shipping;
  - 2901: research on better images with machine learning, to be shipped
    as a separate repository with a loader plugin.

### Still open

From the game's ticket 0059:

- whether the grid-tactics kit (grid, movement ranges, turns, the battle
  cursor) is open in the engine or stays the game's (the plan assumes
  open);
- who may contribute code (the recommendation is issues only until the
  first public release, then pull requests under a Developer Certificate
  of Origin).

## 2026-10-15: export targets, the Playdate rule, benchmarks

Source: Nick, in a Claude Code session on 2026-10-15.

> "lets make sure there are tickets for export targets like consoles,
> even obscure like playdate, with expected performance benchmarks for
> game exports
>
> like I want my visions of shuyi to run on playdate, so it must be able
> to translate its graphics to 1-bit easily and run at a good fps and
> support dpad/a+b/lever only control scheme if there are more buttons
> necessary (not optional) in any context it simply cannot run on
> playdate
>
> I also want it to be exported to steam deck, and eventually nintendo
> eshop and maybe even mobile and console"

**Rules:**

- **Targets and their order.** The details are in
  [ADR-0008](adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
  and tickets 3001–3016.
  - **The Steam Deck and the Playdate** are wanted for the game. Their
    tickets come after the MVP and are not parked.
  - **The Nintendo eShop, mobile, and PlayStation and Xbox** ("eventually",
    "maybe") are `Parked:` tickets until Nick unparks them. Each costs
    money and brings an agreement: a developer program, kits, store
    fees.
- **The Playdate rule.** On the Playdate a game may need only the d-pad,
  A, B and the crank. "A+B" is read as two buttons, plus the A+B chord
  Nick offered on 2026-10-07 (game ticket 1025: "chording A+B if we need
  to"). Nick can veto the chord.
  - If any screen needs more inputs than that for a required action, the
    game doesn't ship on the Playdate. `shuyi check --target playdate`
    fails, and `shuyi export playdate` refuses.
  - Optional actions may go anywhere, the system menu included.
  - The engine never drops a required action to make a game fit.
- **1-bit, easily.** Graphics are a skin (ADR-0002 §4), so a game gets a
  1-bit look from a pattern map for its palette roles, plus dithering of
  its sprites at bake time.
  - Roles that tell things apart, such as the sides in a battle, must
    stay distinct, or the check fails.
  - Hand-drawn 1-bit art overrides any asset.
- **A good frame rate, as numbers.** Every target has budgets in its
  profile. On the Playdate the starting budget is 30 frames a second,
  held: no frame over 50 ms, on both hardware revisions, even while the
  enemy thinks.
  - CI gates on exact proxies (instruction counts, memory, size).
  - Device tickets measure the real thing.
  - Every number is provisional until its device ticket measures it.
- **What this doesn't change.**
  - The MVP is still parity with the game.
  - The game's own Playdate port (game ticket 1025) is still parked
    until after launch, in the game repository. These tickets make the
    engine ready for it.
  - Steam itself (the partner account, Steamworks, the store page) stays
    the game's ticket 0903.
