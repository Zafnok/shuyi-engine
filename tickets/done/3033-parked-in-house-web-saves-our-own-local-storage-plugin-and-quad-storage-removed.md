---
id: "3033"
title: "In-house web saves: our own localStorage plugin, and quad-storage removed"
type: feature
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-12
---

# 3033 — In-house web saves: our own localStorage plugin

> **Unparked** (Nick, 2026-11-12, "Unpark 3033 only", `docs/decisions.md`). It was parked on 2026-10-19 with the in-house platform layer, which stays parked.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): "the two small ones (web saves,
asset packing)" go in-house after controllers. The web build saves through
`quad-storage` 0.1.3 (last release August 2021), in
`crates/app/src/storage/web.rs`, with its JS vendored as
`crates/cli/web/quad-storage.js` and `sapp_jsutils.js` and their license
files. Today `quad-storage.js` calls `localStorage.setItem` with no `try`
(line 26), so a full or blocked store throws through the frame instead of
returning an error.

The pattern to follow is already in the repository: `crates/app/src/pads/web.rs`
declares functions our own `crates/cli/web/gamepad.js` provides, as a
miniquad plugin with a version the loader checks. Written from MDN's *Web
Storage API* (`localStorage`: `getItem`, `setItem`, `removeItem`, `key`,
`length`), never from quad-storage's source
([ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§2).

## Nick input

None.

## Scope

**In:**
- **`crates/cli/web/storage.js`**, our own, a miniquad plugin
  `shuyi_storage` with a `version`, giving `get`, `set`, `remove`, `len`
  and `key`. Strings cross as UTF-8 bytes in the wasm memory: Rust passes a
  pointer and a length in; for reads, JS writes into a buffer Rust gave
  and returns the length, or a length larger than the buffer so Rust asks
  again with a bigger one.
- **`localStorage` that throws** (private windows, storage turned off, the
  quota full): every call catches it and returns an error code, and Rust
  turns it into a `StorageError`, so the game warns and plays on.
- **Writes stay synchronous:** `set` and `remove` call `localStorage`
  inside the call and return after it, never batched or deferred. 3043
  saves inside the browser's `visibilitychange` and `pagehide` events, and
  a deferred write is lost when the page closes.
- **`crates/app/src/storage/web.rs`** on the plugin, with the same
  `WebStorage` API and key prefix. The `extern` block is the only `unsafe`,
  as in `pads/web.rs`, under the same comment rules.
- **A test in `shuyi-cli`** that `storage.js`'s version equals the Rust
  constant, as the gamepad plugin has.
- **`cargo shuyi web`** copies `storage.js` (the file list in
  `crates/cli/src/web.rs`), `index.html` loads it, and
  `crates/cli/web/README.md` describes it in place of the two vendored
  files.
- **Removed**: `quad-storage` from `crates/app/Cargo.toml`;
  `crates/cli/web/quad-storage.js` and its license file; and
  `sapp_jsutils.js` and its license file, if nothing else loads them
  (search `index.html` and macroquad's bundle first, and keep them if
  something does, saying so in the completion notes).
- **From 3039 (done)** (the web build's notices): `storage.js` gets a
  row in `SHELL_NOTICES` (`web.rs`'s shell table) as the engine's own,
  and the rows of the vendored files removed above go too, with the
  `include_str!` of their license files and their lines in
  `THIRD_PARTY_ASSETS.md` (a test keeps the table and that file
  together).
- **Saves keep their keys**, so a player's saves from before still load.

**Out (do not do):**
- IndexedDB, or saves larger than `localStorage` allows: write a ticket
  if a game's saves outgrow it.
- Native storage (`storage/native.rs` is already our own; its writes
  surviving a power cut are 3045).

## Implementation steps

1. Write `storage.js`, then the `extern` block and `WebStorage` over it.
2. Run the storage conformance suite (`shuyi-ui`'s `storage-conformance`
   feature) against `WebStorage` in the browser build's tests, if those
   run in CI; if they don't, test the byte passing on the Rust side with a
   fake, and play the example in a browser.
3. Remove the old files and crate (scope above).
4. Save in the example on a build from `main`, then load the same save on
   this branch's build.
5. In the completion notes, list the documents used.

## Acceptance criteria

- [x] ~~Saving, loading, deleting and listing work in Chrome and Firefox.~~
      (not done: checked in Chromium only; Firefox not checked, see the
      completion notes)
- [x] With storage turned off in the browser, the game starts, and saving
      shows the game's warning instead of crashing.
- [x] A save made with quad-storage loads after this change.
- [x] `cargo tree --target wasm32-unknown-unknown` shows no
      `quad-storage`.
- [x] The PR body says the code was written from MDN's Web Storage
      documentation.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the grow-and-retry read with a fake plugin; error codes to
  `StorageError`.
- Integration: the version test; the conformance suite (step 2).
- Manual: steps 4 and the acceptance criteria's browsers.

## Completion notes

**Done.** Nick unparked this ticket alone on 2026-11-12
(`docs/decisions.md`); the rest of the in-house platform layer stays
parked.

- **`crates/cli/web/storage.js`**, the engine's own miniquad plugin
  `shuyi_storage` (version 1): `shuyi_storage_get`, `_set`, `_remove`,
  `_len` and `_key`. Strings cross as UTF-8 bytes (pointer and length in;
  for a read, a buffer Rust lends, and the length answered either way).
  Every call catches what `localStorage` throws and answers a code: `-1`
  absent, `-2` blocked (`SecurityError`, or no `localStorage`), `-3` full
  (`QuotaExceededError`), `-4` anything else. `set` and `remove` call
  `localStorage` inside the call.
- **`crates/app/src/storage/local_storage.rs`** (new): the rules over a
  `Plugin` trait: the key check, the prefix, the read that lends a bigger
  buffer when the answer is longer (at most three asks), and each code to
  its `StorageError::Backend` message. Built on wasm32 and in tests.
- **`crates/app/src/storage/web.rs`**: the `extern` block (its only
  `unsafe`, as in `pads/web.rs` and `sound-out`'s `sys.rs`), the version
  export, and `WebStorage` over it, with the same API and key prefix.
- **Removed:** `quad-storage` from `crates/app/Cargo.toml` (and
  `quad-storage-sys` and `nanoserde` from the locks, the ten tutorial
  steps' included), `crates/cli/web/quad-storage.js` and its license file,
  its `SHELL_NOTICES` and `notices::FILES` rows, its line in
  `THIRD_PARTY_ASSETS.md`, `_typos.toml` and the export check's list, and
  the two Sonar rules that excused it.
- **`sapp_jsutils.js` stays**, with its license file and rows:
  `player-files.js`, `page-files.js` and `pack-file.js` hand strings and
  bytes across as its objects, and `shuyi-app` depends on `sapp-jsutils`
  for them. Its rows now say so.
- **Documents used:** MDN's *Web Storage API*, *Window: localStorage
  property* and *Storage* (`getItem`, `setItem`, `removeItem`, `key`,
  `length`), and MDN's *TextEncoder* and *TextDecoder*. Not
  quad-storage's source: the one fact kept from the old code is where saves
  live (the key as given, `<name>/<key>`), from `storage/web.rs`'s own
  prefix.

**Tests.** Unit, in `local_storage.rs`, over a fake plugin: the
conformance suite (`storage-conformance`), the grow-and-retry read (once
for a value that fits, twice for one larger than the first buffer, and a
key in the list), giving up on a read that keeps growing, bytes that are
not UTF-8, each code to its error on all four calls, keys under the prefix
as before. `shuyi-cli`: the functions and the version of `storage.js` equal
`storage/web.rs`'s, its error codes equal `local_storage.rs`'s, and
`index.html` loads it between the bundle and the game. The browser build's
tests don't run in CI, so the conformance suite runs over the fake (step
2's other branch).

**Manual check** on `8f37e0c0` (`main`'s tip, this branch's base),
`examples/hello-grid`'s web build, in the desktop app's browser (Chromium),
`main`'s build and this branch's served from one origin:

- `main`'s build (quad-storage) saved `p1-settings` with text speed Fast;
  this branch's build opened Options showing Fast. It then saved Instant
  over it, and the startup smoke check (write, read, delete) passed. The
  loader logged no version mismatch.
- Through the plugin from the console: `len` and `key` list the two keys;
  `key` into a 4-byte buffer answers the full length and writes nothing;
  a key and value beyond ASCII round-trip; `remove` then `get` answers
  `-1`; an 80 MB `set` answers `-3` where `setItem` itself throws
  `QuotaExceededError`.
- A copy of the page whose `localStorage` getter throws `SecurityError`:
  the game starts, each read logs "storage error: localStorage is turned
  off or blocked in this browser" and uses the defaults, and changing an
  option shows the game's "Couldn't save: the change lasts until you quit".
- **Firefox was not checked:** this session had only a Chromium browser.
  The plugin uses only what MDN lists for Firefox too (`localStorage`,
  `TextEncoder`, `TextDecoder`), and maps Firefox's old quota name
  (`NS_ERROR_DOM_QUOTA_REACHED`) to full as well. No ticket: Nick can open
  the web build in Firefox, or the game's next web release check covers it.

**Found:** the smoke check's failure logs a literal `{e}` on the web
(macroquad's log macros don't fill in an argument captured in the
string), and so does the "can't open storage" line: ticket 3311.

**Game.** At the pin move past this change, the game's
`scripts/check-release-package.sh` expects `storage.js` in place of
`quad-storage.js`, and its `assets/data/credits.ron` (`quad_storage_js`)
and `THIRD_PARTY_ASSETS.md` drop quad-storage's entries. Found with
`gh search code --repo Zafnok/visions-of-shuyi` for `quad-storage`,
`quad_storage` and `WebStorage`: no game code calls `WebStorage` or
`quad_storage` directly.

Breaking: shuyi-cli: notices::FILES: holds three files, not four: quad-storage.js left the web shell.

```migration
since: 0.2.0
what: The web shell no longer vendors quad-storage.js; its row left the table of the engine's non-crate files.
old: "notices::FILES: [EngineFile; 4]"
new: "notices::FILES: [EngineFile; 3]"
before: |
  let files: [EngineFile; 4] = notices::FILES;
after: |
  let files: &[EngineFile] = &notices::FILES;
```

Behaviour: shuyi-app: on the web, a `localStorage` that throws (storage turned off, a sandboxed frame, the quota full) answers `StorageError::Backend` ("localStorage is turned off or blocked in this browser", "localStorage is full", "localStorage failed") instead of throwing through the frame.
Behaviour: shuyi-cli: the web build ships `storage.js` in place of `quad-storage.js`, and `NOTICES.txt` lists no quad-storage.
