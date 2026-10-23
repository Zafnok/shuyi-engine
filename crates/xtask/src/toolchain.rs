//! `cargo xtask check-toolchain`: the pinned Rust is the same everywhere
//! (ticket 2012, the `bump-rust` skill).
//!
//! It reads the root `rust-toolchain.toml` and `Cargo.toml`, every
//! `rust-toolchain.toml` and `Cargo.toml` under `templates/`, the members'
//! `crates/*/Cargo.toml` and `examples/*/Cargo.toml`, and
//! `.github/dependabot.yml`, and fails when:
//!
//! 1. the root's `channel` is not an exact stable release `X.Y.Z`;
//! 2. a template's `channel` differs from the root's;
//! 3. a workspace's `rust-version` (the root's, or a template's with
//!    `[workspace.package]`) is not the pin's `X.Y`;
//! 4. a `[package]` doesn't take `rust-version.workspace = true`;
//! 5. Dependabot's `rust-toolchain` update leaves out the folder of a
//!    `rust-toolchain.toml`.
//!
//! The TOML files are read line by line (a `[table]` header, then
//! `key = value` lines), which is all these files use. A missing or
//! unreadable file is an error, never a pass.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::links::{SKIPPED_DIRS, display_path};

const TOOLCHAIN: &str = "rust-toolchain.toml";
const MANIFEST: &str = "Cargo.toml";
const TEMPLATES: &str = "templates";
const DEPENDABOT: &str = ".github/dependabot.yml";
/// The folders whose crates are the workspace's members.
const MEMBER_DIRS: &[&str] = &["crates", "examples"];

/// Checks the files under `repo_root` and returns every disagreement (empty
/// means they agree).
pub fn run(repo_root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    let pin = root_pin(repo_root, &mut errors);

    let mut toolchains = Vec::new();
    let mut manifests = Vec::new();
    if let Err(e) = template_files(&repo_root.join(TEMPLATES), &mut toolchains, &mut manifests) {
        errors.push(format!("{TEMPLATES}: {e}"));
    }
    toolchains.sort();
    manifests.sort();

    let template_manifests: Vec<(String, String)> = manifests
        .iter()
        .filter_map(|file| read(repo_root, file, &mut errors))
        .collect();
    if toolchains.is_empty() {
        errors.push(format!(
            "{TEMPLATES}: no `{TOOLCHAIN}` found; every template pins the engine's Rust"
        ));
    }
    if !template_manifests
        .iter()
        .any(|(_, text)| has_table(text, "workspace.package"))
    {
        errors.push(format!(
            "{TEMPLATES}: no `{MANIFEST}` with `[workspace.package]` found; every template is a workspace"
        ));
    }

    if let Some(pin) = &pin {
        for file in &toolchains {
            if let Some((name, text)) = read(repo_root, file, &mut errors) {
                errors.extend(template_channel(&name, &text, pin));
            }
        }
        if let Some((name, text)) = read(repo_root, &repo_root.join(MANIFEST), &mut errors) {
            errors.extend(workspace_rust_version(&name, &text, pin, true));
        }
        for (name, text) in &template_manifests {
            errors.extend(workspace_rust_version(name, text, pin, false));
        }
    }

    for (name, text) in member_manifests(repo_root, &mut errors)
        .iter()
        .chain(&template_manifests)
    {
        errors.extend(package_rust_version(name, text));
    }

    let mut folders = vec!["/".to_owned()];
    folders.extend(toolchains.iter().map(|file| {
        let dir = file.parent().unwrap_or(repo_root);
        format!("/{}", display_path(repo_root, dir))
    }));
    match fs::read_to_string(repo_root.join(DEPENDABOT)) {
        Ok(text) => errors.extend(dependabot(&text, &folders)),
        Err(e) => errors.push(format!("{DEPENDABOT}: {e}")),
    }
    errors
}

/// The root's pin, or `None` (with the error) when it isn't an exact stable
/// release.
fn root_pin(repo_root: &Path, errors: &mut Vec<String>) -> Option<String> {
    let (name, text) = read(repo_root, &repo_root.join(TOOLCHAIN), errors)?;
    let Some(channel) = channel_of(&text) else {
        errors.push(format!(
            "{name}: no `channel` under `[toolchain]`; pin an exact stable release, such as `channel = \"1.98.1\"`"
        ));
        return None;
    };
    if major_minor(&channel).is_none() {
        errors.push(format!(
            "{name}: `channel = \"{channel}\"` is not an exact stable release `X.Y.Z`; pin one, such as `\"1.98.1\"`, so every build uses the same Rust"
        ));
        return None;
    }
    Some(channel)
}

/// Rule 2: a template's channel is the root's.
fn template_channel(name: &str, text: &str, pin: &str) -> Option<String> {
    match channel_of(text) {
        Some(channel) if channel == pin => None,
        Some(channel) => Some(format!(
            "{name}: `channel = \"{channel}\"` differs from the root's `{pin}`; set it to `\"{pin}\"`"
        )),
        None => Some(format!(
            "{name}: no `channel` under `[toolchain]`; set it to `\"{pin}\"`, the root's"
        )),
    }
}

/// Rule 3: `rust-version` in `[workspace.package]` is the pin's `X.Y`. The
/// root must have the table; a template's manifest without it is a member.
fn workspace_rust_version(name: &str, text: &str, pin: &str, required: bool) -> Option<String> {
    let (pin_major, pin_minor) = major_minor(pin)?;
    let want = format!("{pin_major}.{pin_minor}");
    if !has_table(text, "workspace.package") {
        return required.then(|| {
            format!("{name}: no `[workspace.package]`; add one with `rust-version = \"{want}\"`")
        });
    }
    let Some(have) = value_of(text, "workspace.package", "rust-version") else {
        return Some(format!(
            "{name}: no `rust-version` in `[workspace.package]`; set it to `\"{want}\"`, the pin's X.Y"
        ));
    };
    let why = match version_of(&have) {
        Some(v) if v == (pin_major, pin_minor) => return None,
        Some(v) if v < (pin_major, pin_minor) => format!(
            "is below the pin's `{want}`: nothing in CI builds on an older Rust, so a lower `rust-version` needs the `msrv` job first (the `bump-rust` skill)"
        ),
        Some(_) => {
            format!("is above the pin's `{want}`: cargo would refuse to build on the pinned Rust")
        }
        None => "is not a Rust version `X.Y`".to_owned(),
    };
    Some(format!(
        "{name}: `rust-version = \"{have}\"` in `[workspace.package]` {why}; set it to `\"{want}\"`"
    ))
}

/// Rule 4: a `[package]` takes the workspace's `rust-version`.
fn package_rust_version(name: &str, text: &str) -> Option<String> {
    let inherits = value_of(text, "package", "rust-version.workspace").as_deref() == Some("true");
    (has_table(text, "package") && !inherits).then(|| {
        format!(
            "{name}: `[package]` has no `rust-version.workspace = true`; write it so the crate takes the workspace's `rust-version`"
        )
    })
}

/// The update Dependabot reads for the pinned Rust.
#[derive(Deserialize)]
struct Dependabot {
    #[serde(default)]
    updates: Vec<Update>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Update {
    package_ecosystem: String,
    directory: Option<String>,
    #[serde(default)]
    directories: Vec<String>,
}

/// Rule 5: Dependabot's `rust-toolchain` updates watch every folder in
/// `folders` (`/` and each template's, from the root).
fn dependabot(text: &str, folders: &[String]) -> Vec<String> {
    let config: Dependabot = match serde_norway::from_str(text) {
        Ok(config) => config,
        Err(e) => return vec![format!("{DEPENDABOT}: {e}")],
    };
    let watched: Vec<String> = config
        .updates
        .into_iter()
        .filter(|u| u.package_ecosystem == "rust-toolchain")
        .flat_map(|u| u.directory.into_iter().chain(u.directories))
        .map(|dir| normalize_dir(&dir))
        .collect();
    if watched.is_empty() {
        return vec![format!(
            "{DEPENDABOT}: no update with `package-ecosystem: rust-toolchain` and a directory; add one (the `bump-rust` skill)"
        )];
    }
    folders
        .iter()
        .filter(|folder| !watched.contains(folder))
        .map(|folder| {
            let file = if folder == "/" {
                TOOLCHAIN.to_owned()
            } else {
                format!("{}/{TOOLCHAIN}", &folder[1..])
            };
            format!(
                "{DEPENDABOT}: the `rust-toolchain` update's `directories` leaves out `{folder}`, the folder of `{file}`; add it"
            )
        })
        .collect()
}

/// `dir` with one leading `/` and no trailing one (`/` stays `/`).
fn normalize_dir(dir: &str) -> String {
    format!("/{}", dir.trim_matches('/'))
}

/// Every `rust-toolchain.toml` and `Cargo.toml` under `dir`, at any depth,
/// skipping build folders.
fn template_files(
    dir: &Path,
    toolchains: &mut Vec<PathBuf>,
    manifests: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            if !SKIPPED_DIRS.iter().any(|d| entry.file_name() == *d) {
                template_files(&path, toolchains, manifests)?;
            }
        } else if entry.file_name() == TOOLCHAIN {
            toolchains.push(path);
        } else if entry.file_name() == MANIFEST {
            manifests.push(path);
        }
    }
    Ok(())
}

/// The `Cargo.toml` of every folder in `crates/` and `examples/`, read.
fn member_manifests(repo_root: &Path, errors: &mut Vec<String>) -> Vec<(String, String)> {
    let mut files = Vec::new();
    for dir in MEMBER_DIRS {
        match fs::read_dir(repo_root.join(dir)) {
            Ok(entries) => files.extend(
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path().join(MANIFEST))
                    .filter(|file| file.is_file()),
            ),
            Err(e) => errors.push(format!("{dir}: {e}")),
        }
    }
    files.sort();
    files
        .iter()
        .filter_map(|file| read(repo_root, file, errors))
        .collect()
}

/// `file`'s name from the root and its text, or `None` with the error.
fn read(repo_root: &Path, file: &Path, errors: &mut Vec<String>) -> Option<(String, String)> {
    let name = display_path(repo_root, file);
    match fs::read_to_string(file) {
        Ok(text) => Some((name, text)),
        Err(e) => {
            errors.push(format!("{name}: {e}"));
            None
        }
    }
}

/// The `channel` under `[toolchain]`.
fn channel_of(text: &str) -> Option<String> {
    value_of(text, "toolchain", "channel")
}

/// `X.Y` of an exact stable release `X.Y.Z`, or `None` for anything else
/// (`stable`, `1.98`, `nightly-…`).
fn major_minor(channel: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = channel.split('.').collect();
    let [major, minor, patch] = parts.as_slice() else {
        return None;
    };
    patch.parse::<u32>().ok()?;
    Some((number(major)?, number(minor)?))
}

/// A `rust-version` `X.Y` as numbers.
fn version_of(version: &str) -> Option<(u32, u32)> {
    let (major, minor) = version.split_once('.')?;
    Some((number(major)?, number(minor)?))
}

/// Only digits: `u32::from_str` takes a leading `+` too.
fn number(text: &str) -> Option<u32> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// Whether `text` has the table `[table]`.
fn has_table(text: &str, table: &str) -> bool {
    text.lines().any(|line| header_of(line) == Some(table))
}

/// The value of `key` in `[table]`: a quoted string without its quotes, or
/// the bare value (`true`), without a trailing comment.
fn value_of(text: &str, table: &str, key: &str) -> Option<String> {
    let mut current = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(header) = header_of(line) {
            current = Some(header);
            continue;
        }
        if current != Some(table) {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        if k.split('.').map(str::trim).eq(key.split('.')) {
            return Some(bare_value(v.trim()));
        }
    }
    None
}

/// The name of a `[table]` or `[[table]]` header line.
fn header_of(line: &str) -> Option<&str> {
    let line = line.trim();
    let inner = line.strip_prefix('[')?;
    let end = inner.find(']')?;
    Some(inner[..end].trim_matches(['[', ' ']))
}

/// A value without its quotes or trailing comment.
fn bare_value(value: &str) -> String {
    if let Some(rest) = value.strip_prefix('"') {
        return rest.split('"').next().unwrap_or_default().to_owned();
    }
    value
        .split('#')
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT_TOOLCHAIN: &str = "[toolchain]\nchannel = \"1.98.1\"\ncomponents = [\"rustfmt\", \"clippy\"]\ntargets = [\"wasm32-unknown-unknown\"]\n";
    const TEMPLATE_TOOLCHAIN: &str =
        "# The Rust the engine builds with.\n[toolchain]\nchannel = \"1.98.1\"\n";
    const WORKSPACE: &str = "[workspace]\nmembers = [\"crates/*\"]\n\n[workspace.package]\nversion = \"0.1.0\"\nrust-version = \"1.98\"\n";
    const MEMBER: &str = "[package]\nname = \"a\"\nversion.workspace = true\nrust-version.workspace = true\n\n[dependencies]\nserde.workspace = true\n";
    const DEPENDABOT_OK: &str = "version: 2\nupdates:\n  - package-ecosystem: cargo\n    directory: /\n  - package-ecosystem: rust-toolchain\n    directories: [\"/\", \"/templates/blank\", \"/templates/grid\"]\n    schedule:\n      interval: weekly\n";

    /// A repository where every file agrees, in a fresh temporary folder.
    struct Fixture(PathBuf);

    impl Fixture {
        fn new(test: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("xtask-toolchain-{}-{test}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            let fixture = Fixture(root);
            fixture.write(TOOLCHAIN, ROOT_TOOLCHAIN);
            fixture.write(MANIFEST, WORKSPACE);
            fixture.write("crates/core/Cargo.toml", MEMBER);
            fixture.write("examples/hello/Cargo.toml", MEMBER);
            for template in ["blank", "grid"] {
                fixture.write(
                    &format!("templates/{template}/{TOOLCHAIN}"),
                    TEMPLATE_TOOLCHAIN,
                );
                fixture.write(&format!("templates/{template}/{MANIFEST}"), WORKSPACE);
                fixture.write(
                    &format!("templates/{template}/crates/game/{MANIFEST}"),
                    MEMBER,
                );
            }
            fixture.write(DEPENDABOT, DEPENDABOT_OK);
            fixture
        }

        fn write(&self, file: &str, text: &str) {
            let path = self.0.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }

        fn remove(&self, file: &str) {
            let path = self.0.join(file);
            if path.is_dir() {
                fs::remove_dir_all(path).unwrap();
            } else {
                fs::remove_file(path).unwrap();
            }
        }

        fn run(&self) -> Vec<String> {
            run(&self.0)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn files_that_agree_pass() {
        assert_eq!(Fixture::new("agree").run(), Vec::<String>::new());
    }

    #[test]
    fn the_real_repository_passes() {
        assert_eq!(run(&crate::repo_root()), Vec::<String>::new());
    }

    #[test]
    fn a_root_channel_that_is_not_exact_fails() {
        for channel in ["stable", "1.98", "nightly-2026-10-01"] {
            let fixture = Fixture::new(&format!("channel-{}", channel.replace('.', "_")));
            fixture.write(
                TOOLCHAIN,
                &format!("[toolchain]\nchannel = \"{channel}\"\n"),
            );
            assert_eq!(
                fixture.run(),
                [format!(
                    "rust-toolchain.toml: `channel = \"{channel}\"` is not an exact stable release `X.Y.Z`; pin one, such as `\"1.98.1\"`, so every build uses the same Rust"
                )]
            );
        }
    }

    #[test]
    fn a_root_without_a_channel_fails() {
        let fixture = Fixture::new("no-channel");
        fixture.write(TOOLCHAIN, "[other]\nchannel = \"1.98.1\"\n[toolchain]\n");
        assert_eq!(
            fixture.run(),
            [
                "rust-toolchain.toml: no `channel` under `[toolchain]`; pin an exact stable release, such as `channel = \"1.98.1\"`"
            ]
        );
    }

    #[test]
    fn a_template_channel_that_differs_fails_naming_that_file_only() {
        let fixture = Fixture::new("template-channel");
        fixture.write(
            "templates/grid/rust-toolchain.toml",
            "[toolchain]\nchannel = \"1.97.0\"\n",
        );
        assert_eq!(
            fixture.run(),
            [
                "templates/grid/rust-toolchain.toml: `channel = \"1.97.0\"` differs from the root's `1.98.1`; set it to `\"1.98.1\"`"
            ]
        );
    }

    #[test]
    fn a_template_without_a_channel_fails() {
        let fixture = Fixture::new("template-no-channel");
        fixture.write("templates/blank/rust-toolchain.toml", "[toolchain]\n");
        assert_eq!(
            fixture.run(),
            [
                "templates/blank/rust-toolchain.toml: no `channel` under `[toolchain]`; set it to `\"1.98.1\"`, the root's"
            ]
        );
    }

    #[test]
    fn a_template_rust_version_below_the_pin_fails() {
        let fixture = Fixture::new("template-below");
        fixture.write(
            "templates/blank/Cargo.toml",
            &WORKSPACE.replace("1.98", "1.97"),
        );
        assert_eq!(
            fixture.run(),
            [
                "templates/blank/Cargo.toml: `rust-version = \"1.97\"` in `[workspace.package]` is below the pin's `1.98`: nothing in CI builds on an older Rust, so a lower `rust-version` needs the `msrv` job first (the `bump-rust` skill); set it to `\"1.98\"`"
            ]
        );
    }

    #[test]
    fn the_root_rust_version_below_the_pin_names_the_msrv_job() {
        let fixture = Fixture::new("root-below");
        fixture.write(MANIFEST, &WORKSPACE.replace("1.98", "1.97"));
        assert_eq!(
            fixture.run(),
            [
                "Cargo.toml: `rust-version = \"1.97\"` in `[workspace.package]` is below the pin's `1.98`: nothing in CI builds on an older Rust, so a lower `rust-version` needs the `msrv` job first (the `bump-rust` skill); set it to `\"1.98\"`"
            ]
        );
    }

    #[test]
    fn a_rust_version_above_the_pin_fails() {
        let fixture = Fixture::new("root-above");
        fixture.write(MANIFEST, &WORKSPACE.replace("1.98", "1.99"));
        assert_eq!(
            fixture.run(),
            [
                "Cargo.toml: `rust-version = \"1.99\"` in `[workspace.package]` is above the pin's `1.98`: cargo would refuse to build on the pinned Rust; set it to `\"1.98\"`"
            ]
        );
    }

    #[test]
    fn a_rust_version_that_is_not_x_y_fails() {
        let fixture = Fixture::new("root-bad-version");
        fixture.write(MANIFEST, &WORKSPACE.replace("1.98", "1.98.1"));
        assert_eq!(
            fixture.run(),
            [
                "Cargo.toml: `rust-version = \"1.98.1\"` in `[workspace.package]` is not a Rust version `X.Y`; set it to `\"1.98\"`"
            ]
        );
    }

    #[test]
    fn a_root_without_a_rust_version_or_workspace_package_fails() {
        let fixture = Fixture::new("root-no-version");
        fixture.write(MANIFEST, "[workspace.package]\nversion = \"0.1.0\"\n");
        assert_eq!(
            fixture.run(),
            [
                "Cargo.toml: no `rust-version` in `[workspace.package]`; set it to `\"1.98\"`, the pin's X.Y"
            ]
        );
        fixture.write(MANIFEST, "[workspace]\nmembers = []\n");
        assert_eq!(
            fixture.run(),
            ["Cargo.toml: no `[workspace.package]`; add one with `rust-version = \"1.98\"`"]
        );
    }

    #[test]
    fn a_member_with_its_own_rust_version_fails() {
        let fixture = Fixture::new("member");
        let written_out =
            MEMBER.replace("rust-version.workspace = true", "rust-version = \"1.98\"");
        fixture.write("crates/core/Cargo.toml", &written_out);
        fixture.write("templates/grid/crates/game/Cargo.toml", &written_out);
        assert_eq!(
            fixture.run(),
            [
                "crates/core/Cargo.toml: `[package]` has no `rust-version.workspace = true`; write it so the crate takes the workspace's `rust-version`",
                "templates/grid/crates/game/Cargo.toml: `[package]` has no `rust-version.workspace = true`; write it so the crate takes the workspace's `rust-version`",
            ]
        );
    }

    #[test]
    fn dependabot_without_the_update_fails() {
        let fixture = Fixture::new("dependabot-none");
        fixture.write(
            DEPENDABOT,
            "version: 2\nupdates:\n  - package-ecosystem: cargo\n    directory: /\n",
        );
        assert_eq!(
            fixture.run(),
            [
                ".github/dependabot.yml: no update with `package-ecosystem: rust-toolchain` and a directory; add one (the `bump-rust` skill)"
            ]
        );
    }

    #[test]
    fn dependabot_leaving_out_a_folder_fails() {
        let fixture = Fixture::new("dependabot-folder");
        fixture.write(
            DEPENDABOT,
            "version: 2\nupdates:\n  - package-ecosystem: rust-toolchain\n    directories: [\"/\", \"/templates/blank/\"]\n",
        );
        assert_eq!(
            fixture.run(),
            [
                ".github/dependabot.yml: the `rust-toolchain` update's `directories` leaves out `/templates/grid`, the folder of `templates/grid/rust-toolchain.toml`; add it"
            ]
        );
        fixture.write(
            DEPENDABOT,
            "version: 2\nupdates:\n  - package-ecosystem: rust-toolchain\n    directory: /templates/blank\n  - package-ecosystem: rust-toolchain\n    directory: /templates/grid\n",
        );
        assert_eq!(
            fixture.run(),
            [
                ".github/dependabot.yml: the `rust-toolchain` update's `directories` leaves out `/`, the folder of `rust-toolchain.toml`; add it"
            ]
        );
    }

    #[test]
    fn a_template_one_folder_deeper_is_found() {
        let fixture = Fixture::new("deeper");
        fixture.write("templates/one/base/rust-toolchain.toml", TEMPLATE_TOOLCHAIN);
        assert_eq!(
            fixture.run(),
            [
                ".github/dependabot.yml: the `rust-toolchain` update's `directories` leaves out `/templates/one/base`, the folder of `templates/one/base/rust-toolchain.toml`; add it"
            ]
        );
    }

    #[test]
    fn a_target_folder_is_skipped() {
        let fixture = Fixture::new("target");
        fixture.write(
            "templates/blank/target/debug/rust-toolchain.toml",
            "[toolchain]\nchannel = \"stable\"\n",
        );
        fixture.write(
            "templates/blank/target/debug/Cargo.toml",
            "[package]\nname = \"x\"\n",
        );
        assert_eq!(fixture.run(), Vec::<String>::new());
    }

    #[test]
    fn a_missing_root_toolchain_is_an_error() {
        let fixture = Fixture::new("no-root");
        fixture.remove(TOOLCHAIN);
        let errors = fixture.run();
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("rust-toolchain.toml: "), "{errors:?}");
    }

    #[test]
    fn templates_without_a_toolchain_or_a_workspace_are_errors() {
        let fixture = Fixture::new("empty-templates");
        fixture.remove(TEMPLATES);
        fixture.write("templates/README.md", "nothing here\n");
        assert_eq!(
            fixture.run(),
            [
                "templates: no `rust-toolchain.toml` found; every template pins the engine's Rust",
                "templates: no `Cargo.toml` with `[workspace.package]` found; every template is a workspace",
            ]
        );
    }

    #[test]
    fn a_missing_dependabot_file_is_an_error() {
        let fixture = Fixture::new("no-dependabot");
        fixture.remove(DEPENDABOT);
        let errors = fixture.run();
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].starts_with(".github/dependabot.yml: "),
            "{errors:?}"
        );
    }

    #[test]
    fn a_missing_members_folder_is_an_error() {
        let fixture = Fixture::new("no-examples");
        fixture.remove("examples");
        let errors = fixture.run();
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("examples: "), "{errors:?}");
    }

    #[test]
    fn the_line_reader_reads_only_its_table_and_skips_comments() {
        let text = "[other]\nchannel = \"other\"\n[toolchain]\n# channel = \"commented\"\n  channel   =   \"1.98.1\"  # the pin\n";
        assert_eq!(channel_of(text).as_deref(), Some("1.98.1"));
        assert_eq!(channel_of("[other]\nchannel = \"1.98.1\"\n"), None);
    }

    #[test]
    fn the_line_reader_reads_a_dotted_key_and_a_bare_value() {
        let text = "[package]\nrust-version . workspace = true # inherited\n";
        assert_eq!(
            value_of(text, "package", "rust-version.workspace").as_deref(),
            Some("true")
        );
        assert_eq!(value_of(text, "package", "rust-version"), None);
    }

    #[test]
    fn headers_are_read_with_their_dots_and_brackets() {
        assert_eq!(
            header_of("[workspace.package] # x"),
            Some("workspace.package")
        );
        assert_eq!(header_of("[[bin]]"), Some("bin"));
        assert_eq!(header_of("key = [1]"), None);
    }

    #[test]
    fn only_an_exact_release_has_a_major_minor() {
        assert_eq!(major_minor("1.98.1"), Some((1, 98)));
        assert_eq!(major_minor("1.98"), None);
        assert_eq!(major_minor("1.98.x"), None);
        assert_eq!(major_minor("+1.98.1"), None);
        assert_eq!(major_minor("1.98.1.0"), None);
    }

    #[test]
    fn folders_are_written_with_one_leading_slash() {
        assert_eq!(normalize_dir("/"), "/");
        assert_eq!(normalize_dir("templates/blank/"), "/templates/blank");
    }
}
