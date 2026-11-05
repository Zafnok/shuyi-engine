//! A language's tag ([`LangCode`]): BCP 47 (RFC 5646) in the subset a game
//! names its languages with, `pt-BR`, `zh-Hans`, `es-419`, written one way
//! only, and the reasons a text isn't one ([`TagError`]).
use crate::prelude::*;

use core::borrow::Borrow;
use core::fmt;

/// Longest [`LangCode`], in characters.
const MAX_CODE_CHARS: usize = 35;

/// The tag of the source language when a game declares none: English.
pub(super) const DEFAULT_SOURCE: &str = "en";

/// What starts a tag's private-use part.
const PRIVATE_USE: &str = "x";

/// A private-use part's start inside a tag.
const PRIVATE_USE_PART: &str = "-x-";

/// A language's tag, the name of its folder among the languages: BCP 47
/// (RFC 5646) in this subset, its subtags joined by `-`:
///
/// - the language: 2 to 8 letters (`en`, `ja`);
/// - a script, if any: 4 letters (`zh-Hans`);
/// - a region, if any: 2 letters or 3 digits (`pt-BR`, `es-419`);
/// - variants, each once: 5 to 8 letters or digits, or a digit and 3
///   letters or digits (`de-CH-1996`);
/// - a private-use part, if any: `x` and one or more subtags of 1 to 8
///   letters or digits (`en-x-pseudo`).
///
/// At most 35 characters in all. A tag has one spelling: the language,
/// variants and private use in lower case, the script in title case and the
/// region in upper case. A tag compares, orders and hashes as its text, so
/// a map keyed by tags is looked up by `&str` too.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LangCode(String);

/// Why a text isn't a language tag ([`LangCode::parse`]).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TagError {
    /// There is no text.
    Empty,
    /// The text is longer than 35 characters.
    TooLong,
    /// The text holds a character a tag can't: anything but letters, digits
    /// and `-`.
    Character(char),
    /// Two `-` side by side, or one at an end.
    EmptySubtag,
    /// A POSIX locale with no language: `C` or `POSIX`.
    NoLanguage,
    /// The first subtag isn't a language of 2 to 8 letters.
    Language(String),
    /// A grandfathered tag (`i-klingon`), which the subset leaves out.
    Grandfathered,
    /// 3 letters right after the language (`zh-yue`): an extended language
    /// subtag, which the subset leaves out.
    Extlang(String),
    /// An extension (`-u-ca-gregory`, `-t-`), which the subset leaves out.
    Extension(char),
    /// A variant written twice.
    RepeatedVariant(String),
    /// `x` with nothing after it, or a private-use subtag longer than 8.
    PrivateUse,
    /// A subtag where it can't be: a script after the region, say.
    Misplaced(String),
}

impl fmt::Display for TagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("a tag can't be empty"),
            Self::TooLong => write!(f, "a tag is at most {MAX_CODE_CHARS} characters"),
            Self::Character(c) => write!(f, "{c:?} can't be in a tag: only letters, digits and -"),
            Self::EmptySubtag => f.write_str("a tag has no empty part: no -- and no - at an end"),
            Self::NoLanguage => f.write_str("C and POSIX name no language"),
            Self::Language(subtag) => {
                write!(
                    f,
                    "a tag starts with its language, 2 to 8 letters, not {subtag:?}"
                )
            }
            Self::Grandfathered => f.write_str("an old tag starting with i- isn't taken"),
            Self::Extlang(subtag) => write!(
                f,
                "{subtag:?} after the language is an extended language, which isn't taken; \
                 write the language alone"
            ),
            Self::Extension(singleton) => write!(
                f,
                "extensions (-{singleton}-) aren't taken; only a private-use part (-x-)"
            ),
            Self::RepeatedVariant(subtag) => write!(f, "the variant {subtag:?} is written twice"),
            Self::PrivateUse => f.write_str(
                "a private-use part is x and one or more parts of 1 to 8 letters or digits",
            ),
            Self::Misplaced(subtag) => write!(
                f,
                "{subtag:?} can't be there: a tag is its language, then a script, a region, \
                 variants and a private-use part, each if it has one"
            ),
        }
    }
}

/// Whether `s` is `len` ASCII letters long, any case.
fn letters(s: &str, len: core::ops::RangeInclusive<usize>) -> bool {
    len.contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphabetic())
}

/// Whether `s` is a script subtag: 4 letters.
fn is_script(s: &str) -> bool {
    letters(s, 4..=4)
}

/// Whether `s` is a region subtag: 2 letters or 3 digits.
fn is_region(s: &str) -> bool {
    letters(s, 2..=2) || (s.len() == 3 && s.bytes().all(|b| b.is_ascii_digit()))
}

/// Whether `s` is a variant subtag: 5 to 8 letters or digits, or a digit
/// and 3 letters or digits.
fn is_variant(s: &str) -> bool {
    let alnum = s.bytes().all(|b| b.is_ascii_alphanumeric());
    let digit_first = s.len() == 4 && s.as_bytes()[0].is_ascii_digit();
    alnum && ((5..=8).contains(&s.len()) || digit_first)
}

/// `tag` without its last subtag, a private-use part all at once; `None`
/// for a bare language. `tag` is canonical, so the result is too.
pub(super) fn parent_of(tag: &str) -> Option<&str> {
    match tag.find(PRIVATE_USE_PART) {
        Some(at) => Some(&tag[..at]),
        None => tag.rsplit_once('-').map(|(parent, _)| parent),
    }
}

/// `tag`, then each of its parents, down to its language.
pub(super) fn ancestry(tag: &str) -> impl Iterator<Item = &str> {
    core::iter::successors(Some(tag), |tag| parent_of(tag))
}

/// What a subtag can be next, in a tag's order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Slot {
    Script,
    Region,
    Variant,
}

impl LangCode {
    /// `text` as a tag, if it is one written as a tag is written (`pt-BR`,
    /// not `pt_br`); `None` otherwise. For text from outside, use
    /// [`parse`](Self::parse).
    pub fn new(text: &str) -> Option<Self> {
        Self::parse(text).ok().filter(|code| code.as_str() == text)
    }

    /// The tag `text` names, written as a tag is written: any case, `_` for
    /// `-`, and a POSIX locale's `.charset` and `@modifier` left out
    /// (`pt_BR.UTF-8` is `pt-BR`). For the system's languages and the
    /// command line. Fails with what is wrong.
    pub fn parse(text: &str) -> Result<Self, TagError> {
        let text = text.split(['.', '@']).next().unwrap_or_default();
        if text.eq_ignore_ascii_case("C") || text.eq_ignore_ascii_case("POSIX") {
            return Err(TagError::NoLanguage);
        }
        if text.is_empty() {
            return Err(TagError::Empty);
        }
        if let Some(c) = text
            .chars()
            .find(|&c| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
        {
            return Err(TagError::Character(c));
        }
        if text.len() > MAX_CODE_CHARS {
            return Err(TagError::TooLong);
        }
        let mut subtags = text.split(['-', '_']);
        if subtags.clone().any(str::is_empty) {
            return Err(TagError::EmptySubtag);
        }
        let language = subtags.next().unwrap_or_default().to_ascii_lowercase();
        if language == "i" {
            return Err(TagError::Grandfathered);
        }
        if !letters(&language, 2..=8) {
            return Err(TagError::Language(language));
        }
        let mut tag = language;
        let mut next = Slot::Script;
        let mut variants: Vec<String> = Vec::new();
        let mut first = true;
        while let Some(subtag) = subtags.next() {
            let lower = subtag.to_ascii_lowercase();
            tag.push('-');
            if first && letters(subtag, 3..=3) {
                return Err(TagError::Extlang(lower));
            }
            first = false;
            if lower == PRIVATE_USE {
                tag.push_str(PRIVATE_USE);
                return Self::private_use(tag, subtags);
            }
            if lower.len() == 1 {
                return Err(TagError::Extension(char::from(lower.as_bytes()[0])));
            }
            if next <= Slot::Script && is_script(subtag) {
                tag.push_str(&subtag[..1].to_ascii_uppercase());
                tag.push_str(&lower[1..]);
                next = Slot::Region;
            } else if next <= Slot::Region && is_region(subtag) {
                tag.push_str(&subtag.to_ascii_uppercase());
                next = Slot::Variant;
            } else if is_variant(subtag) {
                if variants.contains(&lower) {
                    return Err(TagError::RepeatedVariant(lower));
                }
                tag.push_str(&lower);
                variants.push(lower);
                next = Slot::Variant;
            } else {
                return Err(TagError::Misplaced(subtag.to_owned()));
            }
        }
        Ok(Self(tag))
    }

    /// `tag` (ending in `x`) with the private-use subtags `rest` after it.
    fn private_use<'a>(
        mut tag: String,
        rest: impl Iterator<Item = &'a str>,
    ) -> Result<Self, TagError> {
        let mut any = false;
        for subtag in rest {
            if subtag.len() > 8 {
                return Err(TagError::PrivateUse);
            }
            tag.push('-');
            tag.push_str(&subtag.to_ascii_lowercase());
            any = true;
        }
        if any {
            Ok(Self(tag))
        } else {
            Err(TagError::PrivateUse)
        }
    }

    /// English, the source language when a game declares none.
    #[deprecated(note = "a game's source language is `Lang::source_lang()`")]
    pub fn english() -> Self {
        Self(DEFAULT_SOURCE.to_owned())
    }

    /// The tag as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The language subtag: `pt` for `pt-BR`.
    pub fn language(&self) -> &str {
        self.0.split('-').next().unwrap_or_default()
    }

    /// The subtags after the language, up to a private-use part.
    fn subtags(&self) -> impl Iterator<Item = &str> {
        self.0.split('-').skip(1).take_while(|s| *s != PRIVATE_USE)
    }

    /// The script subtag, if the tag has one: `Hans` for `zh-Hans-CN`.
    pub fn script(&self) -> Option<&str> {
        self.subtags().next().filter(|s| is_script(s))
    }

    /// The region subtag, if the tag has one: `BR` for `pt-BR`.
    pub fn region(&self) -> Option<&str> {
        self.subtags().take(2).find(|s| is_region(s))
    }

    /// The tag without its last subtag (`pt` for `pt-BR`), a private-use
    /// part all at once (`en` for `en-x-pseudo`). `None` for a bare
    /// language.
    pub fn parent(&self) -> Option<LangCode> {
        parent_of(&self.0).map(|parent| Self(parent.to_owned()))
    }
}

impl Default for LangCode {
    /// `en`, English: the source language when a game declares none.
    fn default() -> Self {
        Self(DEFAULT_SOURCE.to_owned())
    }
}

impl fmt::Display for LangCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Borrow<str> for LangCode {
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(text: &str) -> LangCode {
        LangCode::new(text).unwrap_or_else(|| panic!("{text}"))
    }

    #[test]
    fn canonical_tags_are_taken_as_they_are() {
        for good in [
            "en",
            "ja",
            "dbg",
            "test",
            "abcdefgh",
            "pt-BR",
            "es-419",
            "zh-Hans",
            "zh-Hant-TW",
            "sr-Latn-RS",
            "de-CH-1996",
            "en-x-pseudo",
            "sl-rozaj-biske",
            "en-US-x-a-12345678",
        ] {
            assert_eq!(tag(good).as_str(), good);
            assert_eq!(tag(good).to_string(), good);
        }
    }

    #[test]
    fn other_spellings_and_other_tags_are_not() {
        for bad in [
            "",
            "e",
            "EN",
            "e1",
            "pt-br",
            "PT-BR",
            "pt_BR",
            "zh-hans",
            "zh-HANS",
            "en-us",
            "pt-BR.UTF-8",
            "abcdefghi",
            "日本",
            "en-u-ca-gregory",
            "en-t-ja",
            "i-klingon",
            "en--US",
            "-en",
            "en-",
            "zh-yue",
            "x-pseudo",
            "en-x",
            "en-US-Latn",
            "en-BR-US",
            "de-1996-1996",
            "en-x-abcdefghi",
            "en US",
        ] {
            assert_eq!(LangCode::new(bad), None, "{bad}");
        }
        let long = "en-x-aaaaaaaa-bbbbbbbb-cccccccc-ddd";
        assert_eq!(long.len(), MAX_CODE_CHARS);
        assert!(LangCode::new(long).is_some());
        assert_eq!(LangCode::parse(&format!("{long}d")), Err(TagError::TooLong));
    }

    #[test]
    fn text_from_outside_is_parsed_to_its_one_spelling() {
        for (text, canonical) in [
            ("pt_br.UTF-8", "pt-BR"),
            ("ZH-hans-cn", "zh-Hans-CN"),
            ("en_US@euro", "en-US"),
            ("ja_JP.eucJP@x", "ja-JP"),
            ("DE-ch-1996", "de-CH-1996"),
            ("EN-X-Pseudo", "en-x-pseudo"),
            ("es_419", "es-419"),
        ] {
            assert_eq!(LangCode::parse(text).unwrap().as_str(), canonical, "{text}");
        }
    }

    #[test]
    fn each_refusal_says_what_is_wrong() {
        let refused = |text: &str| LangCode::parse(text).unwrap_err();
        assert_eq!(refused("C"), TagError::NoLanguage);
        assert_eq!(refused("C.UTF-8"), TagError::NoLanguage);
        assert_eq!(refused("POSIX"), TagError::NoLanguage);
        assert_eq!(refused(""), TagError::Empty);
        assert_eq!(refused(".UTF-8"), TagError::Empty);
        assert_eq!(refused("en US"), TagError::Character(' '));
        assert_eq!(refused("en--US"), TagError::EmptySubtag);
        assert_eq!(refused("x1"), TagError::Language("x1".into()));
        assert_eq!(refused("x-pseudo"), TagError::Language("x".into()));
        assert_eq!(refused("i-klingon"), TagError::Grandfathered);
        assert_eq!(refused("zh-YUE"), TagError::Extlang("yue".into()));
        assert_eq!(refused("en-u-ca-gregory"), TagError::Extension('u'));
        assert_eq!(
            refused("de-1996-1996"),
            TagError::RepeatedVariant("1996".into())
        );
        assert_eq!(refused("en-x"), TagError::PrivateUse);
        assert_eq!(refused("en-x-abcdefghi"), TagError::PrivateUse);
        assert_eq!(refused("en-US-Latn"), TagError::Misplaced("Latn".into()));
        let words: Vec<String> = [
            TagError::Empty,
            TagError::TooLong,
            TagError::Character(' '),
            TagError::EmptySubtag,
            TagError::NoLanguage,
            TagError::Language("x1".into()),
            TagError::Grandfathered,
            TagError::Extlang("yue".into()),
            TagError::Extension('u'),
            TagError::RepeatedVariant("1996".into()),
            TagError::PrivateUse,
            TagError::Misplaced("Latn".into()),
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        assert_eq!(
            words,
            [
                "a tag can't be empty",
                "a tag is at most 35 characters",
                "' ' can't be in a tag: only letters, digits and -",
                "a tag has no empty part: no -- and no - at an end",
                "C and POSIX name no language",
                "a tag starts with its language, 2 to 8 letters, not \"x1\"",
                "an old tag starting with i- isn't taken",
                "\"yue\" after the language is an extended language, which isn't taken; write \
                 the language alone",
                "extensions (-u-) aren't taken; only a private-use part (-x-)",
                "the variant \"1996\" is written twice",
                "a private-use part is x and one or more parts of 1 to 8 letters or digits",
                "\"Latn\" can't be there: a tag is its language, then a script, a region, \
                 variants and a private-use part, each if it has one",
            ]
        );
    }

    #[test]
    fn a_tag_names_its_parts() {
        let parts = |text: &str| {
            let t = tag(text);
            (
                t.language().to_owned(),
                t.script().map(str::to_owned),
                t.region().map(str::to_owned),
            )
        };
        let own = |s: &str| Some(s.to_owned());
        assert_eq!(parts("en"), ("en".into(), None, None));
        assert_eq!(parts("pt-BR"), ("pt".into(), None, own("BR")));
        assert_eq!(parts("es-419"), ("es".into(), None, own("419")));
        assert_eq!(parts("zh-Hans"), ("zh".into(), own("Hans"), None));
        assert_eq!(parts("zh-Hant-TW"), ("zh".into(), own("Hant"), own("TW")));
        assert_eq!(parts("de-CH-1996"), ("de".into(), None, own("CH")));
        assert_eq!(parts("de-1996"), ("de".into(), None, None));
        assert_eq!(parts("sl-rozaj"), ("sl".into(), None, None));
        assert_eq!(parts("en-x-ab"), ("en".into(), None, None));
        assert_eq!(parts("en-x-abcd"), ("en".into(), None, None));
    }

    #[test]
    fn a_parent_drops_the_last_subtag_and_private_use_at_once() {
        let parent = |text: &str| tag(text).parent().map(|p| p.as_str().to_owned());
        let own = |s: &str| Some(s.to_owned());
        assert_eq!(parent("en"), None);
        assert_eq!(parent("pt-BR"), own("pt"));
        assert_eq!(parent("zh-Hans"), own("zh"));
        assert_eq!(parent("zh-Hant-TW"), own("zh-Hant"));
        assert_eq!(parent("de-CH-1996"), own("de-CH"));
        assert_eq!(parent("sl-rozaj-biske"), own("sl-rozaj"));
        assert_eq!(parent("en-x-pseudo"), own("en"));
        assert_eq!(parent("en-US-x-a-b"), own("en-US"));
        let all: Vec<&str> = ancestry("zh-Hant-TW-x-a").collect();
        assert_eq!(all, ["zh-Hant-TW-x-a", "zh-Hant-TW", "zh-Hant", "zh"]);
    }

    #[test]
    fn the_default_is_english() {
        assert_eq!(LangCode::default().as_str(), "en");
        let en = LangCode::default();
        let found: &str = en.borrow();
        assert_eq!(found, "en");
    }

    #[test]
    #[expect(deprecated)]
    fn english_is_the_default_source() {
        assert_eq!(LangCode::english(), LangCode::default());
    }

    /// A tag built from the subset's parts, as subtags.
    fn subtags() -> impl proptest::strategy::Strategy<Value = Vec<String>> {
        use proptest::prelude::*;
        (
            "[a-z]{2,8}",
            proptest::option::of("[A-Z][a-z]{3}"),
            proptest::option::of("[A-Z]{2}|[0-9]{3}"),
            proptest::collection::btree_set("[0-9][a-z0-9]{3}|[a-z0-9]{5,8}", 0..3),
            proptest::option::of(proptest::collection::vec("[a-z0-9]{1,8}", 1..3)),
        )
            .prop_map(|(language, script, region, variants, private)| {
                let mut subtags = vec![language];
                subtags.extend(script);
                subtags.extend(region);
                subtags.extend(variants);
                if let Some(private) = private {
                    subtags.push(PRIVATE_USE.to_owned());
                    subtags.extend(private);
                }
                subtags
            })
    }

    proptest::proptest! {
        /// Any tag of the subset is taken as it is, read back from upper
        /// case and from `_`, and its parents end within its subtags.
        #[test]
        fn a_tag_reads_back_and_its_parents_end(subtags in subtags()) {
            let text = subtags.join("-");
            proptest::prop_assume!(text.len() <= MAX_CODE_CHARS);
            let t = LangCode::new(&text);
            proptest::prop_assert_eq!(t.as_ref().map(LangCode::as_str), Some(text.as_str()));
            let upper = LangCode::parse(&text.to_ascii_uppercase()).unwrap();
            proptest::prop_assert_eq!(upper.as_str(), text.as_str());
            let posix = LangCode::parse(&subtags.join("_")).unwrap();
            proptest::prop_assert_eq!(posix.as_str(), text.as_str());
            let steps = core::iter::successors(t, LangCode::parent).count();
            proptest::prop_assert!(steps <= subtags.len(), "{} steps for {}", steps, text);
        }
    }
}
