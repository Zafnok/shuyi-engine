//! `cargo xtask check-pins`: the actions and the templates follow the root
//! (ticket 2011, ADR-0037 §2).
//!
//! It fails on:
//!
//! 1. a `uses:` in a workflow file (a `*.yml` or `*.yaml` in any
//!    `.github/workflows` folder) that isn't `owner/repo[/path]@` a 40-hex
//!    commit SHA followed by a `# v…` comment; a local `./…` one is
//!    skipped;
//! 2. one action (`owner/repo`, whatever path follows) at more than one SHA
//!    anywhere in the repository;
//! 3. a `.github/workflows` folder other than the root's missing from the
//!    `github-actions` update's `directories` in `.github/dependabot.yml`;
//! 4. a crate that the root's `[workspace.dependencies]` and a template's
//!    both name, with a different version requirement in each. `shuyi-*`
//!    entries (placeholders) are skipped; an entry must keep to one line.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::links::{display_path, is_skipped_dir};

/// Dependabot's configuration, from the repository's root.
pub const DEPENDABOT: &str = ".github/dependabot.yml";

/// Checks the repository at `root` and returns every error (empty means
/// clean).
pub fn run(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    let mut errors = Vec::new();
    if let Err(e) = all_files(root, root, &mut files) {
        errors.push(format!("{}: {e}", root.display()));
    }
    files.sort();
    let mut workflows = Vec::new();
    let mut templates = Vec::new();
    for file in &files {
        let display = display_path(root, file);
        if is_workflow(&display) {
            workflows.push((display, file));
        } else if display.starts_with("templates/") && display.ends_with("/Cargo.toml") {
            templates.push((display, file));
        }
    }

    let mut texts = Vec::new();
    for (display, file) in workflows {
        match fs::read_to_string(file) {
            Ok(text) => texts.push((display, text)),
            Err(e) => errors.push(format!("{display}: {e}")),
        }
    }
    errors.extend(check_workflows(&texts));

    let folders = workflow_folders(texts.iter().map(|(display, _)| display.as_str()));
    if !folders.is_empty() {
        match fs::read_to_string(root.join(DEPENDABOT)) {
            Ok(text) => errors.extend(check_dependabot(&text, &folders)),
            Err(e) => errors.push(format!("{DEPENDABOT}: {e}")),
        }
    }

    match fs::read_to_string(root.join("Cargo.toml")) {
        Ok(root_text) => {
            for (display, file) in templates {
                match fs::read_to_string(file) {
                    Ok(text) => errors.extend(check_template(&root_text, &display, &text)),
                    Err(e) => errors.push(format!("{display}: {e}")),
                }
            }
        }
        Err(e) => errors.push(format!("Cargo.toml: {e}")),
    }
    errors
}

/// Every file under `dir`, but in the folders `check-links` skips too.
fn all_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            if !is_skipped_dir(root, &path) {
                all_files(root, &path, out)?;
            }
        } else {
            out.push(path);
        }
    }
    Ok(())
}

/// Whether `display`, a path from the root, is a workflow file.
fn is_workflow(display: &str) -> bool {
    let Some((folder, name)) = display.rsplit_once('/') else {
        return false;
    };
    let in_workflows = folder == ".github/workflows" || folder.ends_with("/.github/workflows");
    in_workflows
        && name
            .rsplit_once('.')
            .is_some_and(|(_, ext)| ext == "yml" || ext == "yaml")
}

/// The workflow folders other than the root's, as Dependabot names them
/// (`/templates/grid/.github/workflows`), once each and sorted.
fn workflow_folders<'a>(workflows: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut folders: Vec<String> = workflows
        .filter_map(|display| display.rsplit_once('/').map(|(folder, _)| folder))
        .filter(|folder| *folder != ".github/workflows")
        .map(|folder| format!("/{folder}"))
        .collect();
    folders.sort();
    folders.dedup();
    folders
}

/// The `uses:` lines of a workflow, with their line numbers and the value
/// after `uses:`. A line counts when its text, after spaces and an optional
/// `- `, starts with `uses:`.
fn uses_lines(text: &str) -> Vec<(usize, &str)> {
    text.lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let line = line.trim_start();
            let line = line.strip_prefix("- ").map_or(line, str::trim_start);
            line.strip_prefix("uses:").map(|rest| (i + 1, rest.trim()))
        })
        .collect()
}

/// A pinned `uses:` value: its action (`owner/repo`) and its SHA.
#[derive(Debug, PartialEq, Eq)]
struct Pin<'a> {
    action: &'a str,
    sha: &'a str,
}

/// Reads one `uses:` value (`None` for a local `./…` workflow), or says
/// why it isn't pinned.
fn parse_uses(value: &str) -> Result<Option<Pin<'_>>, String> {
    let (reference, comment) = match value.split_once('#') {
        Some((reference, comment)) => (reference.trim(), Some(comment.trim())),
        None => (value.trim(), None),
    };
    let reference = reference.trim_matches(|c| c == '"' || c == '\'');
    if reference.starts_with("./") {
        return Ok(None);
    }
    let Some((path, sha)) = reference.split_once('@') else {
        return Err(format!("`{reference}` names no version after `@`"));
    };
    let mut parts = path.split('/');
    let (Some(owner), Some(repo)) = (parts.next(), parts.next()) else {
        return Err(format!("`{reference}` is not `owner/repo@<sha>`"));
    };
    if owner.is_empty() || repo.is_empty() {
        return Err(format!("`{reference}` is not `owner/repo@<sha>`"));
    }
    if sha.len() != 40 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!(
            "`{reference}` is not pinned by a 40-hex commit SHA"
        ));
    }
    if !comment.is_some_and(|c| c.starts_with('v')) {
        return Err(format!(
            "`{reference}` has no `# v…` comment naming its version"
        ));
    }
    Ok(Some(Pin {
        action: &path[..owner.len() + 1 + repo.len()],
        sha,
    }))
}

/// Rules 1 and 2 over the workflow files, given as (path, text).
fn check_workflows(workflows: &[(String, String)]) -> Vec<String> {
    let mut errors = Vec::new();
    // action -> sha -> places
    let mut shas: BTreeMap<&str, BTreeMap<&str, Vec<String>>> = BTreeMap::new();
    for (display, text) in workflows {
        for (line, value) in uses_lines(text) {
            match parse_uses(value) {
                Ok(Some(pin)) => shas
                    .entry(pin.action)
                    .or_default()
                    .entry(pin.sha)
                    .or_default()
                    .push(format!("{display}:{line}")),
                Ok(None) => {}
                Err(why) => errors.push(format!("{display}:{line}: {why}")),
            }
        }
    }
    for (action, by_sha) in shas {
        if by_sha.len() > 1 {
            let places: Vec<String> = by_sha
                .iter()
                .flat_map(|(sha, places)| places.iter().map(move |p| format!("{p} ({sha})")))
                .collect();
            errors.push(format!(
                "`{action}` is pinned at {} SHAs; use one: {}",
                by_sha.len(),
                places.join(", ")
            ));
        }
    }
    errors
}

/// `.github/dependabot.yml`, as far as rule 3 reads it.
#[derive(Debug, Deserialize)]
struct Dependabot {
    #[serde(default)]
    updates: Vec<Update>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Update {
    package_ecosystem: String,
    directory: Option<String>,
    #[serde(default)]
    directories: Vec<String>,
}

/// Rule 3: every folder of `folders` is a directory of the `github-actions`
/// update in `text`.
fn check_dependabot(text: &str, folders: &[String]) -> Vec<String> {
    let config: Dependabot = match serde_norway::from_str(text) {
        Ok(config) => config,
        Err(e) => return vec![format!("{DEPENDABOT}: {e}")],
    };
    let listed: Vec<&str> = config
        .updates
        .iter()
        .filter(|u| u.package_ecosystem == "github-actions")
        .flat_map(|u| u.directory.iter().chain(&u.directories))
        .map(|d| d.trim_end_matches('/'))
        .collect();
    folders
        .iter()
        .filter(|folder| !listed.contains(&folder.as_str()))
        .map(|folder| {
            format!(
                "{DEPENDABOT}: the github-actions update's `directories` doesn't list \
                 `{folder}`, so Dependabot never moves its pins"
            )
        })
        .collect()
}

/// One entry of a `[workspace.dependencies]` table: its crate, its version
/// requirement (`None` for a path or git one) and its line.
#[derive(Debug, PartialEq, Eq)]
struct Requirement<'a> {
    name: &'a str,
    version: Option<&'a str>,
    line: usize,
}

/// The `[workspace.dependencies]` entries of a `Cargo.toml`, read line by
/// line, and the errors for entries that aren't on one line.
fn workspace_requirements<'a>(display: &str, text: &'a str) -> (Vec<Requirement<'a>>, Vec<String>) {
    let mut found = Vec::new();
    let mut errors = Vec::new();
    let mut in_table = false;
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_table = line == "[workspace.dependencies]";
            continue;
        }
        if !in_table || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            errors.push(format!(
                "{display}:{}: an entry must keep to one line",
                i + 1
            ));
            continue;
        };
        let (name, value) = (name.trim(), value.trim());
        let version = if let Some(rest) = value.strip_prefix('"') {
            rest.split_once('"').map(|(version, _)| Some(version))
        } else if value.starts_with('{') && value.ends_with('}') {
            Some(inline_version(value))
        } else {
            None
        };
        match version {
            Some(version) => found.push(Requirement {
                name,
                version,
                line: i + 1,
            }),
            None => errors.push(format!(
                "{display}:{}: `{name}` must keep to one line, as a string or an inline table",
                i + 1
            )),
        }
    }
    (found, errors)
}

/// The `version = "…"` of an inline table, if it has one.
fn inline_version(table: &str) -> Option<&str> {
    let inner = table.strip_prefix('{')?.strip_suffix('}')?;
    inner.split(',').find_map(|field| {
        let (key, value) = field.split_once('=')?;
        if key.trim() != "version" {
            return None;
        }
        value.trim().strip_prefix('"')?.strip_suffix('"')
    })
}

/// Rule 4: the template at `display` asks for every crate the root names
/// with the root's requirement.
fn check_template(root_text: &str, display: &str, text: &str) -> Vec<String> {
    let (root, mut errors) = workspace_requirements("Cargo.toml", root_text);
    let (template, template_errors) = workspace_requirements(display, text);
    errors.extend(template_errors);
    for entry in template {
        if entry.name.starts_with("shuyi-") {
            continue;
        }
        let Some(theirs) = root.iter().find(|r| r.name == entry.name) else {
            continue;
        };
        if theirs.version != entry.version {
            errors.push(format!(
                "{display}:{}: `{}` requires {}, but the root's Cargo.toml:{} requires {}",
                entry.line,
                entry.name,
                shown(entry.version),
                theirs.line,
                shown(theirs.version)
            ));
        }
    }
    errors
}

fn shown(version: Option<&str>) -> String {
    version.map_or_else(|| "no version".to_string(), |v| format!("\"{v}\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "3d3c42e5aac5ba805825da76410c181273ba90b1";
    const OTHER: &str = "6323deb102c322ba6fcbdcafc7e3dddab59af2b6";

    fn workflow(path: &str, text: &str) -> (String, String) {
        (path.to_string(), text.to_string())
    }

    #[test]
    fn reads_uses_lines_and_not_comments_or_scripts() {
        let text = format!(
            "jobs:\n  a:\n    steps:\n      - uses: actions/checkout@{SHA} # v7.0.1\n      \
             # uses: not/this@v1\n      - run: echo 'uses: nor/this@v1'\n        \
             uses: ./.github/workflows/changes.yml\n      -   uses:   x/y@main\n"
        );
        assert_eq!(
            uses_lines(&text),
            vec![
                (4, format!("actions/checkout@{SHA} # v7.0.1").as_str()),
                (7, "./.github/workflows/changes.yml"),
                (8, "x/y@main"),
            ]
        );
    }

    #[test]
    fn a_pinned_uses_is_read() {
        assert_eq!(
            parse_uses(&format!("actions/checkout@{SHA} # v7.0.1")),
            Ok(Some(Pin {
                action: "actions/checkout",
                sha: SHA
            }))
        );
        assert_eq!(
            parse_uses(&format!("\"github/codeql-action/init@{SHA}\" # v4.39.0")),
            Ok(Some(Pin {
                action: "github/codeql-action",
                sha: SHA
            }))
        );
    }

    #[test]
    fn a_local_workflow_is_skipped() {
        assert_eq!(parse_uses("./.github/workflows/changes.yml"), Ok(None));
    }

    #[test]
    fn a_tag_a_short_sha_and_a_missing_comment_are_reported() {
        let not_pinned = "is not pinned by a 40-hex commit SHA";
        assert_eq!(
            parse_uses("actions/checkout@v7 # v7"),
            Err(format!("`actions/checkout@v7` {not_pinned}"))
        );
        assert_eq!(
            parse_uses("actions/checkout@3d3c42e # v7.0.1"),
            Err(format!("`actions/checkout@3d3c42e` {not_pinned}"))
        );
        // 40 characters, not all hex.
        let not_hex = format!("actions/checkout@{}", "g".repeat(40));
        assert_eq!(
            parse_uses(&format!("{not_hex} # v1")),
            Err(format!("`{not_hex}` {not_pinned}"))
        );
        let missing = "has no `# v…` comment naming its version";
        assert_eq!(
            parse_uses(&format!("actions/checkout@{SHA}")),
            Err(format!("`actions/checkout@{SHA}` {missing}"))
        );
        assert_eq!(
            parse_uses(&format!("actions/checkout@{SHA} # main")),
            Err(format!("`actions/checkout@{SHA}` {missing}"))
        );
    }

    #[test]
    fn a_uses_without_owner_repo_or_version_is_reported() {
        assert_eq!(
            parse_uses("actions/checkout"),
            Err("`actions/checkout` names no version after `@`".to_string())
        );
        let shape = "is not `owner/repo@<sha>`";
        assert_eq!(
            parse_uses(&format!("checkout@{SHA} # v1")),
            Err(format!("`checkout@{SHA}` {shape}"))
        );
        assert_eq!(
            parse_uses(&format!("/checkout@{SHA} # v1")),
            Err(format!("`/checkout@{SHA}` {shape}"))
        );
        assert_eq!(
            parse_uses(&format!("actions/@{SHA} # v1")),
            Err(format!("`actions/@{SHA}` {shape}"))
        );
    }

    #[test]
    fn an_unpinned_uses_is_reported_with_its_file_and_line() {
        let errors = check_workflows(&[workflow(
            ".github/workflows/ci.yml",
            "steps:\n  - uses: actions/checkout@v7\n",
        )]);
        assert_eq!(
            errors,
            vec![
                ".github/workflows/ci.yml:2: `actions/checkout@v7` is not pinned by a 40-hex \
                 commit SHA"
                    .to_string()
            ]
        );
    }

    #[test]
    fn two_shas_for_one_action_are_reported_with_both_places() {
        let errors = check_workflows(&[
            workflow(
                ".github/workflows/codeql.yml",
                &format!(
                    "- uses: github/codeql-action/init@{SHA} # v4\n\
                     - uses: github/codeql-action/analyze@{SHA} # v4\n"
                ),
            ),
            workflow(
                "templates/grid/.github/workflows/ci.yml",
                &format!("\n- uses: github/codeql-action/upload-sarif@{OTHER} # v4\n"),
            ),
        ]);
        assert_eq!(
            errors,
            vec![format!(
                "`github/codeql-action` is pinned at 2 SHAs; use one: \
                 .github/workflows/codeql.yml:1 ({SHA}), .github/workflows/codeql.yml:2 ({SHA}), \
                 templates/grid/.github/workflows/ci.yml:2 ({OTHER})"
            )]
        );
    }

    #[test]
    fn one_sha_per_action_passes() {
        let text = format!(
            "- uses: github/codeql-action/init@{SHA} # v4\n\
             - uses: github/codeql-action/analyze@{SHA} # v4\n\
             - uses: actions/checkout@{OTHER} # v7\n"
        );
        assert!(check_workflows(&[workflow(".github/workflows/a.yml", &text)]).is_empty());
    }

    #[test]
    fn workflow_files_are_yml_or_yaml_in_a_workflows_folder() {
        assert!(is_workflow(".github/workflows/ci.yml"));
        assert!(is_workflow("templates/grid/.github/workflows/ci.yaml"));
        assert!(!is_workflow(".github/workflows/README.md"));
        assert!(!is_workflow(".github/dependabot.yml"));
        assert!(!is_workflow("docs/workflows/ci.yml"));
        assert!(!is_workflow("x.github/workflows/ci.yml"));
        assert!(!is_workflow("ci.yml"));
        assert!(!is_workflow(".github/workflows/yml"));
    }

    #[test]
    fn workflow_folders_leave_out_the_root_and_repeat_none() {
        assert_eq!(
            workflow_folders(
                [
                    ".github/workflows/ci.yml",
                    "templates/grid/.github/workflows/a.yml",
                    "templates/grid/.github/workflows/b.yml",
                    "templates/blank/.github/workflows/ci.yml",
                ]
                .into_iter()
            ),
            vec![
                "/templates/blank/.github/workflows".to_string(),
                "/templates/grid/.github/workflows".to_string(),
            ]
        );
    }

    const DEPENDABOT_TEXT: &str = "\
version: 2
updates:
  - package-ecosystem: cargo
    directory: \"/\"
    schedule:
      interval: weekly
  - package-ecosystem: rust-toolchain
    directory: \"/templates/blank/.github/workflows\"
  - package-ecosystem: github-actions
    directories:
      - \"/\"
      - \"/templates/grid/.github/workflows/\"
";

    #[test]
    fn a_listed_template_folder_passes() {
        let folders = vec!["/templates/grid/.github/workflows".to_string()];
        assert!(check_dependabot(DEPENDABOT_TEXT, &folders).is_empty());
    }

    #[test]
    fn a_template_folder_missing_from_dependabot_is_reported() {
        // Listed only by the rust-toolchain update, which doesn't count.
        let folders = vec![
            "/templates/blank/.github/workflows".to_string(),
            "/templates/grid/.github/workflows".to_string(),
        ];
        assert_eq!(
            check_dependabot(DEPENDABOT_TEXT, &folders),
            vec![
                ".github/dependabot.yml: the github-actions update's `directories` doesn't list \
                 `/templates/blank/.github/workflows`, so Dependabot never moves its pins"
                    .to_string()
            ]
        );
    }

    #[test]
    fn a_single_directory_counts() {
        let text = "updates:\n  - package-ecosystem: github-actions\n    directory: /t/.github/workflows\n";
        assert!(check_dependabot(text, &["/t/.github/workflows".to_string()]).is_empty());
    }

    #[test]
    fn an_unreadable_dependabot_is_reported() {
        let errors = check_dependabot("updates: [", &[]);
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0].starts_with(".github/dependabot.yml: "),
            "{errors:?}"
        );
    }

    const ROOT_TOML: &str = "\
[workspace]
members = [\"crates/*\"]

[workspace.dependencies]
macroquad = { version = \"0.4\", features = [\"audio\"] }
# A comment.
ron = { version = \"0.12\", default-features = false }
include_dir = \"0.7\"
local = { path = \"crates/local\" }

[workspace.lints.rust]
unsafe_code = \"forbid\"
";

    #[test]
    fn requirements_are_read_as_strings_and_inline_tables() {
        let (found, errors) = workspace_requirements("Cargo.toml", ROOT_TOML);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(
            found,
            vec![
                Requirement {
                    name: "macroquad",
                    version: Some("0.4"),
                    line: 5
                },
                Requirement {
                    name: "ron",
                    version: Some("0.12"),
                    line: 7
                },
                Requirement {
                    name: "include_dir",
                    version: Some("0.7"),
                    line: 8
                },
                Requirement {
                    name: "local",
                    version: None,
                    line: 9
                },
            ]
        );
    }

    #[test]
    fn a_multi_line_entry_is_reported() {
        let text =
            "[workspace.dependencies]\nserde = { version = \"1\",\n  \"derive\" }\nron = [\n";
        let (found, errors) = workspace_requirements("templates/x/Cargo.toml", text);
        assert!(found.is_empty(), "{found:?}");
        assert_eq!(
            errors,
            vec![
                "templates/x/Cargo.toml:2: `serde` must keep to one line, as a string or an \
                 inline table"
                    .to_string(),
                "templates/x/Cargo.toml:3: an entry must keep to one line".to_string(),
                "templates/x/Cargo.toml:4: `ron` must keep to one line, as a string or an inline \
                 table"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn an_inline_table_reads_only_its_version_key() {
        assert_eq!(
            inline_version("{ path = \"x\", version = \"2\" }"),
            Some("2")
        );
        assert_eq!(inline_version("{ versions = \"2\" }"), None);
        assert_eq!(inline_version("{ version = 2 }"), None);
        assert_eq!(inline_version("version = \"2\""), None);
    }

    #[test]
    fn a_template_that_follows_the_root_passes() {
        let template = "\
[workspace.dependencies]
shuyi-app = \"{{engine.app}}\"
macroquad = { version = \"0.4\", features = [\"audio\"] }
ron = \"0.12\"
serde = { version = \"1\", features = [\"derive\"] }
";
        assert!(check_template(ROOT_TOML, "templates/grid/Cargo.toml", template).is_empty());
    }

    #[test]
    fn a_template_ron_that_differs_from_the_root_is_reported() {
        let template = "[workspace.dependencies]\nshuyi-ron = \"9\"\nron = \"0.11\"\n";
        assert_eq!(
            check_template(ROOT_TOML, "templates/grid/Cargo.toml", template),
            vec![
                "templates/grid/Cargo.toml:3: `ron` requires \"0.11\", but the root's \
                 Cargo.toml:7 requires \"0.12\""
                    .to_string()
            ]
        );
        let path_only = "[workspace.dependencies]\nlocal = \"1\"\n";
        assert_eq!(
            check_template(ROOT_TOML, "templates/grid/Cargo.toml", path_only),
            vec![
                "templates/grid/Cargo.toml:2: `local` requires \"1\", but the root's \
                 Cargo.toml:9 requires no version"
                    .to_string()
            ]
        );
    }

    fn temp_repo(unique: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "xtask-check-pins-test-{unique}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        for dir in [
            ".github/workflows",
            "templates/grid/.github/workflows",
            ".claude/worktrees/other/.github/workflows",
            "target/.github/workflows",
        ] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        fs::write(root.join("Cargo.toml"), ROOT_TOML).unwrap();
        fs::write(
            root.join("templates/grid/Cargo.toml"),
            "[workspace.dependencies]\nron = \"0.12\"\n",
        )
        .unwrap();
        let pinned = format!("- uses: actions/checkout@{SHA} # v7\n");
        fs::write(root.join(".github/workflows/ci.yml"), &pinned).unwrap();
        fs::write(
            root.join("templates/grid/.github/workflows/ci.yml"),
            &pinned,
        )
        .unwrap();
        fs::write(
            root.join(DEPENDABOT),
            "updates:\n  - package-ecosystem: github-actions\n    directories: [\"/\", \
             \"/templates/grid/.github/workflows\"]\n",
        )
        .unwrap();
        // Other checkouts and build output, never read.
        let broken = format!("- uses: actions/checkout@{OTHER} # v6\n- uses: x/y@main\n");
        fs::write(
            root.join(".claude/worktrees/other/.github/workflows/ci.yml"),
            &broken,
        )
        .unwrap();
        fs::write(root.join("target/.github/workflows/ci.yml"), &broken).unwrap();
        root
    }

    #[test]
    fn a_clean_tree_passes_and_skips_worktrees_and_target() {
        let root = temp_repo("clean");
        let errors = run(&root);
        fs::remove_dir_all(&root).unwrap();
        assert!(errors.is_empty(), "{errors:#?}");
    }

    #[test]
    fn a_tree_reports_each_rule() {
        let root = temp_repo("broken");
        fs::write(
            root.join("templates/grid/.github/workflows/ci.yml"),
            format!("- uses: actions/checkout@{OTHER} # v6\n- uses: x/y@main\n"),
        )
        .unwrap();
        fs::create_dir_all(root.join("templates/blank/.github/workflows")).unwrap();
        fs::write(
            root.join("templates/blank/.github/workflows/ci.yaml"),
            format!("- uses: actions/checkout@{SHA} # v7\n"),
        )
        .unwrap();
        fs::write(
            root.join("templates/blank/Cargo.toml"),
            "[workspace.dependencies]\ninclude_dir = \"0.6\"\n",
        )
        .unwrap();
        let errors = run(&root);
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            errors,
            vec![
                "templates/grid/.github/workflows/ci.yml:2: `x/y@main` is not pinned by a 40-hex \
                 commit SHA"
                    .to_string(),
                format!(
                    "`actions/checkout` is pinned at 2 SHAs; use one: \
                     .github/workflows/ci.yml:1 ({SHA}), \
                     templates/blank/.github/workflows/ci.yaml:1 ({SHA}), \
                     templates/grid/.github/workflows/ci.yml:1 ({OTHER})"
                ),
                ".github/dependabot.yml: the github-actions update's `directories` doesn't list \
                 `/templates/blank/.github/workflows`, so Dependabot never moves its pins"
                    .to_string(),
                "templates/blank/Cargo.toml:2: `include_dir` requires \"0.6\", but the root's \
                 Cargo.toml:8 requires \"0.7\""
                    .to_string(),
            ]
        );
    }

    #[test]
    fn a_missing_dependabot_is_reported_when_a_template_has_workflows() {
        let root = temp_repo("no-dependabot");
        fs::remove_file(root.join(DEPENDABOT)).unwrap();
        let errors = run(&root);
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(errors.len(), 1, "{errors:#?}");
        assert!(
            errors[0].starts_with(".github/dependabot.yml: "),
            "{errors:?}"
        );
    }

    #[test]
    fn real_repo_pins_pass() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("xtask is at <repo>/crates/xtask");
        let errors = run(repo_root);
        assert!(errors.is_empty(), "pins: {errors:#?}");
    }
}
