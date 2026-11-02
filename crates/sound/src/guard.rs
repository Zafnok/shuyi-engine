#![cfg(feature = "std")]
//! The decoding libraries are other people's code reading whatever a file says,
//! and one that panics on a corrupt file must not take the game with it.

use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::error::Error;

/// What `decode` gives, with a panic in it as [`Error::Malformed`] of `format`.
/// A caller drops the decoder after that: it may be left halfway through
/// anything. A target that can't unwind stops at the panic.
pub(crate) fn guarded<T>(
    format: &'static str,
    decode: impl FnOnce() -> Result<T, Error>,
) -> Result<T, Error> {
    catch_unwind(AssertUnwindSafe(decode)).unwrap_or_else(|_| {
        Err(Error::Malformed {
            format,
            reason: "the decoder gave up on it".to_owned(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_is_an_error_and_an_answer_is_itself() {
        assert_eq!(guarded("WAV", || Ok(1)), Ok(1));
        assert_eq!(
            guarded::<u8>("WAV", || Err(Error::Channels(3))),
            Err(Error::Channels(3))
        );
        let panicked = guarded::<u8>("WAV", || panic!("a corrupt file"));
        assert!(
            matches!(panicked, Err(Error::Malformed { format: "WAV", .. })),
            "{panicked:?}"
        );
    }
}
