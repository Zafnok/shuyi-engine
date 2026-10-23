//! The MCP server: JSON-RPC 2.0 messages, one per line ([`Server::handle`]),
//! the methods `initialize`, `ping`, `tools/list` and `tools/call`, and
//! the session.
//!
//! A tool that runs the game is answered by a host of the game's tools
//! ([`super::host`]) that a [`Launch`] starts: the game's tool program,
//! run by cargo ([`super::child::Cargo`]), or one in this process
//! ([`super::host::InProcess`]). A one-off operation gets a host of its
//! own; the session keeps one from `start` to `stop`.
//!
//! The session's folder is the client's first root (`roots/list`), or the
//! working folder: a relative `project` is from it. A call whose project
//! is in another cargo workspace than the server's goes to that
//! workspace's own server ([`super::forward`]), and the tools the server
//! lists follow the session folder's.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::forward::{Forward, Forwarding, path_of_uri};
use super::host::{Reply, Request, failed, no_session, one_off};
use super::{StartInput, takes_project, tool, tools};
use crate::contract::codes::{self, Code};
use crate::contract::{Op, VERSION};
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

/// The id of the server's `roots/list` request.
pub const ROOTS: &str = "shuyi-roots";

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
    /// The session's project's `command`, for what its answers say to
    /// run.
    command: String,
    /// How it starts the projects' own servers: none in one of them, which
    /// answers every call itself.
    forwarding: Option<Box<dyn Forwarding<'a> + 'a>>,
    /// Whether the client lists its roots.
    roots: bool,
    /// The session's folder: the client's first root, once it said.
    folder: Option<PathBuf>,
    /// The workspace of each project root looked up, none outside one.
    workspaces: BTreeMap<PathBuf, Option<PathBuf>>,
    /// The projects' own servers, by workspace.
    forwarded: BTreeMap<PathBuf, Box<dyn Forward + 'a>>,
    /// The workspace whose server runs the session's game, if not this
    /// server.
    session_at: Option<PathBuf>,
    /// The workspace whose server's tools the server will list, once it
    /// has answered `initialize`.
    follow: Option<PathBuf>,
    /// The workspace whose server's tools it lists, if not its own.
    listing: Option<PathBuf>,
    /// The tools it lists.
    tools: Vec<Value>,
    /// Whether its own workspace's calls go to a fresh server of it: the
    /// engine moved under it.
    restarted: bool,
}

/// A JSON-RPC error's code: the message isn't JSON.
const PARSE_ERROR: i64 = -32700;
/// The message isn't a request.
const INVALID_REQUEST: i64 = -32600;
/// No such method.
const METHOD_NOT_FOUND: i64 = -32601;
/// Its parameters are wrong.
const INVALID_PARAMS: i64 = -32602;
/// The server couldn't answer.
const INTERNAL_ERROR: i64 = -32603;

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

/// The root of the project at `folder`, or `folder` outside one.
fn project_root(folder: &Path) -> PathBuf {
    Project::find(folder).map_or_else(|_| folder.to_path_buf(), |project| project.root)
}

impl<'a> Server<'a> {
    /// A server in the working folder `cwd`, starting hosts with
    /// `launch`. It answers every call itself until given
    /// [`Self::forwarding`].
    pub fn new(cwd: PathBuf, launch: Box<dyn Launch<'a> + 'a>) -> Self {
        Self {
            cwd,
            launch,
            game: None,
            command: "shuyi".to_owned(),
            forwarding: None,
            roots: false,
            folder: None,
            workspaces: BTreeMap::new(),
            forwarded: BTreeMap::new(),
            session_at: None,
            follow: None,
            listing: None,
            tools: tools(),
            restarted: false,
        }
    }

    /// The server, handing each call whose project is in another workspace
    /// to that workspace's own server, which `forwarding` starts.
    #[must_use]
    pub fn forwarding(mut self, forwarding: Box<dyn Forwarding<'a> + 'a>) -> Self {
        self.forwarding = Some(forwarding);
        self
    }

    /// Whether a game is running in the session.
    pub fn has_game(&self) -> bool {
        self.game.is_some() || self.session_at.is_some()
    }

    /// The session's folder: the client's first root, or the working
    /// folder.
    pub fn folder(&self) -> &Path {
        self.folder.as_deref().unwrap_or(&self.cwd)
    }

    /// The messages to write for `message`, one JSON-RPC message: its
    /// response if it wants one (a notification and a response want none),
    /// and the server's own request or notification it led to.
    pub fn handle(&mut self, message: &str) -> Vec<String> {
        if message.trim().is_empty() {
            return Vec::new();
        }
        let Ok(message) = serde_json::from_str::<Value>(message) else {
            return vec![error(&Value::Null, PARSE_ERROR, "Parse error")];
        };
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            if message.get("result").is_some() || message.get("error").is_some() {
                // A response to a request of ours: taken, not answered.
                if message.get("id") == Some(&json!(ROOTS)) {
                    return self.rooted(&message);
                }
                return Vec::new();
            }
            let id = message.get("id").cloned().unwrap_or(Value::Null);
            return vec![error(&id, INVALID_REQUEST, "Invalid Request")];
        };
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        // A notification (`notifications/initialized`, `…/cancelled`):
        // nothing to answer, but the roots to ask for.
        let Some(id) = message.get("id") else {
            if method == "notifications/initialized" && self.roots {
                let ask = json!({ "jsonrpc": "2.0", "id": ROOTS, "method": "roots/list" });
                return vec![ask.to_string()];
            }
            return Vec::new();
        };
        vec![match method {
            "initialize" => {
                self.roots = params["capabilities"]["roots"].is_object();
                response(id, &initialize(&params))
            }
            "ping" => response(id, &json!({})),
            "tools/list" => response(id, &json!({ "tools": self.tools })),
            "tools/call" => self.tool_call(id, &params),
            _ => error(id, METHOD_NOT_FOUND, &format!("Method not found: {method}")),
        }]
    }

    /// The client's roots, `answer`: the first is the session's folder. If
    /// it is in another workspace than the server's, that workspace's
    /// server starts now, so its build runs while the agent works, and the
    /// server will list its tools.
    fn rooted(&mut self, answer: &Value) -> Vec<String> {
        let Some(folder) = answer["result"]["roots"][0]["uri"]
            .as_str()
            .and_then(path_of_uri)
        else {
            return Vec::new();
        };
        let root = project_root(&folder);
        self.folder = Some(folder);
        if let Some(workspace) = self.forward_to(&root) {
            self.follow = Some(workspace);
        }
        self.poll()
    }

    /// The notification the client gets when the tools it should list
    /// changed: once the followed server has answered `initialize`, its
    /// list, if it isn't the server's. Asks nothing of a server still
    /// building.
    pub fn poll(&mut self) -> Vec<String> {
        let Some(workspace) = self.follow.clone() else {
            return Vec::new();
        };
        let Some(server) = self.forwarded.get_mut(&workspace) else {
            self.follow = None;
            return Vec::new();
        };
        match server.ready(false) {
            None => return Vec::new(),
            Some(Err(_)) => {
                // Its next call says why.
                self.follow = None;
                return Vec::new();
            }
            Some(Ok(())) => self.follow = None,
        }
        let Ok(listed) = server.request("tools/list", &json!({})) else {
            return Vec::new();
        };
        match listed["result"]["tools"].as_array() {
            Some(list) if *list != self.tools => {
                self.tools.clone_from(list);
                self.listing = Some(workspace);
                let changed =
                    json!({ "jsonrpc": "2.0", "method": "notifications/tools/list_changed" });
                vec![changed.to_string()]
            }
            _ => Vec::new(),
        }
    }

    /// The workspace of the project root `root`, looked up once.
    fn workspace_of(&mut self, root: &Path) -> Option<PathBuf> {
        let forwarding = self.forwarding.as_ref()?;
        self.workspaces
            .entry(root.to_path_buf())
            .or_insert_with(|| forwarding.workspace(root))
            .clone()
    }

    /// The workspace of the server whose project root is `root`, when
    /// another server answers for it (one of another workspace, or a fresh
    /// one of its own once restarted), started now if it isn't running.
    /// None when this server answers.
    fn forward_to(&mut self, root: &Path) -> Option<PathBuf> {
        let workspace = self.workspace_of(root)?;
        let own = self.workspace_of(&self.cwd.clone());
        if own.as_ref() == Some(&workspace) && !self.restarted {
            return None;
        }
        if !self.forwarded.contains_key(&workspace) {
            let (words, run_in) = super::forward::words(root, &workspace)?;
            let server = self.forwarding.as_ref()?.spawn(&words, &run_in);
            self.forwarded.insert(workspace.clone(), server);
        }
        Some(workspace)
    }

    /// The response of the server of `workspace` to the tool `name` with
    /// `arguments`, under the client's `id`, or why it has none: it
    /// stopped, and is forgotten, so the next call starts it again.
    fn forwarded_call(
        &mut self,
        workspace: &Path,
        id: &Value,
        name: &str,
        arguments: &Value,
    ) -> Result<String, String> {
        let Some(server) = self.forwarded.get_mut(workspace) else {
            return Err("the project's own server isn't running".to_owned());
        };
        let params = json!({ "name": name, "arguments": arguments });
        match server.request("tools/call", &params) {
            Ok(mut answer) => {
                answer["id"] = id.clone();
                Ok(answer.to_string())
            }
            Err(why) => {
                self.forwarded.remove(workspace);
                if self.session_at.as_deref() == Some(workspace) {
                    self.session_at = None;
                }
                Err(why)
            }
        }
    }

    /// The response to `tools/call` with `params`, under `id`.
    fn tool_call(&mut self, id: &Value, params: &Value) -> String {
        let name = params.get("name").and_then(Value::as_str).unwrap_or("");
        let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
        let Some((op, session)) = tool(name) else {
            // A tool of the list it follows that it doesn't know itself.
            let listed = self.tools.iter().any(|tool| tool["name"] == name);
            if let Some(workspace) = self.listing.clone().filter(|_| listed) {
                let mut arguments = arguments;
                if let Some(Value::String(project)) = arguments.get("project") {
                    let project = self.folder().join(project).display().to_string();
                    arguments["project"] = json!(project);
                }
                return self
                    .forwarded_call(&workspace, id, name, &arguments)
                    .unwrap_or_else(|why| error(id, INTERNAL_ERROR, &why));
            }
            return error(id, INVALID_PARAMS, &format!("Unknown tool: {name}"));
        };
        let gone = |op: Op, why: String| {
            response(
                id,
                &tool_result(&failed(op, codes::GAME_PROGRAM, why), None),
            )
        };
        if session && op != Op::Start {
            if let Some(workspace) = self.session_at.clone() {
                return self
                    .forwarded_call(&workspace, id, name, &arguments)
                    .unwrap_or_else(|why| gone(op, why));
            }
            let (answer, png) = self.call(op, session, arguments);
            return response(id, &tool_result(&answer, png));
        }
        let (arguments, root) = self.absolute(op, session, arguments);
        if let Some(workspace) = self.forward_to(&root) {
            return self.forwarded_tool(&workspace, id, op, &arguments);
        }
        if op == Op::Start {
            self.end_session_elsewhere(None);
        }
        let (answer, png) = self.call(op, session, arguments.clone());
        if self.forwarding.is_some()
            && !self.restarted
            && stale(&answer, VERSION, "").is_some()
            && self
                .workspace_of(&root)
                .is_some_and(|ws| Some(ws) == self.workspace_of(&self.cwd.clone()))
        {
            // The engine moved under the server: a fresh server of its own
            // workspace answers this call and every later one.
            self.restarted = true;
            if op == Op::Start {
                self.game = None;
            }
            if let Some(workspace) = self.forward_to(&root) {
                let name = op.name();
                match self.forwarded_call(&workspace, id, name, &arguments) {
                    Ok(answered) => {
                        if self.follow.is_none() && self.listing.is_none() {
                            self.follow = Some(workspace.clone());
                        }
                        if op == Op::Start && is_ok(&answered) {
                            self.session_at = Some(workspace);
                        }
                        return answered;
                    }
                    Err(why) => {
                        self.restarted = false;
                        let mut answer = answer;
                        if let Some(limits) = answer["limits"].as_array_mut() {
                            limits.push(Value::String(format!(
                                "a fresh server of this checkout didn't start: {why}"
                            )));
                        }
                        return response(id, &tool_result(&answer, png));
                    }
                }
            }
            self.restarted = false;
        }
        response(id, &tool_result(&answer, png))
    }

    /// The response of the server of `workspace` to the tool of `op` (not
    /// a session tool, or `start`) with `arguments`, under `id`; a `start`
    /// there ends the session's game anywhere else.
    fn forwarded_tool(
        &mut self,
        workspace: &Path,
        id: &Value,
        op: Op,
        arguments: &Value,
    ) -> String {
        if op == Op::Start {
            self.end_session_elsewhere(Some(workspace));
        }
        match self.forwarded_call(workspace, id, op.name(), arguments) {
            Ok(answered) => {
                if op == Op::Start && is_ok(&answered) {
                    self.session_at = Some(workspace.to_path_buf());
                }
                answered
            }
            Err(why) => response(
                id,
                &tool_result(&failed(op, codes::GAME_PROGRAM, why), None),
            ),
        }
    }

    /// Stops the session's game unless it runs in the server of `at`
    /// (this server's own if none): `start` is starting one there.
    fn end_session_elsewhere(&mut self, at: Option<&Path>) {
        if let Some(workspace) = self.session_at.take_if(|w| Some(w.as_path()) != at)
            && let Some(server) = self.forwarded.get_mut(&workspace)
        {
            let _ = server.request("tools/call", &json!({ "name": "stop", "arguments": {} }));
        }
        if at.is_some()
            && let Some(mut game) = self.game.take()
        {
            let _ = game.call(&Request {
                op: Op::Stop,
                input: json!({}),
            });
        }
    }

    /// `arguments` of the tool of `op` (not a session tool, or `start`)
    /// with `project` made absolute from the session's folder, the folder
    /// itself if left out, and `new`'s `dir` absolute too; and the root of
    /// that project (the folder itself outside one).
    fn absolute(&self, op: Op, session: bool, arguments: Value) -> (Value, PathBuf) {
        let mut arguments = if arguments.is_null() {
            json!({})
        } else {
            arguments
        };
        let folder = self.folder().to_path_buf();
        let mut at = folder.clone();
        if let Some(arguments) = arguments.as_object_mut() {
            if takes_project(op, session) || op == Op::Start {
                match arguments.get("project") {
                    Some(Value::String(project)) => at = folder.join(project),
                    None | Some(Value::Null) => {}
                    // Wrong: the call says so.
                    Some(_) => return (Value::Object(arguments.clone()), project_root(&at)),
                }
                arguments.insert("project".to_owned(), json!(at.display().to_string()));
            }
            if op == Op::New
                && let Some(Value::String(dir)) = arguments.get("dir")
            {
                let dir = folder.join(dir).display().to_string();
                arguments.insert("dir".to_owned(), json!(dir));
            }
        }
        let root = project_root(&at);
        (arguments, root)
    }

    /// The answer of the tool of `op` (a session tool if `session`) to
    /// `arguments`, as JSON, and its picture, if it has one: this server's
    /// own.
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
        let cwd = self.folder().to_path_buf();
        if !runs_in_game {
            return (one_off(op, arguments, &cwd, &project, None), None);
        }
        let Ok(project) = project else {
            return (one_off(op, arguments, &cwd, &project, None), None);
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
            Ok(reply) => answer_of(op, reply, project.config.command()),
            Err(e) => (failed(op, codes::GAME_PROGRAM, e), None),
        }
    }

    /// The project of `folder` (from the session's folder), or the
    /// session folder's own.
    fn project(&self, folder: Option<&str>) -> Result<Project, (Code, String)> {
        let at = self.folder();
        Project::find(&folder.map_or_else(|| at.to_path_buf(), |f| at.join(f)))
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
            Ok(reply) => answer_of(op, reply, project.config.command()),
            Err(e) => return failed(op, codes::GAME_PROGRAM, e),
        };
        if answer["ok"] == true {
            self.game = Some(host);
            project.config.command().clone_into(&mut self.command);
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
                let (answer, png) = answer_of(op, reply, &self.command);
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

/// Whether the response `answered` holds a tool result that isn't an
/// error.
fn is_ok(answered: &str) -> bool {
    serde_json::from_str::<Value>(answered).is_ok_and(|answer| answer["result"]["isError"] == false)
}

/// The answer and the picture of `reply` to the tool of `op`, in a
/// project whose `command` is `command`: a host that answered nothing is a
/// [`codes::GAME_PROGRAM`] answer, and one of another contract than the
/// server's gets the limit that says so ([`stale`]).
fn answer_of(op: Op, reply: Reply, command: &str) -> (Value, Option<String>) {
    match (reply.answer, reply.error) {
        (Some(mut answer), _) => {
            if let Some(limit) = stale(&answer, VERSION, command)
                && let Some(limits) = answer["limits"].as_array_mut()
            {
                limits.push(Value::String(limit));
            }
            (answer, reply.png)
        }
        (None, error) => {
            let error = error.unwrap_or_else(|| "the game's tool program answered nothing".into());
            (failed(op, codes::GAME_PROGRAM, error), None)
        }
    }
}

/// The limit a host's `answer` gets when its engine speaks another
/// contract than `version`, the server's: the server was built before the
/// engine changed, and `command mcp --ready` and a reconnect bring it up to
/// date. None when they speak the same one: a change of the engine that
/// changes no schema leaves the server's tools right. A server that
/// forwards first hands the call to a fresh server of its own workspace,
/// and writes this only if that one doesn't start.
pub fn stale(answer: &Value, version: u32, command: &str) -> Option<String> {
    let theirs = answer.get("contract")?.as_u64()?;
    (theirs != u64::from(version)).then(|| {
        format!(
            "the game's engine speaks contract {theirs} and this server {version}: the server was \
             built before the engine changed; run \"{command} mcp --ready\", then reconnect it \
             (in Claude Code: /mcp)"
        )
    })
}

/// `initialize`'s result: the protocol version, the tools, the server.
fn initialize(params: &Value) -> Value {
    let asked = params.get("protocolVersion").and_then(Value::as_str);
    let version = asked
        .filter(|asked| PROTOCOL_VERSIONS.contains(asked))
        .unwrap_or(PROTOCOL_VERSIONS[0]);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": true } },
        "serverInfo": { "name": "shuyi", "version": env!("CARGO_PKG_VERSION") },
        "instructions": INSTRUCTIONS,
    })
}

#[cfg(test)]
mod tests;
