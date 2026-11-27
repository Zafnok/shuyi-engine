//! A part set file checked alone ([`PartsDef::from_source`]): every
//! problem, each at the line that shows it when the file has one.

use crate::prelude::*;

use alloc::collections::{BTreeMap, BTreeSet};

use crate::error::{Code, ContentError, codes};
use crate::palette::parse_hex;

use super::{MAX_KINDS, MAX_PARTS, MAX_ROWS, MAX_SIDE, MAX_SLOTS, PartsDef};

/// Every problem of `def`, read from `source`, the file shown as `file`.
pub(super) fn problems(def: &PartsDef, file: &str, source: &str) -> Vec<ContentError> {
    let mut found = Found {
        file,
        lines: source.lines().collect(),
        errors: Vec::new(),
    };
    found.size(def);
    found.slots(def);
    let keys = found.keys(def);
    found.colours(def, &keys);
    found.parts(def);
    found.kinds(def);
    found.errors
}

/// The problems found so far, and the file's lines to place them.
struct Found<'a> {
    file: &'a str,
    lines: Vec<&'a str>,
    errors: Vec<ContentError>,
}

/// Where a problem shows: the `n`th line (from 0) at or after the field
/// `section` that holds `needle`, or the field's own line.
#[derive(Clone, Copy)]
struct At<'a> {
    section: &'a str,
    needle: Option<&'a str>,
    nth: usize,
}

/// The field `section`'s line.
const fn field(section: &str) -> At<'_> {
    At {
        section,
        needle: None,
        nth: 0,
    }
}

/// The first line after the field `section` that holds `needle`.
const fn under<'a>(section: &'a str, needle: &'a str) -> At<'a> {
    At {
        section,
        needle: Some(needle),
        nth: 0,
    }
}

/// `"name"`, as the file writes a name.
fn quoted(name: &str) -> String {
    format!("\"{name}\"")
}

/// `"name":`, as the file writes the entry it defines.
fn entry(name: &str) -> String {
    format!("\"{name}\":")
}

/// Whether `line` holds `needle`, not as the end of a longer word or
/// number (`20:` in `120:`).
fn holds(line: &str, needle: &str) -> bool {
    line.match_indices(needle).any(|(i, _)| {
        line[..i]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_')
    })
}

impl Found<'_> {
    /// The 1-based line `at` names, if the file shows one.
    fn line(&self, at: At<'_>) -> Option<u32> {
        let start = self.lines.iter().position(|l| {
            l.trim_start()
                .strip_prefix(at.section)
                .is_some_and(|rest| rest.trim_start().starts_with(':'))
        })?;
        let index = match at.needle {
            None => start,
            // The `nth` line that holds it, or the last when fewer do (a
            // name written twice on one line).
            Some(needle) => {
                let holding: Vec<usize> = self.lines[start..]
                    .iter()
                    .enumerate()
                    .filter(|(_, l)| holds(l, needle))
                    .map(|(i, _)| start + i)
                    .collect();
                *holding.get(at.nth).or(holding.last())?
            }
        };
        u32::try_from(index + 1).ok()
    }

    /// Reports `message` at `at`, with `code`.
    fn report(&mut self, at: At<'_>, code: Option<Code>, message: String) {
        let mut e = ContentError::new(self.file, message);
        if let Some(line) = self.line(at) {
            e = e.at(line, None);
        }
        if let Some(code) = code {
            e = e.with_code(code);
        }
        self.errors.push(e);
    }

    fn size(&mut self, def: &PartsDef) {
        let (w, h) = def.size;
        if !(1..=MAX_SIDE).contains(&w) || !(1..=MAX_SIDE).contains(&h) {
            self.report(
                field("size"),
                Some(codes::OUT_OF_RANGE),
                format!("size is {w}×{h} px; each side must be 1 to {MAX_SIDE} px"),
            );
        }
    }

    fn slots(&mut self, def: &PartsDef) {
        if def.slots.is_empty() {
            self.report(field("slots"), None, "the set has no slot".to_owned());
        }
        if def.slots.len() > MAX_SLOTS {
            self.report(
                field("slots"),
                Some(codes::OUT_OF_RANGE),
                format!(
                    "{} slots; a set may have at most {MAX_SLOTS}",
                    def.slots.len()
                ),
            );
        }
        let mut seen = BTreeSet::new();
        for slot in &def.slots {
            if !seen.insert(slot) {
                let needle = quoted(slot);
                let at = At {
                    nth: 1,
                    ..under("slots", &needle)
                };
                self.report(
                    at,
                    Some(codes::DUPLICATE),
                    format!("slot \"{slot}\" is named twice"),
                );
            }
        }
    }

    /// Checks the keys, and gives each key's grey by what it stands for.
    fn keys(&mut self, def: &PartsDef) -> BTreeMap<String, u8> {
        let mut greys = BTreeSet::new();
        let mut names = BTreeMap::new();
        for (grey, name) in &def.keys {
            let needle = format!("{grey}:");
            if !greys.insert(*grey) {
                let at = At {
                    nth: 1,
                    ..under("keys", &needle)
                };
                self.report(
                    at,
                    Some(codes::DUPLICATE),
                    format!("key grey {grey} is named twice"),
                );
            }
            if names.insert(name.clone(), *grey).is_some() {
                self.report(
                    under("keys", &needle),
                    Some(codes::DUPLICATE),
                    format!("two key greys stand for \"{name}\""),
                );
            }
        }
        names
    }

    fn colours(&mut self, def: &PartsDef, keys: &BTreeMap<String, u8>) {
        let mut grouped: BTreeMap<&str, &str> = BTreeMap::new();
        for (name, group) in &def.colours {
            let needle = entry(name);
            let at = under("colours", &needle);
            let what = format!("colour group \"{name}\"");
            if group.keys.is_empty() {
                self.report(at, None, format!("{what} colours no key"));
            }
            for key in &group.keys {
                if !keys.contains_key(key) {
                    self.report(
                        at,
                        Some(codes::UNKNOWN_NAME),
                        format!("{what}: \"{key}\" is not a key"),
                    );
                } else if let Some(other) = grouped.insert(key, name) {
                    self.report(
                        at,
                        Some(codes::DUPLICATE),
                        format!("{what}: key \"{key}\" is in group \"{other}\" too"),
                    );
                }
            }
            self.rows(at, &what, group.keys.len(), &group.rows);
        }
    }

    /// Checks `rows`, each of `width` colours, of `what`, shown at `at`.
    fn rows(&mut self, at: At<'_>, what: &str, width: usize, rows: &[Vec<String>]) {
        if rows.is_empty() {
            self.report(at, None, format!("{what} has no rows"));
        }
        if rows.len() > MAX_ROWS {
            self.report(
                at,
                Some(codes::OUT_OF_RANGE),
                format!(
                    "{what} has {} rows; a group may have at most {MAX_ROWS}",
                    rows.len()
                ),
            );
        }
        for (i, row) in rows.iter().enumerate() {
            if row.len() != width {
                self.report(
                    at,
                    None,
                    format!(
                        "{what}: row {} has {} colours; it needs one per key, {width}",
                        i + 1,
                        row.len()
                    ),
                );
            }
            for colour in row {
                if let Err(why) = parse_hex(colour) {
                    self.report(at, None, format!("{what}: row {}: {why}", i + 1));
                }
            }
        }
    }

    fn parts(&mut self, def: &PartsDef) {
        if def.parts.len() > MAX_PARTS {
            self.report(
                field("parts"),
                Some(codes::OUT_OF_RANGE),
                format!(
                    "{} parts; a set may have at most {MAX_PARTS}",
                    def.parts.len()
                ),
            );
        }
        let slot_of = |name: &str| def.slots.iter().position(|s| s == name);
        for (name, part) in &def.parts {
            let needle = entry(name);
            let at = under("parts", &needle);
            let own = slot_of(&part.slot);
            if own.is_none() {
                self.report(
                    at,
                    Some(codes::UNKNOWN_NAME),
                    format!(
                        "part \"{name}\": slot \"{}\" is not one of `slots`",
                        part.slot
                    ),
                );
            }
            for hidden in &part.hides {
                match (slot_of(hidden), own) {
                    (None, _) => self.report(
                        at,
                        Some(codes::UNKNOWN_NAME),
                        format!("part \"{name}\": it hides \"{hidden}\", which is not one of `slots`"),
                    ),
                    (Some(h), Some(own)) if h >= own => self.report(
                        at,
                        None,
                        format!(
                            "part \"{name}\": it hides \"{hidden}\", which is not under its own slot \"{}\"",
                            part.slot
                        ),
                    ),
                    _ => {}
                }
            }
        }
    }

    fn kinds(&mut self, def: &PartsDef) {
        if def.kinds.len() > MAX_KINDS {
            self.report(
                field("kinds"),
                Some(codes::OUT_OF_RANGE),
                format!(
                    "{} kinds; a set may have at most {MAX_KINDS}",
                    def.kinds.len()
                ),
            );
        }
        for (name, kind) in &def.kinds {
            let needle = entry(name);
            let at = under("kinds", &needle);
            let what = format!("kind \"{name}\"");
            if let Some(base) = &kind.base
                && !def.kinds.contains_key(base)
            {
                self.report(
                    at,
                    Some(codes::UNKNOWN_NAME),
                    format!("{what}: its base \"{base}\" is not a kind"),
                );
            }
            for (slot, list) in &kind.slots {
                if !def.slots.contains(slot) {
                    self.report(
                        at,
                        Some(codes::UNKNOWN_NAME),
                        format!("{what}: slot \"{slot}\" is not one of `slots`"),
                    );
                }
                if list.is_empty() {
                    self.report(
                        at,
                        None,
                        format!("{what}: the list of slot \"{slot}\" is empty"),
                    );
                }
                for part in list.iter().flatten() {
                    match def.parts.get(part) {
                        None => self.report(
                            at,
                            Some(codes::UNKNOWN_NAME),
                            format!("{what}: slot \"{slot}\": \"{part}\" is not a part"),
                        ),
                        Some(p) if p.slot != *slot => self.report(
                            at,
                            None,
                            format!(
                                "{what}: slot \"{slot}\": part \"{part}\" goes in slot \"{}\"",
                                p.slot
                            ),
                        ),
                        Some(_) => {}
                    }
                }
            }
            for (group, rows) in &kind.colours {
                match def.colours.get(group) {
                    None => self.report(
                        at,
                        Some(codes::UNKNOWN_NAME),
                        format!("{what}: \"{group}\" is not a colour group"),
                    ),
                    Some(g) => {
                        let what = format!("{what}: colour group \"{group}\"");
                        self.rows(at, &what, g.keys.len(), rows);
                    }
                }
            }
        }
        for cycle in cycles(def) {
            let names: Vec<String> = cycle.iter().map(|k| quoted(k)).collect();
            let needle = entry(&cycle[0]);
            self.report(
                under("kinds", &needle),
                None,
                format!("kinds {} make a cycle of bases", names.join(", ")),
            );
        }
    }
}

/// Every cycle of the kinds' bases, each once, from its first kind by
/// name, in the order the bases lead.
fn cycles(def: &PartsDef) -> BTreeSet<Vec<String>> {
    let mut found = BTreeSet::new();
    for start in def.kinds.keys() {
        let mut path: Vec<&String> = vec![start];
        let mut at = start;
        while let Some(base) = def.kinds.get(at).and_then(|k| k.base.as_ref()) {
            if let Some(i) = path.iter().position(|k| *k == base) {
                let cycle = &path[i..];
                let first = (0..cycle.len()).min_by_key(|&j| cycle[j]).unwrap_or(0);
                let turned = cycle[first..].iter().chain(&cycle[..first]);
                found.insert(turned.map(|k| (*k).clone()).collect());
                break;
            }
            path.push(base);
            at = base;
        }
    }
    found
}
