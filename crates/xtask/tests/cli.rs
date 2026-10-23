//! `cargo xtask` as CI runs it: its commands, its options and its exit code.

use std::path::PathBuf;
use std::process::Command;

/// Runs the `xtask` binary with `args`, from a folder that isn't the
/// repository's root (the binary finds the root by itself), and returns
/// whether it succeeded and what it printed.
fn xtask(args: &[&str]) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap_or_else(|e| panic!("the xtask binary does not run: {e}"));
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// A game checkout that doesn't exist: `game#` ids are warned about and
/// skipped, so the tests never need `gh` or the game.
fn no_game() -> String {
    let path: PathBuf = std::env::temp_dir().join("xtask-cli-test-no-game");
    path.to_string_lossy().into_owned()
}

#[test]
fn no_command_prints_the_usage_and_fails() {
    let (ok, err) = xtask(&[]);
    assert!(!ok);
    assert!(err.contains("usage:"), "{err}");
}

#[test]
fn check_links_passes_on_the_repository() {
    let (ok, err) = xtask(&["check-links"]);
    assert!(ok, "{err}");
    assert!(err.contains("check-links: ok"), "{err}");
}

#[test]
fn check_links_takes_no_arguments() {
    let (ok, err) = xtask(&["check-links", "extra"]);
    assert!(!ok);
    assert!(err.contains("usage:"), "{err}");
}

#[test]
fn check_docs_passes_on_the_repository() {
    let (ok, err) = xtask(&["check-docs"]);
    assert!(ok, "{err}");
    assert!(err.contains("check-docs: ok"), "{err}");
}

#[test]
fn check_docs_takes_no_arguments() {
    let (ok, err) = xtask(&["check-docs", "extra"]);
    assert!(!ok);
    assert!(err.contains("usage:"), "{err}");
}

#[test]
fn ticket_lint_passes_on_the_repository() {
    let (ok, err) = xtask(&["ticket-lint", "--game-repo", &no_game()]);
    assert!(ok, "{err}");
    assert!(err.contains("ticket-lint: ok"), "{err}");
    assert!(err.contains("warning: "), "game# ids are skipped: {err}");
}

/// `ticket-lint` on the repository, with `GITHUB_ACTIONS` set to `true` or
/// removed (CI sets it), and what it printed on stdout.
fn ticket_lint_stdout(on_github: bool) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
    command
        .args(["ticket-lint", "--game-repo", &no_game()])
        .current_dir(std::env::temp_dir());
    if on_github {
        command.env("GITHUB_ACTIONS", "true");
    } else {
        command.env_remove("GITHUB_ACTIONS");
    }
    let output = command
        .output()
        .unwrap_or_else(|e| panic!("the xtask binary does not run: {e}"));
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn ticket_lint_annotates_its_warnings_on_github_actions() {
    let out = ticket_lint_stdout(true);
    assert!(
        out.lines().any(|l| l.starts_with("::warning::")),
        "game# ids are skipped: {out}"
    );
    assert!(out.lines().all(|l| l.starts_with("::warning::")), "{out}");
}

#[test]
fn ticket_lint_prints_no_annotation_off_github_actions() {
    let out = ticket_lint_stdout(false);
    assert!(!out.contains("::warning::"), "{out}");
}

#[test]
fn ticket_lint_accepts_a_branch_that_names_no_ticket() {
    let (ok, err) = xtask(&[
        "ticket-lint",
        "--pr-branch",
        "main",
        "--game-repo",
        &no_game(),
    ]);
    assert!(ok, "{err}");
}

#[test]
fn ticket_lint_fails_on_a_branch_naming_an_unknown_ticket() {
    let (ok, err) = xtask(&[
        "ticket-lint",
        "--pr-branch",
        "t9999-ghost",
        "--game-repo",
        &no_game(),
    ]);
    assert!(!ok);
    assert!(err.contains("no ticket with that id exists"), "{err}");
    assert!(err.contains("ticket-lint: 1 error(s)"), "{err}");
}

#[test]
fn ticket_lint_rejects_an_unknown_option() {
    for args in [
        &["ticket-lint", "--bogus", "x"][..],
        &["ticket-lint", "--pr-branch"],
    ] {
        let (ok, err) = xtask(args);
        assert!(!ok, "{args:?}");
        assert!(err.contains("usage:"), "{args:?}: {err}");
    }
}

#[test]
fn check_no_std_passes_on_the_repository() {
    let (ok, err) = xtask(&["check-no-std"]);
    assert!(ok, "{err}");
    assert!(err.contains("check-no-std: ok"), "{err}");
}

#[test]
fn check_no_std_takes_no_arguments() {
    let (ok, err) = xtask(&["check-no-std", "extra"]);
    assert!(!ok);
    assert!(err.contains("usage:"), "{err}");
}

#[test]
fn check_layers_passes_on_the_repository() {
    let (ok, err) = xtask(&["check-layers"]);
    assert!(ok, "{err}");
    assert!(err.contains("check-layers: ok"), "{err}");
}

#[test]
fn check_layers_takes_no_arguments() {
    let (ok, err) = xtask(&["check-layers", "extra"]);
    assert!(!ok);
    assert!(err.contains("usage:"), "{err}");
}

#[test]
fn api_refuses_an_unknown_option() {
    let (ok, err) = xtask(&["api", "--bless"]);
    assert!(!ok);
    assert!(err.contains("usage:"), "{err}");
}
