//! Reading RON assets with `file:line:col` errors.
use crate::prelude::*;

use serde::de::DeserializeOwned;

use crate::bundle::Bundle;
use crate::error::ContentError;

/// Loads and deserializes the RON file at `path` in `bundle`.
pub fn load_ron<T: DeserializeOwned>(bundle: &Bundle, path: &str) -> Result<T, ContentError> {
    let display = bundle.display_path(path);
    let text = bundle
        .file(path)
        .ok_or_else(|| ContentError::new(&display, "file not found in asset bundle"))?;
    parse_ron(&display, text)
}

/// Deserializes RON `source`; errors are attributed to `file` (a
/// human-facing path) with RON's 1-based line and column.
pub fn parse_ron<T: DeserializeOwned>(file: &str, source: &str) -> Result<T, ContentError> {
    ::ron::from_str(source).map_err(|e| {
        // `span.end` is the offending token; `span.start` can be the
        // whitespace before it.
        let pos = e.span.end;
        ContentError::new(file, e.code.to_string()).at(to_u32(pos.line), Some(to_u32(pos.col)))
    })
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
