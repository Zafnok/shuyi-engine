---
id: "3033"
title: "Parked: In-house web saves: our own localStorage plugin, and quad-storage removed"
type: feature
milestone: "Engine: in-house platform"
model: sonnet-5
effort: medium
status: blocked
blocked_by: []
nick_input: none
completed:
---

# 3033 — In-house web saves: our own localStorage plugin

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title. In the roadmap's order it comes after controllers, but it needs nothing else, so it can start once unparked.

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

- [ ] Saving, loading, deleting and listing work in Chrome and Firefox.
- [ ] With storage turned off in the browser, the game starts, and saving
      shows the game's warning instead of crashing.
- [ ] A save made with quad-storage loads after this change.
- [ ] `cargo tree --target wasm32-unknown-unknown` shows no
      `quad-storage`.
- [ ] The PR body says the code was written from MDN's Web Storage
      documentation.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the grow-and-retry read with a fake plugin; error codes to
  `StorageError`.
- Integration: the version test; the conformance suite (step 2).
- Manual: steps 4 and the acceptance criteria's browsers.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
