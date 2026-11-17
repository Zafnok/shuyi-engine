use std::fs;
use std::path::PathBuf;

use super::*;

/// The root's container, as the repository's.
const ROOT_JSON: &str = r#"{
  "name": "Shuyi Engine",
  "image": "mcr.microsoft.com/devcontainers/base:ubuntu-24.04",
  "postCreateCommand": "bash scripts/setup-linux.sh && . \"$HOME/.cargo/env\" && cargo run -q --bin shuyi -- mcp install --yes",
  "remoteEnv": { "PATH": "${containerEnv:PATH}:/home/vscode/.cargo/bin" }
}"#;

/// The book's, with comments, building three steps.
const BOOK_JSON: &str = r#"// The book's project.
{
  "name": "The book's project (Box Pusher)",
  /* the root's */ "image": "mcr.microsoft.com/devcontainers/base:ubuntu-24.04",
  "workspaceFolder": "/workspaces/${localWorkspaceFolderBasename}/site/tutorial",
  // A comment's "quotes" and a URL in a string: "https://example.com/x"
  "onCreateCommand": "bash ../../scripts/setup-linux.sh && . \"$HOME/.cargo/env\" && bash ../../.devcontainer/book/build.sh 01-new 02-board 03-walk",
  "postAttachCommand": "bash serve.sh",
  "remoteEnv": { "PATH": "${containerEnv:PATH}:/home/vscode/.cargo/bin" },
  "docs": "https://example.com/a // not a comment /* nor this */"
}"#;

fn steps(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn three() -> Vec<String> {
    steps(&["01-new", "02-board", "03-walk"])
}

#[test]
fn the_book_in_step_passes() {
    assert_eq!(check(ROOT_JSON, BOOK_JSON, &three()), Vec::<String>::new());
}

#[test]
fn comments_are_taken_out_and_strings_kept() {
    let value = parse(BOOK, BOOK_JSON).unwrap();
    assert_eq!(
        value["docs"],
        "https://example.com/a // not a comment /* nor this */"
    );
    assert_eq!(
        value["image"],
        "mcr.microsoft.com/devcontainers/base:ubuntu-24.04"
    );
    // A line is still a line, so an error names the file's.
    let error = parse(BOOK, "// one\n/* two\nthree */\n{ oops }").unwrap_err();
    assert!(error.contains("line 4"), "{error}");
    // An escaped quote doesn't end a string.
    assert_eq!(
        parse(BOOK, r#"{ "a": "x\" // y" }"#).unwrap()["a"],
        "x\" // y"
    );
}

#[test]
fn a_step_missing_from_the_container_fails() {
    let errors = check(
        ROOT_JSON,
        BOOK_JSON,
        &steps(&["01-new", "02-board", "03-walk", "04-push"]),
    );
    assert_eq!(
        errors,
        [format!(
            "{BOOK}: it doesn't build step `04-push` of {TUTORIAL}: add it to `build.sh`'s steps"
        )]
    );
}

#[test]
fn a_step_that_isnt_there_fails() {
    let errors = check(ROOT_JSON, BOOK_JSON, &steps(&["01-new", "02-board"]));
    assert_eq!(
        errors,
        [format!(
            "{BOOK}: it builds step `03-walk`, which {TUTORIAL} doesn't have (a step is a folder with a `shuyi.ron`)"
        )]
    );
}

#[test]
fn a_step_twice_fails() {
    let twice = BOOK_JSON.replace("02-board 03-walk", "02-board 02-board 03-walk");
    assert_eq!(
        check(ROOT_JSON, &twice, &three()),
        [format!("{BOOK}: it builds step `02-board` twice")]
    );
}

#[test]
fn the_steps_may_be_split_between_the_two_commands_a_prebuild_runs() {
    // Step 2 first, then the rest behind it, in any order.
    let split = BOOK_JSON
        .replace("build.sh 01-new 02-board 03-walk", "build.sh 02-board")
        .replace(
            "\"postAttachCommand\"",
            "\"updateContentCommand\": \"bash ../../.devcontainer/book/build.sh 03-walk 01-new\", \"postAttachCommand\"",
        );
    assert_eq!(check(ROOT_JSON, &split, &three()), Vec::<String>::new());
    // A build in a command a prebuild doesn't run doesn't count.
    let late = split.replace("updateContentCommand", "postCreateCommand");
    assert_eq!(
        check(ROOT_JSON, &late, &three()),
        [
            format!(
                "{BOOK}: it doesn't build step `01-new` of {TUTORIAL}: add it to `build.sh`'s steps"
            ),
            format!(
                "{BOOK}: it doesn't build step `03-walk` of {TUTORIAL}: add it to `build.sh`'s steps"
            ),
        ]
    );
    // Nor do both name one step.
    let both = split.replace(
        "build.sh 03-walk 01-new",
        "build.sh 03-walk 01-new 02-board",
    );
    assert_eq!(
        check(ROOT_JSON, &both, &three()),
        [format!("{BOOK}: it builds step `02-board` twice")]
    );
}

#[test]
fn no_build_fails() {
    let none = BOOK_JSON.replace(
        " && bash ../../.devcontainer/book/build.sh 01-new 02-board 03-walk",
        "",
    );
    let errors = check(ROOT_JSON, &none, &three());
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("nor its `updateContentCommand` runs"),
        "{errors:?}"
    );
    // A script of the same name elsewhere isn't the build.
    let elsewhere = BOOK_JSON.replace(".devcontainer/book/build.sh", "build.sh");
    assert!(
        check(ROOT_JSON, &elsewhere, &three())[0].contains("nor its `updateContentCommand` runs")
    );
}

#[test]
fn a_drifted_image_fails() {
    let drifted = BOOK_JSON.replace("ubuntu-24.04", "ubuntu-22.04");
    assert_eq!(
        check(ROOT_JSON, &drifted, &three()),
        [format!(
            "{BOOK}: its `image` is `\"mcr.microsoft.com/devcontainers/base:ubuntu-22.04\"`, and the root's \
             ({ROOT}) is `\"mcr.microsoft.com/devcontainers/base:ubuntu-24.04\"`: they are kept the same"
        )]
    );
}

#[test]
fn a_drifted_setup_fails() {
    // The root's setup moves on, and the book's doesn't.
    let root = ROOT_JSON.replace(
        "bash scripts/setup-linux.sh &&",
        "bash scripts/setup-linux.sh --web &&",
    );
    let errors = check(&root, BOOK_JSON, &three());
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("doesn't start with the root's setup"),
        "{errors:?}"
    );
    assert!(
        errors[0].contains("`bash scripts/setup-linux.sh --web && . \"$HOME/.cargo/env\"`"),
        "{errors:?}"
    );
    // The book's runs another script, or doesn't read cargo's environment.
    for book in [
        BOOK_JSON.replace("../../scripts/setup-linux.sh", "scripts/setup-linux.sh"),
        BOOK_JSON.replace(" && . \\\"$HOME/.cargo/env\\\"", ""),
    ] {
        let errors = check(ROOT_JSON, &book, &three());
        assert!(
            errors
                .iter()
                .any(|e| e.contains("doesn't start with the root's setup")),
            "{errors:?}"
        );
    }
}

#[test]
fn a_root_without_a_setup_fails() {
    let root = ROOT_JSON.replace(" && . \\\"$HOME/.cargo/env\\\"", "");
    let errors = check(&root, BOOK_JSON, &three());
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].starts_with(ROOT) && errors[0].contains("never reads cargo's environment"),
        "{errors:?}"
    );
}

#[test]
fn a_drifted_remote_env_fails() {
    let drifted = BOOK_JSON.replace("/home/vscode/.cargo/bin\" },", "/usr/local/cargo/bin\" },");
    let errors = check(ROOT_JSON, &drifted, &three());
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("`remoteEnv`"), "{errors:?}");
}

#[test]
fn the_wrong_folder_fails() {
    let root_folder = BOOK_JSON.replace("/site/tutorial\"", "\"");
    let errors = check(ROOT_JSON, &root_folder, &three());
    assert_eq!(
        errors,
        [format!(
            "{BOOK}: its `workspaceFolder` opens ``, not the book's project, `{TUTORIAL}`"
        )]
    );
    let outside = BOOK_JSON.replace("/workspaces/${localWorkspaceFolderBasename}", "/home");
    let errors = check(ROOT_JSON, &outside, &three());
    assert!(
        errors[0].contains("isn't in the repository's folder"),
        "{errors:?}"
    );
}

#[test]
fn a_file_that_isnt_json_fails() {
    let errors = check("{", "[1,", &three());
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(errors[0].starts_with(ROOT), "{errors:?}");
    assert!(errors[1].starts_with(BOOK), "{errors:?}");
}

#[test]
fn paths_resolve_from_the_folder() {
    assert_eq!(
        resolve("site/tutorial", "../../scripts/a.sh").as_deref(),
        Some("scripts/a.sh")
    );
    assert_eq!(
        resolve("", "./scripts//a.sh").as_deref(),
        Some("scripts/a.sh")
    );
    assert_eq!(resolve("site", "../../a.sh"), None);
    assert_eq!(resolve("", "/abs/a.sh"), None);
    assert_eq!(
        normalize("site/tutorial", "bash  ../../scripts/a.sh  --x"),
        "bash scripts/a.sh --x"
    );
    assert_eq!(
        normalize("", ". \"$HOME/.cargo/env\""),
        ". \"$HOME/.cargo/env\""
    );
}

/// A repository in a temporary folder: the two containers, and a project
/// whose steps are `on_disk` (each with a `shuyi.ron`), beside a folder
/// that isn't a step.
fn repository(name: &str, book: &str, on_disk: &[&str]) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("xtask-devcontainers-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".devcontainer/book")).unwrap();
    fs::write(root.join(ROOT), ROOT_JSON).unwrap();
    fs::write(root.join(BOOK), book).unwrap();
    for step in on_disk {
        fs::create_dir_all(root.join(TUTORIAL).join(step)).unwrap();
        fs::write(root.join(TUTORIAL).join(step).join("shuyi.ron"), "()").unwrap();
    }
    fs::create_dir_all(root.join(TUTORIAL).join("notes")).unwrap();
    fs::write(root.join(TUTORIAL).join("tries.toml"), "").unwrap();
    root
}

#[test]
fn a_fixture_tree_in_step_passes() {
    let root = repository("ok", BOOK_JSON, &["01-new", "02-board", "03-walk"]);
    assert_eq!(run(&root), Vec::<String>::new());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_fixture_tree_with_a_missing_and_an_extra_step_and_a_drifted_container_fails() {
    // 03-walk is gone from the project, 04-push is new, and the image drifted.
    let book = BOOK_JSON.replace("ubuntu-24.04", "ubuntu-26.04");
    let root = repository("drift", &book, &["01-new", "02-board", "04-push"]);
    let errors = run(&root);
    fs::remove_dir_all(root).unwrap();
    assert_eq!(errors.len(), 3, "{errors:?}");
    assert!(errors[0].contains("its `image`"), "{errors:?}");
    assert!(
        errors[1].contains("builds step `03-walk`, which"),
        "{errors:?}"
    );
    assert!(
        errors[2].contains("doesn't build step `04-push`"),
        "{errors:?}"
    );
}

#[test]
fn a_fixture_tree_without_the_book_fails() {
    let root = repository("gone", BOOK_JSON, &["01-new"]);
    fs::remove_file(root.join(BOOK)).unwrap();
    let errors = run(&root);
    fs::remove_dir_all(root).unwrap();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].starts_with(BOOK), "{errors:?}");
}

#[test]
fn the_repository_passes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert_eq!(run(&root), Vec::<String>::new());
}
