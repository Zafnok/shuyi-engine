//! Registering the server: the entries, the files kept, the agents found,
//! the diff, and what is written.

use super::*;

/// A fresh empty folder under the system temp folder.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-mcp-install-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Claude Code.
fn claude() -> &'static Agent {
    &AGENTS[0]
}

/// A ready run that must not happen.
fn no_ready(_: &str) -> Result<Duration, String> {
    panic!("no ready run")
}

#[test]
fn an_entry_runs_the_command_with_mcp() {
    assert_eq!(
        claude().entry("shuyi"),
        json!({ "command": "shuyi", "args": ["mcp"] })
    );
    assert_eq!(
        claude().entry("cargo run -q -p shuyi-cli --"),
        json!({ "command": "cargo", "args": ["run", "-q", "-p", "shuyi-cli", "--", "mcp"] })
    );
    let vscode = AGENTS.iter().find(|a| a.name == "VS Code").unwrap();
    assert_eq!(
        vscode.entry("cargo shuyi"),
        json!({ "type": "stdio", "command": "cargo", "args": ["shuyi", "mcp"] })
    );
}

#[test]
fn the_file_keeps_everything_else_in_it() {
    let new = claude().file_text(None, "shuyi").unwrap();
    assert_eq!(
        new,
        "{\n  \"mcpServers\": {\n    \"shuyi\": {\n      \"args\": [\n        \"mcp\"\n      ],\n      \"command\": \"shuyi\"\n    }\n  }\n}\n"
    );
    let old = r#"{"mcpServers": {"other": {"command": "x"}}, "theme": "dark"}"#;
    let new: Value =
        serde_json::from_str(&claude().file_text(Some(old), "shuyi").unwrap()).unwrap();
    assert_eq!(new["mcpServers"]["other"]["command"], "x");
    assert_eq!(new["mcpServers"]["shuyi"]["command"], "shuyi");
    assert_eq!(new["theme"], "dark");
    // An entry of an older command is replaced.
    let again = claude()
        .file_text(Some(&new.to_string()), "cargo shuyi")
        .unwrap();
    assert!(again.contains("\"cargo\""));
    assert!(!again.contains("\"command\": \"shuyi\""));
    assert_eq!(
        claude().file_text(Some("  "), "shuyi").unwrap(),
        claude().file_text(None, "shuyi").unwrap()
    );
    for (old, said) in [
        ("{nope", "not JSON"),
        ("[]", "not a JSON object"),
        (r#"{"mcpServers": 3}"#, "`mcpServers` is not an object"),
    ] {
        let error = claude().file_text(Some(old), "shuyi").unwrap_err();
        assert!(
            error.starts_with(".mcp.json: ") && error.contains(said),
            "{error}"
        );
    }
}

#[test]
fn an_agent_is_found_by_its_folder_or_its_program() {
    let dir = scratch("found");
    let (root, home, bin) = (dir.join("game"), dir.join("home"), dir.join("bin"));
    for folder in [&root, &home, &bin] {
        fs::create_dir_all(folder).unwrap();
    }
    let machine = Machine {
        home: Some(home.clone()),
        path: vec![bin.clone()],
    };
    let found = |machine: &Machine| -> Vec<&str> {
        AGENTS
            .iter()
            .filter(|a| a.found(&root, machine))
            .map(|a| a.name)
            .collect()
    };
    assert_eq!(found(&machine), ["Claude Code"]);
    assert_eq!(found(&Machine::default()), ["Claude Code"]);
    fs::create_dir_all(root.join(".cursor")).unwrap();
    fs::create_dir_all(home.join(".gemini")).unwrap();
    fs::write(bin.join("code.cmd"), "").unwrap();
    assert_eq!(
        found(&machine),
        ["Claude Code", "Cursor", "VS Code", "Gemini CLI"]
    );
    let this = Machine::this();
    assert!(!this.path.is_empty() && this.home.is_some(), "{this:?}");
    fs::remove_dir_all(&dir).unwrap();
}

/// The length of the longest common subsequence of `a` and `b`, counted
/// the slow way.
fn common(a: &[&str], b: &[&str]) -> usize {
    match (a.split_first(), b.split_first()) {
        (Some((x, a_rest)), Some((y, b_rest))) if x == y => 1 + common(a_rest, b_rest),
        (Some((_, a_rest)), Some((_, b_rest))) => common(a_rest, b).max(common(a, b_rest)),
        _ => 0,
    }
}

/// Every text of up to `lines` lines of `a` and `b`.
fn texts(lines: usize) -> Vec<String> {
    let mut all = vec![String::new()];
    let mut last = vec![Vec::<&str>::new()];
    for _ in 0..lines {
        let next: Vec<Vec<&str>> = last
            .iter()
            .flat_map(|text| {
                ["a", "b"].map(|line| {
                    let mut longer = text.clone();
                    longer.push(line);
                    longer
                })
            })
            .collect();
        all.extend(next.iter().map(|text| {
            text.join(
                "
",
            )
        }));
        last = next;
    }
    all
}

#[test]
fn the_diff_keeps_each_side_and_the_most_lines_they_share() {
    let all = texts(4);
    for old in &all {
        for new in &all {
            let out = diff(old, new);
            let side = |drop: &str| -> Vec<&str> {
                out.lines()
                    .filter(|line| !line.starts_with(drop))
                    .map(|line| &line[2..])
                    .collect()
            };
            let (a, b): (Vec<&str>, Vec<&str>) = (old.lines().collect(), new.lines().collect());
            assert_eq!(side("+ "), a, "{old:?} {new:?}: {out}");
            assert_eq!(side("- "), b, "{old:?} {new:?}: {out}");
            let kept = out.lines().filter(|line| line.starts_with("  ")).count();
            assert_eq!(kept, common(&a, &b), "{old:?} {new:?}: {out}");
        }
    }
}

#[test]
fn only_a_yes_is_a_yes() {
    for (line, yes) in [
        (
            "y
", true,
        ),
        (
            "Y
", true,
        ),
        (
            "yes
", true,
        ),
        (
            "n
", false,
        ),
        (
            "
", false,
        ),
        ("", false),
        (
            "yep
", false,
        ),
    ] {
        assert_eq!(said_yes(&mut line.as_bytes()), yes, "{line:?}");
    }
}

#[test]
fn the_diff_marks_what_each_side_has() {
    assert_eq!(diff("a\nb\nc", "a\nx\nc\nd"), "  a\n- b\n+ x\n  c\n+ d\n");
    assert_eq!(diff("", "a"), "+ a\n");
    assert_eq!(diff("a", ""), "- a\n");
    assert_eq!(diff("a", "a"), "  a\n");
}

#[test]
fn its_arguments_are_a_dry_run_yes_and_a_command() {
    let args = |items: &[&str]| -> Vec<String> { items.iter().map(|s| (*s).to_owned()).collect() };
    assert_eq!(parse_args(&[]).unwrap(), Options::default());
    assert_eq!(
        parse_args(&args(&["--dry-run", "--yes", "--command", "cargo shuyi"])).unwrap(),
        Options {
            dry_run: true,
            yes: true,
            command: Some("cargo shuyi".to_owned())
        }
    );
    assert!(parse_args(&args(&["--command"])).is_err());
    assert!(parse_args(&args(&["--command", " "])).is_err());
    assert!(parse_args(&args(&["--force"])).is_err());
    let dir = scratch("args");
    assert_eq!(main(&args(&["--force"]), &dir), 2);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_dry_run_writes_nothing_and_prints_each_file() {
    let dir = scratch("dry");
    let options = Options {
        dry_run: true,
        ..Options::default()
    };
    let mut never = |_: &str| -> bool { panic!("a dry run asks nothing") };
    let out = install(
        &dir,
        &options,
        &Machine::default(),
        "shuyi",
        &mut never,
        &mut no_ready,
    )
    .unwrap();
    let expected = format!(
        "mcp install: would write .mcp.json (Claude Code):\n{}",
        claude().file_text(None, "shuyi").unwrap()
    );
    assert_eq!(out, expected);
    assert!(!dir.join(".mcp.json").exists());
    // Through `main`, in a project: its own command.
    fs::write(dir.join("shuyi.ron"), r#"(command: "cargo shuyi")"#).unwrap();
    assert_eq!(main(&["--dry-run".to_owned()], &dir), 0);
    assert!(!dir.join(".mcp.json").exists());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn it_asks_writes_keeps_a_backup_and_knows_when_it_is_done() {
    let dir = scratch("write");
    fs::write(
        dir.join(".mcp.json"),
        r#"{"mcpServers": {"other": {"command": "x"}}}"#,
    )
    .unwrap();
    let machine = Machine::default();
    let options = Options::default();
    let mut asked = Vec::new();
    let mut no = |question: &str| {
        asked.push(question.to_owned());
        false
    };
    let out = install(&dir, &options, &machine, "shuyi", &mut no, &mut no_ready).unwrap();
    assert_eq!(out, "mcp install: nothing written\n");
    assert!(
        asked[0].starts_with(".mcp.json (Claude Code):\n"),
        "{}",
        asked[0]
    );
    assert!(asked[0].contains("+     \"shuyi\": {"), "{}", asked[0]);
    assert!(asked[0].ends_with("write 1 file(s)? [y/N] "));
    assert!(!dir.join(".mcp.json.bak").exists());

    let mut yes = |_: &str| true;
    let mut said = String::new();
    let mut ready = |so_far: &str| {
        said.push_str(so_far);
        Ok(Duration::from_secs(1))
    };
    let out = install(&dir, &options, &machine, "shuyi", &mut yes, &mut ready).unwrap();
    // What it wrote is said before the ready run, which says how long it
    // took.
    assert_eq!(
        said,
        "mcp install: wrote .mcp.json (Claude Code), its old text in .mcp.json.bak\n"
    );
    assert_eq!(
        out,
        "mcp install: the server is ready (\"shuyi mcp --ready\" took 1.0 s)\n"
    );
    let backup = fs::read_to_string(dir.join(".mcp.json.bak")).unwrap();
    assert!(!backup.contains("shuyi"));
    assert!(
        fs::read_to_string(dir.join(".mcp.json"))
            .unwrap()
            .contains("\"shuyi\"")
    );

    let mut never = |_: &str| -> bool { panic!("nothing to ask") };
    let out = install(&dir, &options, &machine, "shuyi", &mut never, &mut no_ready).unwrap();
    assert_eq!(
        out,
        "mcp install: .mcp.json (Claude Code) has the server already\n"
    );

    // `--yes` doesn't ask, and a new file has no backup.
    fs::create_dir_all(dir.join(".cursor")).unwrap();
    let options = Options {
        yes: true,
        ..Options::default()
    };
    let mut said = String::new();
    let mut ready = |so_far: &str| {
        said.push_str(so_far);
        Ok(Duration::from_secs(1))
    };
    install(&dir, &options, &machine, "shuyi", &mut never, &mut ready).unwrap();
    assert!(
        said.ends_with("mcp install: wrote .cursor/mcp.json (Cursor)\n")
            || said.ends_with("mcp install: wrote .cursor\\mcp.json (Cursor)\n"),
        "{said}"
    );
    // A file it can't read stops it before anything is written.
    fs::write(dir.join(".cursor/mcp.json"), "{nope").unwrap();
    assert!(
        install(
            &dir,
            &options,
            &machine,
            "cargo shuyi",
            &mut never,
            &mut no_ready
        )
        .is_err()
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_slow_ready_run_says_how_to_wait_longer_and_a_failed_one_is_the_error() {
    let dir = scratch("ready");
    let options = Options {
        yes: true,
        ..Options::default()
    };
    let machine = Machine::default();
    let mut never = |_: &str| -> bool { panic!("--yes asks nothing") };
    let mut slow = |_: &str| Ok(SLOW + Duration::from_millis(1500));
    let out = install(
        &dir,
        &options,
        &machine,
        "cargo shuyi",
        &mut never,
        &mut slow,
    )
    .unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[0],
        "mcp install: the server is ready (\"cargo shuyi mcp --ready\" took 21.5 s)"
    );
    assert!(lines[1].contains("MCP_TIMEOUT"), "{out}");
    assert!(
        lines[1].contains("run \"cargo shuyi mcp --ready\" after each update"),
        "{out}"
    );
    // Just at the line, no hint.
    fs::remove_file(dir.join(".mcp.json")).unwrap();
    let mut at = |_: &str| Ok(SLOW);
    let out = install(&dir, &options, &machine, "shuyi", &mut never, &mut at).unwrap();
    assert!(!out.contains("MCP_TIMEOUT"), "{out}");

    fs::remove_file(dir.join(".mcp.json")).unwrap();
    let mut failed = |_: &str| Err("error[E0425]: cannot find value `x`".to_owned());
    let error = install(&dir, &options, &machine, "shuyi", &mut never, &mut failed).unwrap_err();
    assert_eq!(error, "error[E0425]: cannot find value `x`");
    // The file is written all the same: the next ready run builds it.
    assert!(dir.join(".mcp.json").is_file());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_ready_run_runs_the_command_with_mcp_ready() {
    let dir = scratch("run-ready");
    let error = run_ready(&dir, "no-such-shuyi-program --x").unwrap_err();
    assert!(
        error.starts_with("cannot run \"no-such-shuyi-program --x mcp --ready\": "),
        "{error}"
    );
    // `cargo mcp --ready` is no cargo command: it fails, and says what
    // was run.
    let error = run_ready(&dir, "cargo").unwrap_err();
    assert!(
        error.starts_with("\"cargo mcp --ready\" failed (exit code "),
        "{error}"
    );
    fs::remove_dir_all(&dir).unwrap();
}
