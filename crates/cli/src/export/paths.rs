//! Nothing of the build machine in a package: the build's paths remapped
//! to short fixed names ([`remapped_flags`]), and the built binary searched
//! for what is left of them ([`search`]).
//!
//! Cargo's `trim-paths` isn't stable on the pinned Rust, so the remapping
//! is rustc's `--remap-path-prefix`, passed through
//! `CARGO_ENCODED_RUSTFLAGS` after the flags already in it or in
//! `RUSTFLAGS`. That variable wins over the `rustflags` of a game's
//! `.cargo/config.toml`, which these builds then don't get.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// What starts the error [`search`]'s finding gives: `web::run` returns
/// it as text, and the web exporter tells it from a failed build by it.
pub const FOUND: &str = "the build holds a path of the build machine";

/// Cargo's flags variable, its arguments separated by `\x1f`.
pub const ENCODED: &str = "CARGO_ENCODED_RUSTFLAGS";

/// The folders a release build is remapped from: the workspace's root
/// (to the package's name), `CARGO_HOME` (`/cargo`), `RUSTUP_HOME`
/// (`/rustup`) and the target folder (`/target`), in that order. rustc
/// tries the last first, so a folder inside another one still gets its
/// own name.
pub fn remaps(workspace: &Path, target: &Path, name: &str) -> Vec<(PathBuf, String)> {
    let mut remaps = vec![(workspace.to_path_buf(), name.to_owned())];
    if let Some(cargo) = cargo_home() {
        remaps.push((cargo, "/cargo".to_owned()));
    }
    if let Some(rustup) = rustup_home() {
        remaps.push((rustup, "/rustup".to_owned()));
    }
    remaps.push((target.to_path_buf(), "/target".to_owned()));
    remaps
}

/// The home folder: `HOME`, else `USERPROFILE`.
pub fn home() -> Option<PathBuf> {
    ["HOME", "USERPROFILE"]
        .into_iter()
        .filter_map(std::env::var_os)
        .find(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// `CARGO_HOME`, else `.cargo` in the home folder.
pub fn cargo_home() -> Option<PathBuf> {
    std::env::var_os("CARGO_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| home().map(|home| home.join(".cargo")))
}

/// `RUSTUP_HOME`, else `.rustup` in the home folder.
fn rustup_home() -> Option<PathBuf> {
    std::env::var_os("RUSTUP_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| home().map(|home| home.join(".rustup")))
}

/// The value of [`ENCODED`] for a build with `remaps`: the flags already
/// in `encoded` (that variable), else those in `plain` (`RUSTFLAGS`, split
/// on white space), then a `--remap-path-prefix` for each remap.
pub fn remapped_flags(
    encoded: Option<OsString>,
    plain: Option<OsString>,
    remaps: &[(PathBuf, String)],
) -> String {
    let mut flags: Vec<String> = match (encoded, plain) {
        (Some(encoded), _) if !encoded.is_empty() => encoded
            .to_string_lossy()
            .split('\x1f')
            .map(str::to_owned)
            .collect(),
        (_, Some(plain)) => plain
            .to_string_lossy()
            .split_whitespace()
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    };
    for (from, to) in remaps {
        flags.push(format!("--remap-path-prefix={}={to}", from.display()));
    }
    flags.join("\x1f")
}

/// [`remapped_flags`] with this process's variables.
pub fn remapped_flags_here(remaps: &[(PathBuf, String)]) -> String {
    remapped_flags(
        std::env::var_os(ENCODED),
        std::env::var_os("RUSTFLAGS"),
        remaps,
    )
}

/// The paths a package must not hold: the home folder, `CARGO_HOME` and
/// the `roots` (the project's and the workspace's).
pub fn needles(roots: &[&Path]) -> Vec<PathBuf> {
    let mut needles: Vec<PathBuf> = home().into_iter().chain(cargo_home()).collect();
    needles.extend(roots.iter().map(|root| root.to_path_buf()));
    needles
}

/// The first of `needles` that `bytes` hold, with either slash (or both)
/// and in any case of its ASCII letters, as the needle is written. A path
/// of fewer than two parts (`/`, `C:\`) is never looked for.
pub fn search(bytes: &[u8], needles: &[PathBuf]) -> Option<String> {
    let alike = |byte: &u8| match byte.to_ascii_lowercase() {
        b'\\' => b'/',
        other => other,
    };
    let haystack: Vec<u8> = bytes.iter().map(alike).collect();
    for needle in needles {
        let text = needle.to_string_lossy();
        let trimmed = text.trim_end_matches(['/', '\\']);
        let parts = trimmed.split(['/', '\\']).filter(|p| !p.is_empty()).count();
        if parts < 2 {
            continue;
        }
        let form: Vec<u8> = trimmed.as_bytes().iter().map(alike).collect();
        if contains(&haystack, &form) {
            return Some(trimmed.to_owned());
        }
    }
    None
}

/// Whether `haystack` holds `needle`.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

/// [`search`] on the file at `file`, as the error the package is refused
/// with.
pub fn check_file(file: &Path, needles: &[PathBuf]) -> Result<(), String> {
    let bytes = std::fs::read(file).map_err(|e| format!("read {}: {e}", file.display()))?;
    match search(&bytes, needles) {
        Some(found) => Err(format!(
            "{FOUND}: {} holds {found}; the package is refused",
            file.display()
        )),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_search_finds_a_home_folder_with_either_slash_in_any_case() {
        let needles = [PathBuf::from(r"C:\Users\Someone")];
        for bytes in [
            &b"panicked at C:\\Users\\Someone\\.cargo\\registry\\src\\x.rs"[..],
            b"at C:/Users/Someone/.cargo/registry",
            b"at c:\\users\\someone\\x",
            b"at C:\\Users/Someone/x",
        ] {
            assert_eq!(
                search(bytes, &needles).as_deref(),
                Some(r"C:\Users\Someone")
            );
        }
        let unix = [PathBuf::from("/home/runner/")];
        assert_eq!(
            search(b"/home/runner/work/x/src/lib.rs", &unix).as_deref(),
            Some("/home/runner")
        );
        assert_eq!(
            search(b"\\home\\runner\\work", &unix).as_deref(),
            Some("/home/runner")
        );
    }

    #[test]
    fn the_search_passes_the_remapped_names_and_skips_a_root() {
        let needles = [
            PathBuf::from("/home/runner"),
            PathBuf::from("/"),
            PathBuf::from(r"C:\"),
            PathBuf::new(),
        ];
        let remapped = b"/cargo/registry/src/index.crates.io-1/x/src/lib.rs \
                         /rustup/toolchains/x /target/release/build game/crates/core/src/lib.rs";
        assert_eq!(search(remapped, &needles), None);
        assert_eq!(search(b"", &needles), None);
    }

    #[test]
    fn a_file_with_a_path_is_refused_by_name() {
        let dir = std::env::temp_dir().join(format!("shuyi-paths-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("game.wasm");
        std::fs::write(&file, b"\0\0/home/runner/work/a.rs\0").unwrap();
        let error = check_file(&file, &[PathBuf::from("/home/runner")]).unwrap_err();
        assert!(error.starts_with(FOUND), "{error}");
        assert!(error.ends_with("holds /home/runner; the package is refused"));
        std::fs::write(&file, b"/cargo/registry").unwrap();
        assert_eq!(check_file(&file, &[PathBuf::from("/home/runner")]), Ok(()));
        assert!(check_file(&dir.join("missing"), &[]).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_flags_go_after_the_ones_already_set_and_never_replace_them() {
        let remaps = [
            (PathBuf::from("/work/game"), "game".to_owned()),
            (PathBuf::from("/home/me/.cargo"), "/cargo".to_owned()),
        ];
        let tail =
            "--remap-path-prefix=/work/game=game\x1f--remap-path-prefix=/home/me/.cargo=/cargo";
        // RUSTFLAGS, split on white space.
        assert_eq!(
            remapped_flags(None, Some("-D warnings".into()), &remaps),
            format!("-D\x1fwarnings\x1f{tail}")
        );
        // CARGO_ENCODED_RUSTFLAGS wins over RUSTFLAGS, as in cargo.
        assert_eq!(
            remapped_flags(
                Some("-C\x1ftarget-cpu=native".into()),
                Some("-D warnings".into()),
                &remaps
            ),
            format!("-C\x1ftarget-cpu=native\x1f{tail}")
        );
        // An empty encoded variable is no flags: RUSTFLAGS then counts.
        assert_eq!(
            remapped_flags(Some("".into()), Some("-D warnings".into()), &remaps),
            format!("-D\x1fwarnings\x1f{tail}")
        );
        assert_eq!(remapped_flags(None, None, &remaps), tail);
    }

    #[test]
    fn the_remaps_end_with_the_target_folder_and_start_with_the_workspace() {
        let remaps = remaps(Path::new("/w"), Path::new("/w/target"), "game");
        assert_eq!(remaps[0], (PathBuf::from("/w"), "game".to_owned()));
        assert_eq!(
            remaps.last().unwrap(),
            &(PathBuf::from("/w/target"), "/target".to_owned())
        );
    }
}
