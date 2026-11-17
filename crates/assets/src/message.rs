//! Messages that choose their words: a text of a language file in a subset
//! of ICU `MessageFormat` 1 (ADR-0126), which picks a wording by a number's
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
//! ([`plural`]); [`Message::format`] writes it.
use crate::prelude::*;

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

/// What may be handed to a message as a value ([`Arg`]): every integer
/// type, text and characters. An integer too big for an `i64` becomes
/// [`Arg::Text`] of its digits.
pub trait ToArg {
    /// It as an argument.
    fn to_arg(&self) -> Arg;
}

macro_rules! int_args {
    ($($t:ty),*) => {$(
        impl ToArg for $t {
            fn to_arg(&self) -> Arg {
                i64::try_from(*self).map_or_else(|_| Arg::Text(self.to_string()), Arg::Int)
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
}

impl ToArg for String {
    fn to_arg(&self) -> Arg {
        Arg::Text(self.clone())
    }
}

impl ToArg for char {
    fn to_arg(&self) -> Arg {
        Arg::Text(self.to_string())
    }
}

impl ToArg for Arg {
    fn to_arg(&self) -> Arg {
        self.clone()
    }
}

impl<T: ToArg + ?Sized> ToArg for &T {
    fn to_arg(&self) -> Arg {
        (**self).to_arg()
    }
}

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message<'a> {
    nodes: Vec<Node<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Node<'a> {
    Text(&'a str),
    /// A `#`, and the name of the plural it stands for.
    Hash(&'a str),
    Simple(&'a str),
    Choice(Choice<'a>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Choice<'a> {
    name: &'a str,
    /// `None` for a select.
    plural: Option<Kind>,
    branches: Vec<Branch<'a>>,
    /// Which branch is `other`.
    other: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Branch<'a> {
    key: Selector<'a>,
    body: Vec<Node<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Selector<'a> {
    Exact(i64),
    Category(Category),
    Value(&'a str),
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
        let mut parser = Parser { text, at: 0 };
        let nodes = parser.pattern(None, 0)?;
        if count(&nodes) > MAX_FORMS {
            let problem = format!("too many forms: a message shows at most {MAX_FORMS} texts");
            return Err(MessageError::new(0, problem));
        }
        Ok(Self { nodes })
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
        walk(&self.nodes, lang, &value, &mut each, None);
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
        uses(&self.nodes, &mut out);
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
        for form in forms(&self.nodes) {
            if !out.contains(&form) {
                out.push(form);
            }
        }
        out
    }
}

/// How many texts `nodes` can show (at most `usize::MAX`).
fn count(nodes: &[Node<'_>]) -> usize {
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

fn walk(
    nodes: &[Node<'_>],
    lang: &str,
    value: &dyn Fn(&str) -> Option<Arg>,
    each: &mut dyn FnMut(Piece<'_>),
    number: Option<&Arg>,
) {
    for node in nodes {
        match node {
            Node::Text(text) => each(Piece::Text(text)),
            Node::Simple(name) => each(Piece::Arg(name)),
            Node::Hash(name) => match number {
                Some(Arg::Int(n)) => each(Piece::Number(*n)),
                Some(Arg::Text(text)) => each(Piece::Text(text)),
                None => each(Piece::Arg(name)),
            },
            Node::Choice(choice) => {
                let arg = value(choice.name);
                let branch = choice.pick(lang, arg.as_ref());
                let number = if choice.plural.is_some() {
                    arg.as_ref()
                } else {
                    number
                };
                walk(&branch.body, lang, value, each, number);
            }
        }
    }
}

impl<'a> Choice<'a> {
    /// The branch `arg` picks in `lang`: a plural's `=N`, else its
    /// category's; a select's value; else `other`.
    fn pick(&self, lang: &str, arg: Option<&Arg>) -> &Branch<'a> {
        let find = |key: Selector<'_>| self.branches.iter().find(|b| b.key == key);
        let found = match (self.plural, arg) {
            (Some(kind), Some(Arg::Int(n))) => find(Selector::Exact(*n)).or_else(|| {
                let n = n.unsigned_abs();
                find(Selector::Category(match kind {
                    Kind::Cardinal => plural::cardinal(lang, n),
                    Kind::Ordinal => plural::ordinal(lang, n),
                }))
            }),
            (None, Some(arg)) => {
                let text = arg.to_string();
                self.branches
                    .iter()
                    .find(|b| matches!(b.key, Selector::Value(v) if v == text))
            }
            _ => None,
        };
        found.unwrap_or(&self.branches[self.other])
    }
}

fn uses<'a>(nodes: &[Node<'a>], out: &mut Vec<ArgUse<'a>>) {
    for node in nodes {
        match node {
            Node::Simple(name) => out.push(ArgUse::new(name, ArgKind::Simple)),
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
                                Selector::Value(v) => Some(v),
                                _ => None,
                            })
                            .collect(),
                    },
                };
                out.push(ArgUse::new(choice.name, kind));
                for branch in &choice.branches {
                    uses(&branch.body, out);
                }
            }
            Node::Text(_) | Node::Hash(_) => {}
        }
    }
}

fn forms(nodes: &[Node<'_>]) -> Vec<String> {
    let mut acc = vec![String::new()];
    for node in nodes {
        match node {
            Node::Text(text) => acc.iter_mut().for_each(|form| form.push_str(text)),
            Node::Simple(name) | Node::Hash(name) => {
                for form in &mut acc {
                    let _ = write!(form, "{{{name}}}");
                }
            }
            Node::Choice(choice) => {
                let alternatives: Vec<String> = choice
                    .branches
                    .iter()
                    .flat_map(|b| forms(&b.body))
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

    /// The text up to the first character `stop` is true of, or the end.
    fn until(&mut self, stop: impl Fn(char) -> bool) -> &'a str {
        let rest = self.rest();
        let end = rest.find(stop).unwrap_or(rest.len());
        self.at += end;
        &rest[..end]
    }

    /// The byte at `at` as text: one ASCII character.
    fn one(&self, at: usize) -> &'a str {
        &self.text[at..=at]
    }

    /// Text and arguments up to the end or, in a branch (`depth` > 0), up
    /// to the `}` that closes it, which is left. `hash` is the plural a
    /// `#` stands for.
    fn pattern(
        &mut self,
        hash: Option<&'a str>,
        depth: usize,
    ) -> Result<Vec<Node<'a>>, MessageError> {
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
                    nodes.push(Node::Text(self.one(self.at)));
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
    fn apostrophe(&mut self, in_plural: bool, nodes: &mut Vec<Node<'a>>) {
        let start = self.at;
        self.at += 1;
        match self.peek() {
            Some('\'') => {
                nodes.push(Node::Text(self.one(start)));
                self.at += 1;
            }
            Some('{' | '}') => self.quoted(nodes),
            Some('#') if in_plural => self.quoted(nodes),
            _ => nodes.push(Node::Text(self.one(start))),
        }
    }

    /// Quoted text, from after its opening `'` to the next lone `'` or the
    /// end; `''` in it is one apostrophe.
    fn quoted(&mut self, nodes: &mut Vec<Node<'a>>) {
        while !self.overrun(nodes.len()) {
            let text = self.until(|c| c == '\'');
            if !text.is_empty() {
                nodes.push(Node::Text(text));
            }
            if !self.eat('\'') {
                return;
            }
            if self.peek() != Some('\'') {
                return;
            }
            nodes.push(Node::Text(self.one(self.at)));
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
    fn argument(&mut self, hash: Option<&'a str>, depth: usize) -> Result<Node<'a>, MessageError> {
        let open = self.at;
        self.at += 1;
        self.skip_space();
        let name_at = self.at;
        let name = self.until(|c| c.is_whitespace() || matches!(c, '{' | '}' | ','));
        if name.is_empty() {
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
        let kind = self.until(|c| !c.is_ascii_alphanumeric());
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
        let mut branches: Vec<Branch<'a>> = Vec::new();
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
            if word.is_empty() {
                return Err(self.error(key_at, "expected a branch or `}`"));
            }
            let key = self.key(plural.is_some(), word, key_at)?;
            if branches.iter().any(|b| b.key == key) {
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
        let is_other = |b: &Branch<'_>| {
            matches!(
                b.key,
                Selector::Category(Category::Other) | Selector::Value("other")
            )
        };
        let Some(other) = branches.iter().position(is_other) else {
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
    fn key(&self, plural: bool, word: &'a str, at: usize) -> Result<Selector<'a>, MessageError> {
        if !plural {
            return Ok(Selector::Value(word));
        }
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
