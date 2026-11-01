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

## 2026-10-19: the platform layer goes in-house, and the end goal

Source: Nick, in a Claude Code session on 2026-10-19, after asking how
much the engine leans on frameworks. The check (crates.io and the
repositories, 2026-10-19): the core crates (`shuyi-core`, `shuyi-ui`,
`shuyi-assets`) use no framework. Only `shuyi-app` does, through
macroquad, quad-snd and gilrs. gilrs was released in January and May
2026, with commits into August 2026. quad-snd's last release was in May
2024, and the OGG and WAV readers it brings (`audrey`, `lewton`) were last
released in January 2021. The web build's saves (`quad-storage`) were
last released in August 2021, and `include_dir` in June 2024. miniquad,
under macroquad, draws with OpenGL, GLES, WebGL 1 and Metal, and its
README rules out Vulkan and DirectX 12.

> "I think probably at some point we can look at swapping this out for
> in-house. Anything else that's framework sized? text file parsing is
> fine, math is fine"

> "lets check the other deps make sure they have good maintenance...
>
> anyway here's my ordering and reasoning...
>
> 1st - replace gilrs bc that is scary af
> 2nd - replace quadsnd for same reason
>
> lets scope other candidates here...
>
> eventually - macroquad replace
>
> and super down the line - carve each of these new things into their
> own repos which we pull into engine
>
> but this will be super down the line bc we want to make the engine
> work well enough and I want them to be full fledged replacements
> first. Absolutely do not steal source code line by line.
>
> These will also be permissive use like the engine.
>
> And the end goal for my Shuyi engine is to be the #1 AI friendly Rust
> cross plat 2D engine (eventually 3D) with a wide variety of ports!!
>
> it should take less than an hour to make a wide port range game with
> basic functionality, like snake or mario. You should be able to say
> "make me snake game with shuyi engine" and ai can do it super quick,
> with a web ver, a playdate ver, etc. all done in not too many LOC."

Told that gilrs is maintained and quad-snd is the one that went quiet, he
chose **sound first**. Asked when the work starts, he chose **parked until
the MVP**. Shown the full list, he confirmed:

> "Yes: sound (with our own OGG/WAV readers), controllers, the two small
> ones (web saves, asset packing), then macroquad, then separate repos.
> All parked until MVP."

On graphics:

> "it looked like macroquad might be lagging on support for graphics
> libs like GL and Vulkan and Metal etc we should have a wide compat
> with all GFX libs"

**Rules:**

- **The end goal.** Shuyi Engine aims to be the most AI-friendly
  cross-platform 2D engine in Rust, 3D later, with many ports. The
  measure: an agent asked "make me a snake game with Shuyi Engine" makes
  it, with a web build, a Playdate build and the other targets, in under
  an hour and in few lines. Every new API is judged against it.
- **The platform layer goes in-house, in this order:**
  1. sound, replacing quad-snd and macroquad's audio. The research
     comes first (2902), then our own WAV, Ogg and Vorbis readers, a
     mixer and sound output on each platform (2210–2214, 3030–3032), if
     the research keeps that plan;
  2. controllers, replacing gilrs. The research comes first (2903), then
     2402 and 2403 if it keeps that plan;
  3. the two small ones: saves on the web, replacing `quad-storage`
     (3033), and putting a game's files in its binary, replacing
     `include_dir` (2215);
  4. macroquad itself: the window, its input and the graphics, through
     winit and wgpu with our own 2D renderer on top. The research comes
     first (3034).
  5. Each replacement moves to its own repository, which the engine
     depends on (2008).
- **All of it is parked until the MVP.** Nobody starts these tickets
  until Nick unparks them. The repositories (2008) wait longer, until
  each piece is a full replacement, and Nick says when.
- **Written by us, never copied.** No source code is copied, translated
  line by line or closely paraphrased from another library. Each piece is
  written from the format's specification and the operating system's
  documentation. [ADR-0021](adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
  says how.
- **Same license as the engine.** Every new piece is MIT OR Apache-2.0.
- **Every graphics API, through wgpu.** The engine draws on Vulkan,
  Metal, Direct3D 12, OpenGL and GLES, and WebGL 2 and WebGPU on the
  web, not only what miniquad covers. Asked whether that meant writing a
  backend per graphics API ourselves, Nick said no:

  > "I mean... I don't wanna rewrite graphics bindings... so I think wgpu
  > is safe...?
  >
  > macroquad is higher level than this by a mile afaik."

  So the graphics go through `wgpu` (MIT OR Apache-2.0), which covers all
  of those APIs, and our own 2D renderer sits on top of it. 3034 plans
  it.
- **The window through winit; controllers and sound decided after
  research.** Told that Bevy (the engine Nick names as the one to rival
  someday) uses winit for its window, wgpu for its graphics, gilrs for
  controllers and rodio for sound (checked on crates.io, Bevy 0.20,
  2026-10-19):

  > "we can use winit since it's not really something I wanna
  > reimplement. The gilrs seems low stars and barely maintained so I
  > still plan to skip it... I feel like of everything, controller is
  > quite easy to add support for but this can be a 2 way door decision
  > and we can make it much later in the future after additional
  > research in a dedicated ticket. And for audio, we can look at our
  > options as well when we get there."

  So:
  - the window and its input go through `winit` (Apache-2.0), as in
    Bevy. Our own code is the run loop's glue and the 2D renderer;
  - controllers: the plan is still to leave gilrs, but how (our own
    code, or another library) is decided by a research ticket, 2903,
    before 2402 and 2403 start. It can be changed later either way;
  - sound: the same, with 2902 deciding before 2210–2214 and
    3030–3032 start. quad-snd still goes.

## 2026-10-20: Steam achievements and cloud saves get tickets

Source: Nick, in a Claude Code session on 2026-10-20, after a comparison
with Bevy 0.20 and Godot 4.6 found features no ticket covered, and two
of them (Steam achievements and cloud saves, and reading Spine's files)
were left as questions for him:

> "anyway make sure steam achievements and cloud saves are covered by
> some ticket so it's not lost"

So, taking the recommendation he was offered (he may veto it):

- **The engine gets an achievements service with no store in it** (3036):
  achievements and stats as data, a local backend for builds with no
  store, a test backend, an unlock notice and an achievements screen.
- **Steam goes under it, behind the `steam` feature** (3037): Steamworks
  achievements and stats, and saves in Steam Cloud (Auto-Cloud by
  default, the Cloud API when a game needs it). 1144 already puts
  Steamworks in the engine behind that feature for networking.
- **What stays the game's** (2026-10-15, game ticket 0903): the Steam
  partner account, the app id, the store page, and setting up the
  achievements and the cloud quota on the Steamworks site.
- Both tickets are parked like the rest, until after the MVP.

Reading Spine's files is still open: Nick asked for more checking first.

## 2026-10-20: the engine may read Spine's files, with a reader of our own

Source: Nick, in the same session, after the Spine Editor License
Agreement (last updated 2025-04-05, `esotericsoftware.com/spine-editor-license`)
and the `EsotericSoftware/spine-runtimes` repository were read. Section 2
of the agreement and its Exhibit A (the Spine Runtimes License) restrict
integrating Esoteric's runtime libraries, or works derived from them.
They say nothing about a reader written from the public format
documentation. The editor's own license terms (no commercial use of
Trial or Education work, Enterprise above the revenue threshold) bind
whoever makes the files.

> "to me it sounds like we can load spine output files so long as user
> is using their own ver of spine to make them...?"

Then:

> "Yes. Record in decisions.md that the engine may read Spine files with
> our own reader, written from the public format docs with no Spine
> code, and the user brings their own Spine license. Add a parked Spine
> JSON import ticket beside 2216 on PR 24."

So:

- **The engine may read Spine's JSON and atlas files** with a reader we
  write ourselves (2217, parked like 2216; the next entry makes this one
  of two paths).
- **It is written from Esoteric's public format documentation only.** No
  code from the Spine Runtimes is copied, translated or read while
  writing it, so the engine is neither the Spine Runtimes nor a work
  derived from them. This is ADR-0021 §2's rule (written from
  specifications, never copied) applied to a file format.
- **The user brings their own Spine license.** Making the files is
  between the user and Esoteric. The engine's docs say so, and say
  that Spine is Esoteric Software's trademark and the engine isn't
  affiliated with it.
- **The official runtimes stay out, for now.** Wrapping them (as
  `rusty_spine` and `bevy_spine` do) is allowed by Esoteric. Each
  developer who builds a game with them needs their own Spine license
  (Section 2.4), and with it may ship the game to players who have none
  (Section 2.2). But the runtimes' license isn't one of the permissive
  licenses ADR-0002 §8 allows, so using them is a separate decision for
  Nick.
- This is a reading of the license, not legal advice. If Esoteric ever
  objects, or changes its terms to cover the file format, the reader
  stops shipping until Nick decides again.

## 2026-10-20: Spine import: an Essential license or our own reader, picked when unparked

Source: Nick, in the same session, after the entry above was corrected:
Esoteric allows its runtimes inside an engine, as long as each developer
who builds with them has a Spine license, and a developer with one may
ship a game to players who have none. On Esoteric's forum (thread 26587,
2024-07-25), Esoteric's Nate confirms that Essential or Professional is
enough (below the $500k revenue threshold) to put the runtimes into any
number of one's own products.

> "sounds like probably essentials license is all we would ever need and
> it's $69 so I might just pull the trigger if we get to this milestone.
> Otherwise we can park the ticket as TBD between essentials license and
> our own reader but I have a feeling our own reader would cost more than
> $69 in tokens"

So:

- **Spine import stays parked** (2217), and its first step is Nick's
  pick between two paths:
  - **A. The official runtime under an Essential license** ($69,
    checked 2026-10-20), in an opt-in `shuyi-spine` module. Nick leans
    this way: our own reader would likely cost more than $69 in tokens.
  - **B. Our own reader**, written from the public format docs with no
    Spine code, as the entry above sets out.
- **If he picks A**, it carries duties to record then:
  - an exception to ADR-0002 §8 for that one crate: its license is the
    Spine Runtimes License, not a permissive one. It is never on by
    default, and the engine's repository holds no Spine code;
  - every developer who turns it on needs their own Spine license, and
    the module's docs say so;
  - the Essential license is for one named user (Nick), and ends if
    revenue, investment or financing passes $500k in 12 months. What was
    integrated while it was valid may still ship (Section 2.2); new
    integration then needs Enterprise. (Refined in the next entry: the
    engine needs no license of Nick's to carry the runtime.)

## 2026-10-20: Spine and a free engine; the game never uses Spine

Source: Nick, in the same session:

> "given our engine is intended to be FOSS I think this $500K doesn't
> apply, and visions of shuyi will never use spine so it should be
> stripped in shipping"

What the terms say about it (the Spine Runtimes License, Exhibit A, and
the `spine-runtimes` README, read 2026-10-20; not legal advice):

- **A free engine needs no license of Nick's to carry the runtime.**
  Exhibit A's second branch lets anyone integrate the runtimes into
  software free of charge, as long as each user of that software has
  their own Spine license. The README says toolkits usually take this
  branch. So `shuyi-spine` takes it: the engine pays nothing, and each
  developer who turns the module on brings their own license.
- **The $500k threshold is about the license holder, not the product.**
  It counts all of the holder's revenue, investment and financing, so a
  game's sales count even when the engine is free. It matters only for
  whoever holds an Essential or Professional license: each developer for
  their own games, and Nick only if he holds one to test the module
  (running it is using it). If Nick's revenue passes $500k, testing the
  module needs Enterprise, or a maintainer with their own license.
- **Visions of Shuyi never turns `shuyi-spine` on.** The module is
  opt-in and never a default feature, so the game's builds hold no Spine
  code. The game's own `cargo deny` allows no Spine license, so the
  module can't come in by accident.

So path A of 2217 no longer needs Nick to buy anything for the engine.
He needs a license only to test the module himself.

## 2026-10-20: everything on Bevy's homepage, and a website like Bevy's

Source: Nick, in a session of this repository:

> "every feature outlined on Bevy's homepage should be available in
> Shuyi engine at some point in the not so distant future.
> https://bevy.org/ Ensure we have all features outlined there like hot
> reload, fast compile, ECS system (perhaps optional)"

> "also would be good if we have extensive documentation, a robust
> tutorial, a newspage with automatic changelog, a devlog if I want to
> post a manual update to community, and some sponsor segment. probably
> hosted on github pages to start and can use custom domain later."

Asked when the work happens, he chose the recommendation: **"New after
MVP"**. Asked how people sponsor the engine: **"GitHub Sponsors +
Ko-fi"**. Asked how he posts a devlog:

> "just a normal blog that's hosted in a larger dev website the way Bevy
> does it... I can pick if I wanna use claude or use my own hands and
> brain..."

So:

- **Every feature on Bevy's homepage (read 2026-10-20) has a ticket.**
  The list and where each one lives is in `docs/ROADMAP.md`, *Bevy's
  homepage*. Features no ticket covered get new tickets.
- **The new engine tickets come after the MVP, not parked.** They are
  ordinary tickets, picked up once the MVP's are done. The ECS is
  **optional**: an opt-in module beside the arena of plain structs
  (ADR-0003 §4 stays the default).
- **Bevy features already parked stay parked** until Nick unparks them:
  3D (3101–3109), the wgpu renderer (3034, and 2326–2328 on it),
  skeletal animation (2105), and mobile (3015). New tickets that need
  them are parked with them.
- **The website is part of the MVP.** It replaces the docs site in
  1132, which keeps the community files. It is on GitHub Pages, with
  room for a custom domain later (parked until Nick asks; buying a
  domain costs money). It has extensive docs, a tutorial, a news page
  with a changelog made automatically, a blog, and a sponsor page.
- **The blog is an ordinary blog on the website, like Bevy's news.** A
  post is a Markdown file. Nick writes it himself or asks Claude for a
  draft; either way it is his choice, and merging it publishes it.
- **Sponsors through GitHub Sponsors and Ko-fi.** Nick enrolls in both
  himself (money, bank and tax details are his). The ticket lists his
  steps and checks both platforms' fees on the day.

## 2026-10-20: a second comparison, with Unity and Unreal; the engine is built for one game first

Source: Nick, in a session of this repository:

> "I want to ensure we're on track for long-term maintainability, and
> the ability to have a competitive edge. Check our current features,
> our planned features, against the best Rust engine (Bevy, presumably)
> and the best normal game engine (presumably Godot, Unreal, or Unity).
> See if there are obvious gaps in our planned features, or the way we
> are building towards our goals, against these benchmarks."

The comparison (2026-10-20) read Bevy 0.20, Godot 4.7, Unity 6 (6.3 LTS
and 6.6), Unreal Engine 5.8, and the 2D engines GameMaker, Defold and
Construct 3. It looked at features, and at how the engine is built:
its process, its architecture, its upkeep over years and its edge. It
found about 118 gaps a ticket could close, in three groups. Asked when
each group happens, Nick chose the recommendation, **"Split by
group"**:

- about 14 small protections for the code go in the MVP, before the
  first release: a check that names API breaks, Dependabot, a build of
  Visions of Shuyi against the engine's `main`, governance, license
  notices in web builds, and logs and error codes agents can read;
- about 75 features most 2D games expect come after the MVP, not
  parked;
- about 26 for things the game doesn't need stay parked.

Asked whether the real-time work that his end goal's snake and Mario
need should be unparked:

> "unless these real-time work tickets overlap with visions of shuyi
> they should be parked until visions of shuyi is complete since it's
> the first game I want to be launched with the engine... think like
> how Jonathan Blow is handling his Jai programming language, its
> primarily built for one game but it happens to handle a lot of games"

Asked which new kinds of game get parked tickets, he picked
**roguelikes**, **a gallery of small games** and **hex grids**.

**Rules:**

- **Built for one game first.** As Jai is built for Jonathan Blow's
  games, the engine is built for Visions of Shuyi first, and happens to
  handle many other games. Work the game doesn't need never goes ahead
  of work it does.
- **Real time waits for the game's launch.** The real-time tickets
  (1119–1125 and 1127), and the tickets built on them (1126, 1128,
  2106, 2317, 2318, and this comparison's real-time tickets), are
  parked until Visions of Shuyi launches, not only until the MVP. This
  replaces "after the MVP" (2026-10-14) for them. What the game needs
  comes sooner:
  - the game's ticket 0060 (the army walks around freely) still
    unparks 1119–1125;
  - a piece the game asks for through its `engine-change` skill is
    split out and unparked on its own.
- **The end goal waits with it.** Snake and a platformer need that
  real-time work, so they come after the launch too. Until then the
  agent check of the end goal measures turn-based games.
  [`ROADMAP.md`](ROADMAP.md), *The end goal*, shows the chain.
- **The protections are in the MVP,** and the first release (2005) waits
  for the ones a release needs. [`ROADMAP.md`](ROADMAP.md), phase 2,
  lists them.
- **The features most games expect come after the MVP, not parked.**
  [`ROADMAP.md`](ROADMAP.md), phase 3, lists them in order. Some take
  the turn-game half of a parked ticket (the mouse in menus and on grid
  maps from 1121, rotated sprites and drawn shapes from 2703); the
  real-time half stays parked.
- **Three more kinds of game, parked until after the launch.**
  Roguelikes, a gallery of small games and hex grids join the kinds of
  game the engine serves (ADR-0009). Under the rule above they wait for
  the launch, as real-time strategy, visual novels and detective games
  do (2026-10-18), not only for the MVP. Nick may veto that reading.
- **Decisions left for later** are in the tickets that need them
  (`nick_input`): how much the scheduled agent check may spend, paid
  code signing, a backup owner and the repository's settings, sending
  crash reports, the community directory and the names of other
  people's modules, phones' browsers as a target, machine-translated
  engine screens, and stores beyond Steam.

## 2026-10-21: the crate names are held on crates.io

Asked whether the `shuyi-*` names were free on crates.io, Claude checked
the 55 names the docs and tickets plan: none was taken, and a search for
"shuyi" found no crate at all. Ticket 2005 allowed holding the names
early with empty 0.0.0 versions. Nick:

> "is kebab case standard for Rust crates? if so, I guess get a headstart
> on 2005 and reserve the 6 for now"

Claude answered that both styles are common on crates.io (`tokio-util`,
`serde_json`), that crates.io treats `-` and `_` as the same name, and
that code writes `shuyi_core` either way, so the names stay as chosen.

**Rules:**

- **Six names are held.** `shuyi-core`, `shuyi-assets`, `shuyi-ui`,
  `shuyi-platform`, `shuyi-app` and `shuyi-cli` were published on
  2026-10-21 as empty 0.0.0 placeholders, owned by `Zafnok`. Each is a
  library with no code, both license files, and a README that points
  here. The repository's own crates stay `publish = false` until 2005.
- **The token.** Nick made it for this run only: the `publish-new`
  scope, the crates `shuyi-*`, a short expiry. It was revoked after the
  run. Its value is recorded nowhere.
- **Only these six.** The other planned names (`shuyi-sound`,
  `shuyi-playdate` and the rest) are not held. crates.io forbids
  holding names "for a prolonged period" with no development behind
  them, so each is published when its crate is real. (A seventh,
  `shuyi`, followed on 2026-10-22: see *the bare name `shuyi` is held
  too*.)
- **2005 publishes the next version of each** through trusted
  publishing, with no publish by hand.

Told that the placeholders would be published, Nick added:

> "yea for now we're just doing name reservation, not any publishing
> until we're more stable unless we have to in order to keep the name
> reservation"

- **No real version before the engine is stable.** The placeholders
  hold the names; 2005 still waits for the MVP and the protections it
  is blocked by. The names do not hurry it.
- **Unless the names are at risk.** If crates.io questions a
  placeholder (its policy lets the team ask an owner to justify a
  crate before removing it), the answer points at this repository's
  activity. A real version goes out early only if that is what keeping
  the names takes, and Nick is asked first.

## 2026-10-22: weather, day and night, voice chat and ray tracing get tickets

Source: Nick, in a Claude Code session on 2026-10-22:

> "some random thoughts I have are supporting weather system, day/night
> cycle, and proximity audio and voice chat (voice chat in multiplayer
> enabled games)
>
> also raytraced path traced visuals and audio"

Told which already had tickets and which were engine or game, and that
ray tracing would be a poor fit for the small targets, he added:

> "I mean obv the ray and path tracing would be for diff comp targets
> like PC, new gen consoles and wouldn't be shipped for i.e. GBA... we
> already have feature toggles and target toggles in our roadmap
> right...?"

He then chose to have parked tickets written for the gaps.

**Rules:**

- **Proximity audio already had a ticket**: 2363 (positional 2D sound),
  parked with the real-time work.
- **A day and night is engine look, the day's meaning is the game's.**
  2369 blends named lights over the game's own clock; when shops close
  or what comes out at night stays the game's.
- **Weather's rules are the game's.** 2370 brings its look and sound
  together (particles across the view, a tinted overlay, a looped
  sound, a fade between them); what rain or snow does is the game's.
- **Voice chat is researched first, and shipping it is Nick's call.**
  2910 ends with a question to him on the duties it brings
  (moderation, minors, the stores' rules).
- **Ray and path tracing are an opt-in module for PCs and current
  consoles.** It is behind a new `ray_tracing` capability (ADR-0012 §1,
  3017) that the Playdate, the GBA and the DS never have, and is
  optional by default, so a game that turns it on still runs elsewhere
  with its usual look. 2911 researches light (3D and 2D) and sound.
- All four tickets are parked like the rest, until Nick unparks them or
  the game asks for one.

## 2026-10-22: the bare name `shuyi` is held too

Asked whether more crate names should be held, Claude checked crates.io
again: the six placeholders are live, and `shuyi` was still free. Ticket
2003 plans a crate by that name (`crates/shuyi`): it re-exports the other
crates and has one feature per module, so it is the one crate a game
adds and the name people type first. The 2026-10-21 run left it out only
because the crate doesn't exist yet. Nick:

> "Yes, hold `shuyi` too."

**Rules:**

- **`shuyi` is held** as an empty 0.0.0 placeholder like the six (a
  library with no code, both license files, a README that points here),
  owned by `Zafnok`. A crate with real work planned behind it (2003)
  fits the 2026-10-21 rule against holding names with nothing behind
  them.
- **The token.** The 2026-10-21 token's crate scope `shuyi-*` doesn't
  match the bare name, and it was revoked anyway. Nick made a new one
  for this run only: the `publish-new` scope, the crate `shuyi` only, a
  short expiry, revoked after the run. Its value is recorded nowhere.
- **Still no defensive names.** `shuyi-engine` (the repository's name)
  and `shuyi_engine` are free, but no crate is planned under them, so
  they are not held; crates.io's policy is against exactly that. The
  planned module crates are still published when each is real.
- **2003, not 2005, publishes its first real version**, since 2003
  (after the MVP) is where the crate is written. It goes through trusted
  publishing like the others, with no publish by hand.

## 2026-10-22: dungeon crawlers, Diablo-likes and more roguelike pieces, engine-shaped, with a demo per genre

Source: Nick, in a Claude Code session on 2026-10-22:

> "another game type I like a lot is Wizardry first person dungeon
> crawler, especially the intro to Variant Daphne... what I would want
> most is a single player party blobber (i.e. 1 character only not 4 or
> 6 party) but anyway support for any character amount is ok... but if
> we can have tickets to add support for this type of game too, however
> you think makes sense to add it or if you think all the tickets in our
> backlog will already support it you can let me know too"

While the tickets were being written, he added:

> "I also want support for roguelikes, and Diablo-likes"

> "ensure these are all 'engine-shaped' tickets and not 'game-shaped'
> tickets -- be sure they pass the bar to be engine features or provided
> templates rather than specific game features shipped in an engine"

> "though certainly we can build one of each genre as a demo of the
> engine..."

**Rules:**

- **Nothing in the backlog served a first-person crawler.** The grid kit
  has cells, not walls between them, and nothing drew a grid from
  inside or ran a party RPG. ADR-0033 adds two modules: `shuyi-crawl`
  (the labyrinth: 2741–2744, 2753, 2755) and `shuyi-rpg` (a party of
  any size, sheets, items, battles, abilities, encounters, shops:
  2745–2752), with the example and template in 2754.
- **A party of one comes first.** Every part of `shuyi-rpg` is tested
  with a party of one and a full one. The crawler template starts with
  a party of one (`max: 1`); raising `max` gives a full party with no
  other change.
- **Daphne's opening needs nothing new.** It is a cinematic, then
  dialogue over the labyrinth's view before the first step. The crawler
  example has an opening of that shape, with its own story.
- **Diablo-likes** get ADR-0034: a `shuyi-action` module for real-time
  hits, skills, packs and the HUD, loot rolled from affix tables and an
  inventory grid in `shuyi-rpg`, and waypoints in the default crates
  (2756–2764). They wait for the real-time work, like the rest of it.
- **Roguelikes already had tickets** (2118–2120, 2735). The gaps were a
  turn order by speed (2765), a message log (2766), progress kept
  between runs (2767), and the genre's habits in its template (2768).
- **The bar for engine work** (ADR-0033 §17). A piece goes in an engine
  crate when at least two different kinds of game would use it as it
  is and it names no game's content. A genre's habit (Wizardry's
  trapped chests, a town of an inn and a temple, permadeath, potions
  that look different each run, a town portal) goes in that genre's
  template, as code and data the new game owns, edits or deletes. Each
  engine ticket says which kinds of game use it.
- **One demo per genre.** Each genre's example is a small complete game
  made only from the engine, with its own story, numbers and art made
  for it, copying no other game. It is also the genre's integration
  test.
- All of it is parked until after Visions of Shuyi launches, like the
  other kinds of game. The 3D labyrinth skin (2753) also waits for the
  3D work.

## 2026-10-24: perf traces agents can read, in the MVP, with the live link

Source: Nick, in a Claude Code session on 2026-10-24, while the skill
pack (1134) was being worked:

> "we should also have easily agent-readable / streamable perf traces
> when building in debug -- both timings and heap snapshots"

The frame profiler (2325) and memory in the debug view (2368) draw such
numbers on screen, and both are parked. 2368 left "the reading in the
contract's answers or over the live link" to a follow-up "if an agent
or a developer asks for it". The follow-up is six tickets: frame times
and spans as data, the trace through `run`, MCP and a running build's
file, a heap counter, memory tags, heap snapshots in the trace, and the
trace over the live link. None of them needs the debug view.

Asked when those tickets happen, he chose **"In the MVP"**. The
recommendation was after the MVP, not parked. Asked where the part over
the live link goes, since the live link (2521) came after the MVP, he
chose **"Move 2521 into the MVP"**. The recommendation was a trace file
in the MVP and the link part later.

**Rules:**

- **Perf traces are in the MVP:** frame times and spans (3063), the
  trace through `shuyi run`, MCP and a running build's file (2553), the
  heap counter (3064), memory tags (3065), heap snapshots and diffs in
  the trace (2554), and the trace over the live link (2555).
  [`ROADMAP.md`](ROADMAP.md), phase 2, has their order.
- **The live link (2521) is in the MVP.** The link for the web build and
  a Steam Deck on the LAN (2522) stays after the MVP.
- **Only debug builds measure,** and a new game's debug builds count
  the heap with no setting to turn on, so the traces are there when an
  agent asks.
- **The on-screen views stay parked.** 2325 and 2368 draw what these
  tickets measure, and wait for Nick as before.

## 2026-10-24: local split screen for two to four players, parked

Source: Nick, in a Claude Code session on 2026-10-24:

> "add parked tickets for adding local split screen multiplayer, both
> vertical and horizontal, and lets say even 3 or 4 player split
> screen. These will be parked since they're not needed for Visions of
> Shuyi"

Seats (2406) and a viewport per world layer with one fixed split per
player count (1120) were planned already. What no ticket had: a choice
of layout (two side by side or stacked, the three-player shapes, four
as a square or strips), a view per seat with its camera, the spare
quarter with three players, a check that views fit a small screen, a
listener per view, an example and a template part.

**Rules:**

- **Split screen is parked** with the rest of the real-time work, until
  Nick unparks it: 2385 (the layouts and the player's pick), 2386 (a view
  per seat and the check per target), 2387 (a listener per view), 2774
  (the example) and 2775 (the template part).
- **Up to four players**, the same cap as 2406's seats.
- **Both directions for two players**: side by side (a vertical
  divider) and stacked (a horizontal one). The game says which it
  allows, and the player picks among them in the options.

## 2026-10-26: an A/B against Bevy, on a maze chase, parked with real time

Source: Nick, in a Claude Code session on 2026-10-26:

> "can we do an A/B test how easy it is, how many LOC it takes, to make
> a Pac-Man clone in say Bevy vs Shuyi Engine? We can host an example on
> our github pages and in our samples or wherever we're committing
> these"

A maze chase runs in real time, and the real-time work (1119, 2739) is
parked until Visions of Shuyi launches (2026-10-20, above). Built today,
the Shuyi side would fake real time, the pattern 2739 says no agent
should write. Asked how to go, with four options (a turn game compared
now; unpark real time for it; the maze chase as-is today; a parked
ticket), none marked as recommended, he chose **"Ticket it, parked"**.

Then, on what the A/B is for:

> "our goal should be 1) agent can one shot games easier with 2) less
> friction and 3) more deterministic handling (i.e. we have skills and
> MCP) and 4) approx equal LOC (this matters less since it's agent first
> engine but would be a less subjective benchmark)"

**Rules:**

- **The A/B is parked** with the real-time work, until Nick unparks it:
  2776 (the maze chase, Shuyi's reference) and 2562 (the A/B itself).
  Unparking 1119 and 2739 doesn't unpark these; Nick says.
- **What it measures, in this order:** whether an agent makes the game in
  one shot, the friction on the way (failed builds, retries, minutes,
  tokens), how deterministically it checks its work (replays, the
  contract and MCP, against looking at the screen), and the lines it
  writes. Lines count least, but they are the measure least open to
  argument, so they are always reported.
- **The same task on both sides:** the same agent, model, prompt (only
  the engine's name changes), time box and budget, run several times
  each.
- **Its games and its numbers go on the engine's GitHub Pages**, and the
  games' code is committed in this repository. The agents' transcripts
  are not published unless Nick says.
- **It is a maze chase, not Pac-Man.** Pac-Man is Bandai Namco's
  trademark, so the A/B uses no name, character, maze layout, sound or
  art from it. Nick may veto this rule.

## 2026-10-26: no build of Visions of Shuyi here; the game's pin moves find its breaks

Source: Nick, in a session of the game's repository, asked whether 2014
(a CI job building the game against each engine PR and nightly `main`)
was still needed now that the game moves its pin by a watcher:

> "the pin system seems better, than having to run whole CI for game as
> well as engine which is what 2014 seems to suggest... should we not
> just axe 2014? [...] I think it's wholly unnecessary"

On 2026-10-20 (*a second comparison*, above) he put "a build of Visions
of Shuyi against the engine's `main`" among the MVP's protections, and
2014 was that build. Since then the game's watcher on this repository's
`main` (game ADR-0096) files an `engine-pin` issue within minutes of a
public API change and within 48 hours of any other change, and each pin
move builds the game and runs all its tests. A break shows within two
days, a few PRs at a time, not weeks later mixed with every change since
(2014's reason). 2010's `api` job names each API change on the PR
itself. Of the game's first five pin moves, three engine PRs needed a
change in the game, each made in the pin move's own PR.

**Rules:**

- **2014 is dropped:** its file is deleted, and its number isn't reused.
  This repository's CI builds no copy of the game.
- **The game finds its own breaks** at its pin moves (the game's
  `engine-pin` skill). An engine PR that knows it breaks the game says
  so in its `Breaking:` line (2010), as before.
- **The first release (2005) no longer waits for 2014.** When the game
  moves from the git revision to released versions, the game's ticket
  for the move keeps its pin moves working on versions.

## 2026-10-28: pictures in the book, a playground, and a site that keeps up with the tickets

Source: Nick, in a session of this repository, after the book (2802)
went up, and while 2808 (`llms.txt`) was in its pull request:

> "would be good if the book/site also included some screenshots of how
> things play together, plus a playground, plus if it auto updates as
> tickets complete (I know the index.html auto updates but the
> subdirectories like book idk if they auto update as more stuff gets
> added or not yet)."

> "and the same note for llms.txt"

**Rules:**

- **The book shows the engine at work:** its chapters carry pictures of
  a game running, made by the engine's own tools from a script and
  checked like the book's code, so a picture can't go stale (2813).
- **The site has a playground:** a game playing in the page beside its
  data files, which a reader edits and runs again with no install. It
  edits files, not Rust code: compiling Rust in the browser needs a
  build server, which costs money (3069, 2814).
- **What the site says follows the tickets.** The home page's feature
  cards take their status from the tickets they name, and a change to a
  public item changes the book or says why not (2815). `llms.txt` and
  `llms-full.txt` are made from the Learn pages at each deploy (2808),
  so they follow the book, and the pictures and the playground are in
  them too.

## 2026-10-30: real demos: snake, the maze chase and the A/B unparked

Source: Nick, in a session of this repository:

> "we need better demos. hello, grid is garbage it's too easy and
> simple. now that our engine is growing robust we can have real demos,
> like snake or pac-man. I think we had pac-man clone referenced in the
> tickets parked behind realtime. what's the golden path to get there"

Shown the chain (2115, 1119's tick part, 2739, 2776, 2562) and that
real time waits for the game's launch (2026-10-20, above):

> "I think we've already unblocked visions of shuyi entirely from an
> engine perspective and now we're just doing victory laps. so this is a
> good time to add more features."

On that day the game's open requests here were wide glyphs and the line
breaking of Chinese and Japanese text (2301–2303), scripts (2101,
2102) and two tidy-ups of the bots (2133, 2544). None held up the game.
Asked how far the unpark reaches (all 30 real-time tickets, the demo
chain alone, or every one of the 193 parked tickets), he chose **"Just
the demo chain"**. Asked whether the A/B against Bevy unparks too, with
the cost of its agent runs, he chose **"After the maze chase"**.

**Rules:**

- **The demo chain is unparked**, in this order:
  - 2115 (timers, already open);
  - 2137, 1119's tick part split out of it: `TickSim`, records that
    replay, and `TickDriver` with the latch that gives each press to
    one step;
  - 2739 (snake and the arcade template part);
  - 2776 (the maze chase example);
  - 2572 (the maze chase's eval task, split out of 2776 so the game
    doesn't wait for the eval);
  - 2562 (the A/B). It still asks Nick what it may spend before any
    agent runs.
- **The rest of real time stays parked**, 1119 among them, now
  fixed-point numbers and the entity arena, after 2137. So do
  collision, cameras, the platformer, and the brick-breaker and the
  shooter (2738). The 2026-10-20 rule holds for them. A gap the demos
  find becomes a ticket, open if a demo needs it, parked if not.
- **The game still comes first** when it asks for something: a request
  from the game goes ahead of the demo chain.
- **Not Pac-Man** (2026-10-26): the maze chase takes no name,
  character, maze, sound or art from it.
- **The end goal's snake is measured again**: the agent eval's snake
  task gets its reference (2739) and is graded in CI once 2524 lands.

## 2026-10-29: a ticket is one PR of about 1,500 changed lines or fewer

Source: Nick, on the limit of about 600 changed lines:

> "Engine work is more technical than game so 1500 is about right there"

**Rule:** a ticket is one PR of about 1,500 changed lines or fewer, without
fixtures and snapshots (the `write-ticket` skill). Estimates and periodic checks
keep to it, and no CI check measures a PR.

## 2026-10-31: sound takes the hybrid path behind automated parity gates

Source: Nick, after asking to do research ticket 2902 and reading the
ownership options and their implementation risks:

> I want A but only if we can have some way of automated parity checking (speed + correctness). Additionally A should be surrounded by a first ticket which gates progress (i.e. do not immediately rm quad-snd) and slowly reduces dependency on it as we get confirmation each goal has been met... and end ticket removes it after everything is checked out

**Rules:** choose the hybrid ownership plan: library codecs and desktop
output, an owned small mixer and current-loader browser bridge. A first
ticket (2573) must establish working automated correctness and speed
checks before replacement implementation starts. Keep old and new paths
side by side; advance each stage only after its comparative checks pass.
A final ticket (3079) removes quad-snd and macroquad audio only after all
consumers and required behavior qualify. The research request unparks
2902 alone; the replacement and parity build tickets stay parked. The
technical plan is [ADR-0066](adr/0066-sound-uses-library-codecs-and-output-with-an-owned-mixer-and-parity-gates.md).

## 2026-10-31: public demo defenses get a ticket track

Source: Nick, after research into AI-assisted video imitation, demo extraction,
binary analysis, watermarking and capture controls:

> "ok all of these countermeasures are worth a ticket track so go ahead and make tickets in engine repo and make a pr"

**Rules:**

- Preserve separate tickets for what a game discloses, what its final
  package contains, hosted previews, release identity and bounded
  experiments. A future implementation still follows one ticket, one
  branch and one PR.
- The ticket track does not claim that a public clip can be made
  human-readable and reliably AI-inaccessible. Measure video and binary
  hardening before promising it as protection.
- Hosted networking work stays parked until Nick explicitly unparks
  it. Putting it in the backlog is not starting that work.

## 2026-10-31: the sound build chain starts now, ahead of the MVP

Source: Nick, in a Claude Code session, after asking to do 2573 and being
asked whether to unpark it alone, unpark the chain, or keep it parked until
the MVP as decided on 2026-10-19. His answer:

> "Unpark the whole chain"

**Rules:** 2573, 2210, 3030, 3031, 3032, 2397 and 3079 are no longer parked.
This commits to leaving quad-snd now rather than after the MVP. Everything
else about the plan of 2026-10-31 above holds: they run one ticket at a time,
in the order of their blockers, each behind 2573's gates, and only 3079
removes quad-snd. The controllers, the two small ones and macroquad, the
later steps of the platform layer's plan, stay parked.

## 2026-10-31: the sound replacement stays reversible

Source: Nick, in a Claude Code session, while 2573 was being worked:

> "btw, quad-snd replacement and the whole chain should be easily reversible if we encounter an unclimbable hurdle. Ensure this is the case"

> "btw by reversible I mean... say we hit a roadbump and it adds 50 more tickets at that point I might decide (not as a rule, so don't independently decide for me) that it's not worth it and to scrap all progress. It should be a clean revert no matter what stage im at until quad-snd is removed outright. Then I understand it will be more difficult"

> "like I would wanna one shot say \"ok lets give up on sound replacement\" and then its done in one session"

**Rules:**

- Whether to give up is Nick's alone. An agent that hits a roadbump reports what
  it found and what it costs, and goes on with what it can. It never gives up,
  stops the chain or drops a ticket on its own judgement.
- At any stage until 3079 merges, Nick saying so scraps all of it in one session
  (the `give-up-sound-replacement` skill): one PR, with the game on quad-snd.
- After 3079 it is harder, as he says: a revert, which later work can conflict with.
- Making the new backend the default (3084) and deleting quad-snd (3079) are
  separate steps. 3079 waits until the game has run on that default through a
  moved pin, and its removal must be proven undoable before it merges.
- How this is held is technical:
  [ADR-0068](adr/0068-the-sound-replacement-stays-reversible-at-every-stage.md).
