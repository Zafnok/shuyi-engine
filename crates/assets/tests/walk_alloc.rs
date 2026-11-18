//! What walking a message costs (ticket 2268): a select on `device` and a
//! `{name}` filled with text allocate nothing when their values are lent
//! ([`Message::walk_lent`]), nor do a number, a character and an integer
//! too big for an `i64`. One test: the heap counter is the whole
//! process's, so nothing runs beside it.

use shuyi_alloc_count::global;
use shuyi_assets::message::{Message, Piece, ToArg};

/// The heap's calls since the last reading: only a build that counts runs
/// this test.
fn allocs() -> u32 {
    global::reading()
        .unwrap_or_else(|| panic!("this build doesn't count"))
        .allocs
}

#[test]
fn a_lent_walk_allocates_nothing() {
    let hint = "{device, select, playdate {Crank} other {Turn}} to browse · {coins, plural, \
                one {# coin} other {# coins}} · {name} · {big, select, \
                340282366920938463463374607431768211455 {max} other {less}} · {c, select, é {E} \
                other {C}} {big}";
    let message = Message::parse(hint).unwrap_or_else(|e| panic!("{e}"));
    let name = String::from("Mei");
    let values: [(&str, &dyn ToArg); 5] = [
        ("device", &"playdate"),
        ("coins", &120u32),
        ("name", &name),
        ("big", &u128::MAX),
        ("c", &'é'),
    ];
    let value = |key: &str| values.iter().find(|(n, _)| *n == key).map(|(_, v)| *v);
    // What a caller writes, measured by its length: a `{name}` written as
    // it is lent.
    let mut written = 0;
    let mut write = |piece: Piece<'_>| match piece {
        Piece::Text(text) => written += text.len(),
        Piece::Number(n) => written += usize::try_from(n.unsigned_abs().ilog10() + 1).unwrap_or(0),
        Piece::Arg(name) => {
            if let Some(value) = value(name) {
                value.with_arg_ref(&mut |arg| arg.with_str(|text| written += text.len()));
            }
        }
        _ => {}
    };
    // A first walk, then the count.
    message.walk_lent("en", value, &mut write);
    allocs();
    for _ in 0..10 {
        message.walk_lent("en", value, &mut write);
    }
    assert_eq!(allocs(), 0);
    let line =
        "Crank to browse · 120 coins · Mei · max · E 340282366920938463463374607431768211455";
    assert_eq!(written, 11 * line.len());
}
