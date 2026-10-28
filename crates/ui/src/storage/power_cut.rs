//! The power-cut suite (ticket 3045): what "a save survives a power cut"
//! means, for every backend's [`Storage`]. A backend fakes its medium (a
//! folder of files, a flash chip) in a [`PowerCutRig`] that can lose the
//! power after any operation, and runs [`power_cut_suite`] over it.

use super::{Storage, StorageError};
use crate::prelude::*;

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

// check-text: not player text (the suite's panics)
const PROMISES: [&str; 7] = [
    "a write over an old value reads the old value or the new one",
    "a first write reads nothing or the new value",
    "a delete reads the old value or nothing",
    "a write or delete that returned Ok reads its result after the restart",
    "another key reads the same after every restart",
    "list() names exactly the keys that read Some",
    "with the power on, a write after the cut works and reads back",
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
/// 7. with the power on, a write after the cut works and reads back.
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
}

fn check_op(rig: &mut dyn PowerCutRig, op: Op) {
    for ops in 0..=MAX_OPS {
        let cut = Some((op, ops));
        let set_back = match op.before() {
            Some(value) => rig.storage().write(KEY, value),
            None => rig.storage().delete(KEY),
        };
        if let Err(e) = set_back {
            fail(7, cut, &format!("setting {KEY} back failed: {e}"));
        }
        rig.restart();
        expect_read(rig, KEY, op.before(), 4, cut, "set back with the power on");

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
            expect_read(rig, KEY, op.after(), 4, cut, "after an Ok");
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

fn expect_read(
    rig: &mut dyn PowerCutRig,
    key: &str,
    expected: Option<&str>,
    promise: usize,
    cut: Option<(Op, u32)>,
    when: &str,
) {
    let read = rig.storage().read(key);
    if read != Ok(expected.map(ToOwned::to_owned)) {
        fail(
            promise,
            cut,
            &format!("{key} {when} read {read:?}, not {expected:?}"),
        );
    }
}

fn check_list(rig: &mut dyn PowerCutRig, cut: Option<(Op, u32)>) {
    let storage = rig.storage();
    let mut reading = Vec::new();
    for key in [KEY, OTHER] {
        if matches!(storage.read(key), Ok(Some(_))) {
            reading.push(key.to_owned());
        }
    }
    let mut listed = match storage.list() {
        Ok(keys) => keys,
        Err(e) => fail(6, cut, &format!("list() failed: {e}")),
    };
    listed.sort();
    if listed != reading {
        fail(
            6,
            cut,
            &format!("list() named {listed:?}, but the keys that read are {reading:?}"),
        );
    }
}

// check-text: not player text (the suite's panics)
fn fail(promise: usize, cut: Option<(Op, u32)>, what: &str) -> ! {
    let text = PROMISES[promise - 1];
    match cut {
        Some((op, ops)) => panic!(
            "power-cut suite, promise {promise} ({text}): {}, power cut after {ops} operations: {what}",
            op.name()
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
        /// A write empties the value, then sets it: two operations.
        InPlace,
        /// A write returns `Ok`, but the restart loses it.
        Forgetful,
    }

    /// A rig over a map, which is its own storage.
    #[derive(Debug)]
    struct MemoryRig {
        mode: Mode,
        values: BTreeMap<String, String>,
        durable: BTreeMap<String, String>,
        ops_left: Option<u32>,
    }

    impl MemoryRig {
        fn new(mode: Mode) -> Self {
            Self {
                mode,
                values: BTreeMap::new(),
                durable: BTreeMap::new(),
                ops_left: None,
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
            if self.mode == Mode::InPlace {
                self.set(key, "")?;
            }
            self.set(key, value)
        }

        fn delete(&mut self, key: &str) -> Result<(), StorageError> {
            self.op()?;
            self.values.remove(key);
            self.durable.remove(key);
            Ok(())
        }

        fn list(&self) -> Result<Vec<String>, StorageError> {
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
