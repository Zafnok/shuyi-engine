//! Reading RON assets with `file:line:col` errors. As a format (ADR-0007),
//! [`Ron`] reads any kind that serde reads: a game's own data tables.
use crate::prelude::*;

use core::fmt;
use core::marker::PhantomData;

use serde::de::{DeserializeOwned, DeserializeSeed};

use crate::bundle::Bundle;
use crate::error::{ContentError, codes};
use crate::format::{AssetKind, AssetPath, DecodeCx, Format, TEXT_MAX_BYTES, text};

/// A RON file of a kind serde reads (`ron`, `.ron`, unless
/// [`Ron::named`] says), read as [`parse_ron`] says: a game's data table,
/// say, whose own checks run after.
pub struct Ron<T> {
    id: &'static str,
    extensions: &'static [&'static str],
    kind: PhantomData<fn() -> T>,
}

impl<T> Ron<T> {
    /// The format `ron`, of `.ron` files.
    pub const fn new() -> Self {
        Self::named("ron", &["ron"])
    }

    /// The format `id`, of files with `extensions`.
    pub const fn named(id: &'static str, extensions: &'static [&'static str]) -> Self {
        Self {
            id,
            extensions,
            kind: PhantomData,
        }
    }
}

impl<T> Default for Ron<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> fmt::Debug for Ron<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ron")
            .field("id", &self.id)
            .field("extensions", &self.extensions)
            .finish()
    }
}

impl<T: AssetKind + DeserializeOwned> Format for Ron<T> {
    type Asset = T;

    fn id(&self) -> &'static str {
        self.id
    }

    fn extensions(&self) -> &'static [&'static str] {
        self.extensions
    }

    fn max_bytes(&self) -> usize {
        TEXT_MAX_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<T, Vec<ContentError>> {
        parse_ron(file.shown(), text(file, bytes)?).map_err(|e| vec![e])
    }
}

/// Loads and deserializes the RON file at `path` in `bundle`.
pub fn load_ron<T: DeserializeOwned>(bundle: &Bundle, path: &str) -> Result<T, ContentError> {
    let display = bundle.display_path(path);
    let text = bundle
        .file(path)
        .ok_or_else(|| ContentError::not_found(&display))?;
    parse_ron(&display, text)
}

/// Deserializes RON `source`; errors are attributed to `file` (a
/// human-facing path) with RON's 1-based line and column.
pub fn parse_ron<T: DeserializeOwned>(file: &str, source: &str) -> Result<T, ContentError> {
    ::ron::from_str(source).map_err(|e| error(file, &e))
}

/// Like [`parse_ron`], with RON's `implicit_some` on: an optional field
/// is written as its value, not in `Some(…)`.
pub fn parse_ron_implicit_some<T: DeserializeOwned>(
    file: &str,
    source: &str,
) -> Result<T, ContentError> {
    ::ron::Options::default()
        .with_default_extension(::ron::extensions::Extensions::IMPLICIT_SOME)
        .from_str(source)
        .map_err(|e| error(file, &e))
}

/// Like [`parse_ron`], reading with `seed`: for a value whose shape is
/// known only at run time.
pub fn parse_ron_seed<'de, S: DeserializeSeed<'de>>(
    file: &str,
    source: &'de str,
    seed: S,
) -> Result<S::Value, ContentError> {
    ::ron::Options::default()
        .from_str_seed(source, seed)
        .map_err(|e| error(file, &e))
}

/// `e` as an error of `file`.
pub(crate) fn error(file: &str, e: &::ron::error::SpannedError) -> ContentError {
    // `span.end` is the offending token; `span.start` can be the
    // whitespace before it.
    let pos = e.span.end;
    ContentError::new(file, e.code.to_string())
        .at(to_u32(pos.line), Some(to_u32(pos.col)))
        .with_code(codes::PARSE)
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::bundle::Layer;

    #[test]
    fn parses_valid_ron() {
        let m: BTreeMap<String, u8> =
            parse_ron("x.ron", "{\"a\": 1, \"b\": 2}").unwrap_or_default();
        assert_eq!(m.get("b"), Some(&2));
    }

    #[test]
    fn syntax_error_reports_file_line_and_column() {
        // Line 3, column 10 is the stray `?` where a value should start.
        let src = "{\n    \"a\": 1,\n    \"b\": ?,\n}";
        let err = parse_ron::<BTreeMap<String, u8>>("base/data/t.ron", src)
            .err()
            .unwrap_or_else(|| ContentError::new("", "expected an error"));
        assert_eq!(err.file, "base/data/t.ron");
        assert_eq!(err.line, Some(3));
        assert_eq!(err.column, Some(10));
        assert_eq!(err.code, Some(codes::PARSE));
        assert!(err.to_string().starts_with("base/data/t.ron:3:10: "));
    }

    #[test]
    fn type_error_reports_position() {
        let src = "{\n\"a\": \"not a number\"}";
        let err = parse_ron::<BTreeMap<String, u8>>("t.ron", src)
            .err()
            .unwrap_or_else(|| ContentError::new("", "expected an error"));
        assert_eq!(err.line, Some(2));
    }

    #[test]
    fn huge_positions_saturate() {
        assert_eq!(to_u32(usize::MAX), u32::MAX);
        assert_eq!(to_u32(7), 7);
    }

    static FILES: &[(&str, &[u8])] = &[
        ("data/bad.ron", b"{\n\"a\": ?}"),
        ("data/good.ron", b"{\"a\": 1}"),
    ];

    fn bundle() -> Bundle {
        Bundle::new(vec![Layer::table("base", FILES)])
    }

    #[test]
    fn loads_a_file_of_the_bundle() {
        let m: Result<BTreeMap<String, u8>, _> = load_ron(&bundle(), "data/good.ron");
        assert_eq!(m.ok().and_then(|m| m.get("a").copied()), Some(1));
    }

    #[test]
    fn an_error_names_the_file_as_the_bundle_shows_it() {
        let err = load_ron::<BTreeMap<String, u8>>(&bundle(), "data/bad.ron")
            .err()
            .unwrap_or_else(|| ContentError::new("", "expected an error"));
        assert_eq!(
            (err.file.as_str(), err.line),
            ("base/data/bad.ron", Some(2))
        );
    }

    #[test]
    fn missing_file_names_the_path() {
        let err = load_ron::<BTreeMap<String, u8>>(&bundle(), "data/nope.ron")
            .err()
            .unwrap_or_else(|| ContentError::new("", "expected an error"));
        assert_eq!(err.file, "base/data/nope.ron");
        assert_eq!(err.line, None);
        assert!(err.message.contains("not found"));
    }
}
