//! Text handed to the SDK, which takes C strings (`NUL`-terminated).

use alloc::vec::Vec;
use core::fmt;

/// `text` as a C string: its bytes without any `NUL`, then one `NUL`.
pub fn c_string(text: &str) -> Vec<u8> {
    let mut bytes: Vec<u8> = text.bytes().filter(|&byte| byte != 0).collect();
    bytes.push(0);
    bytes
}

/// A C string of at most `N - 1` bytes written with [`fmt::Write`], without
/// allocating: the panic handler's message, which must not fail when the
/// heap is what ran out. Text that doesn't fit is cut at a character
/// boundary; `NUL`s are left out. `N` is at least 1.
#[derive(Debug)]
pub struct FixedText<const N: usize> {
    bytes: [u8; N],
    len: usize,
    /// Something didn't fit: nothing more is written, so the text is always
    /// a beginning of what was written.
    full: bool,
}

impl<const N: usize> FixedText<N> {
    /// Empty text.
    pub const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
            full: false,
        }
    }

    /// The text so far with its `NUL`, for the SDK.
    pub fn as_c_bytes(&self) -> &[u8] {
        &self.bytes[..=self.len]
    }
}

impl<const N: usize> Default for FixedText<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> fmt::Write for FixedText<N> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if self.full {
            return Ok(());
        }
        for ch in text.chars().filter(|&ch| ch != '\0') {
            let mut utf8 = [0; 4];
            let encoded = ch.encode_utf8(&mut utf8).as_bytes();
            // One byte stays free for the `NUL`.
            let end = self.len + encoded.len();
            if end + 1 > N {
                self.full = true;
                return Ok(());
            }
            self.bytes[self.len..end].copy_from_slice(encoded);
            self.len = end;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::fmt::Write as _;

    use super::*;

    #[test]
    fn a_c_string_has_one_nul_at_its_end() {
        assert_eq!(c_string("a\0b"), b"ab\0");
        assert_eq!(c_string(""), b"\0");
    }

    #[test]
    fn fixed_text_keeps_a_byte_for_its_nul() {
        let mut text = FixedText::<4>::new();
        let _ = write!(text, "abcd");
        assert_eq!(text.as_c_bytes(), b"abc\0");
    }

    #[test]
    fn fixed_text_is_cut_at_a_character_and_stays_cut() {
        let mut text = FixedText::<5>::new();
        let _ = write!(text, "ab\0é");
        assert_eq!(text.as_c_bytes(), "abé\0".as_bytes());
        let _ = write!(text, "é");
        let _ = write!(text, "c");
        assert_eq!(text.as_c_bytes(), "abé\0".as_bytes());
    }
}
