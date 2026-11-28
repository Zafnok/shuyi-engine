//! `cargo xtask compile-messages`: what the compiler tells a game that
//! misses an impl of one of the engine's traits (ticket 2510).
//!
//! Each fixture in `crates/xtask/compile-messages/<name>.rs` is a small
//! program, shaped like a new game's code, that misses one impl. Its first
//! lines are `//~ <text>`: text the compiler's output for that file must
//! hold ([`expectations`]). The folder is not a cargo target, so the
//! workspace's builds never compile it.
//!
//! The command writes a package in `target/compile-messages/`
//! ([`manifest`]): its own empty `[workspace]`, path dependencies on the
//! engine's crates, the workspace's `Cargo.lock`, and each fixture as
//! `src/bin/<name>.rs`. It runs `cargo check --bins --keep-going` there
//! once, with none of the caller's `RUSTFLAGS` (CI's `-D warnings` would
//! make a fixture's warnings errors), splits cargo's stderr into
//! diagnostics by the file of their first ` --> ` line ([`split`]), and
//! fails a fixture with no error, or whose diagnostics lack one of its
//! texts ([`verdict`]). A text is matched as a substring, never the whole
//! output, so a new rustc that rewords its own lines keeps the gate green.
//! It reads the human format: the short one drops the label and the note.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The fixtures, from the repository's root.
const FIXTURES: &str = "crates/xtask/compile-messages";
/// The package the fixtures are checked in, from the repository's root.
const PACKAGE: &str = "target/compile-messages";
/// What starts a fixture's line of expected text.
const MARK: &str = "//~";
/// The variables that would replace the package's `rustflags`.
const RUSTFLAGS: &[&str] = &[
    "RUSTFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_BUILD_RUSTFLAGS",
];

/// The texts a fixture's output must hold: its first lines that start
/// with `//~`, each without the mark and the spaces around the text.
pub fn expectations(source: &str) -> Vec<String> {
    source
        .lines()
        .map_while(|line| line.trim_start().strip_prefix(MARK))
        .map(|text| text.trim().to_owned())
        .collect()
}

/// The diagnostics in cargo's stderr, by the fixture whose file their first
/// ` --> ` line names (`src/bin/<name>.rs`, with `/` or `\`). A diagnostic
/// starts at a line that begins with `error` or `warning`; one with no
/// ` --> ` line (cargo's "could not compile"), or one in another file,
/// goes to no fixture.
pub fn split(stderr: &str) -> BTreeMap<String, Vec<String>> {
    let mut diagnostics: Vec<String> = Vec::new();
    for line in stderr.lines() {
        if line.starts_with("error") || line.starts_with("warning") {
            diagnostics.push(String::new());
        }
        if let Some(current) = diagnostics.last_mut() {
            current.push_str(line);
            current.push('\n');
        }
    }
    let mut by_fixture: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for diagnostic in diagnostics {
        if let Some(name) = fixture_of(&diagnostic) {
            by_fixture.entry(name).or_default().push(diagnostic);
        }
    }
    by_fixture
}

/// The fixture a diagnostic's first ` --> ` line names, if it names one.
fn fixture_of(diagnostic: &str) -> Option<String> {
    let place = diagnostic
        .lines()
        .find_map(|line| line.split_once(" --> ").map(|(_, place)| place))?;
    let file = place.trim().replace('\\', "/");
    let rest = file.strip_prefix("src/bin/")?;
    let (name, _) = rest.split_once(".rs")?;
    (!name.is_empty() && !name.contains('/')).then(|| name.to_owned())
}

/// What is wrong with fixture `name`, if anything: it must have an
/// `error` diagnostic, and its diagnostics together must hold each of
/// `expected`.
pub fn verdict(name: &str, expected: &[String], diagnostics: &[String]) -> Option<String> {
    if expected.is_empty() {
        return Some(format!(
            "{FIXTURES}/{name}.rs: no `{MARK} <text>` line at its top"
        ));
    }
    if !diagnostics.iter().any(|d| d.starts_with("error")) {
        return Some(format!(
            "{FIXTURES}/{name}.rs: compiled without an error; it must miss an impl"
        ));
    }
    let output = diagnostics.concat();
    let missing: Vec<&str> = expected
        .iter()
        .map(String::as_str)
        .filter(|text| !output.contains(text))
        .collect();
    if missing.is_empty() {
        return None;
    }
    let mut message = format!("{FIXTURES}/{name}.rs: the compiler's output lacks");
    for text in missing {
        let _ = write!(message, "\n  {text:?}");
    }
    message.push_str("\nits output:\n");
    message.push_str(&output);
    Some(message)
}

/// The package's `Cargo.toml`, its path dependencies at `root`'s crates.
pub fn manifest(root: &Path) -> String {
    let crate_path = |name: &str| {
        root.join("crates")
            .join(name)
            .to_string_lossy()
            .replace('\\', "/")
    };
    let mut text = String::from(
        "# Written by `cargo xtask compile-messages` (ticket 2510): the fixtures of\n\
         # crates/xtask/compile-messages/, each a binary. Its own workspace, so the\n\
         # repository's doesn't claim it.\n\
         [package]\n\
         name = \"compile-messages\"\n\
         version = \"0.0.0\"\n\
         edition = \"2024\"\n\
         publish = false\n\n\
         [workspace]\n\n\
         [dependencies]\n\
         serde = { version = \"1\", features = [\"derive\"] }\n",
    );
    for (package, folder) in [
        ("shuyi", "shuyi"),
        ("shuyi-core", "core"),
        ("shuyi-assets", "assets"),
        ("shuyi-ui", "ui"),
        ("shuyi-platform", "platform"),
        ("shuyi-cli", "cli"),
    ] {
        let _ = writeln!(text, "{package} = {{ path = \"{}\" }}", crate_path(folder));
    }
    text
}

/// The fixtures under `root`, by name, each with its source.
fn fixtures(root: &Path) -> Result<BTreeMap<String, String>, String> {
    let dir = root.join(FIXTURES);
    let entries = fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut found = BTreeMap::new();
    for entry in entries {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        if path.extension().is_some_and(|ext| ext == "rs")
            && let Some(name) = path.file_stem().and_then(|stem| stem.to_str())
        {
            let source =
                fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            found.insert(name.to_owned(), source);
        }
    }
    if found.is_empty() {
        return Err(format!("{FIXTURES}: no fixture"));
    }
    Ok(found)
}

/// Writes the package in `dir`: its manifest, the workspace's lock and
/// each fixture as a binary, and no binary of an earlier run.
fn write_package(
    root: &Path,
    dir: &Path,
    fixtures: &BTreeMap<String, String>,
) -> Result<(), String> {
    let bin = dir.join("src/bin");
    if bin.exists() {
        fs::remove_dir_all(&bin).map_err(|e| format!("{}: {e}", bin.display()))?;
    }
    fs::create_dir_all(&bin).map_err(|e| format!("{}: {e}", bin.display()))?;
    let write = |path: PathBuf, text: &str| {
        fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
    };
    write(dir.join("Cargo.toml"), &manifest(root))?;
    let lock =
        fs::read_to_string(root.join("Cargo.lock")).map_err(|e| format!("Cargo.lock: {e}"))?;
    write(dir.join("Cargo.lock"), &lock)?;
    for (name, source) in fixtures {
        write(bin.join(format!("{name}.rs")), source)?;
    }
    Ok(())
}

/// `compile-messages`: every fixture's verdict, as errors.
pub fn run(root: &Path) -> Vec<String> {
    let fixtures = match fixtures(root) {
        Ok(fixtures) => fixtures,
        Err(e) => return vec![e],
    };
    let dir = root.join(PACKAGE);
    if let Err(e) = write_package(root, &dir, &fixtures) {
        return vec![e];
    }
    let mut command = Command::new("cargo");
    command
        .args(["check", "--bins", "--keep-going", "--color", "never"])
        .current_dir(&dir)
        .env("CARGO_TARGET_DIR", dir.join("target"));
    for name in RUSTFLAGS {
        command.env_remove(name);
    }
    eprintln!(
        "compile-messages: checking {} fixtures in {PACKAGE}",
        fixtures.len()
    );
    let output = match command.output() {
        Ok(output) => output,
        Err(e) => return vec![format!("`cargo check` did not run ({e})")],
    };
    let stderr = String::from_utf8_lossy(&output.stderr);
    let by_fixture = split(&stderr);
    if by_fixture.is_empty() && !output.status.success() {
        // It failed before any fixture: the engine, or the lock.
        return vec![format!("`cargo check` failed in {PACKAGE}:\n{stderr}")];
    }
    let errors: Vec<String> = fixtures
        .iter()
        .filter_map(|(name, source)| {
            let diagnostics = by_fixture.get(name).map_or(&[][..], Vec::as_slice);
            verdict(name, &expectations(source), diagnostics)
        })
        .collect();
    if errors.is_empty() {
        eprintln!(
            "compile-messages: {} fixtures, each told what its game misses",
            fixtures.len()
        );
    }
    errors
}

#[cfg(test)]
mod tests;
