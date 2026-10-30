//! `check-modules` on fixture workspaces: a clean one, and one for each
//! failure, each written to a folder of its own and read through `cargo
//! metadata`, as the repository is.

use std::path::PathBuf;
use std::process::Command;

use super::*;

/// A fixture: its files, by path from its root.
type Files = BTreeMap<String, String>;

/// A crate's `Cargo.toml`: its `[package]`, then `rest`.
fn manifest(name: &str, rest: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n{rest}")
}

/// A module's `[package.metadata.shuyi]`.
fn metadata(concern: &str, answer: &str, alternatives: &[&str]) -> String {
    let alternatives: Vec<String> = alternatives.iter().map(|a| format!("\"{a}\"")).collect();
    format!(
        "[package.metadata.shuyi]\nanswers = [{{ concern = \"{concern}\", answer = \"{answer}\" }}]\n\
         serves = [\"side-view\"]\nalternatives = [{}]\n",
        alternatives.join(", ")
    )
}

const CONCERNS_RON: &str = r#"(concerns: [
    (id: "bodies", name: "Bodies", alternatives: true, answers: [
        (id: "rigid-body", name: "rigid", by: "shuyi-alpha", status: planned, tickets: []),
        (id: "procedural", name: "procedural", by: "shuyi-beta", status: planned, tickets: []),
    ]),
    (id: "kind", name: "Kind of game", alternatives: false, answers: [
        (id: "rts", name: "rts", by: "shuyi-alpha", status: planned, tickets: []),
        (id: "vn", name: "vn", by: "shuyi-beta", status: planned, tickets: []),
    ]),
])"#;

const CATALOG_MD: &str = "# The modules\n\n| Module | Serves | Status | Tickets |\n\
                          | ------ | ------ | ------ | ------- |\n\
                          | `shuyi-alpha` | rigid | planned | 1 |\n\
                          | `shuyi-beta` | procedural | planned | 2 |\n";

/// A clean workspace: a default crate, the front door with a feature per
/// module, two modules that are alternatives, and the xtask.
fn clean() -> Files {
    let mut files = Files::new();
    let mut add = |path: &str, text: String| {
        files.insert(path.to_owned(), text);
    };
    add(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\"]\nresolver = \"3\"\n".into(),
    );
    add("crates/core/Cargo.toml", manifest("shuyi-core", ""));
    add(
        "crates/shuyi/Cargo.toml",
        manifest(
            "shuyi",
            "[features]\nalpha = [\"dep:shuyi-alpha\"]\nbeta = [\"dep:shuyi-beta\"]\n\n\
             [dependencies]\nshuyi-core = { path = \"../core\" }\n\
             shuyi-alpha = { path = \"../alpha\", optional = true }\n\
             shuyi-beta = { path = \"../beta\", optional = true }\n",
        ),
    );
    add(
        "crates/alpha/Cargo.toml",
        manifest(
            "shuyi-alpha",
            &metadata("bodies", "rigid-body", &["shuyi-beta"]),
        ),
    );
    add("crates/alpha/README.md", "# shuyi-alpha\n\nRigid.\n".into());
    add(
        "crates/beta/Cargo.toml",
        manifest(
            "shuyi-beta",
            &metadata("bodies", "procedural", &["shuyi-alpha"]),
        ),
    );
    add(
        "crates/beta/README.md",
        "# shuyi-beta\n\nProcedural.\n".into(),
    );
    add("crates/xtask/Cargo.toml", manifest("xtask", ""));
    add(CONCERNS, CONCERNS_RON.into());
    add(CATALOG, CATALOG_MD.into());
    files
}

/// `files` as a workspace in a folder of its own named `name`, with a
/// `src/lib.rs` in each crate and a lockfile, checked.
fn checked(name: &str, files: &Files) -> Vec<String> {
    let root: PathBuf =
        std::env::temp_dir().join(format!("xtask-modules-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    for (path, text) in files {
        let to = root.join(path);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::write(&to, text).unwrap();
        if path.ends_with("/Cargo.toml") {
            let src = to.parent().unwrap().join("src");
            fs::create_dir_all(&src).unwrap();
            fs::write(src.join("lib.rs"), "").unwrap();
        }
    }
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(cargo)
        .args(["generate-lockfile", "--offline", "--quiet"])
        .current_dir(&root)
        .status()
        .unwrap();
    assert!(status.success(), "{name}: cargo generate-lockfile failed");
    let errors = run(&root);
    fs::remove_dir_all(&root).unwrap();
    errors
}

#[test]
fn a_clean_workspace_passes() {
    assert_eq!(checked("clean", &clean()), Vec::<String>::new());
}

#[test]
fn a_default_crate_that_depends_on_a_module_fails() {
    let mut files = clean();
    files.insert(
        "crates/core/Cargo.toml".into(),
        manifest(
            "shuyi-core",
            "[dev-dependencies]\nshuyi-alpha = { path = \"../alpha\" }\n",
        ),
    );
    assert_eq!(
        checked("default", &files),
        [
            "shuyi-core depends on the module shuyi-alpha (a dev-dependency): no default crate \
             depends on a module (ADR-0009 §4); move what needs it into the module, or into the \
             game"
        ]
    );
}

#[test]
fn a_module_without_a_readme_metadata_or_a_row_fails_on_each() {
    let mut files = clean();
    files.insert(
        "crates/gamma/Cargo.toml".into(),
        manifest("shuyi-gamma", ""),
    );
    assert_eq!(
        checked("bare", &files),
        [
            "shuyi-gamma is a module (a crate under crates/ outside the default engine) with no \
             README.md: write one in ADR-0009 §4's order",
            "shuyi-gamma is a module with no row in docs/modules.md: add one under its concern \
             (what it serves, its status, its tickets)",
            "shuyi-gamma is a module with no [package.metadata.shuyi] in its Cargo.toml: say what \
             it answers (`answers`, `serves`, `alternatives`, `works_with`, ADR-0009 §4)",
        ]
    );
}

#[test]
fn metadata_that_doesnt_read_or_serves_nothing_fails() {
    let mut files = clean();
    let alpha = files["crates/alpha/Cargo.toml"].replace("serves = [\"side-view\"]", "serves = []");
    files.insert("crates/alpha/Cargo.toml".into(), alpha);
    let beta = files["crates/beta/Cargo.toml"].replace("serves", "serve");
    files.insert("crates/beta/Cargo.toml".into(), beta);
    assert_eq!(
        checked("unread", &files),
        [
            "shuyi-alpha's `serves` in [package.metadata.shuyi] is empty: name the directions \
             it is built for (ADR-0009 §7)",
            "shuyi-beta's [package.metadata.shuyi] doesn't read: unknown key `serve`, expected \
             one of answers, serves, alternatives, works_with, formats (ADR-0009 §4)",
        ]
    );
}

#[test]
fn an_answer_or_a_concern_not_in_the_concerns_file_fails() {
    let mut files = clean();
    let alpha = files["crates/alpha/Cargo.toml"].replace("rigid-body", "soft-body");
    files.insert("crates/alpha/Cargo.toml".into(), alpha);
    let beta = files["crates/beta/Cargo.toml"].replace("\"bodies\"", "\"sound\"");
    files.insert("crates/beta/Cargo.toml".into(), beta);
    assert_eq!(
        checked("answers", &files),
        [
            "shuyi-alpha answers `bodies` with `soft-body`, which docs/modules.ron doesn't list \
             under it: add the answer there (ADR-0009 §2), or fix the name",
            "shuyi-beta answers the concern `sound`, which docs/modules.ron doesn't have: add it \
             there (ADR-0009 §2), or fix the name",
        ]
    );
}

#[test]
fn alternatives_that_dont_name_each_other_fail() {
    let mut files = clean();
    let beta = files["crates/beta/Cargo.toml"].replace("[\"shuyi-alpha\"]", "[]");
    files.insert("crates/beta/Cargo.toml".into(), beta);
    assert_eq!(
        checked("alternatives", &files),
        [
            "shuyi-beta and shuyi-alpha both answer `bodies` and shuyi-beta's `alternatives` \
             doesn't name shuyi-alpha: alternatives name each other (ADR-0009 §2), unless the \
             concern is `alternatives: false` in docs/modules.ron"
        ]
    );
}

#[test]
fn answers_to_a_concern_that_shares_a_stage_need_not_name_each_other() {
    let mut files = clean();
    for (module, answer) in [("alpha", "rts"), ("beta", "vn")] {
        let path = format!("crates/{module}/Cargo.toml");
        let text = manifest(&format!("shuyi-{module}"), &metadata("kind", answer, &[]));
        files.insert(path, text);
    }
    assert_eq!(checked("kind", &files), Vec::<String>::new());
}

#[test]
fn a_feature_of_the_front_door_with_no_row_fails() {
    let mut files = clean();
    let shuyi =
        files["crates/shuyi/Cargo.toml"].replace("[features]\n", "[features]\nghost = []\n");
    files.insert("crates/shuyi/Cargo.toml".into(), shuyi);
    assert_eq!(
        checked("feature", &files),
        [
            "the shuyi crate's feature `ghost` names no row of docs/modules.md: a feature turns \
             on a module (`ghost` is shuyi-ghost), and every module has a row"
        ]
    );
}

/// A backend answers no concern: its README and its row in the table of
/// backends are what it needs, and no metadata.
#[test]
fn a_backend_needs_its_readme_and_its_row_not_metadata() {
    let mut files = clean();
    let row = "\n## Platform backends\n\n| Backend |\n| ------- |\n| `shuyi-gadget` |\n";
    files.insert(CATALOG.to_owned(), format!("{CATALOG_MD}{row}"));
    files.insert(
        "crates/gadget/Cargo.toml".into(),
        manifest("shuyi-gadget", ""),
    );
    assert_eq!(
        checked("backend-bare", &files),
        [
            "shuyi-gadget is a backend (it has a row in docs/modules.md's Platform backends) \
             with no README.md: write one"
        ]
    );
    files.insert("crates/gadget/README.md".into(), "# shuyi-gadget\n".into());
    assert_eq!(checked("backend", &files), [] as [String; 0]);
}

/// `std` and `app` are the default engine's, and no module's.
#[test]
fn the_front_doors_own_features_need_no_row() {
    let mut files = clean();
    files.insert(
        "crates/shuyi/Cargo.toml".into(),
        manifest(
            "shuyi",
            "[features]\ndefault = [\"std\", \"app\"]\nstd = []\napp = [\"std\"]\n\
             alpha = [\"dep:shuyi-alpha\"]\nbeta = [\"dep:shuyi-beta\"]\n\n\
             [dependencies]\nshuyi-core = { path = \"../core\" }\n\
             shuyi-alpha = { path = \"../alpha\", optional = true }\n\
             shuyi-beta = { path = \"../beta\", optional = true }\n",
        ),
    );
    assert_eq!(checked("front-door", &files), [] as [String; 0]);
}

#[test]
fn missing_files_are_errors() {
    let mut files = clean();
    files.remove(CONCERNS);
    files.remove(CATALOG);
    let errors = checked("missing", &files);
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(errors[0].starts_with("docs/modules.ron: "), "{errors:?}");
    assert!(errors[1].starts_with("docs/modules.md: "), "{errors:?}");
    assert!(
        parse_concerns("(concerns: 1)")
            .unwrap_err()
            .starts_with("docs/modules.ron: ")
    );
}

#[test]
fn a_row_is_the_first_name_of_a_table_rows_first_cell() {
    let rows = rows(
        "| Module | x |\n| --- | --- |\n| `shuyi-a` | `not-this` |\n  | `shuyi-assets` sfx | y |\n\
         text `shuyi-b`\n| plain | z |\n",
    );
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    assert_eq!(rows, ["shuyi-a", "shuyi-assets"]);
}

#[test]
fn the_repository_passes() {
    assert_eq!(run(&crate::repo_root()), Vec::<String>::new());
}

#[test]
fn the_keys_are_shuyi_clis_module_info_fields() {
    let source = "crates/cli/src/catalog.rs";
    let text = fs::read_to_string(crate::repo_root().join(source)).unwrap();
    let info = text
        .split("pub struct ModuleInfo {")
        .nth(1)
        .and_then(|rest| rest.split("\n}").next())
        .unwrap_or_else(|| panic!("{source} has no `pub struct ModuleInfo`"));
    let fields: Vec<&str> = info
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub "))
        .filter_map(|line| line.split(':').next())
        .collect();
    assert_eq!(
        fields, KEYS,
        "{source}'s `ModuleInfo` and `KEYS` differ: keep them equal"
    );
}
