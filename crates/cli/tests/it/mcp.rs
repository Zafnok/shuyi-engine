//! `shuyi mcp` driven over stdio by a scripted client, as an agent drives
//! it, against Hello, Grid from the repository's root (as its `.mcp.json`
//! runs it): start the game, step into the stage, read the scene, paint
//! the frame, and one call that fails with its code; then reset, it reads
//! the frames' times as a stream with `perf`, and `run --perf` writes a
//! trace. The session's game runs in Hello, Grid's tool program, which
//! `start` builds with cargo.
//! Then the session attaches to a running build over the live link (the
//! toy behind a test host), steps it and reads its frame and picture.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};
use shuyi_cli::codec::decode_png;
use shuyi_cli::toy::link::{LinkHost, TOKEN};

/// The repository's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A client of a running `shuyi mcp`.
struct Client {
    server: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl Client {
    /// `shuyi mcp` started in `dir`.
    fn start(dir: &Path) -> Self {
        let mut server = Command::new(env!("CARGO_BIN_EXE_shuyi"))
            .arg("mcp")
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn shuyi mcp: {e}"));
        let stdin = server.stdin.take();
        let stdout = BufReader::new(
            server
                .stdout
                .take()
                .unwrap_or_else(|| panic!("shuyi mcp has no stdout")),
        );
        Self {
            server,
            stdin,
            stdout,
            next_id: 1,
        }
    }

    /// Sends `message`, one line.
    fn send(&mut self, message: &Value) {
        let stdin = self
            .stdin
            .as_mut()
            .unwrap_or_else(|| panic!("stdin is closed"));
        writeln!(stdin, "{message}")
            .and_then(|()| stdin.flush())
            .unwrap_or_else(|e| panic!("send {message}: {e}"));
    }

    /// The response to request `method` with `params`.
    fn request(&mut self, method: &str, params: &Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .unwrap_or_else(|e| panic!("read a response: {e}"));
        let response: Value = serde_json::from_str(&line)
            .unwrap_or_else(|e| panic!("not a JSON-RPC response ({e}): {line}"));
        assert_eq!(response["id"], id, "{response}");
        response
    }

    /// The result of tool `name` called with `arguments`.
    fn call(&mut self, name: &str, arguments: &Value) -> Value {
        let response = self.request(
            "tools/call",
            &json!({ "name": name, "arguments": arguments }),
        );
        response["result"].clone()
    }
}

/// `text` decoded from standard base64.
fn unbase64(text: &str) -> Vec<u8> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let values: Vec<u32> = text
        .bytes()
        .filter(|b| *b != b'=')
        .map(|b| {
            ALPHABET
                .iter()
                .zip(0u32..)
                .find(|(a, _)| **a == b)
                .map_or_else(|| panic!("not base64: {b}"), |(_, value)| value)
        })
        .collect();
    let mut out = Vec::new();
    for chunk in values.chunks(4) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0, |n, (i, v)| n | (v << (18 - 6 * i)));
        out.extend(n.to_be_bytes()[1..chunk.len()].iter());
    }
    out
}

#[test]
fn an_agent_starts_hello_grid_steps_into_the_stage_and_looks_at_it() {
    let mut client = Client::start(&root());
    let init = client.request(
        "initialize",
        &json!({ "protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": { "name": "the test", "version": "1" } }),
    );
    assert_eq!(init["result"]["serverInfo"]["name"], "shuyi");
    client.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
    let tools = client.request("tools/list", &json!({}));
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    for name in [
        "start",
        "step",
        "scene",
        "frame_png",
        "events",
        "perf",
        "reset",
        "stop",
        "bots",
    ] {
        assert!(names.contains(&name), "{name} in {names:?}");
    }

    // Start: build the game's tool program and start at the title.
    let started = client.call("start", &json!({ "project": "examples/hello-grid" }));
    assert_eq!(started["isError"], false, "{started}");
    let answer = &started["structuredContent"];
    assert_eq!(answer["result"]["top_screen"], "title", "{answer}");
    let flags: Vec<&str> = answer["result"]["options"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|o| o["flag"].as_str())
        .collect();
    assert!(flags.contains(&"--stage"), "{flags:?}");

    // Step to the battle: Enter on the title opens the stage.
    let stepped = client.call(
        "step",
        &json!({ "script": { "steps": [{ "keys": "Enter" }] }, "frames": 2 }),
    );
    assert_eq!(stepped["isError"], false, "{stepped}");
    let result = &stepped["structuredContent"]["result"];
    assert_eq!(result["frames"], 4);
    assert!(
        result["screens"]
            .as_array()
            .unwrap()
            .contains(&json!("stage")),
        "{result}"
    );
    assert!(result["log"].is_array());

    // Read the scene: the stage's map and state, and the log.
    let scene = client.call("scene", &json!({}));
    assert_eq!(scene["isError"], false, "{scene}");
    let result = &scene["structuredContent"]["result"];
    assert_eq!(result["scene"]["screen"], "stage");
    assert_eq!(result["scene"]["kind"], "grid map");
    assert_eq!(result["scene"]["data"]["turn"], 1);
    assert!(result["log"].is_array());

    // Render the frame: an image, the size the answer says.
    let frame = client.call("frame_png", &json!({ "scale": 1 }));
    assert_eq!(frame["isError"], false, "{frame}");
    let image = &frame["content"][1];
    assert_eq!(image["type"], "image");
    assert_eq!(image["mimeType"], "image/png");
    let (width, height, rgba) = decode_png(&unbase64(image["data"].as_str().unwrap())).unwrap();
    let result = &frame["structuredContent"]["result"];
    assert_eq!(
        (result["width"].as_u64(), result["height"].as_u64()),
        (Some(u64::from(width)), Some(u64::from(height)))
    );
    assert!(
        rgba.chunks(4).any(|p| p != &rgba[..4]),
        "something is drawn"
    );

    // A call that fails: bots with 0 games, an error result with its code.
    let failed = client.call(
        "bots",
        &json!({ "games": 0, "project": "examples/hello-grid" }),
    );
    assert_eq!(failed["isError"], true, "{failed}");
    let answer = &failed["structuredContent"];
    assert_eq!(answer["code"], "SH0001", "{answer}");
    assert!(answer["help"].is_string());

    // The session outlived the one-off call.
    let events = client.call("events", &json!({}));
    assert_eq!(
        events["structuredContent"]["result"]["frames"], 4,
        "{events}"
    );

    read_perf_as_a_stream(&mut client);

    let ended = client.call("stop", &json!({}));
    assert_eq!(ended["structuredContent"]["result"]["frames"], 0, "{ended}");
    let after = client.call("scene", &json!({}));
    assert_eq!(after["structuredContent"]["code"], "SH0002");

    attach_to_a_running_build(&mut client);

    client.stdin = None;
    let status = client.server.wait().unwrap();
    assert!(status.success(), "{status}");
}

/// The records' frames in `perf`'s answer.
fn perf_frames(answer: &Value) -> Vec<u64> {
    answer["structuredContent"]["result"]["records"]
        .as_array()
        .unwrap_or_else(|| panic!("records: {answer}"))
        .iter()
        .filter_map(|r| r["frame"].as_u64())
        .collect()
}

/// The session's game started again and read as a stream with `perf`,
/// then `run --perf` with a trace file: the session is left just reset.
fn read_perf_as_a_stream(client: &mut Client) {
    let reset = client.call("reset", &json!({}));
    assert_eq!(reset["isError"], false, "{reset}");
    client.call("step", &json!({ "frames": 10 }));
    let first = client.call("perf", &json!({ "since": 0 }));
    assert_eq!(first["isError"], false, "{first}");
    assert_eq!(perf_frames(&first), (1..=10).collect::<Vec<_>>());
    let result = &first["structuredContent"]["result"];
    assert_eq!(result["next"], 10, "{result}");
    assert_eq!(result["summary"]["frames"], 10, "{result}");
    assert!(result["records"][0]["times"]["frame_us"].is_u64());
    client.call("step", &json!({ "frames": 5 }));
    let then = client.call("perf", &json!({ "since": 10 }));
    assert_eq!(perf_frames(&then), (11..=15).collect::<Vec<_>>());
    client.call("reset", &json!({}));
    let reset = client.call("perf", &json!({ "since": 15 }));
    let limits = reset["structuredContent"]["limits"].to_string();
    assert!(limits.contains("read from 0"), "{limits}");

    // `run --perf --perf-out`: a summary, and a line per kept frame.
    let dir = crate::scratch("mcp-perf");
    let file = dir.join("perf.jsonl");
    let ran = client.call(
        "run",
        &json!({ "project": "examples/hello-grid", "frames": 120,
                "perf_out": file.display().to_string() }),
    );
    assert_eq!(ran["isError"], false, "{ran}");
    let answer = &ran["structuredContent"];
    let summary = &answer["result"]["perf"];
    assert_eq!(summary["frames"], 120, "{summary}");
    for part in ["frame", "update", "draw", "paint", "present"] {
        assert!(summary[part]["p99"].is_u64(), "{part}: {summary}");
    }
    assert!(
        answer["limits"].to_string().contains("this machine's"),
        "{answer}"
    );
    let trace = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{e}"));
    let lines: Vec<shuyi_ui::perf::PerfLine> = trace
        .lines()
        .map(|line| serde_json::from_str(line).unwrap_or_else(|e| panic!("{e}: {line}")))
        .collect();
    assert_eq!(lines.len(), 120);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The session attached to a running build over the live link (the toy
/// behind a test host): stepped, its frame and its picture read.
fn attach_to_a_running_build(client: &mut Client) {
    let host = LinkHost::start();
    let attached = client.call(
        "start",
        &json!({ "link": { "address": host.address(), "token": TOKEN } }),
    );
    assert_eq!(attached["isError"], false, "{attached}");
    assert_eq!(attached["structuredContent"]["result"]["game"], "Toy");
    let stepped = client.call(
        "step",
        &json!({ "script": { "steps": [{ "keys": "f" }] }, "frames": 1 }),
    );
    let result = &stepped["structuredContent"]["result"];
    assert_eq!(result["screens"], json!(["home", "menu"]), "{stepped}");
    let text = result["frame"].as_str().unwrap_or_default();
    assert!(text.contains("--- "), "{stepped}");
    let frame = client.call("frame_png", &json!({}));
    assert_eq!(frame["isError"], false, "{frame}");
    assert_eq!(frame["content"][1]["type"], "image");
    let png = frame["content"][1]["data"].as_str().unwrap_or_default();
    assert_eq!(unbase64(png), b"\x89PNG");
    let ended = client.call("stop", &json!({}));
    assert_eq!(ended["isError"], false, "{ended}");
    host.let_go();
}

#[test]
fn this_repository_mcp_json_is_what_install_writes_for_claude_code() {
    let text = std::fs::read_to_string(root().join(".mcp.json"))
        .unwrap()
        .replace("\r\n", "\n");
    let claude = &shuyi_cli::mcp::install::AGENTS[0];
    assert_eq!(claude.name, "Claude Code");
    assert!(
        text == claude.file_text(None, ENGINE_COMMAND).unwrap(),
        ".mcp.json is not what `cargo shuyi mcp install --dry-run --command \"{ENGINE_COMMAND}\"` \
         prints for Claude Code: write that"
    );
    // And Claude Code waits for the server's first build, as install sets.
    let settings = std::fs::read_to_string(root().join(shuyi_cli::mcp::install::SETTINGS))
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(
        Ok(settings.clone()),
        shuyi_cli::mcp::install::settings_text(Some(&settings)),
        ".claude/settings.json doesn't set what `mcp install` sets"
    );
}

/// How this repository runs `shuyi`, which has none installed.
const ENGINE_COMMAND: &str = "cargo run -q -p shuyi-cli --";
