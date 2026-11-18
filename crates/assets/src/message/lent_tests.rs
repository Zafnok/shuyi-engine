//! Values lent to a walk (ticket 2268): [`ToArg::with_arg_ref`] lends what
//! [`ToArg::to_arg`] gives, and [`Message::walk_lent`] hands out what
//! [`Message::walk`] does.

use proptest::prelude::*;

use super::*;

/// What `value` lends, as an [`Arg`], and how many times it lent.
fn lent(value: &dyn ToArg) -> (Option<Arg>, usize) {
    let (mut arg, mut times) = (None, 0);
    value.with_arg_ref(&mut |lent| {
        arg = Some(Arg::from(lent));
        times += 1;
    });
    (arg, times)
}

/// A piece, written to compare.
fn shown(piece: Piece<'_>) -> String {
    match piece {
        Piece::Text(text) => format!("t:{text}"),
        Piece::Number(n) => format!("n:{n}"),
        Piece::Arg(name) => format!("a:{name}"),
    }
}

/// The pieces `walk` hands out, each value `to_arg` of `values`'s.
fn walked(message: &Message<'_>, lang: &str, values: &[(&str, &dyn ToArg)]) -> Vec<String> {
    let mut out = Vec::new();
    let value = |name: &str| {
        let found = values.iter().find(|(n, _)| *n == name);
        found.map(|(_, value)| value.to_arg())
    };
    message.walk(lang, value, |piece| out.push(shown(piece)));
    out
}

/// The pieces `walk_lent` hands out, each of `values` lent.
fn walked_lent(message: &Message<'_>, lang: &str, values: &[(&str, &dyn ToArg)]) -> Vec<String> {
    let mut out = Vec::new();
    let value = |name: &str| values.iter().find(|(n, _)| *n == name).map(|(_, v)| *v);
    message.walk_lent(lang, value, |piece| out.push(shown(piece)));
    out
}

/// A game's own value, which only says what it is as an [`Arg`].
struct Coins(u32);

impl ToArg for Coins {
    fn to_arg(&self) -> Arg {
        Arg::Int(self.0.into())
    }
}

/// A value that lends nothing, or lends twice.
struct Lends(usize);

impl ToArg for Lends {
    fn to_arg(&self) -> Arg {
        Arg::Text("playdate".into())
    }

    fn with_arg_ref(&self, lend: &mut dyn FnMut(ArgRef<'_>)) {
        for value in ["playdate", "web"].iter().take(self.0) {
            lend(ArgRef::Text(value));
        }
    }
}

#[test]
fn each_type_lends_once_what_to_arg_gives() {
    let text = String::from("b");
    let arg = Arg::Text("x".into());
    let values: [&dyn ToArg; 22] = [
        &3u8,
        &-3i8,
        &-3i16,
        &(-3i32),
        &i64::MIN,
        &i64::MAX,
        &i128::MIN,
        &i128::MAX,
        &7isize,
        &u16::MAX,
        &u32::MAX,
        &u64::MAX,
        &(i64::MAX as u64),
        &u128::MAX,
        &usize::MAX,
        &"a",
        &text,
        &'c',
        &'é',
        &arg,
        &&&5u16,
        &Coins(12),
    ];
    for value in values {
        assert_eq!(
            lent(value),
            (Some(value.to_arg()), 1),
            "{:?}",
            value.to_arg()
        );
    }
    assert_eq!(Arg::Int(4).as_arg_ref(), ArgRef::Int(4));
    assert_eq!(ArgRef::Int(-12).with_str(str::to_owned), "-12");
    assert_eq!(ArgRef::Text("x").with_str(str::len), 1);
}

#[test]
fn a_select_compares_a_number_by_its_digits() {
    let text = "{v, select, 7 {seven} 007 {padded} -3 {minus} 340282366920938463463374607431768211455 {max} é {e} other {else}}";
    let message = Message::parse(text).unwrap();
    let pick = |value: &dyn ToArg| walked_lent(&message, "en", &[("v", value)]).join("");
    assert_eq!(pick(&7u8), "t:seven");
    assert_eq!(pick(&7i64), "t:seven");
    assert_eq!(pick(&"007"), "t:padded");
    assert_eq!(pick(&-3i32), "t:minus");
    assert_eq!(pick(&u128::MAX), "t:max");
    assert_eq!(pick(&'é'), "t:e");
    assert_eq!(pick(&70u8), "t:else");
    for value in [&7u8 as &dyn ToArg, &"007", &-3i32, &u128::MAX, &'é', &70u8] {
        assert_eq!(
            walked(&message, "en", &[("v", value)]),
            walked_lent(&message, "en", &[("v", value)]),
        );
    }
}

#[test]
fn a_big_number_is_text_to_a_plural_and_its_hash() {
    let message = Message::parse("{n, plural, one {# one} other {# many}}").unwrap();
    let big = [("n", &u128::MAX as &dyn ToArg)];
    let digits = format!("t:{}", u128::MAX);
    assert_eq!(
        walked_lent(&message, "en", &big),
        [digits.as_str(), "t: many"]
    );
    assert_eq!(
        walked(&message, "en", &big),
        walked_lent(&message, "en", &big)
    );
}

#[test]
fn a_games_own_value_fills_as_before() {
    let message = Message::parse("{c, plural, one {# coin} other {# coins}} {c}").unwrap();
    for coins in [0, 1, 2, 21] {
        let values = [("c", &Coins(coins) as &dyn ToArg)];
        assert_eq!(
            walked(&message, "en", &values),
            walked_lent(&message, "en", &values)
        );
    }
    let one = [("c", &Coins(1) as &dyn ToArg)];
    assert_eq!(
        walked_lent(&message, "en", &one),
        ["n:1", "t: coin", "t: ", "a:c"]
    );
}

#[test]
fn a_value_that_lends_nothing_is_none_and_one_lent_twice_is_heard_once() {
    let message = Message::parse("{d, select, playdate {P} web {W} other {O}}").unwrap();
    let pick = |lends: &Lends| walked_lent(&message, "en", &[("d", lends)]);
    assert_eq!(pick(&Lends(0)), ["t:O"]);
    assert_eq!(pick(&Lends(1)), ["t:P"]);
    assert_eq!(pick(&Lends(2)), ["t:P"]);
}

/// Every string literal of Rust source `source`, its escapes roughly read.
fn literals(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = source.chars();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut text = String::new();
        while let Some(c) = chars.next() {
            match c {
                '"' => break,
                '\\' => match chars.next() {
                    Some('n') => text.push('\n'),
                    Some(other) => text.push(other),
                    None => {}
                },
                _ => text.push(c),
            }
        }
        out.push(text);
    }
    out
}

#[test]
fn walk_lent_walks_every_message_of_the_message_tests_as_walk_does() {
    let big = u128::MAX;
    let other = String::from("other");
    let pool: [&dyn ToArg; 12] = [
        &0i64,
        &1u8,
        &2i32,
        &21u64,
        &"playdate",
        &"ab",
        &'a',
        &big,
        &-3i16,
        &other,
        &7usize,
        &Coins(5),
    ];
    let mut walked_any = 0;
    for text in literals(include_str!("tests.rs")) {
        let Ok(message) = Message::parse(&text) else {
            continue;
        };
        let names = message.names();
        for shift in 0..pool.len() {
            let values: Vec<(&str, &dyn ToArg)> = names
                .iter()
                .enumerate()
                .map(|(at, name)| (*name, pool[(at + shift) % pool.len()]))
                .collect();
            for lang in ["en", "ru", "fr", "pl"] {
                let lent = walked_lent(&message, lang, &values);
                assert_eq!(walked(&message, lang, &values), lent, "{text:?} in {lang}");
                // With no values at all, too.
                assert_eq!(
                    walked(&message, lang, &[]),
                    walked_lent(&message, lang, &[])
                );
            }
        }
        walked_any += 1;
    }
    assert!(walked_any > 40, "{walked_any}");
}

proptest! {
    #[test]
    fn walk_lent_hands_out_what_walk_does(
        text in r"([a-z ]{1,3}|#|\{n\}|\{w\}|\{n, plural, =0 \{z\} one \{# o\} other \{# x \{w\}\}\}|\{n, selectordinal, two \{#nd\} other \{#th\}\}|\{w, select, ab \{A\} 7 \{S\} other \{\{n, plural, other \{#\}\}\}\}|\{c, select, é \{E\} other \{C\}\}){0,8}",
        n in proptest::num::i64::ANY,
        small in -30i64..30,
        word in "(ab|7|other|[a-z]{1,3})",
        c in proptest::char::any(),
        big in proptest::num::u128::ANY,
    ) {
        let Ok(message) = Message::parse(&text) else {
            return Ok(());
        };
        let word = word.as_str();
        let sets: [[(&str, &dyn ToArg); 3]; 3] = [
            [("n", &n), ("w", &word), ("c", &c)],
            [("n", &small), ("w", &7u8), ("c", &"é")],
            [("n", &big), ("w", &c), ("c", &word)],
        ];
        for values in &sets {
            for lang in ["en", "ru", "cy"] {
                prop_assert_eq!(walked(&message, lang, values), walked_lent(&message, lang, values));
            }
        }
    }
}
