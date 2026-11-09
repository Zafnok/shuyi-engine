//! The plain values a script gives back: [`Value`].

use std::collections::BTreeMap;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

/// What a script gave, when a game reads it without a type of its own:
/// numbers, booleans, strings, and lists and maps of them. A map's keys are
/// in order, so the same value always prints and compares the same.
///
/// ```
/// use shuyi_script::{Runtime, Value};
///
/// let runtime = Runtime::default();
/// let script = runtime
///     .compile("door.rhai", "#{ open: view.switches >= 3, left: 3 - view.switches }")
///     .unwrap_or_else(|e| panic!("{e}"));
/// let out: Value = runtime
///     .run(&script, &Value::map([("switches", Value::Int(3))]))
///     .unwrap_or_else(|e| panic!("{e}"));
/// assert_eq!(out, Value::map([("left", Value::Int(0)), ("open", Value::Bool(true))]));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(untagged)]
pub enum Value {
    /// Nothing: a script's `()`.
    Unit,
    /// `true` or `false`.
    Bool(bool),
    /// A whole number.
    Int(i64),
    /// A string, or a character.
    Text(String),
    /// A list.
    List(Vec<Value>),
    /// A map by name.
    Map(BTreeMap<String, Value>),
}

impl Value {
    /// A map of `entries`.
    #[must_use]
    pub fn map<K: Into<String>>(entries: impl IntoIterator<Item = (K, Value)>) -> Self {
        Self::Map(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
}

impl<'de> Deserialize<'de> for Value {
    /// Whatever the value is: a script's values say their own type.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("a unit, a boolean, a whole number, a string, a list or a map")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Unit)
    }

    fn visit_none<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Unit)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        Value::deserialize(deserializer)
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Int(value))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
        i64::try_from(value)
            .map(Value::Int)
            .map_err(|_| E::custom("a whole number too large for i64"))
    }

    fn visit_char<E: de::Error>(self, value: char) -> Result<Value, E> {
        Ok(Value::Text(value.into()))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(Value::Text(value.into()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Value, E> {
        Ok(Value::Text(value))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element()? {
            items.push(item);
        }
        Ok(Value::List(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut entries = BTreeMap::new();
        while let Some((key, value)) = map.next_entry::<String, Value>()? {
            entries.insert(key, value);
        }
        Ok(Value::Map(entries))
    }
}

impl core::fmt::Display for Value {
    /// The script's own form: `42`, `true`, `"text"`, `[1, 2]`,
    /// `#{"a": 1}`, `()`.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unit => f.write_str("()"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Int(n) => write!(f, "{n}"),
            Self::Text(text) => write!(f, "{text:?}"),
            Self::List(items) => {
                f.write_str("[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_str("]")
            }
            Self::Map(entries) => {
                f.write_str("#{")?;
                for (i, (key, value)) in entries.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{key:?}: {value}")?;
                }
                f.write_str("}")
            }
        }
    }
}
