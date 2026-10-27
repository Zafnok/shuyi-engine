//! The server on the fake game, its host in this process: JSON-RPC, the
//! tools, the session, and a host that stops.

use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

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

/// The one message the server writes for `message`, as JSON.
fn handled(server: &mut Server<'_>, message: &Value) -> Value {
    let written = server.handle(&message.to_string());
    assert_eq!(written.len(), 1, "{written:?}");
    serde_json::from_str(&written[0]).unwrap()
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
    let seen = server.handle(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    assert!(seen.is_empty(), "{seen:?}");
    let seen = server.handle(r#"{"jsonrpc":"2.0","id":5,"result":{}}"#);
    assert!(seen.is_empty(), "{seen:?}");
    let seen = server.handle(" ");
    assert!(seen.is_empty(), "{seen:?}");
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

    let parse: Value = serde_json::from_str(&server.handle("{nope")[0]).unwrap();
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
    let (answer, png) = answer_of(Op::Bots, Reply::default(), "shuyi");
    assert_eq!(answer["code"], "SH3005");
    assert!(png.is_none());
    let refused = Reply {
        error: Some("is its engine older than the server's?".into()),
        ..Reply::default()
    };
    let (answer, _) = answer_of(Op::Step, refused, "shuyi");
    assert_eq!(answer["error"], "is its engine older than the server's?");
}

/// A host whose engine speaks contract `contract`.
struct Speaks {
    contract: u32,
}

impl Link for Speaks {
    fn call(&mut self, request: &Request) -> Result<Reply, String> {
        let mut answer = crate::contract::Answer::new(request.op, true, json!({}), Vec::new());
        answer.contract = self.contract;
        Ok(Reply {
            answer: Some(serde_json::to_value(answer).unwrap()),
            ..Reply::default()
        })
    }
}

impl<'a> Launch<'a> for Speaks {
    fn launch(&self, _: &Project) -> Result<Box<dyn Link + 'a>, (Code, String)> {
        Ok(Box::new(Speaks {
            contract: self.contract,
        }))
    }
}

#[test]
fn a_server_older_than_the_games_engine_says_so() {
    let limit = stale(&json!({ "contract": VERSION + 1 }), VERSION, "cargo shuyi").unwrap();
    assert_eq!(
        limit,
        format!(
            "the game's engine speaks contract {} and this server {VERSION}: the server was \
             built before the engine changed; run \"cargo shuyi mcp --ready\", then reconnect \
             it (in Claude Code: /mcp)",
            VERSION + 1
        )
    );
    assert_eq!(
        stale(&json!({ "contract": VERSION }), VERSION, "shuyi"),
        None
    );
    assert_eq!(stale(&json!({}), VERSION, "shuyi"), None);

    // Through the server: `start`, each session tool and a one-off
    // operation say it, with the project's command.
    let dir = scratch(
        "stale",
        r#"(command: "cargo shuyi", game: (package: "fake"))"#,
    );
    let newer = VERSION + 1;
    let mut server = Server::new(dir.clone(), Box::new(Speaks { contract: newer }));
    for (name, arguments) in [
        ("start", json!({})),
        ("step", json!({})),
        ("bots", json!({})),
    ] {
        let result = called(&mut server, name, &arguments);
        let limits = &result["structuredContent"]["limits"];
        assert_eq!(limits.as_array().map(Vec::len), Some(1), "{name}: {result}");
        assert!(
            limits[0]
                .as_str()
                .unwrap()
                .contains("run \"cargo shuyi mcp --ready\""),
            "{name}: {result}"
        );
    }
    let mut server = Server::new(dir.clone(), Box::new(Speaks { contract: VERSION }));
    for name in ["start", "step", "bots"] {
        let result = called(&mut server, name, &json!({}));
        assert_eq!(result["structuredContent"]["limits"], json!([]), "{name}");
    }
    fs::remove_dir_all(&dir).unwrap();
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

/// What the fake forwarded servers were asked: each server started (its
/// words and folder), each tool called (its name and arguments), and the
/// folder of each server asked for its tools.
#[derive(Default)]
struct Asked {
    spawned: RefCell<Vec<(String, PathBuf)>>,
    called: RefCell<Vec<(String, Value)>>,
    listed: RefCell<Vec<PathBuf>>,
}

/// Workspaces by folder (the longest that a folder is in), and forwarded
/// servers that list `tools`, answer every call (a `start` of
/// `"fail"` not ok), or stop while `fail` is set, and are ready once
/// `ready` is.
struct Workspaces {
    of: Vec<(PathBuf, PathBuf)>,
    tools: Vec<Value>,
    fail: Rc<Cell<bool>>,
    ready: Rc<Cell<bool>>,
    asked: Rc<Asked>,
}

impl Workspaces {
    fn new(of: &[(&Path, &str)]) -> Self {
        Self {
            of: of
                .iter()
                .map(|(folder, ws)| (folder.to_path_buf(), PathBuf::from(ws)))
                .collect(),
            tools: tools(),
            fail: Rc::default(),
            ready: Rc::new(Cell::new(true)),
            asked: Rc::default(),
        }
    }
}

impl<'a> Forwarding<'a> for Workspaces {
    fn workspace(&self, folder: &Path) -> Option<PathBuf> {
        self.of
            .iter()
            .filter(|(prefix, _)| folder.starts_with(prefix))
            .max_by_key(|(prefix, _)| prefix.components().count())
            .map(|(_, ws)| ws.clone())
    }

    fn spawn(&self, words: &[String], root: &Path) -> Box<dyn Forward + 'a> {
        self.asked
            .spawned
            .borrow_mut()
            .push((words.join(" "), root.to_path_buf()));
        Box::new(Forwarded {
            root: root.to_path_buf(),
            tools: self.tools.clone(),
            fail: Rc::clone(&self.fail),
            ready: Rc::clone(&self.ready),
            asked: Rc::clone(&self.asked),
        })
    }
}

/// A fake forwarded server, run in `root`.
struct Forwarded {
    root: PathBuf,
    tools: Vec<Value>,
    fail: Rc<Cell<bool>>,
    ready: Rc<Cell<bool>>,
    asked: Rc<Asked>,
}

/// Why a fake forwarded server that fails answers nothing.
const DIDNT_BUILD: &str = "the project's own server (\"cargo shuyi mcp\" in there) stopped \
                           before it answered (exit code 101): error[E0425]: cannot find value";

impl Forward for Forwarded {
    fn ready(&mut self, wait: bool) -> Option<Result<(), String>> {
        if self.fail.get() {
            Some(Err(DIDNT_BUILD.to_owned()))
        } else {
            (wait || self.ready.get()).then_some(Ok(()))
        }
    }

    fn request(&mut self, method: &str, params: &Value) -> Result<Value, String> {
        if self.fail.get() {
            return Err(DIDNT_BUILD.to_owned());
        }
        let result = match method {
            "tools/list" => {
                self.asked.listed.borrow_mut().push(self.root.clone());
                json!({ "tools": self.tools })
            }
            "tools/call" => {
                let name = params["name"].as_str().unwrap().to_owned();
                self.asked
                    .called
                    .borrow_mut()
                    .push((name.clone(), params["arguments"].clone()));
                let ok = !(name == "start" && params["arguments"]["start"] == "fail");
                let answer = json!({ "ok": ok, "op": name, "by": "forwarded", "limits": [] });
                tool_result(&answer, None)
            }
            _ => panic!("{method}"),
        };
        Ok(json!({ "jsonrpc": "2.0", "id": 99, "result": result }))
    }
}

/// The messages the server wrote, as JSON.
fn messages(written: &[String]) -> Vec<Value> {
    written
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

/// The client's `initialize`, with roots if `roots`, and its
/// `notifications/initialized`: what the server writes for that.
fn initialized(server: &mut Server<'_>, roots: bool) -> Vec<Value> {
    let capabilities = if roots {
        json!({ "roots": { "listChanged": true } })
    } else {
        json!({})
    };
    let init = handled(
        server,
        &json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize",
                 "params": { "protocolVersion": "2025-11-25", "capabilities": capabilities } }),
    );
    assert_eq!(init["result"]["capabilities"]["tools"]["listChanged"], true);
    messages(&server.handle(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#))
}

/// The client's answer to `roots/list`: `folder` as a `file://` URI.
fn rooted(server: &mut Server<'_>, folder: &Path) -> Vec<Value> {
    let path = folder
        .display()
        .to_string()
        .replace('\\', "/")
        .replace(' ', "%20");
    let uri = if path.starts_with('/') {
        format!("file://{path}")
    } else {
        format!("file:///{path}")
    };
    let answer = json!({ "jsonrpc": "2.0", "id": ROOTS, "result": { "roots": [{ "uri": uri }] } });
    messages(&server.handle(&answer.to_string()))
}

#[test]
fn the_server_asks_for_roots_only_when_the_client_has_them() {
    let dir = scratch("roots", GAME);
    let game = Fake::default();
    let mut server = Server::new(dir.clone(), Box::new(InProcess::new(&game)));
    assert_eq!(initialized(&mut server, false), Vec::<Value>::new());
    let mut server = Server::new(dir.clone(), Box::new(InProcess::new(&game)));
    let asked = initialized(&mut server, true);
    assert_eq!(
        asked,
        [json!({ "jsonrpc": "2.0", "id": ROOTS, "method": "roots/list" })]
    );
    // An answer with no root, or one that isn't a file, leaves the
    // working folder.
    let none = json!({ "jsonrpc": "2.0", "id": ROOTS, "result": { "roots": [] } });
    let got = server.handle(&none.to_string());
    assert!(got.is_empty(), "{got:?}");
    let web = json!({ "jsonrpc": "2.0", "id": ROOTS,
                      "result": { "roots": [{ "uri": "https://example.com" }] } });
    let got = server.handle(&web.to_string());
    assert!(got.is_empty(), "{got:?}");
    assert_eq!(server.folder(), dir.as_path());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_relative_project_is_from_the_first_root() {
    let cwd = scratch("root-cwd", "()");
    let root = scratch("root-folder", "()");
    let spaced = root.join("a game");
    fs::create_dir_all(&spaced).unwrap();
    fs::write(spaced.join("shuyi.ron"), GAME).unwrap();
    let game = Fake::default();
    let mut server = Server::new(cwd.clone(), Box::new(InProcess::new(&game)));
    let code = |result: &Value| result["structuredContent"]["code"].clone();
    // From the working folder it is the working folder's project, which
    // has no game.
    let result = called(&mut server, "start", &json!({ "project": "a game" }));
    assert_eq!(code(&result), "SH1002");
    initialized(&mut server, true);
    let got = rooted(&mut server, &root);
    assert!(got.is_empty(), "{got:?}");
    assert_eq!(server.folder(), root.as_path());
    let result = called(&mut server, "start", &json!({ "project": "a game" }));
    assert_eq!(result["isError"], false, "{result}");
    // With no project, the root's own.
    let mut server = Server::new(cwd.clone(), Box::new(InProcess::new(&game)));
    initialized(&mut server, true);
    rooted(&mut server, &spaced);
    assert_eq!(called(&mut server, "start", &json!({}))["isError"], false);
    let result = called(&mut server, "bots", &json!({ "games": 2 }));
    assert_eq!(result["isError"], false, "{result}");
    fs::remove_dir_all(&cwd).unwrap();
    fs::remove_dir_all(&root).unwrap();
}

/// A server in `own` whose client's root is `root`, forwarding with
/// `workspaces`, and what it wrote at the roots' answer.
fn forwarding<'a>(
    own: &Path,
    root: &Path,
    game: &'a Fake,
    workspaces: Workspaces,
) -> (Server<'a>, Vec<Value>) {
    let mut server = Server::new(own.to_path_buf(), Box::new(InProcess::new(game)))
        .forwarding(Box::new(workspaces));
    initialized(&mut server, true);
    let written = rooted(&mut server, root);
    (server, written)
}

#[test]
fn a_call_in_another_workspace_is_answered_by_its_own_server() {
    let own = scratch("fw-own", GAME);
    let theirs = scratch(
        "fw-theirs",
        r#"(command: "cargo shuyi", game: (package: "fake"))"#,
    );
    fs::create_dir_all(theirs.join("other")).unwrap();
    fs::write(theirs.join("other/shuyi.ron"), GAME).unwrap();
    let game = Fake::default();
    let workspaces = Workspaces::new(&[(&own, "own"), (&theirs, "theirs")]);
    let asked = Rc::clone(&workspaces.asked);
    let fail = Rc::clone(&workspaces.fail);
    let (mut server, written) = forwarding(&own, &theirs, &game, workspaces);
    // Started at the roots' answer, and its list is the server's: no
    // notification.
    assert!(written.is_empty(), "{written:?}");
    assert_eq!(
        *asked.spawned.borrow(),
        [("cargo shuyi mcp".to_owned(), theirs.clone())]
    );
    let last = || asked.called.borrow().last().cloned().unwrap();

    let response = handled(
        &mut server,
        &json!({ "jsonrpc": "2.0", "id": "c1", "method": "tools/call",
                 "params": { "name": "bots", "arguments": { "project": "other", "games": 2 } } }),
    );
    assert_eq!(response["id"], "c1", "under the client's id");
    assert_eq!(response["result"]["structuredContent"]["by"], "forwarded");
    let (name, arguments) = last();
    assert_eq!((name.as_str(), &arguments["games"]), ("bots", &json!(2)));
    assert_eq!(
        Path::new(arguments["project"].as_str().unwrap()),
        theirs.join("other"),
        "made absolute"
    );
    // With no project: the root's, absolute. A tool with none gets none.
    called(&mut server, "check", &json!({}));
    assert_eq!(Path::new(last().1["project"].as_str().unwrap()), theirs);
    called(&mut server, "explain", &json!({ "code": "SH0002" }));
    assert_eq!(last(), ("explain".to_owned(), json!({ "code": "SH0002" })));
    called(&mut server, "new", &json!({ "dir": "fresh" }));
    assert_eq!(
        Path::new(last().1["dir"].as_str().unwrap()),
        theirs.join("fresh")
    );
    // One server for the workspace.
    assert_eq!(asked.spawned.borrow().len(), 1);

    // A project in the server's own workspace is its own.
    let before = asked.called.borrow().len();
    let result = called(
        &mut server,
        "bots",
        &json!({ "project": own.display().to_string(), "games": 2 }),
    );
    assert_eq!(result["isError"], false, "{result}");
    assert!(result["structuredContent"]["by"].is_null());
    assert_eq!(asked.called.borrow().len(), before);

    // A one-off call there is no session, and nor is a start that fails.
    assert!(!server.has_game());
    let result = called(&mut server, "start", &json!({ "start": "fail" }));
    assert_eq!(result["isError"], true, "{result}");
    assert!(!server.has_game());

    // The session runs where `start` went, and a start elsewhere ends it.
    assert_eq!(called(&mut server, "start", &json!({}))["isError"], false);
    assert!(server.has_game());
    called(&mut server, "step", &json!({ "frames": 1 }));
    assert_eq!(last().0, "step");
    let result = called(
        &mut server,
        "start",
        &json!({ "project": own.display().to_string() }),
    );
    assert_eq!(result["isError"], false, "{result}");
    assert_eq!(last().0, "stop");
    let before = asked.called.borrow().len();
    let result = called(&mut server, "step", &json!({}));
    assert_eq!(result["isError"], false, "{result}");
    assert_eq!(asked.called.borrow().len(), before, "the server's own game");
    // A one-off call there leaves the server's own game running.
    called(&mut server, "explain", &json!({}));
    let result = called(&mut server, "step", &json!({}));
    assert_eq!(result["isError"], false, "{result}");

    // A session there ends when its server stops.
    assert_eq!(called(&mut server, "start", &json!({}))["isError"], false);
    fail.set(true);
    let result = called(&mut server, "step", &json!({}));
    assert_eq!(result["structuredContent"]["code"], "SH3005", "{result}");
    assert!(!server.has_game());
    fs::remove_dir_all(&own).unwrap();
    fs::remove_dir_all(&theirs).unwrap();
}

#[test]
fn a_project_outside_a_workspace_or_with_no_server_is_the_servers_own() {
    let own = scratch("fw-alone", GAME);
    let other = scratch("fw-nowhere", "()");
    let bare = scratch("fw-bare", "()");
    fs::remove_file(bare.join("shuyi.ron")).unwrap();
    let game = Fake::default();
    // `other` is in no workspace, `bare` in one with no project and no
    // `.mcp.json`.
    let workspaces = Workspaces::new(&[(&own, "own"), (&bare, "bare")]);
    let asked = Rc::clone(&workspaces.asked);
    let (mut server, _) = forwarding(&own, &other, &game, workspaces);
    let result = called(&mut server, "check", &json!({}));
    assert_eq!(result["structuredContent"]["code"], "SH1002", "{result}");
    let result = called(&mut server, "explain", &json!({ "code": "SH0002" }));
    assert_eq!(result["isError"], false);
    rooted(&mut server, &bare);
    let result = called(&mut server, "check", &json!({}));
    assert_eq!(result["structuredContent"]["code"], "SH1001", "{result}");
    let got = asked.spawned.borrow();
    assert!(got.is_empty(), "{got:?}");
    fs::remove_dir_all(&own).unwrap();
    fs::remove_dir_all(&other).unwrap();
    fs::remove_dir_all(&bare).unwrap();
}

#[test]
fn the_tool_list_follows_the_session_folders_server() {
    let own = scratch("list-own", GAME);
    let theirs = scratch("list-theirs", GAME);
    let game = Fake::default();
    let mut workspaces = Workspaces::new(&[(&own, "own"), (&theirs, "theirs")]);
    let fly = json!({ "name": "fly", "description": "Fly.", "inputSchema": { "type": "object" } });
    workspaces.tools.push(fly.clone());
    workspaces.ready.set(false);
    let ready = Rc::clone(&workspaces.ready);
    let asked = Rc::clone(&workspaces.asked);
    let fail = Rc::clone(&workspaces.fail);
    let (mut server, written) = forwarding(&own, &theirs, &game, workspaces);
    // Still building: nothing yet, and the server's own list.
    assert!(written.is_empty(), "{written:?}");
    let got = server.poll();
    assert!(got.is_empty(), "{got:?}");
    let list = handled(
        &mut server,
        &json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }),
    );
    assert_eq!(list["result"]["tools"], json!(tools()));
    ready.set(true);
    assert_eq!(
        messages(&server.poll()),
        [json!({ "jsonrpc": "2.0", "method": "notifications/tools/list_changed" })]
    );
    assert!(server.poll().is_empty(), "once");
    let list = handled(
        &mut server,
        &json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
    );
    assert_eq!(
        list["result"]["tools"].as_array().unwrap().last(),
        Some(&fly)
    );
    // Its tool the server doesn't know goes to it.
    let result = called(&mut server, "fly", &json!({ "project": "up" }));
    assert_eq!(result["structuredContent"]["op"], "fly");
    assert_eq!(
        Path::new(asked.called.borrow()[0].1["project"].as_str().unwrap()),
        theirs.join("up")
    );
    // A tool on neither list is unknown.
    let unknown = handled(
        &mut server,
        &json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "swim" } }),
    );
    assert_eq!(unknown["error"]["code"], -32_602);
    // One only the followed server knows, once it stopped, can't be
    // answered.
    fail.set(true);
    let gone = handled(
        &mut server,
        &json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": { "name": "fly" } }),
    );
    assert_eq!(gone["error"]["code"], -32_603);
    assert!(
        gone["error"]["message"]
            .as_str()
            .unwrap()
            .ends_with("cannot find value")
    );
    fs::remove_dir_all(&own).unwrap();
    fs::remove_dir_all(&theirs).unwrap();
}

#[test]
fn the_server_reads_the_roots_again_when_they_change() {
    let own = scratch("again-own", GAME);
    let theirs = scratch("again-theirs", GAME);
    let game = Fake::default();
    let mut workspaces = Workspaces::new(&[(&own, "own"), (&theirs, "theirs")]);
    let fly = json!({ "name": "fly", "description": "Fly.", "inputSchema": { "type": "object" } });
    workspaces.tools.push(fly.clone());
    let changed = json!({ "jsonrpc": "2.0", "method": "notifications/tools/list_changed" });
    let ask = json!({ "jsonrpc": "2.0", "id": ROOTS, "method": "roots/list" });
    let change = r#"{"jsonrpc":"2.0","method":"notifications/roots/list_changed"}"#;
    // A client with no roots is not asked.
    let mut bare = Server::new(own.clone(), Box::new(InProcess::new(&game)));
    initialized(&mut bare, false);
    assert_eq!(bare.handle(change), Vec::<String>::new());
    let (mut server, written) = forwarding(&own, &own, &game, workspaces);
    assert!(written.is_empty(), "{written:?}");
    assert_eq!(messages(&server.handle(change)), std::slice::from_ref(&ask));
    // Moved to another workspace: its list is followed, with the notice.
    assert_eq!(rooted(&mut server, &theirs), std::slice::from_ref(&changed));
    assert_eq!(server.folder(), theirs.as_path());
    let list = handled(
        &mut server,
        &json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }),
    );
    assert_eq!(
        list["result"]["tools"].as_array().unwrap().last(),
        Some(&fly)
    );
    // A relative project is from the new root.
    called(&mut server, "fly", &json!({ "project": "up" }));
    let got = server.handle(change);
    assert_eq!(messages(&got), [ask]);
    // Back to the server's own workspace: its own list again.
    assert_eq!(rooted(&mut server, &own), [changed]);
    let list = handled(
        &mut server,
        &json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
    );
    assert_eq!(list["result"]["tools"], json!(tools()));
    // And once more it is the same list: nothing to say.
    assert_eq!(rooted(&mut server, &own), Vec::<Value>::new());
    fs::remove_dir_all(&own).unwrap();
    fs::remove_dir_all(&theirs).unwrap();
}

#[test]
fn a_forwarded_server_that_doesnt_build_is_an_error_result_with_its_output() {
    let own = scratch("fail-own", GAME);
    let theirs = scratch("fail-theirs", GAME);
    let game = Fake::default();
    let workspaces = Workspaces::new(&[(&own, "own"), (&theirs, "theirs")]);
    workspaces.fail.set(true);
    let asked = Rc::clone(&workspaces.asked);
    let (mut server, written) = forwarding(&own, &theirs, &game, workspaces);
    assert!(written.is_empty(), "{written:?}");
    for name in ["start", "bots"] {
        let result = called(&mut server, name, &json!({}));
        assert_eq!(result["isError"], true);
        let answer = &result["structuredContent"];
        assert_eq!(
            (answer["op"].as_str(), answer["code"].as_str()),
            (Some(name), Some("SH3005"))
        );
        assert!(
            answer["error"]
                .as_str()
                .unwrap()
                .ends_with("cannot find value"),
            "{answer}"
        );
    }
    assert!(!server.has_game());
    // Each call starts it again: the error may have been fixed.
    assert_eq!(asked.spawned.borrow().len(), 2);
    fs::remove_dir_all(&own).unwrap();
    fs::remove_dir_all(&theirs).unwrap();
}

#[test]
fn a_stale_server_hands_its_own_workspace_to_a_fresh_one() {
    let dir = scratch(
        "restart",
        r#"(command: "cargo shuyi", game: (package: "fake"))"#,
    );
    let newer = VERSION + 1;
    let workspaces = Workspaces::new(&[(&dir, "own")]);
    let asked = Rc::clone(&workspaces.asked);
    let mut server = Server::new(dir.clone(), Box::new(Speaks { contract: newer }))
        .forwarding(Box::new(workspaces));
    // The server's own workspace is its own, until an answer says the
    // engine moved under it.
    let result = called(&mut server, "start", &json!({}));
    assert_eq!(result["structuredContent"]["by"], "forwarded", "{result}");
    assert_eq!(
        *asked.spawned.borrow(),
        [("cargo shuyi mcp".to_owned(), dir.clone())]
    );
    assert!(server.has_game());
    for name in ["step", "bots"] {
        let result = called(&mut server, name, &json!({}));
        assert_eq!(result["structuredContent"]["by"], "forwarded", "{name}");
    }
    let names: Vec<String> = asked
        .called
        .borrow()
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    assert_eq!(names, ["start", "step", "bots"]);

    // If the fresh one doesn't start, the limit says what to do.
    let workspaces = Workspaces::new(&[(&dir, "own")]);
    workspaces.fail.set(true);
    let mut server = Server::new(dir.clone(), Box::new(Speaks { contract: newer }))
        .forwarding(Box::new(workspaces));
    let result = called(&mut server, "bots", &json!({}));
    let limits = result["structuredContent"]["limits"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(limits.len(), 2, "{result}");
    assert!(
        limits[0]
            .as_str()
            .unwrap()
            .contains("run \"cargo shuyi mcp --ready\"")
    );
    assert!(
        limits[1]
            .as_str()
            .unwrap()
            .starts_with("a fresh server of this checkout didn't start: ")
    );
    fs::remove_dir_all(&dir).unwrap();
}

/// Launches hosts whose engine speaks the contract `contract` says when
/// they are launched.
struct Moving {
    contract: Rc<Cell<u32>>,
}

impl<'a> Launch<'a> for Moving {
    fn launch(&self, _: &Project) -> Result<Box<dyn Link + 'a>, (Code, String)> {
        Ok(Box::new(Speaks {
            contract: self.contract.get(),
        }))
    }
}

#[test]
fn a_restart_keeps_the_session_that_runs_and_the_list_the_server_follows() {
    let own = scratch("moved-own", GAME);
    let theirs = scratch("moved-theirs", GAME);
    let contract = Rc::new(Cell::new(VERSION));
    let mut workspaces = Workspaces::new(&[(&own, "own"), (&theirs, "theirs")]);
    workspaces
        .tools
        .push(json!({ "name": "fly", "inputSchema": { "type": "object" } }));
    workspaces.ready.set(false);
    let ready = Rc::clone(&workspaces.ready);
    let asked = Rc::clone(&workspaces.asked);
    let mut server = Server::new(
        own.clone(),
        Box::new(Moving {
            contract: Rc::clone(&contract),
        }),
    )
    .forwarding(Box::new(workspaces));
    let own_project = json!({ "project": own.display().to_string() });
    // A session of the server's own, before the engine moves.
    let result = called(&mut server, "start", &own_project);
    assert_eq!(result["isError"], false, "{result}");
    // The session folder is another workspace's, still building.
    initialized(&mut server, true);
    let got = rooted(&mut server, &theirs);
    assert!(got.is_empty(), "{got:?}");
    // The engine moves: a one-off call goes to a fresh server.
    contract.set(VERSION + 1);
    let result = called(&mut server, "bots", &own_project);
    assert_eq!(result["structuredContent"]["by"], "forwarded", "{result}");
    // The session's game runs on where it ran.
    let result = called(&mut server, "step", &json!({}));
    assert_eq!(result["isError"], false, "{result}");
    assert!(result["structuredContent"]["by"].is_null(), "{result}");
    // The list followed is still the session folder's.
    ready.set(true);
    assert_eq!(server.poll().len(), 1);
    assert_eq!(*asked.listed.borrow(), std::slice::from_ref(&theirs));
    fs::remove_dir_all(&own).unwrap();
    fs::remove_dir_all(&theirs).unwrap();
}
