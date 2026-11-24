# ADR-0143: The seeded RNG's outputs are frozen: its helpers, named streams and forks too

- **Status:** Accepted
- **Date:** 2026-11-23
- **Related tickets:** 2114
- **Extends:** ADR-0002 §2

## Context

ADR-0002 §2 says a simulation's randomness comes from a seeded RNG it
owns, and the same seed and inputs give the same state on every
platform. `shuyi_core::rng::SimRng` is a PCG32 written in the crate for
that reason: its output is fixed by one file, not by a dependency's
version. The `rand` crate's book says its minor releases may change
values (https://rust-random.github.io/book/crate-reprod.html), so a
dependency bump would change a game's replays.

Ticket 2114 adds, on top of `SimRng`:

- `mix64` and `mix_at`, SplitMix64 as a function of a state, or of a
  seed and an index;
- `SimRng::stream(seed, name)`, one stream per purpose, and
  `SimRng::fork(name)`, a child for a sub-task;
- helpers on `RandomSource` (`range`, `chance`, `index`, `pick`,
  `shuffle`, `weighted_index`, `pick_weighted`) and a shuffle `Bag`.

Saves, recorded replays and golden hashes (`state_hash`) all depend on
the values these give and on how many draws each takes. Visions of
Shuyi keeps one `SimRng` in a battle's state, so a new draw anywhere
moves every later draw.

## Decision

1. **Frozen outputs.** The outputs of `SimRng::new`, `SimRng::stream`,
   `SimRng::fork`, `mix64` and `mix_at`, and the draws of every
   `RandomSource` helper and of `Bag`, are part of the engine's promise:
   - `range(r)`: one `roll_below(r.end - r.start)`, none when empty;
   - `chance(num, den)`: one `roll_below(den)` when `den > 0`, whatever
     `num`;
   - `index(len)` and `pick`: one `roll_below(len)`, none when 0;
   - `shuffle`: Fisher-Yates from the end, one `roll_below(i + 1)` for
     `i` from `len - 1` down to 1;
   - `weighted_index` and `pick_weighted`: one `roll_below(total)`, none
     when the total is 0 or above `u32::MAX`;
   - `Bag::draw`: a `shuffle` of `0..len` when the bag is empty, then
     deals from the end; no draw otherwise.

   `stream` is PCG32 stream `fnv1a(name)` seeded with
   `mix_at(seed, fnv1a(name))`; `fork` takes two `next_u32` (the first
   the high half) as the seed of `stream(seed, name)`.
2. **A change is a new name.** A different algorithm or draw count is a
   new function, never a changed one. The values pinned in
   `crates/core/src/rng.rs`'s and `crates/core/src/rng/tests.rs`'s tests
   (all from seed 2026) are the check: changing one is a breaking change
   with a `Breaking:` line (ADR-0038). 2116's `determinism` job runs
   them on x64, ARM64 and wasm32.
3. **One stream per purpose.** A game keeps a stream for each purpose
   (`"combat"`, `"loot"`, `"weather"`), so a draw added for one leaves
   the others' replays as they were, and forks a child for a sub-task.
   Names are told apart by their 64-bit FNV-1a hash.
4. **No `rand` in a simulation.** A simulation never takes values from
   the `rand` crate, or any generator whose values a dependency's
   version decides.

## Consequences

- A game's replays survive an engine update unless a `Breaking:` line
  says otherwise.
- A better shuffle or a faster stream can only be added beside the old
  one, so a few frozen functions may stay forever.
- Helpers are built on `roll_below` alone, so a game's own
  `RandomSource` and `ScriptedRng` have them, and a test scripts the
  exact draws.

## Alternatives considered

- **The `rand` crate's helpers** (`gen_range`, `choose`, `shuffle`):
  their values may change in a minor release; and a new dependency for
  a few lines.
- **One stream for everything, as today**: every new draw moves every
  later one, so a cosmetic pick changes a battle's replay.
- **Streams told apart by an index a game hands out**: a game would
  keep a table of numbers; a name reads as its purpose, and its hash is
  as fixed.
