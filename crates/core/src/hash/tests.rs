//! The form of each kind of value, byte for byte, so it never changes
//! unnoticed; and what the hash is for: the same state, the same hash.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use serde::Serialize;

use super::*;
use crate::grid::{Dir, Grid, Pos};
use crate::rng::SimRng;

/// FNV-1a 64 of `bytes`, written out again here as the reference.
fn fnv(bytes: &[u8]) -> StateHash {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    StateHash(hash)
}

fn hash<T: Serialize + ?Sized>(value: &T) -> StateHash {
    state_hash(value).unwrap()
}

/// `tag` and then `rest`.
fn form(tag: u8, rest: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    out.extend_from_slice(rest);
    out
}

#[derive(Serialize)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Serialize)]
struct Unit;

#[derive(Serialize)]
struct Meters(u16);

#[derive(Serialize)]
struct Pair(u8, u8);

#[derive(Serialize)]
enum Shape {
    Empty,
    Round(u8),
    Box(u8, u8),
    Named { side: u8 },
}

#[test]
fn numbers_are_tagged_and_little_endian_at_one_width() {
    let signed = |v: i64| fnv(&form(tag::SIGNED, &v.to_le_bytes()));
    let unsigned = |v: u64| fnv(&form(tag::UNSIGNED, &v.to_le_bytes()));
    assert_eq!(hash(&-3i8), signed(-3));
    assert_eq!(hash(&-300i16), signed(-300));
    assert_eq!(hash(&-70_000i32), signed(-70_000));
    assert_eq!(hash(&i64::MIN), signed(i64::MIN));
    assert_eq!(hash(&7u8), unsigned(7));
    assert_eq!(hash(&700u16), unsigned(700));
    assert_eq!(hash(&70_000u32), unsigned(70_000));
    assert_eq!(hash(&u64::MAX), unsigned(u64::MAX));
    assert_eq!(
        hash(&-1i128),
        fnv(&form(tag::SIGNED_WIDE, &(-1i128).to_le_bytes()))
    );
    assert_eq!(
        hash(&u128::MAX),
        fnv(&form(tag::UNSIGNED_WIDE, &u128::MAX.to_le_bytes()))
    );
    // One width, so a value hashes the same whatever its type's width;
    // the sign is in the tag.
    assert_eq!(hash(&5u8), hash(&5u64));
    assert_ne!(hash(&5u8), hash(&5i8));
}

#[test]
fn floats_hash_by_their_bits() {
    assert_eq!(
        hash(&1.5f32),
        fnv(&form(tag::F32, &1.5f32.to_bits().to_le_bytes()))
    );
    assert_eq!(
        hash(&-2.25f64),
        fnv(&form(tag::F64, &(-2.25f64).to_bits().to_le_bytes()))
    );
    assert_ne!(hash(&0.0f64), hash(&-0.0f64));
}

#[test]
fn text_and_bytes_carry_their_length() {
    assert_eq!(hash(&true), fnv(&[tag::TRUE]));
    assert_eq!(hash(&false), fnv(&[tag::FALSE]));
    assert_eq!(
        hash(&'é'),
        fnv(&form(tag::CHAR, &u32::from('é').to_le_bytes()))
    );
    let mut ab = form(tag::STR, &2u64.to_le_bytes());
    ab.extend_from_slice(b"ab");
    assert_eq!(hash("ab"), fnv(&ab));
    assert_eq!(hash(&String::from("ab")), fnv(&ab));
    let mut bytes = form(tag::BYTES, &3u64.to_le_bytes());
    bytes.extend_from_slice(&[1, 2, 3]);
    assert_eq!(hash(&Bytes(&[1, 2, 3])), fnv(&bytes));
    // The length keeps one string from passing for two.
    assert_ne!(hash(&["ab"]), hash(&["a", "b"]));
}

/// A byte string, which serde serializes as bytes (not a sequence).
struct Bytes<'a>(&'a [u8]);

impl Serialize for Bytes<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(self.0)
    }
}

#[test]
fn options_units_and_newtypes() {
    assert_eq!(hash(&None::<u8>), fnv(&[tag::NONE]));
    let mut some = vec![tag::SOME, tag::UNSIGNED];
    some.extend_from_slice(&0u64.to_le_bytes());
    assert_eq!(hash(&Some(0u8)), fnv(&some));
    assert_eq!(hash(&()), fnv(&[tag::UNIT]));
    assert_eq!(hash(&Unit), fnv(&[tag::UNIT]));
    // A newtype is its value.
    assert_eq!(hash(&Meters(9)), hash(&9u16));
}

#[test]
fn sequences_maps_and_structs_are_closed() {
    let n = |v: u64| form(tag::UNSIGNED, &v.to_le_bytes());
    let closed = |open: u8, items: &[Vec<u8>]| {
        let mut out = vec![open];
        for item in items {
            out.extend_from_slice(item);
        }
        out.push(tag::END);
        out
    };
    assert_eq!(hash(&vec![1u8, 2]), fnv(&closed(tag::SEQ, &[n(1), n(2)])));
    assert_eq!(hash(&(1u8, 2u8)), fnv(&closed(tag::SEQ, &[n(1), n(2)])));
    assert_eq!(hash(&Pair(1, 2)), fnv(&closed(tag::SEQ, &[n(1), n(2)])));
    let map: BTreeMap<u8, u8> = [(1, 10), (2, 20)].into();
    assert_eq!(
        hash(&map),
        fnv(&closed(tag::MAP, &[n(1), n(10), n(2), n(20)]))
    );
    let point = Point { x: -1, y: 2 };
    let i = |v: i64| form(tag::SIGNED, &v.to_le_bytes());
    assert_eq!(hash(&point), fnv(&closed(tag::STRUCT, &[i(-1), i(2)])));
    // The end byte keeps `[[1], [2]]` from passing for `[[1, 2]]`.
    assert_ne!(hash(&vec![vec![1u8], vec![2]]), hash(&vec![vec![1u8, 2]]));
    assert_ne!(hash(&Vec::<u8>::new()), hash(&()));
}

#[test]
fn variants_are_their_index() {
    let variant = |index: u32| form(tag::VARIANT, &index.to_le_bytes());
    let n = |v: u64| form(tag::UNSIGNED, &v.to_le_bytes());
    assert_eq!(hash(&Shape::Empty), fnv(&variant(0)));
    assert_eq!(hash(&Shape::Round(4)), fnv(&[variant(1), n(4)].concat()));
    let tuple = [variant(2), vec![tag::SEQ], n(1), n(2), vec![tag::END]].concat();
    assert_eq!(hash(&Shape::Box(1, 2)), fnv(&tuple));
    let named = [variant(3), vec![tag::STRUCT], n(5), vec![tag::END]].concat();
    assert_eq!(hash(&Shape::Named { side: 5 }), fnv(&named));
    assert_ne!(hash(&Dir::Up), hash(&Dir::Down));
}

/// A value serialized through its `Display`, differently for people and
/// for machines.
struct Shown;

impl Serialize for Shown {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_u8(0)
        } else {
            serializer.collect_str("shown")
        }
    }
}

#[test]
fn the_form_is_for_machines_and_displayed_values_are_text() {
    assert_eq!(hash(&Shown), hash("shown"));
}

/// A value whose `Serialize` refuses.
struct Refuses;

impl Serialize for Refuses {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(ser::Error::custom("no"))
    }
}

#[test]
fn a_refusal_is_an_error_naming_it() {
    let error = state_hash(&Refuses).unwrap_err();
    assert_eq!(error, HashError("no".into()));
    assert_eq!(
        alloc::format!("{error}"),
        "the state could not be hashed: no"
    );
    assert!(state_hash(&vec![Some(Refuses)]).is_err());
}

#[test]
fn a_hash_shows_as_sixteen_hex_digits() {
    assert_eq!(alloc::format!("{}", StateHash(0xab)), "00000000000000ab");
    assert_eq!(
        alloc::format!("{}", StateHash(u64::MAX)),
        "ffffffffffffffff"
    );
}

#[test]
fn the_same_state_has_the_same_hash_and_a_step_changes_it() {
    let grid = Grid::filled(3, 2, 0u16);
    let mut rng = SimRng::new(7);
    let state = |rng: &SimRng| (grid.clone(), Pos::new(1, 1), rng.clone());
    let before = hash(&state(&rng));
    assert_eq!(hash(&state(&rng.clone())), before);
    rng.roll_percent();
    assert_ne!(hash(&state(&rng)), before);
}
