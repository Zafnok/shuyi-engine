//! Plural and ordinal categories of whole numbers, by language: which of
//! CLDR's categories (`one`, `few`, `other`…) a number falls in, so a
//! message ([`Message`](super::Message)) picks the branch its language
//! needs ("1 key", "2 keys"; "21 ключ", "22 ключа", "25 ключей").
//!
//! Written from CLDR 49 (`release-49`, 2026-10-21), its
//! `common/supplemental/plurals.xml` and `ordinals.xml`, for whole numbers
//! only (CLDR's operands `v`, `w`, `f`, `t` and `e` are 0). Unicode's data,
//! under Unicode-3.0: `CLDR-LICENSE.txt` beside this file. The test
//! fixture `plural_samples.txt` holds CLDR's integer samples for each
//! language, kind and category, and each gets its category.
//!
//! A language not in the table has only `other`. Adding one is a row of its table
//! and its samples in the fixture.

/// One of CLDR's plural categories. Which ones a language has is
/// [`categories`]; every language has [`Other`](Self::Other).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Category {
    /// `zero`.
    Zero,
    /// `one`.
    One,
    /// `two`.
    Two,
    /// `few`.
    Few,
    /// `many`.
    Many,
    /// `other`: what every language has, and what a message falls back
    /// to.
    Other,
}

impl Category {
    /// Every category, in CLDR's order.
    pub const ALL: &[Category] = &[
        Self::Zero,
        Self::One,
        Self::Two,
        Self::Few,
        Self::Many,
        Self::Other,
    ];

    /// Its name in a message and in CLDR (`few`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Zero => "zero",
            Self::One => "one",
            Self::Two => "two",
            Self::Few => "few",
            Self::Many => "many",
            Self::Other => "other",
        }
    }

    /// The category named `name`, if one is.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|c| c.name() == name)
    }
}

/// Which rules: counting (`plural`, "3 keys") or ranking (`selectordinal`,
/// "3rd").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Kind {
    /// Cardinal numbers: a message's `plural`.
    Cardinal,
    /// Ordinal numbers: a message's `selectordinal`.
    Ordinal,
}

/// A language's cardinal rule for whole numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cardinal {
    /// `other` only (ja, ko, zh).
    Other,
    /// `one` for 1 (en, de).
    One,
    /// `one` for 1, `many` for a non-zero multiple of a million (es, it,
    /// pt-PT).
    OneMillion,
    /// `one` for 0 and 1, `many` for a non-zero multiple of a million
    /// (fr, pt).
    ZeroOneMillion,
    /// `one`, `few` and `many` by the last digits (ru, uk).
    EastSlavic,
    /// `one` for 1 only, `few` and `many` by the last digits (pl).
    Polish,
}

/// A language's ordinal rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ordinal {
    /// `other` only.
    Other,
    /// 1st, 2nd, 3rd, 4th (en).
    English,
    /// `one` for 1 (fr).
    One,
    /// `one` for numbers ending in 1 or 3, but 11 (es).
    Spanish,
    /// `many` for 8, 11, 80 and 800 (it).
    Italian,
    /// `few` for numbers ending in 3, but 13 (uk).
    Ukrainian,
}

/// The languages with rules, by tag: a tag is looked up whole first
/// (`pt-PT`), then by its language (`pt-BR` uses `pt`).
const TABLE: &[(&str, Cardinal, Ordinal)] = &[
    ("de", Cardinal::One, Ordinal::Other),
    ("en", Cardinal::One, Ordinal::English),
    ("es", Cardinal::OneMillion, Ordinal::Spanish),
    ("fr", Cardinal::ZeroOneMillion, Ordinal::One),
    ("it", Cardinal::OneMillion, Ordinal::Italian),
    ("ja", Cardinal::Other, Ordinal::Other),
    ("ko", Cardinal::Other, Ordinal::Other),
    ("pl", Cardinal::Polish, Ordinal::Other),
    ("pt", Cardinal::ZeroOneMillion, Ordinal::Other),
    ("pt-PT", Cardinal::OneMillion, Ordinal::Other),
    ("ru", Cardinal::EastSlavic, Ordinal::Other),
    ("uk", Cardinal::EastSlavic, Ordinal::Ukrainian),
    ("zh", Cardinal::Other, Ordinal::Other),
];

/// The rules of `lang`, a BCP 47 tag: its row, else its language's.
fn rules(lang: &str) -> (Cardinal, Ordinal) {
    let language = lang.split('-').next().unwrap_or(lang);
    let row = |tag: &str| TABLE.iter().find(|(t, ..)| *t == tag);
    row(lang)
        .or_else(|| row(language))
        .map_or((Cardinal::Other, Ordinal::Other), |&(_, c, o)| (c, o))
}

/// The category `n` counts as in `lang` ("1 key", "2 keys").
pub fn cardinal(lang: &str, n: u64) -> Category {
    let (ten, hundred) = (n % 10, n % 100);
    let million = n != 0 && n.is_multiple_of(1_000_000);
    match rules(lang).0 {
        Cardinal::Other => Category::Other,
        Cardinal::OneMillion | Cardinal::ZeroOneMillion if million => Category::Many,
        Cardinal::One | Cardinal::OneMillion => one_if(n == 1),
        Cardinal::ZeroOneMillion => one_if(n <= 1),
        Cardinal::EastSlavic if ten == 1 && hundred != 11 => Category::One,
        Cardinal::Polish if n == 1 => Category::One,
        Cardinal::EastSlavic | Cardinal::Polish
            if (2..=4).contains(&ten) && !(12..=14).contains(&hundred) =>
        {
            Category::Few
        }
        Cardinal::EastSlavic | Cardinal::Polish => Category::Many,
    }
}

/// The category `n` ranks as in `lang` ("1st", "2nd", "3rd", "4th").
pub fn ordinal(lang: &str, n: u64) -> Category {
    let (ten, hundred) = (n % 10, n % 100);
    match rules(lang).1 {
        Ordinal::English if ten == 1 && hundred != 11 => Category::One,
        Ordinal::English if ten == 2 && hundred != 12 => Category::Two,
        Ordinal::English | Ordinal::Ukrainian if ten == 3 && hundred != 13 => Category::Few,
        Ordinal::One => one_if(n == 1),
        Ordinal::Spanish => one_if((ten == 1 || ten == 3) && hundred != 11),
        Ordinal::Italian if matches!(n, 8 | 11 | 80 | 800) => Category::Many,
        Ordinal::Other | Ordinal::English | Ordinal::Italian | Ordinal::Ukrainian => {
            Category::Other
        }
    }
}

/// `one` if `one`, else `other`.
fn one_if(one: bool) -> Category {
    if one { Category::One } else { Category::Other }
}

/// The categories `lang`'s numbers of `kind` fall in, in CLDR's order:
/// what a message in `lang` may have branches for. Always ends with
/// `other`, which every plural keeps.
pub fn categories(lang: &str, kind: Kind) -> &'static [Category] {
    use Category::{Few, Many, One, Other, Two};
    let (cardinal, ordinal) = rules(lang);
    match kind {
        Kind::Cardinal => match cardinal {
            Cardinal::Other => &[Other],
            Cardinal::One => &[One, Other],
            Cardinal::OneMillion | Cardinal::ZeroOneMillion => &[One, Many, Other],
            Cardinal::EastSlavic | Cardinal::Polish => &[One, Few, Many, Other],
        },
        Kind::Ordinal => match ordinal {
            Ordinal::Other => &[Other],
            Ordinal::English => &[One, Two, Few, Other],
            Ordinal::One | Ordinal::Spanish => &[One, Other],
            Ordinal::Italian => &[Many, Other],
            Ordinal::Ukrainian => &[Few, Other],
        },
    }
}

/// The categories a plural of `kind` in `lang` is expected to have
/// branches for: [`categories`] without the cardinal `many` of French,
/// Spanish, Italian and Portuguese, which only whole numbers of a million
/// or more pick (ADR-0127 §8). `lang-status` lists a plural that lacks
/// one of these.
pub(crate) fn expected(lang: &str, kind: Kind) -> &'static [Category] {
    match (kind, rules(lang).0) {
        (Kind::Cardinal, Cardinal::OneMillion | Cardinal::ZeroOneMillion) => {
            &[Category::One, Category::Other]
        }
        _ => categories(lang, kind),
    }
}

#[cfg(test)]
mod tests;
