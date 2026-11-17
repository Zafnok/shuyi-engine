//! The checks of a language file's texts as messages (ADR-0127): each
//! parses, a select on `device` names devices, and a pack's entry keeps
//! its source's arguments and select branches and uses only the plural
//! categories its language has.
use crate::prelude::*;

use alloc::collections::BTreeSet;

use super::LangCode;
use crate::message::plural::{Category, Kind, categories, expected};
use crate::message::{ArgKind, DEVICE, DEVICES, Message};

/// `text` as a message, or what is wrong with it: it doesn't parse
/// (`what` names it in the problem), or a select on `device` has a value
/// no device has.
fn message<'a>(text: &'a str, what: &str) -> Result<Message<'a>, Vec<String>> {
    let message = Message::parse(text).map_err(|e| vec![format!("{what} isn't a message: {e}")])?;
    let mut problems = Vec::new();
    for arg in message.args() {
        let ArgKind::Select { values } = arg.kind else {
            continue;
        };
        if arg.name != DEVICE {
            continue;
        }
        for value in values {
            if value != "other" && !DEVICES.contains(&value) {
                problems.push(format!(
                    "a select on {{{DEVICE}}} has {value:?}, which no device is: the devices are \
                     {}",
                    DEVICES.join(", ")
                ));
            }
        }
    }
    if problems.is_empty() {
        Ok(message)
    } else {
        Err(problems)
    }
}

/// What is wrong with a text of the source's screen text: it isn't a
/// message, or it selects by a device no device is.
pub(super) fn source_problems(text: &str) -> Vec<String> {
    message(text, "the text").err().unwrap_or_default()
}

/// What is wrong with a pack's entry, its `text` translating `source`, in
/// the language `code` (`None` when it isn't known: no plural categories
/// are checked).
///
/// Both must be messages. They use the same arguments, so a translation
/// may wrap a `{n}` in a plural or drop a plural its language doesn't
/// need. Each value of a select of the source on an argument has a branch
/// in the text's selects on that argument, so a translation keeps every
/// device's wording. A plural or selectordinal uses only categories its
/// language has.
pub(super) fn entry_problems(text: &str, source: &str, code: Option<&LangCode>) -> Vec<String> {
    let (text, source) = match (message(text, "the text"), message(source, "its source")) {
        (Ok(text), Ok(source)) => (text, source),
        (text, source) => {
            let mut problems = text.err().unwrap_or_default();
            problems.extend(source.err().unwrap_or_default());
            return problems;
        }
    };
    let mut problems = Vec::new();
    let names: BTreeSet<&str> = text.names().into_iter().collect();
    if names != source.names().into_iter().collect() {
        problems.push("the text and its source must have the same placeholders".to_owned());
    }
    let selects = |m: &Message<'_>, name: &str| -> BTreeSet<String> {
        let mut values = BTreeSet::new();
        for arg in m.args().into_iter().filter(|a| a.name == name) {
            if let ArgKind::Select { values: v } = arg.kind {
                values.extend(v.into_iter().map(str::to_owned));
            }
        }
        values
    };
    for name in source.names() {
        let kept = selects(&text, name);
        for value in selects(&source, name).difference(&kept) {
            problems.push(format!(
                "its source's select on {{{name}}} has a branch {value:?}, which the text drops"
            ));
        }
    }
    if let Some(code) = code {
        for arg in text.args() {
            let ArgKind::Plural {
                kind,
                categories: used,
            } = arg.kind
            else {
                continue;
            };
            let has = categories(code.as_str(), kind);
            for category in used.into_iter().filter(|c| !has.contains(c)) {
                let kind = match kind {
                    Kind::Ordinal => "ordinal",
                    _ => "plural",
                };
                let names: Vec<&str> = has.iter().map(|c| c.name()).collect();
                problems.push(format!(
                    "{code} has no {kind} category {:?}: its categories are {}",
                    category.name(),
                    names.join(", ")
                ));
            }
        }
    }
    problems
}

/// The categories that a plural or selectordinal of `text`, in the
/// language `code`, has no branch for, each once, in CLDR's order
/// ([`expected`]: not the integer `many` of a million). None for a text
/// that isn't a message.
pub(super) fn lacks(text: &str, code: &LangCode) -> Vec<Category> {
    let Ok(message) = Message::parse(text) else {
        return Vec::new();
    };
    let mut lacks = BTreeSet::new();
    for arg in message.args() {
        if let ArgKind::Plural {
            kind,
            categories: used,
        } = arg.kind
        {
            let wanted = expected(code.as_str(), kind);
            lacks.extend(wanted.iter().filter(|c| !used.contains(c)));
        }
    }
    lacks.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ru() -> LangCode {
        LangCode::new("ru").unwrap()
    }

    #[test]
    fn source_text_must_parse_and_name_known_devices() {
        assert_eq!(
            source_problems("{n, plural, one {#} other {#}} {Confirm}"),
            [] as [String; 0]
        );
        assert_eq!(
            source_problems("at most {n profiles"),
            [
                "the text isn't a message: at character 11: expected `}` or `,` after the argument's name"
            ]
        );
        assert_eq!(
            source_problems("{device, select, playdat {a} other {b}}"),
            [
                "a select on {device} has \"playdat\", which no device is: the devices are \
              keyboard, xbox, playstation, playstation4, nintendo, playdate, generic"
            ]
        );
        // A select on another argument may have any value.
        assert_eq!(
            source_problems("{who, select, playdat {a} other {b}}"),
            [] as [String; 0]
        );
    }

    #[test]
    fn a_translation_may_wrap_or_drop_a_plural_but_keeps_its_arguments() {
        let source = "{n} keys";
        let wrapped = "{n, plural, one {# ключ} few {# ключа} many {# ключей} other {# ключа}}";
        assert_eq!(
            entry_problems(wrapped, source, Some(&ru())),
            [] as [String; 0]
        );
        assert_eq!(entry_problems("鍵{n}個", wrapped, None), [] as [String; 0]);
        assert_eq!(
            entry_problems("ключи", source, Some(&ru())),
            ["the text and its source must have the same placeholders"]
        );
    }

    #[test]
    fn a_translation_keeps_every_branch_of_its_sources_selects() {
        let source = "{device, select, playdate {Crank} other {Turn}}";
        assert_eq!(
            entry_problems("{device, select, other {Tourne}}", source, None),
            ["its source's select on {device} has a branch \"playdate\", which the text drops"]
        );
        let more = "{device, select, playdate {M} xbox {X} other {T}}";
        assert_eq!(entry_problems(more, source, None), [] as [String; 0]);
        // A misspelled device is that problem alone.
        let typo = entry_problems("{device, select, playdat {M} other {T}}", source, None);
        assert_eq!(typo.len(), 1);
        assert!(
            typo[0].starts_with("a select on {device} has \"playdat\""),
            "{typo:?}"
        );
    }

    #[test]
    fn a_plural_uses_only_the_categories_of_its_language() {
        let source = "{n, plural, one {# key} other {# keys}}";
        let fr = LangCode::new("fr").unwrap();
        let few = "{n, plural, one {# clé} few {# clés} other {# clés}}";
        assert_eq!(
            entry_problems(few, source, Some(&fr)),
            ["fr has no plural category \"few\": its categories are one, many, other"]
        );
        let rank = "{n, selectordinal, one {#er} two {#e} other {#e}}";
        assert_eq!(
            entry_problems(rank, "{n, selectordinal, other {#}}", Some(&fr)),
            ["fr has no ordinal category \"two\": its categories are one, other"]
        );
        // Without the pack's language nothing is checked; `=N` is always fine.
        assert_eq!(entry_problems(few, source, None), [] as [String; 0]);
        let exact = "{n, plural, =0 {aucune} one {# clé} other {# clés}}";
        assert_eq!(entry_problems(exact, source, Some(&fr)), [] as [String; 0]);
    }

    #[test]
    fn a_plural_lacks_the_categories_its_language_has_but_a_millions_many() {
        use Category::{Few, Many, One};
        let some = "{n, plural, one {# ключ} other {# ключей}}";
        assert_eq!(lacks(some, &ru()), [Few, Many]);
        // Across its plurals, each category once, in CLDR's order.
        let two = "{n, plural, one {#} many {#} other {#}} {m, plural, few {#} other {#}}";
        assert_eq!(lacks(two, &ru()), [One, Few, Many]);
        let fr = LangCode::new("fr").unwrap();
        assert_eq!(lacks("{n, plural, one {#} other {#}}", &fr), []);
        assert_eq!(lacks("{n, plural, other {#}}", &fr), [One]);
        // A selectordinal counts too; no plural, or no message, lacks nothing.
        let en = LangCode::new("en").unwrap();
        let rank = "{n, selectordinal, one {#st} other {#th}}";
        assert_eq!(lacks(rank, &en), [Category::Two, Few]);
        assert_eq!(lacks("{n} keys", &ru()), []);
        assert_eq!(lacks("{n", &ru()), []);
    }

    #[test]
    fn a_text_or_source_that_does_not_parse_is_named() {
        assert_eq!(
            entry_problems("{n", "{n}", None),
            [
                "the text isn't a message: at character 2: expected `}` or `,` after the argument's name"
            ]
        );
        assert_eq!(
            entry_problems("{n}", "{}", None),
            ["its source isn't a message: at character 1: expected an argument's name"]
        );
    }
}
