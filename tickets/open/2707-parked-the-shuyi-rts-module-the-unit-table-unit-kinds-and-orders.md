---
id: "2707"
title: "Parked: The shuyi-rts module: the unit table, unit kinds as data, orders and order queues, and the Kind of game concern"
type: feature
milestone: "Engine: real-time strategy"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["2705", "2706", "2003", "2201", "2116"]
nick_input: none
completed:
---

# 2707 — The shuyi-rts module: units, kinds and orders

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-18 (`docs/decisions.md`): "I wanna be able to support high
unit count RTS games".
[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
sets the design. This ticket is:

- §1: the crate `shuyi-rts`, its module duties, and its answer to the
  *Kind of game* concern;
- §2: the commands a player can give;
- §3: units in columns, kinds as data, and staggered work in buckets.

It builds on:

- **2705**, the measurement. If 2705 wrote an ADR that supersedes
  ADR-0015 §3, this ticket follows that ADR instead, and 2705 updated
  this text.
- **2706**, `CommandSim`, `Timed` and `CommandRecord`. `RtsSim`
  implements `CommandSim`.
- **2003**, the catalog, the `shuyi` crate, `docs/modules.ron` and
  `check-modules`. 2003 adds the `kind` concern with `alternatives:
  false` and `rts` as a parked answer.
- **2201**, the format registry: `*.unit.ron` is a format
  ([ADR-0007](../../docs/adr/0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md)).
- **2116**, golden tables (`shuyi_assets::golden`) and CI's
  `determinism` job, which check the format's fixture on x64, ARM64
  and wasm32.

Units move in straight lines only here. Paths are 2709 and 2710,
steering 2711, combat 2712.

## Nick input

None.

## Scope

**In:**
- **The crate** `crates/rts` (`shuyi-rts`), with every duty of
  [ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
  §4:
  - **metadata:** concern `kind`, answer `rts`; `serves` the top-down
    free and isometric directions (their ids in `docs/modules.ron`);
    `alternatives` empty (the `kind` concern's answers can share a game);
    `works_with` empty;
  - **the README** in the fixed order. Sections 1, 2 and 5 in full;
    section 3 with the WASM size measured now; section 4 with the
    smallest code that spawns two units and orders a move. It also
    says what buckets cost: a reaction can wait up to `k` steps
    (ADR-0015, *Consequences*). 2719 finishes the README;
  - **the `rts` feature** of the `shuyi` crate;
  - **`docs/modules.ron` and the catalog:** set the `rts` answer and
    the `shuyi-rts` row from parked to building (as 2701 does); 2719
    sets both to ready when the example passes CI (ADR-0009 §4). If 2003 somehow lacks the
    `kind` concern, add it as ADR-0015 §1 writes it (`pick: Many`,
    question "What kind of game is it?", `alternatives: false`, answers
    `tactics`, `rts`, `visual-novel`, `mystery`). Don't change
    `check-modules`; 2003 owns its rules;
  - **the skill** `shuyi-rts`, if the skill pack (1134) is done; if not,
    add it to 1134's list in this PR;
  - **`needs`** (ADR-0012 §1): if 3017 is done, a provisional `needs`
    key with a comment that 2718 measures it. If not, 3017 gives it one
    with the other modules;
  - **the wizard:** if 2502 is done, check that `--kind rts` appears in
    `docs/templates.md`'s flags list, and add it if not.
- **Features of the crate:** `assets` (on by default) holds the `Format`
  registration, so the simulation parts depend on `shuyi-core` alone
  when it is off (ADR-0015 §1); `view` is declared empty, filled by
  2716.
- **Units, `shuyi_rts::units`** (ADR-0015 §3):
  - `UnitId { index: u32, generation: u32 }`. An index of slots maps an
    id to a row. A removed id's generation goes up, so a stale id never
    finds a row. Free slots are reused last-freed first.
  - `UnitTable`: one `Vec` per column, all the same length, dense:
    `id`, `pos: FxVec2`, `vel: FxVec2`, `facing` (1119's 256-step
    angle), `kind: KindIndex`, `owner: PlayerId`, `health: i32`,
    `orders: OrderQueue`, `flags: UnitFlags` (hand-written bit flags; no
    new dependency).
  - **Read the last state, write the next:** `pos` and `vel` have a
    second buffer (`pos_next`, `vel_next`). A pass reads the last
    buffers and writes only its own row in the next ones. `swap()` at
    the end of the step. Other columns change only in the command phase
    at the start of a step.
  - `spawn(kind, owner, pos) -> UnitId` and `remove(id) -> bool`, both
    queued during a step and applied after the last pass: removals in
    `UnitId` order, each swapping the last row in and updating the
    index, then spawns in the order queued.
  - `get(id) -> Option<UnitRef<'_>>`, a read view of one row, and
    column slices for passes (`positions()`, `kinds()`, …).
  - serde of the whole table (rows in row order, plus the slot index,
    so ids round-trip), a `hash()` over every column in row order with
    the hash 1119's sims use, and a text dump: one line per row, `row
    id kind owner pos vel facing health orders flags`.
- **Kinds as data:** `UnitKind` from `*.unit.ron`:
  - `name`, `speed` (`Fx` per step), `radius`, `health`, `armour`,
    `weapons` (range, cooldown in steps, damage, an optional
    projectile; 2712 uses them), `sight`, `cost` (a list of resource
    name and amount), `build_time` in steps, `movement` (a movement
    class name; 2709 resolves it), `footprint` (cells, for buildings,
    whose speed is 0), `produces` and `builds` (kind names), `gathers`
    (bool);
  - `format_version: 1`, `deny_unknown_fields`; later tickets add
    fields with `#[serde(default)]`;
  - `UnitKinds`: every loaded kind, indexed in name order, so a
    `KindIndex` is the same on every machine. Loading checks unknown
    kind names in `produces` and `builds`, a radius ≤ 0 and a negative
    speed, each error naming the file and the kind;
  - **the format** (ADR-0007): `unit-kind`, extension `.unit.ron`, a
    `Format` registered by `shuyi_rts::register`, a fuzz-style test,
    and a determinism fixture (the decoded fixture kinds' hash as a row
    of a golden table, 2116's `shuyi_assets::golden`, with `-p
    shuyi-rts` in the wasm32 run of CI's `determinism` job).
- **Commands, `shuyi_rts::orders`** (ADR-0015 §2):
  - `RtsCommand<G = NoGameCommand>`:
    - `Move { units, to, queued }`;
    - `Attack { units, target, queued }`;
    - `AttackMove { units, to, queued }`;
    - `Patrol { units, to, queued }`;
    - `Hold { units }` and `Stop { units }`;
    - `Build { units, kind, at, queued }` and `Gather { units, resource,
      queued }`;
    - `Produce { building, kind, count }` and `Rally { building, to }`;
    - `Game(G)` for a game's own, passed to an `RtsGame::command` hook
      the game implements.
  - `units` is a list of `UnitId`, capped by a setting (default 1 024).
  - `queued: true` adds to the unit's order queue; `false` replaces it.
    The player's "add to queue" action decides it (2716); nothing here
    names a key.
  - A command naming a unit the player doesn't own, or a dead one,
    skips that unit and emits `RtsEvent::CommandIgnored { player,
    unit, reason }`.
  - `OrderQueue`: a fixed-capacity ring of `Order`s per unit (capacity a
    setting, default 8; one more is refused with an event).
- **`RtsSim<G>`**, implementing `CommandSim` (2706):
  - it holds the `UnitTable`, the `UnitKinds`, the step count, the
    seed's random state and `RtsSettings`;
  - `RtsSettings`: serde, with defaults, round-trips through RON; the
    bucket counts (§3) and the caps above. A game keeps it in a file
    (ADR-0009 §4, settings as data);
  - a step: apply the due commands in order; for each row, read the
    head order and move straight toward its point at the kind's speed;
    within one step's distance, snap to the point, pop the order and
    emit `Arrived`; `Patrol` swaps its two ends and keeps going;
    `Attack` and `AttackMove` move to the point (the target's position)
    until 2712; then swap buffers and apply removals and spawns;
  - `Build`, `Gather`, `Produce` and `Rally` are validated and queued,
    and the unit stands still with that order at the head (see *Out*).
- **Buckets** (ADR-0015 §3): `Buckets { k }` with `runs(step, id) ->
  bool` (`step % k == id.slot() % k`, where the slot is the
  `UnitId`'s index, which a swap-remove never changes) and
  `due(step, &table)`, an iterator over this step's bucket in row order.

**Out (do not do):**
- what `Build`, `Gather`, `Produce` and `Rally` do in the world
  (resources, workers, construction, production): 2733, the RTS
  economy, and 2734 for the player's side of it;
- the spatial grid (2708), paths and movement classes (2709, 2710),
  steering and groups (2711), combat (2712), vision (2713), AI (2714,
  2715), the view and selection (2716);
- lockstep (2720);
- the example and the template (2719).

## Implementation steps

1. Read 2705's report. Create `crates/rts` with its metadata, features,
   README start, `shuyi` feature and catalog status.
2. Write `UnitId`, the slot index and `UnitTable`, with the property test
   against a model before anything uses it.
3. Write `UnitKind`, `UnitKinds`, the `unit-kind` format, the fixtures
   and the error fixtures.
4. Write `RtsCommand`, `Order`, `OrderQueue` and validation.
5. Write `RtsSim`, `RtsSettings`, `Buckets`, the text dump and the hash.
6. Add the skill (or 1134's list entry), the `needs` key (or nothing),
   and the wizard flag check.

## Acceptance criteria

- [ ] Under random spawns and removals, `UnitTable` holds exactly what a
      `BTreeMap` model holds, its columns stay the same length, and no
      live id is ever reused (property test).
- [ ] A stale `UnitId` never returns a row (property test).
- [ ] Replaying an `RtsSim` record twice gives the same hash list
      (property test over random commands).
- [ ] A queued order runs after the ones before it; an order that isn't
      queued replaces the queue; a patrol keeps going between its ends
      (unit tests).
- [ ] A command naming another player's unit leaves that unit unchanged
      and emits `CommandIgnored`.
- [ ] With no removals, every row runs exactly once in any `k`
      consecutive steps (property test on `Buckets`).
- [ ] The fixture kinds decode to their golden row in CI's `test` and
      `determinism` jobs; each bad fixture gives an error naming the
      file and the kind; arbitrary bytes into the format never panic.
- [ ] A snapshot of the text dump after 120 steps of the fixture passes.
- [ ] `cargo xtask check-modules` passes, with `rts` building under `kind`.
- [ ] `shuyi-rts` builds with `--no-default-features` and depends only on
      `shuyi-core` then (`cargo tree`).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `OrderQueue`; command validation; each kind error; straight
  moves and arrival.
- Property: `UnitTable` against a model; stale ids; replay determinism;
  buckets.
- Fuzz-style: arbitrary bytes into the `unit-kind` format never panic.
- Fixture and determinism: the decoded kinds' golden row (x64, ARM64,
  wasm32).
- Snapshot: the text dump after 120 steps.

## Completion notes
