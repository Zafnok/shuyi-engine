//! Persistence: a game's saves and the player's settings go through
//! [`Storage`], so the UI stays pure. The platform supplies the real
//! implementation (files, the browser's `localStorage`, a console's own
//! files); [`MemoryStorage`] is for tests and the test harness.

use crate::prelude::*;
use alloc::collections::BTreeMap;
use core::fmt;

/// Longest key [`Storage`] accepts, in bytes.
pub const MAX_KEY_LEN: usize = 64;

/// A place to persist small named blobs of text (RON documents, typically):
/// files on native, `localStorage` on web.
///
/// Keys must match `^[a-z0-9_-]{1,64}$` ([`is_valid_key`]); implementations
/// turn a key straight into a file name, so anything else is rejected rather
/// than sanitised.
pub trait Storage: fmt::Debug {
    /// The value stored at `key`, or `None` if nothing has been written
    /// there (or it was deleted).
    fn read(&self, key: &str) -> Result<Option<String>, StorageError>;
    /// Stores `value` at `key`, replacing whatever was there.
    fn write(&mut self, key: &str, value: &str) -> Result<(), StorageError>;
    /// Removes `key`. Not an error if it didn't exist.
    fn delete(&mut self, key: &str) -> Result<(), StorageError>;
    /// Every key currently stored, in no particular order.
    fn list(&self) -> Result<Vec<String>, StorageError>;
}

/// Why a [`Storage`] operation failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StorageError {
    /// The key doesn't match `^[a-z0-9_-]{1,64}$`.
    InvalidKey(String),
    /// The backend (filesystem, `localStorage`, …) reported a problem.
    Backend(String),
}

// check-text: not player text (for the log)
impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKey(key) => write!(f, "invalid storage key {key:?}"),
            Self::Backend(message) => write!(f, "storage error: {message}"),
        }
    }
}

/// Whether `key` matches `^[a-z0-9_-]{1,64}$`: lowercase ASCII letters,
/// digits, `_` and `-`, 1 to [`MAX_KEY_LEN`] bytes.
#[must_use]
pub fn is_valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= MAX_KEY_LEN
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// Returns [`StorageError::InvalidKey`] unless `key` is valid.
fn check_key(key: &str) -> Result<(), StorageError> {
    if is_valid_key(key) {
        Ok(())
    } else {
        Err(StorageError::InvalidKey(key.to_owned()))
    }
}

/// An in-memory [`Storage`], for tests and the harness. Nothing persists
/// past the process.
#[derive(Debug, Clone, Default)]
pub struct MemoryStorage {
    values: BTreeMap<String, String>,
}

impl MemoryStorage {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Storage for MemoryStorage {
    fn read(&self, key: &str) -> Result<Option<String>, StorageError> {
        check_key(key)?;
        Ok(self.values.get(key).cloned())
    }

    fn write(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        check_key(key)?;
        self.values.insert(key.to_owned(), value.to_owned());
        Ok(())
    }

    fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        check_key(key)?;
        self.values.remove(key);
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>, StorageError> {
        Ok(self.values.keys().cloned().collect())
    }
}

/// Unwraps a storage result, panicking with the error on failure. Named
/// `expect_ok` rather than `.unwrap()`/`.expect()` so it isn't flagged by
/// `clippy::unwrap_used`/`expect_used` outside `#[cfg(test)]` (this helper
/// backs [`conformance_suite`], which the platforms' tests and debug builds
/// call through the `storage-conformance` feature, not just `cfg(test)`).
#[cfg(any(test, feature = "storage-conformance"))]
fn expect_ok<T>(result: Result<T, StorageError>) -> T {
    result.unwrap_or_else(|e| panic!("storage error: {e}"))
}

/// A conformance suite every [`Storage`] implementation must pass, shared by
/// this crate's own tests and the platforms' implementations.
#[cfg(any(test, feature = "storage-conformance"))]
pub fn conformance_suite(storage: &mut dyn Storage) {
    assert_eq!(
        expect_ok(storage.read("missing")),
        None,
        "reading an absent key returns None"
    );
    assert_eq!(expect_ok(storage.list()), Vec::<String>::new());

    expect_ok(storage.write("save-slot-1", "hello"));
    assert_eq!(
        expect_ok(storage.read("save-slot-1")),
        Some("hello".to_owned())
    );
    assert_eq!(expect_ok(storage.list()), ["save-slot-1"]);

    expect_ok(storage.write("save-slot-1", "overwritten"));
    assert_eq!(
        expect_ok(storage.read("save-slot-1")),
        Some("overwritten".to_owned())
    );

    expect_ok(storage.write("settings", "{}"));
    let mut keys = expect_ok(storage.list());
    keys.sort();
    assert_eq!(keys, ["save-slot-1", "settings"]);

    expect_ok(storage.delete("save-slot-1"));
    assert_eq!(expect_ok(storage.read("save-slot-1")), None);
    assert_eq!(expect_ok(storage.list()), ["settings"]);

    // Deleting an absent key is not an error.
    expect_ok(storage.delete("save-slot-1"));

    // check-text: not player text (storage keys)
    for bad in ["", "UPPER", "has space", "has/slash", &"x".repeat(65)] {
        assert!(
            matches!(storage.read(bad), Err(StorageError::InvalidKey(_))),
            "{bad:?} should be rejected"
        );
        assert!(matches!(
            storage.write(bad, "x"),
            Err(StorageError::InvalidKey(_))
        ));
        assert!(matches!(
            storage.delete(bad),
            Err(StorageError::InvalidKey(_))
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn memory_storage_passes_the_conformance_suite() {
        conformance_suite(&mut MemoryStorage::new());
    }

    /// A [`Storage`] that ignores every write and accepts any key: proves
    /// [`conformance_suite`] actually checks something, rather than being
    /// vacuously satisfied by a broken implementation.
    #[derive(Debug, Default)]
    struct Broken;

    impl Storage for Broken {
        fn read(&self, _key: &str) -> Result<Option<String>, StorageError> {
            Ok(None)
        }
        fn write(&mut self, _key: &str, _value: &str) -> Result<(), StorageError> {
            Ok(())
        }
        fn delete(&mut self, _key: &str) -> Result<(), StorageError> {
            Ok(())
        }
        fn list(&self) -> Result<Vec<String>, StorageError> {
            Ok(Vec::new())
        }
    }

    /// A storage that ignores writes and key validation must fail the suite.
    #[test]
    #[should_panic(expected = "assertion `left == right` failed")]
    fn conformance_suite_rejects_a_broken_storage() {
        conformance_suite(&mut Broken);
    }

    #[test]
    fn error_display() {
        assert_eq!(
            StorageError::InvalidKey("BAD".to_owned()).to_string(),
            "invalid storage key \"BAD\""
        );
        assert_eq!(
            StorageError::Backend("disk full".to_owned()).to_string(),
            "storage error: disk full"
        );
    }

    /// A definition of `^[a-z0-9_-]{1,64}$` independent of [`is_valid_key`],
    /// so the property test below checks the real implementation against a
    /// second one rather than against itself.
    fn matches_pattern(key: &str) -> bool {
        let chars: Vec<char> = key.chars().collect();
        (1..=MAX_KEY_LEN).contains(&chars.len())
            && chars
                .iter()
                .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '-'))
    }

    proptest! {
        #[test]
        fn accepted_iff_pattern_matches(key in ".{0,80}") {
            prop_assert_eq!(is_valid_key(&key), matches_pattern(&key));
        }

        #[test]
        fn generated_valid_keys_are_accepted(
            key in "[a-z0-9_-]{1,64}"
        ) {
            prop_assert!(is_valid_key(&key));
        }
    }
}
