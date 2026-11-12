//! The power-cut suite (ticket 3045): what "a save survives a power cut"
//! means, for every backend's [`Storage`]. A backend fakes its medium (a
//! folder of files, a flash chip) in a [`PowerCutRig`] that can lose the
//! power after any operation, and runs [`power_cut_suite`] over it.

use super::bundle::{Verdict, apply, plan_import, recover, write_bundle};
use super::{MemoryStorage, Storage, StorageError};
use crate::prelude::*;
use shuyi_core::made::{ContentHash, Made};

/// A [`Storage`] over a fake of its medium, which can lose the power: what
/// a backend writes to run [`power_cut_suite`].
pub trait PowerCutRig {
    /// The storage as the game sees it.
    fn storage(&mut self) -> &mut dyn Storage;
    /// The power goes after `ops` more operations on the medium (a file
    /// written, renamed or removed, a folder synced; a flash sector
    /// erased), and every operation after that fails.
    fn cut_after(&mut self, ops: u32);
    /// The power comes back. The medium keeps only what it had made
    /// durable, the storage is opened again over it, as at the game's next
    /// start, and no cut is pending.
    fn restart(&mut self);
}

/// The key the operations run on.
const KEY: &str = "save-slot-1";
/// Another key, written once at the start.
const OTHER: &str = "settings";
const OTHER_VALUE: &str = "(volume: 80)";
/// The value before the operation. [`NEW`] is longer, so a torn write shows
/// as a prefix of it.
const OLD: &str = "(slot: 1, gold: 10)";
const NEW: &str = "(slot: 1, gold: 250, items: [lamp, rope, map])";
/// What the suite writes with the power on, after each cut.
const AFTER: &str = "(slot: 1, gold: 0)";
/// The most operations of the medium a write or a delete may take.
const MAX_OPS: u32 = 10_000;

/// The keys of the save bundle the suite imports: each with its value
/// before the import, if any, and the bundle's.
// check-text: not player text (storage keys and values)
const IMPORT: [(&str, Option<&str>, &str); 3] = [
    (
        "slot-a",
        Some("(slot: a, gold: 1)"),
        "(slot: a, gold: 100, items: [lamp])",
    ),
    (
        "slot-b",
        Some("(slot: b, gold: 2)"),
        "(slot: b, gold: 200, items: [rope])",
    ),
    ("slot-c", None, "(slot: c, gold: 300)"),
];

/// The import, as the suite's panics name it.
// check-text: not player text (the suite's panics)
const AN_IMPORT: &str = "an import of a save bundle";

// check-text: not player text (the suite's panics)
const PROMISES: [&str; 8] = [
    "a write over an old value reads the old value or the new one",
    "a first write reads nothing or the new value",
    "a delete reads the old value or nothing",
    "a write or delete that returned Ok reads its result after the restart",
    "another key reads the same after every restart",
    "list() names exactly the keys that read Some",
    "with the power on, a write after the cut works and reads back",
    "an import of a save bundle, once recovered, reads every old value or every new one",
];

/// One of the operations the suite cuts the power in.
#[derive(Debug, Clone, Copy)]
enum Op {
    Overwrite,
    FirstWrite,
    Delete,
}

impl Op {
    // check-text: not player text (the suite's panics)
    fn name(self) -> &'static str {
        match self {
            Self::Overwrite => "a write over an old value",
            Self::FirstWrite => "a first write",
            Self::Delete => "a delete",
        }
    }

    /// The promise (1 to 3) of what the key reads after a cut in it.
    fn promise(self) -> usize {
        match self {
            Self::Overwrite => 1,
            Self::FirstWrite => 2,
            Self::Delete => 3,
        }
    }

    /// What the key holds before the operation.
    fn before(self) -> Option<&'static str> {
        match self {
            Self::Overwrite | Self::Delete => Some(OLD),
            Self::FirstWrite => None,
        }
    }

    /// What the key holds once the operation is done.
    fn after(self) -> Option<&'static str> {
        match self {
            Self::Overwrite | Self::FirstWrite => Some(NEW),
            Self::Delete => None,
        }
    }

    fn run(self, storage: &mut dyn Storage) -> Result<(), StorageError> {
        match self.after() {
            Some(value) => storage.write(KEY, value),
            None => storage.delete(KEY),
        }
    }
}

/// Cuts the power in a write over an old value, a first write and a
/// delete, after each number of operations of the medium in turn, until
/// the operation finishes, and checks what the storage reads after each
/// restart. Panics with the promise broken, the operation and the cut.
///
/// The promises:
/// 1. a write over an old value reads the old value or the new one: never
///    `None`, a mix or an error;
/// 2. a first write reads `None` or the new value;
/// 3. a delete reads the old value or `None`;
/// 4. a write or delete that returned `Ok` reads its result after the
///    restart (the restart is a power cut too);
/// 5. another key, written once at the start, reads the same after every
///    restart;
/// 6. `list()` names exactly the keys that read `Some`;
/// 7. with the power on, a write after the cut works and reads back;
/// 8. an import of a save bundle ([`super::bundle::apply`]) cut anywhere
///    reads, once [`recover`] has run after the restart, every old value
///    of its keys or every new one, and the new ones if it returned `Ok`;
///    no key of the bundle's own is left.
///
/// An operation that needs more than 10 000 operations of the medium fails
/// the suite.
pub fn power_cut_suite(rig: &mut dyn PowerCutRig) {
    rig.restart();
    if let Err(e) = rig.storage().write(OTHER, OTHER_VALUE) {
        fail(5, None, &format!("writing {OTHER} failed: {e}"));
    }
    for op in [Op::Overwrite, Op::FirstWrite, Op::Delete] {
        check_op(rig, op);
    }
    check_import(rig);
}

fn check_op(rig: &mut dyn PowerCutRig, op: Op) {
    for ops in 0..=MAX_OPS {
        let cut = Some((op.name(), ops));
        let set_back = match op.before() {
            Some(value) => rig.storage().write(KEY, value),
            None => rig.storage().delete(KEY),
        };
        if let Err(e) = set_back {
            fail(7, cut, &format!("setting {KEY} back failed: {e}"));
        }
        rig.restart();
        expect_read(rig, KEY, op.before(), 4, cut, " set back with the power on");

        rig.cut_after(ops);
        let result = op.run(rig.storage());
        rig.restart();

        let read = rig.storage().read(KEY);
        if !matches!(&read, Ok(value) if *value == op.before().map(ToOwned::to_owned)
            || *value == op.after().map(ToOwned::to_owned))
        {
            fail(op.promise(), cut, &format!("{KEY} read {read:?}"));
        }
        if result.is_ok() {
            expect_read(rig, KEY, op.after(), 4, cut, " after an Ok");
        }
        expect_read(rig, OTHER, Some(OTHER_VALUE), 5, cut, "");
        check_list(rig, cut);

        if let Err(e) = rig.storage().write(KEY, AFTER) {
            fail(7, cut, &format!("the write failed: {e}"));
        }
        expect_read(rig, KEY, Some(AFTER), 7, cut, "");

        if result.is_ok() {
            return;
        }
    }
    // check-text: not player text (the suite's panics)
    panic!(
        "power-cut suite: {} still fails after {MAX_OPS} operations of the medium",
        op.name()
    );
}

/// Cuts the power in an import of a save bundle after each number of
/// operations of the medium in turn, until it finishes, and checks what
/// its keys read once [`recover`] has run after the restart (promise 8).
// check-text: not player text (the suite's panics and its bundle's stamp)
fn check_import(rig: &mut dyn PowerCutRig) {
    let made = Made::new("power-cut suite", ContentHash(0));
    let mut source = MemoryStorage::new();
    for (key, _, new) in IMPORT {
        if let Err(e) = source.write(key, new) {
            fail(8, None, &format!("writing the bundle's {key} failed: {e}"));
        }
    }
    let text = match write_bundle(&source, &made, |_| true) {
        Ok(text) => text,
        Err(e) => fail(8, None, &format!("writing the bundle failed: {e}")),
    };
    let old: Vec<Option<String>> = IMPORT
        .iter()
        .map(|(_, old, _)| old.map(ToOwned::to_owned))
        .collect();
    let new: Vec<Option<String>> = IMPORT
        .iter()
        .map(|(_, _, new)| Some((*new).to_owned()))
        .collect();
    let mut keys = vec![KEY, OTHER];
    keys.extend(IMPORT.iter().map(|(key, _, _)| *key));
    let accept = |_: &str, _: &str| Verdict::Ready;
    for ops in 0..=MAX_OPS {
        let cut = Some((AN_IMPORT, ops));
        if let Err(e) = set_back_import(rig.storage()) {
            fail(
                8,
                cut,
                &format!("setting the bundle's keys back failed: {e}"),
            );
        }
        rig.restart();
        let plan = match plan_import(&text, rig.storage(), &accept, &made) {
            Ok(plan) => plan,
            Err(e) => fail(8, cut, &format!("planning the import failed: {e}")),
        };

        rig.cut_after(ops);
        let result = apply(&plan, rig.storage(), |_| true);
        rig.restart();
        if let Err(e) = recover(rig.storage()) {
            fail(8, cut, &format!("recover failed: {e}"));
        }

        let storage = rig.storage();
        let mut read = Vec::new();
        for (key, _, _) in IMPORT {
            match storage.read(key) {
                Ok(value) => read.push(value),
                Err(e) => fail(8, cut, &format!("reading {key} failed: {e}")),
            }
        }
        if read != old && read != new {
            fail(
                8,
                cut,
                &format!("the keys read {read:?}, a mix of old and new"),
            );
        }
        if result.is_ok() && read != new {
            fail(
                8,
                cut,
                &format!("it returned Ok, but the keys read {read:?}"),
            );
        }
        expect_read(rig, OTHER, Some(OTHER_VALUE), 5, cut, "");
        check_list_of(rig, &keys, 8, cut);

        if result.is_ok() {
            return;
        }
    }
    // check-text: not player text (the suite's panics)
    panic!("power-cut suite: {AN_IMPORT} still fails after {MAX_OPS} operations of the medium");
}

/// Gives the import's keys their values before it, with the power on.
fn set_back_import(storage: &mut dyn Storage) -> Result<(), StorageError> {
    recover(storage)?;
    for (key, old, _) in IMPORT {
        match old {
            Some(value) => storage.write(key, value)?,
            None => storage.delete(key)?,
        }
    }
    Ok(())
}

fn expect_read(
    rig: &mut dyn PowerCutRig,
    key: &str,
    expected: Option<&str>,
    promise: usize,
    cut: Option<(&str, u32)>,
    when: &str,
) {
    let read = rig.storage().read(key);
    if read != Ok(expected.map(ToOwned::to_owned)) {
        fail(
            promise,
            cut,
            &format!("{key}{when} read {read:?}, not {expected:?}"),
        );
    }
}

fn check_list(rig: &mut dyn PowerCutRig, cut: Option<(&str, u32)>) {
    check_list_of(rig, &[KEY, OTHER], 6, cut);
}

/// Checks that `list()` names exactly those of `keys` that read `Some`, as
/// promise `promise` says.
fn check_list_of(
    rig: &mut dyn PowerCutRig,
    keys: &[&str],
    promise: usize,
    cut: Option<(&str, u32)>,
) {
    let storage = rig.storage();
    let mut reading = Vec::new();
    for key in keys {
        if matches!(storage.read(key), Ok(Some(_))) {
            reading.push((*key).to_owned());
        }
    }
    reading.sort();
    let mut listed = match storage.list() {
        Ok(keys) => keys,
        Err(e) => fail(promise, cut, &format!("list() failed: {e}")),
    };
    listed.sort();
    if listed != reading {
        fail(
            promise,
            cut,
            &format!("list() named {listed:?}, but the keys that read are {reading:?}"),
        );
    }
}

// check-text: not player text (the suite's panics)
fn fail(promise: usize, cut: Option<(&str, u32)>, what: &str) -> ! {
    let text = PROMISES[promise - 1];
    match cut {
        Some((op, ops)) => panic!(
            "power-cut suite, promise {promise} ({text}): {op}, power cut after {ops} operations: {what}"
        ),
        None => panic!("power-cut suite, promise {promise} ({text}): {what}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeMap;

    /// How [`MemoryRig`] writes.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Mode {
        /// A write is one operation, durable at once.
        Atomic,
        /// A write empties the value, then sets it, and a delete empties
        /// it, then removes it: two operations each.
        InPlace,
        /// A write returns `Ok`, but the restart loses it.
        Forgetful,
        /// Atomic, but `list()` names no key.
        Unlisted,
    }

    /// A rig over a map, which is its own storage.
    #[derive(Debug)]
    struct MemoryRig {
        mode: Mode,
        values: BTreeMap<String, String>,
        durable: BTreeMap<String, String>,
        ops_left: Option<u32>,
        /// Every write (`Some`) and delete (`None`) the suite asked for.
        asked: Vec<(String, Option<String>)>,
    }

    impl MemoryRig {
        fn new(mode: Mode) -> Self {
            Self {
                mode,
                values: BTreeMap::new(),
                durable: BTreeMap::new(),
                ops_left: None,
                asked: Vec::new(),
            }
        }

        /// One operation of the medium: fails once the power is off.
        fn op(&mut self) -> Result<(), StorageError> {
            match self.ops_left {
                Some(0) => Err(StorageError::Backend("the power is off".to_owned())),
                Some(left) => {
                    self.ops_left = Some(left - 1);
                    Ok(())
                }
                None => Ok(()),
            }
        }

        fn set(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
            self.op()?;
            self.values.insert(key.to_owned(), value.to_owned());
            if self.mode != Mode::Forgetful {
                self.durable.insert(key.to_owned(), value.to_owned());
            }
            Ok(())
        }
    }

    impl Storage for MemoryRig {
        fn read(&self, key: &str) -> Result<Option<String>, StorageError> {
            Ok(self.values.get(key).cloned())
        }

        fn write(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
            self.asked.push((key.to_owned(), Some(value.to_owned())));
            if self.mode == Mode::InPlace {
                self.set(key, "")?;
            }
            self.set(key, value)
        }

        fn delete(&mut self, key: &str) -> Result<(), StorageError> {
            self.asked.push((key.to_owned(), None));
            if self.mode == Mode::InPlace {
                self.set(key, "")?;
            }
            self.op()?;
            self.values.remove(key);
            self.durable.remove(key);
            Ok(())
        }

        fn list(&self) -> Result<Vec<String>, StorageError> {
            if self.mode == Mode::Unlisted {
                return Ok(Vec::new());
            }
            Ok(self.values.keys().cloned().collect())
        }
    }

    impl PowerCutRig for MemoryRig {
        fn storage(&mut self) -> &mut dyn Storage {
            self
        }

        fn cut_after(&mut self, ops: u32) {
            self.ops_left = Some(ops);
        }

        fn restart(&mut self) {
            self.values = self.durable.clone();
            self.ops_left = None;
        }
    }

    #[test]
    fn an_atomic_storage_passes() {
        power_cut_suite(&mut MemoryRig::new(Mode::Atomic));
    }

    /// The suite writes the old value and the new one, and deletes, on the
    /// key it checks.
    #[test]
    fn the_suite_writes_the_old_and_the_new_value_and_deletes() {
        let mut rig = MemoryRig::new(Mode::Atomic);
        power_cut_suite(&mut rig);
        for asked in [Some(OLD), Some(NEW), None] {
            let asked = (KEY.to_owned(), asked.map(ToOwned::to_owned));
            assert!(rig.asked.contains(&asked), "{asked:?}");
        }
    }

    #[test]
    #[should_panic(
        expected = "promise 2 (a first write reads nothing or the new value): a first write, power cut after 1 operations"
    )]
    fn a_first_write_in_place_fails_promise_2() {
        let mut rig = MemoryRig::new(Mode::InPlace);
        rig.write(OTHER, OTHER_VALUE).unwrap();
        check_op(&mut rig, Op::FirstWrite);
    }

    #[test]
    #[should_panic(
        expected = "promise 3 (a delete reads the old value or nothing): a delete, power cut after 1 operations"
    )]
    fn a_delete_in_place_fails_promise_3() {
        let mut rig = MemoryRig::new(Mode::InPlace);
        rig.write(OTHER, OTHER_VALUE).unwrap();
        check_op(&mut rig, Op::Delete);
    }

    /// An import on a storage that loses its writes at the restart reads
    /// neither its old values nor its new ones.
    #[test]
    #[should_panic(
        expected = "promise 8 (an import of a save bundle, once recovered, reads every old value or every new one): an import of a save bundle, power cut after 0 operations"
    )]
    fn an_import_on_a_storage_that_forgets_its_writes_fails_promise_8() {
        check_import(&mut MemoryRig::new(Mode::Forgetful));
    }

    /// A storage that writes in place keeps promise 8 all the same: the
    /// journal is written before any value, so a torn one has nothing to
    /// undo, and a torn value is put back from its backup.
    #[test]
    fn an_import_on_a_storage_that_writes_in_place_keeps_promise_8() {
        let mut rig = MemoryRig::new(Mode::InPlace);
        rig.write(OTHER, OTHER_VALUE).unwrap();
        check_import(&mut rig);
    }

    /// The suite imports the bundle's keys, and leaves none of the
    /// bundle's own.
    #[test]
    fn the_suite_imports_a_bundle() {
        let mut rig = MemoryRig::new(Mode::Atomic);
        power_cut_suite(&mut rig);
        for (key, _, new) in IMPORT {
            let asked = (key.to_owned(), Some(new.to_owned()));
            assert!(rig.asked.contains(&asked), "{asked:?}");
        }
        assert!(rig.asked.iter().any(|(key, _)| key == "_bundle-journal"));
        assert!(rig.values.keys().all(|key| !key.starts_with('_')));
    }

    #[test]
    #[should_panic(expected = "promise 6 (list() names exactly the keys that read Some)")]
    fn a_storage_that_lists_no_key_fails() {
        power_cut_suite(&mut MemoryRig::new(Mode::Unlisted));
    }

    #[test]
    #[should_panic(
        expected = "promise 1 (a write over an old value reads the old value or the new one): a write over an old value, power cut after 1 operations"
    )]
    fn a_storage_that_writes_in_place_fails() {
        power_cut_suite(&mut MemoryRig::new(Mode::InPlace));
    }

    #[test]
    #[should_panic(
        expected = "promise 4 (a write or delete that returned Ok reads its result after the restart)"
    )]
    fn a_storage_that_forgets_its_writes_fails() {
        power_cut_suite(&mut MemoryRig::new(Mode::Forgetful));
    }

    /// A rig whose cut never comes: every write fails, so the suite gives
    /// up rather than loop for ever.
    #[test]
    #[should_panic(expected = "a write over an old value still fails after 10000 operations")]
    fn an_operation_that_never_finishes_fails() {
        #[derive(Debug)]
        struct Failing(MemoryRig);
        impl Storage for Failing {
            fn read(&self, key: &str) -> Result<Option<String>, StorageError> {
                self.0.read(key)
            }
            fn write(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
                if self.0.ops_left.is_some() {
                    return Err(StorageError::Backend("always".to_owned()));
                }
                self.0.write(key, value)
            }
            fn delete(&mut self, key: &str) -> Result<(), StorageError> {
                self.0.delete(key)
            }
            fn list(&self) -> Result<Vec<String>, StorageError> {
                self.0.list()
            }
        }
        impl PowerCutRig for Failing {
            fn storage(&mut self) -> &mut dyn Storage {
                self
            }
            fn cut_after(&mut self, ops: u32) {
                self.0.cut_after(ops);
            }
            fn restart(&mut self) {
                self.0.restart();
            }
        }
        power_cut_suite(&mut Failing(MemoryRig::new(Mode::Atomic)));
    }
}
