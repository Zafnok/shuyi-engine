//! A hash of a game's state, the same on every platform and every run: what
//! a replay compares step by step to find where two runs part, and what
//! two machines playing one game can exchange to find a desync.
//!
//! [`state_hash`] feeds any [`Serialize`] value to a 64-bit FNV-1a hash in
//! a fixed form of this file's own: every value is a tag byte and its
//! bytes, integers little-endian at a fixed width, strings and byte
//! strings with their length, sequences, maps and structs closed by an end
//! byte, variants by their index. So the form never depends on a
//! dependency's version or on the platform, and two different values of
//! one type never have the same form. Field and type names are left out:
//! renaming a field keeps a state's hash; reordering fields changes it.
//!
//! A state hashes the same each run only if it serializes the same: its
//! maps must be ordered (`BTreeMap`), never a hash map whose order changes
//! from run to run.

use alloc::string::{String, ToString};
use core::fmt;

use serde::Serialize;
use serde::ser::{self, Serializer};

/// The 64-bit FNV-1a offset basis.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// The 64-bit FNV-1a prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// The hash of a state ([`state_hash`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct StateHash(pub u64);

impl fmt::Display for StateHash {
    /// Sixteen lowercase hex digits.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// Why a state could not be hashed: its own `Serialize` refused, with this
/// message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashError(pub String);

impl fmt::Display for HashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the state could not be hashed: {}", self.0)
    }
}

impl ser::StdError for HashError {}

impl ser::Error for HashError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self(msg.to_string())
    }
}

/// The hash of `value` ([module docs](self)).
pub fn state_hash<T: Serialize + ?Sized>(value: &T) -> Result<StateHash, HashError> {
    let mut hasher = Hasher(FNV_OFFSET);
    value.serialize(&mut hasher)?;
    Ok(StateHash(hasher.0))
}

/// The tag bytes of the form, one per kind of value.
mod tag {
    pub const FALSE: u8 = 0x00;
    pub const TRUE: u8 = 0x01;
    pub const SIGNED: u8 = 0x02;
    pub const UNSIGNED: u8 = 0x03;
    pub const SIGNED_WIDE: u8 = 0x04;
    pub const UNSIGNED_WIDE: u8 = 0x05;
    pub const F32: u8 = 0x06;
    pub const F64: u8 = 0x07;
    pub const CHAR: u8 = 0x08;
    pub const STR: u8 = 0x09;
    pub const BYTES: u8 = 0x0a;
    pub const NONE: u8 = 0x0b;
    pub const SOME: u8 = 0x0c;
    pub const UNIT: u8 = 0x0d;
    pub const VARIANT: u8 = 0x0e;
    pub const SEQ: u8 = 0x0f;
    pub const MAP: u8 = 0x10;
    pub const STRUCT: u8 = 0x11;
    pub const END: u8 = 0x12;
}

/// The running FNV-1a hash, fed by serializing a value into it.
struct Hasher(u64);

impl Hasher {
    fn bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(FNV_PRIME);
        }
    }

    fn tag(&mut self, tag: u8) {
        self.bytes(&[tag]);
    }

    /// A length or an index: as a `u64`, so it is the same on 32 and 64
    /// bits.
    fn count(&mut self, n: usize) {
        self.bytes(&u64::try_from(n).unwrap_or(u64::MAX).to_le_bytes());
    }

    fn variant(&mut self, index: u32) {
        self.tag(tag::VARIANT);
        self.bytes(&index.to_le_bytes());
    }
}

impl Serializer for &mut Hasher {
    type Ok = ();
    type Error = HashError;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<(), HashError> {
        self.tag(if v { tag::TRUE } else { tag::FALSE });
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<(), HashError> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<(), HashError> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<(), HashError> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<(), HashError> {
        self.tag(tag::SIGNED);
        self.bytes(&v.to_le_bytes());
        Ok(())
    }

    fn serialize_i128(self, v: i128) -> Result<(), HashError> {
        self.tag(tag::SIGNED_WIDE);
        self.bytes(&v.to_le_bytes());
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<(), HashError> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<(), HashError> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<(), HashError> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<(), HashError> {
        self.tag(tag::UNSIGNED);
        self.bytes(&v.to_le_bytes());
        Ok(())
    }

    fn serialize_u128(self, v: u128) -> Result<(), HashError> {
        self.tag(tag::UNSIGNED_WIDE);
        self.bytes(&v.to_le_bytes());
        Ok(())
    }

    /// By its bits: a state should hold no floats (ADR-0002), but one that
    /// does still hashes the same for the same bits.
    fn serialize_f32(self, v: f32) -> Result<(), HashError> {
        self.tag(tag::F32);
        self.bytes(&v.to_bits().to_le_bytes());
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<(), HashError> {
        self.tag(tag::F64);
        self.bytes(&v.to_bits().to_le_bytes());
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<(), HashError> {
        self.tag(tag::CHAR);
        self.bytes(&u32::from(v).to_le_bytes());
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<(), HashError> {
        self.tag(tag::STR);
        self.count(v.len());
        self.bytes(v.as_bytes());
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<(), HashError> {
        self.tag(tag::BYTES);
        self.count(v.len());
        self.bytes(v);
        Ok(())
    }

    fn serialize_none(self) -> Result<(), HashError> {
        self.tag(tag::NONE);
        Ok(())
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), HashError> {
        self.tag(tag::SOME);
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<(), HashError> {
        self.tag(tag::UNIT);
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), HashError> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        index: u32,
        _variant: &'static str,
    ) -> Result<(), HashError> {
        self.variant(index);
        Ok(())
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<(), HashError> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<(), HashError> {
        self.variant(index);
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self, HashError> {
        self.tag(tag::SEQ);
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self, HashError> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<Self, HashError> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self, HashError> {
        self.variant(index);
        self.serialize_seq(Some(len))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self, HashError> {
        self.tag(tag::MAP);
        Ok(self)
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self, HashError> {
        self.tag(tag::STRUCT);
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self, HashError> {
        self.variant(index);
        self.tag(tag::STRUCT);
        Ok(self)
    }

    fn collect_str<T: fmt::Display + ?Sized>(self, value: &T) -> Result<(), HashError> {
        self.serialize_str(&value.to_string())
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

impl ser::SerializeSeq for &mut Hasher {
    type Ok = ();
    type Error = HashError;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), HashError> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), HashError> {
        self.tag(tag::END);
        Ok(())
    }
}

impl ser::SerializeTuple for &mut Hasher {
    type Ok = ();
    type Error = HashError;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), HashError> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), HashError> {
        self.tag(tag::END);
        Ok(())
    }
}

impl ser::SerializeTupleStruct for &mut Hasher {
    type Ok = ();
    type Error = HashError;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), HashError> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), HashError> {
        self.tag(tag::END);
        Ok(())
    }
}

impl ser::SerializeTupleVariant for &mut Hasher {
    type Ok = ();
    type Error = HashError;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), HashError> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), HashError> {
        self.tag(tag::END);
        Ok(())
    }
}

impl ser::SerializeMap for &mut Hasher {
    type Ok = ();
    type Error = HashError;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), HashError> {
        key.serialize(&mut **self)
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), HashError> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), HashError> {
        self.tag(tag::END);
        Ok(())
    }
}

impl ser::SerializeStruct for &mut Hasher {
    type Ok = ();
    type Error = HashError;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), HashError> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), HashError> {
        self.tag(tag::END);
        Ok(())
    }
}

impl ser::SerializeStructVariant for &mut Hasher {
    type Ok = ();
    type Error = HashError;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), HashError> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), HashError> {
        self.tag(tag::END);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
