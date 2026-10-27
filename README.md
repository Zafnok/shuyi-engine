# Shuyi Engine

An open-source 2D game engine in Rust, built so that you **and your coding
agent** can make, run, see, test and replay a game without a window and
without guessing.

> **Status: early, not yet released.** The engine's code was carved out of
> the game it was built for, [Visions of Shuyi](https://github.com/Zafnok/visions-of-shuyi),
> a Fire Emblem-like tactics game drawn with colored glyphs and sprites,
> and moved here with its history on 2026-10-19 (the game's ticket 1131).
> The game runs on it, pinned to a revision of this repository, and so
> does one example. Its crates on crates.io are empty 0.0.0 placeholders
> that hold the names until the first release, and its API still moves.
> See [`docs/ROADMAP.md`](docs/ROADMAP.md).

[Open in GitHub Codespaces](https://codespaces.new/Zafnok/shuyi-engine?quickstart=1):
a container with the C linker, the system libraries and the pinned Rust
set up (`.devcontainer/`), and the MCP server registered with your
coding agents (its first creation also takes the server's first build).
Codespaces bills whoever opens it, from their
own GitHub account's free hours or plan.

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
movement, isometric, side-scrolling, and moving between them come after
Visions of Shuyi launches, unless the game needs them sooner. The engine
is built for one game first, and happens to handle many
([ADR-0003](docs/adr/0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md)).

**3D, later.** First person, third person, top-down 3D, and HD-2D (2D
pixel-art characters in a lit 3D diorama) are planned as modules, still
deterministic and still testable without a window. HD-2D is a skin, so
a 2D game's rules run under it unchanged. All of it is parked until the
engine reaches parity with the game
([ADR-0011](docs/adr/0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md)).

**More kinds of game, later.** Real-time strategy with a thousand units
or more, with the pathfinding, steering and computer opponents to run
them
([ADR-0015](docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)),
visual novels, and detective games with a check that every case can be
solved
([ADR-0016](docs/adr/0016-visual-novels-and-detective-games-a-story-stage-a-case-file-and-a-fair-play-check.md)),
roguelikes, hex grids, and a gallery of small games in common genres.
Each is a module or an example, parked until after Visions of Shuyi
launches.

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
([ADR-0013](docs/adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)).

**Networking.** It is planned in layers, from LAN play to sessions,
hosted servers, matchmaking and chat
([ADR-0006](docs/adr/0006-networking-in-layers-transports-sessions-sync-models-services.md)).
It is parked until the engine reaches parity with the game.

**Targets.** Windows, Linux, macOS and the web come first, then the
Steam Deck and the Playdate. The GBA, the DS, the Switch, mobile and
other consoles are parked. Each target says what its hardware can do,
and a game that needs more than that doesn't export there
([ADR-0008](docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md),
[ADR-0012](docs/adr/0012-gba-and-ds-targets-capabilities-indexed-colour-two-screens-and-homebrew.md)).

## Map of the repository

| Path | What |
| ---- | ---- |
| `AGENTS.md` | How to work here (people and agents). `CLAUDE.md` points to it |
| `CONTRIBUTING.md` | Issues, discussions and pull requests: what is welcome while the engine is early |
| `docs/ROADMAP.md` | Phases: what comes first, what is parked |
| `docs/decisions.md` | The owner's product decisions: name, license, scope, what is parked |
| `docs/adr/` | Technical decisions |
| `tickets/` | The backlog, one ticket per pull request |
| `.claude/skills/` | Skills for developing the engine: working a ticket, writing tickets and ADRs, the gates, input rules, the Sonar queue, dependency upkeep, the pinned Rust |
| `crates/` | The engine's crates ([below](#the-crates)) |
| `examples/` | Games made from the engine's crates alone ([below](#the-first-example-hello-grid)) |
| `.github/dependabot.yml`, `.github/workflows/security.yml`, `codeql.yml`, `scorecard.yml` | Dependency upkeep: Dependabot for the crates and the actions' pins (the templates' too), the weekly advisory check that files `advisory` issues, CodeQL and Scorecard; zizmor audits the workflows in `ci.yml` (ADR-0037) |
| `sonar-project.properties`, `tools/sonar-triage/` | SonarCloud's scan of the code in CI, and the weekly job that files its findings as `sonar` GitHub issues (ADR-0017) |
| `docs/carve.md` | What came from the game, layer by layer, and the game's tickets that moved it |
| `docs/formats.md` | Asset kinds and file formats: the built-in formats, and how to write a format plugin |
| `docs/contract.md` | The tool contract: `shuyi new`, `check`, `run`, `scene`, `replay`, `bots`, `import`, `doctor`, their JSON answers and schemas |
| `docs/platform-backends.md` | How to write a platform backend: the `Platform` trait, what each method must do, the headless backend as a reference, and why console backends stay private |
| `templates/` | The two games `shuyi new` makes: `grid` (a turn-based grid stage) and `blank` (one screen, on the short path: `BasicGame`, `run_game` and `BasicTools`) |
| `scripts/setup-linux.sh`, `.devcontainer/` | Setting up Linux in one command (apt, dnf or pacman): the C linker, `pkg-config`, the libraries the backend links and opens, and the pinned Rust, as `shuyi doctor` checks them; the dev container (and Codespaces) runs it, and so does CI |
| `skills/` | The skill pack: Agent Skills for making games on the engine, each with a worked example CI runs on a new game. `shuyi new` copies them into every game, and `.claude-plugin/` makes them a Claude Code plugin ([`skills/README.md`](skills/README.md)) |
| `site/` | The website: a Zola project that `pages.yml` publishes on GitHub Pages, with the examples playing in the browser ([`site/README.md`](site/README.md)) |
| `THIRD_PARTY_ASSETS.md` | Every file here that isn't the engine's own, with its license |

## The crates

Each sits under the game crate of the same layer (game ADR-0063 §2). The
core is pure and deterministic, only platform backends touch devices,
time is an argument, and there's no `unsafe` outside the app and the heap
counter. An engine crate never depends on a game.

| Crate | Folder | Owns |
| ----- | ------ | ---- |
| `shuyi-core` | `crates/core` | The seeded RNG, the simulation traits, and the tactics kit: grid, paths, turns. No I/O. |
| `shuyi-assets` | `crates/assets` | Bundle layers, images, fonts, palette, audio cues, languages, keymaps, tilesets, importers, error reports. |
| `shuyi-ui` | `crates/ui` | The frame, screens, input actions, widgets, skins, cinematics and the grid map. |
| `shuyi-platform` | `crates/platform` | The platform boundary: the `Platform` trait every backend implements, `App`, which runs one frame of a game for the backend's loop (the loading card, fixed steps, suspend and resume), the headless backend and the test Harness; a game on the short path (`BasicGame`) and the `prelude` it names. No devices ([`docs/platform-backends.md`](docs/platform-backends.md)). |
| `shuyi-app` | `crates/app` | The macroquad backend, for desktop and the web: the window, blitting, input devices, the audio player and storage, and the loop that drives `App`; `run_game` runs a game on the short path from a plain `main`. |
| `shuyi-alloc-count` | `crates/alloc-count` | The heap counter: `CountingAlloc`, a global allocator around another that counts the bytes live, their peak and the allocations, and with its `global` feature the one counter every debug build of a game on `shuyi-app`, and of its tool program, runs on, counting by memory tag too (the frame's phase, the game's span, the asset kind being decoded). Each frame's perf record carries its counts; `--cfg shuyi_no_heap_count` opts out. Builds without `std`. |
| `shuyi-cli` | `crates/cli` | Tools, binary `shuyi` (`cargo shuyi <command>`): the tool contract (`new`, `check`, `lint`, `modules`, `add`, `run`, `scene`, `replay`, `bots`, `playtest`, `autobalance`, `import`, `bake`, `doctor`, `explain`, `attach`, each with `--json`), the same contract as an MCP server with a session (`mcp`, `mcp install`), and `frame-png`, `font-atlas`, `glyph-tiles`, `check-keys`, `check-text`, `check-skills`, `lang-status`, `web` and `private-assets`, on the project a `shuyi.ron` describes. |
| `shuyi` | `crates/shuyi` | The front door a game depends on: the crates above under short names (`shuyi::core` to `shuyi::app`), the `prelude` with `run_game`, and one cargo feature per module, all off by default, with `enabled_modules`, the modules turned on, for the game to install. |

`shuyi-core`, `shuyi-assets` and `shuyi-ui` also build without `std` (for
the Playdate): turn off their default `std` feature.

A game depends on `shuyi`, by git revision until the first release:

```toml
[dependencies]
shuyi = { git = "https://github.com/Zafnok/shuyi-engine", rev = "<commit>" }
```

Each merged PR says what it did to the public API in one label, `api:
none`, `api: added` or `api: breaking`, which anyone can read without a
token (`gh pr view <N> --json labels -R Zafnok/shuyi-engine`). A game
pinned by revision moves its pin when a PR past it isn't `api: none`,
and reads the ticket's `Breaking:` lines when it is `api: breaking`
([ADR-0038](docs/adr/0038-the-public-api-gate-a-listing-diff-and-semver-checks-on-every-pr-and-a-breaking-line-for-each-break.md)).

A project need not poll for that: a push to `main` starts the workflows
listed in the repository variable `DEPENDENT_WORKFLOWS`, one `owner/repo
workflow-file` per line, with the new commit as the input `engine_sha`
([ADR-0045](docs/adr/0045-a-push-to-main-starts-dependents-watchers-workflow-dispatch-with-an-actions-only-token-kept-in-an-environment-for-main.md)).
To be on the list, give your workflow a `workflow_dispatch` trigger with
an `engine_sha` input, and open an issue here naming the repository and
the file. The dispatch token needs Actions: write on your repository, so
until it has that, poll on a schedule (keep a slow one anyway, as a
backstop).

Everything else is a module, turned on as a feature of `shuyi`: the
catalog, [`docs/modules.md`](docs/modules.md), lists them by concern
(perspective, bodies and motion, online play, kind of game), each with
its status. `cargo shuyi modules` lists the same catalog for a game, and
`cargo shuyi add <module>` turns one on.

## Starting a game, and the tools on it

`shuyi new` makes a game that builds and passes its own gates as it is,
from the engine's template, made of parts that its answers pick
([`docs/templates.md`](docs/templates.md)): `--template grid` (a title
and a turn-based grid stage, with its rules in a crate of their own, a
bot and records) or `--template blank` (one screen). On Linux, run
`bash scripts/setup-linux.sh` first: it installs the C linker,
`pkg-config`, the system libraries and the pinned Rust, and the new game
has its own copy. From this repository:

```
cargo shuyi new ../moon-garden --template grid --engine-rev <commit>
```

In the new game's folder, `cargo shuyi` runs the engine's tools on it.
Each answers for people, or with `--json` for a program, and says what
it couldn't determine ([`docs/contract.md`](docs/contract.md)):

```
cargo shuyi check                                # every error in the game's files
cargo shuyi run --script "Right Down" --frames 30   # screens, scene, events, sounds
cargo shuyi bots 20                              # how 20 bot games ended
cargo shuyi doctor                               # what the toolchain lacks
```

## The first example: Hello, Grid

`examples/hello-grid/` is a small game made from the engine's crates
alone. Its art is the Terminus font (OFL-1.1) and a tileset `glyph-tiles`
draws from it; its sounds are made by code. Its
[README](examples/hello-grid/README.md) says how it is laid out.

Run it, from the repository's root:

```
cargo run -p hello-grid
```

Build it for the web, from its folder (the page goes to `dist/web/`;
serve that folder over HTTP):

```
cd examples/hello-grid
cargo shuyi web --release
```

CI builds it natively and for the web and runs its tests (`cargo test -p
hello-grid`), and the website plays the web build at `/examples/hello-grid/`.

What it shows, and the engine's part in each:

| In the game | The engine's |
| ----------- | ------------ |
| A title menu, an end screen | `shuyi_ui::widgets::Menu`; the screen stack (`Screen`, `Transition`, `Game`) |
| "Leave the stage?", asked on a screen of its own | a screen that pops with its answer (`Transition::pop_with`) and the stage that reads it (`Screen::uncovered`) |
| Options with two settings (text speed, the map's look), saved | `shuyi_ui::screens::options` with the game's rows; `settings` (`GameSettings`); `storage` |
| Key rebinding, from the options | `shuyi_ui::screens::key_bindings`; the keymap file (`shuyi_assets::keymap`) over the game's `ActionSet` |
| A short scene before the stage | the `.dlg` format (`shuyi_assets::dialogue`), `shuyi_ui::dialogue`'s player and text box |
| A 12 × 10 stage: walk your piece to the goal while a foe steps toward it | the `.map` format (`shuyi_assets::grid_map`); `shuyi_core`'s `TurnRules`, `Sides`, `reachable`, `flow_field`, `SimRng`; `shuyi_ui::grid`'s scene, cursor, path steering and walks |
| Two looks of the map | the glyph skin and the sprite skin (`GridSkin`), with the game's `UnitPainter`s |
| The foe's mind, and the bot test | `shuyi_core::bots`: `Bot`, `RandomBot`, `play` |
| Its tests | the `Harness` (`shuyi_ui::harness`); `frame-png`'s `Painter`, as binary snapshots |
| Its tools | `shuyi.ron` with `font-atlas`, `glyph-tiles`, `frame-png`, `lang-status` and `web` |

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
