//! Messages that choose their words: a text of a language file in a subset
//! of ICU `MessageFormat` 1 (ADR-0127), which picks a wording by a number's
//! plural category, its rank, or a value.
//!
//! ```text
//! {name}                                     a value, or a key's name ({Confirm})
//! {n, number}                                a number, as plain digits for now
//! {n, plural, =0 {no keys} one {# key} other {# keys}}
//! {n, selectordinal, one {#st} two {#nd} few {#rd} other {#th}}
//! {device, select, playdate {Crank it} other {Turn it}}
//! ```
//!
//! A plural, selectordinal and select has an `other` branch, and branches
//! may nest. In a plural or selectordinal branch, `#` is the number. An
//! apostrophe is a letter ("l'épée", "don't"), except that `''` is one
//! apostrophe and a `'` right before `{`, `}` or (in a plural's branch)
//! `#` starts quoted text, up to the next lone `'`: `'{'` writes `{`.
//!
//! [`Message::parse`] reads one; [`Message::walk`] hands out what it says
//! for some values, in [`Piece`]s, by the plural rules of a language
//! ([`plural`]), and [`Message::walk_lent`] the same for values lent
//! ([`ArgRef`]), copying none; [`Message::format`] writes it.
use crate::prelude::*;

use alloc::borrow::Cow;
use core::fmt::{self, Write as _};

pub mod plural;

use plural::{Category, Kind};

/// What a built-in [`DEVICE`] argument may be: the kind of device the
/// player pressed last, which a `select` may choose by.
pub const DEVICES: &[&str] = &[
    "keyboard",
    "xbox",
    "playstation",
    "playstation4",
    "nintendo",
    "playdate",
    "generic",
];

/// The built-in argument naming the device the player pressed last (one of
/// [`DEVICES`]), which every screen text and tip may use.
pub const DEVICE: &str = "device";

/// The built-in argument naming the target the game runs on, by its
/// profile's id (`desktop`, `web`, `playdate`).
pub const TARGET: &str = "target";

/// The most texts a message may show ([`Message::forms`]): more is a parse
/// error.
pub const MAX_FORMS: usize = 64;

/// How deep plurals and selects may nest.
const MAX_DEPTH: usize = 8;

/// A value a message is given: a whole number, or text.
///
/// A set that grows (ADR-0039, kind 6): fixed-point numbers come with 1119.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Arg {
    /// A whole number: a plural picks by it, `{name}` and `#` write its
    /// digits.
    Int(i64),
    /// Text: a plural picks `other` for it, a select compares it.
    Text(String),
}

impl fmt::Display for Arg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(n) => write!(f, "{n}"),
            Self::Text(text) => f.write_str(text),
        }
    }
}

impl Arg {
    /// It borrowed: what a walk reads ([`ArgRef`]).
    pub fn as_arg_ref(&self) -> ArgRef<'_> {
        match self {
            Self::Int(n) => ArgRef::Int(*n),
            Self::Text(text) => ArgRef::Text(text),
        }
    }
}

/// A value a message is given, borrowed ([`Arg`] without its `String`):
/// what [`ToArg::with_arg_ref`] lends a walk ([`Message::walk_lent`]), so
/// a select and a `{name}` filled with text copy nothing.
///
/// A set that grows, as [`Arg`] does (ADR-0039, kind 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ArgRef<'a> {
    /// A whole number: a plural picks by it, `{name}` and `#` write its
    /// digits.
    Int(i64),
    /// Text: a plural picks `other` for it, a select compares it.
    Text(&'a str),
}

impl ArgRef<'_> {
    /// Hands `f` it as text: an [`Int`](Self::Int)'s digits, written on
    /// the stack, or the text. What a select compares and a `{name}`
    /// writes; nothing is allocated.
    pub fn with_str<R>(self, f: impl FnOnce(&str) -> R) -> R {
        match self {
            Self::Int(n) => {
                let mut digits = Digits::new();
                let _ = write!(digits, "{n}");
                f(digits.as_str())
            }
            Self::Text(text) => f(text),
        }
    }
}

impl From<ArgRef<'_>> for Arg {
    fn from(arg: ArgRef<'_>) -> Self {
        match arg {
            ArgRef::Int(n) => Self::Int(n),
            ArgRef::Text(text) => Self::Text(text.to_owned()),
        }
    }
}

/// Text written on the stack: the digits of any integer (an `i128`'s
/// take 40 bytes, its sign among them) or a character. More is cut off.
struct Digits {
    buf: [u8; 40],
    len: usize,
}

impl Digits {
    fn new() -> Self {
        Self {
            buf: [0; 40],
            len: 0,
        }
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or_default()
    }
}

impl fmt::Write for Digits {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len + s.len();
        let room = self.buf.get_mut(self.len..end).ok_or(fmt::Error)?;
        room.copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

/// What may be handed to a message as a value ([`Arg`]): every integer
/// type, text and characters. An integer too big for an `i64` becomes
/// [`Arg::Text`] of its digits.
///
/// A game's own type implements [`to_arg`](Self::to_arg) and may
/// implement [`with_arg_ref`](Self::with_arg_ref) too, to lend its value
/// without copying it.
pub trait ToArg {
    /// It as an argument.
    fn to_arg(&self) -> Arg;

    /// Lends it, borrowed, to `lend`, called once: what a walk reads
    /// ([`Message::walk_lent`]). By default it lends
    /// [`to_arg`](Self::to_arg)'s result, which may allocate; text, a
    /// character and every integer type lend theirs without allocating (a
    /// character and an integer too big for an `i64` written on the
    /// stack). A callback rather than a return value, as a character has
    /// no `str` to borrow.
    ///
    /// An implementation that never calls `lend` is read as no value:
    /// a plural or select picks `other`, and a `{name}` writes nothing.
    /// One that calls it again is heard once.
    fn with_arg_ref(&self, lend: &mut dyn FnMut(ArgRef<'_>)) {
        lend(self.to_arg().as_arg_ref());
    }
}

macro_rules! int_args {
    ($($t:ty),*) => {$(
        impl ToArg for $t {
            fn to_arg(&self) -> Arg {
                i64::try_from(*self).map_or_else(|_| Arg::Text(self.to_string()), Arg::Int)
            }

            fn with_arg_ref(&self, lend: &mut dyn FnMut(ArgRef<'_>)) {
                // `map_err`: an `i64`'s own conversion never fails.
                if let Ok(n) = i64::try_from(*self).map_err(|_| ()) {
                    lend(ArgRef::Int(n));
                } else {
                    let mut digits = Digits::new();
                    let _ = write!(digits, "{self}");
                    lend(ArgRef::Text(digits.as_str()));
                }
            }
        }
    )*};
}

int_args!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

impl ToArg for str {
    fn to_arg(&self) -> Arg {
        Arg::Text(self.to_owned())
    }

    fn with_arg_ref(&self, lend: &mut dyn FnMut(ArgRef<'_>)) {
        lend(ArgRef::Text(self));
    }
}

impl ToArg for String {
    fn to_arg(&self) -> Arg {
        Arg::Text(self.clone())
    }

    fn with_arg_ref(&self, lend: &mut dyn FnMut(ArgRef<'_>)) {
        lend(ArgRef::Text(self));
    }
}

impl ToArg for char {
    fn to_arg(&self) -> Arg {
        Arg::Text(self.to_string())
    }

    fn with_arg_ref(&self, lend: &mut dyn FnMut(ArgRef<'_>)) {
        let mut buf = [0; 4];
        lend(ArgRef::Text(self.encode_utf8(&mut buf)));
    }
}

impl ToArg for Arg {
    fn to_arg(&self) -> Arg {
        self.clone()
    }

    fn with_arg_ref(&self, lend: &mut dyn FnMut(ArgRef<'_>)) {
        lend(self.as_arg_ref());
    }
}

impl<T: ToArg + ?Sized> ToArg for &T {
    fn to_arg(&self) -> Arg {
        (**self).to_arg()
    }

    fn with_arg_ref(&self, lend: &mut dyn FnMut(ArgRef<'_>)) {
        (**self).with_arg_ref(lend);
    }
}

/// Lends `value` to `then` ([`ToArg::with_arg_ref`]), `None` if it lends
/// nothing: `then` is called exactly once.
fn lend_once(value: Option<&dyn ToArg>, then: &mut dyn FnMut(Option<ArgRef<'_>>)) {
    let mut lent = false;
    if let Some(value) = value {
        value.with_arg_ref(&mut |arg| {
            if !lent {
                lent = true;
                then(Some(arg));
            }
        });
    }
    if !lent {
        then(None);
    }
}

/// How a walk looks a value up: it hands the value of a name, borrowed, to
/// a continuation, which walks on inside it.
type Lookup<'l> = dyn Fn(&str, &mut dyn FnMut(Option<ArgRef<'_>>)) + 'l;

/// A piece of what a message says ([`Message::walk`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Piece<'p> {
    /// Text to show as it is.
    Text(&'p str),
    /// A `#`: the number its plural chose by.
    Number(i64),
    /// A `{name}`: the caller writes its value, or a key's name.
    Arg(&'p str),
}

/// Why a text isn't a message: where, in characters from its start, and
/// what was expected there.
///
/// Kind 8 of ADR-0039 in spirit: built by [`MessageError::new`], read by
/// its fields.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MessageError {
    /// Where, in characters (not bytes) from the start of the text.
    pub offset: usize,
    /// What is wrong, mostly what was expected there.
    pub problem: String,
}

impl MessageError {
    /// The error `problem` at character `offset`.
    pub fn new(offset: usize, problem: impl Into<String>) -> Self {
        Self {
            offset,
            problem: problem.into(),
        }
    }
}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at character {}: {}", self.offset, self.problem)
    }
}

impl core::error::Error for MessageError {}

/// How a message uses an argument ([`Message::args`]).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ArgKind<'a> {
    /// Written as it is: `{name}` or `{name, number}`.
    Simple,
    /// Chosen by: a plural (`kind` [`Kind::Cardinal`]) or selectordinal
    /// ([`Kind::Ordinal`]), with the categories it has branches for (not
    /// its `=N` branches).
    Plural {
        /// Cardinal or ordinal.
        kind: Kind,
        /// The categories of its branches, in the order written.
        categories: Vec<Category>,
    },
    /// Chosen by its value: a select, with the values of its branches
    /// (`other` among them).
    Select {
        /// The values of its branches, in the order written.
        values: Vec<&'a str>,
    },
}

/// An argument a message uses, and how ([`Message::args`]).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ArgUse<'a> {
    /// Its name.
    pub name: &'a str,
    /// How it is used.
    pub kind: ArgKind<'a>,
}

impl<'a> ArgUse<'a> {
    /// `name`, used as `kind`.
    pub fn new(name: &'a str, kind: ArgKind<'a>) -> Self {
        Self { name, kind }
    }
}

/// A text read as a message ([`Message::parse`]), borrowing its text.
///
/// Its pieces are where they are in the text, in bytes, so a parse can be
/// kept apart from its text: a language keeps its messages' parses when it
/// loads, and lends each with its text ([`Lang::form_in`], ADR-0131).
///
/// [`Lang::form_in`]: crate::lang::Lang::form_in
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message<'a> {
    text: &'a str,
    nodes: Cow<'a, [Node]>,
}

/// Where a piece is in its message's text, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Span {
    start: usize,
    end: usize,
}

impl Span {
    /// The piece in `text`: empty if `text` isn't the message's (never, as
    /// a parse is only ever lent with its own text).
    fn of(self, text: &str) -> &str {
        text.get(self.start..self.end).unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Node {
    Text(Span),
    /// A `#`, and the name of the plural it stands for.
    Hash(Span),
    Simple(Span),
    Choice(Choice),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Choice {
    name: Span,
    /// `None` for a select.
    plural: Option<Kind>,
    branches: Vec<Branch>,
    /// Which branch is `other`.
    other: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Branch {
    key: Selector,
    body: Vec<Node>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Selector {
    Exact(i64),
    Category(Category),
    Value(Span),
}

impl Selector {
    /// Whether it is `other` in `text`.
    fn is_other(self, text: &str) -> bool {
        match self {
            Self::Category(category) => category == Category::Other,
            Self::Value(value) => value.of(text) == "other",
            Self::Exact(_) => false,
        }
    }

    /// Whether it selects what `other` does, both in `text`.
    fn same(self, other: Self, text: &str) -> bool {
        match (self, other) {
            (Self::Value(a), Self::Value(b)) => a.of(text) == b.of(text),
            _ => self == other,
        }
    }
}

/// A message's parse, kept apart from its text: what a language keeps of
/// each of its messages, and lends with its text ([`Parsed::with`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Parsed(Vec<Node>);

impl Parsed {
    /// The message `text` was read as: `text` must be the text it was
    /// parsed from.
    pub(crate) fn with<'a>(&'a self, text: &'a str) -> Message<'a> {
        Message {
            text,
            nodes: Cow::Borrowed(&self.0),
        }
    }
}

impl<'a> Message<'a> {
    /// Reads `text` as a message.
    ///
    /// # Errors
    ///
    /// Where `text` isn't one: an argument unclosed or with no name, a
    /// type other than `plural`, `selectordinal`, `select` and `number`
    /// (`date`, `choice`; a number's style), `offset:`, a plural branch
    /// that is no category and no `=N`, a branch written twice, no
    /// `other`, nesting deeper than 8, or more than [`MAX_FORMS`] forms.
    pub fn parse(text: &'a str) -> Result<Self, MessageError> {
        #[cfg(feature = "parse-count")]
        parse_count::count();
        let mut parser = Parser { text, at: 0 };
        let nodes = parser.pattern(None, 0)?;
        if count(&nodes) > MAX_FORMS {
            let problem = format!("too many forms: a message shows at most {MAX_FORMS} texts");
            return Err(MessageError::new(0, problem));
        }
        Ok(Self {
            text,
            nodes: Cow::Owned(nodes),
        })
    }

    /// Its text.
    pub fn text(&self) -> &'a str {
        self.text
    }

    /// Its parse, kept apart from its text.
    pub(crate) fn keep(self) -> Parsed {
        Parsed(self.nodes.into_owned())
    }

    /// Hands `each` what the message says in language `lang` (a BCP 47
    /// tag, whose plural rules count), in order: each plural,
    /// selectordinal and select picks its branch by `value` of its
    /// argument (`other` when it has none), and `{name}`s are handed out
    /// for the caller to write.
    pub fn walk(
        &self,
        lang: &str,
        value: impl Fn(&str) -> Option<Arg>,
        mut each: impl FnMut(Piece<'_>),
    ) {
        let lookup = |name: &str, then: &mut dyn FnMut(Option<ArgRef<'_>>)| {
            then(value(name).as_ref().map(Arg::as_arg_ref));
        };
        walk(&self.nodes, self.text, lang, &lookup, &mut each, None);
    }

    /// [`walk`](Self::walk), each value lent rather than given
    /// ([`ToArg::with_arg_ref`]): a select compares a text value, or a
    /// number's digits written on the stack, and copies nothing, so a
    /// walk on text, characters and integers allocates nothing. A `#` of
    /// a number too big for an `i64` is handed out as
    /// [`Piece::Text`] of its digits, as `walk` hands it.
    ///
    /// ```
    /// use shuyi_assets::message::{Message, Piece, ToArg};
    /// let hint = Message::parse("{device, select, playdate {Crank} other {Turn}} · {coins}")
    ///     .unwrap();
    /// let value = |name: &str| match name {
    ///     "device" => Some(&"playdate" as &dyn ToArg),
    ///     "coins" => Some(&120u32 as &dyn ToArg),
    ///     _ => None,
    /// };
    /// let mut pieces = Vec::new();
    /// hint.walk_lent("en", value, |piece| pieces.push(format!("{piece:?}")));
    /// assert_eq!(pieces, [r#"Text("Crank")"#, r#"Text(" · ")"#, r#"Arg("coins")"#]);
    /// ```
    pub fn walk_lent<'v>(
        &self,
        lang: &str,
        value: impl Fn(&str) -> Option<&'v dyn ToArg>,
        mut each: impl FnMut(Piece<'_>),
    ) {
        let lookup = |name: &str, then: &mut dyn FnMut(Option<ArgRef<'_>>)| {
            lend_once(value(name), then);
        };
        walk(&self.nodes, self.text, lang, &lookup, &mut each, None);
    }

    /// What the message says in language `lang`, each argument written by
    /// `value`: an [`Arg::Int`] as its digits, and one with no value as
    /// `{name}`.
    pub fn format(&self, lang: &str, value: impl Fn(&str) -> Option<Arg>) -> String {
        let mut out = String::new();
        self.walk(lang, &value, |piece| match piece {
            Piece::Text(text) => out.push_str(text),
            Piece::Number(n) => {
                let _ = write!(out, "{n}");
            }
            Piece::Arg(name) => {
                let _ = match value(name) {
                    Some(arg) => write!(out, "{arg}"),
                    None => write!(out, "{{{name}}}"),
                };
            }
        });
        out
    }

    /// Each use of an argument, in the order written (a branch's after its
    /// choice's).
    pub fn args(&self) -> Vec<ArgUse<'a>> {
        let mut out = Vec::new();
        uses(&self.nodes, self.text, &mut out);
        out
    }

    /// The names of its arguments, each once, in the order first used.
    pub fn names(&self) -> Vec<&'a str> {
        let mut names: Vec<&'a str> = Vec::new();
        for arg in self.args() {
            if !names.contains(&arg.name) {
                names.push(arg.name);
            }
        }
        names
    }

    /// Every text the message can show, each once, its arguments and `#`
    /// written as `{name}`: what a box's width is checked against.
    pub fn forms(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for form in forms(&self.nodes, self.text) {
            if !out.contains(&form) {
                out.push(form);
            }
        }
        out
    }
}

/// Whether `text` may say more as a message than its `{name}`s filled in
/// ([`split_tokens`](shuyi_core::text::split_tokens)) say: a `'` before a
/// `'` or a brace (a quote), or, after a `{` and before its `}`, a `,` (a
/// plural, select or number), a space (`{ n }`, which a message trims),
/// another `{`, or any letter outside ASCII (which may be a space too). A
/// text without any of them fills the same either way, in one pass over
/// its bytes, never parsed: plain text ([`Form::Plain`]).
pub fn has_message_syntax(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut in_braces = false;
    for (at, &byte) in bytes.iter().enumerate() {
        match byte {
            b'\'' => {
                if matches!(bytes.get(at + 1), Some(b'\'' | b'{' | b'}')) {
                    return true;
                }
            }
            b'{' | b',' | b'\t'..=b'\r' | b' ' | 0x80.. if in_braces => return true,
            b'{' => in_braces = true,
            b'}' => in_braces = false,
            _ => {}
        }
    }
    false
}

/// What a text of a language is, learned when it loads
/// ([`Lang::form_in`], ADR-0131), or when a caller reads it
/// ([`Form::of`]): with it, a fill parses nothing.
///
/// Config read by a match (ADR-0039, kind 2): a set that may grow.
///
/// [`Lang::form_in`]: crate::lang::Lang::form_in
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Form<'a> {
    /// Text with no message syntax ([`has_message_syntax`]): filled by its
    /// `{name}`s alone.
    Plain(&'a str),
    /// A message, its parse lent.
    Message(Message<'a>),
    /// Text with message syntax that doesn't parse as a message (text from
    /// outside the checked language files): filled by its `{name}`s alone,
    /// an unclosed `{` and what follows it plain text.
    NotAMessage(&'a str),
}

impl<'a> Form<'a> {
    /// The form of `text`, which parses it if it has message syntax.
    pub fn of(text: &'a str) -> Self {
        if !has_message_syntax(text) {
            return Self::Plain(text);
        }
        Message::parse(text).map_or(Self::NotAMessage(text), Self::Message)
    }

    /// Its text.
    pub fn text(&self) -> &'a str {
        match self {
            Self::Plain(text) | Self::NotAMessage(text) => text,
            Self::Message(message) => message.text(),
        }
    }
}

/// How many [`Message::parse`] calls the thread has made: what a test
/// reads to check that no frame parses a text. Only with the
/// `parse-count` feature, which only tests turn on.
#[cfg(feature = "parse-count")]
pub mod parse_count {
    use std::cell::Cell;

    std::thread_local! {
        static PARSES: Cell<usize> = const { Cell::new(0) };
    }

    pub(super) fn count() {
        PARSES.with(|parses| parses.set(parses.get() + 1));
    }

    /// The parses on this thread since the last `take`, and starts again
    /// from none.
    pub fn take() -> usize {
        PARSES.with(|parses| parses.replace(0))
    }
}

/// How many texts `nodes` can show (at most `usize::MAX`).
fn count(nodes: &[Node]) -> usize {
    nodes.iter().fold(1, |total, node| match node {
        Node::Choice(choice) => {
            let alternatives = choice
                .branches
                .iter()
                .fold(0, |sum: usize, b| sum.saturating_add(count(&b.body)));
            total.saturating_mul(alternatives)
        }
        _ => total,
    })
}

/// Hands `each` the pieces of `nodes`, `number` the value of the plural a
/// `#` stands for. A choice looks its value up by `value`, and walks its
/// branch inside the lookup, where the value is lent.
fn walk(
    nodes: &[Node],
    text: &str,
    lang: &str,
    value: &Lookup<'_>,
    each: &mut dyn FnMut(Piece<'_>),
    number: Option<ArgRef<'_>>,
) {
    for node in nodes {
        match node {
            Node::Text(piece) => each(Piece::Text(piece.of(text))),
            Node::Simple(name) => each(Piece::Arg(name.of(text))),
            Node::Hash(name) => match number {
                Some(ArgRef::Int(n)) => each(Piece::Number(n)),
                Some(ArgRef::Text(number)) => each(Piece::Text(number)),
                None => each(Piece::Arg(name.of(text))),
            },
            Node::Choice(choice) => {
                value(choice.name.of(text), &mut |arg| {
                    let branch = choice.pick(text, lang, arg);
                    let number = if choice.plural.is_some() { arg } else { number };
                    walk(&branch.body, text, lang, value, each, number);
                });
            }
        }
    }
}

impl Choice {
    /// The branch `arg` picks in `lang`: a plural's `=N`, else its
    /// category's; a select's value (a number by its digits); else
    /// `other`.
    fn pick(&self, text: &str, lang: &str, arg: Option<ArgRef<'_>>) -> &Branch {
        let find = |key: Selector| self.branches.iter().find(|b| b.key == key);
        let found = match (self.plural, arg) {
            (Some(kind), Some(ArgRef::Int(n))) => find(Selector::Exact(n)).or_else(|| {
                let n = n.unsigned_abs();
                find(Selector::Category(match kind {
                    Kind::Cardinal => plural::cardinal(lang, n),
                    Kind::Ordinal => plural::ordinal(lang, n),
                }))
            }),
            (None, Some(arg)) => arg.with_str(|value| {
                self.branches
                    .iter()
                    .find(|b| matches!(b.key, Selector::Value(v) if v.of(text) == value))
            }),
            _ => None,
        };
        found.unwrap_or(&self.branches[self.other])
    }
}

fn uses<'a>(nodes: &[Node], text: &'a str, out: &mut Vec<ArgUse<'a>>) {
    for node in nodes {
        match node {
            Node::Simple(name) => out.push(ArgUse::new(name.of(text), ArgKind::Simple)),
            Node::Choice(choice) => {
                let kind = match choice.plural {
                    Some(kind) => ArgKind::Plural {
                        kind,
                        categories: choice
                            .branches
                            .iter()
                            .filter_map(|b| match b.key {
                                Selector::Category(c) => Some(c),
                                _ => None,
                            })
                            .collect(),
                    },
                    None => ArgKind::Select {
                        values: choice
                            .branches
                            .iter()
                            .filter_map(|b| match b.key {
                                Selector::Value(v) => Some(v.of(text)),
                                _ => None,
                            })
                            .collect(),
                    },
                };
                out.push(ArgUse::new(choice.name.of(text), kind));
                for branch in &choice.branches {
                    uses(&branch.body, text, out);
                }
            }
            Node::Text(_) | Node::Hash(_) => {}
        }
    }
}

fn forms(nodes: &[Node], text: &str) -> Vec<String> {
    let mut acc = vec![String::new()];
    for node in nodes {
        match node {
            Node::Text(piece) => acc
                .iter_mut()
                .for_each(|form| form.push_str(piece.of(text))),
            Node::Simple(name) | Node::Hash(name) => {
                let name = name.of(text);
                for form in &mut acc {
                    let _ = write!(form, "{{{name}}}");
                }
            }
            Node::Choice(choice) => {
                let alternatives: Vec<String> = choice
                    .branches
                    .iter()
                    .flat_map(|b| forms(&b.body, text))
                    .collect();
                acc = acc
                    .iter()
                    .flat_map(|form| alternatives.iter().map(move |alt| format!("{form}{alt}")))
                    .collect();
            }
        }
    }
    acc
}

/// Reads a message, `at` bytes into `text`.
struct Parser<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Parser<'a> {
    fn rest(&self) -> &'a str {
        &self.text[self.at..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    /// `problem` at byte `at`, its offset counted in characters.
    fn error(&self, at: usize, problem: impl Into<String>) -> MessageError {
        MessageError::new(self.text[..at].chars().count(), problem)
    }

    fn skip_space(&mut self) {
        self.at = self.text.len() - self.rest().trim_start().len();
    }

    fn eat(&mut self, c: char) -> bool {
        let found = self.peek() == Some(c);
        if found {
            self.at += c.len_utf8();
        }
        found
    }

    /// Where the text up to the first character `stop` is true of, or the
    /// end, is.
    fn until(&mut self, stop: impl Fn(char) -> bool) -> Span {
        let rest = self.rest();
        let start = self.at;
        self.at += rest.find(stop).unwrap_or(rest.len());
        Span {
            start,
            end: self.at,
        }
    }

    /// The byte at `at` as text: one ASCII character.
    fn one(at: usize) -> Span {
        Span {
            start: at,
            end: at + 1,
        }
    }

    /// Text and arguments up to the end or, in a branch (`depth` > 0), up
    /// to the `}` that closes it, which is left. `hash` is the plural a
    /// `#` stands for.
    fn pattern(&mut self, hash: Option<Span>, depth: usize) -> Result<Vec<Node>, MessageError> {
        let mut nodes = Vec::new();
        loop {
            if self.overrun(nodes.len()) {
                return Err(self.error(self.at, "the parser stopped moving on"));
            }
            let before = self.at;
            let Some(c) = self.peek() else {
                if depth > 0 {
                    return Err(self.error(self.at, "expected `}` to close the branch"));
                }
                return Ok(nodes);
            };
            match (c, hash) {
                ('{', _) => nodes.push(self.argument(hash, depth)?),
                ('}', _) if depth > 0 => return Ok(nodes),
                ('}', _) => {
                    // A lone `}` outside every argument is a letter.
                    nodes.push(Node::Text(Self::one(self.at)));
                    self.at += 1;
                }
                ('#', Some(name)) => {
                    nodes.push(Node::Hash(name));
                    self.at += 1;
                }
                ('\'', _) => self.apostrophe(hash.is_some(), &mut nodes),
                _ => {
                    let special = |c| matches!(c, '{' | '}' | '\'') || (c == '#' && hash.is_some());
                    nodes.push(Node::Text(self.until(special)));
                }
            }
            // Each piece takes a byte or more: one that took none would
            // loop here for ever.
            if self.at == before {
                return Err(self.error(self.at, "the parser stopped moving on"));
            }
        }
    }

    /// An apostrophe: `''` is one, one before `{`, `}` or (in a plural)
    /// `#` starts quoted text, and any other is a letter.
    fn apostrophe(&mut self, in_plural: bool, nodes: &mut Vec<Node>) {
        let start = self.at;
        self.at += 1;
        match self.peek() {
            Some('\'') => {
                nodes.push(Node::Text(Self::one(start)));
                self.at += 1;
            }
            Some('{' | '}') => self.quoted(nodes),
            Some('#') if in_plural => self.quoted(nodes),
            _ => nodes.push(Node::Text(Self::one(start))),
        }
    }

    /// Quoted text, from after its opening `'` to the next lone `'` or the
    /// end; `''` in it is one apostrophe.
    fn quoted(&mut self, nodes: &mut Vec<Node>) {
        while !self.overrun(nodes.len()) {
            let text = self.until(|c| c == '\'');
            if !text.of(self.text).is_empty() {
                nodes.push(Node::Text(text));
            }
            if !self.eat('\'') {
                return;
            }
            if self.peek() != Some('\'') {
                return;
            }
            nodes.push(Node::Text(Self::one(self.at)));
            self.at += 1;
        }
    }

    /// Whether `made` pieces are more than the text has bytes: each piece
    /// of a parse that moves on takes at least one, so a loop that stopped
    /// moving ends here rather than filling the memory.
    fn overrun(&self, made: usize) -> bool {
        made > self.text.len()
    }

    /// An argument, from its `{` to its `}`.
    fn argument(&mut self, hash: Option<Span>, depth: usize) -> Result<Node, MessageError> {
        let open = self.at;
        self.at += 1;
        self.skip_space();
        let name_at = self.at;
        let name = self.until(|c| c.is_whitespace() || matches!(c, '{' | '}' | ','));
        if name.end == name.start {
            return Err(self.error(name_at, "expected an argument's name"));
        }
        self.skip_space();
        if self.eat('}') {
            return Ok(Node::Simple(name));
        }
        if !self.eat(',') {
            let problem = "expected `}` or `,` after the argument's name";
            return Err(self.error(self.at, problem));
        }
        self.skip_space();
        let type_at = self.at;
        let kind = self.until(|c| !c.is_ascii_alphanumeric()).of(self.text);
        self.skip_space();
        let plural = match kind {
            "number" if self.eat('}') => return Ok(Node::Simple(name)),
            "number" => {
                let problem = "a number takes no style yet: expected `}`";
                return Err(self.error(self.at, problem));
            }
            "plural" => Some(Kind::Cardinal),
            "selectordinal" => Some(Kind::Ordinal),
            "select" => None,
            _ => {
                let problem =
                    format!("expected plural, selectordinal, select or number, not {kind:?}");
                return Err(self.error(type_at, problem));
            }
        };
        if depth >= MAX_DEPTH {
            return Err(self.error(open, "plurals and selects nest at most 8 deep"));
        }
        if !self.eat(',') {
            return Err(self.error(self.at, "expected `,` before the branches"));
        }
        let inner = if plural.is_some() { Some(name) } else { hash };
        let mut branches: Vec<Branch> = Vec::new();
        loop {
            if self.overrun(branches.len()) {
                return Err(self.error(self.at, "the parser stopped moving on"));
            }
            self.skip_space();
            if self.eat('}') {
                break;
            }
            let key_at = self.at;
            let word = self.until(|c| c.is_whitespace() || matches!(c, '{' | '}'));
            if word.end == word.start {
                return Err(self.error(key_at, "expected a branch or `}`"));
            }
            let key = self.key(plural.is_some(), word, key_at)?;
            if branches.iter().any(|b| b.key.same(key, self.text)) {
                let word = word.of(self.text);
                let problem = format!("the branch {word:?} is written twice");
                return Err(self.error(key_at, problem));
            }
            self.skip_space();
            if !self.eat('{') {
                return Err(self.error(self.at, "expected `{` to open the branch"));
            }
            let body = self.pattern(inner, depth + 1)?;
            self.at += 1;
            branches.push(Branch { key, body });
        }
        let Some(other) = branches.iter().position(|b| b.key.is_other(self.text)) else {
            return Err(self.error(
                open,
                "a plural, selectordinal or select needs an `other` branch",
            ));
        };
        Ok(Node::Choice(Choice {
            name,
            plural,
            branches,
            other,
        }))
    }

    /// A branch's key `word`, at byte `at`: a plural's category or `=N`,
    /// or a select's value.
    fn key(&self, plural: bool, word: Span, at: usize) -> Result<Selector, MessageError> {
        if !plural {
            return Ok(Selector::Value(word));
        }
        let word = word.of(self.text);
        if word.starts_with("offset:") {
            return Err(self.error(at, "`offset:` isn't supported"));
        }
        if let Some(n) = word.strip_prefix('=') {
            return n
                .parse()
                .map(Selector::Exact)
                .map_err(|_| self.error(at, "expected a whole number after `=`"));
        }
        Category::from_name(word)
            .map(Selector::Category)
            .ok_or_else(|| {
                let problem =
                    format!("expected zero, one, two, few, many, other or =N, not {word:?}");
                self.error(at, problem)
            })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod lent_tests;
