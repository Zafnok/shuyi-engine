//! The game's language that fits the system's languages ([`best_match`]).
use crate::prelude::*;

use super::tag::{self, LangCode};

/// The language subtag of Chinese.
const CHINESE: &str = "zh";

/// The script a Chinese tag without one takes from its region: `Hans` for
/// `CN` and `SG`, `Hant` for `TW`, `HK` and `MO`. `None` for another
/// language, a tag with a script, or another region.
fn chinese_script(code: &LangCode) -> Option<&'static str> {
    if code.language() != CHINESE || code.script().is_some() {
        return None;
    }
    match code.region()? {
        "CN" | "SG" => Some("Hans"),
        "TW" | "HK" | "MO" => Some("Hant"),
        _ => None,
    }
}

/// The first of `tag`, then each of its parents, that is offered.
fn offered_along<'a>(tag: &str, offered: &'a [LangCode]) -> Option<&'a LangCode> {
    tag::ancestry(tag).find_map(|step| offered.iter().find(|code| code.as_str() == step))
}

/// The offered tag `wanted` matches, by the three steps of [`best_match`].
fn match_one(wanted: &LangCode, offered: &[LangCode]) -> Option<LangCode> {
    if let Some(found) = offered_along(wanted.as_str(), offered) {
        return Some(found.clone());
    }
    let language = wanted.language();
    let region_script = chinese_script(wanted);
    if let Some(script) = region_script {
        let rest = &wanted.as_str()[language.len()..];
        if let Some(found) = offered_along(&format!("{language}-{script}{rest}"), offered) {
            return Some(found.clone());
        }
    }
    let mut same = offered.iter().filter(|code| code.language() == language);
    let only = same.next()?;
    if same.next().is_some() {
        return None;
    }
    let script = wanted.script().or(region_script);
    match (script, only.script().or_else(|| chinese_script(only))) {
        (Some(want), Some(have)) if want != have => None,
        _ => Some(only.clone()),
    }
}

/// The offered language that fits the system's: `preferred` is the
/// system's languages, most preferred first, and `offered` the source's tag
/// and the packs'. For each preferred tag in order:
///
/// 1. the tag, then each of its [`parent`](LangCode::parent)s, if offered
///    (`en-GB` matches an `en` source);
/// 2. Chinese without a script takes one from its region (`CN` and `SG`
///    give `Hans`; `TW`, `HK` and `MO` give `Hant`), then step 1 again with
///    the script written after the language (`zh-CN` matches `zh-Hans`,
///    through `zh-Hans-CN`);
/// 3. the only offered tag with the same language subtag, unless both have
///    a script (written, or from step 2) and they differ (`pt-PT` matches a
///    lone `pt-BR`; `zh-TW` doesn't match `zh-Hans`).
///
/// The first preferred tag that matches wins. `None` if none does: the
/// caller then uses the source. No CLDR data: no likely subtags beyond
/// step 2's five regions, no aliases, and no parent but the shorter tag.
pub fn best_match(preferred: &[LangCode], offered: &[LangCode]) -> Option<LangCode> {
    preferred
        .iter()
        .find_map(|wanted| match_one(wanted, offered))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(texts: &[&str]) -> Vec<LangCode> {
        texts
            .iter()
            .map(|text| LangCode::new(text).unwrap_or_else(|| panic!("{text}")))
            .collect()
    }

    fn best(preferred: &[&str], offered: &[&str]) -> Option<String> {
        best_match(&tags(preferred), &tags(offered)).map(|code| code.as_str().to_owned())
    }

    #[test]
    fn a_tag_matches_itself_or_a_parent() {
        assert_eq!(best(&["pt-BR"], &["en", "pt"]), Some("pt".into()));
        assert_eq!(best(&["en-GB"], &["en"]), Some("en".into()));
        assert_eq!(best(&["pt-BR"], &["pt", "pt-BR"]), Some("pt-BR".into()));
        assert_eq!(best(&["ja"], &["en", "ja"]), Some("ja".into()));
    }

    #[test]
    fn chinese_takes_its_script_from_its_region() {
        assert_eq!(best(&["zh-CN"], &["en", "zh-Hans"]), Some("zh-Hans".into()));
        assert_eq!(best(&["zh-TW"], &["en", "zh-Hans"]), None);
        assert_eq!(
            best(&["zh-CN"], &["zh-Hant", "zh-Hans-CN"]),
            Some("zh-Hans-CN".into())
        );
        for (region, script) in [
            ("CN", "Hans"),
            ("SG", "Hans"),
            ("TW", "Hant"),
            ("HK", "Hant"),
            ("MO", "Hant"),
        ] {
            let offered = ["en", "zh-Hans", "zh-Hant"];
            let wanted = format!("zh-{region}");
            assert_eq!(
                best(&[&wanted], &offered),
                Some(format!("zh-{script}")),
                "{wanted}"
            );
        }
        assert_eq!(best(&["zh-MY"], &["zh-Hans", "zh-Hant"]), None);
    }

    #[test]
    fn only_chinese_without_a_script_takes_one_from_its_region() {
        let script = |text: &str| chinese_script(&tags(&[text])[0]);
        assert_eq!(script("zh-SG"), Some("Hans"));
        assert_eq!(script("ms-SG"), None);
        assert_eq!(script("zh-Hant-CN"), None);
        assert_eq!(best(&["ms-SG"], &["en", "ms-Arab"]), Some("ms-Arab".into()));
    }

    #[test]
    fn a_lone_tag_of_the_language_matches_unless_the_scripts_differ() {
        assert_eq!(best(&["pt-PT"], &["en", "pt-BR"]), Some("pt-BR".into()));
        assert_eq!(best(&["pt-PT"], &["en", "pt-BR", "pt-AO"]), None);
        assert_eq!(best(&["sr-Latn"], &["en", "sr-Cyrl"]), None);
        assert_eq!(best(&["sr-Latn"], &["en", "sr-RS"]), Some("sr-RS".into()));
        assert_eq!(best(&["zh-HK"], &["en", "zh-CN"]), None);
        assert_eq!(best(&["zh-HK"], &["en", "zh-TW"]), Some("zh-TW".into()));
        assert_eq!(best(&["zh"], &["en", "zh-Hant"]), Some("zh-Hant".into()));
    }

    #[test]
    fn the_first_preferred_tag_that_matches_wins() {
        assert_eq!(best(&["de", "fr-CA"], &["en", "fr"]), Some("fr".into()));
        assert_eq!(best(&["fr-CA", "en"], &["en", "fr"]), Some("fr".into()));
        assert_eq!(best(&["de", "it"], &["en", "fr"]), None);
        assert_eq!(best(&[], &["en", "fr"]), None);
        assert_eq!(best(&["en"], &[]), None);
    }
}
