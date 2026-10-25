---
id: "2111"
title: "Stamped records: the engine version, the game's stamp and a fingerprint of the content in records and saves, and shuyi replay naming a mismatch"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2111 — Records and saves stamped with what made them

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6 and
Unreal 5.8. Unreal writes into each saved package the engine version
that saved it, and refuses a package from a newer engine. Games that
replay inputs tie a replay to what made it: Factorio refuses a replay
from an older version, and adding or removing mods breaks one
(`wiki.factorio.com/Replay_system`). Bevy and Godot have no stamp of
their own.

What Shuyi has (read on 2026-10-21):

- ADR-0006 §2: a session's handshake checks the engine version and a
  hash of the loaded content. ADR-0007 §5: decoders are deterministic
  because content hashes are part of replays and the handshake. Nothing
  computes that hash: `Bundle` (`crates/assets/src/bundle.rs`) has no
  fingerprint, and the only `fingerprint` is the contract's, of its
  schemas (`crates/cli/src/contract.rs`).
- A pack has a stamp, the game's word for the shapes of its own kinds
  (`PackInfo::stamp`, `crates/assets/src/pack.rs`); `Pack::open` refuses
  another. Hello-grid's is `PACK_STAMP`, its name and version
  (`examples/hello-grid/src/lib.rs`).
- `shuyi replay` (`crates/cli/src/contract/replay.rs`) hands the game a
  `Record` (a file and its text) and compares the state hash at every
  step. The grid template's and hello-grid's `StageRecord` hold a seed
  and commands (`templates/grid/crates/game/src/tools.rs`,
  `examples/hello-grid/src/tools.rs`). A record made with other content
  or another engine runs anyway, and its difference looks like a desync.
- `SaveHeader` (`crates/core/src/save.rs`) reads a save's version and
  nothing else. A save that replays its commands can play differently
  after a content change, and nothing tells (game ADR-0039 accepts it).
- Plans, all parked: 1119's `TickRecord` (a seed and inputs), 2706's
  `CommandRecord` (a `format_version`), and 1140's handshake ("a content
  hash the game supplies") with 2720's, which builds on it. None has a
  stamp. After the MVP, 2518 (session records) and 2520 (crash replays)
  need one, and 2116 (determinism fixtures) pins the fingerprint.

Why it matters:

- A record, or a crash report's replay (2520), of Visions of Shuyi from
  one patch, played on another, should say "made with other content",
  not show a desync that isn't one.
- Every game with replays or online play (1140) needs the same hash, so
  it is computed once, in the engine, the same way everywhere.
- The end goal (2026-10-19, `docs/decisions.md`) is games made by
  agents: an agent reading `shuyi replay`'s answer learns the real cause
  of a difference.

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::made`** (`no_std`, serde only, no new dependency):
  - `ENGINE_VERSION: &str`, `env!("CARGO_PKG_VERSION")` (every `shuyi-*`
    crate has the workspace's version);
  - `ContentHash(pub u64)` (`Debug, Clone, Copy, PartialEq, Eq,
    Default`), shown and serialized as sixteen lowercase hex digits, as
    `StateHash` shows itself (a string, since JSON can't hold every
    `u64`), and read back from one;
  - `Made { engine: String, game: String, content: ContentHash }`
    (serde), and `Made::new(game: impl Into<String>, content:
    ContentHash)`, which fills `engine`. `game` is the game's word for
    its build, as a pack's stamp is;
  - `Made::check(&self, now: &Made) -> Result<(), Mismatch>`. `Mismatch`
    lists each part that differs with both values, and shows as "made
    with engine 0.1.0, content 1a2b3c4d5e6f7a8b; this build: engine
    0.2.0, content 3c4d5e6f7a8b9c0d";
  - the module docs say how a game stamps its own save or record (a
    `made: Made` field, checked with `Made::check` after reading), and
    that the engine's version moves only at a release: a game pinned to
    a git revision that wants the revision in its stamp puts it in
    `game`;
  - `crates/core/src/lib.rs` declares `pub mod made;` and re-exports
    `Made` and `ContentHash` beside `StateHash`.
- **`Bundle::fingerprint(&self) -> ContentHash`**
  (`crates/assets/src/bundle.rs`):
  - over the layers that ship, never the test layers, whatever the
    thread (`story_only` doesn't change it);
  - a running value starts at FNV-1a's offset basis, and the shipped
    layers are taken from the last read to the first (the base layer
    first). Each file of a layer, in path order, is fed as its path's
    length and bytes, then its contents' length and bytes, lengths as
    `u64`;
  - the step is the word-wise FNV-1a step of the pack's `checksum`
    (`crates/assets/src/pack.rs`), moved into a small `pub(crate)`
    hasher both call, so `checksum` gives what it gave;
  - the fingerprint is the running value after the first layer, with no
    closing step, so a layer laid over others changes it as its files
    do;
  - it reads the layers' `Files` directly, not through `Bundle::bytes`,
    so the watcher (`bundle::watch`) isn't told. It reads every shipped
    byte: a game computes it once and keeps it in its `Made`.
- **A pack stands for its sources:**
  - `Bake::bake` (`crates/assets/src/pack/bake.rs`) records the source
    bundle's fingerprint in `PackInfo` (`content: ContentHash`), and
    `pack::VERSION` becomes 3 (a pack is refused, never migrated: bake
    again; the pinned snapshot of the binary forms is accepted again);
  - `Pack::into_layer` marks its layer as standing for that fingerprint:
    a `Layer` field `stands_for: Option<ContentHash>`, set by a const
    builder `Layer::standing_for(content)` as `Layer::private` is;
  - when the fold reaches such a layer, the running value becomes its
    fingerprint instead of taking its files, and the layers read after
    it no longer count (a pack holds every file of the bundle it was
    baked from);
  - so a build that ships one pack has the fingerprint of the loose files
    it was baked from, and loose layers over a pack fold in as they would
    over those files.
- **`shuyi replay` names a mismatch:**
  - `GameTools::made(&self) -> Option<Made>` (`crates/cli/src/game.rs`),
    defaulted to `None`: this build's stamp;
  - `ReplayRun` gains `made: Option<Made>`, which the game's `replay`
    sets from its record (`turns` leaves it `None`). `shuyi-core` has no
    `schemars`, so the schema shows it through a private `MadeShape` of
    three strings (`#[schemars(with = "Option<MadeShape>")]`);
  - before the hashes are compared, a run whose `made` differs from the
    game's is refused: the answer fails with the `Mismatch` text,
    naming the record;
  - `ReplayInput` gains `any_stamp: bool` (`#[serde(default)]`), and
    the command line takes `--any-stamp` (the `replay` arm of
    `crates/cli/src/contract/cli.rs`, its usage line, and the help line
    in `crates/cli/src/lib.rs`). With it the runs go ahead, and `limits`
    says a difference may come from the stamp;
  - a record without a stamp, or a game whose tools give none, runs as
    today, with a line in `limits`;
  - `text` shows each run's stamp; the contract's `VERSION` goes up and
    the schemas and `crates/cli/schemas/versions.json` are regenerated
    (ADR-0022 §5);
  - the refusal gets its own problem code and help (2509's table).
  - **Either order with 2520:** if it is done, a test shows
    `replay --crash` refusing a crash report whose record is stamped
    with other content. If not, nothing to do (2520 adds the test).
- **The grid template and hello-grid stamp their records:**
  - each crate's `lib.rs` gets `pub fn made() -> Made`:
    `Made::new(stamp, bundle.fingerprint())` over the bundle of its
    embedded files. Hello-grid's stamp is `PACK_STAMP` and its bundle
    `Bundle::new(vec![layer()])`, as its loaders read. The template
    reads `FILES` with no `Bundle`, so it gains `pub fn layer() ->
    Layer` as hello-grid has, and its stamp is its crate's name and
    version (`concat!(env!("CARGO_PKG_NAME"), " ",
    env!("CARGO_PKG_VERSION"))`);
  - **Either order with 3048:** if it is done, the template's stamp is
    `APP.id` and `APP.version` from its generated
    `crates/game/src/app_info.rs`. If not, 3048 changes the stamp to
    them;
  - `StageRecord` gains `made: Option<Made>` (`#[serde(default)]`, so a
    record without one still reads), written by `bots`. Their tools'
    `made()` is `Some(crate::made())`, and `replay` passes the record's
    stamp on;
  - **Either order with 3046:** if it is done, the `crash_record`
    functions beside `StageRecord` in the stage screens' modules
    (hello-grid's `src/screens/stage.rs`, the template's
    `crates/game/src/stage.rs`) stamp the record they write with the
    same `made()`. If not, 3046 adds it.
- **Either order with 2116:** if it is done, the fixed bundle's
  fingerprint is a row `fingerprint` of
  `crates/assets/tests/golden/formats.ron` (through
  `shuyi_assets::golden`) instead of a constant in a test, and Hello,
  Grid's table (`examples/hello-grid/tests/golden/hello-grid.ron`) gets
  a row `fingerprint files`, with its pack's fingerprint `assert_eq!` to
  it. CI's `determinism` job then checks both on wasm32. Hello, Grid's
  checked-in record (`examples/hello-grid/tests/records/`) stays
  unstamped, so a content change shows as its rows moving. If 2116 is
  not done, it adds the rows.
- **From 2533 (done):** each record of the `shuyi playtest` history
  (`target/playtest-history/*.jsonl`, `history::Record` in
  `crates/cli/src/contract/playtest/history.rs`) gains
  `made: Option<Made>` (`#[serde(default)]`, so older lines still
  read), from this build's `GameTools::made()`, set in
  `playtest::kept`.
- **Either order with the parked records:** if 1119, 2706, 1140 or 2720
  is done when this lands, its record or handshake (`TickRecord`,
  `CommandRecord`, the lockstep handshakes) takes a `Made` here. If not,
  it takes one when it is unparked.
- **An ADR** (the next free number in `docs/adr/`, the `write-adr`
  skill): what a stamp holds; how the fingerprint is computed and why a
  pack stands for its sources; that the engine's version moves only at
  a release; and that the tools refuse a mismatch by default while a
  game decides what its own saves do with one. It builds what ADR-0006
  §2 and ADR-0007 §5 promised. ADR-0023 stays as written: the new ADR
  extends its §1 (the index holds the sources' fingerprint), in its
  `Extends:` line and its row in `docs/adr/README.md`.
- **Docs:** `docs/contract.md`'s `GameTools` table gains a `made` row,
  and its `replay` rows say a record stamped with other content is
  refused unless `--any-stamp`; `docs/formats.md`'s *The pack* says the
  index holds the sources' fingerprint.
- **For the game:** packs must be baked again (pack version 3). `Layer`
  gains a public field, which a game built with `Layer::dir`,
  `Layer::table` and the builders doesn't notice. `PackInfo`,
  `ReplayInput` and `ReplayRun` gain a field each, and `GameTools` a
  defaulted method. Say so in the PR, with 2010's `Breaking:` lines;
  the game's PR that moves the pin follows.

**Out (do not do):**
- What a game does with a save made with other content: its own load
  screen decides (Visions of Shuyi's under game ADR-0039).
- Reading a save's stamp in `SaveHeader`: a save of the build's version
  parses whole, so a game reads its `made` with the rest, and a save of
  another version is refused by its version (or upgraded, 2110).
  `SaveHeader` stays as it is, `Copy` included.
- Session records (2518): they carry a `Made`, added in their ticket. A
  crash replay (2520, `replay --crash`) goes through `replay`'s path, so
  this ticket's refusal covers it.
- Leaderboard submissions (3054, parked): a `Submission`'s `record` is
  the game's own record text, so it carries whatever stamp the game's
  record carries. **Either order with 3054:** if 3054 is done, nothing
  to do here; if not, nothing to do either: 3054 passes the record on
  as it gets it.
- Playing a record across versions; reading older saves (2110).
- `History` (`crates/core/src/history.rs`) stays as it is: it lives in
  memory too, and a record or a save holds a `Made` beside it.
- A stamp in the engine's own records (settings, key bindings,
  profiles): they hold no content.
- A cryptographic hash or signed content: the fingerprint finds another
  build, not an attacker (ADR-0023's Alternatives).

## Implementation steps

1. Read ADR-0006 §2, ADR-0007 §5, ADR-0022 §5, ADR-0023 §1 and §5,
   `crates/assets/src/bundle.rs`, `crates/assets/src/pack.rs` and
   `pack/bake.rs`, `crates/cli/src/contract/replay.rs`,
   `crates/cli/src/contract/cli.rs` and `crates/cli/src/game.rs`.
2. Write `shuyi_core::made` and its re-exports.
3. Move the pack checksum's step into the shared hasher (the test
   `the_checksum_is_pinned_and_folds_its_high_bits_down` must pass
   unchanged). Write `Layer::standing_for` and `Bundle::fingerprint`,
   then `PackInfo::content`, `Pack::into_layer`'s mark and `VERSION` 3,
   and accept the new snapshot.
4. Change the contract: `GameTools::made`, `ReplayRun::made` and its
   schema shape, the refusal, `any_stamp` and `--any-stamp`, the text,
   the schemas and the version line.
5. Stamp the grid template's and hello-grid's records, with the
   either-order parts that apply. The templates are outside `cargo fmt
   --all`: format them as CI's template job checks them.
6. Write the ADR and the docs, and run the gates.

Size: if the diff runs well past 600 changed lines (schemas and
snapshots aside), move *A pack stands for its sources* (step 3's
`Layer::standing_for`, `PackInfo::content` and pack `VERSION` 3) to a
follow-up ticket that 2519 and 2520 also wait for, and keep the stamp,
the fingerprint of loose files and the contract here.

## Acceptance criteria

- [ ] `shuyi-core` gains no dependency, and `shuyi-core` and
      `shuyi-assets` still build for `thumbv7em-none-eabihf` (CI's
      no-std job).
- [ ] Changing one byte, renaming one file or adding one file in a
      shipped layer changes `Bundle::fingerprint`; changing a test layer
      doesn't, inside `story_only` or not.
- [ ] A fixed bundle's fingerprint is a constant checked in by a test (a
      golden row, if 2116 is done); the hash uses no integer whose size
      depends on the platform.
- [ ] The pack's pinned checksum test passes unchanged.
- [ ] Hello-grid's files, baked into a pack and loaded back as the only
      layer, have the fingerprint of the loose files; a loose layer laid
      over that pack gives what it gives over the loose files.
- [ ] `shuyi replay` on a hello-grid record stamped with other content
      fails, naming both stamps; with `any_stamp` (`--any-stamp` on the
      command line) it runs and says why in `limits`; an unstamped
      record runs as before, with its line in `limits`.
- [ ] The grid template's `bots` records carry `made`, and its replay
      test passes in CI's template job.
- [ ] The contract's schemas and version line are regenerated, and the
      schema test passes.
- [ ] The ADR is written and listed in `docs/adr/README.md`;
      `docs/contract.md` and `docs/formats.md` say what changed.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Made::check` for each part that differs, and the `Mismatch`
  text; `ContentHash`'s text form both ways, in RON and JSON; the
  fingerprint's rules (shipped layers only, path order, one byte, one
  name, one new file); the fold with a loose layer over a pack layer,
  and a layer under a pack not counted; `--any-stamp` read by the
  `replay` arm.
- Property: changing one random byte of one random file of a random
  bundle changes its fingerprint.
- Integration: `shuyi replay` with stamped, mismatched and unstamped
  records (the contract's fake game in
  `crates/cli/src/contract/fake.rs`, and hello-grid's
  `tests/it/contract.rs`); hello-grid baked, then fingerprinted
  (`tests/it/pack.rs`); the template's replay test
  (`templates/grid/crates/game/tests/it/main.rs`) with a stamped record.

## Completion notes

*(Filled in by the session that completes the ticket.)*
