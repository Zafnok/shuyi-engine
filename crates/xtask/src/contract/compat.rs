//! Whether a change to the contract's schemas breaks a client (ADR-0150
//! §1, §4): it compares the base's schemas with the head's, file by file,
//! and lists each break. Anything it can't prove a client survives is a
//! break.
//!
//! - **An answer** (`*.output.json`, and the other files a client reads)
//!   may gain fields, gain values of an enum and gain tagged variants. It
//!   breaks when a field goes, a value or a variant goes (a rename is
//!   one gone and one new), a field that was always there may be absent,
//!   or a value may be of a type it wasn't (`null` among them).
//! - **An input** (`*.input.json`) may gain optional fields, values and
//!   variants, and take more types. It breaks when a field goes, a new
//!   field is required, an optional one becomes required, a value or a
//!   variant goes, or it takes fewer types.
//!
//! Words that don't change the shape (`description`, `title`, `default`,
//! `examples`) are ignored: a changed default is a `Behaviour:` line, not
//! a break. Any other keyword that changed (`format`, `minimum`,
//! `additionalProperties`, …) is a break.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::LazyLock;

use serde_json::Value;

/// Which way a schema's data goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The client writes it.
    Input,
    /// The client reads it.
    Answer,
}

impl Side {
    /// The side of the schema file `name`: `*.input.json` is an input,
    /// every other file one the client reads.
    pub fn of(name: &str) -> Self {
        if name.ends_with(".input.json") {
            Self::Input
        } else {
            Self::Answer
        }
    }
}

/// The fields of an object that lists none.
static EMPTY: LazyLock<serde_json::Map<String, Value>> = LazyLock::new(serde_json::Map::new);

/// Keywords that describe a schema without changing what it accepts.
const IGNORED: &[&str] = &[
    "$schema",
    "$defs",
    "$ref",
    "description",
    "title",
    "default",
    "examples",
    "deprecated",
    "readOnly",
    "writeOnly",
];

/// Keywords the walk compares itself.
const WALKED: &[&str] = &[
    "type",
    "const",
    "enum",
    "properties",
    "required",
    "items",
    "anyOf",
    "oneOf",
    "allOf",
];

/// Every break from the `old` schema files to the `new` ones, by file
/// name: a file gone is a break, a file new isn't.
pub fn breaks_between(old: &BTreeMap<String, Value>, new: &BTreeMap<String, Value>) -> Vec<String> {
    let mut breaks = Vec::new();
    for (name, old_schema) in old {
        match new.get(name) {
            None => breaks.push(format!("{name}: the schema is gone")),
            Some(new_schema) => breaks.extend(breaks_in(name, old_schema, new_schema)),
        }
    }
    breaks
}

/// Every break from `old` to `new`, the two versions of schema file
/// `name`.
pub fn breaks_in(name: &str, old: &Value, new: &Value) -> Vec<String> {
    let mut walk = Walk {
        name,
        side: Side::of(name),
        old_root: old,
        new_root: new,
        seen: HashSet::new(),
        breaks: Vec::new(),
    };
    walk.node("", old, new);
    walk.breaks
}

struct Walk<'a> {
    name: &'a str,
    side: Side,
    old_root: &'a Value,
    new_root: &'a Value,
    /// The pairs of definitions already compared, so a type that holds
    /// itself ends.
    seen: HashSet<(String, String)>,
    breaks: Vec<String>,
}

impl<'a> Walk<'a> {
    fn broke(&mut self, path: &str, why: impl std::fmt::Display) {
        let at = if path.is_empty() { "(the root)" } else { path };
        self.breaks.push(format!("{}: {at}: {why}", self.name));
    }

    fn node(&mut self, path: &str, old: &'a Value, new: &'a Value) {
        let old_ref = ref_of(old);
        let new_ref = ref_of(new);
        if (old_ref.is_some() || new_ref.is_some())
            && !self.seen.insert((
                old_ref.unwrap_or_default().to_owned(),
                new_ref.unwrap_or_default().to_owned(),
            ))
        {
            return;
        }
        let old = resolve(self.old_root, old);
        let new = resolve(self.new_root, new);
        let (Some(old_map), Some(new_map)) = (old.as_object(), new.as_object()) else {
            if old != new {
                self.broke(path, format!("changed from {old} to {new}"));
            }
            return;
        };
        if union(old_map).is_some() || union(new_map).is_some() {
            self.variants(path, old, new);
            return;
        }
        self.types(path, old_map, new_map);
        self.values(path, old_map, new_map);
        self.properties(path, old_map, new_map);
        match (old_map.get("items"), new_map.get("items")) {
            (Some(old_items), Some(new_items)) => {
                self.node(&join(path, "[]"), old_items, new_items);
            }
            (None, None) => {}
            _ => self.broke(path, "its items changed"),
        }
        let keys: BTreeSet<&String> = old_map.keys().chain(new_map.keys()).collect();
        for key in keys {
            if IGNORED.contains(&key.as_str()) || WALKED.contains(&key.as_str()) {
                continue;
            }
            match (old_map.get(key), new_map.get(key)) {
                (Some(o), Some(n))
                    if key == "additionalProperties" && o.is_object() && n.is_object() =>
                {
                    self.node(&join(path, "{}"), o, n);
                }
                (o, n) if o != n => self.broke(
                    path,
                    format!("`{key}` changed from {} to {}", shown(o), shown(n)),
                ),
                _ => {}
            }
        }
    }

    /// `type`: an answer may narrow it, an input widen it.
    fn types(
        &mut self,
        path: &str,
        old: &serde_json::Map<String, Value>,
        new: &serde_json::Map<String, Value>,
    ) {
        match (types_of(old), types_of(new)) {
            (Some(o), Some(n)) => {
                let (lost, why) = match self.side {
                    Side::Answer => (
                        n.difference(&o).cloned().collect::<Vec<_>>(),
                        "it can now be",
                    ),
                    Side::Input => (
                        o.difference(&n).cloned().collect::<Vec<_>>(),
                        "it no longer takes",
                    ),
                };
                if !lost.is_empty() {
                    self.broke(path, format!("{why} {}", lost.join(", ")));
                }
            }
            (None, None) => {}
            (Some(_), None) if self.side == Side::Input => {}
            (None, Some(_)) if self.side == Side::Answer => {}
            _ => self.broke(path, "its type changed"),
        }
    }

    /// `const` and `enum`: values may come, never go.
    fn values(
        &mut self,
        path: &str,
        old: &serde_json::Map<String, Value>,
        new: &serde_json::Map<String, Value>,
    ) {
        match (values_of(old), values_of(new)) {
            (Some(o), Some(n)) => {
                let gone: Vec<String> = o
                    .iter()
                    .filter(|v| !n.contains(v))
                    .map(ToString::to_string)
                    .collect();
                if !gone.is_empty() {
                    self.broke(path, format!("the value {} is gone", gone.join(", ")));
                }
            }
            (None, None) => {}
            (Some(_), None) if self.side == Side::Input => {}
            _ => self.broke(path, "its values changed"),
        }
    }

    fn properties(
        &mut self,
        path: &str,
        old: &'a serde_json::Map<String, Value>,
        new: &'a serde_json::Map<String, Value>,
    ) {
        let old_props = old
            .get("properties")
            .and_then(Value::as_object)
            .unwrap_or(&EMPTY);
        let new_props = new
            .get("properties")
            .and_then(Value::as_object)
            .unwrap_or(&EMPTY);
        let old_required = required_of(old);
        let new_required = required_of(new);
        for (field, old_field) in old_props {
            let at = join(path, field);
            let Some(new_field) = new_props.get(field) else {
                self.broke(&at, "the field is gone");
                continue;
            };
            let (was, is) = (old_required.contains(field), new_required.contains(field));
            match self.side {
                Side::Answer if was && !is => self.broke(&at, "the field may now be absent"),
                Side::Input if !was && is => self.broke(&at, "the field is now required"),
                _ => {}
            }
            self.node(&at, old_field, new_field);
        }
        if self.side == Side::Input {
            for field in new_props.keys().filter(|f| !old_props.contains_key(*f)) {
                if new_required.contains(field) {
                    self.broke(&join(path, field), "a new field is required");
                }
            }
        }
    }

    /// `anyOf`, `oneOf` and `allOf`: each variant is matched by its tag,
    /// else by its place. A variant may come (but an answer that can now
    /// be `null` breaks), never go.
    fn variants(&mut self, path: &str, old: &'a Value, new: &'a Value) {
        let old_kind = old.as_object().and_then(union).map_or("", |(kind, _)| kind);
        let new_kind = new.as_object().and_then(union).map_or("", |(kind, _)| kind);
        if (old_kind == "allOf") != (new_kind == "allOf") {
            self.broke(path, "it changed from a combination to a choice, or back");
            return;
        }
        let old_variants = keyed(self.old_root, old);
        let new_variants = keyed(self.new_root, new);
        for (key, old_variant) in &old_variants {
            match new_variants.get(key) {
                None => self.broke(path, format!("the variant {key} is gone")),
                Some(new_variant) => self.node(path, old_variant, new_variant),
            }
        }
        for key in new_variants
            .keys()
            .filter(|k| !old_variants.contains_key(*k))
        {
            if self.side == Side::Answer && key == "type:null" {
                self.broke(path, "it can now be null");
            } else if old_kind == "allOf" {
                self.broke(path, format!("a new part {key} is required"));
            }
        }
    }
}

/// The variants of `schema` (itself, if it is no union), by key.
fn keyed<'a>(root: &'a Value, schema: &'a Value) -> BTreeMap<String, &'a Value> {
    let list: Vec<&'a Value> = match schema.as_object().and_then(union) {
        Some((_, variants)) => variants.iter().collect(),
        None => vec![schema],
    };
    list.into_iter()
        .enumerate()
        .map(|(place, variant)| (key_of(root, variant, place), variant))
        .collect()
}

/// `schema`'s union keyword and its variants.
fn union(schema: &serde_json::Map<String, Value>) -> Option<(&'static str, &Vec<Value>)> {
    ["anyOf", "oneOf", "allOf"].into_iter().find_map(|kind| {
        schema
            .get(kind)
            .and_then(Value::as_array)
            .map(|v| (kind, v))
    })
}

/// A variant's key: its value, its tag field's value, its one field's
/// name, `type:null`, or else its place.
fn key_of(root: &Value, variant: &Value, place: usize) -> String {
    let variant = resolve(root, variant);
    let Some(map) = variant.as_object() else {
        return format!("#{place}");
    };
    if let Some(values) = values_of(map)
        && values.len() == 1
    {
        return format!("={}", values[0]);
    }
    let required = required_of(map);
    if let Some(props) = map.get("properties").and_then(Value::as_object) {
        for (field, schema) in props {
            if !required.contains(field) {
                continue;
            }
            let schema = resolve(root, schema);
            if let Some(values) = schema.as_object().and_then(values_of)
                && values.len() == 1
            {
                return format!("{field}={}", values[0]);
            }
        }
        if props.len() == 1 && required.len() == 1 {
            return format!("{{{}}}", required.iter().next().map_or("", String::as_str));
        }
    }
    if map.get("type").and_then(Value::as_str) == Some("null") {
        return "type:null".to_owned();
    }
    format!("#{place}")
}

fn ref_of(schema: &Value) -> Option<&str> {
    schema.get("$ref").and_then(Value::as_str)
}

/// `schema`, or what its `$ref` (`#/$defs/<name>`) names in `root`.
fn resolve<'a>(root: &'a Value, schema: &'a Value) -> &'a Value {
    let mut at = schema;
    for _ in 0..16 {
        let Some(target) = ref_of(at).and_then(|r| r.strip_prefix('#')) else {
            return at;
        };
        match root.pointer(target) {
            Some(found) => at = found,
            None => return at,
        }
    }
    at
}

fn types_of(schema: &serde_json::Map<String, Value>) -> Option<BTreeSet<String>> {
    match schema.get("type")? {
        Value::String(one) => Some(BTreeSet::from([one.clone()])),
        Value::Array(many) => Some(
            many.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
        ),
        _ => None,
    }
}

fn values_of(schema: &serde_json::Map<String, Value>) -> Option<Vec<&Value>> {
    if let Some(one) = schema.get("const") {
        return Some(vec![one]);
    }
    schema
        .get("enum")
        .and_then(Value::as_array)
        .map(|v| v.iter().collect())
}

fn required_of(schema: &serde_json::Map<String, Value>) -> BTreeSet<String> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map_or_default(|r| {
            r.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
}

fn join(path: &str, part: &str) -> String {
    if path.is_empty() {
        part.to_owned()
    } else if part.starts_with('[') || part.starts_with('{') {
        format!("{path}{part}")
    } else {
        format!("{path}.{part}")
    }
}

fn shown(value: Option<&Value>) -> String {
    value.map_or_else(|| "nothing".to_owned(), ToString::to_string)
}

#[cfg(test)]
mod tests;
