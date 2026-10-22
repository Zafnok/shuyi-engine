//! The MCP server: JSON-RPC 2.0 messages, one per line ([`Server::handle`]),
//! the methods `initialize`, `ping`, `tools/list` and `tools/call`, and
//! the session.
//!
//! A tool that runs the game is answered by a host of the game's tools
//! ([`super::host`]) that a [`Launch`] starts: the game's tool program,
//! run by cargo ([`super::child::Cargo`]), or one in this process
//! ([`super::host::InProcess`]). A one-off operation gets a host of its
//! own; the session keeps one from `start` to `stop`.

use std::path::PathBuf;

use serde_json::{Value, json};

use super::host::{Reply, Request, failed, no_session, one_off};
use super::{StartInput, takes_project, tool, tools};
use crate::contract::Op;
use crate::contract::codes::{self, Code};
use crate::project::Project;

/// The MCP protocol versions the server speaks, newest first: it answers
/// with the client's if it is one of them, else with the first.
pub const PROTOCOL_VERSIONS: [&str; 4] = ["2025-06-18", "2025-11-25", "2025-03-26", "2024-11-05"];

/// What the server tells a client it is for.
const INSTRUCTIONS: &str = "Shuyi Engine's tool contract on a game. Each operation is a tool; \
    the tools that run the game take `project`, the game's folder. To play a game and look at \
    it: `start` (with `project`), then `step`, `scene`, `frame_png` and `events`, `reset` to \
    start over, `stop` at the end; call `start` again after changing the game's code. A failed \
    answer is an error result with a `code` (`SH0001`): branch on it, and `explain` says what \
    it means.";

/// A host of the game's tools to send requests to.
pub trait Link {
    /// The host's reply to `request`, or why there is none: the host
    /// stopped (with its last output).
    fn call(&mut self, request: &Request) -> Result<Reply, String>;
}

/// Starts hosts of a game's tools.
pub trait Launch<'a> {
    /// A host of `project`'s game, or the problem code and why there is
    /// none.
    fn launch(&self, project: &Project) -> Result<Box<dyn Link + 'a>, (Code, String)>;
}

/// The server: where it runs, how it starts hosts, and its session.
pub struct Server<'a> {
    cwd: PathBuf,
    launch: Box<dyn Launch<'a> + 'a>,
    /// The session's game: the host it runs in.
    game: Option<Box<dyn Link + 'a>>,
}

/// A JSON-RPC error's code: the message isn't JSON.
const PARSE_ERROR: i64 = -32700;
/// The message isn't a request.
const INVALID_REQUEST: i64 = -32600;
/// No such method.
const METHOD_NOT_FOUND: i64 = -32601;
/// Its parameters are wrong.
const INVALID_PARAMS: i64 = -32602;

/// A JSON-RPC response to `id` with `result`.
fn response(id: &Value, result: &Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

/// A JSON-RPC error response to `id`.
fn error(id: &Value, code: i64, message: &str) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }).to_string()
}

/// A tool's MCP result: the answer as text and as structured content,
/// the picture if there is one, and an error result unless the answer is
/// `ok`.
pub fn tool_result(answer: &Value, png: Option<String>) -> Value {
    let text = serde_json::to_string_pretty(answer).unwrap_or_default();
    let mut content = vec![json!({ "type": "text", "text": text })];
    if let Some(png) = png {
        content.push(json!({ "type": "image", "data": png, "mimeType": "image/png" }));
    }
    let ok = answer.get("ok").and_then(Value::as_bool).unwrap_or(false);
    json!({ "content": content, "structuredContent": answer, "isError": !ok })
}

impl<'a> Server<'a> {
    /// A server in the working folder `cwd`, starting hosts with
    /// `launch`.
    pub fn new(cwd: PathBuf, launch: Box<dyn Launch<'a> + 'a>) -> Self {
        Self {
            cwd,
            launch,
            game: None,
        }
    }

    /// Whether a game is running in the session.
    pub fn has_game(&self) -> bool {
        self.game.is_some()
    }

    /// The response to `message`, one JSON-RPC message, if it wants one
    /// (a notification and a response want none).
    pub fn handle(&mut self, message: &str) -> Option<String> {
        if message.trim().is_empty() {
            return None;
        }
        let Ok(message) = serde_json::from_str::<Value>(message) else {
            return Some(error(&Value::Null, PARSE_ERROR, "Parse error"));
        };
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            if message.get("result").is_some() || message.get("error").is_some() {
                // A response to a request of ours: the server sends none.
                return None;
            }
            let id = message.get("id").cloned().unwrap_or(Value::Null);
            return Some(error(&id, INVALID_REQUEST, "Invalid Request"));
        };
        // A notification (`notifications/initialized`, `…/cancelled`):
        // nothing to answer.
        let id = message.get("id")?;
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        Some(match method {
            "initialize" => response(id, &initialize(&params)),
            "ping" => response(id, &json!({})),
            "tools/list" => response(id, &json!({ "tools": tools() })),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let Some((op, session)) = tool(name) else {
                    return Some(error(id, INVALID_PARAMS, &format!("Unknown tool: {name}")));
                };
                let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
                let (answer, png) = self.call(op, session, arguments);
                response(id, &tool_result(&answer, png))
            }
            _ => error(id, METHOD_NOT_FOUND, &format!("Method not found: {method}")),
        })
    }

    /// The answer of the tool of `op` (a session tool if `session`) to
    /// `arguments`, as JSON, and its picture, if it has one.
    pub fn call(&mut self, op: Op, session: bool, arguments: Value) -> (Value, Option<String>) {
        if op == Op::Start {
            return (self.start(arguments), None);
        }
        if session {
            return self.in_session(op, arguments);
        }
        let mut arguments = if arguments.is_null() {
            json!({})
        } else {
            arguments
        };
        let folder = match arguments.as_object_mut().map(|a| a.remove("project")) {
            Some(Some(Value::String(folder))) => Some(folder),
            Some(None | Some(Value::Null)) | None => None,
            Some(Some(other)) => {
                let error = format!("project: a folder's path, not {other}");
                return (failed(op, codes::WRONG_ARGUMENTS, error), None);
            }
        };
        if folder.is_some() && !takes_project(op, false) {
            let error = format!("{} takes no project", op.name());
            return (failed(op, codes::WRONG_ARGUMENTS, error), None);
        }
        let project = self.project(folder.as_deref());
        let runs_in_game = crate::contract::cli::runs_the_game(op.name())
            && project
                .as_ref()
                .is_ok_and(|project| project.config.game.is_some());
        if !runs_in_game {
            return (one_off(op, arguments, &self.cwd, &project, None), None);
        }
        let Ok(project) = project else {
            return (one_off(op, arguments, &self.cwd, &project, None), None);
        };
        let mut host = match self.launch.launch(&project) {
            Ok(host) => host,
            Err((code, e)) => return (failed(op, code, e), None),
        };
        let request = Request {
            op,
            input: arguments,
        };
        match host.call(&request) {
            Ok(reply) => answer_of(op, reply),
            Err(e) => (failed(op, codes::GAME_PROGRAM, e), None),
        }
    }

    /// The project of `folder` (from the working folder), or the
    /// server's own.
    fn project(&self, folder: Option<&str>) -> Result<Project, (Code, String)> {
        Project::find(&folder.map_or_else(|| self.cwd.clone(), |f| self.cwd.join(f)))
    }

    /// `start`: a host of the project's game launched, the game started
    /// in it, and kept as the session's, in place of one running.
    fn start(&mut self, arguments: Value) -> Value {
        let op = Op::Start;
        let input: StartInput = match serde_json::from_value(if arguments.is_null() {
            json!({})
        } else {
            arguments
        }) {
            Ok(input) => input,
            Err(e) => return failed(op, codes::WRONG_ARGUMENTS, format!("input: {e}")),
        };
        let mut limits = Vec::new();
        if let Some(mut old) = self.game.take() {
            let stop = Request {
                op: Op::Stop,
                input: json!({}),
            };
            let frames = old
                .call(&stop)
                .ok()
                .and_then(|reply| reply.answer)
                .and_then(|answer| answer["result"]["frames"].as_u64());
            limits.push(match frames {
                Some(frames) => format!("the game that was running was stopped on frame {frames}"),
                None => "the game that was running was stopped".to_owned(),
            });
        }
        let project = match self.project(input.project.as_deref()) {
            Ok(project) => project,
            Err((code, e)) => return failed(op, code, e),
        };
        if project.config.game.is_none() {
            let error = format!(
                "{} names no game tool program (`game`), and start runs the game",
                crate::project::FILE
            );
            return failed(op, codes::PROJECT_FILE, error);
        }
        let mut host = match self.launch.launch(&project) {
            Ok(host) => host,
            Err((code, e)) => return failed(op, code, e),
        };
        let request = Request {
            op,
            input: json!({ "start": input.start }),
        };
        let (mut answer, _) = match host.call(&request) {
            Ok(reply) => answer_of(op, reply),
            Err(e) => return failed(op, codes::GAME_PROGRAM, e),
        };
        if answer["ok"] == true {
            self.game = Some(host);
        }
        if let Some(list) = answer["limits"].as_array_mut() {
            list.extend(limits.into_iter().map(Value::String));
        }
        answer
    }

    /// Session tool `op` with `arguments`, on the session's game.
    fn in_session(&mut self, op: Op, arguments: Value) -> (Value, Option<String>) {
        let Some(game) = self.game.as_mut() else {
            return (no_session(op), None);
        };
        let request = Request {
            op,
            input: arguments,
        };
        match game.call(&request) {
            Ok(reply) => {
                let (answer, png) = answer_of(op, reply);
                if op == Op::Stop && answer["ok"] == true {
                    self.game = None;
                }
                (answer, png)
            }
            Err(e) => {
                self.game = None;
                (failed(op, codes::GAME_PROGRAM, e), None)
            }
        }
    }
}

/// The answer and the picture of `reply` to the tool of `op`: a host
/// that answered nothing is a [`codes::GAME_PROGRAM`] answer.
fn answer_of(op: Op, reply: Reply) -> (Value, Option<String>) {
    match (reply.answer, reply.error) {
        (Some(answer), _) => (answer, reply.png),
        (None, error) => {
            let error = error.unwrap_or_else(|| "the game's tool program answered nothing".into());
            (failed(op, codes::GAME_PROGRAM, error), None)
        }
    }
}

/// `initialize`'s result: the protocol version, the tools, the server.
fn initialize(params: &Value) -> Value {
    let asked = params.get("protocolVersion").and_then(Value::as_str);
    let version = asked
        .filter(|asked| PROTOCOL_VERSIONS.contains(asked))
        .unwrap_or(PROTOCOL_VERSIONS[0]);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "shuyi", "version": env!("CARGO_PKG_VERSION") },
        "instructions": INSTRUCTIONS,
    })
}

#[cfg(test)]
mod tests;
