---
id: "3259"
title: "The crash-safe key-file protocol written once: KeyFiles over a FileOps trait, used by the native and Playdate storage"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3259 — The crash-safe key-file protocol written once

## Context

The redundancy sweep of 2026-11-28 found that two backends write the
same storage protocol line for line:

- `crates/app/src/storage/native.rs` (about 52–305, `FileStorage`);
- `crates/playdate/src/storage.rs` (about 12–219, `SdkStorage`).

Each one has, for a key:

- the paths `<key>.ron`, `<key>.ron.tmp` and `<key>.ron.bak`;
- `is_good` and `check_key`;
- `good_backup`, falling back to a good `.bak`;
- the `.tmp` sweep;
- the write: back up the last good copy, write the `.tmp`, then rename;
- `delete`;
- `list`, counting a lone good `.bak`.

That is about 390 lines across the two. The real differences:

**native**
- makes the folder on every write (`create_dir_all`);
- syncs the folder after a rename and a delete;
- warns with `damage()` text;
- reads `io::ErrorKind::NotFound`.

**Playdate**
- probes with `is_file`;
- has a `replace()` fallback that removes, then renames, only after a
  rename error;
- filters names by `is_valid_key` in `list`;
- lists a missing subfolder as empty.

A third copy is planned: the parked DS backend (3024) writes its saves
as files "in the order `FileStorage` uses".

## Nick input

None.

## Scope

**In:**
- **`shuyi_ui::storage::KeyFiles<F: FileOps>`**, `no_std` with `alloc`.
  It implements `Storage`'s `read`, `write`, `delete` and `list` once.
  - `FileOps` has `read` (returning `None` when not found),
    `write_synced`, `rename`, `remove`, `names`, `make_folder`,
    `sync_folder` (a default no-op) and a `warn` hook.
  - A policy flag says whether a failed rename may fall back to
    remove-then-rename. It is on for Playdate and must never be on for
    native.
- **`FileStorage` and `SdkStorage` become `FileOps` adapters.**
  `FileStorage` keeps `data_dir`, `StdDisk` and `damage()` as its
  warning text. `SdkStorage` keeps its fallback and its key filter.
- **The protocol's unit tests are written once**, against a fake
  `FileOps`. The backends keep `conformance_suite` and
  `power_cut_suite`, including `Rename::Refuses`.
- **The API is additive:** a new public type and trait. The two storage
  types keep their names and constructors.
- **Behaviour:** none. The files on disk, the messages and the order of
  writes are the same.

**Out (do not do):**
- The GBA backend (3021): it writes to battery RAM or flash, not to
  files.
- Changing the file format or adding a key-name rule.
- Deleting `crates/app/src/storage/native/fake_disk.rs`: the power-cut
  tests use it.

**Estimated size:** about 800 lines changed (about 450 removed, 350
added).

## Implementation steps

1. Write `crates/ui/src/storage/key_files.rs`: `FileOps`, `KeyFiles`
   and the fake `FileOps` with the protocol's tests.
2. Move `FileStorage` onto it. `conformance_suite` and
   `power_cut_suite` pass.
3. Move `SdkStorage` onto it. Its three tests pass unchanged.
4. **Either order with 3024:** if 3024 is done, move its storage onto
   `KeyFiles` too. If not, 3024 uses `KeyFiles` (its storage bullet
   says so).

## Acceptance criteria

- [ ] `crates/app/src/storage/native.rs` and
      `crates/playdate/src/storage.rs` hold no copy of the backup,
      sweep or list rules (`grep -n "fn good_backup\|fn sweep"` finds
      them only in `key_files.rs`).
- [ ] Both backends pass `conformance_suite` and `power_cut_suite`,
      including `Rename::Refuses`.
- [ ] A save written before this ticket reads back the same after it,
      with a fixture folder of `.ron`, `.ron.bak` and a stray `.tmp`.
- [ ] The native storage never removes a file before a rename (a fake
      that records the order of operations).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `KeyFiles` over the fake. Read with a bad main file and a good
  backup, write after a crash between steps, list with a lone `.bak`,
  and both fallback settings.
- The backends' suites are the regression suite.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
