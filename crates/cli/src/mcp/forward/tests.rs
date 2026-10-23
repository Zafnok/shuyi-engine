//! A forwarded server over scripted pipes, the words that start one, and
//! a root's URI as a path.

use std::sync::mpsc::Sender;

use super::*;

/// What a process wrote to its server's stdin.
#[derive(Clone, Default)]
struct Written(Arc<Mutex<Vec<u8>>>);

impl Written {
    /// Each line written, as JSON.
    fn messages(&self) -> Vec<Value> {
        let bytes = self.0.lock().unwrap().clone();
        String::from_utf8(bytes)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

impl Write for Written {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A process on scripted pipes: what it wrote, and where its server's
/// lines go.
fn scripted(tail: &str) -> (Process, Written, Sender<String>) {
    let written = Written::default();
    let (sent, lines) = channel();
    let tail = Arc::new(Mutex::new(tail.as_bytes().to_vec()));
    let process = Process::new(
        "the project's own server (\"shuyi mcp\" in here)".to_owned(),
        Box::new(written.clone()),
        lines,
        tail,
    );
    (process, written, sent)
}

#[test]
fn it_initializes_then_sends_each_request_and_reads_until_its_answer() {
    let (mut process, written, server) = scripted("");
    let first = &written.messages()[0];
    assert_eq!(
        (first["method"].as_str(), first["id"].as_u64()),
        (Some("initialize"), Some(0))
    );
    assert!(
        first["params"]["capabilities"]["roots"].is_null(),
        "no roots"
    );
    // Still building.
    assert_eq!(process.ready(false), None);
    server
        .send(r#"{"jsonrpc":"2.0","id":0,"result":{}}"#.to_owned())
        .unwrap();
    assert_eq!(process.ready(false), Some(Ok(())));
    assert_eq!(written.messages()[1]["method"], "notifications/initialized");
    // A line that isn't JSON, a notification and another answer come
    // first: read past.
    for line in [
        "Compiling shuyi-cli",
        r#"{"jsonrpc":"2.0","method":"notifications/tools/list_changed"}"#,
        r#"{"jsonrpc":"2.0","id":9,"result":{}}"#,
        r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#,
    ] {
        server.send(line.to_owned()).unwrap();
    }
    let answer = process.request("tools/list", &json!({})).unwrap();
    assert_eq!(answer["result"]["tools"], json!([]));
    let asked = &written.messages()[2];
    assert_eq!(
        (asked["method"].as_str(), asked["id"].as_u64()),
        (Some("tools/list"), Some(1))
    );
    // The next request has the next id: its answer, then the pipe closes,
    // so one waiting for another id stops instead of waiting forever.
    server
        .send(r#"{"jsonrpc":"2.0","id":2,"result":{"ok":1}}"#.to_owned())
        .unwrap();
    drop(server);
    let answer = process
        .request("tools/call", &json!({ "name": "explain" }))
        .unwrap();
    assert_eq!(answer["result"]["ok"], 1);
    assert_eq!(written.messages()[3]["params"]["name"], "explain");
}

#[test]
fn a_server_that_exits_before_answering_says_why_with_its_last_output() {
    let (mut process, _, server) = scripted("error[E0425]: cannot find value `hero`\n");
    drop(server);
    let why = process.request("tools/call", &json!({})).unwrap_err();
    assert_eq!(
        why,
        "the project's own server (\"shuyi mcp\" in here) stopped before it answered: \
         error[E0425]: cannot find value `hero`"
    );
    assert_eq!(process.ready(false), Some(Err(why.clone())));
    assert_eq!(process.request("tools/list", &json!({})), Err(why));

    // One that stops after `initialize`, before its answer.
    let (mut process, _, server) = scripted("");
    server
        .send(r#"{"jsonrpc":"2.0","id":0,"result":{}}"#.to_owned())
        .unwrap();
    assert_eq!(process.ready(true), Some(Ok(())));
    drop(server);
    let why = process.request("tools/list", &json!({})).unwrap_err();
    assert!(why.ends_with("stopped before it answered"), "{why}");
}

#[test]
fn a_server_that_refuses_to_initialize_says_so() {
    let (mut process, _, server) = scripted("");
    server
        .send(r#"{"jsonrpc":"2.0","id":0,"error":{"code":-32600,"message":"no"}}"#.to_owned())
        .unwrap();
    let Some(Err(why)) = process.ready(true) else {
        panic!("ready");
    };
    assert!(why.contains("refused to start"), "{why}");
    assert!(process.request("tools/list", &json!({})).is_err());
}

#[test]
fn a_server_that_never_started_says_why() {
    let words = ["no-such-program-of-shuyi".to_owned(), "mcp".to_owned()];
    let mut server = Processes.spawn(&words, &std::env::temp_dir());
    let Some(Err(why)) = server.ready(true) else {
        panic!("started");
    };
    assert!(
        why.starts_with("the project's own server (\"no-such-program-of-shuyi mcp\" in "),
        "{why}"
    );
    assert!(why.contains("didn't start"), "{why}");
    assert!(server.request("tools/list", &json!({})).is_err());
    let mut server = Processes.spawn(&[], &std::env::temp_dir());
    assert!(server.request("tools/list", &json!({})).is_err());
}

#[test]
fn a_root_is_a_path() {
    let path = |uri| path_of_uri(uri).map(|p| p.display().to_string().replace('\\', "/"));
    assert_eq!(
        path("file:///C:/Users/nick/game").as_deref(),
        Some("C:/Users/nick/game")
    );
    assert_eq!(path("file:///d:/a%20b/%C3%A9").as_deref(), Some("d:/a b/é"));
    assert_eq!(
        path("file://localhost/home/nick").as_deref(),
        Some("/home/nick")
    );
    assert_eq!(
        path("file:///home/nick%2Fx").as_deref(),
        Some("/home/nick/x")
    );
    assert_eq!(path("https://example.com/x"), None);
    assert_eq!(path("file://server/share"), None);
    assert_eq!(path("file:///a%2"), None);
    assert_eq!(path("file:///a%zz"), None);
    assert_eq!(path("file:///a%ff"), None, "not UTF-8");
}

/// A fresh folder under the system temp folder.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-mcp-forward-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_projects_server_is_its_command_or_its_workspaces_entry() {
    let dir = scratch("words");
    let game = dir.join("game");
    fs::create_dir_all(&game).unwrap();
    let words = |root: &Path| words(root, &dir).map(|(words, at)| (words.join(" "), at));
    // Outside a project with no entry: none.
    assert_eq!(words(&dir), None);
    // A project with no command: `shuyi mcp`.
    fs::write(game.join("shuyi.ron"), "()").unwrap();
    assert_eq!(words(&game), Some(("shuyi mcp".to_owned(), game.clone())));
    // The workspace's `.mcp.json` entry, in the workspace.
    fs::write(
        dir.join(".mcp.json"),
        r#"{ "mcpServers": { "shuyi": { "command": "cargo", "args": ["run", "-p", "shuyi-cli", "--", "mcp"] } } }"#,
    )
    .unwrap();
    let entry = Some(("cargo run -p shuyi-cli -- mcp".to_owned(), dir.clone()));
    assert_eq!(words(&dir), entry);
    assert_eq!(words(&game), entry);
    // The project's own command first, in its root.
    fs::write(game.join("shuyi.ron"), r#"(command: "cargo shuyi")"#).unwrap();
    assert_eq!(
        words(&game),
        Some(("cargo shuyi mcp".to_owned(), game.clone()))
    );
    // An entry with no command, or args that aren't words, is none.
    fs::write(
        dir.join(".mcp.json"),
        r#"{ "mcpServers": { "shuyi": { "args": [] } } }"#,
    )
    .unwrap();
    assert_eq!(words(&dir), None);
    fs::write(
        dir.join(".mcp.json"),
        r#"{ "mcpServers": { "shuyi": { "command": "shuyi", "args": [3] } } }"#,
    )
    .unwrap();
    assert_eq!(words(&dir), None);
    fs::write(
        dir.join(".mcp.json"),
        r#"{ "mcpServers": { "shuyi": { "command": "shuyi" } } }"#,
    )
    .unwrap();
    assert_eq!(words(&dir), Some(("shuyi".to_owned(), dir.clone())));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_plain_path_has_no_verbatim_prefix() {
    assert_eq!(plain(PathBuf::from(r"\\?\C:\a")), PathBuf::from(r"C:\a"));
    let unc = PathBuf::from(r"\\?\UNC\host\share");
    assert_eq!(plain(unc.clone()), unc);
    assert_eq!(plain(PathBuf::from("/home/a")), PathBuf::from("/home/a"));
}

#[test]
fn cargo_finds_a_folders_workspace() {
    let dir = scratch("workspace");
    assert_eq!(Processes.workspace(&dir), None, "outside a workspace");
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"tiny\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join("src/below")).unwrap();
    fs::write(dir.join("src/lib.rs"), "").unwrap();
    let found = Processes.workspace(&dir.join("src/below"));
    assert_eq!(found, Some(plain(fs::canonicalize(&dir).unwrap())));
    assert!(!found.unwrap().display().to_string().starts_with(r"\\?\"));
    assert_eq!(Processes.workspace(&dir.join("missing")), None);
    fs::remove_dir_all(&dir).unwrap();
}

/// Whether the process `pid` runs.
fn runs(pid: u32) -> bool {
    if cfg!(windows) {
        let filter = format!("PID eq {pid}");
        let listed = Command::new("tasklist")
            .args(["/FI", &filter, "/NH"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&listed.stdout).contains(&format!(" {pid} "))
    } else {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }
}

#[test]
fn dropping_a_forwarded_server_ends_it() {
    // A program that never reads its stdin, so only the drop ends it.
    let words: Vec<String> = if cfg!(windows) {
        vec!["ping".into(), "-n".into(), "60".into(), "127.0.0.1".into()]
    } else {
        vec!["sleep".into(), "60".into()]
    };
    let process = Process::start(&words, &std::env::temp_dir()).unwrap();
    let pid = process.child.as_ref().unwrap().id();
    assert!(runs(pid), "it started");
    drop(process);
    assert!(!runs(pid), "it ended with the drop");
}
