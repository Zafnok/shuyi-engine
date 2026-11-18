//! The steps resolve one set of features (ticket 2839): the book's Codespace
//! builds every step into one target folder, and a crate whose features
//! differ between two steps is built again, with every crate above it, the
//! engine's included. Step 2's `serde` with its `default` feature, where
//! step 1 had none, cost 73 s of 2-core Codespace; step 7's `ron`, 239 s.
//!
//! It reads `cargo tree` of each step, for the Codespace's two targets: its
//! own (Linux, which builds the `shuyi` command) and the web's. Not `cargo
//! metadata`: its `resolve` holds every target's features at once, and
//! showed step 1 and step 2 alike where the build didn't.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

/// The targets a step is built for in the book's Codespace.
pub const TARGETS: [&str; 2] = ["x86_64-unknown-linux-gnu", "wasm32-unknown-unknown"];

/// `cargo tree`'s format: the package, then its features.
const FORMAT: &str = "{p}|{f}";

/// Each package's sets of features in one step for one target: a package
/// built for the host (a build script's, a macro's) and for the target can
/// have two.
pub type Resolved = BTreeMap<String, Vec<String>>;

/// Runs `cargo tree` in each step for each target and returns every error.
pub fn run(tutorial: &Path, steps: &[String]) -> Vec<String> {
    let mut errors = Vec::new();
    for target in TARGETS {
        let mut resolved = Vec::new();
        for step in steps {
            let folder = tutorial.join(step);
            match tree(&folder, target) {
                Ok(text) => resolved.push((step.clone(), parse(&text, &folder))),
                Err(e) => errors.push(format!("{}/{step}: {e}", super::TUTORIAL)),
            }
        }
        errors.extend(check(target, &resolved));
    }
    errors
}

/// `cargo tree` of the step in `folder`, for `target`, from its lock.
fn tree(folder: &Path, target: &str) -> Result<String, String> {
    let output = Command::new("cargo")
        .args(["tree", "--locked", "--workspace", "--prefix", "none"])
        .args(["--edges", "normal,build", "--target", target])
        .args(["--format", FORMAT])
        .current_dir(folder)
        .output()
        .map_err(|e| format!("cargo tree: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo tree --target {target} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The packages in `cargo tree`'s output (`FORMAT`, one per line), but the
/// step's own (a path under `folder`): those are its own to build.
pub fn parse(text: &str, folder: &Path) -> Resolved {
    let own = slashed(&folder.to_string_lossy());
    let own = format!("({}/", own.trim_end_matches('/'));
    let mut resolved = Resolved::new();
    for line in text.lines() {
        let Some((package, features)) = line.split_once('|') else {
            continue;
        };
        let features = features.trim_end_matches(" (*)").trim();
        let package = package.trim_end_matches(" (proc-macro)").trim();
        if slashed(package).contains(&own) {
            continue;
        }
        let sets = resolved.entry(package.to_owned()).or_default();
        if !sets.iter().any(|set| set == features) {
            sets.push(features.to_owned());
            sets.sort();
        }
    }
    resolved
}

/// `path` with `/` between its folders, as on Windows `cargo tree` writes
/// `\` and a joined path can hold both.
fn slashed(path: &str) -> String {
    path.replace('\\', "/")
}

/// Every package two steps resolve with different features for `target`,
/// each against the first step that has it.
pub fn check(target: &str, steps: &[(String, Resolved)]) -> Vec<String> {
    let mut first: BTreeMap<&str, (&str, &[String])> = BTreeMap::new();
    let mut errors = Vec::new();
    for (step, resolved) in steps {
        for (package, sets) in resolved {
            match first.get(package.as_str()) {
                None => {
                    first.insert(package, (step, sets));
                }
                Some((was, theirs)) if *theirs != sets.as_slice() => errors.push(format!(
                    "{}/{step}: `{package}` has the features {} for {target}, and {was} has {}: \
                     the book's Codespace builds it, and the engine above it, once for each \
                     (ask for it in each step with the features the engine turns on)",
                    super::TUTORIAL,
                    shown(sets),
                    shown(theirs)
                )),
                Some(_) => {}
            }
        }
    }
    errors
}

/// A package's sets of features, as `[a,b]`, `[]` for none.
fn shown(sets: &[String]) -> String {
    sets.iter()
        .map(|set| format!("[{set}]"))
        .collect::<Vec<_>>()
        .join(" and ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEP_1: &str = "\
box-pusher v0.1.0 (/b/site/tutorial/01-new/crates/game)|
serde v1.0.229|alloc,derive,serde_derive,std
serde_derive v1.0.229 (proc-macro)|default
shuyi v0.1.0 (/b/crates/shuyi)|default
serde v1.0.229|alloc,derive,serde_derive,std (*)
";

    fn step(name: &str, text: &str) -> (String, Resolved) {
        let folder = format!("/b/site/tutorial/{name}");
        (name.to_owned(), parse(text, Path::new(&folder)))
    }

    #[test]
    fn parse_drops_the_steps_own_crates_and_keeps_the_engines() {
        let (_, resolved) = step("01-new", STEP_1);
        let packages: Vec<&str> = resolved.keys().map(String::as_str).collect();
        assert_eq!(
            packages,
            [
                "serde v1.0.229",
                "serde_derive v1.0.229",
                "shuyi v0.1.0 (/b/crates/shuyi)"
            ]
        );
        assert_eq!(
            resolved["serde v1.0.229"],
            ["alloc,derive,serde_derive,std"]
        );
        assert_eq!(resolved["serde_derive v1.0.229"], ["default"]);
    }

    #[test]
    fn parse_keeps_a_packages_host_and_target_sets() {
        let (_, resolved) = step("01-new", "libc v0.2.1|std\nlibc v0.2.1|default,std\n");
        assert_eq!(resolved["libc v0.2.1"], ["default,std", "std"]);
    }

    #[test]
    fn steps_alike_pass() {
        let steps = [step("01-new", STEP_1), step("02-board", STEP_1)];
        assert_eq!(
            check("wasm32-unknown-unknown", &steps),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_feature_apart_fails_naming_both_steps() {
        let step_2 = STEP_1.replace(
            "serde v1.0.229|alloc,derive",
            "serde v1.0.229|alloc,default,derive",
        );
        let steps = [step("01-new", STEP_1), step("02-board", &step_2)];
        let errors = check("wasm32-unknown-unknown", &steps);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].starts_with("site/tutorial/02-board: `serde v1.0.229`"),
            "{errors:?}"
        );
        assert!(
            errors[0].contains("[alloc,default,derive,serde_derive,std]"),
            "{errors:?}"
        );
        assert!(
            errors[0].contains("01-new has [alloc,derive,serde_derive,std]"),
            "{errors:?}"
        );
        assert!(errors[0].contains("wasm32-unknown-unknown"), "{errors:?}");
    }

    #[test]
    fn a_package_one_step_adds_passes() {
        let step_7 = format!("{STEP_1}ron v0.12.3|std\n");
        let steps = [step("06-levels", STEP_1), step("07-save", &step_7)];
        assert_eq!(
            check("x86_64-unknown-linux-gnu", &steps),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_step_whose_own_crate_differs_passes() {
        let step_2 = STEP_1.replace("01-new/crates/game)|", "02-board/crates/game)|serde");
        let steps = [step("01-new", STEP_1), step("02-board", &step_2)];
        assert_eq!(
            check("wasm32-unknown-unknown", &steps),
            Vec::<String>::new()
        );
    }
}
