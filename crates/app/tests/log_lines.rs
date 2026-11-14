//! No log line of `shuyi-app` names its argument inside its string
//! (ticket 3311).
//!
//! On the web, macroquad's logging macros (`error!`, `warn!`, `info!`,
//! `debug!`, `trace!`) hand a call with the string alone to the console as
//! it is written: `error!("failed: {e}")` prints `failed: {e}`. A call
//! that passes its arguments after the string (`error!("failed: {}", e)`)
//! prints them. This scans `src` for the first kind.

use std::fs;
use std::path::{Path, PathBuf};

/// The logging macros this scan reads.
const MACROS: [&str; 5] = ["error", "warn", "info", "debug", "trace"];

/// Whether `c` can be part of a Rust name.
fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Whether the format string `text` (its contents, between the quotes)
/// names an argument inside it: a `{` that starts a name, not `{{`, `{}`,
/// `{:?}` or `{0}`.
fn captures_a_name(text: &str) -> bool {
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            match chars.peek() {
                Some('{') => {
                    chars.next();
                }
                Some(&next) if next.is_ascii_alphabetic() || next == '_' => return true,
                _ => {}
            }
        }
    }
    false
}

/// The contents of the string literal at the start of `rest`, if it starts
/// with one (a plain `"…"`, escapes kept as written).
fn leading_string(rest: &str) -> Option<&str> {
    let rest = rest.trim_start().strip_prefix('"')?;
    let mut escaped = false;
    for (i, c) in rest.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '"' => return Some(&rest[..i]),
            _ => {}
        }
    }
    None
}

/// The format strings of the logging macros' calls in `source` that name
/// an argument inside them, each with its line number.
fn offending_calls(source: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for name in MACROS {
        let call = format!("{name}!(");
        let mut from = 0;
        while let Some(at) = source[from..].find(&call) {
            let start = from + at;
            from = start + call.len();
            let preceded_by_name = source[..start]
                .chars()
                .next_back()
                .is_some_and(is_name_char);
            if preceded_by_name {
                continue;
            }
            if let Some(text) = leading_string(&source[from..])
                && captures_a_name(text)
            {
                let line = source[..start].matches('\n').count() + 1;
                found.push((line, text.to_owned()));
            }
        }
    }
    found
}

/// Every `.rs` file under `dir`.
fn rust_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(rust_files(&path)?);
        } else if path.extension().is_some_and(|e| e == "rs") {
            files.push(path);
        }
    }
    Ok(files)
}

#[test]
fn no_log_line_names_its_argument_inside_its_string() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let files = rust_files(&src).unwrap();
    assert!(files.len() > 10, "the scan found {} files", files.len());
    let mut offenders = Vec::new();
    for file in files {
        let source = fs::read_to_string(&file).unwrap();
        for (line, text) in offending_calls(&source) {
            offenders.push(format!("{}:{line}: \"{text}\"", file.display()));
        }
    }
    assert!(
        offenders.is_empty(),
        "these log lines print the name, not the value, on the web; pass it after the string:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn the_scan_finds_a_line_written_like_the_old_ones() {
    let old = "    Err(e) => {\n        error!(\"storage: smoke check failed: {e}\");\n    }\n";
    assert_eq!(
        offending_calls(old),
        vec![(2, "storage: smoke check failed: {e}".to_owned())]
    );
    let old_long = "macroquad::prelude::error!(\n    \"storage: {e}; saves and settings will not persist\"\n);";
    assert_eq!(offending_calls(old_long).len(), 1);
}

#[test]
fn the_scan_passes_arguments_after_the_string() {
    for fine in [
        r#"error!("storage: smoke check failed: {}", e);"#,
        r#"warn!("{:?} and {0} and {{e}}", e);"#,
        r#"info!("storage: smoke check passed");"#,
        r#"my_error!("{e}");"#,
        "warn!(message);",
    ] {
        assert_eq!(offending_calls(fine), Vec::new(), "{fine}");
    }
}
