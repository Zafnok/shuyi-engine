//! The rule against neighbouring numbers (ticket 3503, ADR-0150): a
//! constructor of a type that can grow takes no two neighbouring arguments a
//! caller could swap without a word.
//!
//! A literal names each field; a constructor names none, so
//! `PlanSettings::new(4, 8, 2, 16, 10)` reads nothing, and two of its numbers
//! can swap and still compile. The check reads the head's listing of each
//! crate (`cargo-public-api`, which shows argument types but not their names)
//! and finds each `pub fn` of a `#[non_exhaustive]` type that returns `Self`,
//! takes no receiver, and takes two neighbouring integers (of any widths), two
//! floats, or two tuples of one type. Each must be in [`FILE`] with why it may
//! stay (a width then a height), and each entry there must still be such a
//! constructor: a stale one fails too.
//!
//! The listing lines of a type's block follow its `pub struct` or `pub enum`
//! line, and a type re-exported at another path is listed again there with
//! its methods at the path that defines them: a function belongs to the type
//! whose block it is in.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde::Deserialize;

use super::CrateApi;

/// The constructors allowed to break the rule, each with why.
pub const FILE: &str = "tools/constructors.ron";

/// A `pub fn` of a `#[non_exhaustive]` type that returns `Self` and takes no
/// receiver, as the listing shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constructor {
    /// Its path, without generic arguments
    /// (`shuyi_assets::audio::MusicSwitch::new`).
    pub path: String,
    /// Its listing line.
    pub line: String,
    /// The first two neighbouring arguments a caller could swap, if it takes
    /// two: their types, and what they are (`integers`, `floats`, `tuples`).
    pub pair: Option<Pair>,
}

/// Two neighbouring arguments a caller could swap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    pub first: String,
    pub second: String,
    pub kind: &'static str,
}

/// The constructors of `listing`'s `#[non_exhaustive]` types, in its order,
/// each path once.
pub fn constructors(listing: &[String]) -> Vec<Constructor> {
    let mut found = Vec::new();
    let mut seen = BTreeSet::new();
    let mut in_growing = false;
    for line in listing {
        if let Some(growing) = type_header(line) {
            in_growing = growing;
            continue;
        }
        if !in_growing {
            continue;
        }
        let Some(function) = function(line) else {
            continue;
        };
        if function.args.first().is_some_and(|arg| is_receiver(arg)) || !function.returns_self {
            continue;
        }
        if seen.insert(function.path.clone()) {
            found.push(Constructor {
                pair: neighbours(&function.args),
                path: function.path,
                line: line.clone(),
            });
        }
    }
    found
}

/// Whether `line` opens a block, and whether the block is a
/// `#[non_exhaustive]` struct or enum's: `None` for a line inside a block.
fn type_header(line: &str) -> Option<bool> {
    let (growing, rest) = match line.strip_prefix("#[non_exhaustive] ") {
        Some(rest) => (true, rest),
        None => (false, line),
    };
    let rest = rest.strip_prefix("pub ")?;
    if rest.starts_with("struct ") || rest.starts_with("enum ") {
        Some(growing)
    } else if ["union ", "trait ", "mod ", "type ", "macro ", "static "]
        .iter()
        .any(|word| rest.starts_with(word))
    {
        Some(false)
    } else {
        None
    }
}

/// A function's listing line, read.
struct Function {
    path: String,
    args: Vec<String>,
    returns_self: bool,
}

/// `line` as a `pub fn`: its path without generics, its arguments' types,
/// and whether it returns `Self` (or its own type's path).
fn function(line: &str) -> Option<Function> {
    let mut rest = line.strip_prefix("pub ")?;
    for word in ["const ", "async ", "unsafe ", "extern \"C\" "] {
        rest = rest.strip_prefix(word).unwrap_or(rest);
    }
    let rest = rest.strip_prefix("fn ")?;
    let open = top_level(rest, '(')?;
    let close = open + closing(&rest[open..])?;
    let path = without_generics(&rest[..open]);
    let args = split_args(&rest[open + 1..close]);
    let after = rest[close + 1..].trim_start();
    let returned = after.strip_prefix("-> ").map(|ret| {
        let end = ret.find(" where ").unwrap_or(ret.len());
        ret[..end].trim()
    });
    let own_type = path.rsplit_once("::").map(|(owner, _)| owner);
    let returns_self = returned
        .is_some_and(|ret| ret == "Self" || Some(without_generics(ret).as_str()) == own_type);
    Some(Function {
        path,
        args,
        returns_self,
    })
}

/// Each character of `text` with its byte offset and whether it opens
/// (`Some(true)`) or closes (`Some(false)`) a bracket of `brackets`: the `>`
/// of an arrow (`Fn(u32) -> u32`) closes nothing.
fn brackets<'a>(
    text: &'a str,
    brackets: &'a [(char, char)],
) -> impl Iterator<Item = (usize, char, Option<bool>)> + 'a {
    let mut previous = ' ';
    text.char_indices().map(move |(at, c)| {
        let arrow = c == '>' && previous == '-';
        previous = c;
        let bracket = if brackets.iter().any(|(open, _)| *open == c) {
            Some(true)
        } else if !arrow && brackets.iter().any(|(_, close)| *close == c) {
            Some(false)
        } else {
            None
        };
        (at, c, bracket)
    })
}

const ANGLES: &[(char, char)] = &[('<', '>')];
const ALL: &[(char, char)] = &[('(', ')'), ('<', '>'), ('[', ']')];

/// The byte offset of the first `wanted` outside any `<…>`.
fn top_level(text: &str, wanted: char) -> Option<usize> {
    let mut depth = 0_usize;
    for (at, c, bracket) in brackets(text, ANGLES) {
        match bracket {
            Some(true) => depth += 1,
            Some(false) => depth = depth.saturating_sub(1),
            None if c == wanted && depth == 0 => return Some(at),
            None => {}
        }
    }
    None
}

/// The byte offset of the `)` that closes the `(` `text` starts with.
fn closing(text: &str) -> Option<usize> {
    let mut depth = 0_usize;
    for (at, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

/// `path` with every `<…>` taken out: `shuyi_ui::Bag<T>::new` is
/// `shuyi_ui::Bag::new`.
fn without_generics(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut depth = 0_usize;
    for (_, c, bracket) in brackets(path, ANGLES) {
        match bracket {
            Some(true) => depth += 1,
            Some(false) => depth = depth.saturating_sub(1),
            None if depth == 0 => out.push(c),
            None => {}
        }
    }
    out.trim().to_string()
}

/// The arguments of an argument list, split at its top-level commas.
fn split_args(list: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut depth = 0_usize;
    let mut start = 0;
    for (at, c, bracket) in brackets(list, ALL) {
        match bracket {
            Some(true) => depth += 1,
            Some(false) => depth = depth.saturating_sub(1),
            None if c == ',' && depth == 0 => {
                args.push(list[start..at].trim().to_string());
                start = at + 1;
            }
            None => {}
        }
    }
    let last = list[start..].trim();
    if !last.is_empty() {
        args.push(last.to_string());
    }
    args
}

/// Whether an argument is a receiver: `self`, `&self`, `&mut self`, or
/// `self: <type>`.
fn is_receiver(arg: &str) -> bool {
    let arg = arg.strip_prefix("mut ").unwrap_or(arg);
    arg == "self" || arg.starts_with("self:") || (arg.starts_with('&') && arg.ends_with("self"))
}

const INTEGERS: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
];
const FLOATS: &[&str] = &["f32", "f64"];

/// What a caller could swap `first` with, if `second` is one: two integers
/// of any widths, two floats, or two tuples of one type.
fn swappable(first: &str, second: &str) -> Option<&'static str> {
    if INTEGERS.contains(&first) && INTEGERS.contains(&second) {
        Some("integers")
    } else if FLOATS.contains(&first) && FLOATS.contains(&second) {
        Some("floats")
    } else if first.starts_with('(') && first != "()" && first == second {
        Some("tuples")
    } else {
        None
    }
}

/// The first two neighbouring arguments of `args` a caller could swap.
fn neighbours(args: &[String]) -> Option<Pair> {
    args.windows(2).find_map(|pair| {
        swappable(&pair[0], &pair[1]).map(|kind| Pair {
            first: pair[0].clone(),
            second: pair[1].clone(),
            kind,
        })
    })
}

/// [`FILE`], as read.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Allowed {
    allowed: Vec<Entry>,
}

/// One constructor allowed to break the rule.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    /// Its path in the listing, without generic arguments.
    path: String,
    /// Why it may stay ("a width then a height").
    why: String,
}

/// What the check found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// [`FILE`] didn't parse, or an entry is wrong (no reason, listed twice).
    File(String),
    /// A constructor breaks the rule and isn't listed.
    Breaks {
        krate: String,
        constructor: Constructor,
    },
    /// A listed path is a constructor that no longer breaks the rule.
    Mended(String),
    /// A listed path is no constructor of a growing type in any listing.
    Gone(String),
}

/// The problems of `crates`' constructors against `file`, the text of
/// [`FILE`] (`None`: there is none, and nothing is allowed).
pub fn check(file: Option<&str>, crates: &[CrateApi]) -> Vec<Problem> {
    let mut problems = Vec::new();
    let entries = match file.map(ron::from_str::<Allowed>) {
        None => Vec::new(),
        Some(Ok(allowed)) => allowed.allowed,
        Some(Err(e)) => return vec![Problem::File(format!("{FILE}: {e}"))],
    };
    let mut allowed: BTreeMap<&str, &str> = BTreeMap::new();
    for entry in &entries {
        if entry.why.trim().is_empty() {
            problems.push(Problem::File(format!(
                "{FILE}: `{}` has no reason: say why it may stay",
                entry.path
            )));
        }
        if allowed.insert(&entry.path, &entry.why).is_some() {
            problems.push(Problem::File(format!(
                "{FILE}: `{}` is listed twice",
                entry.path
            )));
        }
    }
    // Each constructor's path, and whether any listing of it breaks the rule.
    let mut found: BTreeMap<&str, bool> = BTreeMap::new();
    for krate in crates {
        for constructor in &krate.constructors {
            let breaks = found.entry(&constructor.path).or_insert(false);
            *breaks |= constructor.pair.is_some();
            if constructor.pair.is_some() && !allowed.contains_key(constructor.path.as_str()) {
                problems.push(Problem::Breaks {
                    krate: krate.name.clone(),
                    constructor: constructor.clone(),
                });
            }
        }
    }
    for path in allowed.keys() {
        match found.get(path) {
            Some(true) => {}
            Some(false) => problems.push(Problem::Mended((*path).to_string())),
            None => problems.push(Problem::Gone((*path).to_string())),
        }
    }
    problems
}

impl Problem {
    /// The problem, in a line or two.
    fn describe(&self) -> String {
        match self {
            Self::File(why) => why.clone(),
            Self::Breaks { krate, constructor } => {
                let pair = constructor.pair.as_ref().map_or_else(String::new, |pair| {
                    format!(
                        " takes two neighbouring {} (`{}, {}`)",
                        pair.kind, pair.first, pair.second
                    )
                });
                format!(
                    "{krate}: `{}`{pair}\n    {}",
                    constructor.path, constructor.line
                )
            }
            Self::Mended(path) => format!(
                "`{path}` is listed in {FILE} but no longer takes two neighbouring numbers or \
                 tuples: take its entry out"
            ),
            Self::Gone(path) => format!(
                "`{path}` is listed in {FILE} but is no constructor of a `#[non_exhaustive]` \
                 type in any listing: take its entry out, or write its new path"
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
        "{} problem(s) with constructors that take two neighbouring numbers (ADR-0150):\n\
         {text}A caller can swap two such arguments without a word. Name them: a named \
         constructor (`MusicSwitch::crossfade`), a `with_<field>`, or a plain value whose \
         fields name them. Where the order is the meaning (a width then a height), list the \
         path in {FILE} with why.",
        problems.len()
    ))
}

/// The summary's section for `problems`: nothing when there are none.
pub fn section(problems: &[Problem]) -> String {
    if problems.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n**Constructors, fail:**\n\n");
    for problem in problems {
        let _ = writeln!(
            out,
            "- {}",
            problem.describe().replace("\n    ", " — listed as: ")
        );
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

    const SWITCH: &[&str] = &[
        "#[non_exhaustive] pub struct shuyi_assets::audio::MusicSwitch",
        "pub shuyi_assets::audio::MusicSwitch::in_ms: u32",
        "impl shuyi_assets::audio::MusicSwitch",
        "pub const fn shuyi_assets::audio::MusicSwitch::new(u32, u32, bool) -> Self",
        "pub const fn shuyi_assets::audio::MusicSwitch::crossfade(u32) -> Self",
    ];

    fn paths(found: &[Constructor]) -> Vec<(&str, Option<&'static str>)> {
        found
            .iter()
            .map(|c| (c.path.as_str(), c.pair.as_ref().map(|pair| pair.kind)))
            .collect()
    }

    #[test]
    fn two_integers_break_the_rule_and_one_does_not() {
        assert_eq!(
            paths(&constructors(&listing(SWITCH))),
            [
                ("shuyi_assets::audio::MusicSwitch::new", Some("integers")),
                ("shuyi_assets::audio::MusicSwitch::crossfade", None),
            ]
        );
    }

    #[test]
    fn integers_of_mixed_widths_break_it() {
        let found = constructors(&listing(&[
            "#[non_exhaustive] pub struct shuyi_assets::audio::MusicCue",
            "pub fn shuyi_assets::audio::MusicCue::new(alloc::string::String, u8, u32, \
             shuyi_assets::audio::CreditRef) -> Self",
        ]));
        let pair = found[0].pair.clone().unwrap();
        assert_eq!((pair.first.as_str(), pair.second.as_str()), ("u8", "u32"));
        assert_eq!(pair.kind, "integers");
    }

    #[test]
    fn two_floats_and_two_tuples_of_one_type_break_it() {
        let found = constructors(&listing(&[
            "#[non_exhaustive] pub struct shuyi_ui::Clock",
            "pub fn shuyi_ui::Clock::new(u32, f32, f64) -> Self",
            "#[non_exhaustive] pub struct shuyi_ui::input::pad::PadState",
            "pub fn shuyi_ui::input::pad::PadState::new(u32, (f32, f32), (f32, f32)) -> Self",
            "pub fn shuyi_ui::input::pad::PadState::mixed((f32, f32), (i32, i32)) -> Self",
        ]));
        assert_eq!(
            paths(&found),
            [
                ("shuyi_ui::Clock::new", Some("floats")),
                ("shuyi_ui::input::pad::PadState::new", Some("tuples")),
                ("shuyi_ui::input::pad::PadState::mixed", None),
            ]
        );
    }

    #[test]
    fn an_integer_beside_a_float_or_a_string_does_not() {
        let found = constructors(&listing(&[
            "#[non_exhaustive] pub struct shuyi_ui::Mix",
            "pub fn shuyi_ui::Mix::new(u32, f32, &str, &str, u8) -> Self",
        ]));
        assert_eq!(paths(&found), [("shuyi_ui::Mix::new", None)]);
    }

    #[test]
    fn a_self_method_is_no_constructor() {
        let found = constructors(&listing(&[
            "#[non_exhaustive] pub struct shuyi_ui::Mix",
            "pub const fn shuyi_ui::Mix::with_gain(self, u8) -> Self",
            "pub fn shuyi_ui::Mix::scaled(&self, u32, u32) -> Self",
            "pub fn shuyi_ui::Mix::grow(&mut self, u32, u32) -> Self",
            "pub fn shuyi_ui::Mix::boxed(self: alloc::boxed::Box<Self>, u32, u32) -> Self",
        ]));
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn what_does_not_return_self_is_no_constructor() {
        let found = constructors(&listing(&[
            "#[non_exhaustive] pub struct shuyi_ui::Mix",
            "pub fn shuyi_ui::Mix::area(u32, u32) -> u32",
            "pub fn shuyi_ui::Mix::parse(u32, u32) -> core::option::Option<Self>",
            "pub fn shuyi_ui::Mix::nothing(u32, u32)",
        ]));
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_constructor_naming_its_own_type_returns_self() {
        let found = constructors(&listing(&[
            "#[non_exhaustive] pub struct shuyi_ui::Bag<T>",
            "pub fn shuyi_ui::Bag<T>::new(u32, u32) -> shuyi_ui::Bag<T>",
        ]));
        assert_eq!(paths(&found), [("shuyi_ui::Bag::new", Some("integers"))]);
    }

    #[test]
    fn generics_leave_the_path() {
        let found = constructors(&listing(&[
            "#[non_exhaustive] pub struct shuyi_core::ease::Tween<T, E>",
            "impl<T, E> shuyi_core::ease::Tween<T, E>",
            "pub const fn shuyi_core::ease::Tween<T, E>::new<F: core::ops::Fn(u32) -> u32>\
             (T, impl core::ops::Fn(u8, u8) -> u8, u32, u32) -> Self where E: core::clone::Clone",
        ]));
        assert_eq!(
            paths(&found),
            [("shuyi_core::ease::Tween::new", Some("integers"))]
        );
    }

    #[test]
    fn a_constructor_of_an_exhaustive_type_passes() {
        let found = constructors(&listing(&[
            "pub struct shuyi_core::grid::Pos",
            "pub const fn shuyi_core::grid::Pos::new(i32, i32) -> Self",
            "#[non_exhaustive] pub enum shuyi_core::Kind",
            "pub shuyi_core::Kind::Strike",
            "pub fn shuyi_core::Kind::of(u8, u8) -> Self",
            "pub trait shuyi_core::Rule",
            "pub fn shuyi_core::Rule::make(u8, u8) -> Self",
        ]));
        assert_eq!(paths(&found), [("shuyi_core::Kind::of", Some("integers"))]);
    }

    #[test]
    fn a_re_exported_type_is_listed_once() {
        let found = constructors(&listing(&[
            "#[non_exhaustive] pub struct shuyi_core::threat::Strike",
            "pub const fn shuyi_core::threat::Strike::new(i64, i64, i64) -> Self",
            "#[non_exhaustive] pub struct shuyi_core::Strike",
            "pub const fn shuyi_core::threat::Strike::new(i64, i64, i64) -> Self",
        ]));
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn an_unlisted_break_names_its_path() {
        let crates = [crate_with("shuyi-assets", SWITCH)];
        let problems = check(None, &crates);
        assert_eq!(problems.len(), 1, "{problems:?}");
        let error = error(&problems).unwrap();
        assert!(
            error.contains(
                "shuyi-assets: `shuyi_assets::audio::MusicSwitch::new` takes two \
                            neighbouring integers (`u32, u32`)"
            ),
            "{error}"
        );
    }

    #[test]
    fn a_listed_entry_passes() {
        let crates = [crate_with("shuyi-assets", SWITCH)];
        let file = r#"(allowed: [
            (path: "shuyi_assets::audio::MusicSwitch::new", why: "a fade out, then in"),
        ])"#;
        assert_eq!(check(Some(file), &crates), []);
        assert_eq!(error(&[]), None);
        assert_eq!(section(&[]), "");
    }

    #[test]
    fn a_stale_entry_fails() {
        let crates = [crate_with("shuyi-assets", SWITCH)];
        let file = r#"(allowed: [
            (path: "shuyi_assets::audio::MusicSwitch::new", why: "a fade out, then in"),
            (path: "shuyi_assets::audio::MusicSwitch::crossfade", why: "one number"),
            (path: "shuyi_assets::audio::Fade::new", why: "gone"),
        ])"#;
        assert_eq!(
            check(Some(file), &crates),
            [
                Problem::Gone("shuyi_assets::audio::Fade::new".to_string()),
                Problem::Mended("shuyi_assets::audio::MusicSwitch::crossfade".to_string()),
            ]
        );
    }

    #[test]
    fn an_entry_needs_a_reason_and_one_line() {
        let crates = [crate_with("shuyi-assets", SWITCH)];
        let file = r#"(allowed: [
            (path: "shuyi_assets::audio::MusicSwitch::new", why: " "),
            (path: "shuyi_assets::audio::MusicSwitch::new", why: "twice"),
        ])"#;
        let problems = check(Some(file), &crates);
        assert_eq!(problems.len(), 2, "{problems:?}");
        let error = error(&problems).unwrap();
        assert!(error.contains("has no reason"), "{error}");
        assert!(error.contains("is listed twice"), "{error}");
    }

    #[test]
    fn a_file_that_does_not_parse_fails() {
        let problems = check(Some("(allowed: [(path: \"a\")])"), &[]);
        assert!(
            matches!(&problems[..], [Problem::File(why)] if why.starts_with(FILE)),
            "{problems:?}"
        );
    }

    #[test]
    fn the_section_shows_each_problem() {
        let crates = [crate_with("shuyi-assets", SWITCH)];
        let text = section(&check(None, &crates));
        assert!(text.contains("**Constructors, fail:**"), "{text}");
        assert!(
            text.contains("- shuyi-assets: `shuyi_assets::audio::MusicSwitch::new`"),
            "{text}"
        );
    }
}
