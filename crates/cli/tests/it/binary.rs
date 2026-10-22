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
fn outside_a_project_it_says_so_and_exits_3() {
    let dir = scratch("no-project");
    let output = shuyi(&dir, &["check-keys"]);
    assert_eq!(output.status.code(), Some(3));
    let said = stderr(&output);
    assert!(
        said.starts_with("check-keys: error[SH1001]: no shuyi.ron in "),
        "{said}"
    );
    assert!(said.contains("\n  see: shuyi explain SH1001\n"), "{said}");
    assert!(said.contains("\nusage: shuyi <command>\n"), "{said}");
    // An op asked for JSON answers it, and nothing else.
    let output = shuyi(&dir, &["check", "--json"]);
    assert_eq!(output.status.code(), Some(3));
    let answer = json(&output);
    assert_eq!(
        (answer["op"].as_str(), answer["ok"].as_bool()),
        (Some("check"), Some(false))
    );
    assert_eq!(answer["code"], "SH1001");
    assert!(answer["help"].as_str().unwrap().contains("shuyi new"));
    assert!(stderr(&output).is_empty(), "{}", stderr(&output));
    // The same op for people says it, then the usage.
    let output = shuyi(&dir, &["check"]);
    assert_eq!(output.status.code(), Some(3));
    let said = stderr(&output);
    assert!(said.starts_with("check: error[SH1001]: "), "{said}");
    assert!(said.contains("\nusage: shuyi <command>\n"), "{said}");
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
    assert!(stderr(&output).starts_with("tools: error[SH0001]: unknown command: bogus\n"));
    // A command whose part the project lacks.
    let output = shuyi(&dir, &["check-keys"]);
    assert_eq!(output.status.code(), Some(3));
    assert!(
        stderr(&output).starts_with("check-keys: error[SH1002]: shuyi.ron has no `check_keys`\n"),
        "{}",
        stderr(&output)
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
        assert_eq!(output.status.code(), Some(3), "{command}");
        assert!(
            stderr(&output).contains("error[SH1002]: shuyi.ron names no game tool program"),
            "{command}"
        );
    }
    // Named, it is run with cargo: here there is no such package.
    fs::write(
        dir.join("shuyi.ron"),
        r#"(game: (package: "no-such-game"))"#,
    )
    .unwrap();
    // The contract's operations that run the game go there too.
    for args in [
        &["lang-status", "xx"][..],
        &["check"],
        &["check", "--target", "all"],
        &["run", "--json"],
    ] {
        let output = shuyi(&dir, args);
        assert_ne!(output.status.code(), Some(0), "{args:?}");
        assert!(
            stderr(&output).contains("Cargo.toml"),
            "{args:?}: {}",
            stderr(&output)
        );
    }
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
    glyph_tiles: (font: "font", palette: "palette.ron", out: "tiles.png", tile: 24, columns: 4),
)"#,
    )
    .unwrap();
    for (args, said) in [
        (
            &["font-atlas", "one"][..],
            "usage: shuyi font-atlas <font.bdf>... <out-dir>",
        ),
        (&["glyph-tiles", "one"], "usage: shuyi glyph-tiles"),
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
    let output = shuyi(&dir, &["glyph-tiles"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).starts_with("glyph-tiles: reading "),
        "{}",
        stderr(&output)
    );
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

/// `output`'s stdout as a JSON answer.
fn json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("not JSON ({e}): {}", stdout(output)))
}

#[test]
fn new_doctor_explain_and_schema_need_no_project() {
    let dir = scratch("no-project-ops");
    let output = shuyi(&dir, &["schema"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).lines().any(|l| l == "run.output"));
    let output = shuyi(&dir, &["schema", "run.output"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(json(&output)["$schema"].is_string());
    assert_eq!(shuyi(&dir, &["schema", "nope"]).status.code(), Some(2));
    assert_eq!(shuyi(&dir, &["schema", "a", "b"]).status.code(), Some(2));
    let output = shuyi(&dir, &["schema", "--out", "s"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(fs::read_dir(dir.join("s")).unwrap().count(), 36);

    let output = shuyi(&dir, &["new", "blank", "g", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    let answer = json(&output);
    assert_eq!(
        (answer["op"].as_str(), answer["ok"].as_bool()),
        (Some("new"), Some(true))
    );
    assert_eq!(answer["result"]["crate_name"], "g");
    assert!(dir.join("g/crates/game/Cargo.toml").is_file());
    let output = shuyi(&dir, &["new", "blank", "g"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).starts_with("new: error[SH0001]: g is not empty\n"));
    let output = shuyi(&dir, &["new", "--json"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        json(&output)["error"],
        "a <template> and a <dir>, nothing else"
    );
    let output = shuyi(&dir, &["new", "grid"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("usage: shuyi new <template> <dir>"));

    let output = shuyi(&dir, &["doctor", "--json"]);
    let answer = json(&output);
    assert_eq!(answer["op"], "doctor");
    assert_eq!(answer["result"]["checks"][0]["name"], "rust");
    // 0 on a complete toolchain, 3 with its code if a check failed.
    match output.status.code() {
        Some(0) => assert!(answer["code"].is_null()),
        Some(3) => assert_eq!(answer["code"], "SH1003"),
        other => panic!("doctor exited {other:?}"),
    }
    assert_eq!(shuyi(&dir, &["doctor", "x"]).status.code(), Some(2));
    let output = shuyi(&dir, &["doctor"]);
    assert!(
        stdout(&output).starts_with("doctor: host "),
        "{}",
        stdout(&output)
    );

    let output = shuyi(&dir, &["explain"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(
        stdout(&output).starts_with("SH0001  usage "),
        "{}",
        stdout(&output)
    );
    let output = shuyi(&dir, &["explain", "SH2003"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).starts_with("# SH2003: A file isn't there\n"));
    let output = shuyi(&dir, &["explain", "SH9999", "--json"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(json(&output)["code"], "SH0001");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn import_outside_a_project_reads_the_engines_standard_kinds() {
    let dir = scratch("import");
    fs::write(dir.join("colors.ron"), "{ \"text\": \"#ffffff\" }").unwrap();
    let output = shuyi(&dir, &["import", "colors.ron", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    let answer = json(&output);
    let kinds: Vec<&str> = answer["result"]["read_as"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["kind"].as_str())
        .collect();
    assert!(kinds.contains(&"palette"), "{kinds:?}");
    let limits = answer["limits"].as_array().unwrap();
    assert!(
        limits
            .iter()
            .any(|l| l.as_str().unwrap().starts_with("no project here"))
    );
    let output = shuyi(
        &dir,
        &["import", "colors.ron", "--kind", "palette", "--print"],
    );
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).starts_with("import: colors.ron is palette: 1 color\n"));
    let output = shuyi(&dir, &["import", "missing.ron"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).starts_with("import: error[SH3004]: missing.ron: "));
    assert_eq!(shuyi(&dir, &["import"]).status.code(), Some(2));
    // In a project with no game tool program, the same.
    fs::write(dir.join("shuyi.ron"), "()").unwrap();
    let output = shuyi(&dir, &["import", "colors.ron", "--json"]);
    let limits = json(&output)["limits"].clone();
    assert!(
        limits.to_string().contains("names no game tool program"),
        "{limits}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_ops_that_run_the_game_say_they_need_its_tool_program() {
    let dir = scratch("no-game-ops");
    fs::write(dir.join("shuyi.ron"), "()").unwrap();
    for op in ["check", "run", "scene", "replay", "bots", "bake"] {
        let output = shuyi(&dir, &[op, "--json"]);
        assert_eq!(output.status.code(), Some(3), "{op}");
        let answer = json(&output);
        assert_eq!(answer["op"], op);
        assert_eq!(answer["code"], "SH1002");
        assert!(
            answer["error"]
                .as_str()
                .unwrap()
                .contains("names no game tool program"),
            "{answer}"
        );
        let output = shuyi(&dir, &[op]);
        assert!(
            stderr(&output).starts_with(&format!("{op}: error[SH1002]: shuyi.ron names no game"))
        );
        assert_eq!(output.status.code(), Some(3), "{op}");
    }
    fs::remove_dir_all(&dir).unwrap();
}
