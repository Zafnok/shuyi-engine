---
id: "2114"
title: "RNG helpers: ranges, chances, picks, shuffles, bags and weighted picks on RandomSource, and named streams and forks of SimRng"
type: feature
milestone: After MVP
model: opus-5.5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2114 — RNG helpers: ranges, picks, shuffles, bags, named streams and forks

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker and Construct 3. Each of
them gives a game more than a raw number from its random generator:

- Godot's `RandomNumberGenerator` has a `seed` and a `state`,
  `randi_range`, and `rand_weighted`, which takes float weights and
  returns an index
  (https://docs.godotengine.org/en/stable/classes/class_randomnumbergenerator.html);
- Unreal's `FRandomStream` is a seeded stream a system keeps as its
  own, with `RandRange`;
- Unity's `Unity.Mathematics.Random` is a seeded value each job carries;
- Bevy has none in the engine; the community crate `bevy_rand` gives
  each entity an RNG forked from a parent;
- GameMaker has `choose`, `irandom_range` and `array_shuffle`
  (https://manual.gamemaker.io/monthly/en/GameMaker_Language/GML_Reference/Variable_Functions/array_shuffle.htm);
- Construct 3's Advanced Random plugin has permutation tables and
  probability tables.

What Shuyi has today, in `crates/core/src/rng.rs`:

- `SimRng`, a PCG32 of our own, with `new(seed)`, `next_u32`,
  `roll_percent` and `roll_below`. `with_stream` (line 55) is private:
  only `new` and a test call it, `new` always with stream 54.
- `RandomSource`, the trait rules take, with only `roll_percent` and
  `roll_below`; and `ScriptedRng`, which scripts both for tests.
- No range, pick, shuffle, weighted pick, bag, named stream or fork.

So callers write the same conversions by hand. `RandomBot::choose`
(`crates/core/src/bots.rs:220`) and the grid template's `wander`
(`templates/grid/crates/rules/src/lib.rs:206`) both turn a length into
a `u32`, draw, and turn the draw back into a `usize`.

SplitMix64 is written three times: `shuyi_ui::audio::splitmix64`
(`crates/ui/src/audio.rs:259`, public, the music pool's pick), `mix`
(`crates/platform/src/app.rs:647`, the launch seed) and `VariantRng`
(`crates/app/src/audio.rs:360`, sound variants; 2345 moves it into
`shuyi-platform`). The parked 2103 needs a stateless one for particles
and takes this ticket's `mix_at`. FNV-1a is in `crates/core/src/hash.rs`
(`Hasher::bytes`).

Planned users: the parked 2103 (particles), 2118 (noise), 2119 (level
generators and weighted tables) and the parked examples 2735, 2736 and
2737 wait on this ticket, and 2108's parallel schedule forks a stream
for each system that draws (its either-order clause). No ticket plans
the helpers themselves.

Why it matters:

1. **The game first.** Visions of Shuyi draws a battle's luck from one
   `SimRng` in its state, and its own soak test writes a `pick(rng, n)`
   helper as `next_u32() % n`, the biased draw below. With one stream,
   any new draw (a bot's tie break, a cosmetic pick) moves every later
   draw, so recorded replays and golden hashes change. A stream per
   purpose keeps a new draw from moving the others. Everything here is
   added, nothing changes: the game's next pin bump builds as it is.
2. **Other 2D games.** A card game deals from a shuffled deck, a puzzle
   draws its next piece from a bag of seven, a grid game puts a pickup
   on a free cell, a shop rolls a weighted stock. Without helpers an
   agent writes `next_u32() % n` (biased) or adds the `rand` crate,
   whose book says minor releases may make value-breaking changes
   (https://rust-random.github.io/book/crate-reprod.html). A dependency
   bump would then change a game's replays.
3. **The end goal** (`docs/decisions.md`, 2026-10-19): a small game in
   few lines. Until real time is unparked, the agent check measures
   turn-based games (2026-10-20), and those use exactly these.

Weighted *tables as data* are loot-system design: the parked 2119 has a
checked `WeightedTable` of `(u32, T)` read from a game's data table,
which picks through this ticket's `pick_weighted`; a table kind, nested
tables, "nothing" and guaranteed entries are the game's, not this
ticket's (`AGENTS.md` rule 4: when unsure, it is game).

**From 2340 (open):** `audio::splitmix64` in `crates/ui/src/audio.rs` becomes a re-export of `shuyi_core::rng::mix64` here; 2340's shake offsets use that same function either way.

## Nick input

None.

## Scope

**In:**
- **One mixing function, in `shuyi_core::rng`** (`no_std`, integers
  only):
  - `pub fn mix64(x: u64) -> u64`: SplitMix64's output for the state
    `x`: add `0x9E37_79B9_7F4A_7C15`, then the finalizer. It is the body
    of today's `shuyi_ui::audio::splitmix64`, moved.
  - `pub fn mix_at(seed: u64, index: u64) -> u64`:
    `mix64(seed.wrapping_add(index.wrapping_mul(0x9E37_79B9_7F4A_7C15)))`,
    the `index`-th output of SplitMix64 started at `seed`, with no
    state. 2103's particles use it. The parked 2118 hashes its noise
    lattice with it, so its frozen outputs also hold every seeded noise
    value.
  - `pub(crate) fn fnv1a(bytes: &[u8]) -> u64` in
    `crates/core/src/hash.rs`, over the `Hasher` already there (no
    second copy of its constants).
- **Named streams and forks of `SimRng`:**
  - `SimRng::stream(seed: u64, name: &str) -> SimRng`: with
    `id = fnv1a(name.as_bytes())`, it is
    `SimRng::with_stream(mix_at(seed, id), id)`. One stream per purpose
    (`"combat"`, `"loot"`, `"weather"`): draws on one never move
    another. Names are told apart by a 64-bit hash (documented).
  - `SimRng::fork(&mut self, name: &str) -> SimRng`: takes exactly two
    `next_u32` draws from `self` (the first is the high half of a
    64-bit seed) and returns `SimRng::stream(that seed, name)`. It
    makes a child for a sub-task (a level built from a run's luck, a
    bot's search): however much the child draws, the parent moved by
    two.
  - `SimRng::new` is unchanged, so every value pinned today stays.
- **Helpers on `RandomSource`**, as default methods built on
  `roll_below` alone, so `SimRng`, `ScriptedRng` and a game's own
  source all have them. None calls `roll_below(0)`, and each makes the
  draws this table says, no more:

  | Method | Gives | Draws |
  | ------ | ----- | ----- |
  | `range(r: Range<i32>) -> i32` | a value in `r`; `r.start` when `r` is empty | one `roll_below(r.end - r.start)`; none when empty |
  | `chance(num: u32, den: u32) -> bool` | `true` with odds `num / den`; `false` when `den` is 0 | one `roll_below(den)` when `den > 0`, even when `num >= den`, so changing the odds never changes the draw count |
  | `index(len: usize) -> Option<usize>` | a value in `0..len`; `None` when `len` is 0 | one `roll_below(len)`; none when 0 |
  | `pick<'a, T>(items: &'a [T]) -> Option<&'a T>` | an item | as `index` |
  | `shuffle<T>(items: &mut [T])` | the items in a random order (Fisher-Yates) | for `i` from `len - 1` down to 1, one `roll_below(i + 1)`, then swap `i` with the draw: `len - 1` draws |
  | `weighted_index(weights: &[u32]) -> Option<usize>` | `i` with odds `weights[i] / total`; `None` when the total is 0 or above `u32::MAX` | one `roll_below(total)`, the total summed in `u64` with saturating adds; none on `None` |
  | `pick_weighted<'a, T>(items: &'a [(u32, T)]) -> Option<&'a T>` | an item, by the weight beside it | as `weighted_index` |

  - `range` works in `i64` between the ends, so `i32::MIN..i32::MAX`
    works and nothing wraps.
  - `index` on a slice longer than `u32::MAX` picks among its first
    `u32::MAX` items, as `RandomBot` does today (documented).
  - The generic ones (`pick`, `shuffle`, `pick_weighted`) are
    `where Self: Sized`, so `&mut dyn RandomSource` still works with
    the others.
  - `(u32, T)` is a shape a game's own RON data already reads into
    (`shuyi_assets::parse_ron` or the `Ron<T>` format). This ticket
    adds no format.
- **`Bag<T>`** in `shuyi_core::rng`, re-exported at the crate root: a
  shuffle bag (a "7-bag"), kept in the game's state.
  - `Bag::new(items: Vec<T>)`; `draw(&mut self, rng: &mut impl
    RandomSource) -> Option<&T>` (`None` only for a bag with no items,
    with no draw); `left(&self) -> usize`; `items(&self) -> &[T]`.
  - It holds the items and the indices still in the bag. When none are
    left, `draw` refills them with `0..len`, `shuffle`s them, then deals
    from the end. So every `len` draws hold each item once.
  - `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Serialize` and
    `Deserialize`. Deserializing checks that each index left is below
    the item count and appears once (`#[serde(try_from)]` on a plain
    struct), so a hand-edited save can't make `draw` panic.
- **The callers move onto the helpers**, with the same draws:
  - `RandomBot::choose` (`crates/core/src/bots.rs`) uses `index`;
  - the grid template's `wander` (`templates/grid/crates/rules/src/lib.rs`)
    uses `index`, and imports `RandomSource`;
  - `shuyi_ui::audio::splitmix64` becomes
    `pub use shuyi_core::rng::mix64 as splitmix64;` with its doc. The
    game imports that name, so it stays. Its test
    `splitmix64_matches_the_reference` stays and passes.
- **An ADR** (the `write-adr` skill, the next free number), extending
  ADR-0002 §2, "The seeded RNG's outputs are frozen":
  - the outputs of `SimRng::new`, `SimRng::stream`, `SimRng::fork`,
    `mix64` and `mix_at`, and the draws of every helper and of `Bag`
    (the table above), are part of the engine's promise, because saves,
    replays and golden hashes depend on them;
  - a change to any of them is a new function with a new name, never a
    changed one; changing a pinned value in `rng.rs`'s tests is a
    breaking change (2010's `Breaking:` line);
  - a game keeps one stream per purpose, and forks for sub-tasks;
  - a simulation never takes values from the `rand` crate.
- **Docs for agents:** `rng.rs`'s module doc says which to use (`new`
  for one stream, `stream` for one per purpose, `fork` for a sub-task),
  links the ADR, and has three doctests: dealing a shuffled deck,
  putting a pickup on a free cell with `pick`, and drawing pieces from
  a `Bag` of seven.
- **Either order:**
  - with 2511 (done): add `Bag` and `RandomSource` to its prelude
    (`shuyi_platform::prelude`, `crates/platform/src/lib.rs`).
    `shuyi-platform` then gains a direct `shuyi-core` dependency
    (already in its tree through `shuyi-ui`), with
    `default-features = false` as `shuyi-ui` has it, so it stays
    `no_std`, and its line in `tools/layers.ron`;
  - **from 1134 (done):** its `shuyi-stage-grid` skill
    (`skills/shuyi-stage-grid/SKILL.md`) gains a short section naming
    these helpers (`range`, `chance`, `pick`, `shuffle`,
    `pick_weighted`, `Bag`) and one stream per purpose
    (`SimRng::stream`, `fork`); `check-skills` passes;
  - from 2802 (done): the book's chapter 6 (*Rules as commands*,
    `site/content/learn/book/rules-as-commands.md`) gains a section on
    luck with a snippet in `site/snippets/`;
  - with 2116: the golden values below stay as asserts in `rng.rs`'s
    tests. 2116's `determinism` job runs `shuyi-core`'s tests on
    wasm32, so they are checked on x64, ARM64 and wasm32 whichever
    ticket lands first;
  - with 2108: if 2108 is done, give each system that draws its own
    stream, `SimRng::fork(<system name>)` on the world's `SimRng` in
    schedule order before the systems run, in both of `shuyi-ecs`'s
    schedules (serial and parallel), and add two such systems to 2108's
    property test; if not, nothing to do (2108 forks them, its scope
    says so);
  - with 2739 (parked, the snake example): if 2739 is done, its food
    placement moves onto `pick` with the same draw, like the callers
    above; if not, nothing to do (2739 uses `pick`, as its scope says);
  - `Bag` follows ADR-0039's rules for public types.

**Out (do not do):**
- weighted tables as data: a checked `WeightedTable` (2119, parked with
  roguelikes); a table asset kind, nested tables, "nothing" and
  guaranteed entries are the game's;
- float weights, float ranges, and distributions such as a bell curve
  (ADR-0002 §2: no floats in a simulation);
- noise (2118, parked);
- streams for the systems of a parallel schedule (2108 forks them),
  unless 2108 is done (the clause with 2108 above);
- the `rand` crate, or any new dependency;
- `VariantRng` (`crates/app/src/audio.rs`, which 2345 moves) and
  `shuyi-platform`'s `mix`: they never touch a simulation, so leave
  them;
- moving `roll_percent` callers (hello-grid's trip roll) onto `chance`:
  their `ScriptedRng` scripts would change;
- any change to what `SimRng::new`, `roll_percent` or `roll_below`
  give;
- a bag's preview of what comes next, and a bag that never repeats an
  item across a refill (a game keeps its own queue; 2345's `Shuffle`
  does it for sounds);
- dice notation such as `2d6` (it is two `range(1..7)`).

## Implementation steps

1. Read `crates/core/src/rng.rs`, `hash.rs`, `bots.rs`, the grid
   template's rules, `crates/ui/src/audio.rs` and ADR-0002 §2. Check
   which either-order tickets above are done.
2. Add `fnv1a`, `mix64` and `mix_at`, with the reference tests. Point
   `shuyi_ui::audio::splitmix64` at `mix64`.
3. Add `stream` and `fork`, with their tests.
4. Add the helpers, then `Bag`, each with its exact-draw tests through
   `ScriptedRng`.
5. Compute the golden values once and pin them (below), with a comment
   naming the ADR.
6. Move `RandomBot::choose` and the template's `wander` onto `index`.
   The template is outside the workspace and `cargo fmt --all`: check
   it as CI's `templates` job does (`shuyi new grid` with
   `--engine-path`, then its gates).
7. Write the ADR, the module doc and the doctests.
8. Run the gates.

If `rng.rs` passes about 800 lines, make it `rng/` with `bag.rs` and a
`tests.rs`, as `hash.rs` keeps its tests in `hash/tests.rs`. Moved
lines are not new work.

If the diff passes about 700 changed lines, leave `Bag` out, with its
doctest, its golden values and its tests. Write a follow-up ticket for
it that carries its 2511 and 2013 clauses (the `write-ticket` skill),
and say so in the completion notes.

## Acceptance criteria

- [ ] `mix64` and `mix_at` give SplitMix64's published outputs:
      `mix_at(0, 0) == 0xE220_A839_7B1D_CDAF` and
      `mix_at(0, 1) == 0x6E78_9E6A_A1B9_65F4`.
- [ ] `fnv1a` gives FNV-1a's published values: `""` gives
      `0xcbf2_9ce4_8422_2325`, `"a"` gives `0xaf63_dc4c_8601_ec8c`,
      `"foobar"` gives `0x8594_4171_f739_67e8`.
- [ ] Every helper makes exactly the draws in the table (the tests
      read `ScriptedRng::consumed_below`).
- [ ] Each helper's odds are exact: scripting every possible draw gives
      each outcome its share (tests below).
- [ ] `fork` moves its parent by exactly two draws.
- [ ] A draw added on one stream leaves another stream's outputs the
      same, and the same test with one shared `SimRng` shows them move.
- [ ] Golden values are pinned, all from seed 2026: the first 8
      `next_u32` of `SimRng::stream(2026, "loot")`, of
      `SimRng::new(2026).fork("level")` and of its parent after the
      fork; and, each from a fresh `SimRng::new(2026)`, a `shuffle` of
      `0..10`, 14 draws of a `Bag` of `0..7`, and 10
      `weighted_index(&[3, 0, 5])` draws.
- [ ] Every existing test in `shuyi-core`, `shuyi-ui`,
      `examples/hello-grid` and the grid template passes unchanged: no
      pinned value, snapshot or hash moves.
- [ ] `&mut dyn RandomSource` still compiles (a test takes one and
      calls `range` on it).
- [ ] `shuyi-core` builds for `thumbv7em-none-eabihf`, with no new
      `#[allow]`.
- [ ] The ADR is written and linked from `rng.rs`'s module doc.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit:
  - the reference values above;
  - edges: an empty range, `range(i32::MIN..i32::MAX)`, `chance(0, 5)`,
    `chance(5, 5)`, `chance(1, 0)`, `index(0)`, `index(1)`, `pick` of
    nothing, `shuffle` of 0 and 1 items (no draws), weights with a 0,
    all zeros, and a total of `u32::MAX + 1`;
  - `Bag`: no items, one item, a refill after `len` draws, a bag saved
    mid-way resumes the same, a bad saved index is refused.
- Exact odds through `ScriptedRng`, every draw scripted once:
  - `weighted_index(&[3, 0, 5])` over draws `0..8`: index 0 three
    times, index 2 five times, index 1 never;
  - `chance(3, 7)` over draws `0..7`: `true` three times;
  - `range(-2..3)` over draws `0..5`: each value once;
  - `shuffle` of 4 items over all 24 draw scripts: each of the 24
    orders once.
- Property (proptest): `range` stays in its range; `shuffle` gives a
  permutation; every `len` draws of a `Bag` hold each item once; for
  random seeds and two different names, the streams' first 16 outputs
  are not the same list; 1 000 names over one seed share no pair of
  first two outputs.
- Serde: a stream, a fork and a `Bag` round-trip through RON and resume
  the same draws.
- Integration: `hello-grid`'s bot and determinism tests; the grid
  template's own tests in CI's `templates` job.

## Completion notes

*(Filled in by the session that completes the ticket.)*
