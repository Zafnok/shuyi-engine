//! `table!("<folder>")`: the table of a folder's files, written at compile
//! time, for a crate with no build script. `shuyi_assets::files!` expands
//! to it, hidden; a game names `shuyi::files!`, never this crate.
//!
//! The folder is a string literal, relative to the crate's folder, and may
//! name environment variables the build sets (`$CARGO_MANIFEST_DIR`, or one
//! a build script sets, as a new game's `$SHUYI_FILES`). It expands to
//! `shuyi-embed`'s [`table_source`](shuyi_embed::table_source) of
//! [`walk`](shuyi_embed::walk): a `&'static [(&'static str, &'static [u8])]`
//! sorted by path, refusing what `walk` refuses.
//!
//! A change to a file it embedded rebuilds the crate (`include_bytes!`).
//! A file added or removed rebuilds it only when something watches the
//! folder: a build script's `cargo::rerun-if-changed` (ADR-0136).

use std::path::{Path, PathBuf};

use proc_macro::{Delimiter, TokenStream, TokenTree};

/// The table of the folder named by a string literal (the crate's docs).
#[proc_macro]
pub fn table(input: TokenStream) -> TokenStream {
    let source = folder(input)
        .and_then(|folder| {
            let root = resolve(&folder, |name| std::env::var(name).ok())?;
            let walk = shuyi_embed::walk(&root, &[]).map_err(|e| e.to_string())?;
            Ok(format!(
                "{{ const TABLE: &[(&str, &[u8])] = {}; TABLE }}",
                shuyi_embed::table_source(&walk.files)
            ))
        })
        .unwrap_or_else(|error| format!("compile_error!({:?})", format!("files!: {error}")));
    source.parse().unwrap_or_else(|_| {
        "compile_error!(\"files!: no table\")"
            .parse()
            .unwrap_or_default()
    })
}

/// The string literal `input` holds, unquoted.
fn folder(input: TokenStream) -> Result<String, String> {
    let mut tokens: Vec<TokenTree> = input.into_iter().collect();
    // A `macro_rules!` fragment arrives in a group without delimiters.
    while let [TokenTree::Group(group)] = tokens.as_slice() {
        if group.delimiter() != Delimiter::None {
            break;
        }
        tokens = group.stream().into_iter().collect();
    }
    match tokens.as_slice() {
        [TokenTree::Literal(literal)] => unquote(&literal.to_string()),
        _ => Err("expected one string literal, the folder".to_owned()),
    }
}

/// The text of string literal `literal`, as Rust writes one: `"…"` with its
/// escapes, or raw, `r"…"` or `r#"…"#`.
fn unquote(literal: &str) -> Result<String, String> {
    let not_a_string = || format!("`{literal}` is not a string literal");
    if let Some(raw) = literal.strip_prefix('r') {
        let hashes = raw.len() - raw.trim_start_matches('#').len();
        let fence = "#".repeat(hashes);
        return raw
            .strip_prefix(&fence)
            .and_then(|r| r.strip_prefix('"'))
            .and_then(|r| r.strip_suffix(&fence))
            .and_then(|r| r.strip_suffix('"'))
            .map(str::to_owned)
            .ok_or_else(not_a_string);
    }
    let body = literal
        .strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
        .ok_or_else(not_a_string)?;
    let mut text = String::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            text.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => text.push('\\'),
            Some('"') => text.push('"'),
            Some('\'') => text.push('\''),
            Some('n') => text.push('\n'),
            Some('t') => text.push('\t'),
            Some('r') => text.push('\r'),
            Some('0') => text.push('\0'),
            other => {
                return Err(format!(
                    "`{literal}`: the escape `\\{}` is not read in a folder's name",
                    other.map(String::from).unwrap_or_default()
                ));
            }
        }
    }
    Ok(text)
}

/// The folder `folder` names, with each `$NAME` replaced by what `var` gives
/// for it, and taken from the crate's folder when relative.
fn resolve(folder: &str, var: impl Fn(&str) -> Option<String>) -> Result<PathBuf, String> {
    let mut path = String::new();
    let mut rest = folder;
    while let Some(at) = rest.find('$') {
        path.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(after.len());
        let name = &after[..end];
        if name.is_empty() {
            return Err(format!("`{folder}`: a `$` names no variable"));
        }
        let value = var(name).ok_or_else(|| format!("`{folder}`: `${name}` is not set"))?;
        path.push_str(&value);
        rest = &after[end..];
    }
    path.push_str(rest);
    let path = PathBuf::from(path);
    if path.is_absolute() {
        return Ok(path);
    }
    let here = var("CARGO_MANIFEST_DIR")
        .ok_or_else(|| format!("`{folder}`: relative, and `$CARGO_MANIFEST_DIR` is not set"))?;
    Ok(Path::new(&here).join(path))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{resolve, unquote};

    #[test]
    fn string_literals_are_read_plain_escaped_and_raw() {
        assert_eq!(unquote("\"files\"").as_deref(), Ok("files"));
        assert_eq!(
            unquote(r#""C:\\game\\files""#).as_deref(),
            Ok(r"C:\game\files")
        );
        assert_eq!(unquote(r#"r"C:\game""#).as_deref(), Ok(r"C:\game"));
        assert_eq!(
            unquote(r###"r#"say "files""#"###).as_deref(),
            Ok(r#"say "files""#)
        );
        assert!(unquote("files").is_err());
        assert!(unquote("42").is_err());
        assert!(unquote(r#""\u{41}""#).is_err());
        assert!(unquote(r##"r#"unclosed""##).is_err());
    }

    fn vars(name: &str) -> Option<String> {
        match name {
            "CARGO_MANIFEST_DIR" => Some("/game/crates/game".to_owned()),
            "SHUYI_FILES" => Some("/out/files".to_owned()),
            _ => None,
        }
    }

    #[test]
    fn variables_are_replaced_and_a_relative_folder_is_the_crates() {
        assert_eq!(
            resolve("$CARGO_MANIFEST_DIR/files", vars),
            Ok(PathBuf::from("/game/crates/game/files"))
        );
        assert_eq!(
            resolve("$SHUYI_FILES", vars),
            Ok(PathBuf::from("/out/files"))
        );
        assert_eq!(
            resolve("files@playdate", vars),
            Ok(PathBuf::from("/game/crates/game").join("files@playdate"))
        );
    }

    #[test]
    fn an_unset_variable_or_a_bare_dollar_is_an_error() {
        let unset = resolve("$NOT_SET/files", vars);
        assert!(unset.is_err_and(|e| e.contains("$NOT_SET")));
        assert!(resolve("files/$", vars).is_err());
        assert!(resolve("files", |_| None).is_err());
    }
}
