//! Word wrap for text boxes.

use crate::prelude::*;

/// Splits `text` into lines of at most `width` characters, breaking at
/// spaces. A word is never split unless it alone is longer than `width`;
/// then it is cut into `width`-long pieces. Runs of spaces count as one
/// break. Empty (or all-space) text gives no lines. A `width` of 0 is
/// treated as 1.
pub fn word_wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut line_len = 0;
    for word in text.split_whitespace() {
        let chars: Vec<char> = word.chars().collect();
        if line_len > 0 && line_len + 1 + chars.len() <= width {
            line.push(' ');
            line.extend(&chars);
            line_len += 1 + chars.len();
            continue;
        }
        if line_len > 0 {
            lines.push(core::mem::take(&mut line));
        }
        let mut pieces = chars.chunks(width).peekable();
        while let Some(piece) = pieces.next() {
            line = piece.iter().collect();
            line_len = piece.len();
            if pieces.peek().is_some() {
                lines.push(core::mem::take(&mut line));
            }
        }
    }
    if line_len > 0 {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn breaks_at_spaces() {
        assert_eq!(
            word_wrap("the quick brown fox", 10),
            ["the quick", "brown fox"]
        );
        assert_eq!(word_wrap("abc def", 7), ["abc def"]);
        assert_eq!(word_wrap("abc def", 6), ["abc", "def"]);
    }

    #[test]
    fn hard_splits_only_over_long_words() {
        assert_eq!(
            word_wrap("ab abcdefgh c", 3),
            ["ab", "abc", "def", "gh", "c"]
        );
        assert_eq!(word_wrap("abcdef", 3), ["abc", "def"]);
    }

    #[test]
    fn empty_and_extra_spaces() {
        assert!(word_wrap("", 5).is_empty());
        assert!(word_wrap("   ", 5).is_empty());
        assert_eq!(word_wrap("  a   b  ", 5), ["a b"]);
        assert_eq!(word_wrap("ab", 0), ["a", "b"]);
    }

    #[test]
    fn counts_characters_not_bytes() {
        assert_eq!(word_wrap("ééé ééé", 3), ["ééé", "ééé"]);
    }

    fn words(max_len: usize) -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(
            prop::string::string_regex(&format!("[a-zA-Z.,!?']{{1,{max_len}}}")).unwrap(),
            0..40,
        )
    }

    proptest! {
        #[test]
        fn no_line_is_too_wide(ws in words(30), width in 1usize..40) {
            for line in word_wrap(&ws.join(" "), width) {
                prop_assert!(line.chars().count() <= width, "{line:?} > {width}");
                prop_assert!(!line.is_empty());
            }
        }

        #[test]
        fn keeps_every_word_when_words_fit(
            ws in words(12),
            width in 12usize..40,
            gaps in prop::collection::vec(1usize..4, 40),
        ) {
            let mut text = String::new();
            for (w, &g) in ws.iter().zip(&gaps) {
                text.push_str(w);
                text.push_str(&" ".repeat(g));
            }
            let lines = word_wrap(&text, width);
            let rejoined: Vec<&str> = lines.iter().flat_map(|l| l.split(' ')).collect();
            prop_assert_eq!(rejoined, ws.iter().map(String::as_str).collect::<Vec<_>>());
        }

        #[test]
        fn keeps_every_character(ws in words(30), width in 1usize..40) {
            let text = ws.join(" ");
            let kept: String = word_wrap(&text, width).concat().replace(' ', "");
            prop_assert_eq!(kept, text.replace(' ', ""));
        }

        #[test]
        fn lines_are_filled_greedily(ws in words(12), width in 12usize..40) {
            let lines = word_wrap(&ws.join(" "), width);
            for pair in lines.windows(2) {
                let next_word = pair[1].split(' ').next().unwrap_or("");
                prop_assert!(
                    pair[0].chars().count() + 1 + next_word.chars().count() > width,
                    "{:?} could have taken {next_word:?}", pair[0]
                );
            }
        }
    }
}
