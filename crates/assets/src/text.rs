//! Word wrap for text boxes, the one the checks and the boxes share.
//!
//! Latin text breaks at spaces. Text without spaces (Japanese) breaks
//! between any two of its characters, except where the usual *kinsoku*
//! rules forbid it: never before closing punctuation or a small kana,
//! never after an opening bracket. A run of Latin letters or digits inside
//! it is kept whole, and so is text joined by a no-break space
//! ([`NO_BREAK_SPACE`]). Chinese keeps the same rules with its own
//! punctuation (`，` `》` `《`), and where its quotes, ellipsis and dash are
//! wide ([`WIDE_PUNCTUATION`](crate::font::WIDE_PUNCTUATION)) a line never
//! ends with `“` or starts with `”`, and `……` and `——` are never split.

use crate::font::CellWidths;
use crate::lang::text_width;
use crate::prelude::*;

/// A space [`word_wrap`] never breaks at (U+00A0), drawn as a space: what
/// joins the words of a key's name filled into a text, so the name is
/// never split across lines.
pub const NO_BREAK_SPACE: char = '\u{a0}';

/// Characters a line never starts with: closing punctuation, the
/// prolonged sound mark and the small kana.
const NO_START: &str = "、。，．・：；？！）》〉」』】〕ーぁぃぅぇぉっゃゅょァィゥェォッャュョ\
                        ,.:;?!)]}";

/// Characters a line never ends with: opening brackets.
const NO_END: &str = "（《〈「『【〔([{";

/// Closing quotes, the ellipsis and the dash, which a line never starts
/// with where they are wide (Chinese): so `……` and `——` are never split.
/// Narrow, they are English's, and break as they always have.
const NO_START_WIDE: &str = "\u{201D}\u{2019}\u{2026}\u{2014}";

/// Opening quotes, which a line never ends with where they are wide.
const NO_END_WIDE: &str = "\u{201C}\u{2018}";

/// Whether a line may not start with `c`.
fn no_start(c: char, widths: &CellWidths) -> bool {
    NO_START.contains(c) || (NO_START_WIDE.contains(c) && widths.is_wide(c))
}

/// Whether a line may not end with `c`.
fn no_end(c: char, widths: &CellWidths) -> bool {
    NO_END.contains(c) || (NO_END_WIDE.contains(c) && widths.is_wide(c))
}

/// Whether text may break at `c`, dropping it: whitespace, but not a
/// no-break space.
fn breaks_at(c: char) -> bool {
    c.is_whitespace() && c != NO_BREAK_SPACE
}

/// Whether `c` is written without spaces between words, so a line may
/// break on either side of it: a character `widths` has wide, or one of
/// the Japanese and Chinese scripts and their punctuation (so text breaks
/// the same before an atlas has its wide glyphs).
fn breaks_around(c: char, widths: &CellWidths) -> bool {
    widths.is_wide(c)
        || matches!(c,
            '\u{3000}'..='\u{30ff}'   // CJK punctuation, hiragana, katakana
            | '\u{3400}'..='\u{4dbf}' // CJK extension A
            | '\u{4e00}'..='\u{9fff}' // CJK unified ideographs
            | '\u{f900}'..='\u{faff}' // CJK compatibility ideographs
            | '\u{ff00}'..='\u{ffef}' // fullwidth and halfwidth forms
        )
}

/// Whether a line may break between `a` and `b`, two characters of a word
/// (text between spaces).
fn may_break(a: char, b: char, widths: &CellWidths) -> bool {
    (breaks_around(a, widths) || breaks_around(b, widths))
        && a != NO_BREAK_SPACE
        && b != NO_BREAK_SPACE
        && !no_start(b, widths)
        && !no_end(a, widths)
}

/// The pieces of `word` (text without a space to break at) that a line
/// may break between.
fn pieces<'a>(word: &'a str, widths: &CellWidths) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = word.char_indices().peekable();
    while let Some((_, a)) = chars.next() {
        if let Some(&(at, b)) = chars.peek()
            && may_break(a, b, widths)
        {
            out.push(&word[start..at]);
            start = at;
        }
    }
    out.push(&word[start..]);
    out
}

/// The lines being built: those done, the last one, and how many cells
/// it takes.
struct Lines<'w> {
    width: usize,
    widths: &'w CellWidths,
    done: Vec<String>,
    line: String,
    cells: usize,
}

impl Lines<'_> {
    /// Ends the line, if it holds anything.
    fn end_line(&mut self) {
        if self.cells > 0 {
            self.done.push(core::mem::take(&mut self.line));
            self.cells = 0;
        }
    }

    /// Adds `piece`, after a space if `spaced` and the line holds
    /// something: on this line if it fits, else on a new one, cut into
    /// lines of `width` cells if it is wider than a line.
    fn add(&mut self, piece: &str, spaced: bool) {
        let cells = text_width(piece, self.widths);
        let gap = usize::from(spaced && self.cells > 0);
        if self.cells + gap + cells <= self.width {
            if gap > 0 {
                self.line.push(' ');
            }
            self.line.push_str(piece);
            self.cells += gap + cells;
            return;
        }
        self.end_line();
        for c in piece.chars() {
            let c_cells = self.widths.of_char(c);
            if self.cells + c_cells > self.width {
                self.end_line();
            }
            self.line.push(c);
            self.cells += c_cells;
        }
    }
}

/// Splits `text` into lines of at most `width` cells (a character
/// `widths` has wide takes two, [`text_width`]).
///
/// Latin text breaks at spaces; a word is never split unless it alone is
/// wider than `width`, and then it is cut into `width`-wide pieces. Runs
/// of spaces count as one break. Text without spaces (Japanese) breaks
/// between any two characters with a wide or Japanese one among them,
/// except before closing punctuation or a small kana (`。`, `、`, `」`,
/// `っ`, `，`, `》`) and after an opening bracket (`「`, `《`); where `widths`
/// has them wide, also before `” ’ … —` (so `……` and `——` stay whole)
/// and after `“ ‘`. A run of Latin letters or
/// digits inside it is kept whole, as is anything joined by a
/// [`NO_BREAK_SPACE`]. Where those rules leave no break that fits, the
/// text is cut anyway: a line never runs past `width`, unless `width` is
/// 1 and the line is one wide character.
///
/// Empty (or all-space) text gives no lines. A `width` of 0 is treated as
/// 1.
pub fn word_wrap(text: &str, width: usize, widths: &CellWidths) -> Vec<String> {
    let mut lines = Lines {
        width: width.max(1),
        widths,
        done: Vec::new(),
        line: String::new(),
        cells: 0,
    };
    for word in text.split(breaks_at).filter(|w| !w.is_empty()) {
        for (i, piece) in pieces(word, widths).into_iter().enumerate() {
            lines.add(piece, i == 0);
        }
    }
    lines.end_line();
    lines.done
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn wrap(text: &str, width: usize) -> Vec<String> {
        word_wrap(text, width, &CellWidths::NARROW)
    }

    /// Widths where every Japanese character of these tests is wide.
    fn japanese() -> CellWidths {
        CellWidths::wide(
            "今日はいい天気ですね。散歩に行きましょうか、それとも家で本を読みますか？\
             彼女は「明日また来ます」と言った。ちょっと待ってくださいコーヒーを一杯\
             私の名前ですゲームが好きで毎日遊んでいる遊ぶ（はい）"
                .chars(),
        )
    }

    #[test]
    fn breaks_at_spaces() {
        assert_eq!(wrap("the quick brown fox", 10), ["the quick", "brown fox"]);
        assert_eq!(wrap("abc def", 7), ["abc def"]);
        assert_eq!(wrap("abc def", 6), ["abc", "def"]);
    }

    #[test]
    fn hard_splits_only_over_long_words() {
        assert_eq!(wrap("ab abcdefgh c", 3), ["ab", "abc", "def", "gh", "c"]);
        assert_eq!(wrap("abcdef", 3), ["abc", "def"]);
    }

    #[test]
    fn empty_and_extra_spaces() {
        let found = wrap("", 5);
        assert!(found.is_empty(), "{found:?}");
        let found = wrap(" ", 5);
        assert!(found.is_empty(), "{found:?}");
        assert_eq!(wrap("  a   b  ", 5), ["a b"]);
        assert_eq!(wrap("ab", 0), ["a", "b"]);
    }

    #[test]
    fn counts_characters_not_bytes() {
        assert_eq!(wrap("ééé ééé", 3), ["ééé", "ééé"]);
    }

    #[test]
    fn a_wide_character_takes_two_cells() {
        let widths = japanese();
        // Six characters, twelve cells: three to a line of six cells.
        assert_eq!(word_wrap("今日はいい天", 6, &widths), ["今日は", "いい天"]);
        // Measured narrow, the same text is six cells and one line.
        assert_eq!(
            word_wrap("今日はいい天", 6, &CellWidths::NARROW),
            ["今日はいい天"]
        );
        // A wide character never straddles the end of a line.
        assert_eq!(word_wrap("今日は", 5, &widths), ["今日", "は"]);
    }

    #[test]
    fn japanese_breaks_between_characters_without_an_atlas_and_a_wide_one_anywhere() {
        // Measured narrow, Japanese still breaks between its characters:
        // `今日` joins `a` rather than the run being cut.
        assert_eq!(
            word_wrap("a 今日はいい", 4, &CellWidths::NARROW),
            ["a 今日", "はいい"]
        );
        // A wide character of no Japanese script breaks around it too.
        let hearts = CellWidths::wide(['♥']);
        assert_eq!(word_wrap("a ♥♥", 4, &hearts), ["a ♥", "♥"]);
    }

    #[test]
    fn a_japanese_paragraph_breaks_anywhere_within_the_width() {
        let widths = japanese();
        let text = "今日はいい天気ですね。散歩に行きましょうか、それとも家で本を読みますか？";
        let lines = word_wrap(text, 16, &widths);
        assert_eq!(
            lines,
            [
                "今日はいい天気で",
                "すね。散歩に行き",
                "ましょうか、それ",
                "とも家で本を読み",
                "ますか？",
            ]
        );
        assert_eq!(lines.concat(), text);
    }

    #[test]
    fn no_line_starts_with_closing_punctuation_or_a_small_kana() {
        let widths = japanese();
        // `。` would start the second line at width 10: `ね` goes with it.
        assert_eq!(
            word_wrap("天気ですね。散歩", 10, &widths),
            ["天気です", "ね。散歩"]
        );
        // `、` likewise.
        assert_eq!(
            word_wrap("行きましょうか、それ", 14, &widths),
            ["行きましょう", "か、それ"]
        );
        // A small `ょ` never starts a line: at width 8 `行きまし` would
        // leave it to start the next, so `し` goes down with it.
        assert_eq!(word_wrap("行きましょう", 8, &widths), ["行きま", "しょう"]);
        // Nor the prolonged sound mark, nor a closing bracket.
        assert_eq!(word_wrap("コーヒー", 6, &widths), ["コー", "ヒー"]);
        assert_eq!(word_wrap("（はい）", 6, &widths), ["（は", "い）"]);
    }

    #[test]
    fn no_line_ends_with_an_opening_bracket() {
        let widths = japanese();
        let text = "彼女は「明日また来ます」と言った。";
        // At width 8 the first line could be `彼女は「`: the bracket goes
        // down with the word it opens.
        let lines = word_wrap(text, 8, &widths);
        assert_eq!(
            lines,
            ["彼女は", "「明日ま", "た来ま", "す」と", "言った。"]
        );
        for line in &lines {
            assert!(!line.ends_with('「'), "{lines:?}");
            assert!(!line.starts_with(['。', '、', '」']), "{lines:?}");
        }
    }

    #[test]
    fn breaks_anyway_where_the_rules_leave_no_break_that_fits() {
        // Nothing may break in `「「「「`: it is cut at the width.
        let brackets = CellWidths::wide(['「']);
        assert_eq!(word_wrap("「「「「", 4, &brackets), ["「「", "「「"]);
        // `ね。」` can't break, and is wider than a line of 4.
        let widths = CellWidths::wide("ね。」".chars());
        assert_eq!(word_wrap("ね。」", 4, &widths), ["ね。", "」"]);
    }

    #[test]
    fn a_latin_run_inside_japanese_is_kept_whole() {
        let widths = japanese();
        let text = "私の名前はMeiです";
        // `Mei` doesn't fit after `私の名前は` (10 cells) at width 12: it
        // goes down whole.
        assert_eq!(word_wrap(text, 12, &widths), ["私の名前は", "Meiです"]);
        // Digits too.
        assert_eq!(
            word_wrap("毎日遊ぶ2026回", 10, &widths),
            ["毎日遊ぶ", "2026回"]
        );
        // A run wider than the width is cut, as a long Latin word is.
        assert_eq!(word_wrap("私abcdefg", 4, &widths), ["私", "abcd", "efg"]);
    }

    #[test]
    fn latin_and_japanese_with_spaces_break_at_the_spaces_and_between_wide_characters() {
        let widths = japanese();
        assert_eq!(
            word_wrap("Press Z ゲームが好きで", 12, &widths),
            ["Press Z ゲー", "ムが好きで"]
        );
    }

    #[test]
    fn never_breaks_at_a_no_break_space() {
        let nbsp = NO_BREAK_SPACE;
        let text = format!("press Left{nbsp}Shift to run");
        assert_eq!(
            wrap(&text, 12),
            ["press", format!("Left{nbsp}Shift").as_str(), "to run"]
        );
        // Nor beside one, between Japanese characters.
        let widths = japanese();
        let text = format!("毎日Left{nbsp}Shiftを押す");
        assert_eq!(
            word_wrap(&text, 12, &widths),
            ["毎日", format!("Left{nbsp}Shiftを").as_str(), "押す"]
        );
    }

    /// Chinese sentences with speech in `“”`, a doubled `……` and `——`.
    const CHINESE: &str = "他说：“今天天气很好，我们去公园散步吧。”她回答：“好啊……不过我有点累\
                           ——我们明天去，好吗？”《西游记》是一本书（很有名）。";

    /// Widths where every character of [`CHINESE`] but the punctuation is
    /// wide, and the punctuation as a Chinese pack has it.
    fn chinese() -> CellWidths {
        let hanzi = CHINESE
            .chars()
            .filter(|c| !crate::font::is_wide_punctuation(*c));
        CellWidths::wide(hanzi).in_lang(Some("zh"), true)
    }

    #[test]
    fn a_chinese_paragraph_wraps_within_the_width_by_its_rules() {
        let widths = chinese();
        assert_eq!(
            word_wrap(CHINESE, 16, &widths),
            [
                "他说：“今天天气",
                "很好，我们去公园",
                "散步吧。”她回",
                "答：“好啊……不",
                "过我有点累——我",
                "们明天去，好",
                "吗？”《西游记》",
                "是一本书（很有",
                "名）。",
            ]
        );
        // From 6, every piece the rules keep whole (`啊……`, `吗？”`) fits a
        // line; narrower, such a piece is cut anyway.
        for width in 6..40 {
            let lines = word_wrap(CHINESE, width, &widths);
            assert_eq!(lines.concat(), CHINESE, "at {width}");
            for (i, line) in lines.iter().enumerate() {
                assert!(text_width(line, &widths) <= width, "{lines:?}");
                assert!(
                    !line.starts_with(['，', '。', '”', '》', '）']),
                    "{lines:?}"
                );
                assert!(!line.ends_with(['“', '《', '（']), "{lines:?}");
                // `……` and `——` stay whole: a line never starts with the
                // second half of either.
                let next = lines.get(i + 1);
                for pair in ['…', '—'] {
                    let split = line.ends_with(pair) && next.is_some_and(|n| n.starts_with(pair));
                    assert!(!split, "{pair} split at {width}: {lines:?}");
                }
            }
        }
    }

    #[test]
    fn chinese_punctuation_never_starts_or_ends_a_line() {
        let widths = chinese();
        // `，` would start the second line: `好` goes down with it.
        assert_eq!(word_wrap("很好，我们", 4, &widths), ["很", "好，", "我们"]);
        // `“` would end the first line: it goes down with what it opens.
        assert_eq!(word_wrap("他说“好”", 6, &widths), ["他说", "“好”"]);
        // `……` is never split, even where half of it fits.
        assert_eq!(word_wrap("好啊……", 6, &widths), ["好", "啊……"]);
        assert_eq!(word_wrap("累——我", 6, &widths), ["累——", "我"]);
        // `《` would end the first line, `》` start the last.
        assert_eq!(
            word_wrap("他是《西游记》", 6, &widths),
            ["他是", "《西游", "记》"]
        );
    }

    #[test]
    fn narrow_quotes_and_dashes_wrap_as_english_does() {
        // Narrow, `“`, `…` and `—` are English's: the rules for them in
        // Chinese don't apply, and English wraps as it always has.
        let text = "she said “wait… no—go” and left";
        let lines = wrap(text, 10);
        assert_eq!(lines, ["she said", "“wait…", "no—go” and", "left"]);
        assert_eq!(lines, word_wrap(text, 10, &chinese().in_lang(None, false)));
    }

    fn words(max_len: usize) -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(
            prop::string::string_regex(&format!("[a-zA-Z.,!?']{{1,{max_len}}}")).unwrap(),
            0..40,
        )
    }

    /// Japanese text with Latin runs, punctuation and brackets, and no
    /// spaces.
    fn japanese_text() -> impl Strategy<Value = String> {
        prop::collection::vec(
            prop::string::string_regex("[今日はいい天気ですね。、「」（）ょっーab1]{1,4}").unwrap(),
            0..30,
        )
        .prop_map(|parts| parts.concat())
    }

    proptest! {
        #[test]
        fn no_line_is_too_wide(ws in words(30), width in 1usize..40) {
            for line in wrap(&ws.join(" "), width) {
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
            let lines = wrap(&text, width);
            let rejoined: Vec<&str> = lines.iter().flat_map(|l| l.split(' ')).collect();
            prop_assert_eq!(rejoined, ws.iter().map(String::as_str).collect::<Vec<_>>());
        }

        #[test]
        fn keeps_every_character(ws in words(30), width in 1usize..40) {
            let text = ws.join(" ");
            let kept: String = wrap(&text, width).concat().replace(' ', "");
            prop_assert_eq!(kept, text.replace(' ', ""));
        }

        #[test]
        fn lines_are_filled_greedily(ws in words(12), width in 12usize..40) {
            let lines = wrap(&ws.join(" "), width);
            for pair in lines.windows(2) {
                let next_word = pair[1].split(' ').next().unwrap_or("");
                prop_assert!(
                    pair[0].chars().count() + 1 + next_word.chars().count() > width,
                    "{:?} could have taken {next_word:?}", pair[0]
                );
            }
        }

        #[test]
        fn japanese_lines_fit_and_join_back_to_the_text(
            text in japanese_text(),
            width in 2usize..30,
        ) {
            let widths = japanese();
            let lines = word_wrap(&text, width, &widths);
            for line in &lines {
                let cells = text_width(line, &widths);
                prop_assert!(cells <= width, "{line:?} is {cells} cells > {width}");
                prop_assert!(!line.is_empty());
            }
            prop_assert_eq!(lines.concat(), text);
        }

        #[test]
        fn chinese_lines_fit_and_join_back_to_the_text_with_wide_punctuation(
            parts in prop::collection::vec(
                prop::string::string_regex("[他说好我们，。“”‘’…—《》ab]{1,4}").unwrap(),
                0..30,
            ),
            width in 2usize..30,
        ) {
            let text = parts.concat();
            let widths = chinese();
            let lines = word_wrap(&text, width, &widths);
            for line in &lines {
                let cells = text_width(line, &widths);
                prop_assert!(cells <= width, "{line:?} is {cells} cells > {width}");
                prop_assert!(!line.is_empty());
            }
            prop_assert_eq!(lines.concat(), text);
        }

        #[test]
        fn japanese_lines_keep_the_rules_where_a_break_fits(
            text in japanese_text(),
            width in 8usize..30,
        ) {
            let widths = japanese();
            let lines = word_wrap(&text, width, &widths);
            for pair in lines.windows(2) {
                let (end, start) = (pair[0].chars().last(), pair[1].chars().next());
                let (Some(end), Some(start)) = (end, start) else { continue };
                // Only a piece wider than a line is cut against the rules.
                let unbreakable = pieces(&format!("{}{}", pair[0], pair[1]), &widths)
                    .iter()
                    .any(|p| text_width(p, &widths) > width);
                if !unbreakable {
                    prop_assert!(may_break(end, start, &widths), "{lines:?}");
                }
            }
        }
    }
}
