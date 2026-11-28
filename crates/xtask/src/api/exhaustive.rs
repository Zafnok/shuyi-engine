//! The public types left exhaustive, each with its kind (ticket 2096,
//! ADR-0039).
//!
//! A struct or an enum without `#[non_exhaustive]` breaks every game that
//! builds it by literal, or matches it, when it grows a field or a variant.
//! ADR-0039 leaves three kinds exhaustive: plain values (kind 1), closed
//! sets (kind 6), and types with a private field (kind 8), which no game
//! builds by literal. The check reads the head's listing of each crate
//! (`cargo-public-api`, which writes `#[non_exhaustive]` before `pub`) and
//! fails:
//!
//! - an exhaustive `pub struct` or `pub enum` that [`FILE`] doesn't list
//!   under its crate;
//! - a path [`FILE`] lists that the listing shows `#[non_exhaustive]`, or
//!   doesn't show at all (a stale entry);
//! - an `open` path (a type that may grow but hasn't moved yet) that the
//!   base's file lacks: that list only shrinks.
//!
//! A type re-exported at another path is listed again there, and [`FILE`]
//! lists it at each of its paths.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde::Deserialize;

use super::CrateApi;
use super::constructors::type_header;

/// The exhaustive public structs and enums of each crate, each with its kind.
pub const FILE: &str = "tools/exhaustive.ron";

/// A `pub struct` or `pub enum` of a listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type {
    /// Its path, without generics or tuple fields
    /// (`shuyi_core::grid::Pos`).
    pub path: String,
    /// Whether the listing shows it `#[non_exhaustive]`.
    pub growing: bool,
}

/// The structs and enums of `listing`, in its order.
pub fn types(listing: &[String]) -> Vec<Type> {
    listing
        .iter()
        .map(String::as_str)
        .filter_map(read_type)
        .collect()
}

/// `line` as a `pub struct` or `pub enum` line, if it is one: any
/// attributes before `pub`, the word, then the path up to its generics
/// (`<`), its tuple fields (`(`) or the end of the line.
fn read_type(line: &str) -> Option<Type> {
    let header = type_header(line).filter(super::constructors::Header::is_type)?;
    let end = header
        .rest
        .find(|c: char| matches!(c, '<' | '(' | ';' | '{') || c.is_whitespace())
        .unwrap_or(header.rest.len());
    let path = &header.rest[..end];
    (!path.is_empty()).then(|| Type {
        path: path.to_string(),
        growing: header.growing,
    })
}

/// [`FILE`], as read: each crate's paths by kind.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Listed {
    crates: BTreeMap<String, Kinds>,
}

/// One crate's exhaustive types, by ADR-0039's kind.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Kinds {
    /// Kind 1: the fields are the whole meaning.
    plain: Vec<String>,
    /// Kind 6's closed sets.
    closed: Vec<String>,
    /// Kind 8: a private field, which the listing doesn't show (or shows as a
    /// `_` in a tuple struct's fields), so no game builds it by literal.
    private: Vec<String>,
    /// May grow, and hasn't moved under ADR-0039 yet: it moves in the PR that
    /// grows it.
    open: Vec<String>,
}

impl Kinds {
    /// Each path, with its kind's name.
    fn each(&self) -> impl Iterator<Item = (&'static str, &str)> {
        [
            ("plain", &self.plain),
            ("closed", &self.closed),
            ("private", &self.private),
            ("open", &self.open),
        ]
        .into_iter()
        .flat_map(|(name, paths)| paths.iter().map(move |path| (name, path.as_str())))
    }
}

/// What the check found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// [`FILE`] (or the base's) didn't parse, or an entry is wrong (a crate
    /// outside the catalog, a path listed twice).
    File(String),
    /// An exhaustive type the file doesn't list under its crate.
    Unlisted { krate: String, path: String },
    /// A listed path that the listing shows `#[non_exhaustive]`.
    Moved { krate: String, path: String },
    /// A listed path that its crate's listing doesn't show.
    Gone { krate: String, path: String },
    /// An `open` path that the base's file lacks.
    NewOpen { krate: String, path: String },
}

/// The problems of `crates`' types against `file`, the head's text of
/// [`FILE`] (`None`: there is none, and nothing is listed), and `base`, the
/// base's (`None`: there is none, and the `open` rule doesn't apply).
pub fn check(file: Option<&str>, base: Option<&str>, crates: &[CrateApi]) -> Vec<Problem> {
    let listed = match file.map(ron::from_str::<Listed>) {
        None => BTreeMap::new(),
        Some(Ok(listed)) => listed.crates,
        Some(Err(e)) => return vec![Problem::File(format!("{FILE}: {e}"))],
    };
    let base_open: Option<BTreeMap<String, BTreeSet<String>>> =
        match base.map(ron::from_str::<Listed>) {
            None => None,
            Some(Ok(base)) => Some(
                base.crates
                    .into_iter()
                    .map(|(krate, kinds)| (krate, kinds.open.into_iter().collect()))
                    .collect(),
            ),
            Some(Err(e)) => return vec![Problem::File(format!("the base's {FILE}: {e}"))],
        };
    let mut problems = Vec::new();
    for krate in listed.keys() {
        if !crates.iter().any(|c| &c.name == krate) {
            problems.push(Problem::File(format!(
                "{FILE}: `{krate}` is no crate of `API_CRATES` (crates/xtask/src/api.rs): take \
                 it out"
            )));
        }
    }
    let none = Kinds::default();
    for krate in crates {
        let kinds = listed.get(&krate.name).unwrap_or(&none);
        let mut paths: BTreeMap<&str, &str> = BTreeMap::new();
        for (kind, path) in kinds.each() {
            if let Some(first) = paths.insert(path, kind) {
                problems.push(Problem::File(format!(
                    "{FILE}: `{path}` is listed twice under `{}` ({first} and {kind}): keep one",
                    krate.name
                )));
            }
        }
        let shown: BTreeMap<&str, bool> = krate
            .types
            .iter()
            .map(|t| (t.path.as_str(), t.growing))
            .collect();
        let named = |path: &str| (krate.name.clone(), path.to_string());
        for (path, growing) in &shown {
            if !growing && !paths.contains_key(path) {
                let (krate, path) = named(path);
                problems.push(Problem::Unlisted { krate, path });
            }
        }
        for path in paths.keys() {
            match shown.get(path) {
                Some(false) => {}
                Some(true) => {
                    let (krate, path) = named(path);
                    problems.push(Problem::Moved { krate, path });
                }
                None => {
                    let (krate, path) = named(path);
                    problems.push(Problem::Gone { krate, path });
                }
            }
        }
        if let Some(base_open) = &base_open {
            let had = base_open.get(&krate.name);
            for path in &kinds.open {
                if !had.is_some_and(|had| had.contains(path)) {
                    let (krate, path) = named(path);
                    problems.push(Problem::NewOpen { krate, path });
                }
            }
        }
    }
    problems
}

impl Problem {
    /// The problem, and what to do, in a line or two.
    fn describe(&self) -> String {
        match self {
            Self::File(why) => why.clone(),
            Self::Unlisted { krate, path } => format!(
                "{krate}: `{path}` is an exhaustive public type {FILE} doesn't list: make it \
                 `#[non_exhaustive]` with a constructor (ADR-0039), or list it under `{krate}` \
                 with its kind (`plain`, `closed` or `private`)"
            ),
            Self::Moved { krate, path } => {
                format!("{krate}: `{path}` is `#[non_exhaustive]` now: take it out of {FILE}")
            }
            Self::Gone { krate, path } => format!(
                "{krate}: `{path}` is listed in {FILE} but is no public struct or enum of \
                 {krate}'s listing: take its entry out, or write its new path"
            ),
            Self::NewOpen { krate, path } => format!(
                "{krate}: `{path}` is `open` in {FILE}, and the base's file doesn't have it \
                 there: the `open` list only shrinks. Make it `#[non_exhaustive]` with a \
                 constructor (ADR-0039), or list it as `plain`, `closed` or `private` if it \
                 never grows"
            ),
        }
    }
}

/// The run's error for `problems`, if there are any.
pub fn error(problems: &[Problem]) -> Option<String> {
    if problems.is_empty() {
        return None;
    }
    let mut text = String::new();
    for problem in problems {
        let _ = writeln!(text, "  {}", problem.describe());
    }
    Some(format!(
        "{} problem(s) with exhaustive public types (ADR-0039):\n{text}A struct or an enum \
         without `#[non_exhaustive]` breaks every game that builds or matches it when it grows. \
         {FILE} names each one with its kind: `plain` (kind 1), `closed` (kind 6), `private` \
         (kind 8) or `open` (moves in the PR that grows it).",
        problems.len()
    ))
}

/// The summary's section for `problems`: nothing when there are none.
pub fn section(problems: &[Problem]) -> String {
    if problems.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n**Exhaustive types, fail:**\n\n");
    for problem in problems {
        let _ = writeln!(out, "- {}", problem.describe());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listing(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|line| (*line).to_string()).collect()
    }

    fn crate_with(name: &str, lines: &[&str]) -> CrateApi {
        CrateApi::new(name, None, &listing(lines), Vec::new())
    }

    fn paths(found: &[Type]) -> Vec<(&str, bool)> {
        found.iter().map(|t| (t.path.as_str(), t.growing)).collect()
    }

    const CORE: &[&str] = &[
        "pub mod shuyi_core",
        "pub struct shuyi_core::grid::Pos",
        "pub shuyi_core::grid::Pos::x: i32",
        "pub const fn shuyi_core::grid::Pos::new(i32, i32) -> Self",
        "#[non_exhaustive] pub struct shuyi_core::bots::plan::PlanSettings",
        "pub enum shuyi_core::grid::Dir",
        "pub shuyi_core::grid::Dir::Up",
    ];

    #[test]
    fn structs_and_enums_are_read_with_their_attribute() {
        assert_eq!(
            paths(&types(&listing(CORE))),
            [
                ("shuyi_core::grid::Pos", false),
                ("shuyi_core::bots::plan::PlanSettings", true),
                ("shuyi_core::grid::Dir", false),
            ]
        );
    }

    #[test]
    fn generics_tuple_fields_and_other_attributes_leave_the_path() {
        let found = types(&listing(&[
            "pub struct shuyi_core::time::Millis(pub u32)",
            "pub struct shuyi_ui::Bag<T: core::ops::Fn(u8) -> u8>",
            "pub struct shuyi_ui::Bag2<T>(pub T) where T: core::clone::Clone",
            "#[repr(u8)] pub enum shuyi_ui::Side",
            "#[repr(u8)] #[non_exhaustive] pub enum shuyi_ui::Mode",
            "pub struct shuyi_ui::Unit;",
        ]));
        assert_eq!(
            paths(&found),
            [
                ("shuyi_core::time::Millis", false),
                ("shuyi_ui::Bag", false),
                ("shuyi_ui::Bag2", false),
                ("shuyi_ui::Side", false),
                ("shuyi_ui::Mode", true),
                ("shuyi_ui::Unit", false),
            ]
        );
    }

    #[test]
    fn what_is_no_struct_or_enum_is_not_read() {
        let found = types(&listing(&[
            "pub trait shuyi_ui::Screen",
            "pub mod shuyi_ui::structs",
            "pub type shuyi_ui::Enum = u8",
            "pub union shuyi_ui::Bits",
            "pub fn shuyi_ui::structure() -> shuyi_ui::Pos",
            "pub shuyi_ui::Pos::enum_like: u8",
            "pub struct_field shuyi_ui::X",
        ]));
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_re_exported_type_is_read_at_each_path() {
        let found = types(&listing(&[
            "pub struct shuyi_core::grid::Pos",
            "pub struct shuyi_core::Pos",
        ]));
        assert_eq!(
            paths(&found),
            [("shuyi_core::grid::Pos", false), ("shuyi_core::Pos", false)]
        );
    }

    const LISTED: &str = r#"(crates: {
        "shuyi-core": (plain: ["shuyi_core::grid::Pos"], closed: ["shuyi_core::grid::Dir"]),
    })"#;

    #[test]
    fn a_new_exhaustive_struct_fails_and_names_its_path() {
        let crates = [crate_with("shuyi-core", CORE)];
        let file = r#"(crates: {"shuyi-core": (closed: ["shuyi_core::grid::Dir"])})"#;
        let problems = check(Some(file), None, &crates);
        assert_eq!(
            problems,
            [Problem::Unlisted {
                krate: "shuyi-core".to_string(),
                path: "shuyi_core::grid::Pos".to_string(),
            }]
        );
        let error = error(&problems).unwrap();
        assert!(
            error.contains("shuyi-core: `shuyi_core::grid::Pos` is an exhaustive public type"),
            "{error}"
        );
        assert!(
            error.contains("`#[non_exhaustive]` with a constructor"),
            "{error}"
        );
    }

    #[test]
    fn listed_as_plain_it_passes() {
        let crates = [crate_with("shuyi-core", CORE)];
        assert_eq!(check(Some(LISTED), None, &crates), []);
        assert_eq!(error(&[]), None);
        assert_eq!(section(&[]), "");
    }

    #[test]
    fn no_file_lists_nothing() {
        let crates = [crate_with("shuyi-core", CORE)];
        let problems = check(None, None, &crates);
        assert_eq!(problems.len(), 2, "{problems:?}");
    }

    #[test]
    fn a_stale_path_fails_and_so_does_a_non_exhaustive_one() {
        let crates = [crate_with("shuyi-core", CORE)];
        let file = r#"(crates: {"shuyi-core": (
            plain: ["shuyi_core::grid::Pos", "shuyi_core::grid::Rect"],
            closed: ["shuyi_core::grid::Dir"],
            open: ["shuyi_core::bots::plan::PlanSettings"],
        )})"#;
        let problems = check(Some(file), None, &crates);
        assert_eq!(
            problems,
            [
                Problem::Moved {
                    krate: "shuyi-core".to_string(),
                    path: "shuyi_core::bots::plan::PlanSettings".to_string(),
                },
                Problem::Gone {
                    krate: "shuyi-core".to_string(),
                    path: "shuyi_core::grid::Rect".to_string(),
                },
            ]
        );
        let text = section(&problems);
        assert!(text.contains("**Exhaustive types, fail:**"), "{text}");
        assert!(
            text.contains("`shuyi_core::bots::plan::PlanSettings` is `#[non_exhaustive]` now"),
            "{text}"
        );
        assert!(
            text.contains("`shuyi_core::grid::Rect` is listed in tools/exhaustive.ron but is no"),
            "{text}"
        );
    }

    #[test]
    fn a_path_is_listed_under_its_own_crate() {
        let crates = [crate_with("shuyi-core", CORE), crate_with("shuyi-ui", &[])];
        let file = r#"(crates: {
            "shuyi-core": (closed: ["shuyi_core::grid::Dir"]),
            "shuyi-ui": (plain: ["shuyi_core::grid::Pos"]),
        })"#;
        let problems = check(Some(file), None, &crates);
        assert_eq!(
            problems,
            [
                Problem::Unlisted {
                    krate: "shuyi-core".to_string(),
                    path: "shuyi_core::grid::Pos".to_string(),
                },
                Problem::Gone {
                    krate: "shuyi-ui".to_string(),
                    path: "shuyi_core::grid::Pos".to_string(),
                },
            ]
        );
    }

    #[test]
    fn an_open_path_the_base_lacks_fails_and_one_it_has_passes() {
        let crates = [crate_with("shuyi-core", CORE)];
        let head = r#"(crates: {"shuyi-core": (
            closed: ["shuyi_core::grid::Dir"],
            open: ["shuyi_core::grid::Pos"],
        )})"#;
        let base_had = r#"(crates: {"shuyi-core": (open: ["shuyi_core::grid::Pos"])})"#;
        assert_eq!(check(Some(head), Some(base_had), &crates), []);
        let base_lacked = r#"(crates: {"shuyi-core": (plain: ["shuyi_core::grid::Pos"])})"#;
        let problems = check(Some(head), Some(base_lacked), &crates);
        assert_eq!(
            problems,
            [Problem::NewOpen {
                krate: "shuyi-core".to_string(),
                path: "shuyi_core::grid::Pos".to_string(),
            }]
        );
        assert!(
            error(&problems)
                .unwrap()
                .contains("the `open` list only shrinks"),
            "{problems:?}"
        );
        // A base with no file, or none of this crate, has no `open` list.
        assert_eq!(check(Some(head), None, &crates), []);
        let other = "(crates: {})";
        assert_eq!(check(Some(head), Some(other), &crates).len(), 1);
    }

    #[test]
    fn a_path_listed_twice_or_a_crate_outside_the_catalog_fails() {
        let crates = [crate_with("shuyi-core", CORE)];
        let file = r#"(crates: {
            "shuyi-core": (plain: ["shuyi_core::grid::Pos"], closed: ["shuyi_core::grid::Dir", "shuyi_core::grid::Pos"]),
            "shuyi-gone": (),
        })"#;
        let problems = check(Some(file), None, &crates);
        let error = error(&problems).unwrap();
        assert_eq!(problems.len(), 2, "{error}");
        assert!(
            error.contains("`shuyi-gone` is no crate of `API_CRATES`"),
            "{error}"
        );
        assert!(
            error.contains(
                "`shuyi_core::grid::Pos` is listed twice under `shuyi-core` (plain and closed)"
            ),
            "{error}"
        );
    }

    #[test]
    fn a_file_that_does_not_parse_fails() {
        for (head, base, starts) in [
            ("(crates: {\"a\": (wide: [])})", None, FILE.to_string()),
            (
                "(crates: {})",
                Some("(kinds: 1)"),
                format!("the base's {FILE}"),
            ),
        ] {
            let problems = check(Some(head), base, &[]);
            assert!(
                matches!(&problems[..], [Problem::File(why)] if why.starts_with(&starts)),
                "{problems:?}"
            );
        }
    }
}
