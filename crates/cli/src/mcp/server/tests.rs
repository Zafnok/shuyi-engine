//! The server on the fake game, its host in this process: JSON-RPC, the
//! tools, the session, and a host that stops.

use std::fs;
use std::path::PathBuf;

use serde_json::Map;

use super::*;
use crate::contract::fake::Fake;
use crate::mcp::host::InProcess;

/// A fresh folder under the system temp folder holding a project whose
/// `shuyi.ron` says `config`.
fn scratch(name: &str, config: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-mcp-server-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("shuyi.ron"), config).unwrap();
    dir
}

/// A game tool program's project.
const GAME: &str = r#"(game: (package: "fake"))"#;

/// The response to `message`, as JSON.
fn handled(server: &mut Server<'_>, message: &Value) -> Value {
    let response = server.handle(&message.to_string()).expect("a response");
    serde_json::from_str(&response).unwrap()
}

/// The result of calling tool `name` with `arguments`.
fn called(server: &mut Server<'_>, name: &str, arguments: &Value) -> Value {
    let response = handled(
        server,
        &json!({ "jsonrpc": "2.0", "id": 7, "method": "tools/call",
                 "params": { "name": name, "arguments": arguments } }),
    );
    assert_eq!(response["id"], 7);
    response["result"].clone()
}

#[test]
fn it_speaks_json_rpc() {
    let dir = scratch("rpc", GAME);
    let game = Fake::default();
    let mut server = Server::new(dir.clone(), Box::new(InProcess::new(&game)));
    let init = handled(
        &mut server,
        &json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
                 "params": { "protocolVersion": "2025-03-26", "capabilities": {},
                             "clientInfo": { "name": "test", "version": "1" } } }),
    );
    assert_eq!(init["id"], 1);
    assert_eq!(init["result"]["protocolVersion"], "2025-03-26");
    assert_eq!(init["result"]["serverInfo"]["name"], "shuyi");
    assert!(init["result"]["capabilities"]["tools"].is_object());
    assert!(
        init["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("`start`")
    );
    let init = handled(
        &mut server,
        &json!({ "jsonrpc": "2.0", "id": 2, "method": "initialize",
                 "params": { "protocolVersion": "1999-01-01" } }),
    );
    assert_eq!(init["result"]["protocolVersion"], PROTOCOL_VERSIONS[0]);
    // Notifications and responses want no answer.
    assert!(
        server
            .handle(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .is_none()
    );
    assert!(
        server
            .handle(r#"{"jsonrpc":"2.0","id":5,"result":{}}"#)
            .is_none()
    );
    assert!(server.handle("  ").is_none());
    let ping = handled(
        &mut server,
        &json!({ "jsonrpc": "2.0", "id": "p", "method": "ping" }),
    );
    assert_eq!(
        (
            ping["id"].as_str(),
            ping["result"].as_object().map(Map::len)
        ),
        (Some("p"), Some(0))
    );
    let list = handled(
        &mut server,
        &json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/list" }),
    );
    assert_eq!(
        list["result"]["tools"].as_array().unwrap().len(),
        tools().len()
    );

    let parse: Value = serde_json::from_str(&server.handle("{nope").unwrap()).unwrap();
    assert_eq!(
        (parse["id"].clone(), parse["error"]["code"].as_i64()),
        (Value::Null, Some(-32_700))
    );
    let invalid = handled(&mut server, &json!({ "jsonrpc": "2.0", "id": 4 }));
    assert_eq!(invalid["error"]["code"], -32_600);
    let invalid = handled(&mut server, &json!([1, 2]));
    assert_eq!(invalid["error"]["code"], -32_600);
    let unknown = handled(
        &mut server,
        &json!({ "jsonrpc": "2.0", "id": 5, "method": "resources/list" }),
    );
    assert_eq!(unknown["error"]["code"], -32_601);
    let unknown = handled(
        &mut server,
        &json!({ "jsonrpc": "2.0", "id": 6, "method": "tools/call", "params": { "name": "fly" } }),
    );
    assert_eq!(unknown["error"]["code"], -32_602);
    assert_eq!(unknown["error"]["message"], "Unknown tool: fly");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_session_runs_from_start_to_stop() {
    let dir = scratch("session", GAME);
    let game = Fake::default();
    let mut server = Server::new(dir.clone(), Box::new(InProcess::new(&game)));
    let result = called(&mut server, "step", &json!({}));
    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "SH0002");

    let result = called(
        &mut server,
        "start",
        &json!({ "start": [{ "flag": "--seed", "value": "1" }] }),
    );
    assert_eq!(result["isError"], false, "{result}");
    assert!(server.has_game());
    let answer = &result["structuredContent"];
    assert_eq!(answer["op"], "start");
    // The text is the same answer, for clients that read text.
    let text: Value = serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(&text, answer);

    let result = called(
        &mut server,
        "step",
        &json!({ "script": { "steps": [{ "keys": "Enter" }] } }),
    );
    assert_eq!(result["structuredContent"]["result"]["frames"], 2);
    let result = called(&mut server, "scene", &Value::Null);
    assert_eq!(
        result["structuredContent"]["result"]["scene"]["kind"],
        "grid map"
    );
    let result = called(&mut server, "frame_png", &json!({ "scale": 1 }));
    assert_eq!(result["content"][1]["type"], "image");
    assert_eq!(result["content"][1]["mimeType"], "image/png");
    assert!(
        result["content"][1]["data"]
            .as_str()
            .unwrap()
            .starts_with("iVBORw0KGgo")
    );
    let result = called(&mut server, "events", &json!({}));
    assert_eq!(result["structuredContent"]["result"]["frames"], 2);
    let result = called(&mut server, "reset", &json!({}));
    assert_eq!(result["structuredContent"]["result"]["frames"], 0);

    // Starting again stops the game that runs, and says so.
    let result = called(&mut server, "start", &json!({}));
    assert_eq!(
        result["structuredContent"]["limits"][0],
        "the game that was running was stopped on frame 0"
    );
    let result = called(&mut server, "stop", &json!({}));
    assert_eq!(
        (
            result["isError"].as_bool(),
            result["structuredContent"]["op"].as_str()
        ),
        (Some(false), Some("stop"))
    );
    assert!(!server.has_game());
    let result = called(&mut server, "stop", &json!({}));
    assert_eq!(result["structuredContent"]["code"], "SH0002");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn start_finds_the_game_or_says_why_not() {
    let dir = scratch("start", "()");
    fs::create_dir_all(dir.join("game")).unwrap();
    fs::write(dir.join("game/shuyi.ron"), GAME).unwrap();
    let game = Fake::default();
    let mut server = Server::new(dir.join("game/below"), Box::new(InProcess::new(&game)));
    let code = |result: &Value| {
        result["structuredContent"]["code"]
            .as_str()
            .map(str::to_owned)
    };

    assert_eq!(
        code(&called(&mut server, "start", &json!({}))),
        None,
        "found from a folder above"
    );
    let mut server = Server::new(dir.clone(), Box::new(InProcess::new(&game)));
    let result = called(&mut server, "start", &json!({}));
    assert_eq!(
        code(&result).as_deref(),
        Some("SH1002"),
        "a project with no game program"
    );
    let result = called(&mut server, "start", &json!({ "project": "game" }));
    assert_eq!(result["isError"], false, "{result}");
    let outside = std::env::temp_dir().join(format!("shuyi-mcp-nowhere-{}", std::process::id()));
    let result = called(
        &mut server,
        "start",
        &json!({ "project": outside.join("x") }),
    );
    assert!(
        matches!(code(&result).as_deref(), Some("SH1001" | "SH1002")),
        "{result}"
    );
    let result = called(&mut server, "start", &json!({ "start": "--stage" }));
    assert_eq!(code(&result).as_deref(), Some("SH0001"));

    // A game that won't start leaves no session.
    let broken = Fake {
        session: Err("no such stage".into()),
        ..Fake::default()
    };
    let mut server = Server::new(dir.join("game"), Box::new(InProcess::new(&broken)));
    let result = called(&mut server, "start", &json!({}));
    assert_eq!(code(&result).as_deref(), Some("SH3002"));
    assert!(!server.has_game());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_operations_are_tools_and_a_failed_one_is_an_error_result_with_its_code() {
    let dir = scratch("ops", GAME);
    fs::write(dir.join("a.ron"), "h0\nh1").unwrap();
    let game = Fake::default();
    let mut server = Server::new(dir.clone(), Box::new(InProcess::new(&game)));
    let result = called(&mut server, "bots", &json!({ "games": 0 }));
    assert_eq!(result["isError"], true);
    let answer = &result["structuredContent"];
    assert_eq!(
        (answer["op"].as_str(), answer["code"].as_str()),
        (Some("bots"), Some("SH0001"))
    );
    assert!(answer["help"].is_string());
    let result = called(&mut server, "bots", &json!({ "games": 2, "project": "." }));
    assert_eq!(result["isError"], false, "{result}");
    let result = called(&mut server, "replay", &json!({ "record": "a.ron" }));
    assert_eq!(result["structuredContent"]["ok"], true);
    let result = called(&mut server, "explain", &json!({ "code": "SH0002" }));
    assert_eq!(
        result["structuredContent"]["result"]["entries"][0]["name"],
        "no-session"
    );
    let result = called(&mut server, "explain", &json!({ "project": "." }));
    assert_eq!(
        result["structuredContent"]["error"],
        "explain takes no project"
    );
    let result = called(&mut server, "check", &json!({ "project": 3 }));
    assert_eq!(result["structuredContent"]["code"], "SH0001");
    // A one-off run leaves the session alone.
    let result = called(&mut server, "run", &json!({ "frames": 1 }));
    assert_eq!(result["structuredContent"]["result"]["frames"], 1);
    assert!(!server.has_game());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_project_with_no_game_program_answers_what_needs_none() {
    let dir = scratch("no-game", "()");
    fs::write(dir.join("colors.ron"), "{ \"text\": \"#ffffff\" }").unwrap();
    let game = Fake::default();
    let mut server = Server::new(dir.clone(), Box::new(InProcess::new(&game)));
    let result = called(&mut server, "check", &json!({}));
    assert_eq!(result["structuredContent"]["code"], "SH1002");
    let result = called(&mut server, "import", &json!({ "file": "colors.ron" }));
    assert_eq!(result["structuredContent"]["ok"], true, "{result}");
    fs::remove_dir_all(&dir).unwrap();
}

/// A host that stops at its `fails`-th call.
struct Stopping {
    calls: u32,
    fails: u32,
}

impl Link for Stopping {
    fn call(&mut self, request: &Request) -> Result<Reply, String> {
        self.calls += 1;
        if self.calls >= self.fails {
            return Err("the game's tool program stopped before it answered: panicked".into());
        }
        let answer = crate::contract::Answer::new(request.op, true, json!({}), Vec::new());
        Ok(Reply {
            answer: Some(serde_json::to_value(answer).unwrap()),
            ..Reply::default()
        })
    }
}

/// Launches a [`Stopping`] host, or none (`refuse`).
struct Launches {
    fails: u32,
    refuse: bool,
}

impl<'a> Launch<'a> for Launches {
    fn launch(&self, _: &Project) -> Result<Box<dyn Link + 'a>, (Code, String)> {
        if self.refuse {
            return Err((codes::TOOLCHAIN, "cannot run cargo".into()));
        }
        Ok(Box::new(Stopping {
            calls: 0,
            fails: self.fails,
        }))
    }
}

#[test]
fn a_host_that_stops_ends_the_session_with_its_output() {
    let dir = scratch("stops", GAME);
    let launches = |fails, refuse| Box::new(Launches { fails, refuse });
    let mut server = Server::new(dir.clone(), launches(2, false));
    assert_eq!(called(&mut server, "start", &json!({}))["isError"], false);
    let result = called(&mut server, "step", &json!({}));
    let answer = &result["structuredContent"];
    assert_eq!(
        (answer["op"].as_str(), answer["code"].as_str()),
        (Some("step"), Some("SH3005"))
    );
    assert!(answer["error"].as_str().unwrap().ends_with("panicked"));
    assert!(!server.has_game());

    let mut server = Server::new(dir.clone(), launches(1, false));
    assert_eq!(
        called(&mut server, "start", &json!({}))["structuredContent"]["code"],
        "SH3005"
    );
    assert_eq!(
        called(&mut server, "bots", &json!({}))["structuredContent"]["code"],
        "SH3005"
    );
    let mut server = Server::new(dir.clone(), launches(1, true));
    assert_eq!(
        called(&mut server, "start", &json!({}))["structuredContent"]["code"],
        "SH1003"
    );
    assert_eq!(
        called(&mut server, "check", &json!({}))["structuredContent"]["code"],
        "SH1003"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_host_that_answers_nothing_is_a_game_program_error() {
    let (answer, png) = answer_of(Op::Bots, Reply::default());
    assert_eq!(answer["code"], "SH3005");
    assert!(png.is_none());
    let refused = Reply {
        error: Some("is its engine older than the server's?".into()),
        ..Reply::default()
    };
    let (answer, _) = answer_of(Op::Step, refused);
    assert_eq!(answer["error"], "is its engine older than the server's?");
}

#[test]
fn a_result_is_an_error_unless_its_answer_is_ok() {
    let ok = tool_result(&json!({ "ok": true }), None);
    assert_eq!(
        (
            ok["isError"].as_bool(),
            ok["content"].as_array().map(Vec::len)
        ),
        (Some(false), Some(1))
    );
    let failed = tool_result(&json!({ "ok": false }), Some("AAAA".into()));
    assert_eq!(failed["isError"], true);
    assert_eq!(failed["content"][1]["data"], "AAAA");
    assert_eq!(tool_result(&json!(null), None)["isError"], true);
}
