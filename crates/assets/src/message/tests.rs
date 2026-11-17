use alloc::vec::Vec;

use proptest::prelude::*;

use super::*;

/// `text` in `lang` with `n` for `{n}` (and nothing else).
fn with_n(text: &str, lang: &str, n: i64) -> String {
    let message = Message::parse(text).unwrap();
    message.format(lang, |name| (name == "n").then_some(Arg::Int(n)))
}

fn error(text: &str) -> String {
    Message::parse(text).unwrap_err().to_string()
}

const KEYS: &str = "{n, plural, one {# key} other {# keys}}";

#[test]
fn english_plurals_pick_one_and_other() {
    assert_eq!(with_n(KEYS, "en", 1), "1 key");
    assert_eq!(with_n(KEYS, "en", 2), "2 keys");
    assert_eq!(with_n(KEYS, "en", 0), "0 keys");
    assert_eq!(with_n(KEYS, "en", -1), "-1 key");
}

#[test]
fn russian_picks_one_few_and_many_and_french_counts_zero_as_one() {
    let keys = "{n, plural, one {# ключ} few {# ключа} many {# ключей} other {# ключа}}";
    let ru = |n| with_n(keys, "ru", n);
    assert_eq!([ru(1), ru(21)], ["1 ключ", "21 ключ"]);
    assert_eq!([ru(2), ru(22)], ["2 ключа", "22 ключа"]);
    assert_eq!([ru(5), ru(11)], ["5 ключей", "11 ключей"]);
    let fr = "{n, plural, one {# clé} other {# clés}}";
    assert_eq!(with_n(fr, "fr", 0), "0 clé");
    assert_eq!(with_n(fr, "fr", 1), "1 clé");
    assert_eq!(with_n(fr, "fr", 2), "2 clés");
}

#[test]
fn english_ordinals() {
    let rank = "{n, selectordinal, one {#st} two {#nd} few {#rd} other {#th}}";
    let got: Vec<String> = [1, 2, 3, 4, 11, 12, 13, 21, 112]
        .into_iter()
        .map(|n| with_n(rank, "en", n))
        .collect();
    assert_eq!(
        got,
        [
            "1st", "2nd", "3rd", "4th", "11th", "12th", "13th", "21st", "112th"
        ]
    );
}

#[test]
fn an_exact_branch_wins_over_a_category() {
    let text = "{n, plural, =0 {none} =1 {just one} one {# one} other {#}}";
    assert_eq!(with_n(text, "en", 0), "none");
    assert_eq!(with_n(text, "en", 1), "just one");
    assert_eq!(with_n(text, "en", 7), "7");
    assert_eq!(
        with_n("{n, plural, =-2 {minus two} other {#}}", "en", -2),
        "minus two"
    );
}

#[test]
fn a_select_compares_text_and_an_int_by_its_digits() {
    let text = "{device, select, playdate {Crank} 3 {three} other {Turn}}";
    let message = Message::parse(text).unwrap();
    let with = |arg: Option<Arg>| message.format("en", |_| arg.clone());
    assert_eq!(with(Some(Arg::Text("playdate".into()))), "Crank");
    assert_eq!(with(Some(Arg::Text("keyboard".into()))), "Turn");
    assert_eq!(with(Some(Arg::Int(3))), "three");
    assert_eq!(with(None), "Turn");
}

#[test]
fn a_plural_with_text_or_no_value_takes_other() {
    let message = Message::parse(KEYS).unwrap();
    let text = message.format("en", |_| Some(Arg::Text("1".into())));
    assert_eq!(text, "1 keys");
    assert_eq!(message.format("en", |_| None), "{n} keys");
}

#[test]
fn branches_nest_and_a_hash_is_its_nearest_plurals() {
    let text = "{who, select, she {{n, plural, one {She has # key} other {She has # keys}}} \
                other {{m, plural, one {# and {n, plural, other {#}}} other {#}}}}";
    let message = Message::parse(text).unwrap();
    let values = |who: &str| {
        let who = who.to_owned();
        move |name: &str| match name {
            "who" => Some(Arg::Text(who.clone())),
            "n" | "m" => Some(Arg::Int(1)),
            _ => None,
        }
    };
    assert_eq!(message.format("en", values("she")), "She has 1 key");
    assert_eq!(message.format("en", values("they")), "1 and 1");
    // A `#` in a select inside a plural is the plural's number.
    let inner = "{n, plural, other {{g, select, other {# them}}}}";
    assert_eq!(with_n(inner, "en", 4), "4 them");
}

#[test]
fn quoting_writes_braces_and_apostrophes() {
    let format = |text| Message::parse(text).unwrap().format("en", |_| None);
    assert_eq!(format("'{'"), "{");
    assert_eq!(format("''"), "'");
    assert_eq!(format("l'épée, don't"), "l'épée, don't");
    assert_eq!(format("'{name}' is {name}"), "{name} is {name}");
    assert_eq!(format("it''s '{a}''s'"), "it's {a}'s");
    assert_eq!(format("'}' and a lone } and #"), "} and a lone } and #");
    assert_eq!(format("open '{ to the end"), "open { to the end");
    assert_eq!(with_n("{n, plural, other {'#' is #}}", "en", 5), "# is 5");
    // Outside a plural, `#` is a letter, and so is the `'` before it.
    assert_eq!(format("'#' and '#"), "'#' and '#");
}

#[test]
fn a_number_is_a_simple_argument_written_in_digits() {
    assert_eq!(with_n("{n, number} and { n }", "en", 1234), "1234 and 1234");
}

#[test]
fn walk_hands_out_text_numbers_and_arguments() {
    let message = Message::parse("Press {Confirm}: {n, plural, other {# left}}").unwrap();
    let mut pieces = Vec::new();
    message.walk(
        "en",
        |name| (name == "n").then_some(Arg::Int(2)),
        |piece| {
            pieces.push(match piece {
                Piece::Text(text) => format!("t:{text}"),
                Piece::Number(n) => format!("n:{n}"),
                Piece::Arg(name) => format!("a:{name}"),
            });
        },
    );
    assert_eq!(pieces, ["t:Press ", "a:Confirm", "t:: ", "n:2", "t: left"]);
}

#[test]
fn args_name_each_use_with_its_kind() {
    let text = "{a} {n, plural, =0 {x} one {{b}} other {y}} {d, select, playdate {} other {{a}}}";
    let message = Message::parse(text).unwrap();
    let args = message.args();
    assert_eq!(
        args,
        [
            ArgUse::new("a", ArgKind::Simple),
            ArgUse::new(
                "n",
                ArgKind::Plural {
                    kind: Kind::Cardinal,
                    categories: vec![Category::One, Category::Other]
                }
            ),
            ArgUse::new("b", ArgKind::Simple),
            ArgUse::new(
                "d",
                ArgKind::Select {
                    values: vec!["playdate", "other"]
                }
            ),
            ArgUse::new("a", ArgKind::Simple),
        ]
    );
    assert_eq!(message.names(), ["a", "n", "b", "d"]);
    assert_eq!(Message::parse("no arguments").unwrap().args(), []);
}

#[test]
fn forms_are_every_text_with_arguments_in_braces() {
    let text = "{n, plural, one {# key} other {# keys}} on {d, select, playdate {A} other {B}}";
    let forms = Message::parse(text).unwrap().forms();
    assert_eq!(
        forms,
        [
            "{n} key on A",
            "{n} key on B",
            "{n} keys on A",
            "{n} keys on B"
        ]
    );
    let same = Message::parse("{d, select, a {x} other {x}}").unwrap();
    assert_eq!(same.forms(), ["x"]);
    assert_eq!(Message::parse("").unwrap().forms(), [""]);
}

#[test]
fn more_than_64_forms_is_an_error() {
    let eight = "{a, select, 1 {} 2 {} 3 {} 4 {} 5 {} 6 {} 7 {} other {}}";
    assert!(Message::parse(&format!("{eight}{eight}")).is_ok());
    assert_eq!(
        error(&format!("{eight}{eight}{eight}")),
        "at character 0: too many forms: a message shows at most 64 texts"
    );
}

#[test]
fn each_error_says_where_and_what_was_expected() {
    let cases = [
        ("é {", "at character 3: expected an argument's name"),
        ("{}", "at character 1: expected an argument's name"),
        (
            "{n",
            "at character 2: expected `}` or `,` after the argument's name",
        ),
        (
            "{a b}",
            "at character 3: expected `}` or `,` after the argument's name",
        ),
        (
            "{n,}",
            "at character 3: expected plural, selectordinal, select or number, not \"\"",
        ),
        (
            "{n, date}",
            "at character 4: expected plural, selectordinal, select or number, not \"date\"",
        ),
        (
            "{n, choice, 0#a}",
            "at character 4: expected plural, selectordinal, select or number, not \"choice\"",
        ),
        (
            "{n, spellout}",
            "at character 4: expected plural, selectordinal, select or number, not \"spellout\"",
        ),
        (
            "{n, number, ::compact}",
            "at character 10: a number takes no style yet: expected `}`",
        ),
        (
            "{n, number, integer}",
            "at character 10: a number takes no style yet: expected `}`",
        ),
        (
            "{n, plural}",
            "at character 10: expected `,` before the branches",
        ),
        (
            "{n, plural, offset:1 other {#}}",
            "at character 12: `offset:` isn't supported",
        ),
        (
            "{n, plural, some {x} other {y}}",
            "at character 12: expected zero, one, two, few, many, other or =N, not \"some\"",
        ),
        (
            "{n, plural, =x {x} other {y}}",
            "at character 12: expected a whole number after `=`",
        ),
        (
            "{n, plural, one {x}}",
            "at character 0: a plural, selectordinal or select needs an `other` branch",
        ),
        (
            "{n, select, a {x} a {y} other {}}",
            "at character 18: the branch \"a\" is written twice",
        ),
        (
            "{n, select, a x}",
            "at character 14: expected `{` to open the branch",
        ),
        (
            "{n, select, other {x",
            "at character 20: expected `}` to close the branch",
        ),
        (
            "{n, select, other {x}",
            "at character 21: expected a branch or `}`",
        ),
        (
            "{n, select, {x}}",
            "at character 12: expected a branch or `}`",
        ),
    ];
    for (text, expected) in cases {
        assert_eq!(error(text), expected, "{text}");
    }
}

#[test]
fn nesting_deeper_than_eight_is_an_error() {
    let nest = |depth: usize| {
        let mut text = String::from("x");
        for _ in 0..depth {
            text = format!("{{a, select, other {{{text}}}}}");
        }
        text
    };
    assert!(Message::parse(&nest(8)).is_ok());
    assert_eq!(
        error(&nest(9)),
        format!(
            "at character {}: plurals and selects nest at most 8 deep",
            8 * 19
        )
    );
}

#[test]
fn every_integer_type_text_and_a_char_are_arguments() {
    assert_eq!(3u8.to_arg(), Arg::Int(3));
    assert_eq!((-3i32).to_arg(), Arg::Int(-3));
    // `usize` is 32 bits on wasm32: one that fits is an `Int`.
    assert_eq!(7usize.to_arg(), Arg::Int(7));
    assert_eq!(u128::MAX.to_arg(), Arg::Text(u128::MAX.to_string()));
    assert_eq!(u64::MAX.to_arg(), Arg::Text("18446744073709551615".into()));
    assert_eq!((i64::MAX as u64).to_arg(), Arg::Int(i64::MAX));
    assert_eq!(i128::MIN.to_arg(), Arg::Text(i128::MIN.to_string()));
    assert_eq!("a".to_arg(), Arg::Text("a".into()));
    assert_eq!(String::from("b").to_arg(), Arg::Text("b".into()));
    assert_eq!('c'.to_arg(), Arg::Text("c".into()));
    assert_eq!((&&5u16).to_arg(), Arg::Int(5));
    assert_eq!(Arg::Int(1).to_arg(), Arg::Int(1));
    assert_eq!(Arg::Text("x".into()).to_string(), "x");
    assert_eq!(MessageError::new(2, "p").to_string(), "at character 2: p");
}

proptest! {
    #[test]
    fn parse_never_panics(text in "\\PC*") {
        let _ = Message::parse(&text).map(|m| (m.forms(), m.args(), m.format("ru", |_| None)));
    }

    #[test]
    fn parse_never_panics_on_message_like_text(text in "[{}#', a-z=0-9]{0,40}") {
        let _ = Message::parse(&text).map(|m| (m.forms(), m.format("en", |_| Some(Arg::Int(2)))));
    }

    #[test]
    fn format_gives_one_of_the_forms_filled_in(n in -30i64..30, word in "[a-z]{1,6}") {
        let text = "{w} {n, plural, =0 {none} one {# {w}} few {few #} other {{w, select, ab {AB} other {#}}}}";
        let message = Message::parse(text).unwrap();
        let value = |name: &str| match name {
            "n" => Some(Arg::Int(n)),
            "w" => Some(Arg::Text(word.clone())),
            _ => None,
        };
        for lang in ["en", "ru", "pl"] {
            let shown = message.format(lang, value);
            let filled: Vec<String> = message
                .forms()
                .iter()
                .map(|f| f.replace("{n}", &n.to_string()).replace("{w}", &word))
                .collect();
            prop_assert!(filled.contains(&shown), "{} not in {:?}", shown, filled);
        }
    }
}
