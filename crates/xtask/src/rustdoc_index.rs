//! An index of a crate's public items, read from rustdoc's JSON (ticket
//! 3413): each item by every public path it is reached at, with its kind,
//! its docs, its `#[doc(alias)]`es and its deprecation.
//!
//! [`build`] runs `cargo +<NIGHTLY> rustdoc … --output-format json` with
//! the features it is given ([`crate::api::NIGHTLY`], the one
//! `cargo-public-api` reads), and [`Index::read`] reads the file it
//! writes. The walk starts at the crate's root module and follows its
//! modules and its `pub use`s of the crate's own items, so an item
//! re-exported at a shorter path is found at both. An item of another
//! crate that a `pub use` names is that crate's index's, and an item of a
//! trait's `impl` (`Clone::clone`) is none's: `cargo-public-api` lists it,
//! and a listing line of it has no item here.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::api::{NIGHTLY, Runner};

#[cfg(test)]
mod tests;

/// How deep the walk follows modules and re-exports: past it a path is a
/// loop of re-exports (`pub use self::a as b` in `a`).
const MAX_DEPTH: usize = 32;

/// One public item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// Rustdoc's kind of it: `struct`, `function`, `variant`, `module` …
    pub kind: String,
    /// Its docs, as written.
    pub docs: String,
    /// The words of its `#[doc(alias = "…")]`s, in their order.
    pub aliases: Vec<String>,
    /// `Some` when it is `#[deprecated]`: its note, or empty.
    pub deprecated: Option<String>,
    /// Every public path it is reached at, sorted.
    pub paths: Vec<String>,
}

/// A crate's public items, by path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Index {
    /// The crate's name, as paths start (`shuyi_ui`).
    pub krate: String,
    items: Vec<Item>,
    by_path: BTreeMap<String, usize>,
}

impl Index {
    /// The index of the rustdoc JSON `json`.
    ///
    /// # Errors
    ///
    /// The JSON doesn't parse, or has no root module.
    pub fn read(json: &str) -> Result<Self, String> {
        let doc: Value = serde_json::from_str(json).map_err(|e| format!("rustdoc JSON: {e}"))?;
        let index = doc["index"].as_object().ok_or("rustdoc JSON: no `index`")?;
        let root_id = id_of(&doc["root"]).ok_or("rustdoc JSON: no `root`")?;
        let root = index
            .get(&root_id)
            .ok_or("rustdoc JSON: the root isn't in the index")?;
        let krate = root["name"]
            .as_str()
            .ok_or("rustdoc JSON: the root has no name")?
            .to_owned();
        let mut walk = Walk {
            index,
            found: BTreeMap::new(),
        };
        walk.module(root, &krate, 0);
        let mut out = Self {
            krate,
            ..Self::default()
        };
        for (id, paths) in walk.found {
            let Some(item) = index.get(&id) else { continue };
            let at = out.items.len();
            for path in &paths {
                out.by_path.entry(path.clone()).or_insert(at);
            }
            out.items.push(Item {
                kind: kind_of(item).to_owned(),
                docs: item["docs"].as_str().unwrap_or_default().to_owned(),
                aliases: aliases(item),
                deprecated: item["deprecation"].as_object().map(|d| {
                    d.get("note")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_owned()
                }),
                paths: paths.into_iter().collect(),
            });
        }
        Ok(out)
    }

    /// The item at `path`.
    pub fn get(&self, path: &str) -> Option<&Item> {
        self.by_path.get(path).map(|&at| &self.items[at])
    }

    /// Every item, in the order of their ids.
    pub fn items(&self) -> &[Item] {
        &self.items
    }
}

/// The walk over the modules: each item's id and the paths it was found at.
struct Walk<'a> {
    index: &'a serde_json::Map<String, Value>,
    found: BTreeMap<String, BTreeSet<String>>,
}

impl Walk<'_> {
    /// Records `item` (of id `id`) at `path`, and walks what it holds.
    fn item(&mut self, id: &str, item: &Value, path: &str, depth: usize) {
        if depth > MAX_DEPTH
            || !self
                .found
                .entry(id.to_owned())
                .or_default()
                .insert(path.to_owned())
        {
            return;
        }
        let inner = &item["inner"];
        match kind_of(item) {
            "module" => self.module(item, path, depth),
            "struct" => {
                for field in ids(&inner["struct"]["kind"]["plain"]["fields"]) {
                    self.named(&field, path, depth);
                }
                self.impls(&inner["struct"]["impls"], path, depth);
            }
            "enum" => {
                for variant in ids(&inner["enum"]["variants"]) {
                    self.named(&variant, path, depth);
                }
                self.impls(&inner["enum"]["impls"], path, depth);
            }
            "union" => self.impls(&inner["union"]["impls"], path, depth),
            "trait" => {
                for member in ids(&inner["trait"]["items"]) {
                    self.named(&member, path, depth);
                }
            }
            _ => {}
        }
    }

    /// The items of a module at `path`.
    fn module(&mut self, module: &Value, path: &str, depth: usize) {
        for id in ids(&module["inner"]["module"]["items"]) {
            let Some(item) = self.index.get(&id) else {
                continue;
            };
            if kind_of(item) != "use" {
                self.named(&id, path, depth + 1);
                continue;
            }
            let using = &item["inner"]["use"];
            let Some(target_id) = id_of(&using["id"]) else {
                continue;
            };
            // Another crate's item: that crate's index has it.
            let Some(target) = self.index.get(&target_id) else {
                continue;
            };
            if using["is_glob"].as_bool() == Some(true) {
                if kind_of(target) == "module" {
                    self.module(target, path, depth + 1);
                }
            } else if let Some(name) = using["name"].as_str() {
                self.item(&target_id, target, &format!("{path}::{name}"), depth + 1);
            }
        }
    }

    /// The item of id `id`, at `path` and its own name.
    fn named(&mut self, id: &str, path: &str, depth: usize) {
        let Some(item) = self.index.get(id) else {
            return;
        };
        let Some(name) = item["name"].as_str() else {
            return;
        };
        self.item(id, item, &format!("{path}::{name}"), depth + 1);
    }

    /// The items of the inherent `impls` of the type at `path`; a trait's
    /// impl is left out.
    fn impls(&mut self, impls: &Value, path: &str, depth: usize) {
        for id in ids(impls) {
            let Some(block) = self.index.get(&id) else {
                continue;
            };
            let block = &block["inner"]["impl"];
            if !block["trait"].is_null() {
                continue;
            }
            for member in ids(&block["items"]) {
                self.named(&member, path, depth);
            }
        }
    }
}

/// An id, a number or a string in the JSON, as the index's key.
fn id_of(value: &Value) -> Option<String> {
    match value {
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

/// The ids of a JSON array, its nulls (a stripped field) left out.
fn ids(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(id_of)
        .collect()
}

/// Rustdoc's kind of an item: the one key of its `inner`.
fn kind_of(item: &Value) -> &str {
    item["inner"]
        .as_object()
        .and_then(|inner| inner.keys().next())
        .map_or("", String::as_str)
}

/// The words of an item's `#[doc(alias = "…")]`s and
/// `#[doc(alias("…", "…"))]`s.
fn aliases(item: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for attr in item["attrs"].as_array().into_iter().flatten() {
        let text = attr["other"]
            .as_str()
            .or_else(|| attr.as_str())
            .unwrap_or("");
        out.extend(doc_aliases(text));
    }
    out
}

/// The alias words in one attribute's text, `#[doc(alias = "x")]` or
/// `#[doc(alias("x", "y"))]`.
pub fn doc_aliases(attr: &str) -> Vec<String> {
    let compact: String = attr.chars().filter(|c| !c.is_whitespace()).collect();
    let Some(rest) = compact
        .strip_prefix("#[doc(alias")
        .and_then(|rest| rest.strip_suffix(")]"))
    else {
        return Vec::new();
    };
    let list = rest
        .strip_prefix('=')
        .or_else(|| rest.strip_prefix('(').and_then(|r| r.strip_suffix(')')))
        .unwrap_or("");
    list.split(',')
        .filter_map(|word| word.strip_prefix('"')?.strip_suffix('"'))
        .map(str::to_owned)
        .collect()
}

/// Where `cargo rustdoc` writes `lib`'s JSON under `target_dir`.
pub fn json_path(target_dir: &Path, lib: &str) -> PathBuf {
    target_dir.join("doc").join(format!("{lib}.json"))
}

/// Builds the rustdoc JSON of `package` in the workspace at `root`, on
/// [`NIGHTLY`] with `features` (`--no-default-features`, `--features x` …),
/// in `target_dir`, and reads its index.
///
/// # Errors
///
/// The build fails, or its JSON can't be read.
pub fn build(
    root: &Path,
    package: &str,
    features: &[&str],
    target_dir: &Path,
    runner: &dyn Runner,
) -> Result<Index, String> {
    let toolchain = format!("+{NIGHTLY}");
    let target = target_dir.to_string_lossy();
    let mut args = vec![toolchain.as_str(), "rustdoc", "-p", package, "--lib"];
    args.extend(features);
    args.extend([
        "--target-dir",
        &target,
        "--",
        "-Z",
        "unstable-options",
        "--output-format",
        "json",
    ]);
    let ran = runner.run(root, "cargo", &args)?;
    if !ran.ok {
        return Err(format!(
            "`cargo {}` failed:\n{}",
            args.join(" "),
            ran.stderr.trim_end()
        ));
    }
    let path = json_path(target_dir, &package.replace('-', "_"));
    let json = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    Index::read(&json).map_err(|e| format!("{}: {e}", path.display()))
}
