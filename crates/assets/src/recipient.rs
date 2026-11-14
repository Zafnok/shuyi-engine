//! A recipient's code (ADR-0118): the short code a closed build (a press,
//! playtest or publisher copy) carries in three marks, so a copy that leaks
//! names who it was given to. It is a deterrent with circumstantial
//! evidence, never proof: two recipients comparing their copies find and
//! remove every mark, and a repack or a re-encode drops some of them.
//!
//! A [`Code`] is 40 bits: 32 bits chosen for the recipient and a CRC-8 of
//! them, so a mark read wrong is refused, not taken for another recipient's.
//! It is written as 8 characters of Crockford's base 32 (`7K2Q9MXA`).
//!
//! The marks, each holding the same code:
//!
//! - the program's **field** ([`FIELD`]): 32 bytes a program built with
//!   `shuyi-app` reserves, which [`field_code`] reads and [`write_field`]
//!   writes, and which a game reads with `shuyi_app::recipient`;
//! - the **pack's layout**: the order of its files' bytes and the bytes
//!   between them (`pack::Raw::write_marked`, `pack::Raw::mark`);
//! - one **picture**'s lowest bits (`image::mark`).
//!
//! The public build has none of them: its field holds no code, its pack is
//! laid in path order with nothing between files, and its picture is the
//! game's own.
use crate::prelude::*;

use core::fmt;
use core::str::FromStr;

/// How many bytes a code takes.
pub const CODE_BYTES: usize = 5;

/// A recipient's code: 32 bits and their CRC-8. The bits are never all zero,
/// which is no code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Code(u32);

/// Crockford's base 32: no I, L, O or U.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// The CRC-8 of `bytes`, polynomial `x⁸ + x² + x + 1` (`0x07`): every error
/// in one burst of up to 8 bits is caught.
fn crc8(bytes: &[u8]) -> u8 {
    let mut crc = 0u8;
    for &byte in bytes {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 0x80 == 0 {
                crc << 1
            } else {
                (crc << 1) ^ 0x07
            };
        }
    }
    crc
}

impl Code {
    /// The code of `bits`, if they aren't zero.
    pub const fn new(bits: u32) -> Option<Self> {
        if bits == 0 { None } else { Some(Self(bits)) }
    }

    /// Its 32 bits.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Its bytes: the bits, big-endian, then their CRC-8.
    pub fn to_bytes(self) -> [u8; CODE_BYTES] {
        let [a, b, c, d] = self.0.to_be_bytes();
        [a, b, c, d, crc8(&[a, b, c, d])]
    }

    /// The code `bytes` hold, if their CRC-8 is right and the bits aren't
    /// zero.
    pub fn from_bytes(bytes: [u8; CODE_BYTES]) -> Option<Self> {
        let [a, b, c, d, check] = bytes;
        if crc8(&[a, b, c, d]) != check {
            return None;
        }
        Self::new(u32::from_be_bytes([a, b, c, d]))
    }

    /// Its 40 bits, the first byte's highest first.
    pub fn bit(self, at: usize) -> bool {
        let bytes = self.to_bytes();
        bytes[at / 8] >> (7 - at % 8) & 1 == 1
    }

    /// The code of 40 bits (`bits[0]` the highest of the first byte), if they
    /// hold one.
    pub fn from_bits(bits: &[bool; CODE_BYTES * 8]) -> Option<Self> {
        let mut bytes = [0u8; CODE_BYTES];
        for (at, &bit) in bits.iter().enumerate() {
            bytes[at / 8] |= u8::from(bit) << (7 - at % 8);
        }
        Self::from_bytes(bytes)
    }
}

impl fmt::Display for Code {
    /// The 40 bits as 8 characters of Crockford's base 32.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let all = self
            .to_bytes()
            .iter()
            .fold(0u64, |all, &byte| all << 8 | u64::from(byte));
        for at in (0..8).rev() {
            let digit = (all >> (at * 5)) & 31;
            // A digit is under 32.
            let c = ALPHABET[usize::try_from(digit).unwrap_or(0)];
            write!(f, "{}", char::from(c))?;
        }
        Ok(())
    }
}

/// Why text isn't a code.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CodeError {
    /// It hasn't 8 characters of base 32 (hyphens aside).
    #[error("a recipient's code is 8 characters of Crockford's base 32, such as 7K2Q9MXA")]
    Shape,
    /// Its check doesn't match: a character is wrong.
    #[error("the code's check doesn't match: a character is wrong")]
    Check,
}

impl FromStr for Code {
    type Err = CodeError;

    /// Reads 8 characters of Crockford's base 32, in either case, hyphens
    /// ignored, `I` and `L` read as `1` and `O` as `0`.
    fn from_str(text: &str) -> Result<Self, CodeError> {
        let mut all = 0u64;
        let mut count = 0;
        for c in text.chars().filter(|&c| c != '-') {
            let c = match c.to_ascii_uppercase() {
                'I' | 'L' => '1',
                'O' => '0',
                c => c,
            };
            let digit = ALPHABET
                .iter()
                .position(|&a| char::from(a) == c)
                .ok_or(CodeError::Shape)?;
            all = all << 5 | digit as u64;
            count += 1;
        }
        if count != 8 {
            return Err(CodeError::Shape);
        }
        let bytes = all.to_be_bytes();
        let mut five = [0u8; CODE_BYTES];
        five.copy_from_slice(&bytes[3..]);
        Self::from_bytes(five).ok_or(CodeError::Check)
    }
}

/// How many bytes the program's field takes.
pub const FIELD_BYTES: usize = 32;

/// The bytes the field starts with, which [`write_field`] looks for: none of
/// them text (each has its highest bit set), so a program's strings don't
/// show it.
pub const FIELD_MAGIC: [u8; 16] = [
    0x8f, 0x9d, 0xc3, 0xda, 0x82, 0xe7, 0x94, 0xeb, 0xb1, 0x8e, 0xfc, 0xd8, 0xb3, 0xa6, 0xcf, 0xf2,
];

/// What the field's code bytes are kept under: each byte of the slot is the
/// one meant, exclusive-or this. No zero among them, so an optimizer that
/// drops runs of zero bytes from a program's data (`wasm-opt`) keeps the
/// slot whole, and no text: each has its highest bit set.
const FIELD_KEY: [u8; 16] = [
    0xd4, 0x97, 0xab, 0xe1, 0xdf, 0x86, 0xc9, 0xba, 0xab, 0xf1, 0xee, 0x95, 0x9c, 0xc8, 0xb7, 0xe3,
];

/// The field of a program that holds no code: [`FIELD_MAGIC`], then a slot
/// whose meant bytes are all zero. `shuyi-app` reserves it; only an
/// overlay changes it.
pub const FIELD: [u8; FIELD_BYTES] = {
    let mut field = [0u8; FIELD_BYTES];
    let mut at = 0;
    while at < 16 {
        field[at] = FIELD_MAGIC[at];
        field[16 + at] = FIELD_KEY[at];
        at += 1;
    }
    field
};

/// The field holding `code`: its bytes twice, then zeros, each under the
/// key.
pub fn field_of(code: Option<Code>) -> [u8; FIELD_BYTES] {
    let mut field = FIELD;
    if let Some(code) = code {
        let bytes = code.to_bytes();
        for at in 0..CODE_BYTES {
            field[16 + at] = bytes[at] ^ FIELD_KEY[at];
            field[16 + CODE_BYTES + at] = bytes[at] ^ FIELD_KEY[CODE_BYTES + at];
        }
    }
    field
}

/// What a field holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldRead {
    /// No code: the field of a public build.
    Empty,
    /// A code, held twice and the two the same.
    Code(Code),
    /// Neither: the slot was changed.
    Broken,
}

/// What the field `field` holds.
pub fn field_code(field: &[u8; FIELD_BYTES]) -> FieldRead {
    if field[..16] != FIELD_MAGIC {
        return FieldRead::Broken;
    }
    let mut slot = [0u8; 16];
    slot.copy_from_slice(&field[16..]);
    slot_code(&slot)
}

/// What the slot of a field holds: its last 16 bytes. A program reads its
/// own field's slot with this, never comparing [`FIELD_MAGIC`], so the
/// magic is in the program only once, where the field is.
pub fn slot_code(slot: &[u8; 16]) -> FieldRead {
    let mut meant = [0u8; 16];
    for at in 0..16 {
        meant[at] = slot[at] ^ FIELD_KEY[at];
    }
    if meant == [0; 16] {
        return FieldRead::Empty;
    }
    let mut first = [0u8; CODE_BYTES];
    first.copy_from_slice(&meant[..CODE_BYTES]);
    let rest_clear = meant[2 * CODE_BYTES..].iter().all(|&b| b == 0);
    match Code::from_bytes(first) {
        Some(code) if rest_clear && meant[CODE_BYTES..2 * CODE_BYTES] == first => {
            FieldRead::Code(code)
        }
        _ => FieldRead::Broken,
    }
}

/// Where each field starts in `program`: every place [`FIELD_MAGIC`] is (a
/// universal macOS program has one in each slice).
pub fn find_fields(program: &[u8]) -> Vec<usize> {
    program
        .windows(FIELD_MAGIC.len())
        .enumerate()
        .filter(|(_, window)| *window == FIELD_MAGIC)
        .map(|(at, _)| at)
        .filter(|&at| at + FIELD_BYTES <= program.len())
        .collect()
}

/// What the fields of a program hold, by where each starts.
pub fn read_fields(program: &[u8]) -> Vec<(usize, FieldRead)> {
    find_fields(program)
        .into_iter()
        .map(|at| {
            let mut field = [0u8; FIELD_BYTES];
            field.copy_from_slice(&program[at..at + FIELD_BYTES]);
            (at, field_code(&field))
        })
        .collect()
}

/// Writes `code` into every field of `program` that holds no code or a
/// code, and changes no other byte: how many it wrote, none if the program
/// has no field. A place the magic bytes happen to be that isn't a field is
/// left as it is.
pub fn write_field(program: &mut [u8], code: Code) -> usize {
    let field = field_of(Some(code));
    let places: Vec<usize> = read_fields(program)
        .into_iter()
        .filter(|(_, read)| !matches!(read, FieldRead::Broken))
        .map(|(at, _)| at)
        .collect();
    for &at in &places {
        program[at..at + FIELD_BYTES].copy_from_slice(&field);
    }
    places.len()
}

/// A stream of numbers from `seed` (SplitMix64): how a code lays a pack out,
/// the same on every machine.
pub(crate) struct Stream(u64);

impl Stream {
    /// The stream of `seed`.
    pub(crate) const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Its next number.
    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A number under `n` (`n` > 0).
    pub(crate) fn below(&mut self, n: usize) -> usize {
        // `n` fits in 64 bits, and the remainder is under it.
        usize::try_from(self.next() % n as u64).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests;
