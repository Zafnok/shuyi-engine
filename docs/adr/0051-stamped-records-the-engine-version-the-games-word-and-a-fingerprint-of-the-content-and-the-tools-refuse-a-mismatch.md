# ADR-0051: Stamped records: the engine's version, the game's word and a fingerprint of the content, and the tools refuse a mismatch

- **Status:** Accepted
- **Date:** 2026-10-27
- **Related tickets:** 2111 (this); 2116 (determinism fixtures, which
  pin the fingerprint); 2518 (session records) and 2520 (crash
  replays), which carry a stamp; 1119, 2706, 1140 and 2720 (the parked
  records and handshakes, which take one when unparked)
- **Extends:** ADR-0006 §2 (the handshake's hash of the loaded content),
  ADR-0007 §5 (content hashes in replays), ADR-0023 §1 (the pack's index
  holds the fingerprint of its sources)

## Context

- ADR-0006 §2 says a session's handshake checks the engine version and
  a hash of the loaded content; ADR-0007 §5 says decoders are
  deterministic because content hashes are part of replays. Nothing
  computed that hash: `Bundle` had no fingerprint.
- `shuyi replay` (ADR-0022) runs a game's record and compares the state
  hash at each step. A record made with other content or another engine
  ran anyway, and its difference read as a desync. An agent reading the
  answer (Nick's end goal, games made by agents, 2026-10-19) learned the
  wrong cause.
- A pack (ADR-0023) has a stamp, the game's word for the shapes of its
  own kinds, and refuses another. Saves have a version (game ADR-0039,
  `shuyi_core::save`) and nothing else.
- Others (2026-10-20 comparison): Unreal writes the engine version that
  saved each package and refuses one from a newer engine; Factorio
  refuses a replay from another version, and adding or removing mods
  breaks one (`wiki.factorio.com/Replay_system`). Bevy and Godot have
  no stamp of their own.

## Decision

### 1. A stamp: `shuyi_core::Made`

- `Made { engine, game, content }`: the engine's version
  (`made::ENGINE_VERSION`, the workspace's `CARGO_PKG_VERSION`), the
  game's word for its build (as a pack's stamp is: its name and version,
  say), and `ContentHash`, the fingerprint of the content it ran on.
  `Made::new(game, content)` fills the engine.
- `ContentHash(u64)` is shown and serialized as sixteen lowercase hex
  digits, as `StateHash` is shown, since JSON can't hold every `u64`.
- `Made::check(&now)` gives a `Mismatch` listing each part that differs
  with both values, shown as `made with engine 0.1.0, content …; this
  build: engine 0.2.0, content …`.
- `shuyi-core` gains no dependency; the stamp is `no_std` and serde.

### 2. The engine's version moves only at a release

`ENGINE_VERSION` is the crate version, which moves at a release, not at
each commit. A game pinned to a git revision that wants the revision in
its stamp puts it in `game`. Reading git in a build script was rejected
(*Alternatives*).

### 3. The fingerprint: `Bundle::fingerprint`

- Over the layers that ship, never the test layers, whatever the thread
  (`story_only` changes nothing): the test files never reach a player.
- A running value starts at FNV-1a's offset basis. The shipped layers
  are taken from the last read to the first (the base layer first); each
  file of a layer, in path order, is fed as its path's length (`u64`)
  and bytes, then its contents' length and bytes.
- The step is the word-wise FNV-1a step of the pack's checksum (ADR-0023
  §1), in one `pub(crate)` hasher (`shuyi_assets::fnv`) both call, so
  the checksum is unchanged. No integer's size depends on the platform.
- The fingerprint is the running value after the first layer, with no
  closing step, so a layer laid over others changes it as its files do.
- It reads the layers' files directly, so the bundle's watcher isn't
  told; it reads every shipped byte, so a game computes it once and
  keeps it in its `Made`.

### 4. A pack stands for its sources

- `Bake::bake` writes the source bundle's fingerprint into `PackInfo`
  (`content`), and `pack::VERSION` becomes 3 (refused, never migrated,
  as ADR-0023 §5 says).
- `Pack::into_layer` marks its layer `Layer::standing_for(content)`
  (a field `stands_for`). When the fold reaches such a layer the
  running value becomes its fingerprint instead of taking its files,
  and the layers under it no longer count: a pack holds every file of
  the bundle it was baked from.
- So a build that ships one pack has the fingerprint of the loose files
  it was baked from, and a loose layer over a pack (a mod) counts as it
  would over those files: a record made on the loose files plays on the
  pack as made with the same content.

### 5. The tools refuse a mismatch; a game decides for its own saves

- `GameTools::made` (default `None`) is this build's stamp. A game's
  `replay` passes on its record's stamp in `ReplayRun::made`.
- `shuyi replay` checks each record's stamp against the build's before
  anything is compared and refuses a mismatch with `SH3007`
  (`other-stamp`), naming the record and both stamps. `--any-stamp`
  (`ReplayInput::any_stamp`) runs it anyway, with a limit that a
  difference may come from the stamp. A record with no stamp, or a game
  that gives none, runs as before, with a limit saying so.
- `shuyi playtest` keeps the stamp in each line of its history.
- A game's own saves and records hold a `made: Made` (an
  `Option<Made>` with `#[serde(default)]` keeps older files readable)
  and call `Made::check` after reading. What a mismatch does in a game
  (play on, warn, refuse) is the game's: the engine refuses only in its
  tools, where a wrong answer misleads. `SaveHeader` stays as it is.
- The templates and Hello, Grid stamp their `bots` records with
  `made()`: their name and version and the fingerprint of their embedded
  files.

## Consequences

- A record from another patch says "made with other content" instead
  of showing a desync, and an agent reading the answer learns the cause.
- Every game with replays or online play computes the content hash the
  same way, in the engine; the lockstep handshakes (1140, 2720) take a
  `Made` when unparked.
- Packs must be baked again (version 3). `Layer`, `PackInfo`,
  `ReplayInput` and `ReplayRun` gain a field each, and `GameTools` a
  defaulted method: breaks a game notices only where it builds them by
  literal.
- The fingerprint reads every shipped byte once per run. A game with
  very large content pays that at start-up if it stamps eagerly; it may
  stamp lazily, when it first writes a record.
- Any byte changed changes every stamp: a typo fixed in a text file
  makes old records refused unless `--any-stamp`. That is the point: the
  content they ran on is gone.

## Alternatives considered

- **A cryptographic hash (SHA-256, BLAKE3).** A new dependency in
  `shuyi-assets`, and the fingerprint finds another build, not an
  attacker (ADR-0023's *Alternatives*).
- **Hash the files through `Bundle::bytes`.** Tells the watcher of
  every file, so a front end that logs loads would log the whole bundle.
- **Hash the merged view (each path once, as read).** Then a pack
  couldn't stand for its sources without holding the layering, and a
  layer laid over another that rewrites a file to the bytes under it
  would vanish from the stamp. The layer-by-layer fold is as cheap and
  makes a pack's stand-in exact.
- **A pack's own bytes as its fingerprint.** A record made on the loose
  files would then never play on the shipped pack.
- **The engine's git revision in `ENGINE_VERSION`.** A build script in
  `shuyi-core` reading git breaks builds from crates.io and from a
  tarball, and every commit would refuse every record; a game that
  wants it puts it in `game`.
- **Refusing a mismatched save in the engine.** A game's load screen
  knows what a save can survive (game ADR-0039); the engine gives the
  check, the game the policy.
- **Checking the stamp in `SaveHeader`.** A save of the build's version
  parses whole, so the game reads its `made` with the rest; a save of
  another version is refused by its version first.
