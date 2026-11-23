---
id: "2607"
title: "Parked: Saves sync through a store-neutral cloud layer with a conflict screen, and the web build asks to keep its storage"
type: feature
milestone: "Parked: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3527"]
nick_input: none
completed:
---

# 2607 — Saves sync through a store-neutral cloud layer with a conflict screen

> **Parked** (the networking direction stays parked in `docs/ROADMAP.md`): it may start when Nick unparks it, through the platform and service scope decision of 3527. Nobody starts it until then. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

A blind-spot audit of the engine (2026-11-23) found that cloud saves
exist only through Steam:

- 3037 syncs saves with Steam **Auto-Cloud** (no code: Steam syncs the
  save folder) and leaves a disagreement between two machines to
  Steam's own screen. Its `SteamStorage` writes each key through
  Steam's Cloud API, but with no conflict model of ours.
- 3036 keeps cloud saves out of its `StoreServices`: "they are
  `Storage`" (its lines 113 and 192). This ticket keeps that rule: the
  synced saves are a `Storage`.
- 3322 (GOG) leaves Galaxy's cloud saves out. Nothing covers Epic,
  a self-hosted sync, or the web build.
- The web build keeps saves in `localStorage`
  (`crates/app/src/storage/web.rs`, `crates/cli/web/storage.js`).
  A browser may clear a site's storage when space runs low, unless the
  page asked to keep it with `navigator.storage.persist()`. A search of
  the repository for `navigator.storage`, `persist(` and "persistent
  storage" finds nothing.

What a game has today: `Storage` (`crates/ui/src/storage.rs:27`, keys
`^[a-z0-9_-]{1,64}$`, text values), handed in by
`Platform::storage()` (`crates/platform/src/platform.rs:354`). The
native `FileStorage` keeps a last good copy of each key (3045, done).
Save bundles (`crates/ui/src/storage/bundle.rs`) already reserve a key
prefix (`RESERVED_PREFIX`, `"_bundle-"`) for their own bookkeeping, and
read a save's `Made` stamp to say how it differs from this build. A save
pulled from the cloud is loaded like any other, so a save written by a
newer build is refused by the game's normal load (2110's refusal), not
here.

What other engines do: Unity and Godot leave cloud saves to each
store's plugin. A game made with this engine should not have to write
the same conflict logic once per store.

This ticket adds one layer under every store: a `Storage` that keeps a
local copy and a remote copy in step, finds a conflict without a clock,
and asks the player through a ready-made screen. Each store adds only a
small backend.

## Nick input

None. (Whether cloud saves are wanted at all is 3527's question.)

## Scope

**In:**
- **`CloudSaves`**, a trait in `shuyi-ui` beside `Storage`
  (`crates/ui/src/storage/cloud.rs`): `list` (each key with the
  backend's `Revision`, an opaque string the backend gives, and its
  `Stamp`: the time and the machine name the backend reports, as text
  for the screen, never compared), `read`, `write` (returns the new
  `Revision`), `delete`, and `available`. Errors are `StorageError`.
  Calls are synchronous: a backend that talks to a server keeps its own
  cache, as Steam's Cloud API does. `Revision` and `Stamp` are
  `#[non_exhaustive]` with constructors (ADR-0039).
- **`SyncedStorage`**, a `Storage` over a local `Storage` and a
  `Box<dyn CloudSaves>`:
  - it keeps a ledger in the local storage, under a reserved prefix
    (`"_sync-"`, refused as a game key, left out of save bundles as
    `"_bundle-"` is): for each key, the hash of the value last synced
    and the remote `Revision` it came from;
  - `sync()` compares each key three ways (local, remote, ledger). Only
    one side changed: copy it over. Both changed to different values: a
    `Conflict`. Both changed to the same value: update the ledger. A
    key deleted on one side and unchanged on the other is deleted. No
    clock is read (ADR-0002): the ledger decides;
  - reads and writes go to the local storage at once; a write marks the
    key to push at the next `sync()`;
  - a `Conflict` holds both values and both stamps. Until it is
    resolved, the remote value waits under `_sync-theirs-<hash>` (a
    valid key whatever the game's key length), and neither side is
    overwritten;
  - the game picks a `ConflictPolicy`: `Ask` (the default), `KeepLocal`
    or `KeepRemote`. `resolve(key, Choice::Local | Choice::Remote)`
    settles one;
  - with the backend not `available` (offline, signed out), it acts as
    the local storage and syncs later. The log says why.
- **When it syncs:** `shuyi-app` calls `sync()` once after start,
  before the game's `setup` reads storage, and after each save is
  written, at most once a second (the platform's clock, never the
  game's). Before quitting, a last `sync()` runs.
- **Handing it in:** a game opts in with an `AppConfig` field
  (`cloud_saves: Option<...>`, left `None` by `AppConfig::new`), so
  nothing changes for a game that doesn't. `Platform::storage()` then
  returns the `SyncedStorage`, and `Ctx` gives the list of open
  conflicts.
- **The "which save?" screen** in `crates/ui/src/screens/` (beside
  `profiles.rs`): one row per conflict with each side's stamp and the
  game's summary of each value (a `Fn(&str) -> Vec<String>` the game
  gives; by default the save's `Made` stamp if it parses, else its
  length), and *Keep this machine's* / *Keep the cloud's*. Its text goes
  through the string table, and it is driven by `UiAction`s, never
  keys (`input-actions` skill).
- **Reference backends:**
  - `MemoryCloud`, for tests: two `SyncedStorage`s over one
    `MemoryCloud` act as two machines;
  - `FolderCloud` in `shuyi-app` (native): a folder the player chooses
    (for example one a sync tool already copies between machines),
    with the revision as the hash of the file.
- **Either order with 3037:** if 3037 is done, add `SteamCloudSaves`,
  implementing `CloudSaves` over its `steam` module's handle and
  `ISteamRemoteStorage` (the file's timestamp as the stamp), and have
  `SteamStorage`'s docs point a game that wants the conflict screen to
  `SyncedStorage` over it. If not, nothing to do here for Steam: 3037
  adds `SteamCloudSaves` over this layer.
- **The web build asks to keep its storage:** `storage.js` calls
  `navigator.storage.persist()` once, when the game's storage is first
  opened, if the browser has it. A new `shuyi_storage_persisted`
  function says whether the browser agreed (granted, refused, or not
  asked), and `WebStorage` logs it. The plugin's `VERSION` goes up by
  one in both files. A browser that refuses still plays. Do it wherever
  the web storage plugin lives when this starts.
- **Docs:** a *Cloud saves* section in the book's saving chapter: when
  to use Auto-Cloud (3037) and when `SyncedStorage`; how a conflict is
  found; and that the web build's storage may still be cleared by the
  player.

**Out (do not do):**
- GOG Galaxy, Epic Online Services and console backends: each a ticket
  of its own, when a game ships there and Nick has said yes to its SDK
  (3322 for GOG, 2920 for Epic's research);
- a self-hosted save service on our own server's accounts (1148's
  accounts; a ticket of its own after 1148);
- merging two saves' contents (the game's), and a save from a newer
  build (2110 refuses it on load);
- Steam Auto-Cloud's setup, and `SteamStorage` itself (3037);
- achievements and stats (3036);
- syncing settings or key bindings apart from saves: the game chooses
  which keys sync with a key filter, as save bundles do.

**Estimated size:** about 1,400 lines (code 650, tests 550, docs 200).

**Split:** past 1,000 lines, split off the "which save?" screen (the
screen and its string-table entries) into a new parked ticket blocked by
this one; `ConflictPolicy::Ask` then leaves conflicts open in `Ctx` for
the game's own screen.

## Implementation steps

1. Read `crates/ui/src/storage.rs`, `storage/bundle.rs` (the reserved
   prefix and `Made::check`), `crates/app/src/storage/` (`mod.rs`,
   `native.rs`, `web.rs`), `crates/cli/web/storage.js`, 3036's ADR and,
   if done, 3037.
2. Write `CloudSaves`, `Revision`, `Stamp` and `MemoryCloud` in
   `crates/ui/src/storage/cloud.rs`.
3. Write `SyncedStorage`: the ledger under `"_sync-"`, the three-way
   compare, `Conflict`, `ConflictPolicy` and `resolve`. Reserve the
   prefix in `is_valid_key`'s callers as bundles reserve theirs, and
   leave it out of `write_bundle`.
4. Wire it in `shuyi-app`: the `AppConfig` field, the sync at start,
   after writes (at most once a second) and before quitting, and
   `FolderCloud`.
5. Write the "which save?" screen and its strings, and the open
   conflicts on `Ctx`.
6. If 3037 is done, write `SteamCloudSaves` with 3037's fake.
7. Add the `persist()` request to `storage.js` and
   `shuyi_storage_persisted` to `web.rs`, with the version bump.
8. Write the docs and run the gates.

## Acceptance criteria

- [ ] Two `SyncedStorage`s over one `MemoryCloud`: a save written on
      the first appears on the second after both `sync()`.
- [ ] The same key changed to different values on both: `sync()` gives
      one `Conflict` with both values and stamps, and neither side is
      overwritten before `resolve`.
- [ ] `resolve(key, Choice::Remote)` writes the remote value locally,
      removes the waiting `_sync-theirs-` key and updates the ledger;
      `Choice::Local` pushes the local value.
- [ ] The same key changed to the same value on both gives no conflict.
- [ ] A key deleted on one machine and unchanged on the other is
      deleted on both; deleted on one and changed on the other is a
      `Conflict`.
- [ ] With the backend not available, reads and writes work locally,
      and the next `sync()` with it available pushes them.
- [ ] `KeepLocal` and `KeepRemote` settle conflicts with no screen.
- [ ] A game key starting `_sync-` is refused, and `write_bundle`
      leaves `_sync-` keys out.
- [ ] The core reads no clock: the sync's result depends only on the
      three sides' values and revisions.
- [ ] The "which save?" screen lists each conflict and its two choices,
      driven by `UiAction`s; a snapshot of its scene.
- [ ] With `cloud_saves: None`, a game's storage is exactly as before.
- [ ] The web build calls `navigator.storage.persist()` once and logs
      the answer; the test that compares `storage.js`'s functions with
      `web.rs`'s list passes.
- [ ] If 3037 is done: with its fake, a Steam file's timestamp becomes
      the conflict's stamp.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the three-way compare for each case (one side changed, both
  changed apart, both changed alike, deleted on one side, deleted on one
  and changed on the other); the ledger surviving a restart; the
  reserved prefix; each policy; the backend unavailable then back.
- Property: for random sequences of writes and deletes on two machines
  with syncs between, after a final sync on both with every conflict
  resolved, both local storages hold the same keys and values.
- Snapshot / integration: the "which save?" screen's scene; two
  `FolderCloud` machines over one temporary folder; 3045's power-cut
  suite over a `SyncedStorage` (its ledger keys included).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
