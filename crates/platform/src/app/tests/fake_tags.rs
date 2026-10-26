//! Fake memory tags that note the names entered, on this thread: the
//! App's tests of its phase tags, with `std` for the thread's notes.
#![cfg(feature = "std")]

use super::*;

std::thread_local! {
    static NAMES: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    static ENTERED: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    static CURRENT: core::cell::Cell<u16> = const { core::cell::Cell::new(0) };
}

fn tag(_: u8, name: &'static str) -> u16 {
    NAMES.with_borrow_mut(|names| {
        let at = names.iter().position(|&n| n == name).unwrap_or_else(|| {
            names.push(name);
            names.len() - 1
        });
        u16::try_from(at + 1).unwrap()
    })
}

fn enter(tag: u16) -> u16 {
    let name = NAMES.with_borrow(|names| names[usize::from(tag) - 1]);
    ENTERED.with_borrow_mut(|entered| entered.push(name));
    CURRENT.replace(tag)
}

fn leave(before: u16) {
    CURRENT.set(before);
}

pub(super) fn scopes() -> MemoryScopes {
    MemoryScopes::new(tag, enter, leave)
}

/// The names entered since the last call, and whether every tag
/// entered was left.
pub(super) fn take() -> (Vec<&'static str>, bool) {
    (ENTERED.take(), CURRENT.get() == 0)
}
