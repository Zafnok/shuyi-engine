//! The `shuyi` binary as a subprocess, so `main` itself (which reads the
//! process's argv and working folder) is exercised.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use crate::scratch;

/// `shuyi <args>` run in `dir`.
fn shuyi(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_shuyi"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("spawn shuyi: {e}"))
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn outside_a_project_it_says_so_and_exits_2() {
    let dir = scratch("no-project");
    let output = shuyi(&dir, &["check-keys"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("no shuyi.ron in "),
        "{}",
        stderr(&output)
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_project_is_found_from_a_folder_below_it() {
    let dir = scratch("below");
    fs::write(dir.join("shuyi.ron"), r#"(command: "tools")"#).unwrap();
    fs::create_dir_all(dir.join("a/b")).unwrap();
    let output = shuyi(&dir.join("a/b"), &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stdout(&output).starts_with("usage: tools <command>\n"),
        "{}",
        stdout(&output)
    );
    let output = shuyi(&dir, &["bogus"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).starts_with("unknown command: bogus\n"));
    // A command whose part the project lacks.
    let output = shuyi(&dir, &["check-keys"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr(&output).trim(),
        "check-keys: shuyi.ron has no `check_keys`"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn check_keys_and_check_text_scan_the_projects_folders() {
    let dir = scratch("checks");
    fs::write(
        dir.join("shuyi.ron"),
        r#"(
    check_keys: (rust: ["src"], hint: "use an action"),
    check_text: (dir: "src", hint: "use a key"),
)"#,
    )
    .unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/ok.rs"), "fn f() {}\n").unwrap();
    let output = shuyi(&dir, &["check-keys"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "check-keys: OK\n");
    let output = shuyi(&dir, &["check-text"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "check-text: 0 player-facing string literal(s) in src (at most 0 allowed)\n"
    );

    fs::write(
        dir.join("src/bad.rs"),
        "fn f() { draw(\"Press f\", Key::F); }\n",
    )
    .unwrap();
    let output = shuyi(&dir, &["check-keys"]);
    assert_eq!(output.status.code(), Some(1));
    let said = stderr(&output);
    assert!(
        said.starts_with("check-keys: 2 hard-coded key(s)\n"),
        "{said}"
    );
    assert!(said.contains("  src/bad.rs:1: names a key or button with `Key::` — use an action"));
    let output = shuyi(&dir, &["check-text"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).contains("src/bad.rs: 1\n  1: \"Press f\"\n"));
    assert!(stderr(&output).contains("over the 0 allowed — use a key"));
    let output = shuyi(&dir, &["check-keys", "x"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stderr(&output).trim(), "usage: shuyi check-keys");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_commands_that_run_the_game_need_its_tool_program() {
    let dir = scratch("no-game");
    fs::write(dir.join("shuyi.ron"), "()").unwrap();
    for command in ["frame-png", "lang-status"] {
        let output = shuyi(&dir, &[command, "x"]);
        assert_eq!(output.status.code(), Some(2), "{command}");
        assert!(
            stderr(&output).contains("names no game tool program"),
            "{command}"
        );
    }
    // Named, it is run with cargo: here there is no such package.
    fs::write(
        dir.join("shuyi.ron"),
        r#"(game: (package: "no-such-game"))"#,
    )
    .unwrap();
    let output = shuyi(&dir, &["lang-status", "xx"]);
    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("Cargo.toml"),
        "{}",
        stderr(&output)
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn bad_arguments_print_the_usage() {
    let dir = scratch("usage");
    fs::write(
        dir.join("shuyi.ron"),
        r#"(
    font_atlas: (required_glyphs: "?"),
    web: (package: "game", bin: "game", features: ["debug"]),
    private_assets: (checkout: "private", pin_file: "private.rev", folders: ["game"]),
)"#,
    )
    .unwrap();
    for (args, said) in [
        (
            &["font-atlas", "one"][..],
            "usage: shuyi font-atlas <font.bdf>... <out-dir>",
        ),
        (
            &["web", "--bogus"],
            "usage: shuyi web [--release] [--debug]",
        ),
        (
            &["private-assets", "--library"],
            "usage: shuyi private-assets [--pin]",
        ),
    ] {
        let output = shuyi(&dir, args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert_eq!(stderr(&output).trim(), said);
    }
    let output = shuyi(&dir, &["font-atlas", "no/such.bdf", "out"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).starts_with("font-atlas: reading no/such.bdf"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_step_that_fails_exits_1_and_says_why() {
    let dir = scratch("fails");
    fs::write(
        dir.join("shuyi.ron"),
        r#"(
    web: (package: "game", bin: "game", dist: "dist"),
    private_assets: (checkout: "private", pin_file: "private.rev", folders: ["game"]),
)"#,
    )
    .unwrap();
    // No Cargo.toml here: the build fails before compiling anything.
    let output = shuyi(&dir, &["web"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("web: cargo build exited with "),
        "{}",
        stderr(&output)
    );
    let output = shuyi(&dir, &["private-assets", "--pin"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output).trim(),
        "private-assets: private/ isn't there: run `shuyi private-assets` first"
    );
    fs::remove_dir_all(&dir).unwrap();
}
