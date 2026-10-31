# AGENTS.md

How to work on Shuyi Engine. This applies to people and coding agents
alike; Claude Code reads it through `CLAUDE.md`.

Shuyi Engine is an open-source 2D game engine in Rust (MIT OR Apache-2.0).
It is carved out of the game **Visions of Shuyi**
(`github.com/Zafnok/visions-of-shuyi`), a paid, proprietary game. That
game stays the engine's first user. Read [`README.md`](README.md), then
[`docs/ROADMAP.md`](docs/ROADMAP.md).

Where each kind of fact is written, and what each task reads first, is in
[`docs/README.md`](docs/README.md), the docs map (the `document` skill).

## Shared instructions and skills

`AGENTS.md` is the shared instruction source. `CLAUDE.md` imports it for
Claude Code; Codex reads it directly. Use the same rules with any model.
Each ticket names its Claude/OpenAI model pair on the `model` line
(`tickets/README.md`). Choose the member for the current harness; effort
is separate. Opus pairs with Sol; Fable pairs with Astra.

The portable workflows and their resources live in `.claude/skills/` for
compatibility with existing links and the pinned engine skill-pack updater.
Codex discovers `.agents/skills/` loaders that read those same workflows.
Edit the shared source, then run `pwsh -File scripts/sync-agent-skills.ps1`;
`-Check` verifies that every loader is current. Run it after updating the
engine skill pack too. These are regular tracked files: no symlinks or
Windows administrator rights are needed. A new chat may be needed to
discover newly added skills.

Use the current harness's tools for questions, plans, shell commands and
optional delegation; a named tool in a workflow describes an operation,
not a requirement to change harnesses. If a tool is unavailable, use an
equivalent operation while preserving the checks and authorization rules.
Harness-specific hooks do not run everywhere: execute the workflow's
required checks explicitly. Do not alter another harness's settings to
make a shared instruction work.

## Who decides what

- **Nick (owner)** decides the product: the name, the license, when
  things go public, who may contribute, which parked work starts, and
  anything that changes what the game gets. He does **not** review code
  and doesn't want technical questions. Ask him product questions with
  the `ask-nick` skill. His answers are in
  [`docs/decisions.md`](docs/decisions.md).
- **The coding agent** decides everything technical and records significant
  choices as ADRs in `docs/adr/` (the `write-adr` skill).

## Hard rules

1. **Work one ticket at a time** from `tickets/open/`, with the
   `work-ticket` skill. One ticket = one branch = one PR. The PR moves the
   ticket to `tickets/done/`. Two exceptions, each one PR:
   - a GitHub issue labelled `sonar`, filed weekly from SonarCloud and
     fixed with the `sonar-fix` skill
     ([ADR-0017](docs/adr/0017-sonar-backlog-becomes-batched-github-issues.md));
   - a Dependabot PR, or a GitHub issue labelled `advisory` from the
     weekly advisory check, worked with the `dependency-upkeep` skill
     ([ADR-0037](docs/adr/0037-dependency-upkeep-dependabot-a-weekly-advisory-check-workflow-audits-and-bot-work-as-its-own-ticket.md)).
2. **No unticketed work.** Found something else to do? Write a ticket
   (`write-ticket` skill).
3. **Parked tickets stay parked.** A ticket whose title starts with
   `Parked:` is not started until Nick unparks it ([`docs/ROADMAP.md`](docs/ROADMAP.md)).
4. **Nothing of the game in the engine.** No game crate, no file from the
   game's `assets/`, no class, character, story, map or bought art, and no
   game-specific rule. The game uses the engine like any other game would.
   When unsure whether something is engine or game, it is game.
5. **Respect the layers** ([ADR-0002](docs/adr/0002-inherited-rules-layers-determinism-skins-actions-tests-licenses.md)):
   - the core is pure and deterministic (no I/O, no clock, no floats in a
     simulation);
   - state changes only through commands (turn games) or inputs per step
     (tick games);
   - a screen asks where it runs through `ctx.platform`, never a
     `cfg!`, and the rules never read it ([ADR-0060](docs/adr/0060-platform-facts-at-run-time-read-from-the-targets-profile-handed-through-setup-and-read-by-screens-never-by-rules.md));
   - only platform backends touch windows, files, audio devices and
     the clock: `shuyi-app` (macroquad), and later `shuyi-playdate`,
     `shuyi-gba`, `shuyi-nds` and private console backends ([ADR-0008](docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)).
   - [`tools/layers.ron`](tools/layers.ron) holds the layers as data, and
     `cargo xtask check-layers` checks them ([ADR-0032](docs/adr/0032-the-layers-as-data-checked-over-cargo-metadata-a-float-ban-in-the-simulations-and-a-new-games-rules-crate-kept-pure.md)).
     A new engine crate, engine edge or outside crate of a pure crate adds
     its line there in the same PR.
6. **Tests are the review.** Run the `run-gates` skill before pushing.
   Never weaken a gate to pass it. CI also measures coverage, scans
   with SonarCloud, audits the workflows with zizmor and the code with
   CodeQL; a zizmor finding wrong here goes in `.github/zizmor.yml` with
   why. A Sonar finding is argued with in
   `tools/sonar-triage/policy.jsonc`, never with `// NOSONAR` or
   `#[allow]` (the `sonar-triage` skill).
7. **Licenses.** The engine is MIT OR Apache-2.0. Ship only
   dependencies and assets under the permissive licenses in ADR-0002 §8:
   no GPL, LGPL, MPL or copyleft, nothing non-commercial, nothing that
   costs money. Watch default features that pull in denied code
   (ADR-0004 §7). Every non-crate asset goes in `THIRD_PARTY_ASSETS.md`
   once one exists.
8. **Never hard-code a key.** Engine code and games react to actions, and
   text names keys through the player's keymap (`input-actions` skill).
9. **Graphics are a skin.** Logic builds plain-data views and scenes, and a
   skin paints them. Tests of what happened read the scene or the state;
   only tests of a look read cells, colors and sprite items.
10. **Formats are plugins** ([ADR-0007](docs/adr/0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md)).
    An asset kind is never tied to one file format. Standard formats always
    load. Our text formats stay the source of truth, and an optimized format
    is derived from them and tested equal.
11. **Never write a bare `@word` on GitHub** (PR title or body, commit
    message, comment, release notes). GitHub turns it into a mention of
    that user. Put it in backticks or drop the `@`.

## Working with the game repository

- The engine's code was carved in the game repository's `engine/` folder
  (game tickets 1102–1118 and 1154) and moved here with its history (game
  ticket 1131). [`docs/carve.md`](docs/carve.md) says what came from where.
- The game depends on this repository by a pinned git revision (its
  `shuyi-engine.rev`). A game change that needs an engine change is an
  engine PR here first, then a game PR that moves the pin. The game's
  `cargo xtask engine` checks this repository out inside the game at the
  pin, so one working copy edits both while the change is made.
- A change here must not break the game at its next pin bump: when an
  API changes, write a `Breaking:` line in the ticket's completion notes
  (CI's `api` job finds the change, [ADR-0038](docs/adr/0038-the-public-api-gate-a-listing-diff-and-semver-checks-on-every-pr-and-a-breaking-line-for-each-break.md)),
  and the game's PR that moves the pin follows it.
- Ticket ids:
  - `1119`–`1150` moved here from the game repository and kept their
    numbers.
  - New engine tickets use `2001` and up (blocks in
    [`tickets/README.md`](tickets/README.md)).
  - Any other four-digit id in the text is a game ticket.
  - In `blocked_by`, a game ticket is written `"game#NNNN"`. The game
    writes this repository's tickets `"engine#NNNN"`.
- A request from the game becomes a ticket here as the `write-ticket`
  skill's "A request from the game" says. The game's half of the process
  is its `engine-change` skill
  (`github.com/Zafnok/visions-of-shuyi/blob/main/.claude/skills/engine-change/SKILL.md`).
- ADRs: "ADR-NNNN" means this repository's. "game ADR-NNNN" means the
  game's (`github.com/Zafnok/visions-of-shuyi/tree/main/docs/adr`).
- The study behind all of this: the game repository's
  `docs/engine/feasibility.md` (the report to Nick) and
  `docs/engine/boundary.md` (every module of the game, engine or game).

## Environment

- Nick's machine: Windows 11, Rust with the **GNU** host toolchain; **no
  MSVC**. Never require MSVC locally; CI may use MSVC on GitHub runners.
- Local builds need MSYS2's mingw-w64 toolchain (`winget install -e --id
  MSYS2.MSYS2`, then `pacman -S mingw-w64-x86_64-gcc`), on `PATH` as
  `C:\msys64\mingw64\bin`.
- Use `cargo install --locked <tool>`; `cargo-binstall` doesn't build
  there.
- On Linux, in a dev container or in a cloud agent's container, `bash
  scripts/setup-linux.sh` installs the C linker, `pkg-config`, the
  system libraries and the pinned Rust, and `shuyi doctor` checks the
  same list.
- Never run mutation testing on Nick's machine (CI runs it).
- The pinned Rust (`rust-toolchain.toml`) moves with the `bump-rust` skill.
- The web build uses macroquad's own loader, not `wasm-bindgen`
  (game ADR-0034). A crate that needs `wasm-bindgen` doesn't work in the
  web build without a JS plugin of our own.
- Git remote: `https://github.com/Zafnok/shuyi-engine` (public).
