# ADR-0022: The tool contract: answers with limits, schemas by version, and a session over the Harness

- **Status:** Accepted
- **Date:** 2026-10-20
- **Related tickets:** 1129 (the contract and `shuyi new`), 1133 (the MCP
  server), 1134 (the skills), 2501 (templates made of parts), 2207
  (folders on disk in the bundle), 1140 and 1119 (lockstep and tick
  records, which compare state hashes)
- **Extends:** ADR-0005 §1–§2 (one contract, three surfaces), ADR-0019
  (the format registry, which `import` runs over), ADR-0003 §2 (a match
  is its seed and its commands)

## Context

ADR-0005 decided the contract's operations and that each answer says
what it could not determine. Ticket 1129 builds them as `shuyi-cli`
library functions with serde types, `shuyi <op> [--json]`, JSON schemas
that a test keeps, and `shuyi new` with two templates. Building them
raised questions ADR-0005 leaves open:

- **Where the game is.** The `shuyi` binary knows no game. `frame-png`
  and `lang-status` already run in the game's own tool program, which
  calls `main_with` with the game's `GameTools` (`shuyi.ron`'s `game`).
  `check`, `run`, `scene`, `replay`, `bots` and `import` need the game
  too.
- **What a run reports.** The Harness plays scripts frame by frame but
  kept no record of when screens opened or the game quit, and its music
  commands had no frame.
- **What "the same state" is.** A replay needs a hash per step that is
  the same on every run and platform. The core had none.
- **What a schema change is.** "A test fails if they change without a
  version bump" needs a rule a test can check.
- **What `import` imports.** The importers ADR-0005 names (Tiled, LDtk,
  Aseprite) are tickets 1123 and 1124, parked. The registry of 2201 is
  what reads files today.
- **Where the templates are, and what a new game depends on.** The
  engine is not on crates.io (ticket 2005); games depend on a git
  revision.

## Decision

### 1. Every answer is an `Answer<T>`

In `shuyi_cli::contract` (`crates/cli/src/contract.rs`):

- `Answer<T> { contract, op, ok, result: Option<T>, error: Option<String>,
  limits: Vec<String> }`. `contract` is `contract::VERSION`. `ok` is the
  verdict: a check found nothing, two runs agree, a game was made, a
  toolchain is complete. `error` says why an operation could not run at
  all, and then `result` is absent.
- **`limits`** is on the answer, not inside each result, so every
  operation has it the same way: what it could not see ("the game asked
  to quit on frame 7: the 6 frame(s) after it changed nothing", "no
  scene: no stage is open").
- Each operation is a function from its input type to its answer, in a
  module of `contract`: `new::new`, `check::check`, `run::run`,
  `run::scene`, `replay::replay`, `bots::bots`, `import::import`,
  `doctor::doctor`. Every input and output derives `Serialize`,
  `Deserialize` and `JsonSchema`, with `deny_unknown_fields`.
- **The CLI** (`contract::cli`): `shuyi <op> [args]` prints the answer
  for people, `--json` prints it as one JSON document. The exit code is
  0 when `ok`, 1 when not, 2 for wrong arguments (with `--json`, a
  failed answer naming the error). `new`, `doctor` and `schema` need no
  project; the others run on the project's game.

### 2. The game gives each operation through `GameTools`, with defaults

`GameTools` gains `check`, `session`, `replay`, `bots` and `importers`.
Each has a default, so a game's tools that predate them still build: a
missing one answers "the game's tools don't offer run and scene
(GameTools::session)", and `importers` defaults to the engine's
standard kinds. The `shuyi` binary runs these operations in the game's
tool program, as it does `frame-png`; `import` without one reads only
the standard kinds and says so in its limits.

The game's own start options (`frame_options`) also start its game for
`run`, `scene` and `bots`.

### 3. `run` and `scene` drive a `Session`

`contract::session::Session` is an object-safe game running headless:
play a script, idle frames, and read its screens, frame, events, sounds
and scene. `Headless<G>` is the session over any `Harness<G>`; a game
adds its scene (`with_scene`) and its rules' events (`with_events`).
The MCP server (1133) keeps one between calls.

The Harness records, frame by frame, what a run reports:
`frame_count`, `screen_log` (the stack before the first frame and after
each frame that changed it), `quit_frame`, `audio_log` and `music_log`
(each request with its frame). Events are derived from them: a screen
opened or closed (from the stack's changes), music, voices, the quit,
then the game's own.

A script is `frame_png::Script` (keys, pad buttons, waits), now with
serde and a `check` that refuses what would panic the Harness: an
unknown key or button, or a wait longer than the Harness's 2,000
frames.

### 4. A state hash in the core

`shuyi_core::state_hash(&impl Serialize) -> Result<StateHash, HashError>`
(`crates/core/src/hash.rs`) feeds a value to 64-bit FNV-1a in a fixed
form of the engine's own: a tag byte per value, integers little-endian at
one width, strings and bytes with their length, sequences, maps and
structs closed by an end byte, variants by index; names are left out. It
needs nothing but serde, so it builds without `std`, and its tests pin
the form byte for byte. A state's maps must be ordered for its hash to
be the same each run.

`replay::turns` re-runs a turn game's commands from its first state and
hashes it before and after each. What a record is stays the game's (for
`hello-grid` and the grid template, a `StageRecord`: its seed and its
commands); `bots` writes one of each play.

### 5. Schemas are generated, checked in, and versioned by fingerprint

- `contract::schemas()` generates every operation's input and answer
  schema with `schemars` (MIT). They are checked in under
  `crates/cli/schemas/` (`run.input.json`, `run.output.json`, …), and
  `shuyi schema --out <dir>` writes them.
- `crates/cli/schemas/versions.json` lists each contract version with
  the **fingerprint** of its schemas (a `state_hash` of their names and
  texts).
- The schema test fails when the checked-in files are not what the types
  generate, and when the generated fingerprint is not the one listed for
  `contract::VERSION`. So a change to any input or output means: raise
  `VERSION`, regenerate, and add the version's line. Its message says
  each step.

### 6. `import` runs over the format registry

`contract::import::Importers` is a registry of formats (ADR-0019) and
the kinds to try, each with a one-line summary and, if it has one, a
writer of our text format. `Importers::standard()` holds the engine's
standard kinds; a game adds its own (`hello-grid` its `.map` and keymap).
A file is decoded as every kind (or `--kind`'s), the files it names are
read from its folder (leaked once per run until ticket 2207 gives the
bundle folders on disk), and the answer lists the kinds it reads as, or
the errors of the kinds that claim it. No standard kind has a text
writer yet, so `--print` shows the asset as read and says so in its
limits. The importers of Tiled, LDtk and Aseprite will be formats of the
registry (tickets 1123 and 1124); `import` needs no change for them.

### 7. `new` writes one of two plain-folder templates

- `templates/grid` (a rules crate over `TurnRules` with a seeded wanderer,
  a game crate with one stage screen, a bot, records, every contract
  operation wired) and `templates/blank` (one screen). Each has its
  crates, `shuyi.ron`, `AGENTS.md` (and `CLAUDE.md` pointing to it),
  `.cargo/config.toml` (the `cargo shuyi` alias, the MSYS2 linker), CI
  with the gates, tests through the Harness and the contract.
- They are embedded in the binary with `include_dir`; `crates/cli/build.rs`
  rebuilds when they change. Text files hold three placeholders and no
  other `{{`: `{{name}}`, `{{crate}}`, and `"{{engine.<crate>}}"`, which
  is a valid TOML string in the template and becomes the dependency.
- The game's Rust names its crates by their `[lib] name` (`game`,
  `rules`), so its code holds a placeholder only in a string, and the
  template stays formatted after filling.
- The engine dependency is its repository at `main` (the default, with a
  limit saying it isn't pinned), at `--engine-rev <commit>`, or the
  checkout at `--engine-path <dir>`. CI's `templates` job makes a game
  from each template with `--engine-path` and runs the gates its own CI
  runs.
- Ticket 2501 turns the two folders into one template made of parts.

### 8. `doctor` looks through a `Probe`

The checks (rustc and its host, cargo, rustup's targets, MSYS2's GCC on a
`windows-gnu` host, the wasm target) run through `doctor::Probe`, so each
case is tested on a fake machine and the real one is `ThisMachine`.

## Consequences

- An agent can start a game, check it, play it, read its scene, replay
  it and measure it without a window, from one command line or one JSON
  document, and is told what each answer could not see.
- The game's tool program grows with every operation a game wires. A
  game that wires none still builds, and each operation says what is
  missing.
- Changing a contract type is a deliberate act: the schema test makes
  the version bump and its line in `versions.json` part of the change.
  MCP clients (1133) and skills (1134) can rely on `contract` numbers.
- `state_hash` is now a core API that later tickets (lockstep, tick
  records) can use for desync detection. Its form is pinned by tests; a
  change to it changes every hash.
- The grid template is a second copy of `hello-grid`'s patterns. It is
  tested in CI on every pull request, which costs a build of a game a
  template.
- Templates live at the repository's root, outside `crates/cli`, which
  `cargo package` won't include: the first release (2005) must move them
  into the crate or package them another way.
- `import` leaks the bytes of the file's folder once per call, which is
  fine for a CLI run and small for an MCP session, until 2207.

## Alternatives considered

- **Limits inside each result type** — the same field in eight structs,
  and nowhere for an operation that could not run to put its limits.
- **A separate trait per operation** (`GameCheck`, `GameReplay`, …) — more
  types to wire for the same thing; defaults on one trait keep the game's
  `main` unchanged.
- **Hashing a state's RON or JSON text** — its form belongs to another
  crate and may change with its version; a serializer of our own fixes
  the form in one file.
- **A schema test without a version list** (fail on any change) — it
  can't tell a bumped change from an unbumped one; the fingerprint per
  version can.
- **Templates in `crates/cli/templates/`** — packaged with the crate,
  but the ticket and ADR-0013 name `templates/` at the root, where people
  look for them; 2005 decides the packaging.
- **`cargo-generate`-style templates with conditions and hooks** — ADR-0013
  §7 forbids running a template's code, and 2501 brings parts instead of
  conditions.
