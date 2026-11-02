//! A session record kept as a file, as a test.
#![cfg(feature = "std")]

use std::ffi::OsStr;

use shuyi_ui::GameData;

use super::{bless_session, check_session};
use crate::app::Start;

/// Whether `SHUYI_BLESS`, set to `value` or not, asks to write the file.
fn blesses(value: Option<&OsStr>) -> bool {
    value.is_some_and(|value| value == "1")
}

/// A test that the session record at `path` plays from `start` with no
/// difference ([`check_session`]). With the environment variable
/// `SHUYI_BLESS=1` it writes the file again instead ([`bless_session`]).
///
/// # Panics
///
/// If the file can't be read or written, or the record doesn't play.
#[track_caller]
pub fn assert_session<G: GameData>(start: Start<G>, path: impl AsRef<std::path::Path>) {
    let path = path.as_ref();
    let shown = path.display();
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{shown}: {e}"));
    if blesses(std::env::var_os("SHUYI_BLESS").as_deref()) {
        let blessed = bless_session(start, &text).unwrap_or_else(|e| panic!("{shown}: {e}"));
        std::fs::write(path, blessed).unwrap_or_else(|e| panic!("{shown}: {e}"));
    } else if let Err(e) = check_session(start, &text) {
        panic!("{shown}: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_shuyi_bless_of_1_writes_the_file() {
        assert!(blesses(Some(OsStr::new("1"))));
        assert!(!blesses(Some(OsStr::new("0"))));
        assert!(!blesses(None));
    }
}
