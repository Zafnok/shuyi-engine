//! The word-wise FNV-1a hash the pack's checksum and a bundle's
//! fingerprint share: FNV-1a's constants over 64-bit words (the last few
//! bytes one at a time), each step followed by a shift that folds the
//! high bits into the low ones. Each step is a bijection of the hash so
//! far, so any one word changed changes it. Every integer is a `u64`,
//! whatever the platform.

/// The 64-bit FNV-1a offset basis: where a hash starts.
pub(crate) const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// The 64-bit FNV-1a prime.
const PRIME: u64 = 0x0100_0000_01b3;

/// A running hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Hasher(pub(crate) u64);

impl Hasher {
    /// A hash at FNV-1a's offset basis.
    pub(crate) const fn new() -> Self {
        Self(OFFSET)
    }

    /// Takes one word.
    pub(crate) fn word(&mut self, word: u64) {
        let hash = (self.0 ^ word).wrapping_mul(PRIME);
        self.0 = hash ^ (hash >> 32);
    }

    /// Takes `bytes`: each whole 8 bytes as a little-endian word, then the
    /// rest one byte a word. Not their length: a caller that needs it
    /// takes it too.
    pub(crate) fn bytes(&mut self, bytes: &[u8]) {
        let (words, rest) = bytes.as_chunks::<8>();
        for word in words {
            self.word(u64::from_le_bytes(*word));
        }
        for &b in rest {
            self.word(u64::from(b));
        }
    }

    /// Takes `bytes`' length, then `bytes`: so two runs of bytes taken one
    /// after the other never read as another two.
    pub(crate) fn sized(&mut self, bytes: &[u8]) {
        self.word(bytes.len() as u64);
        self.bytes(bytes);
    }
}
