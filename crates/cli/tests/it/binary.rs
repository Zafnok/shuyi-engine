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
    let usage = stdout(&output);
    assert!(usage.starts_with("usage: tools <command>\n"), "{usage}");
    // The game's bots play through `playtest`; `bots` is gone.
    assert!(usage.contains("\n  playtest [--stage ID]"), "{usage}");
    assert!(!usage.contains("\n  bots "), "{usage}");
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
    for command in ["frame-png", "lang-status", "mcp-host"] {
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
        &["mcp-host"],
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
            "usage: shuyi font-atlas [--no-text] [--wide [<set>=]<wide.bdf>]... [--bold <bold.bdf>] <font.bdf>... <out-dir> (one --wide a set)",
        ),
        (
            &["font-atlas", "--no-text", "one"],
            "usage: shuyi font-atlas [--no-text] [--wide [<set>=]<wide.bdf>]... [--bold <bold.bdf>] <font.bdf>... <out-dir> (one --wide a set)",
        ),
        (
            &["font-atlas", "one.bdf", "out", "--wide"],
            "usage: shuyi font-atlas [--no-text] [--wide [<set>=]<wide.bdf>]... [--bold <bold.bdf>] <font.bdf>... <out-dir> (one --wide a set)",
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
    // With no game tool program, the text isn't read, and that is said.
    assert_eq!(
        stdout(&output).trim(),
        "font-atlas: keeping the ranges and the required glyphs only, not the text's \
         characters: shuyi.ron names no game tool program (`game`)"
    );
    let output = shuyi(&dir, &["font-atlas", "--no-text", "no/such.bdf", "out"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).trim().ends_with("characters: --no-text"));
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

/// The fields of `value` that `schema` (at `node`, its `$ref`s read from
/// its `$defs`) has no property for, each by its path from `at`.
fn unknown_fields(
    schema: &serde_json::Value,
    node: &serde_json::Value,
    value: &serde_json::Value,
    at: &str,
) -> Vec<String> {
    if let Some(reference) = node["$ref"].as_str() {
        let name = reference.trim_start_matches("#/$defs/");
        return unknown_fields(schema, &schema["$defs"][name], value, at);
    }
    for key in ["anyOf", "oneOf"] {
        if let Some(branches) = node[key].as_array() {
            return branches
                .iter()
                .map(|branch| unknown_fields(schema, branch, value, at))
                .min_by_key(Vec::len)
                .unwrap_or_default();
        }
    }
    match value {
        serde_json::Value::Object(fields) => fields
            .iter()
            .flat_map(|(name, field)| {
                let path = format!("{at}.{name}");
                match node["properties"].get(name) {
                    Some(property) => unknown_fields(schema, property, field, &path),
                    None if node["additionalProperties"].is_object() => {
                        unknown_fields(schema, &node["additionalProperties"], field, &path)
                    }
                    None => vec![path],
                }
            })
            .collect(),
        serde_json::Value::Array(items) => items
            .iter()
            .enumerate()
            .flat_map(|(n, item)| {
                unknown_fields(schema, &node["items"], item, &format!("{at}[{n}]"))
            })
            .collect(),
        _ => Vec::new(),
    }
}

#[test]
fn new_lists_its_questions_as_its_schema_says_and_makes_nothing() {
    let dir = scratch("new-questions");
    let output = shuyi(&dir, &["new", "demo", "--questions", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    assert!(!dir.join("demo").exists());
    let answer = json(&output);
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/new.questions.json");
    let schema: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
    assert_eq!(
        unknown_fields(&schema, &schema, &answer, "answer"),
        Vec::<String>::new()
    );
    assert_eq!(answer["op"], "new");
    let questions = answer["result"]["questions"].as_array().unwrap();
    let ids: Vec<&str> = questions
        .iter()
        .map(|q| q["id"].as_str().unwrap())
        .collect();
    // The saves' folder right after the id, the id its default.
    assert_eq!(ids[..4], ["dir", "name", "id", "storage"]);
    assert_eq!(questions[3]["default"], questions[2]["default"]);
    assert_eq!(questions[3]["flag"], "--storage");
    // Every concern of the catalog but `time`, each answer with its status
    // and, if it isn't ready, its tickets.
    let catalog = shuyi_cli::catalog::Concerns::embedded().unwrap();
    for concern in &catalog.concerns {
        assert_eq!(ids.contains(&concern.id.as_str()), concern.id != "time");
    }
    let bodies = questions.iter().find(|q| q["id"] == "bodies").unwrap();
    assert!(bodies["skipped"].is_string());
    let procedural = bodies["answers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "procedural")
        .unwrap();
    assert_eq!(procedural["status"], "parked");
    assert_eq!(procedural["tickets"][0], "2701");
    // The GitHub questions follow the targets; only `github` has an
    // `agent`, which says an agent never answers yes itself.
    let at = ids.iter().position(|id| *id == "targets").unwrap();
    assert_eq!(
        ids[at + 1..],
        [
            "github",
            "visibility",
            "keep-private",
            "private",
            "private-folder",
            "assets-visibility"
        ]
    );
    let agents: Vec<&str> = questions
        .iter()
        .filter_map(|q| q["agent"].as_str())
        .collect();
    assert_eq!(agents.len(), 1);
    assert!(
        agents[0].contains("never answer yes yourself"),
        "{}",
        agents[0]
    );
    assert_eq!(questions[at + 1]["default"][0], "no");
    // For people, the same.
    let output = shuyi(
        &dir,
        &[
            "new",
            "demo",
            "--questions",
            "--perspective",
            "top-down-grid",
        ],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("How do bodies move? (--bodies, now none)\n  not asked: "),
        "{}",
        stdout(&output)
    );
    assert!(!dir.join("demo").exists());
    fs::remove_dir_all(&dir).unwrap();
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
    assert_eq!(fs::read_dir(dir.join("s")).unwrap().count(), 50);

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
    assert!(
        stderr(&output).starts_with("new: error[SH0001]: g is not empty (it would write over ")
    );
    let output = shuyi(&dir, &["new", "--json"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(json(&output)["error"], "a <dir>, nothing else");
    let output = shuyi(&dir, &["new", "a", "b", "c"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("usage: shuyi new <dir> [--template NAME]"));

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
    for op in ["check", "run", "scene", "replay", "playtest", "bake"] {
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

#[test]
fn mcp_ready_builds_what_the_server_needs_and_exits() {
    let dir = scratch("mcp-ready");
    // Outside a project there is only the server, and it is built.
    let output = shuyi(&dir, &["mcp", "--ready"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        format!("mcp: ready (contract {})\n", shuyi_cli::contract::VERSION)
    );
    assert_eq!(shuyi(&dir, &["mcp", "--ready", "x"]).status.code(), Some(2));
    // A game tool program that doesn't build is the build's error, and 1.
    fs::write(
        dir.join("shuyi.ron"),
        r#"(game: (package: "no-such-game"))"#,
    )
    .unwrap();
    let output = shuyi(&dir, &["mcp", "--ready"]);
    assert_eq!(output.status.code(), Some(1));
    let said = stderr(&output);
    assert!(said.contains("Cargo.toml"), "{said}");
    assert!(
        said.contains("mcp: the game's tool program didn't build"),
        "{said}"
    );
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    // One that builds is built, and the server is ready.
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"tiny-game\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
    fs::write(dir.join("shuyi.ron"), r#"(game: (package: "tiny-game"))"#).unwrap();
    // Built in its own `target/`, even where the tests run with a shared
    // `CARGO_TARGET_DIR` (the eval's, grading an example of this workspace).
    let output = Command::new(env!("CARGO_BIN_EXE_shuyi"))
        .args(["mcp", "--ready"])
        .current_dir(&dir)
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .unwrap_or_else(|e| panic!("spawn shuyi: {e}"));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).starts_with("mcp: ready (contract "));
    assert!(dir.join("target").is_dir(), "the program was built");
    fs::remove_dir_all(&dir).unwrap();
}

#[cfg(windows)]
#[test]
fn a_running_server_moves_its_program_aside_so_a_build_can_write_a_fresh_one() {
    use std::io::{BufRead, BufReader, Write};
    use std::process::Stdio;

    // A program as cargo leaves it: in `debug/`, hard-linked from `deps/`.
    let dir = scratch("mcp-aside").join("debug");
    fs::create_dir_all(dir.join("deps")).unwrap();
    let twin = dir.join("deps/shuyi-0123abcd.exe");
    fs::copy(env!("CARGO_BIN_EXE_shuyi"), &twin).unwrap();
    let exe = dir.join("shuyi.exe");
    fs::hard_link(&twin, &exe).unwrap();
    let mut server = Command::new(&exe)
        .arg("mcp")
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = server.stdin.take().unwrap();
    let mut stdout = BufReader::new(server.stdout.take().unwrap());
    let mut ask = |id: u32, method: &str| {
        let request = serde_json::json!({ "jsonrpc": "2.0", "id": id, "method": method,
            "params": { "protocolVersion": "2025-06-18", "capabilities": {},
                        "clientInfo": { "name": "the test", "version": "1" } } });
        writeln!(stdin, "{request}").unwrap();
        stdin.flush().unwrap();
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        serde_json::from_str::<serde_json::Value>(&line).unwrap()
    };
    // It answers once it has moved its program.
    assert_eq!(ask(1, "ping")["id"], 1);
    // A build writes new files at both names while it runs.
    fs::write(&exe, b"a fresh build").unwrap();
    fs::write(&twin, b"a fresh build").unwrap();
    assert_eq!(fs::read_dir(dir.join(".shuyi-running")).unwrap().count(), 2);
    let init = ask(2, "initialize");
    assert_eq!(init["result"]["serverInfo"]["name"], "shuyi", "{init}");
    server.kill().unwrap();
    server.wait().unwrap();
    fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}

/// Whether a process runs the program `exe`: on Windows its file can't
/// be deleted while one does (and is deleted once none does); elsewhere
/// `ps` lists it.
fn runs(exe: &Path) -> bool {
    if cfg!(windows) {
        exe.exists() && fs::remove_file(exe).is_err()
    } else {
        let listed = Command::new("ps")
            .args(["-A", "-o", "args="])
            .output()
            .unwrap_or_else(|e| panic!("ps: {e}"));
        let exe = exe.display().to_string();
        stdout(&listed)
            .lines()
            .any(|line| line.trim().starts_with(&exe))
    }
}

#[test]
fn a_project_in_another_workspace_is_answered_by_its_own_server_which_ends_with_the_server() {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    // A copy of the toy game in a workspace of its own, whose `.mcp.json`
    // runs a copy of the program, so the test can tell its process.
    let game = scratch("mcp-forward-game");
    fs::write(
        game.join("Cargo.toml"),
        "[package]\nname = \"toy-copy\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n",
    )
    .unwrap();
    fs::create_dir_all(game.join("src")).unwrap();
    fs::write(game.join("src/lib.rs"), "").unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("toy/shuyi.ron"),
        game.join("shuyi.ron"),
    )
    .unwrap();
    fs::create_dir_all(game.join("bin")).unwrap();
    let exe = game
        .join("bin")
        .join(Path::new(env!("CARGO_BIN_EXE_shuyi")).file_name().unwrap());
    fs::copy(env!("CARGO_BIN_EXE_shuyi"), &exe).unwrap();
    let entry = serde_json::json!({ "mcpServers": { "shuyi": {
        "command": exe.display().to_string(), "args": ["mcp"] } } });
    fs::write(game.join(".mcp.json"), entry.to_string()).unwrap();

    let cwd = scratch("mcp-forward-cwd");
    let mut server = Command::new(env!("CARGO_BIN_EXE_shuyi"))
        .arg("mcp")
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = server.stdin.take().unwrap();
    let mut stdout = BufReader::new(server.stdout.take().unwrap());
    let mut stderr = server.stderr.take().unwrap();
    let said = std::thread::spawn(move || {
        let mut said = String::new();
        let _ = stderr.read_to_string(&mut said);
        said
    });
    // It owns the server's stdin: dropping it closes it.
    let mut send = move |message: serde_json::Value| {
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    };
    let mut read = || {
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        serde_json::from_str::<serde_json::Value>(&line).unwrap()
    };
    send(
        serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "protocolVersion": "2025-11-25", "capabilities": { "roots": {} },
                    "clientInfo": { "name": "the test", "version": "1" } } }),
    );
    assert_eq!(read()["id"], 1);
    send(serde_json::json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
    let asked = read();
    assert_eq!(asked["method"], "roots/list", "{asked}");
    let path = game.display().to_string().replace('\\', "/");
    let uri = if path.starts_with('/') {
        format!("file://{path}")
    } else {
        format!("file:///{path}")
    };
    send(serde_json::json!({ "jsonrpc": "2.0", "id": asked["id"],
        "result": { "roots": [{ "uri": uri }] } }));
    send(
        serde_json::json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": { "name": "explain", "arguments": { "code": "SH3005" } } }),
    );
    // The same program lists the same tools: no notification comes first.
    let answer = read();
    assert_eq!(answer["id"], 2, "{answer}");
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    assert!(runs(&exe), "the project's own server runs");

    drop(send);
    server.wait().unwrap();
    let said = said.join().unwrap();
    assert!(
        said.contains("is answered by the project's own server"),
        "{said}"
    );
    // The server has ended, and it killed and waited for its project's own
    // server before it did (`Process`'s drop), so no timing of either is
    // left to wait for: on Linux `runs` is already false. What the loop
    // waits for is Windows letting go of the ended program's file, which a
    // scan of the new `.exe` can hold for a moment. 10 seconds is the bound
    // for that alone; ticket 2063 ran this test 260 times under load, on
    // Windows, on Linux and on Linux pinned to 2 cores, and it never failed.
    let ended = Instant::now();
    while runs(&exe) {
        assert!(
            ended.elapsed() < Duration::from_secs(10),
            "the project's own server outlived the server"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    fs::remove_dir_all(&game).unwrap();
    fs::remove_dir_all(&cwd).unwrap();
}
