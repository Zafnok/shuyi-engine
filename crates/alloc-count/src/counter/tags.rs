//! Memory tags (ADR-0046 §6): what a [`CountingAlloc::with_tags`] counts
//! each block under. A tag is a number, `0` to `255`, for a name in a
//! process-wide table: the frame's phase, the game's span, or the asset
//! kind being decoded. Code tags what it allocates by entering a tag
//! around it, and the innermost tag entered wins:
//!
//! ```
//! use shuyi_alloc_count::tags;
//!
//! const SPAN: u8 = 2;
//! let ai = tags::tag(SPAN, "ai");
//! let before = tags::enter(ai);
//! // … what this thread allocates here counts under `ai` …
//! tags::leave(before);
//! assert_eq!(tags::current(), before);
//! ```
//!
//! - **The names** are a table of [`TAGS`]. Tag [`OTHER`] is `other`, the
//!   tag of what is allocated outside any; tag [`OVERFLOW`] is
//!   `overflow`, which every name past the 254th shares, and
//!   [`overflowed`] counts those names. [`tag`] finds a name, or adds it,
//!   under the table's lock, never in the allocator's path. A name's class is
//!   a number its user gives meaning to (`shuyi_ui::perf::TagClass`);
//!   `other` and `overflow` have [`OTHER_CLASS`].
//! - **The current tag** is per thread (with `std`; without, one static
//!   for the target's one thread): [`enter`] sets it and gives back the
//!   one before, and [`leave`] puts that back. Reading it never
//!   allocates, so the allocator reads it on every call.
//!
//! [`CountingAlloc::with_tags`]: crate::CountingAlloc::with_tags

#[cfg(not(feature = "std"))]
use core::cell::UnsafeCell;

/// How many tags there are: [`OTHER`], 254 names, and [`OVERFLOW`].
pub const TAGS: usize = 256;

/// The tag of what is allocated outside any tag: `other`.
pub const OTHER: u16 = 0;

/// The tag every name past the 254th shares: `overflow`. A tag entered
/// that is higher (no tag [`tag`] gave) counts here too.
pub const OVERFLOW: u16 = 255;

/// The class of `other` and `overflow`: `shuyi_ui::perf::TagClass::Other`.
pub const OTHER_CLASS: u8 = 0;

/// The names with a tag of their own: `other` and 254 more.
const NAMED: usize = OVERFLOW as usize;

/// The names past those remembered, each counted once in [`overflowed`].
const REMEMBERED: usize = 256;

/// A name in the table: its class and its text.
type Name = (u8, &'static str);

/// The names given, in the order they were first asked for: the first
/// [`NAMED`] have the tag of their place, the rest share [`OVERFLOW`].
struct Table {
    names: [Name; NAMED + REMEMBERED],
    len: usize,
}

impl Table {
    const fn new() -> Self {
        let mut names = [(OTHER_CLASS, ""); NAMED + REMEMBERED];
        names[OTHER as usize] = (OTHER_CLASS, "other");
        Self { names, len: 1 }
    }

    /// The tag of `class`'s `name`, which it gets now if it had none.
    fn find_or_add(&mut self, class: u8, name: &'static str) -> u16 {
        let given = &self.names[..self.len];
        let at = match given.iter().position(|&n| n == (class, name)) {
            Some(at) => at,
            None if self.len < self.names.len() => {
                self.names[self.len] = (class, name);
                self.len += 1;
                self.len - 1
            }
            // Past every name remembered: `overflow`, uncounted.
            None => NAMED,
        };
        u16::try_from(at.min(NAMED)).unwrap_or(OVERFLOW)
    }

    /// The class and name of each tag: `""` for one no name has yet.
    fn names(&self) -> [Name; TAGS] {
        let mut names = [(OTHER_CLASS, ""); TAGS];
        let given = self.len.min(NAMED);
        names[..given].copy_from_slice(&self.names[..given]);
        names[OVERFLOW as usize] = (OTHER_CLASS, "overflow");
        names
    }

    fn overflowed(&self) -> u32 {
        u32::try_from(self.len.saturating_sub(NAMED)).unwrap_or(u32::MAX)
    }
}

/// The process's one table, behind a lock: `std`'s `Mutex`, which never
/// allocates, and is never taken inside the allocator.
#[cfg(feature = "std")]
static NAMES: std::sync::Mutex<Table> = std::sync::Mutex::new(Table::new());

/// Runs `f` on the process's table, holding its lock.
#[cfg(feature = "std")]
fn with_names<R>(f: impl FnOnce(&mut Table) -> R) -> R {
    let mut table = NAMES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f(&mut table)
}

/// The table without `std`, for the target's one thread.
#[cfg(not(feature = "std"))]
struct Names(UnsafeCell<Table>);

// SAFETY: without `std` the target has one thread (as the current tag's
// static says), so no two threads ever reach the table.
#[cfg(not(feature = "std"))]
unsafe impl Sync for Names {}

#[cfg(not(feature = "std"))]
static NAMES: Names = Names(UnsafeCell::new(Table::new()));

/// Runs `f` on the process's table.
#[cfg(not(feature = "std"))]
fn with_names<R>(f: impl FnOnce(&mut Table) -> R) -> R {
    // SAFETY: one thread, and `f` never comes back here: no other
    // reference to the table lives while this one does.
    f(unsafe { &mut *NAMES.0.get() })
}

/// The tag of `class`'s `name`, the same one each time it is asked for
/// (names compare by class and text), and a new one the first time:
/// [`OVERFLOW`] past the 254th name. Takes the table's lock: call it
/// outside the allocator, never from inside it.
pub fn tag(class: u8, name: &'static str) -> u16 {
    with_names(|table| table.find_or_add(class, name))
}

/// How many names [`tag`] gave [`OVERFLOW`]: each counted once, up to
/// 256 of them.
pub fn overflowed() -> u32 {
    with_names(|table| table.overflowed())
}

/// The class and name of every tag, by number: for a counter's rows.
pub(crate) fn names() -> [Name; TAGS] {
    with_names(|table| table.names())
}

/// The tag a number counts under: itself, or [`OVERFLOW`] past it.
pub(crate) fn slot(tag: u16) -> usize {
    usize::from(tag.min(OVERFLOW))
}

#[cfg(feature = "std")]
std::thread_local! {
    /// This thread's current tag. A `const` initializer and no destructor:
    /// reading it never allocates, so the allocator may.
    static CURRENT: core::cell::Cell<u16> = const { core::cell::Cell::new(OTHER) };
}

/// The current tag on a target's one thread, without `std`.
#[cfg(not(feature = "std"))]
static CURRENT: super::Word = super::Word::new();

/// This thread's current tag: what it allocates now counts under it.
/// [`OTHER`] until a tag is entered (and on a thread being torn down).
pub fn current() -> u16 {
    #[cfg(feature = "std")]
    {
        CURRENT.try_with(core::cell::Cell::get).unwrap_or(OTHER)
    }
    #[cfg(not(feature = "std"))]
    {
        u16::try_from(CURRENT.get()).unwrap_or(OVERFLOW)
    }
}

/// Makes `tag` this thread's current tag, and gives back the one before
/// for [`leave`].
pub fn enter(tag: u16) -> u16 {
    #[cfg(feature = "std")]
    {
        CURRENT.try_with(|c| c.replace(tag)).unwrap_or(OTHER)
    }
    #[cfg(not(feature = "std"))]
    {
        let before = current();
        CURRENT.set(usize::from(tag));
        before
    }
}

/// Puts back `before`, the tag [`enter`] gave.
pub fn leave(before: u16) {
    #[cfg(feature = "std")]
    {
        let _ = CURRENT.try_with(|c| c.set(before));
    }
    #[cfg(not(feature = "std"))]
    {
        CURRENT.set(usize::from(before));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table of its own, so the test sees exact tags whatever other
    /// tests add to the process's.
    #[test]
    fn a_name_is_found_again_by_its_class_and_text() {
        let mut table = Table::new();
        let ai = table.find_or_add(2, "ai");
        assert_eq!(ai, 1);
        // Another `&'static str` with the same text.
        let text = String::from("ai").leak();
        assert_eq!(table.find_or_add(2, text), ai);
        assert_eq!(table.find_or_add(1, "ai"), 2, "another class");
        assert_eq!(table.find_or_add(OTHER_CLASS, "other"), OTHER);
        let names = table.names();
        assert_eq!(&names[..3], [(0, "other"), (2, "ai"), (1, "ai")]);
        assert_eq!(names[3], (0, ""));
        assert_eq!(names[255], (0, "overflow"));
    }

    #[test]
    fn names_past_the_254th_share_overflow_and_are_counted_once() {
        let mut table = Table::new();
        let texts: Vec<&'static str> = (0..600).map(|n| format!("n{n}").leak() as &str).collect();
        for (n, text) in texts.iter().enumerate().take(254) {
            assert_eq!(usize::from(table.find_or_add(1, text)), n + 1);
        }
        assert_eq!(table.overflowed(), 0);
        assert_eq!(table.find_or_add(1, texts[254]), OVERFLOW);
        assert_eq!(table.find_or_add(1, texts[254]), OVERFLOW);
        assert_eq!(table.overflowed(), 1, "a name counts once");
        for text in &texts[255..] {
            assert_eq!(table.find_or_add(1, text), OVERFLOW);
        }
        assert_eq!(table.overflowed(), 256, "at most 256 counted");
        assert_eq!(table.names()[254], (1, "n253"));
    }

    #[test]
    fn a_tag_past_overflow_counts_there() {
        assert_eq!(slot(7), 7);
        assert_eq!(slot(OVERFLOW), 255);
        assert_eq!(slot(1000), 255);
    }

    #[test]
    fn enter_and_leave_nest() {
        let outer = tag(1, "tags-test-outer");
        let inner = tag(1, "tags-test-inner");
        assert_ne!(outer, inner);
        assert_eq!(tag(1, "tags-test-outer"), outer);
        let start = current();
        let before_outer = enter(outer);
        assert_eq!(before_outer, start);
        let before_inner = enter(inner);
        assert_eq!((before_inner, current()), (outer, inner));
        leave(before_inner);
        assert_eq!(current(), outer);
        leave(before_outer);
        assert_eq!(current(), start);
    }

    #[test]
    fn each_thread_has_its_own_current_tag() {
        let mine = tag(1, "tags-test-mine");
        let before = enter(mine);
        let theirs = std::thread::spawn(current).join().unwrap();
        assert_eq!(theirs, OTHER);
        assert_eq!(current(), mine);
        leave(before);
    }
}
