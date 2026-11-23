---
id: "2147"
title: The engine finds a game's mods, orders and checks them, and stamps the enabled ones into records
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["game#1014"]
nick_input: none
completed:
---

# 2147 — The engine finds, orders and checks a game's mods, and stamps them into records

## Context

Found by the blind-spot audit of 2026-11-23 (Scripting, hot reload and
modding: "the engine has no mod framework"). No friction review: a
missing feature.

- **What the engine has.** A mod is one more bundle layer (ADR-0004 §1,
  done ticket 2207). `Layer::owned` (`crates/assets/src/bundle.rs:402`)
  takes files handed in at run time. `shuyi_app::read_folder`
  (`crates/app/src/folder.rs:81`, native only) reads a folder into one,
  with a size limit per file. `shuyi_app::storage::data_dir`
  (`crates/app/src/storage/native.rs:133`) is the player's data folder.
  A layer laid over the game's changes `Bundle::fingerprint`, so a
  record made with a mod already fails `Made::check` on its content
  (ADR-0051 §3).
- **What it leaves to each game.** 2207's Out says where a game's mods
  live, their manifest, order and checks are the game's. 2228's Out says
  the same ("Mods' folders, manifests and load order (game#1014, over
  2207)"). So every game that wants mods writes the same manifest, the
  same order and the same problem list. And a record says only that the
  content differs, never which mods it was made with.
- **The game writes it first:** game#1014
  ([`1014-mods-folder-manifest-and-load-order.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1014-mods-folder-manifest-and-load-order.md)),
  under the rules of game#1011's ADR
  ([`1011-adr-how-mods-work.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1011-adr-how-mods-work.md)).
  It plans a `ModManifest` (id, name, version, author, `game_version`,
  `requires`, `conflicts`) and `parse_manifest` in `trpg-content`, a pure
  `resolve_order(manifests, enabled_ids) -> (Vec<id>, Vec<ModProblem>)`,
  and an `app` file that scans `mods/` beside the executable and the
  player's data folder, reads the enabled list from storage key `mods`,
  and reads each mod with `read_folder`. game#1015 then records the mods
  in the game's saves and shows a Mods screen. This ticket moves the
  generic half of 1014 into the engine once 1014 is done, as 2207 did
  for the bundle.
- **The game today** (pin `50472bbcd3084555a61728a5d07518f720bd6514`):
  `gh search code --repo Zafnok/visions-of-shuyi 'ModManifest'` finds
  only `tickets/open/1014-…`; no code yet. `read_folder` is called by
  the game's hot reload (`crates/ui/src/hot.rs`, `crates/app/src/main.rs`),
  not by mods. game#1011, 1012, 1014 and 1015 are all open.
- **The generic shape:** a game names where its mods may be and how its
  build is named. The engine finds the mod folders, reads a manifest from
  each, puts the enabled ones in an order that respects what each needs,
  refuses what can't run, says which mods overlap on a file, and returns
  layers to lay over the game's. The stamp of a save or record lists the
  mods it was made with, so a mismatch names them.
- **Outside tactics:** a farming game whose mods each add crops (2228's
  example), a platformer whose players share level packs, a roguelike
  with a balance mod that needs a content mod.
- ADR-0051 (stamps), ADR-0007 (formats are plugins), ADR-0039 (growable
  types), ADR-0032 (layers).

## Nick input

None.

## Scope

**In:**
- `shuyi_assets::mods` (pure, no I/O): `ModManifest`
  (`#[non_exhaustive]`, built by parsing; fields: `id`, `name`,
  `version`, `author`, `game` (the game's word for the build it was
  made for), `requires: Vec<String>`, `conflicts: Vec<String>`), read
  from a mod's `mod.ron` through a RON `Format` of a new kind
  (ADR-0007). `resolve(found, enabled, rules) -> Resolved`: the enabled
  mods in load order (each after the mods it requires, otherwise the
  player's order), and a `Vec<ModProblem>` (`#[non_exhaustive]` enum).
  `Resolved::overlaps`: each path two enabled mods both hold, with both
  mods' ids (reported, not refused: the later one wins, as layers do).
- `ModRules` (`#[non_exhaustive]`, `ModRules::new(game)`): the running
  game's word and a `compatible: fn(made_for: &str, running: &str) -> bool`,
  exact equality by default.
- `shuyi_app::mods` (native only, beside `folder.rs`): `ModFolders`
  (`#[non_exhaustive]`; `ModFolders::standard(storage_name)` is `mods/`
  beside the executable then `data_dir(storage_name)/mods`; a game may
  name others), `find(folders) -> Found` (each folder holding a
  `mod.ron`, its manifest or why it didn't read), `enabled(storage, key)`
  (a RON list of ids at a key the game names; none when it is missing),
  and `load(found, enabled, rules, max_file_bytes) -> Loaded`: the
  layers in load order (each `Layer::owned` named `mods/<id>`, so a
  path shows as `mods/<id>/data/items.ron`), the problems and the
  overlaps.
- A problem disables only that mod for this launch: a manifest that
  doesn't parse, two mods with one id, a required mod missing or
  disabled, a cycle of requirements (every mod in it named), a conflict
  between two enabled mods (both named; the later one is disabled), a
  mod made for another game build, an enabled id not found, a file
  `read_folder` refused.
- `shuyi_core::made`: `ModStamp { id, version }` (`#[non_exhaustive]`,
  `ModStamp::new`), and `Made` gains `mods: Vec<ModStamp>`, with
  `#[serde(default, skip_serializing_if = "Vec::is_empty")]`, so a stamp
  with no mods is written exactly as today. `Made::with_mods(stamps)`
  sets them, and `Loaded::stamps()` gives them in load order.
  `Made::check` compares them as a new `Part::Mods`, and its `Mismatch`
  names the mods added, removed or at another version.
- **The API:** `Made` is `#[non_exhaustive]` already, so its field is
  additive. `made::Part` is not `#[non_exhaustive]` today
  (`crates/core/src/made.rs:169`), so `Part::Mods` is a break: mark
  `Part` `#[non_exhaustive]` in the same change, and write one
  `Breaking:` line with a `migration` block. The game's PR that moves
  the pin follows this one, and replaces its own manifest, order and
  scan from game#1014 with these.
- A row for `mods/` in `tools/layers.ron` only if a new crate is made
  (none is planned: the pure half sits in `shuyi-assets`, which has
  `serde` and `ron`; the I/O half in `shuyi-app`).
- The docs: a section in the book's `assets-and-formats` chapter, the
  manifest's row in `docs/formats.md`, and the module docs.

**Out (do not do):**
- Merging data tables entry by entry: 2228's `Bundle::versions`, and the
  merge itself is each game's (game#1011, game#1013).
- A Mods screen to turn mods on and off and order them: the game's
  (game#1015). A generic screen is a later ticket, once a second game
  wants one.
- Saves: a game's save holds a `made: Made` (the module docs of
  `shuyi_core::made`), so it gets the mods through `Made`. What a game
  does with a mismatch (play on, warn, refuse) stays the game's.
- Zips, the web build (no folders; it starts unmodded), downloading
  mods, Steam Workshop or mod.io (the game's game#1024; store backends
  are 3036 and 3037, which leave the Workshop to the game).
- Mods that bring scripts: a script is just a file of the mod's layer
  here. Making scripts an asset kind is 2145's, and nothing passes
  between the two.
- The engine version a mod was made for: a mod is data over a game's
  build, so the game's word stands for it.
- Fuzzing the manifest: 2020 fuzzes every registered format, so the new
  kind is covered when 2020 runs; nothing to hand over.
- "Modules" in the sense of 2526 and 2552 (outside crates a game adds):
  another thing.

**Estimated size:** about 1,500 lines (code 700, tests 600, docs 200).

**Split:** past 900 lines, split off the stamp (step 5, `ModStamp`,
`Made::mods` and `Part::Mods`) into a new ticket.

## Implementation steps

1. Read game#1014 as merged (its `modding.rs` and `app/src/mods.rs`),
   game#1011's ADR, ADR-0004 §1, ADR-0051, `crates/assets/src/bundle.rs`
   (`Layer::owned`), `crates/app/src/folder.rs`,
   `crates/app/src/storage/native.rs` and `crates/core/src/made.rs`.
   Where the game's code and this ticket differ on a field or a rule,
   follow the game's ADR, and say so in the completion notes.
2. `crates/assets/src/mods.rs` (and `mods/tests.rs`): `ModManifest`, its
   kind and RON `Format` (registered with the engine's own formats),
   with `deny_unknown_fields` and an id checked as a storage key is
   (`^[a-z0-9_-]{1,64}$`), so `mods/<id>` is always a good layer name.
3. `resolve`: a stable order (a requirement first; otherwise the
   player's order; ties by id), the problems above, and `overlaps` from
   the manifests' paths (taken as an input list, so it stays pure).
4. `crates/app/src/mods.rs`: `ModFolders`, `find`, `enabled` and `load`
   over `read_folder`, behind the same `cfg` as `folder.rs`. The only
   file in the engine that knows the folder layout. Re-export from
   `crates/app/src/lib.rs` beside `read_folder`.
5. `crates/core/src/made.rs`: `ModStamp`, `Made::mods`,
   `Made::with_mods`, `Part::Mods` and `#[non_exhaustive]` on `Part`;
   `Mismatch`'s text for the mods part (`mods +more-crops 1.0,
   -old-ui 0.3`). Update the contract's schema that holds `Made`
   (`crates/cli/schemas/replay.output.json`).
6. Golden: a row `mod manifest: ron` in
   `crates/assets/tests/golden/formats.ron` over a fixture manifest, and
   check that `crates/core/tests/golden.rs`'s stamp is unchanged.
7. Test mods under `crates/app/tests/mods/` with a test file
   `crates/app/tests/mods.rs` (one file per test, as `log_sink.rs` and
   `crash_hook.rs` are there): a working one, one with
   a bad manifest, one that needs a missing mod, two that overlap on a
   file.
8. The docs (Scope); the `Breaking:` line and `migration` block in the
   completion notes.

## Acceptance criteria

- [ ] `resolve` puts a mod after every mod it requires, and keeps the
      player's order otherwise (unit tests).
- [ ] A cycle of requirements disables every mod in it, each problem
      naming the cycle (unit test).
- [ ] Two enabled mods that conflict: the later one is disabled, the
      problem names both (unit test).
- [ ] A mod made for another game build is disabled with the default
      rules, and loads with a `compatible` that accepts it (unit test).
- [ ] A bad manifest disables only that mod; the others load (test over
      the test mods).
- [ ] With a test mod enabled that changes a file, the bundle reads the
      mod's copy, and `display_path` shows `mods/<id>/…` (integration
      test).
- [ ] Two mods holding one path are listed in `overlaps` with both ids,
      and the later one's file is read (test).
- [ ] With no key in storage, `load` gives no layers and no problems,
      and the bundle's fingerprint is the unmodded one (test).
- [ ] A `Made` with no mods serializes byte for byte as before (the
      golden stamp is unchanged), and an old stamp without `mods` reads
      (test).
- [ ] `Made::check` between stamps with different mods gives a
      `Part::Mods` difference whose text names each mod added, removed
      or changed (unit test).
- [ ] The manifest format has its golden row, and the web build
      compiles (the app half is native only).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: manifest parsing and its errors; `resolve` (order, cycles,
  conflicts, missing, duplicates, versions); `Made` with mods (serde,
  `check`, the text).
- Property: `resolve` gives the same order for the same input whatever
  order the folders were found in, and every mod in the order comes
  after the mods it requires.
- Snapshot / integration: the test mods folder end to end; the golden
  row; the schema snapshot.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
