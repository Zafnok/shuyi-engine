//! [`LocalStorage`]: a [`Storage`] over the five calls of the browser's
//! `localStorage` that `crates/cli/web/storage.js` makes ([`Plugin`]),
//! namespaced under a prefix ([`super::web_prefix`]) so the game doesn't
//! collide with other pages sharing the origin. The web build runs it over
//! the plugin ([`super::WebStorage`]); the tests run it over a fake.
//!
//! Strings cross as UTF-8 bytes. A string the plugin gives back is written
//! into a buffer this side lends; the plugin answers its length in bytes,
//! and a length larger than the buffer means "lend one this big", so the
//! read is asked again ([`read_into`]).

use shuyi_ui::storage::{Storage, StorageError, is_valid_key};

/// No value under the key (`get`), or no key at the index (`key`). The codes
/// equal the ones in `crates/cli/web/storage.js`.
pub(crate) const ABSENT: i32 = -1;
/// The page may not use `localStorage`: storage turned off, or a sandboxed
/// frame.
pub(crate) const BLOCKED: i32 = -2;
/// `localStorage` is full.
pub(crate) const FULL: i32 = -3;
/// `localStorage` threw something else.
pub(crate) const FAILED: i32 = -4;

/// The buffer a read lends first, in bytes: most keys and small saves fit,
/// and a bigger one is lent once the plugin says how big.
const FIRST_BUFFER: usize = 256;

/// How many times a read is asked before it gives up. The plugin answers
/// the length it needs, so the second ask fits unless the value changed in
/// between, which nothing on the page's one thread can do.
const ASKS: usize = 3;

/// The calls of `storage.js`. Each answers a code below 0 when it fails
/// ([`ABSENT`], [`BLOCKED`], [`FULL`], [`FAILED`]). A read answers the
/// length of what it gives, in bytes, and wrote it into `buffer` only when
/// it fits.
pub(crate) trait Plugin: std::fmt::Debug {
    /// `localStorage.getItem(key)`, into `buffer`.
    fn get(&self, key: &[u8], buffer: &mut [u8]) -> i32;
    /// `localStorage.setItem(key, value)`; 0 once it is stored.
    fn set(&self, key: &[u8], value: &[u8]) -> i32;
    /// `localStorage.removeItem(key)`; 0 once it is gone.
    fn remove(&self, key: &[u8]) -> i32;
    /// `localStorage.length`: every key of the origin, other games' too.
    fn len(&self) -> i32;
    /// `localStorage.key(index)`, into `buffer`.
    fn key(&self, index: u32, buffer: &mut [u8]) -> i32;
}

/// A [`Storage`] in `localStorage`, through `plugin`, under one game's
/// prefix.
#[derive(Debug)]
pub(crate) struct LocalStorage<P> {
    /// The calls into the browser.
    plugin: P,
    /// Every key is stored under this prefix.
    prefix: String,
}

impl<P: Plugin> LocalStorage<P> {
    /// The storage called `name`, through `plugin`.
    pub(crate) fn new(plugin: P, name: &str) -> Self {
        Self {
            plugin,
            prefix: super::web_prefix(name),
        }
    }

    fn prefixed(&self, key: &str) -> String {
        format!("{}{key}", self.prefix)
    }
}

impl<P: Plugin> Storage for LocalStorage<P> {
    fn read(&self, key: &str) -> Result<Option<String>, StorageError> {
        check_key(key)?;
        let key = self.prefixed(key);
        read_into(|buffer| self.plugin.get(key.as_bytes(), buffer))
    }

    fn write(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        check_key(key)?;
        done(
            self.plugin
                .set(self.prefixed(key).as_bytes(), value.as_bytes()),
        )
    }

    fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        check_key(key)?;
        done(self.plugin.remove(self.prefixed(key).as_bytes()))
    }

    fn list(&self) -> Result<Vec<String>, StorageError> {
        let len = self.plugin.len();
        let len = u32::try_from(len).map_err(|_| error(len))?;
        let mut keys = Vec::new();
        for index in 0..len {
            let Some(key) = read_into(|buffer| self.plugin.key(index, buffer))? else {
                continue;
            };
            if let Some(ours) = key.strip_prefix(self.prefix.as_str()) {
                keys.push(ours.to_owned());
            }
        }
        Ok(keys)
    }
}

/// What `ask` gives, lending it a bigger buffer while it answers a length
/// larger than the one it got: `None` for [`ABSENT`], an error for any other
/// code below 0.
pub(crate) fn read_into(
    mut ask: impl FnMut(&mut [u8]) -> i32,
) -> Result<Option<String>, StorageError> {
    let mut buffer = vec![0; FIRST_BUFFER];
    for _ in 0..ASKS {
        let code = ask(&mut buffer);
        if code == ABSENT {
            return Ok(None);
        }
        let len = usize::try_from(code).map_err(|_| error(code))?;
        if len <= buffer.len() {
            buffer.truncate(len);
            return String::from_utf8(buffer).map(Some).map_err(|_| {
                StorageError::Backend("localStorage gave text that isn't UTF-8".to_owned())
            });
        }
        buffer.resize(len, 0);
    }
    Err(StorageError::Backend(
        "localStorage kept growing the value it gave".to_owned(),
    ))
}

/// `Ok` for a write's 0, the error of any other code.
fn done(code: i32) -> Result<(), StorageError> {
    if code == 0 { Ok(()) } else { Err(error(code)) }
}

/// The error for a code below 0 (or one the plugin should never answer).
pub(crate) fn error(code: i32) -> StorageError {
    let message = match code {
        BLOCKED => "localStorage is turned off or blocked in this browser".to_owned(),
        FULL => "localStorage is full".to_owned(),
        FAILED => "localStorage failed".to_owned(),
        other => format!("localStorage answered the unknown code {other}"),
    };
    StorageError::Backend(message)
}

fn check_key(key: &str) -> Result<(), StorageError> {
    if is_valid_key(key) {
        Ok(())
    } else {
        Err(StorageError::InvalidKey(key.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use shuyi_ui::storage::conformance_suite;

    use super::*;

    /// `localStorage` in memory, kept as the browser keeps it: keys in the
    /// order they were first set. `fail` makes every call answer that code.
    #[derive(Debug, Default)]
    struct Fake {
        items: RefCell<Vec<(String, String)>>,
        fail: Option<i32>,
        /// How many reads were asked, retries counted.
        asks: Cell<usize>,
    }

    impl Fake {
        fn failing(code: i32) -> Self {
            Self {
                fail: Some(code),
                ..Self::default()
            }
        }

        fn holding(items: &[(&str, &str)]) -> Self {
            let fake = Self::default();
            for &(key, value) in items {
                fake.set(key.as_bytes(), value.as_bytes());
            }
            fake
        }
    }

    /// Writes `text` into `buffer` if it fits, as `give` in `storage.js`
    /// does; answers its length either way.
    fn give(text: &str, buffer: &mut [u8]) -> i32 {
        let bytes = text.as_bytes();
        if let Some(to) = buffer.get_mut(..bytes.len()) {
            to.copy_from_slice(bytes);
        }
        i32::try_from(bytes.len()).unwrap()
    }

    fn text(bytes: &[u8]) -> String {
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    impl Plugin for Fake {
        fn get(&self, key: &[u8], buffer: &mut [u8]) -> i32 {
            self.asks.set(self.asks.get() + 1);
            if let Some(code) = self.fail {
                return code;
            }
            let key = text(key);
            let items = self.items.borrow();
            match items.iter().find(|(k, _)| *k == key) {
                Some((_, value)) => give(value, buffer),
                None => ABSENT,
            }
        }

        fn set(&self, key: &[u8], value: &[u8]) -> i32 {
            if let Some(code) = self.fail {
                return code;
            }
            let (key, value) = (text(key), text(value));
            let mut items = self.items.borrow_mut();
            match items.iter_mut().find(|(k, _)| *k == key) {
                Some(item) => item.1 = value,
                None => items.push((key, value)),
            }
            0
        }

        fn remove(&self, key: &[u8]) -> i32 {
            if let Some(code) = self.fail {
                return code;
            }
            let key = text(key);
            self.items.borrow_mut().retain(|(k, _)| *k != key);
            0
        }

        fn len(&self) -> i32 {
            self.fail
                .unwrap_or_else(|| i32::try_from(self.items.borrow().len()).unwrap())
        }

        fn key(&self, index: u32, buffer: &mut [u8]) -> i32 {
            self.asks.set(self.asks.get() + 1);
            if let Some(code) = self.fail {
                return code;
            }
            let items = self.items.borrow();
            match items.get(usize::try_from(index).unwrap()) {
                Some((key, _)) => give(key, buffer),
                None => ABSENT,
            }
        }
    }

    #[test]
    fn local_storage_passes_the_conformance_suite() {
        conformance_suite(&mut LocalStorage::new(Fake::default(), "the-game"));
    }

    #[test]
    fn a_value_larger_than_the_first_buffer_is_asked_again_once() {
        let big = "é".repeat(FIRST_BUFFER); // 2 bytes each: twice the buffer.
        let mut storage = LocalStorage::new(Fake::default(), "the-game");
        storage.write("save", &big).unwrap();
        assert_eq!(storage.read("save").unwrap().as_deref(), Some(big.as_str()));
        assert_eq!(storage.plugin.asks.get(), 2);
    }

    #[test]
    fn a_value_that_fits_is_asked_once() {
        let mut storage = LocalStorage::new(Fake::default(), "the-game");
        let exact = "-".repeat(FIRST_BUFFER);
        storage.write("save", &exact).unwrap();
        assert_eq!(storage.read("save").unwrap(), Some(exact));
        assert_eq!(storage.plugin.asks.get(), 1);
    }

    #[test]
    fn a_long_key_in_the_list_is_asked_again() {
        let other = "-".repeat(FIRST_BUFFER + 1);
        let fake = Fake::holding(&[(&other, "1"), ("the-game/save", "2")]);
        let storage = LocalStorage::new(fake, "the-game");
        assert_eq!(storage.list().unwrap(), ["save"]);
        assert_eq!(storage.plugin.asks.get(), 3);
    }

    #[test]
    fn keys_go_under_the_prefix_as_they_always_did() {
        // Saves made through quad-storage stored the prefixed key as it is,
        // and still load.
        let fake = Fake::holding(&[
            ("the-game/save-slot-1", "old save"),
            ("another-game/save-slot-1", "theirs"),
            ("unrelated", "other"),
        ]);
        let mut storage = LocalStorage::new(fake, "the-game");
        assert_eq!(
            storage.read("save-slot-1").unwrap().as_deref(),
            Some("old save")
        );
        assert_eq!(storage.list().unwrap(), ["save-slot-1"]);
        storage.write("save-slot-2", "new").unwrap();
        let items = storage.plugin.items.borrow();
        assert!(
            items
                .iter()
                .any(|(k, v)| k == "the-game/save-slot-2" && v == "new")
        );
    }

    #[test]
    fn each_code_becomes_its_error() {
        for (code, message) in [
            (
                BLOCKED,
                "localStorage is turned off or blocked in this browser",
            ),
            (FULL, "localStorage is full"),
            (FAILED, "localStorage failed"),
            (-9, "localStorage answered the unknown code -9"),
        ] {
            let mut storage = LocalStorage::new(Fake::failing(code), "the-game");
            let backend = |result: Result<(), StorageError>| match result {
                Err(StorageError::Backend(got)) => got,
                other => panic!("{code}: {other:?}"),
            };
            assert_eq!(backend(storage.write("save", "value")), message);
            assert_eq!(backend(storage.delete("save")), message);
            assert_eq!(backend(storage.read("save").map(drop)), message);
            assert_eq!(backend(storage.list().map(drop)), message);
        }
    }

    #[test]
    fn a_bad_key_never_reaches_the_browser() {
        let mut storage = LocalStorage::new(Fake::failing(FAILED), "the-game");
        assert!(matches!(
            storage.write("Bad Key", "value"),
            Err(StorageError::InvalidKey(_))
        ));
        assert!(matches!(
            storage.read("../up"),
            Err(StorageError::InvalidKey(_))
        ));
        assert_eq!(storage.plugin.asks.get(), 0);
    }

    #[test]
    fn a_read_that_keeps_growing_gives_up() {
        let mut asks = 0;
        let result = read_into(|buffer| {
            asks += 1;
            i32::try_from(buffer.len() + 1).unwrap()
        });
        assert!(
            matches!(result, Err(StorageError::Backend(_))),
            "{result:?}"
        );
        assert_eq!(asks, ASKS);
    }

    #[test]
    fn bytes_that_are_not_utf8_are_an_error() {
        let result = read_into(|buffer| {
            buffer[0] = 0xff;
            1
        });
        assert!(
            matches!(result, Err(StorageError::Backend(_))),
            "{result:?}"
        );
    }

    #[test]
    fn an_empty_value_reads_back_empty() {
        let mut storage = LocalStorage::new(Fake::default(), "the-game");
        storage.write("save", "").unwrap();
        assert_eq!(storage.read("save").unwrap().as_deref(), Some(""));
    }
}
