---
id: "3045"
title: "Saves that survive a power cut: flushed writes, the last good copy kept, and a power-cut suite every backend's storage runs"
type: bug
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-10-28
---

# 3045 — Saves that survive a power cut

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. The benchmarks are not ahead here. Bevy has no save
storage. Unity's `PlayerPrefs` promises nothing about power loss.
Unreal's `AsyncSaveGameToSlot` is about not blocking the game thread.
Godot's backup-then-rename is an editor setting for the editor's own
files (`filesystem/on_save/safe_save_on_backup_then_rename`,
<https://docs.godotengine.org/en/stable/classes/class_editorsettings.html>),
not a promise for a game's saves. What the comparison found is a bug in
our own save path, so this ticket is a bug.

What Shuyi has today:

- `crates/app/src/storage/native.rs`, `FileStorage::write` (lines
  80–87): `fs::write` to `<key>.ron.tmp`, then `fs::rename` onto
  `<key>.ron`. It never calls `sync_all` on the file or the folder, and
  it keeps no previous copy. Its doc comment (lines 13–16) says "a crash
  mid-write never corrupts the previous save". That holds when the
  process crashes. It does not hold when the power goes or the system
  crashes:
  - on NTFS the new name can reach the disk before the data, which
    leaves `<key>.ron` empty or full of zero bytes, with nothing to fall
    back to;
  - on ext4 (the Steam Deck's home folder) `auto_da_alloc` flushes the
    data when a rename replaces a file, but a first save replaces
    nothing, and other file systems give no such help.
- `FileStorage::read` (lines 71–78) returns whatever is there: an empty
  or zeroed file reaches the caller, and `Ctx::load_settings`
  (`crates/ui/src/ctx.rs`, line 743) and `profiles::load`
  (`crates/ui/src/profiles.rs`, line 292) then go to the defaults or
  rebuild the profile list, with a warning.
- `shuyi_ui::storage::conformance_suite` (`crates/ui/src/storage.rs`,
  lines 120–169) checks what each operation does. Nothing in it cuts
  the power.
- 3010 (the Playdate) and parked 3024 (the DS) ask that "a save
  survives a simulated crash mid-write", and parked 3021 (the GBA) that
  it survives "a simulated power cut mid-write". Each tests it with a
  fake of its own, and nothing shared says what "survives" means.
- The web build: `crates/cli/web/quad-storage.js` (line 26) calls
  `localStorage.setItem` with no `try`, so a full or blocked store
  throws through the frame. Parked 3033 already makes every
  `localStorage` call return a `StorageError` instead. Not this ticket.

Why it matters: the game is sold on Steam and plays on the Steam Deck
(game#0903, 3012), and it saves through `FileStorage`. A Deck whose
battery runs out, or that a player turns off by holding the power
button, right after a save can lose that save. Losing a save is the
worst bug a player meets. Every other game on the engine saves through
the same code.

## Nick input

None.

## Scope

**In:**
- **`FileStorage::write`, in this order**
  (`crates/app/src/storage/native.rs`):
  1. on the first write of this `FileStorage`, remove the `*.ron.tmp`
     files a crash left in the folder;
  2. write the new value to `<key>.ron.tmp` and `sync_all` it (on macOS
     `std`'s `sync_all` already uses `F_FULLFSYNC`);
  3. if `<key>.ron` is there and is a good copy (below), write its bytes
     to `<key>.ron.bak` and `sync_all` that. If the new value is not a
     good copy itself (`""`, or only `\0` characters), remove
     `<key>.ron.bak` instead, so that value reads back as written and
     not as damage;
  4. rename `<key>.ron.tmp` onto `<key>.ron`;
  5. sync the folder (`Disk::sync_dir`), so the rename itself survives
     the power. `write` always calls it; the platform split lives in
     `StdDisk::sync_dir`:
     - on Unix (Linux, the Steam Deck, macOS) it opens the folder and
       calls `sync_all` on it. If that fails with
       `ErrorKind::Unsupported` or `InvalidInput` (a file system that
       doesn't sync folders), it returns `Ok`; any other error is
       returned, and the write fails with it;
     - on Windows it does nothing: `std` opens no folder for a sync.
       NTFS journals the rename, so a power cut in the moments after a
       write leaves the new value or the one before it, never damage.
       The doc comment says so.

  The main file is only ever replaced by a rename, after the backup is
  written. So once a key holds a good value, at no moment is there
  neither a good main file nor a good backup. No hard links: a copy
  works on every file system, FAT and exFAT too.
- **A good copy** is a file that is not empty, not all zero bytes, and
  valid UTF-8. One function, `is_good(&[u8]) -> bool`, used by `write`
  and `read`.
- **`FileStorage::read`:**
  - a good main file: its text, as today;
  - a main file that is missing, empty, all zero bytes or not UTF-8,
    beside a good `<key>.ron.bak`: the backup's text, and a warning
    naming the key and why (for example "save-slot-1: the file is all
    zero bytes, read its backup instead");
  - otherwise, exactly what it returns today: `None` for no file,
    `Some("")` for an empty one, the zero bytes as text, the
    `StorageError::Backend` for text that isn't UTF-8.
- **`FileStorage::delete`:** removes `<key>.ron.bak`, then `<key>.ron`,
  then syncs the folder (`Disk::sync_dir`). So a crash in between leaves
  the save, never a backup that brings a deleted save back.
- **`FileStorage::list`:** still names keys only, never a `.bak` or
  `.tmp` file. It also names a key whose main file is gone while its
  `.bak` is good, since `read` returns that key.
- **Warnings** go through a `warn: fn(&str)` field of `FileStorage`,
  set by `FileStorage::new`, so a test can set its own. Mark each
  message `// check-text: not player text (a warning for the log)`, as
  `ctx.rs` does. **Either order with 3041:** if 3041 is done, the
  function logs with `log::warn!(target: "storage", …)`; if not, it uses
  macroquad's `warn!`, as `storage::platform`
  (`crates/app/src/storage/mod.rs`, line 31) uses `error!`, and 3041
  moves it with the backend's other log calls.
- **A seam for the tests.** `FileStorage`'s file calls go through a
  private trait `Disk: fmt::Debug` in `native.rs`: `read`,
  `write_synced` (create, write, `sync_all`), `rename`, `remove`,
  `sync_dir`, `names` (the file names in a folder) and `create_dir_all`.
  `StdDisk` does them with `std::fs`. `FileStorage` holds a
  `Box<dyn Disk>`, the `warn` function and whether it has swept yet; a
  private `FileStorage::at(dir, disk, warn)` builds it, for `new` and
  the tests. `FileStorage`'s public API (`new`, `data_dir`) doesn't
  change. No new dependency.
- **`shuyi_ui::storage::power_cut_suite`**, in a new
  `crates/ui/src/storage/power_cut.rs`, behind the `storage-conformance`
  feature as `conformance_suite` is (`#[cfg(any(test, feature =
  "storage-conformance"))] mod power_cut;` in `storage.rs`, and
  `pub use` of the trait and the suite), `no_std` with `alloc`:
  - `trait PowerCutRig`, which a backend writes over a fake of its
    medium:
    - `fn storage(&mut self) -> &mut dyn Storage`: the storage as the
      game sees it;
    - `fn cut_after(&mut self, ops: u32)`: the power goes after `ops`
      more operations on the medium (a file written, renamed or
      removed, a folder synced; a flash sector erased), and every
      operation after that fails;
    - `fn restart(&mut self)`: the power comes back. The medium keeps
      only what it had made durable, the storage is opened again over
      it, as at the game's next start, and no cut is pending.
  - `fn power_cut_suite(rig: &mut dyn PowerCutRig)`. It runs three
    operations on one key: a write over an old value, a first write,
    and a delete. For each, for `ops` from 0 up, until the operation
    returns `Ok`, it sets the key back with the power on (the old
    value, or nothing), restarts, cuts after `ops`, runs the operation,
    restarts again and checks:
    1. a write over an old value reads the old value or the new one:
       never `None`, a mix or an error;
    2. a first write (no old value) reads `None` or the new value;
    3. a delete reads the old value or `None`;
    4. a write or delete that returned `Ok` reads its result after the
       restart (the restart is a power cut too);
    5. another key, written once at the start, reads the same after
       every restart;
    6. `list()` names exactly the keys that read `Some`;
    7. with the power on, a write after the cut works and reads back.
  - The new value is longer than the old one (a torn write shows as a
    prefix). A write or delete that needs more than 10 000 operations
    of the medium fails the suite. Each failure panics with the
    promise, the step and `ops`.
- **`FileStorage` passes it** over a fake `Disk`, with two kinds of cut:
  a process crash (what was done stays) and a power cut (the folder goes
  back to how it was at its last `sync_dir`).
- **Docs:**
  - the doc comment of `FileStorage` says what the code promises on each
    OS, and no more, which files the folder holds (`.ron`, `.ron.bak`,
    a `.ron.tmp` between a crash and the next write), and that a sync
    of the folder (Steam Auto-Cloud) carries the `.bak` files with the
    `.ron` files;
  - `docs/platform-backends.md`, the section on
    `storage(&mut self) -> Box<dyn Storage>`: the promises above, and
    that a backend proves them with `power_cut_suite` over a fake of its
    medium; its *Checklist for a new backend* gains "Its storage passes
    the power-cut suite over a fake of the device's medium." under "Its
    storage passes the conformance suite."
- **For the game:** no API change. Its save folder gains a
  `<key>.ron.bak` beside each save; the PR says so for the game's pin
  bump. A Steam Auto-Cloud setup (game#0903, 3037) syncs the backups
  with the saves (`*.ron` and `*.ron.bak`): if it synced `*.ron` only, a
  save deleted on one machine would come back from its backup on
  another, since `read` falls back to a good backup when the main file
  is missing.
- **Either order with 2767:** if 2767 is done, nothing to do: its
  `Meta` is saved through `Storage`, so the new `write` flushes it like
  any other key; if not, nothing to do either (2767 saves through the
  same `Storage`).
- **Size:** if the PR passes about 600 changed lines, the fake disk and
  `FileStorage`'s power-cut runs (steps 5 and 6, and their criteria) go
  to a follow-up ticket (the `write-ticket` skill, After MVP, blocked by
  this one). This PR keeps the suite with its in-memory rigs, and the
  new `write`, `read`, `delete` and `list` with their real-file tests.

**Out (do not do):**
- the web build's `localStorage` errors: parked 3033 catches them. If
  the game's web build ships before 3033 is unparked, raise it with Nick
  (the `ask-nick` skill) rather than copy 3033 here;
- falling back to the backup when the text is whole but doesn't parse
  (a `read_previous` on `Storage` for `Ctx::load_settings`,
  `profiles::load` or a game's own saves): `Storage` stores text and
  never parses it. A follow-up ticket if a game asks;
- checksums or any change inside the files: saves stay plain RON a
  player can open;
- cloud saves (3037), saved-format migrations (2110), version stamps on
  saves (2111);
- saving off the main thread, IndexedDB;
- the backend's log lines on the `log` crate (3041);
- the handheld backends' storage code (3010, 3021, 3024): each runs
  this suite over its own fake when it lands.

## Implementation steps

1. Read `native.rs`, `storage.rs`, `Ctx::load_settings`,
   `profiles::load` and the `storage` section of
   `docs/platform-backends.md`.
2. Write `power_cut.rs`: the trait, the suite, and its own tests with
   one in-memory rig (a struct with a mode, to keep it short) in three
   modes: atomic (a write is one operation), in place (a write empties
   the value, then sets it: two operations) and forgetful (a write
   returns `Ok` but the restart loses it).
3. Put `FileStorage`'s file calls behind `Disk` and `StdDisk`, with no
   change in behaviour. The existing tests change only where they build
   `FileStorage { dir }`, which becomes `FileStorage::at`; they pass.
4. Change `write`, `read`, `delete` and `list` as above, with `is_good`
   and the `warn` field.
5. Write the fake disk in `crates/app/src/storage/native/fake_disk.rs`
   (`#[cfg(test)] mod fake_disk;` in `native.rs`), shared by the rig and
   the storage through an `Rc<RefCell<…>>`:
   - a folder maps names to inodes. `write_synced` to a new name makes
     a new inode with the bytes (one operation). To a name that exists
     it empties that inode (one operation) and then fills it (a
     second), as `File::create` truncates in place. Both are durable as
     they happen;
   - `rename` and `remove` change the folder's names only;
   - `sync_dir` copies the names to the durable folder;
   - every call counts one operation, and calls at or after the cut
     return an `io::Error` of kind `Other`;
   - `restart` with a power cut puts back the durable folder, with a
     process crash keeps the names, then makes a new `FileStorage` over
     the disk (so its first write sweeps again).
6. Run `power_cut_suite` with both kinds of cut. Then, once, take the
   folder sync out of `write` and check that the power-cut run fails;
   put it back and say so in the PR.
7. Write the docs.
8. Time 100 writes of a 10 KB value before and after the change, on the
   machine at hand, with a throwaway loop over a temporary folder that
   is not committed. Put the numbers in the PR description. If one
   write now takes more than 16 ms (a frame at 60 Hz), write a ticket
   for saving off the main thread (the `write-ticket` skill) and name
   it in the PR.
9. Run the gates.

## Acceptance criteria

- [x] `power_cut_suite` passes for the atomic in-memory rig and panics
      for the in-place and the forgetful ones.
- [x] `FileStorage` passes `conformance_suite`, and `power_cut_suite`
      over the fake disk with process-crash cuts and with power cuts.
- [x] Without the folder sync in `write`, the power-cut run fails (said
      in the PR).
- [x] With a good `<key>.ron.bak` beside it, a `<key>.ron` that is
      missing, empty, all zero bytes or not UTF-8 reads the backup, and
      the warning names the key and the reason.
- [x] With no backup, those files read as they do today.
- [x] Writing `""`, or `"\0\0"`, removes the backup and reads back what
      was written.
- [x] `list()` never names a `.bak` or `.tmp` file, and names a key
      whose `.ron` is missing beside a good `.bak`; `delete` removes the
      backup too; a `.tmp` file a crash left is ignored by `read` and
      `list` and is gone after the next write.
- [x] The `FileStorage` doc comment and `docs/platform-backends.md`
      state the promises (the doc comment per OS, with the `.bak` files
      a folder sync must carry), and the backend checklist names the
      suite.
- [x] `cargo build -p shuyi-ui --no-default-features --features
      storage-conformance --target thumbv7em-none-eabihf --locked`
      builds, and `cargo xtask check-no-std` passes.
- [x] The PR description has the timing of step 8.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-ui`): the suite against the three in-memory rigs, with
  and without default features.
- Unit (`shuyi-app`, real files in a `tempfile` folder): each read
  fallback, an empty value and a value of `\0` characters, a leftover
  `.tmp`, `delete` and `list` with backups, the warning caught by a test
  `warn` function (writing into a `thread_local!`).
- Unit (`shuyi-app`, the fake disk): `conformance_suite`, and
  `power_cut_suite` with both kinds of cut.

## Completion notes

- **The suite:** `shuyi_ui::storage::{PowerCutRig, power_cut_suite}` in
  `crates/ui/src/storage/power_cut.rs`, behind `storage-conformance`,
  `no_std` with `alloc`. Its tests run one in-memory rig in three modes:
  atomic passes, in place fails promise 1 (a cut after 1 operation reads
  `""`), forgetful fails promise 4. More tests: a first write and a
  delete in place fail promises 2 and 3, a rig whose `list()` names no
  key fails promise 6, the suite asks for the old value, the new one and
  a delete, and an operation that never finishes fails after 10 000
  operations.
- **`FileStorage`** (`crates/app/src/storage/native.rs`): its file calls
  go through a private `Disk` (`StdDisk` in the game), and `write`,
  `read`, `delete` and `list` work as the scope says, with `is_good`,
  `damage` (the warning's reason) and the `warn` field. Since 3041 is
  not done, the warning goes through macroquad's `warn!` in
  `log_warning`; 3041's text now names it.
- **The fake disk** (`native/fake_disk.rs`) runs `conformance_suite`
  and `power_cut_suite` with process-crash cuts and with power cuts.
  With the folder sync taken out of `write` the power-cut run failed
  (promise 4: a write over an old value set back with the power on read
  nothing after the cut, since its rename never reached the disk), and
  the process-crash run still passed. The sync was put back.
- **Timing** (this Windows 11 machine, NTFS, release build, 100 writes
  of 10 KiB): before about 0.45 ms a write, after about 5 ms (three
  runs: 5.2, 5.1, 4.7 ms). Under the 16 ms of a frame, so no ticket for
  saving off the main thread.
- **Deviations:**
  - The PR is about 1 100 changed lines, over the scope's "about 600",
    and keeps the fake disk and `FileStorage`'s power-cut runs anyway.
    They are test code only, and they are what proves the new `write`
    (acceptance criteria 2 and 3); split off, the fix would land
    unchecked against a power cut and the follow-up would redo code
    already written and passing.
  - `Disk` is `Send + Sync + UnwindSafe + RefUnwindSafe`, and the fake
    disk shares its folder through
    an `Arc<Mutex<…>>` rather than an `Rc<RefCell<…>>`: a
    `Box<dyn Disk>` that wasn't would have made `FileStorage` lose
    those auto traits, a break of `shuyi-app`'s API (CI's `api` job
    caught the unwind ones). A test keeps them.
  - `list` returns its keys sorted (a `BTreeSet`); `Storage` promises no
    order.
  - `delete` on a folder that doesn't exist yet is `Ok`: the folder's
    sync answering `NotFound` is not an error there.
- **For the game:** no API change. Its save folder gains a
  `<key>.ron.bak` beside each save from the second write on; a Steam
  Auto-Cloud setup syncs `*.ron` and `*.ron.bak` (3037 says so). The
  game's pin bump needs no code change.
- No ADR: the seam is private, and the suite follows
  `conformance_suite`'s pattern.
- No break.
