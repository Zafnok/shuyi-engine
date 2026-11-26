//! The contract's version against the base's (ticket 2052, ADR-0150): a
//! PR that changes the contract's schemas (`crates/cli/schemas/`) in a way
//! that breaks a client raises `contract::VERSION`
//! (`crates/cli/src/contract.rs`) by exactly one over the base's, and a PR
//! whose schemas break nothing leaves it as it is. Two PRs that each
//! raised it from 37 to 38 both passed on their own; on the merge ref, the
//! second's is the base's again, and its breaks then ask for 39.
//! `cargo xtask api` runs it against its base, and records the breaks as
//! findings of `shuyi-cli`, which need a `Breaking:` line;
//! `cargo xtask contract` runs it alone.

pub mod compat;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::api::Runner;

/// The file that holds the contract's version.
pub const FILE: &str = "crates/cli/src/contract.rs";

/// The folder of the contract's schemas.
pub const SCHEMAS: &str = "crates/cli/schemas";

/// The file of `SCHEMAS` that is no schema.
const VERSIONS: &str = "versions.json";

/// What the check found: the breaks from the base's schemas to the
/// head's, and the error that fails it, if any.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Checked {
    pub breaks: Vec<String>,
    pub error: Option<String>,
}

/// Checks the working tree's version and schemas against `base`'s. A side
/// whose file or line is missing is not compared: the schemas' own test
/// covers that.
pub fn check(root: &Path, base: &str, runner: &dyn Runner) -> Checked {
    let head = fs::read_to_string(root.join(FILE))
        .ok()
        .and_then(|text| version_in(&text));
    let base_version = show(root, base, FILE, runner)
        .as_deref()
        .and_then(version_in);
    let dir = root.join(SCHEMAS);
    let new = dir.is_dir().then(|| schemas_in(&dir));
    let breaks = match (base_schemas(root, base, runner), new) {
        (Some(old), Some(Ok(new))) if !old.is_empty() => compat::breaks_between(&old, &new),
        (_, Some(Err(error))) => {
            return Checked {
                breaks: Vec::new(),
                error: Some(error),
            };
        }
        _ => Vec::new(),
    };
    let error = match (base_version, head) {
        (Some(base), Some(head)) => compare(base, head, &breaks).err(),
        _ => None,
    };
    Checked { breaks, error }
}

/// `path`'s text at `revision`, if git has it.
fn show(root: &Path, revision: &str, path: &str, runner: &dyn Runner) -> Option<String> {
    runner
        .run(root, "git", &["show", &format!("{revision}:{path}")])
        .ok()
        .filter(|ran| ran.ok)
        .map(|ran| ran.stdout)
}

/// The schemas at `base`, by file name; `None` when git can't list them.
fn base_schemas(root: &Path, base: &str, runner: &dyn Runner) -> Option<BTreeMap<String, Value>> {
    let listed = runner
        .run(
            root,
            "git",
            &["ls-tree", "--name-only", &format!("{base}:{SCHEMAS}")],
        )
        .ok()
        .filter(|ran| ran.ok)?;
    let mut schemas = BTreeMap::new();
    for name in listed.stdout.lines().map(str::trim) {
        if name.is_empty() || name == VERSIONS || !is_json(name) {
            continue;
        }
        let text = show(root, base, &format!("{SCHEMAS}/{name}"), runner)?;
        schemas.insert(name.to_owned(), serde_json::from_str(&text).ok()?);
    }
    Some(schemas)
}

/// The schemas in `dir`, by file name.
pub fn schemas_in(dir: &Path) -> Result<BTreeMap<String, Value>, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut schemas = BTreeMap::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == VERSIONS || !is_json(&name) {
            continue;
        }
        let path = entry.path();
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let schema = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        schemas.insert(name, schema);
    }
    Ok(schemas)
}

/// Whether `name` is a JSON file's.
fn is_json(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("json"))
}

/// The number of `pub const VERSION: u32 = N;` in `text`.
pub fn version_in(text: &str) -> Option<u32> {
    text.lines().find_map(|line| {
        line.trim()
            .strip_prefix("pub const VERSION: u32 = ")?
            .strip_suffix(';')?
            .trim()
            .parse()
            .ok()
    })
}

/// The rule: the base's version when nothing breaks, the base's plus one
/// when something does.
pub fn compare(base: u32, head: u32, breaks: &[String]) -> Result<(), String> {
    let next = base.saturating_add(1);
    if head != base && Some(head) != base.checked_add(1) {
        return Err(format!(
            "{FILE}: `VERSION` goes from {base} to {head}; a break raises it by one, to {next}, \
             and any other change leaves it at {base}. Another PR may have raised it first: \
             set it to {next} if this PR's schemas break a client, else to {base}; move this \
             PR's `schemas/versions.json` line to that version, and fix the PR body and \
             completion notes that name {head}"
        ));
    }
    if breaks.is_empty() && head == next {
        return Err(format!(
            "{FILE}: `VERSION` goes from {base} to {head}, and nothing in the schemas breaks a \
             client (ADR-0150): leave it at {base}, and give this PR's `schemas/versions.json` \
             line version {base}"
        ));
    }
    if !breaks.is_empty() && head == base {
        let listed = breaks.iter().fold(String::new(), |mut out, b| {
            let _ = write!(out, "\n  - {b}");
            out
        });
        return Err(format!(
            "the contract's schemas break a client, and `VERSION` is still {base} (ADR-0150): \
             raise it to {next}, give this PR's `schemas/versions.json` line version {next}, \
             and write a `Breaking: shuyi-cli: contract: …` line in the ticket's completion \
             notes. Better, make the change so that it breaks nothing. The breaks:{listed}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Ran;

    fn a_break() -> Vec<String> {
        vec!["run.output.json: frames: the field is gone".to_owned()]
    }

    #[test]
    fn an_additive_change_keeps_the_version() {
        assert_eq!(compare(41, 41, &[]), Ok(()));
        let error = compare(41, 42, &[]).unwrap_err();
        assert!(error.contains("leave it at 41"), "{error}");
    }

    #[test]
    fn a_break_raises_it_by_one() {
        assert_eq!(compare(41, 42, &a_break()), Ok(()));
        let error = compare(41, 41, &a_break()).unwrap_err();
        assert!(error.contains("raise it to 42"), "{error}");
        assert!(error.contains("frames: the field is gone"), "{error}");
        assert!(error.contains("Breaking: shuyi-cli"), "{error}");
    }

    #[test]
    fn two_more_fails() {
        let error = compare(41, 43, &a_break()).unwrap_err();
        assert!(error.contains("from 41 to 43"), "{error}");
        assert!(error.contains("to 42"), "{error}");
    }

    #[test]
    fn a_lowered_version_fails() {
        assert!(compare(41, 40, &[]).is_err());
        assert!(compare(41, 0, &a_break()).is_err());
    }

    #[test]
    fn reads_the_constant() {
        let text = "/// The version.\npub const VERSION: u32 = 41;\n";
        assert_eq!(version_in(text), Some(41));
        assert_eq!(version_in("const VERSION: u32 = 41;"), None);
        assert_eq!(version_in("pub const VERSION: u32 = x;"), None);
    }

    fn repo_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("xtask is at <repo>/crates/xtask")
    }

    #[test]
    fn real_contract_has_a_version() {
        let text = fs::read_to_string(repo_root().join(FILE)).unwrap();
        assert!(version_in(&text).is_some());
    }

    /// Answers `git show` of the version's file with `version`, and of a
    /// schema with the head's own, but `scene.output.json` with `scene`;
    /// `git ls-tree` with the head's schemas' names.
    struct Base {
        version: Option<String>,
        scene: Option<Value>,
    }

    impl Runner for Base {
        fn run(&self, _: &Path, _: &str, args: &[&str]) -> Result<Ran, String> {
            let dir = repo_root().join(SCHEMAS);
            let answer = |text: Option<String>| Ran {
                ok: text.is_some(),
                stdout: text.unwrap_or_default(),
                stderr: String::new(),
            };
            if args.first() == Some(&"ls-tree") {
                let names: Vec<String> = schemas_in(&dir)?.into_keys().collect();
                return Ok(answer(Some(names.join("\n"))));
            }
            let Some(path) = args.get(1).and_then(|a| a.split_once(':')).map(|(_, p)| p) else {
                return Ok(answer(None));
            };
            if path == FILE {
                return Ok(answer(self.version.clone()));
            }
            let name = path.rsplit('/').next().unwrap_or_default();
            if name == "scene.output.json"
                && let Some(scene) = &self.scene
            {
                return Ok(answer(Some(scene.to_string())));
            }
            Ok(answer(fs::read_to_string(dir.join(name)).ok()))
        }
    }

    fn head_version() -> u32 {
        version_in(&fs::read_to_string(repo_root().join(FILE)).unwrap()).unwrap()
    }

    fn version(v: u32) -> String {
        format!("pub const VERSION: u32 = {v};\n")
    }

    #[test]
    fn checks_the_working_tree_against_the_base() {
        let root = repo_root();
        let head = head_version();
        let same = Base {
            version: Some(version(head)),
            scene: None,
        };
        assert_eq!(check(root, "base", &same), Checked::default());
        let lower = Base {
            version: Some(version(head - 2)),
            scene: None,
        };
        assert!(check(root, "base", &lower).error.is_some());
        let none = Base {
            version: None,
            scene: None,
        };
        assert_eq!(check(root, "base", &none), Checked::default());
    }

    /// The head's `scene.output.json`, with a field the head lacks.
    fn scene_with_a_field_more() -> Value {
        let text = fs::read_to_string(repo_root().join(SCHEMAS).join("scene.output.json")).unwrap();
        let mut scene: Value = serde_json::from_str(&text).unwrap();
        scene["properties"]["gone"] = serde_json::json!({ "type": "integer" });
        scene
    }

    #[test]
    fn a_base_with_a_field_the_head_lost_asks_for_a_raise() {
        let root = repo_root();
        let head = head_version();
        let base = Base {
            version: Some(version(head)),
            scene: Some(scene_with_a_field_more()),
        };
        let checked = check(root, "base", &base);
        assert_eq!(
            checked.breaks,
            ["scene.output.json: gone: the field is gone"]
        );
        let error = checked.error.expect("a break with the same version fails");
        assert!(
            error.contains(&format!("raise it to {}", head + 1)),
            "{error}"
        );
        // The head raised it: it passes, and the break is still listed.
        let base = Base {
            version: Some(version(head - 1)),
            scene: Some(scene_with_a_field_more()),
        };
        let checked = check(root, "base", &base);
        assert_eq!(checked.error, None);
        assert_eq!(checked.breaks.len(), 1);
    }

    #[test]
    fn a_base_with_a_field_less_is_additive() {
        let root = repo_root();
        let head = head_version();
        let text = fs::read_to_string(root.join(SCHEMAS).join("scene.output.json")).unwrap();
        let mut scene: Value = serde_json::from_str(&text).unwrap();
        scene["properties"]
            .as_object_mut()
            .unwrap()
            .remove("limits");
        scene["required"] = serde_json::json!(["contract", "op", "ok"]);
        let base = Base {
            version: Some(version(head)),
            scene: Some(scene.clone()),
        };
        assert_eq!(check(root, "base", &base), Checked::default());
        let raised = Base {
            version: Some(version(head - 1)),
            scene: Some(scene),
        };
        let error = check(root, "base", &raised)
            .error
            .expect("a raise with no break fails");
        assert!(
            error.contains(&format!("leave it at {}", head - 1)),
            "{error}"
        );
    }
}
