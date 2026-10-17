//! [`WebStorage`]: the browser's `localStorage` via `quad_storage`,
//! namespaced under a prefix ([`super::web_prefix`]) so the game doesn't
//! collide with other pages sharing the origin.

use std::sync::PoisonError;

use shuyi_ui::storage::{Storage, StorageError, is_valid_key};

/// A [`Storage`] backed by `quad_storage::STORAGE` (`localStorage`). Cheap
/// to construct: the real state lives in `quad_storage`'s global.
#[derive(Debug)]
pub struct WebStorage {
    /// Every key is stored under this prefix.
    prefix: String,
}

impl WebStorage {
    /// A handle to the browser's `localStorage`, for the game whose storage
    /// is called `name`.
    pub fn new(name: &str) -> Self {
        Self {
            prefix: super::web_prefix(name),
        }
    }

    fn prefixed(&self, key: &str) -> String {
        format!("{}{key}", self.prefix)
    }
}

impl Storage for WebStorage {
    fn read(&self, key: &str) -> Result<Option<String>, StorageError> {
        check_key(key)?;
        let storage = quad_storage::STORAGE.lock().map_err(poisoned)?;
        Ok(storage.get(&self.prefixed(key)))
    }

    fn write(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        check_key(key)?;
        let mut storage = quad_storage::STORAGE.lock().map_err(poisoned)?;
        storage.set(&self.prefixed(key), value);
        Ok(())
    }

    fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        check_key(key)?;
        let mut storage = quad_storage::STORAGE.lock().map_err(poisoned)?;
        storage.remove(&self.prefixed(key));
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>, StorageError> {
        let storage = quad_storage::STORAGE.lock().map_err(poisoned)?;
        let keys = (0..storage.len())
            .filter_map(|i| storage.key(i))
            .filter_map(|key| key.strip_prefix(self.prefix.as_str()).map(str::to_owned))
            .collect();
        Ok(keys)
    }
}

fn check_key(key: &str) -> Result<(), StorageError> {
    if is_valid_key(key) {
        Ok(())
    } else {
        Err(StorageError::InvalidKey(key.to_owned()))
    }
}

fn poisoned<T>(_: PoisonError<T>) -> StorageError {
    StorageError::Backend("localStorage lock poisoned".to_owned())
}
