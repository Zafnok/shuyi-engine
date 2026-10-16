//! `{token}` text: a game's text names what is filled in when it is shown
//! between braces (`Hello, {name}.`). [`split_tokens`] cuts such text into
//! its plain parts and its tokens; what a token stands for is the game's.

/// A piece of text: plain text, a `{token}`, or a `{` with no `}` after it
/// (with the rest of the text).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part<'a> {
    /// Text outside braces.
    Text(&'a str),
    /// The inside of a `{...}`.
    Token(&'a str),
    /// A `{` with no closing `}`, and everything after it.
    Unclosed(&'a str),
}

/// Splits `text` into plain text and `{tokens}`.
pub fn split_tokens(text: &str) -> impl Iterator<Item = Part<'_>> {
    let mut rest = text;
    core::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let part = match rest.split_once('{') {
            None => Part::Text(rest),
            Some(("", after)) => match after.split_once('}') {
                Some((token, after)) => {
                    rest = after;
                    return Some(Part::Token(token));
                }
                None => Part::Unclosed(rest),
            },
            Some((before, _)) => {
                rest = &rest[before.len()..];
                return Some(Part::Text(before));
            }
        };
        rest = "";
        Some(part)
    })
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::*;

    #[test]
    fn splits_tokens() {
        let parts: Vec<_> = split_tokens("a{b}c{d").collect();
        assert_eq!(
            parts,
            [
                Part::Text("a"),
                Part::Token("b"),
                Part::Text("c"),
                Part::Unclosed("{d")
            ]
        );
        assert_eq!(split_tokens("").count(), 0);
        assert_eq!(split_tokens("{}").collect::<Vec<_>>(), [Part::Token("")]);
    }

    #[test]
    fn text_after_the_last_token_is_kept() {
        let parts: Vec<_> = split_tokens("{a}{b} end").collect();
        assert_eq!(
            parts,
            [Part::Token("a"), Part::Token("b"), Part::Text(" end")]
        );
        assert_eq!(
            split_tokens("plain").collect::<Vec<_>>(),
            [Part::Text("plain")]
        );
        assert_eq!(
            split_tokens("{open").collect::<Vec<_>>(),
            [Part::Unclosed("{open")]
        );
    }
}
