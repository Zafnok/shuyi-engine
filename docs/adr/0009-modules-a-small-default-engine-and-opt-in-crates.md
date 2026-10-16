# ADR-0009: Modules: a small default engine, a direction per stage, and opt-in crates that say what they are for

- **Status:** Accepted
- **Date:** 2026-10-16
- **Related tickets:** 2003 (the catalog and opting in), 2004 (guides
  per direction and per move), 1120, 1122, 1126, 1127 (perspective
  markers), 1128 (physics), 1140–1150 (networking), 2202–2204 (formats),
  2701–2704 (procedural animation), 1135 (visual scripting)
- **Extends:** ADR-0002 §1 (crates and layers), ADR-0003 §1 and §6
  (stages, examples, projections), ADR-0005 (the contract and the
  skills), ADR-0007 (formats are plugins)

## Context

Nick, 2026-10-16 (`docs/decisions.md`):

- "we should support plenty of modules for our engine, so that we don't
  overload devs";
- "we should support a wide variety of game types in ways that make
  sense";
- "a dev will set a direction and import non-conflicting modules to help
  them out". A normal physics and animation system is one choice, and
  procedural animation, with "different primitives", is another. The
  same goes for side-scrolling, top-down and isometric games;
- games may move between perspectives, but "we should make it very
  obvious OK this behavior is built for this type of game, this one for
  this type of game, here is how to transition between the two", and
  everything should be "smartly designed and well documented at all
  steps";
- "eventually we support a visual programming like Blueprints or
  Scratch".

What exists in the plans:

- **Five crates** (ADR-0002 §1): `shuyi-core`, `shuyi-assets`,
  `shuyi-ui`, `shuyi-app`, `shuyi-cli`.
- **Stages** (ADR-0003 §1): a game style is a stage, and changing style
  is a screen transition owned by the game's flow. Projections are a
  runtime value (ADR-0003 §6).
- **Some optional pieces, each decided on its own:**
  - `shuyi-physics` is a crate no other engine crate depends on (1128,
    ADR-0004 §5);
  - `shuyi-net` is a crate of its own (1140, ADR-0006);
  - formats are plugins, each behind a cargo feature (ADR-0007, 2203);
  - platform backends are crates of their own (ADR-0008).
- **What is missing:**
  - a common rule for what a module is and what it ships with;
  - a way for a game to turn modules on, and for a developer to find
    them;
  - a way to say which pieces are alternatives to each other;
  - anything that stops a piece built for one kind of game from being
    used in another by mistake.

"Overload" has two costs. A game pays for code it doesn't use: WASM
size, compile time, dependencies and their licenses. And a developer
pays for concepts they don't need: types in the docs, skills in their
agent's context, options in the templates.

## Decision

### 1. The default engine stays small; the rest is modules

- **The default engine** is the five crates of ADR-0002 §1, with what
  the accepted ADRs already put in them (the tick simulation, `Fx`, the
  collision kit and animation clips stay in `shuyi-core` and
  `shuyi-assets`). This ADR doesn't move anything.
- **A module** is anything else: a part some kinds of game need and
  others don't. A new part is a module when a game that doesn't use it
  would still pay for it in size, compile time, dependencies or concepts
  to learn. When in doubt, it is a module.

### 2. A direction is one answer per concern, chosen per stage

A **concern** is a question every game answers one way or another. A
**direction** is the set of answers a stage picks. The concerns, and
the answers planned so far:

| Concern | Answers | Where |
| ------- | ------- | ----- |
| Time | turns; fixed ticks | default (ADR-0003 §2) |
| Perspective | top-down grid; top-down free; isometric; side view | default kits (ADR-0003 §6, 1122, 1126, 1127) |
| Bodies and motion | movers and drawn clips; rigid-body physics; procedural animation | default (ADR-0004 §3–§4); `shuyi-physics` (1128); `shuyi-procanim` (2701–2704) |
| Online play | none; lockstep; rollback; a referee server | `shuyi-net` (ADR-0006) |

- **Answers to the same concern are alternatives.** Two alternatives
  never drive the same thing in the same stage: one body isn't both a
  rapier body and a procedural one.
- **Alternatives can still share a game.** They may be used:
  - in different stages, such as a top-down field that starts a
    side-view fight;
  - on different things in one stage, such as procedural creatures
    pushing rigid crates, when the module's README says how.
- **Every module builds beside every other.** CI builds all modules
  together, so "non-conflicting" is never about compiling. It is about
  what drives what, and §3 and §4 make that visible.
- New concerns and answers are added to this table, in the catalog
  (§6), when a module needs them.

### 3. The types say what a piece is built for

- **Perspective markers.** `shuyi_core::perspective` has a trait
  `Perspective` and one marker type per answer: `TopDownGrid`,
  `TopDown`, `Isometric` and `SideView`. They are plain types, so the
  controllers in `shuyi-core` can name them. `shuyi-ui` gives each
  marker its `Projection` (ADR-0003 §6), so the drawing side reads the
  runtime value from the type. 1120 adds the trait and the first two
  markers; 1126 and 1127 add the others.
- **A stage names its perspective** in its type. A world layer, a camera
  and the scene of a perspective stage are generic over `P:
  Perspective`.
- **A kit built for one perspective takes only its marker.** For
  example:
  - the platformer controller (1127) works with `SideView` only;
  - the top-down controller (1122) works with `TopDown` only;
  - the grid kit works in tile coordinates, so it takes any `P`
    (ADR-0003 §6).

  Using a platformer controller in a top-down stage doesn't compile, and
  the error names both markers.
- **Bodies and motion** work the same way where they can. A procedural
  body and a rapier body are different types, and the systems that step
  them take only their own.
- Where a type can't say it, the README and the skill do (§4), and
  `shuyi check` reports a module used outside the directions it serves,
  when it can see it.

### 4. What a module is, and what it ships with

- **A crate** `shuyi-<name>` in `crates/<name>`.
  - It depends on the default crates, downward only (ADR-0002 §1), and
    on other modules only when its README says so. There are no cycles.
  - **No default crate depends on a module.** 2003 checks this from
    `cargo metadata`.
  - Its parts follow the layers (ADR-0002). A simulation part is pure
    and deterministic, in `Fx`. A drawing part builds scene and frame
    items that a skin paints. It never touches the platform.
  - A part that pulls in a heavy layer (for example `shuyi-ui` for
    drawing) sits behind the module's own cargo feature, so a headless
    server can use the simulation part alone.
- **Two kinds of plugin keep their own form**, and the catalog (§6)
  lists them beside the modules:
  - formats are cargo features of `shuyi-assets` (ADR-0007);
  - platform backends are crates of their own (ADR-0008).
- **Heavy or experimental work** may live in a separate repository with
  its own release, when its size or its dependencies would burden this
  one. Images through machine learning (2901) are the first case.
- **Metadata** in its `Cargo.toml`, under `[package.metadata.shuyi]`:
  - `concern` and `answer` (§2);
  - `serves`: the directions it is built for;
  - `alternatives`: the modules that answer the same concern;
  - `works_with`: modules it has been tested beside in one stage.

  The catalog, `shuyi modules` and `shuyi add` read it.
- **A page**, `crates/<name>/README.md`, in a fixed order:
  1. what it is for, in one paragraph, with the directions it serves;
  2. what it is *not* for, and which module or kit to use instead;
  3. what it costs: the WASM size it adds (measured), its dependencies
     and their licenses;
  4. how to start, as the smallest working code;
  5. how it sits beside its alternatives, and how to move between them.
- **An example** in `examples/`: a small game of a direction it serves,
  and its integration test (ADR-0003 §1). It is driven by the Harness,
  has a replay determinism test, and has a `frame-png` picture. The
  README claims a direction only when its example passes CI.
- **A skill** in the engine's skill pack (1134), named `shuyi-<name>`.
  A game gets it only when it turns the module on (§6), so an agent's
  context holds only what the game uses.
- **Settings as data.** A module's settings (a creature, a physics
  material, a controller's numbers) are text files, never only code.
  That keeps them diffable, hot-reloadable, and ready for a visual
  editor (§8).
- The same gates as everything else. All crates share one workspace
  version until 1.0.

### 5. Guides: one per direction, one per move

`docs/guides/` holds two kinds of page. Each is short, and links to the
READMEs rather than repeating them.

- **A direction guide** (`top-down-grid.md`, `side-view.md`, …) says:
  - which template starts it;
  - which default kits and modules it uses;
  - which ones look related but aren't for it, and why;
  - which example to read first.
- **A move guide** (`top-down-to-side-view.md`, …) covers one move a
  game can make between stages. It says:
  - what the flow screen carries across (ADR-0003 §1);
  - what each stage builds from it;
  - which example does it.

  A move is "supported" when its guide and its example exist. The first
  is the field-to-battle example (1125).
- **A ticket that adds a direction, a module or a move updates its
  guides** in the same PR. 2004 writes the first guides and the check
  that keeps them in step with the catalog.

### 6. Turning a module on, and finding it

- **The `shuyi` crate is the front door.** It re-exports the default
  engine and has one cargo feature per module (`physics`, `net`,
  `procanim`, …), all off by default. A game depends on `shuyi` and
  names the features it wants. A module's crate can also be used
  directly.
- **The contract** (ADR-0005) gains two operations:
  - `shuyi modules` lists the catalog by concern, and says which modules
    this game has on;
  - `shuyi add <module>` turns on the feature, copies the module's skill
    into the game, and prints the README's first paragraph. If the game
    already has an alternative on, it says so and links the guide on
    using both.
- **Templates** (`shuyi new <template>`) are directions. Each one turns
  on the modules its direction needs, and no others.
- **The catalog**, `docs/modules.md`, is grouped by concern. It has one
  row per module, format and backend, planned or built:
  - its name;
  - the directions it serves;
  - its status: planned, parked, or ready;
  - its tickets.

  A test checks that every crate under `crates/` outside the default
  five has a row, a README and its metadata, and that every feature of
  `shuyi` names a catalog row.

### 7. A module is written when a kind of game needs it

"In ways that make sense" means:

- each module names the directions it serves, and an example proves
  one;
- a module is ticketed when a kind of game needs it (the game, Nick, or
  a clear audience), not to fill a list;
- if two modules would overlap without being alternatives, the second
  extends the first, or its README says why it doesn't.

### 8. Ready for visual programming

Visual scripting stays ADR-0005 §5 and the parked research 1135: a graph
is a text file and a view over text. This ADR adds only what keeps
modules ready for it:

- settings are data (§4);
- what a module can do is named in types and in the contract, not
  hidden in callbacks;
- its operations have plain-data inputs and outputs.

When 1135 runs, each module's nodes come from those.

## Consequences

- A developer making a platformer starts from the side-view template and
  its guide. They see the default engine and the modules that template
  turned on. Physics, networking and procedural animation stay out of
  their build, their docs and their agent's context until they `shuyi
  add` them.
- Mistakes of direction fail early. A platformer controller in a
  top-down stage is a compile error that names both perspectives.
- Moving between perspectives is a documented path with an example, not
  something each game works out alone.
- Every module costs a README, an example, a skill, metadata, and a
  line in the guides. That is more than the code alone, and it is on
  purpose: an undocumented module is one a developer can't find or use.
- The perspective markers add a type parameter to world layers and
  perspective kits. It is one more concept, and it is the one that
  makes "built for this type of game" visible.
- The catalog doubles as a public map of kinds of game, with "parked"
  rows showing what is planned.
- Cargo features multiply build combinations. CI builds the default
  engine, each module alone, and all modules together; it doesn't try
  every combination.
- Tickets written before this ADR (1120, 1122, 1126, 1127, 1128, 1140)
  gain its duties. Each is updated to say which.

## Alternatives considered

- **One big crate with everything** — the simplest to depend on, but
  every game pays every module's size, compile time and licenses, and
  its docs show everything at once. That is the overload Nick named.
- **Cargo features inside `shuyi-core` for each module** — one crate's
  features grow into a tangle, tests must cover combinations within one
  crate, and the layers blur, because a feature can reach anything in
  the crate.
- **Perspective as a runtime value only** (ADR-0003 §6 alone) — it
  works, but a platformer controller in a top-down stage runs and
  misbehaves instead of failing to build. The marker types cost one type
  parameter and say it at compile time.
- **Forbid alternatives in one game** (one physics answer per game) —
  that breaks the field-to-battle game ADR-0003 is built around, and
  games that mix procedural creatures with rigid crates. The rule is per
  thing and per stage, not per game.
- **Modules loaded at run time (dynamic libraries)** — Rust has no stable
  ABI, and neither the web build nor the Playdate can load them. Mods
  are data layers (ADR-0004 §1), not code.
- **Every module in its own repository** — versions drift, and a change
  to the default engine can't be tested against every module in one CI
  run. It is kept only for heavy or experimental work (§4), such as
  images through machine learning (2901).
- **No umbrella crate; games depend on each `shuyi-*` crate** — still
  allowed, but a new developer would have to learn the crate list before
  writing a line. One crate with features is the common Rust pattern
  (Bevy, Tokio).
