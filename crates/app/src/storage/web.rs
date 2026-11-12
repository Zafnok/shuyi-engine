//! [`WebStorage`]: the browser's `localStorage`, through the `shuyi_storage`
//! miniquad plugin in `crates/cli/web/storage.js` (our own code, loaded by
//! `crates/cli/web/index.html`), namespaced under a prefix
//! ([`super::web_prefix`]) so the game doesn't collide with other pages
//! sharing the origin. The rules (the prefix, the keys, the reads that ask
//! again with a bigger buffer, the errors) are in
//! [`super::local_storage`], which the tests run over a fake.

use shuyi_ui::storage::{Storage, StorageError};

use super::local_storage::{LocalStorage, Plugin};

/// The plugin's version, which the JS loader compares with the `version`
/// in `crates/cli/web/storage.js` (a `shuyi-cli` test keeps them equal).
/// Bump both when the functions below change.
const PLUGIN_VERSION: u32 = 1;

// Declaring the functions `crates/cli/web/storage.js` provides, and
// exporting the version the loader asks for. They take numbers, and
// pointers with their lengths in bytes that only `Js` below passes, each
// from a slice it holds for the call: the JS reads `key` and `value` and
// writes at most `capacity` bytes at `buffer` before it returns, and keeps
// none. It catches whatever `localStorage` throws and answers a code
// instead, so calling them is safe.
#[allow(unsafe_code)]
unsafe extern "C" {
    /// `localStorage.getItem`: the value's length in bytes, written at
    /// `buffer` if it fits, or a code below 0.
    safe fn shuyi_storage_get(
        key: *const u8,
        key_length: u32,
        buffer: *mut u8,
        capacity: u32,
    ) -> i32;
    /// `localStorage.setItem`: 0 once it is stored, or a code below 0.
    safe fn shuyi_storage_set(
        key: *const u8,
        key_length: u32,
        value: *const u8,
        value_length: u32,
    ) -> i32;
    /// `localStorage.removeItem`: 0 once it is gone, or a code below 0.
    safe fn shuyi_storage_remove(key: *const u8, key_length: u32) -> i32;
    /// `localStorage.length`, or a code below 0.
    safe fn shuyi_storage_len() -> i32;
    /// `localStorage.key`: the key's length in bytes, written at `buffer`
    /// if it fits, or a code below 0.
    safe fn shuyi_storage_key(index: u32, buffer: *mut u8, capacity: u32) -> i32;
}

/// What the JS loader calls to check the plugin's version.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn shuyi_storage_crate_version() -> u32 {
    PLUGIN_VERSION
}

/// A slice's length for the plugin. Longer than `u32::MAX` bytes is longer
/// than any browser's `localStorage`, which then answers it is full.
fn length(bytes: &[u8]) -> u32 {
    u32::try_from(bytes.len()).unwrap_or(u32::MAX)
}

/// The plugin's functions, each passed slices it holds for the call.
#[derive(Debug)]
struct Js;

impl Plugin for Js {
    fn get(&self, key: &[u8], buffer: &mut [u8]) -> i32 {
        let capacity = length(buffer);
        shuyi_storage_get(key.as_ptr(), length(key), buffer.as_mut_ptr(), capacity)
    }

    fn set(&self, key: &[u8], value: &[u8]) -> i32 {
        if u32::try_from(value.len()).is_err() {
            return super::local_storage::FULL;
        }
        shuyi_storage_set(key.as_ptr(), length(key), value.as_ptr(), length(value))
    }

    fn remove(&self, key: &[u8]) -> i32 {
        shuyi_storage_remove(key.as_ptr(), length(key))
    }

    fn len(&self) -> i32 {
        shuyi_storage_len()
    }

    fn key(&self, index: u32, buffer: &mut [u8]) -> i32 {
        let capacity = length(buffer);
        shuyi_storage_key(index, buffer.as_mut_ptr(), capacity)
    }
}

/// A [`Storage`] in the browser's `localStorage`. Cheap to construct: the
/// state lives in the browser.
#[derive(Debug)]
pub struct WebStorage {
    inner: LocalStorage<Js>,
}

impl WebStorage {
    /// A handle to the browser's `localStorage`, for the game whose storage
    /// is called `name`.
    pub fn new(name: &str) -> Self {
        Self {
            inner: LocalStorage::new(Js, name),
        }
    }
}

impl Storage for WebStorage {
    fn read(&self, key: &str) -> Result<Option<String>, StorageError> {
        self.inner.read(key)
    }

    fn write(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        self.inner.write(key, value)
    }

    fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        self.inner.delete(key)
    }

    fn list(&self) -> Result<Vec<String>, StorageError> {
        self.inner.list()
    }
}
