# Shuyi Engine

An open-source 2D game engine in Rust, built so that you **and your coding
agent** can make, run, see, test and replay a game without a window and
without guessing.

> **Status: planning.** The engine's code is being carved out of the game
> it was built for, [Visions of Shuyi](https://github.com/Zafnok/visions-of-shuyi),
> a Fire Emblem-like tactics game drawn with colored glyphs and sprites.
> Until that move lands, this repository holds the engine's design
> records, its tickets and the skills its developers use. See
> [`docs/ROADMAP.md`](docs/ROADMAP.md).

## What it is for

1. **Make Visions of Shuyi easy to build.** The game is the engine's first
   user and stays a separate, paid product.
2. **Make other people's 2D games easy to build.** This isn't only for
   tactics games.

## What it will be like

These come from the code that already runs the game.

- **Deterministic.** A game state changes only through commands (turn
  games) or inputs per fixed step (real-time games). The same seed and
  the same inputs give the same game, on every platform. Replays, bug
  reports, bots and online play all rest on that.
- **Everything is text.** Data, maps, dialogue, cinematics and test
  snapshots are plain text that people, agents and git can read and diff.
  Standard formats (PNG, Tiled, LDtk, Aseprite) load directly. Formats
  are plugins, so a faster or smaller format can be added later without
  touching the code that uses the assets ([ADR-0007](docs/adr/0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md)).
- **Headless first.** A test harness plays the game with key scripts and
  no window, reads what is on screen as data, and renders any frame to a
  PNG.
- **Graphics are a skin.** Screens say *what* to show and a skin says *how*
  it looks, so glyph art, pixel art and bought sprite packs swap without
  touching the rules.
- **Players rebind everything.** Game code reacts to actions, never to
  keys or buttons.
- **AI-native.** One set of operations is offered three ways: a CLI, an
  MCP server, and Agent Skills
  ([ADR-0005](docs/adr/0005-ai-native-tooling-one-contract-three-surfaces-and-visual-scripting-as-a-view-over-text.md)).

**Game styles.** The game's top-down grid comes first. Top-down free
movement, isometric, side-scrolling, and moving between them come later
([ADR-0003](docs/adr/0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md)).
No 3D.

**Modules.** The default engine stays small. You pick a direction for
each part of your game (its perspective, how bodies move and animate,
online play) and add only the modules that fit it, such as rigid-body
physics or procedural animation. Each module says which kinds of game
it is built for, and guides show how to move a game between
perspectives
([ADR-0009](docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)).

**Starting a game.** `shuyi new` asks what you want: one perspective or
several with moves between them, how bodies move, the music, the export
targets, online play. Every answer is also a flag, so agents and CI make
games the same way. Templates are data, and anyone can write one
([ADR-0011](docs/adr/0011-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)).

**Networking.** It is planned in layers, from LAN play to sessions,
hosted servers, matchmaking and chat
([ADR-0006](docs/adr/0006-networking-in-layers-transports-sessions-sync-models-services.md)).
It is parked until the engine reaches parity with the game.

## Map of the repository

| Path | What |
| ---- | ---- |
| `AGENTS.md` | How to work here (people and agents). `CLAUDE.md` points to it |
| `docs/ROADMAP.md` | Phases: what comes first, what is parked |
| `docs/decisions.md` | The owner's product decisions: name, license, scope, what is parked |
| `docs/adr/` | Technical decisions |
| `tickets/` | The backlog, one ticket per pull request |
| `.claude/skills/` | Skills for developing the engine: working a ticket, writing tickets and ADRs, the gates, input rules |

The crates, examples and the engine's own shippable skills arrive with
the code (`crates/`, `examples/`, `skills/`).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE)), or
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms
or conditions. See [`CONTRIBUTING.md`](CONTRIBUTING.md) for what is
accepted today.

### The name

"Shuyi" is a person's name, and the engine's name is not covered by the
licenses above. You're welcome to say your game is "made with Shuyi
Engine". Please give a fork its own name, and don't present a fork or a
product as official.

The game **Visions of Shuyi** isn't part of this repository and isn't
under these licenses. That covers its rules, story, characters, art and
music.
