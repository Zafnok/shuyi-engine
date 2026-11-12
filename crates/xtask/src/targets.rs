//! `cargo xtask check-targets`: no two workspace packages build a binary,
//! or an example, of the same name (ticket 2056).
//!
//! Cargo writes every package's binaries to one `target/<profile>/` and
//! every example to one `target/<profile>/examples/`, so two of one name
//! write one file. Cargo only warns of it, and on Windows the MSVC linker
//! fails when the two links run at once (`LNK1104`), only sometimes.
//!
//! [`check`] is a pure function over `cargo metadata`'s JSON; [`run`]
//! feeds it the repository's.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use serde::Deserialize;

/// The kinds of target whose files share a folder across the workspace.
const KINDS: [&str; 2] = ["bin", "example"];

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    targets: Vec<Target>,
}

#[derive(Deserialize)]
struct Target {
    name: String,
    kind: Vec<String>,
}

/// Checks the repository at `repo_root` and returns every error (empty
/// means clean).
pub fn run(repo_root: &Path) -> Vec<String> {
    match metadata(repo_root) {
        Ok(json) => check(&json),
        Err(e) => vec![e],
    }
}

/// `cargo metadata --no-deps`'s JSON for the repository.
fn metadata(repo_root: &Path) -> Result<String, String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--no-deps", "--locked"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("cargo metadata does not run: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| format!("cargo metadata's output: {e}"))
}

/// One error per binary or example name that more than one workspace
/// package builds.
pub fn check(json: &str) -> Vec<String> {
    let metadata: Metadata = match serde_json::from_str(json) {
        Ok(metadata) => metadata,
        Err(e) => return vec![format!("cargo metadata's output: {e}")],
    };
    // (kind, name) -> the packages that build it.
    let mut owners: BTreeMap<(&str, &str), Vec<&str>> = BTreeMap::new();
    for package in metadata
        .packages
        .iter()
        .filter(|p| metadata.workspace_members.contains(&p.id))
    {
        for target in &package.targets {
            for kind in KINDS {
                if target.kind.iter().any(|k| k == kind) {
                    owners
                        .entry((kind, target.name.as_str()))
                        .or_default()
                        .push(package.name.as_str());
                }
            }
        }
    }
    owners
        .into_iter()
        .filter(|(_, packages)| packages.len() > 1)
        .map(|((kind, name), mut packages)| {
            packages.sort_unstable();
            format!(
                "the {kind} `{name}` is built by {}: their outputs are one file; rename all but one",
                packages
                    .iter()
                    .map(|p| format!("`{p}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A workspace of `hello-grid` and `book-snippets`, with the example
    /// `book-snippets` names; `extra` is one more package outside it.
    fn metadata(snippets_example: &str, outside_example: &str) -> String {
        format!(
            r#"{{
  "packages": [
    {{"id": "hello-grid 0.1.0", "name": "hello-grid", "targets": [
      {{"name": "hello_grid", "kind": ["lib"]}},
      {{"name": "hello-grid", "kind": ["bin"]}},
      {{"name": "tools", "kind": ["example"]}}
    ]}},
    {{"id": "book-snippets 0.1.0", "name": "book-snippets", "targets": [
      {{"name": "book_snippets", "kind": ["lib"]}},
      {{"name": "{snippets_example}", "kind": ["example"]}}
    ]}},
    {{"id": "dep 1.0.0", "name": "dep", "targets": [
      {{"name": "{outside_example}", "kind": ["example"]}}
    ]}}
  ],
  "workspace_members": ["hello-grid 0.1.0", "book-snippets 0.1.0"]
}}"#
        )
    }

    #[test]
    fn distinct_names_pass() {
        assert_eq!(
            check(&metadata("snippet-tools", "other")),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_second_tools_example_fails() {
        let errors = check(&metadata("tools", "other"));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("example `tools`"), "{errors:?}");
        assert!(
            errors[0].contains("`book-snippets`, `hello-grid`"),
            "{errors:?}"
        );
    }

    #[test]
    fn a_package_outside_the_workspace_is_left_out() {
        // `dep`'s example `tools` is never built into the workspace's target.
        assert_eq!(
            check(&metadata("snippet-tools", "tools")),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_bin_and_an_example_of_one_name_do_not_collide() {
        // `hello-grid`'s bin `hello-grid` and an example `hello-grid` are
        // in different folders.
        assert_eq!(check(&metadata("hello-grid", "x")), Vec::<String>::new());
    }

    #[test]
    fn a_lib_is_not_checked() {
        assert_eq!(check(&metadata("book_snippets", "x")), Vec::<String>::new());
    }

    #[test]
    fn bad_json_is_an_error() {
        assert_eq!(check("{").len(), 1);
    }

    #[test]
    fn the_repository_has_no_collision() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(run(&root), Vec::<String>::new());
    }
}
