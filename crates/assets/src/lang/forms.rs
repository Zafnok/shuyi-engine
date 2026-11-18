//! What each text of a language is, learned once when the languages are
//! built (ADR-0130): plain, a message with its parse kept, or not a
//! message. A fill reads it ([`Lang::form_in`]) and parses nothing.
//!
//! Only what isn't plain is kept: a language of plain texts keeps no more
//! than its strings.
use crate::prelude::*;

use alloc::collections::BTreeMap;

use super::{Entry, Lang, LangCode, LangPack};
use crate::message::{Form, Message, Parsed, has_message_syntax};

/// What a text that isn't plain is.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Kept {
    /// A message, its parse kept.
    Message(Parsed),
    /// Message syntax that doesn't parse.
    NotAMessage,
}

/// What a text is: `None` for plain text, which keeps nothing.
fn kept(text: &str) -> Option<Kept> {
    if !has_message_syntax(text) {
        return None;
    }
    Some(Message::parse(text).map_or(Kept::NotAMessage, |m| Kept::Message(m.keep())))
}

/// The texts of `texts` that aren't plain, by key.
fn kept_of<'t>(texts: impl Iterator<Item = (&'t String, &'t str)>) -> BTreeMap<String, Kept> {
    // The key is copied only for a text that isn't plain.
    texts
        .filter_map(|(key, text)| kept(text).map(|kept| (key.clone(), kept)))
        .collect()
}

/// The texts of a pack's `entries` that aren't plain, by key.
fn kept_entries(entries: &BTreeMap<String, Entry>) -> BTreeMap<String, Kept> {
    kept_of(entries.iter().map(|(key, e)| (key, e.text.as_str())))
}

/// `text`'s form, `kept` what was learned of it.
fn form<'a>(kept: Option<&'a Kept>, text: &'a str) -> Form<'a> {
    match kept {
        None => Form::Plain(text),
        Some(Kept::Message(parsed)) => Form::Message(parsed.with(text)),
        Some(Kept::NotAMessage) => Form::NotAMessage(text),
    }
}

/// What a pack's texts are: its screen text's and its data text's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct PackForms {
    ui: BTreeMap<String, Kept>,
    data: BTreeMap<String, Kept>,
}

/// What every screen and data text of the source and the packs is, by
/// key: what [`Lang`] learns as it is built.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Forms {
    ui: BTreeMap<String, Kept>,
    data: BTreeMap<String, Kept>,
    packs: BTreeMap<LangCode, PackForms>,
}

impl Forms {
    /// What the texts of `ui` (the source's screen text), `data` (its
    /// data text) and `packs` are.
    pub(super) fn of(
        ui: &BTreeMap<String, String>,
        data: &BTreeMap<String, String>,
        packs: &BTreeMap<LangCode, LangPack>,
    ) -> Self {
        let packs = packs
            .iter()
            .filter_map(|(code, pack)| {
                let forms = PackForms {
                    ui: kept_entries(&pack.ui),
                    data: kept_entries(&pack.data),
                };
                // A pack of plain texts keeps nothing, not even its tag.
                (forms != PackForms::default()).then(|| (code.clone(), forms))
            })
            .collect();
        Self {
            ui: kept_of(ui.iter().map(|(key, text)| (key, text.as_str()))),
            data: kept_of(data.iter().map(|(key, text)| (key, text.as_str()))),
            packs,
        }
    }
}

impl Lang {
    /// [`text_in`](Self::text_in), as the form learned when the languages
    /// were built: a fill from it parses nothing (ADR-0130). A key the
    /// source lacks comes back as itself, its form read from it.
    pub fn form_in<'a>(&'a self, code: &LangCode, key: &'a str) -> (Form<'a>, &'a str) {
        let source_code = self.written_in.code.as_str();
        let Some(source) = self.ui.get(key) else {
            return (Form::of(key), source_code);
        };
        let found = self.packs_along(code).find_map(|(code, pack)| {
            let entry = pack.ui.get(key).filter(|entry| entry.source == *source)?;
            let kept = self.forms.packs.get(code).and_then(|f| f.ui.get(key));
            Some((form(kept, &entry.text), code.as_str()))
        });
        found.unwrap_or_else(|| (form(self.forms.ui.get(key), source), source_code))
    }

    /// [`data_text_in`](Self::data_text_in), as the form learned when the
    /// languages were built. `english`, when no pack gives the text, has
    /// the form learned of the source's data text for `key` if it is that
    /// text; else, as text the caller holds, its form is read from it
    /// ([`Form::of`]). The key is only built when the chain has a pack or
    /// `english` isn't plain.
    pub fn data_form_in<'a>(
        &'a self,
        code: &LangCode,
        key: impl FnOnce() -> String,
        english: &'a str,
    ) -> (Form<'a>, &'a str) {
        let source_code = self.source_lang().code.as_str();
        let mut packs = self.packs_along(code).peekable();
        if packs.peek().is_none() && !has_message_syntax(english) {
            return (Form::Plain(english), source_code);
        }
        let key = key();
        let found = packs.find_map(|(code, pack)| {
            let entry = pack
                .data
                .get(&key)
                .filter(|entry| entry.source == english)?;
            let kept = self.forms.packs.get(code).and_then(|f| f.data.get(&key));
            Some((form(kept, &entry.text), code.as_str()))
        });
        found.unwrap_or_else(|| {
            let learned = self.source.data.get(&key).filter(|today| *today == english);
            let form = match learned {
                Some(_) => form(self.forms.data.get(&key), english),
                None => Form::of(english),
            };
            (form, source_code)
        })
    }
}

#[cfg(test)]
mod tests;
