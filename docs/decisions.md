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

## 2026-10-16: modules, many kinds of game, procedural animation

Source: Nick, in a Claude Code session on 2026-10-16.

> "another inspiration hit me and that's rain world
>
> we should support plenty of modules for our engine, so that we don't
> overload devs, but one such module should be procedural animation like
> rain world has
>
> this will be very late addition since it won't tie into visions of
> shuyi game but it's something to think about that we should support a
> wide variety of game types in ways that make sense..."

Then:

> "the way I think about it is, high-level, a dev will set a direction
> and import non-conflicting modules to help them out. So for instance,
> if they decide they want a normal physics/animation system, they will
> use that. Or if they want procedural animation, we will have different
> primitives for that. And same for i.e. side scrolling perspective games
> vs top-down or isometric. Not to say that games can't transition
> perspectives, but we should make it very obvious OK this behavior is
> built for this type of game, this one for this type of game, here is
> how to transition between the two... make it smartly designed and well
> documented at all steps. And eventually we support a visual
> programming like Blueprints or Scratch"

**Rules:**

- **The engine grows in modules.** A game takes only the modules it uses,
  and a developer learns only those. The default engine stays small.
  [ADR-0009](adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
  says what a module is; ticket 2003 builds the catalog and the way a
  game opts in.
- **A developer sets a direction, then adds modules that fit it.** A
  direction is a choice per concern: the perspective (top-down grid,
  top-down free, isometric, side view), how bodies move and animate
  (drawn clips, rigid-body physics, procedural animation), and so on.
  Modules that answer the same concern differently are alternatives, and
  the engine says so instead of letting them clash.
- **It is obvious what each piece is for.** Every module and kit says
  which direction it is built for. Where the type system can enforce it,
  it does: a side-view controller can't be used in a top-down stage.
- **Moving between directions is documented.** A game may change
  perspective between stages (ADR-0003 §1). Each supported move has a
  guide and an example. Ticket 2004 writes the guides, one per direction
  and one per move.
- **Many kinds of game, where it makes sense.** Each module names the
  kinds of game it serves and shows one in an example. A module is
  written when a kind of game needs it, not to fill a list.
- **Visual programming, eventually.** "Blueprints or Scratch" is the same
  goal as on 2026-10-14. It stays the parked research ticket 1135
  (ADR-0005 §5: a graph is a view over text). Modules keep their
  settings as data, so a visual editor can show them.
- **Procedural animation is a module, and it is parked.** Bodies and
  limbs that move by rules each step instead of by drawn frames, as in
  Rain World.
  [ADR-0010](adr/0010-procedural-animation-bodies-limbs-and-strips.md)
  sets its design, and tickets 2701–2704 are `Parked:`. They start after
  the MVP and after the real-time tickets they build on, when Nick
  unparks them. Visions of Shuyi doesn't need them.
- **Inspired by, nothing copied.** No name, creature, art or sound from
  Rain World goes into the engine or its examples.

## 2026-10-17: true 3D, HD-2D, and the GBA and DS

Source: Nick, in a Claude Code session on 2026-10-17.

> "I want you to add the following to the roadmap as engine features
>
> * So we added template support in future, with a create-react-app like
>   interface, I want to expand on this and add true 3D support (all
>   kinds, first person, third, top down with 3D models), and HD-2D (like
>   final fantasy remakes)
> * More port support, of course, not all hardware supports all
>   features, but anyway, I want GBA, and DS ports. Right now I think we
>   have native PC, with consoles planned, and Playdate in infancy. The
>   other targets require their own considerations."

**Rules:**

- **The engine is no longer 2D only.** This lifts "without 3D" from
  2026-10-14 (ADR-0003). 2D stays first: everything already planned
  comes before 3D, and the default engine stays 2D. 3D is modules a game
  opts into (ADR-0009).
- **Three 3D directions and one hybrid.** Each is a template for
  `shuyi new`, with its guide and example, as every direction has
  (ADR-0009 §5–§6):
  - **first person**;
  - **third person**;
  - **top-down 3D**: an overhead camera over 3D models;
  - **HD-2D**: 2D pixel-art characters in a 3D diorama world, with
    lighting, depth of field and bloom, as in Square Enix's HD-2D games
    (Octopath Traveler, the Dragon Quest III and Live A Live remakes).
    It is drawn by a skin, so the rules of a 2D game, grid tactics
    included, run under it unchanged.

  [ADR-0011](adr/0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md)
  sets the design. Tickets 3101–3109.
- **The GBA and the DS are export targets.** Each is a profile, a
  backend and a device ticket, as ADR-0008 set for every target.
  [ADR-0012](adr/0012-gba-and-ds-targets-capabilities-indexed-colour-two-screens-and-homebrew.md)
  sets the design. Tickets 3017–3027.
- **Not every target runs every feature.** A target's profile says what
  its hardware can do (3D or none, colours, screens, memory), and each
  module says what it needs. A game that uses something its target
  lacks doesn't export there: `shuyi check --target` names what doesn't
  fit, as the Playdate's controls rule does. The engine never drops a
  feature silently to make a game fit. Planned:
  - **GBA**: 2D only;
  - **DS**: 3D within its hardware's limits, HD-2D in a reduced form,
    and two screens;
  - **PC, the web, the Steam Deck**: everything.
- **Inspired by, nothing copied.** No name, art or asset from Square
  Enix's games goes into the engine or its examples, and nothing of
  Nintendo's (logos, SDKs, firmware) goes into this repository.
- **All of it is parked.** Nothing in Visions of Shuyi needs 3D, HD-2D,
  the GBA or the DS, and the MVP is still parity with the game. Tickets
  3017–3027 and 3101–3109 are `Parked:` until Nick unparks them.
- **To ask Nick when unparked** (each ticket's *Nick input* says which):
  - **Selling GBA and DS games.** Nintendo no longer licenses either
    system, so a game ships as homebrew: a ROM for flash carts and
    emulators, or a cartridge from a small publisher. Booting needs
    Nintendo's logo in the ROM's header. A lawyer's look is worth it
    before anyone sells one (3018).
  - **Which comes first**, 3D or the handhelds, if both are unparked.

## 2026-10-17: a wizard for new games, and MIDI music

Source: Nick, in a Claude Code session on 2026-10-17.

> "basically I want some create-react-app type command with templates and
> let it be extensible too but it should be like do you want top down?
> side scroller? and it can be both (i.e. transition between them) so let
> player tick off multiple
>
> and if they want physics engine, what kind of physics, if they want
> MIDI sound, what export targets they want to support, etc."

Asked when the wizard should be built, Nick chose **after the MVP**, the
recommended option. Asked whether MIDI should be parked, since the game
doesn't need it, he chose **parked**, the recommended option.

**Rules:**

- **`shuyi new` asks.** With no template named, it asks the questions a
  new game needs, one per concern, and each answer turns on what it
  needs. The questions:
  - the perspectives, more than one allowed: top-down, side view, and so
    on. With more than one, the new game gets a stage for each and a way
    to move between them;
  - how bodies move: no physics, the movement kit, rigid bodies, or
    procedural animation;
  - the music: recorded files, MIDI, or both;
  - the export targets, more than one allowed;
  - online play.
- **Every answer can be given without the questions**, as flags or as a
  file, so agents and CI make the same games people do.
- **Templates can be extended.** A template is data, and anyone can write
  one. The engine's own templates use the same format.
  [ADR-0013](adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)
  sets the design. Tickets 2501–2505 build it, after the MVP.
- **Planned answers are shown, not offered.** An answer whose module
  isn't built yet, or is parked, appears with its ticket. It can't be
  picked until it is ready.
- **MIDI is a module, and it is parked.**
  [ADR-0014](adr/0014-midi-music-sequences-and-a-sound-bank-in-a-module.md)
  sets its design. Tickets 2205 and 2206 are `Parked:` until Nick
  unparks them. Visions of Shuyi doesn't need them.

## 2026-10-18: real-time strategy at scale, visual novels, detective games

Source: Nick, in a Claude Code session on 2026-10-18.

> "I wanna be able to support high unit count RTS games and also visual
> novel style games and detective games..... make tickets parked for
> after visions of shuyi launches for all of these, I would imagine
> thousand+ unit count RTS games need advanced optimized AI routines
> provided"

**Rules:**

- **Three more kinds of game, each a module** (ADR-0009): the engine
  names the kinds of game it serves, and these three join the list.
  - **Real-time strategy with a thousand units or more**, and the AI to
    run them: pathfinding shared by whole armies, units that steer
    around each other, squads, and computer opponents that manage an
    economy and an army. The engine provides these routines, so a game
    doesn't write them.
    [ADR-0015](adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
    sets the design. Tickets 2705–2720, 2733 and 2734.
  - **Visual novels**: characters and backgrounds, text, choices, the
    backlog, skipping read text, going back, saves anywhere, a gallery,
    a route chart.
  - **Detective games**: clues, investigation scenes, deductions,
    testimony with contradictions, and a check that every case can be
    solved from what the player can find.

  [ADR-0016](adr/0016-visual-novels-and-detective-games-a-story-stage-a-case-file-and-a-fair-play-check.md)
  sets the design of both. Tickets 2721–2726 (visual novels) and
  2727–2732 (detective games).
- **"Thousand+" is a number the engine is tested against.** The RTS
  module's performance budgets (2718) start at 1 000 units moving and
  fighting at 60 steps a second on the Steam Deck, with 5 000 as the
  stretch. Every number is provisional until its device ticket measures
  it, as ADR-0008 set.
- **The AI is the engine's, and it stays fair and deterministic.** A
  computer opponent sees only what its side sees, unless its difficulty
  settings say otherwise, and it acts only through the commands a player
  could give. It thinks in work units, never time (ADR-0008 §6), so it
  decides the same on every device and replays stay exact.
- **All of it is parked until after Visions of Shuyi launches.** That is
  later than the other parked work, which waits for the MVP: these
  tickets wait for the game's release. Nothing in the game needs them.
  Tickets 2705–2734 are `Parked:` until Nick unparks them.
- **Inspired by, nothing copied.** No name, character, story, art or
  sound from existing strategy games, visual novels or mysteries goes
  into the engine or its examples.

## 2026-10-19: a visual editor, and a debug view in real time

Source: Nick, in a Claude Code session on 2026-10-19.

> "add tickets for UI layer for visual code -- like Godot editor or
> Unreal Blueprints + seeing level layouts and collisions bounds and
> skeletons in realtime (debug mode)
>
> if it doesn't already exist in our roadmap"

**Rules:**

- **What already existed.** Visual scripting was the parked research
  ticket 1135 (2026-10-14, ADR-0005 §5: a graph is a view over text).
  It decided the graph, not an editor around it, and nothing showed a
  running game's layouts, bounds or skeletons.
- **The debug view comes after the MVP, not parked.** Tickets 2315 and
  2316 draw gizmos over the running game by channel, and pause and
  step it. They need nothing parked, and they help any game, Visions
  of Shuyi's grid maps included. They never come before the game's
  Chapter 1 work.
- **Each channel is parked with what it draws.** Collision (2317),
  level layouts (2318), skeletons (2319) and 3D (2320) draw parked
  work. Each starts when that work is unparked and done.
- **The editor is parked, with 1135.** Research on the workspace
  (2506), then its shell with a live view, a tree and an inspector
  (2507), then the graph panel (2508). It stays a view over text files:
  agents and git read the same files a person edits in it.


## 2026-10-19: a frame cap of 60 by default

Source: Nick, in a Claude Code session on 2026-10-19.

> "I want a default engine fps cap at 60
>
> this can be configurable by developer for their game, turned off
> explicitly, turned on to higher value whatever they like, and should
> respect vsync and VRR
>
> this is because too many engines make fps caps unlocked by default
> which causes even simple games to max CPU which just feels bad
>
> options menu should expose this and we can have a messaging system to
> change it safely i.e. engine::set_cap(120) --> and we have some
> promise or something similar in rust-native terms..."

**Rules:**

- **60 by default.** A game made with the engine runs at most 60 frames
  a second unless its developer says otherwise.
- **The developer decides the default and the choices;** the cap can be
  turned off, but only on purpose.
- **The player can change it** on the options screen.
- **It never fights the display:** with vsync or VRR the cap adds no
  stutter.
- **A game asks for a new cap and can check it took effect.** Ticket
  2321.
