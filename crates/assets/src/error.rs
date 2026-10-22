//! Content errors: every problem names the file and, when known, the line and
//! column, so a broken asset can be fixed without reading Rust. An error of
//! a class the tools know carries its problem [`Code`] ([`codes`]), which
//! `shuyi explain` prints the page of.
use crate::prelude::*;

use core::fmt;

/// A problem code: `SH` and four digits, never reused or renumbered
/// (ADR-0028). The content codes are in [`codes`]; the tools' own are in
/// `shuyi_cli::contract::codes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Code(pub &'static str);

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// The content codes (`SH2nnn`): a file of a game's content has a
/// problem. Each has a row and a page in `shuyi_cli::contract::codes`.
pub mod codes {
    use super::Code;

    /// `parse`: a text file's syntax is wrong.
    pub const PARSE: Code = Code("SH2001");
    /// `not-text`: a text format's file isn't valid UTF-8.
    pub const NOT_TEXT: Code = Code("SH2002");
    /// `missing-file`: a file asked for isn't in the bundle.
    pub const MISSING_FILE: Code = Code("SH2003");
    /// `ambiguous-file`: several files could be the asset asked for.
    pub const AMBIGUOUS_FILE: Code = Code("SH2004");
    /// `no-format`: no format of the kind reads the file.
    pub const NO_FORMAT: Code = Code("SH2005");
    /// `too-big`: the file is bigger than its format reads.
    pub const TOO_BIG: Code = Code("SH2006");
    /// `pack-differs`: a baked pack differs from its sources.
    pub const PACK_DIFFERS: Code = Code("SH2007");
    /// `unknown-name`: a name the file uses isn't one it may: an action,
    /// a key or button, a terrain, a tile character.
    pub const UNKNOWN_NAME: Code = Code("SH2008");
    /// `duplicate`: something the file may give once is given twice.
    pub const DUPLICATE: Code = Code("SH2009");
    /// `out-of-range`: a number or a size is outside what it may be.
    pub const OUT_OF_RANGE: Code = Code("SH2010");
}

/// One problem found while loading or validating an asset.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub struct ContentError {
    /// Human-facing file path, e.g. `base/data/colors.ron`.
    pub file: String,
    /// 1-based line, if known.
    pub line: Option<u32>,
    /// 1-based column, if known (only shown together with a line).
    pub column: Option<u32>,
    /// What is wrong, in plain words.
    pub message: String,
    /// Its problem code, if its class has one ([`codes`]).
    pub code: Option<Code>,
}

impl ContentError {
    /// An error about a whole file (no position).
    pub fn new(file: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            line: None,
            column: None,
            message: message.into(),
            code: None,
        }
    }

    /// The error that `file` isn't in the bundle
    /// ([`NOT_FOUND`](crate::format::NOT_FOUND), [`codes::MISSING_FILE`]).
    pub fn not_found(file: impl Into<String>) -> Self {
        Self::new(file, crate::format::NOT_FOUND).with_code(codes::MISSING_FILE)
    }

    /// Attaches a 1-based line and optional column.
    #[must_use]
    pub fn at(mut self, line: u32, column: Option<u32>) -> Self {
        self.line = Some(line);
        self.column = column;
        self
    }

    /// Gives it problem code `code`.
    #[must_use]
    pub fn with_code(mut self, code: Code) -> Self {
        self.code = Some(code);
        self
    }
}

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.file)?;
        if let Some(line) = self.line {
            write!(f, ":{line}")?;
            if let Some(column) = self.column {
                write!(f, ":{column}")?;
            }
        }
        write!(f, ": {}", self.message)
    }
}

/// Every problem found in one load, reported together.
#[derive(Debug, Clone, PartialEq, Eq, Default, thiserror::Error)]
pub struct ContentErrors(pub Vec<ContentError>);

impl fmt::Display for ContentErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self.0.len();
        write!(f, "{n} content error{}:", if n == 1 { "" } else { "s" })?;
        for e in &self.0 {
            write!(f, "\n  {e}")?;
        }
        Ok(())
    }
}

/// 1-based line of the first line containing `needle`.
pub fn line_of(source: &str, needle: &str) -> Option<u32> {
    let index = source.lines().position(|l| l.contains(needle))?;
    u32::try_from(index + 1).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_with_line_and_column() {
        let e =
            ContentError::new("base/data/colors.ron", "unknown colour \"plyer\"").at(12, Some(5));
        assert_eq!(
            e.to_string(),
            "base/data/colors.ron:12:5: unknown colour \"plyer\""
        );
    }

    #[test]
    fn display_with_line_only() {
        let e = ContentError::new("a.ron", "bad").at(3, None);
        assert_eq!(e.to_string(), "a.ron:3: bad");
    }

    #[test]
    fn display_without_position() {
        let e = ContentError::new("a.ron", "missing file");
        assert_eq!(e.to_string(), "a.ron: missing file");
    }

    #[test]
    fn display_lists_all_errors() {
        let errs = ContentErrors(vec![
            ContentError::new("a.ron", "one").at(1, Some(2)),
            ContentError::new("b.ron", "two"),
        ]);
        assert_eq!(
            errs.to_string(),
            "2 content errors:\n  a.ron:1:2: one\n  b.ron: two"
        );
        let single = ContentErrors(vec![ContentError::new("a.ron", "one")]);
        assert_eq!(single.to_string(), "1 content error:\n  a.ron: one");
    }

    #[test]
    fn a_code_leaves_the_text_as_it_is() {
        let e = ContentError::new("a.ron", "bad").at(1, Some(2));
        assert_eq!(e.code, None);
        let coded = e.clone().with_code(codes::PARSE);
        assert_eq!(coded.code, Some(Code("SH2001")));
        assert_eq!(coded.to_string(), e.to_string());
        assert_eq!(codes::PARSE.to_string(), "SH2001");
        let missing = ContentError::not_found("b.png");
        assert_eq!(missing.to_string(), "b.png: file not found in asset bundle");
        assert_eq!(missing.code, Some(codes::MISSING_FILE));
    }

    #[test]
    fn line_of_finds_first_match() {
        assert_eq!(line_of("a\nbb\nb", "b"), Some(2));
        assert_eq!(line_of("a", "z"), None);
    }
}
