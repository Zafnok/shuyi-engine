//! A [`Storage`] that fails when a test tells it to: the one double the
//! ui, platform and app tests share, in place of one hand-written
//! `impl Storage` each.

use super::{MemoryStorage, Storage, StorageError};
use crate::prelude::*;
use alloc::rc::Rc;
use core::cell::RefCell;

/// [`Storage::read`], in [`FaultyStorage`]'s set of failing operations.
const READ: u8 = 1;
/// [`Storage::write`].
const WRITE: u8 = 2;
/// [`Storage::delete`].
const DELETE: u8 = 4;
/// [`Storage::list`].
const LIST: u8 = 8;

/// A [`MemoryStorage`] that fails the operations it is told to, for
/// tests.
///
/// Every failure is [`StorageError::Backend`] with one text
/// ([`with_error`](Self::with_error), `"down"` at first). A clone shares
/// the entries (an `Rc`), so a test keeps a handle on what a game it
/// handed a clone wrote; the faults and the count of writes are each
/// clone's own.
///
/// ```
/// use shuyi_ui::storage::{FaultyStorage, Storage, StorageError};
///
/// let mut storage = FaultyStorage::new().failing_writes().with_error("full");
/// assert_eq!(
///     storage.write("settings", "()"),
///     Err(StorageError::Backend("full".into()))
/// );
/// assert_eq!(storage.read("settings"), Ok(None));
/// ```
#[derive(Debug, Clone)]
#[must_use]
pub struct FaultyStorage {
    /// The entries, shared by every clone.
    memory: Rc<RefCell<MemoryStorage>>,
    /// The text of every failure.
    error: String,
    /// Which operations fail every time: a set of [`READ`], [`WRITE`],
    /// [`DELETE`] and [`LIST`].
    failing: u8,
    /// A key every read, write and delete of fails.
    key: Option<String>,
    /// How many more writes succeed before every write fails.
    budget: Option<usize>,
    /// The write (from 1) that fails, once.
    fail_at: Option<usize>,
    /// The writes tried so far.
    tried: usize,
    /// Whether a write that doesn't fail stores nothing.
    drop_writes: bool,
}

impl Default for FaultyStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl FaultyStorage {
    /// An empty storage that fails nothing yet.
    pub fn new() -> Self {
        Self::over(MemoryStorage::new())
    }

    /// A storage holding `memory`'s entries that fails nothing yet.
    pub fn over(memory: MemoryStorage) -> Self {
        Self {
            memory: Rc::new(RefCell::new(memory)),
            error: "down".to_owned(),
            failing: 0,
            key: None,
            budget: None,
            fail_at: None,
            tried: 0,
            drop_writes: false,
        }
    }

    /// The text of every failure from now on.
    pub fn with_error(mut self, error: &str) -> Self {
        error.clone_into(&mut self.error);
        self
    }

    /// Every operation fails.
    pub fn failing_all(self) -> Self {
        self.failing_reads()
            .failing_writes()
            .failing_deletes()
            .failing_lists()
    }

    /// Every read fails.
    pub fn failing_reads(self) -> Self {
        self.failing_op(READ)
    }

    /// Every write fails.
    pub fn failing_writes(self) -> Self {
        self.failing_op(WRITE)
    }

    /// Every delete fails.
    pub fn failing_deletes(self) -> Self {
        self.failing_op(DELETE)
    }

    /// Every list fails.
    pub fn failing_lists(self) -> Self {
        self.failing_op(LIST)
    }

    /// `op` fails every time.
    fn failing_op(mut self, op: u8) -> Self {
        self.failing |= op;
        self
    }

    /// Whether `op` fails every time.
    fn fails(&self, op: u8) -> bool {
        self.failing & op != 0
    }

    /// Every read, write and delete of `key` fails; the other keys work.
    pub fn failing_key(mut self, key: &str) -> Self {
        self.key = Some(key.to_owned());
        self
    }

    /// The first `writes` writes succeed, and every write after them
    /// fails.
    pub fn with_write_budget(mut self, writes: usize) -> Self {
        self.budget = Some(writes);
        self
    }

    /// The `nth` write (counting from 1, failed ones too) fails, and the
    /// writes before and after it work.
    pub fn failing_write(mut self, nth: usize) -> Self {
        self.fail_at = Some(nth);
        self
    }

    /// A write that doesn't fail is accepted and stores nothing: a storage
    /// that loses what it is given.
    pub fn dropping_writes(mut self) -> Self {
        self.drop_writes = true;
        self
    }

    /// A copy of the entries as they are now, faults left behind.
    #[must_use]
    pub fn memory(&self) -> MemoryStorage {
        self.memory.borrow().clone()
    }

    /// The failure.
    fn fail(&self) -> StorageError {
        StorageError::Backend(self.error.clone())
    }

    /// Whether an operation on `key` fails because of the key.
    fn fails_key(&self, key: &str) -> bool {
        self.key.as_deref() == Some(key)
    }
}

impl Storage for FaultyStorage {
    fn read(&self, key: &str) -> Result<Option<String>, StorageError> {
        if self.fails(READ) || self.fails_key(key) {
            return Err(self.fail());
        }
        self.memory.borrow().read(key)
    }

    fn write(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        self.tried += 1;
        if self.fails(WRITE) || self.fails_key(key) || self.fail_at == Some(self.tried) {
            return Err(self.fail());
        }
        if let Some(left) = &mut self.budget {
            if *left == 0 {
                return Err(self.fail());
            }
            *left -= 1;
        }
        if self.drop_writes {
            return Ok(());
        }
        self.memory.borrow_mut().write(key, value)
    }

    fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        if self.fails(DELETE) || self.fails_key(key) {
            return Err(self.fail());
        }
        self.memory.borrow_mut().delete(key)
    }

    fn list(&self) -> Result<Vec<String>, StorageError> {
        if self.fails(LIST) {
            return Err(self.fail());
        }
        self.memory.borrow().list()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn down() -> StorageError {
        StorageError::Backend("down".to_owned())
    }

    #[test]
    fn with_no_faults_it_passes_the_conformance_suite() {
        super::super::conformance_suite(&mut FaultyStorage::new());
    }

    #[test]
    fn failing_all_fails_every_operation_with_down() {
        let mut s = FaultyStorage::new().failing_all();
        assert_eq!(s.read("a"), Err(down()));
        assert_eq!(s.write("a", "1"), Err(down()));
        assert_eq!(s.delete("a"), Err(down()));
        assert_eq!(s.list(), Err(down()));
    }

    #[test]
    fn each_operation_fails_alone_with_the_text_set() {
        let mut memory = MemoryStorage::new();
        memory.write("a", "1").unwrap();
        let full = Err(StorageError::Backend("full".to_owned()));
        let mut s = FaultyStorage::over(memory.clone())
            .failing_reads()
            .with_error("full");
        assert_eq!(s.read("a"), full);
        assert_eq!(s.list(), Ok(vec!["a".to_owned()]));
        s.write("b", "2").unwrap();
        s.delete("b").unwrap();
        let mut s = FaultyStorage::over(memory.clone()).failing_writes();
        assert_eq!(s.write("b", "2"), Err(down()));
        assert_eq!(s.read("a"), Ok(Some("1".to_owned())));
        let mut s = FaultyStorage::over(memory.clone()).failing_deletes();
        assert_eq!(s.delete("a"), Err(down()));
        s.write("b", "2").unwrap();
        let s = FaultyStorage::over(memory).failing_lists();
        assert_eq!(s.list(), Err(down()));
        assert_eq!(s.read("a"), Ok(Some("1".to_owned())));
    }

    #[test]
    fn a_failing_key_fails_alone() {
        let mut s = FaultyStorage::new().failing_key("bad");
        s.write("good", "1").unwrap();
        assert_eq!(s.write("bad", "2"), Err(down()));
        assert_eq!(s.read("bad"), Err(down()));
        assert_eq!(s.delete("bad"), Err(down()));
        assert_eq!(s.read("good"), Ok(Some("1".to_owned())));
        assert_eq!(s.list(), Ok(vec!["good".to_owned()]));
    }

    #[test]
    fn a_write_budget_lets_that_many_writes_through_then_none() {
        let mut s = FaultyStorage::new().with_write_budget(2);
        s.write("a", "1").unwrap();
        s.write("b", "2").unwrap();
        assert_eq!(s.write("c", "3"), Err(down()));
        assert_eq!(s.write("c", "3"), Err(down()));
        assert_eq!(s.list(), Ok(vec!["a".to_owned(), "b".to_owned()]));
        let mut none = FaultyStorage::new().with_write_budget(0);
        assert_eq!(none.write("a", "1"), Err(down()));
    }

    #[test]
    fn the_nth_write_fails_once() {
        let mut s = FaultyStorage::new().failing_write(2);
        s.write("a", "1").unwrap();
        assert_eq!(s.write("b", "2"), Err(down()));
        s.write("b", "2").unwrap();
        assert_eq!(s.list(), Ok(vec!["a".to_owned(), "b".to_owned()]));
    }

    #[test]
    fn dropped_writes_succeed_and_store_nothing() {
        let mut s = FaultyStorage::new().dropping_writes();
        s.write("a", "1").unwrap();
        assert_eq!(s.read("a"), Ok(None));
        assert_eq!(s.list(), Ok(Vec::new()));
    }

    #[test]
    fn a_clone_shares_the_entries_and_memory_copies_them() {
        let s = FaultyStorage::new();
        let mut game = s.clone().failing_reads();
        game.write("a", "1").unwrap();
        assert_eq!(s.read("a"), Ok(Some("1".to_owned())));
        let copy = s.memory();
        game.write("a", "2").unwrap();
        assert_eq!(copy.read("a"), Ok(Some("1".to_owned())));
        assert_eq!(s.read("a"), Ok(Some("2".to_owned())));
    }

    #[test]
    fn default_is_new() {
        let mut s = FaultyStorage::default();
        s.write("a", "1").unwrap();
        assert_eq!(s.read("a"), Ok(Some("1".to_owned())));
    }
}
