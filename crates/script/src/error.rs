//! What can go wrong with a script: [`ScriptError`], always a value, never
//! a panic. What it means for the game is the caller's to decide.

use core::fmt;

use rhai::{EvalAltResult, ParseError, ParseErrorType, Position};

/// A script that didn't compile, didn't run to its end, or gave what the
/// game can't read: which script, where in it, and what.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ScriptError {
    /// The script's name, as it was compiled: its file's display path.
    pub script: String,
    /// The line, from 1, when the error has one.
    pub line: Option<usize>,
    /// The column on that line, from 1.
    pub column: Option<usize>,
    /// What went wrong.
    pub kind: ErrorKind,
}

/// What went wrong with a script.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// It didn't compile: the language's message.
    Compile(String),
    /// It stopped at one of its [`Limits`](crate::Limits).
    Limit(Limit),
    /// It failed while running: a name not found, a type it can't use, an
    /// overflow, a division by zero, a `throw`.
    Run(String),
    /// The view didn't convert into the script's values.
    View(String),
    /// What it gave didn't read as the type asked for.
    Value(String),
}

/// The limit a script stopped at ([`Limits`](crate::Limits)' fields).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Limit {
    /// Its operations.
    Operations,
    /// A string's bytes.
    Text,
    /// A list's items.
    List,
    /// A map's entries.
    Map,
    /// The variables in scope.
    Variables,
    /// The depth of its calls.
    CallDepth,
    /// The depth of an expression.
    ExpressionDepth,
}

impl ScriptError {
    /// A compile error of the script `name`.
    pub(crate) fn from_parse(name: &str, error: &ParseError) -> Self {
        let kind = match error.err_type() {
            ParseErrorType::ExprTooDeep => ErrorKind::Limit(Limit::ExpressionDepth),
            other => ErrorKind::Compile(other.to_string()),
        };
        Self::at(name, error.position(), kind)
    }

    /// A run's error, from the script `name`: the innermost one, where a
    /// call inside the script failed, and its place.
    pub(crate) fn from_eval(name: &str, error: EvalAltResult) -> Self {
        let mut inner = innermost(error);
        let place = inner.take_position();
        let kind = match &inner {
            EvalAltResult::ErrorTooManyOperations(..) => ErrorKind::Limit(Limit::Operations),
            EvalAltResult::ErrorTooManyVariables(..) => ErrorKind::Limit(Limit::Variables),
            EvalAltResult::ErrorStackOverflow(..) => ErrorKind::Limit(Limit::CallDepth),
            EvalAltResult::ErrorDataTooLarge(what, ..) => ErrorKind::Limit(data_limit(what)),
            EvalAltResult::ErrorParsing(ParseErrorType::ExprTooDeep, ..) => {
                ErrorKind::Limit(Limit::ExpressionDepth)
            }
            EvalAltResult::ErrorParsing(error, ..) => ErrorKind::Compile(error.to_string()),
            other => ErrorKind::Run(other.to_string()),
        };
        Self::at(name, place, kind)
    }

    /// The view didn't convert.
    pub(crate) fn view(name: &str, error: &EvalAltResult) -> Self {
        Self::at(name, Position::NONE, ErrorKind::View(error.to_string()))
    }

    /// The result didn't read.
    pub(crate) fn value(name: &str, error: &EvalAltResult) -> Self {
        Self::at(name, Position::NONE, ErrorKind::Value(error.to_string()))
    }

    fn at(name: &str, place: Position, kind: ErrorKind) -> Self {
        Self {
            script: name.into(),
            line: place.line(),
            column: place.position(),
            kind,
        }
    }
}

/// The error a call inside the script failed with, out of the calls
/// around it.
fn innermost(error: EvalAltResult) -> EvalAltResult {
    match error {
        EvalAltResult::ErrorInFunctionCall(.., inner, _)
        | EvalAltResult::ErrorInModule(_, inner, _) => innermost(*inner),
        other => other,
    }
}

/// Which size limit a data error names: Rhai says it in words.
fn data_limit(what: &str) -> Limit {
    if what.contains("string") {
        Limit::Text
    } else if what.contains("map") {
        Limit::Map
    } else {
        Limit::List
    }
}

impl fmt::Display for ScriptError {
    /// `mods/rain.rhai:3:7: ...`, as a compiler names a place.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.script)?;
        if let Some(line) = self.line {
            write!(f, ":{line}")?;
            if let Some(column) = self.column {
                write!(f, ":{column}")?;
            }
        }
        write!(f, ": {}", self.kind)
    }
}

impl core::error::Error for ScriptError {}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compile(message) => write!(f, "doesn't compile: {message}"),
            Self::Limit(limit) => write!(f, "stopped at its limit of {limit}"),
            Self::Run(message) => write!(f, "failed: {message}"),
            Self::View(message) => write!(f, "the view doesn't convert: {message}"),
            Self::Value(message) => write!(f, "what it gave doesn't read: {message}"),
        }
    }
}

impl fmt::Display for Limit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Operations => "operations",
            Self::Text => "a string's bytes",
            Self::List => "a list's items",
            Self::Map => "a map's entries",
            Self::Variables => "variables",
            Self::CallDepth => "call depth",
            Self::ExpressionDepth => "expression depth",
        })
    }
}
