//! The MCP server: JSON-RPC 2.0 messages, one per line ([`Server::handle`]),
//! in both generations of the protocol, and the session.
//!
//! - **The old protocol** ([`PROTOCOL_VERSIONS`]): `initialize`, `ping`,
//!   `tools/list` and `tools/call`, for a client that starts with
//!   `initialize` and sends no version in a request's `_meta`.
//! - **2026-07-28** (`STATELESS`): a request whose `params._meta` holds
//!   `io.modelcontextprotocol/protocolVersion`. `server/discover`,
//!   `tools/list`, `tools/call` and `subscriptions/listen`; each result
//!   carries `resultType` and the server's `serverInfo`, and a version the
//!   server doesn't know is `-32022`.
//!
//! The session (`start` … `stop`) is the process's on both: on stdio one
//! client has the process, so it needs no handle.
//!
//! A tool that runs the game is answered by a host of the game's tools
//! ([`super::host`]) that a [`Launch`] starts: the game's tool program,
//! run by cargo ([`super::child::Cargo`]), or one in this process
//! ([`super::host::InProcess`]). A one-off operation gets a host of its
//! own; the session keeps one from `start` to `stop`.
//!
//! The session's folder is the client's first root, or the working folder:
//! a relative `project` is from it. On the old protocol the server asks
//! `roots/list` itself; on 2026-07-28 it answers the first `tools/call`
//! `input_required` with a `roots/list` to fill in, once. A call whose project
//! is in another cargo workspace than the server's goes to that
//! workspace's own server ([`super::forward`]), and the tools the server
//! lists follow the session folder's.
//!
//! On 2026-07-28, a long tool's call from a client that declared MCP's
//! tasks extension answers a task at once (`mcp::tasks`): the host's
//! reply is awaited while the server answers `tasks/get`, and the session
//! has no game until `start`'s task completes (ADR-0159). A call another
//! workspace's server answers is a task too, its call sent once that
//! server is ready (ADR-0162).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde_json::{Value, json};

use super::attached::Attached;
use super::forward::{Forward, Forwarding, path_of_uri};
use super::host::{Reply, Request, failed, no_session, one_off};
use super::tasks::{EXTENSION, State, Tasks};
use super::{StartInput, takes_project, tasked, tool, tools};
use crate::contract::codes::{self, Code};
use crate::contract::{Op, VERSION};
use crate::project::Project;

/// The versions of the old MCP protocol, the one `initialize` starts,
/// that the server speaks: it answers with the client's if it is one of
/// them, else with the first. 2026-07-28 has no `initialize`, so it isn't
/// one of them (`STATELESS`).
pub const PROTOCOL_VERSIONS: [&str; 4] = ["2025-06-18", "2025-11-25", "2025-03-26", "2024-11-05"];

/// What the server tells a client it is for.
const INSTRUCTIONS: &str = "Shuyi Engine's tool contract on a game. Each operation is a tool; \
    the tools that run the game take `project`, the game's folder. To play a game and look at \
    it: `start` (with `project`), then `step`, `scene`, `frame_png` and `events` (`perf` for \
    the frames' times), `reset` to start over, `stop` at the end; call `start` again after changing the game's code. `start` \
    with `link` attaches the session to a debug build running in its window instead. A failed \
    answer is an error result with a `code` (`SH0001`): branch on it, and `explain` says what \
    it means. Before guessing a type's, a function's or a file's name, call `docs` (`query`: \
    `play a sound`, `Sprite`): it searches the engine's Learn pages, skills, examples and \
    problem codes at the game's engine revision.";

/// The id of the server's `roots/list` request, and on 2026-07-28 the key
/// of its `inputRequests` entry.
pub const ROOTS: &str = "shuyi-roots";

/// The MCP revision whose requests each carry their version in `_meta`,
/// with no `initialize`: the only one `server/discover` lists.
pub(crate) const STATELESS: &str = "2026-07-28";

/// A request's `_meta` key naming its protocol version.
const META_VERSION: &str = "io.modelcontextprotocol/protocolVersion";
/// A request's `_meta` key holding the client's capabilities.
const META_CAPABILITIES: &str = "io.modelcontextprotocol/clientCapabilities";
/// A result's `_meta` key naming the server.
const META_SERVER: &str = "io.modelcontextprotocol/serverInfo";
/// A notification's `_meta` key naming the subscription it is sent on.
const META_SUBSCRIPTION: &str = "io.modelcontextprotocol/subscriptionId";

/// How long a client may keep what doesn't change while the process runs
/// (`server/discover`, and a list that follows no folder): an hour, in
/// milliseconds.
const KEEP_MS: u64 = 3_600_000;

/// A host of the game's tools to send requests to.
pub trait Link {
    /// The host's reply to `request`, or why there is none: the host
    /// stopped (with its last output).
    fn call(&mut self, request: &Request) -> Result<Reply, String>;

    /// Sends `request` without waiting for its reply, which [`Self::reply`]
    /// then gives, so the server answers while the host works (a task); or
    /// why it couldn't. False if the host only answers as [`Self::call`]
    /// waits (the default): the server calls it when it next looks.
    fn send(&mut self, _: &Request) -> Result<bool, String> {
        Ok(false)
    }

    /// The reply to the request [`Self::send`] sent, once it has come, or
    /// why there is none; none while the host works.
    fn reply(&mut self) -> Option<Result<Reply, String>> {
        None
    }
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
    /// The session's build, when it is attached to one over the live link
    /// instead.
    attached: Option<Attached>,
    /// The session's project's `command`, for what its answers say to
    /// run.
    command: String,
    /// How it starts the projects' own servers: none in one of them, which
    /// answers every call itself.
    forwarding: Option<Box<dyn Forwarding<'a> + 'a>>,
    /// What it knows of its client.
    client: Client,
    /// The ids of the `subscriptions/listen` requests that want to hear
    /// when the tools change.
    subscriptions: Vec<Value>,
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
    /// The tasks of the process, and the calls that wait on their hosts or
    /// on another server.
    tasks: Tasks<Work<'a>>,
    /// The time now, for a task's times.
    now: fn() -> SystemTime,
}

/// A tool's call that waits on its host's reply: a task's work, or the
/// call itself while it waits.
struct Pending<'a> {
    op: Op,
    /// The host it waits on: dropping it stops the host (a cancel).
    host: Box<dyn Link + 'a>,
    request: Request,
    /// Whether the request was sent without waiting ([`Link::send`]).
    sent: bool,
    /// The project's `command`, for what the answer says to run.
    command: String,
    /// `start`'s limits on what it did before: the game it stopped.
    limits: Vec<String>,
    /// The call's arguments, its `project` absolute, and the project's
    /// root: a fresh server takes the call if the engine moved.
    arguments: Value,
    root: PathBuf,
}

/// A task's work: a call that waits on its host here, or on another
/// workspace's server.
enum Work<'a> {
    Here(Pending<'a>),
    There(Away),
}

/// A tool's call that another workspace's server answers (ADR-0162): a
/// task's work while that server builds itself, then the call.
struct Away {
    /// The workspace whose server answers: dropping that server stops it
    /// (a cancel).
    workspace: PathBuf,
    op: Op,
    /// The `tools/call` parameters it is sent.
    params: Value,
    /// The id it was sent under ([`Forward::send`]); none until the server
    /// is ready.
    id: Option<u64>,
    /// When it is the engine-moved restart's call to a fresh server of the
    /// server's own workspace: this server's own answer and picture, which
    /// it answers if that one doesn't start.
    fallback: Option<(Value, Option<String>)>,
}

impl Away {
    /// The call of the tool of `op` with `arguments`, to the server of
    /// `workspace`.
    fn new(workspace: PathBuf, op: Op, arguments: &Value) -> Self {
        Self {
            workspace,
            op,
            params: json!({ "name": op.name(), "arguments": arguments }),
            id: None,
            fallback: None,
        }
    }
}

/// What came for a task's work: its host's reply, or its server's
/// response.
enum Replied {
    Host(Result<Reply, String>),
    Server(Result<Value, String>),
}

/// Why a call to a workspace's server has no answer: it isn't running.
const NOT_RUNNING: &str = "the project's own server isn't running";

/// The response of `away`'s server to its call, once it has come, or why
/// there is none: none while that server builds or works. The call is
/// sent once the server is ready; to a server that only answers as it
/// waits, it is made then if `may_wait`.
fn poll_away<'a>(
    forwarded: &mut BTreeMap<PathBuf, Box<dyn Forward + 'a>>,
    away: &mut Away,
    may_wait: bool,
) -> Option<Result<Value, String>> {
    let Some(server) = forwarded.get_mut(&away.workspace) else {
        return Some(Err(NOT_RUNNING.to_owned()));
    };
    if let Some(id) = away.id {
        return server.response(id);
    }
    if let Err(why) = server.ready(false)? {
        return Some(Err(why));
    }
    match server.send("tools/call", &away.params) {
        Ok(Some(id)) => {
            away.id = Some(id);
            server.response(id)
        }
        Ok(None) if may_wait => Some(server.request("tools/call", &away.params)),
        Ok(None) => None,
        Err(why) => Some(Err(why)),
    }
}

/// How a task ended on the response `answered` (under no id): completed
/// with its result, or failed with its JSON-RPC error.
fn ended_as<W>(mut answered: Value) -> State<W> {
    match answered.get_mut("result") {
        Some(result) => State::Completed(result.take()),
        None => State::Failed(answered["error"].take()),
    }
}

/// How far a call got before its host's reply.
enum Prepared<'a> {
    /// It is answered: no host was needed, or none started.
    Answered(Value, Option<String>),
    /// It waits on its host.
    Waits(Pending<'a>),
}

/// What the server knows of its client, from what it sent.
#[derive(Debug, Default)]
struct Client {
    /// Whether it lists its roots (the old protocol's `initialize` said).
    roots: bool,
    /// Whether it said `initialize`: it hears that the tools changed by
    /// the old protocol's notification.
    initialized: bool,
    /// Whether, on 2026-07-28, it was asked for its roots or gave them: it
    /// is asked once.
    asked_roots: bool,
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
/// The request's `_meta` names a protocol version the server doesn't
/// speak (2026-07-28).
const UNSUPPORTED_VERSION: i64 = -32022;
/// The request needs a capability its client didn't declare: a task's,
/// from a client without the tasks extension.
const MISSING_CAPABILITY: i64 = -32021;

/// A JSON-RPC response to `id` with `result`.
fn response(id: &Value, result: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// A JSON-RPC error response to `id`.
fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// The server's name and version, as `initialize` and each 2026-07-28
/// result give them.
fn server_info() -> Value {
    json!({ "name": "shuyi", "version": env!("CARGO_PKG_VERSION") })
}

/// `message` as a 2026-07-28 response: its result, if it has one, with
/// `result_type` and the server's `serverInfo`. An error is left as it
/// is.
fn of_2026(mut message: Value, result_type: &str) -> Value {
    if let Some(result) = message.get_mut("result").and_then(Value::as_object_mut) {
        result.insert("resultType".to_owned(), json!(result_type));
        let meta = result.entry("_meta").or_insert_with(|| json!({}));
        if let Some(meta) = meta.as_object_mut() {
            meta.insert(META_SERVER.to_owned(), server_info());
        }
    }
    message
}

/// The error for a request whose `_meta` names the version `requested`,
/// which isn't [`STATELESS`].
fn unsupported(id: &Value, requested: &Value) -> Value {
    let requested = requested
        .as_str()
        .map_or_else(|| requested.to_string(), str::to_owned);
    json!({ "jsonrpc": "2.0", "id": id, "error": {
        "code": UNSUPPORTED_VERSION,
        "message": "Unsupported protocol version",
        "data": { "supported": [STATELESS], "requested": requested },
    } })
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
            attached: None,
            command: "shuyi".to_owned(),
            forwarding: None,
            client: Client::default(),
            subscriptions: Vec::new(),
            folder: None,
            workspaces: BTreeMap::new(),
            forwarded: BTreeMap::new(),
            session_at: None,
            follow: None,
            listing: None,
            tools: tools(),
            restarted: false,
            tasks: Tasks::default(),
            now: SystemTime::now,
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
        self.game.is_some() || self.attached.is_some() || self.session_at.is_some()
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
        self.handled(message).iter().map(Value::to_string).collect()
    }

    /// [`Self::handle`]'s messages, as JSON.
    fn handled(&mut self, message: &str) -> Vec<Value> {
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
                    return self.rooted(&message["result"]);
                }
                return Vec::new();
            }
            let id = message.get("id").cloned().unwrap_or(Value::Null);
            return vec![error(&id, INVALID_REQUEST, "Invalid Request")];
        };
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        let Some(id) = message.get("id") else {
            return self.notified(method, &params);
        };
        // `initialize` is the old protocol's, whatever it carries.
        let version = params["_meta"]
            .get(META_VERSION)
            .filter(|_| method != "initialize");
        match version {
            None => vec![self.of_old(id, method, &params)],
            Some(Value::String(version)) if version == STATELESS => {
                self.of_stateless(id, method, &params)
            }
            Some(other) => vec![unsupported(id, other)],
        }
    }

    /// What the notification `method` with `params` leads to: nothing to
    /// answer, but on the old protocol the roots to ask for
    /// (`notifications/initialized`), again when they change; and a
    /// subscription ended (`notifications/cancelled`).
    fn notified(&mut self, method: &str, params: &Value) -> Vec<Value> {
        match method {
            "notifications/initialized" | "notifications/roots/list_changed"
                if self.client.roots =>
            {
                vec![json!({ "jsonrpc": "2.0", "id": ROOTS, "method": "roots/list" })]
            }
            "notifications/cancelled" => {
                let ended = &params["requestId"];
                self.subscriptions.retain(|id| id != ended);
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// The response to the old protocol's request `method` with `params`,
    /// under `id`.
    fn of_old(&mut self, id: &Value, method: &str, params: &Value) -> Value {
        match method {
            "initialize" => {
                self.client.roots = params["capabilities"]["roots"].is_object();
                self.client.initialized = true;
                response(id, &initialize(params))
            }
            "ping" => response(id, &json!({})),
            "tools/list" => response(id, &json!({ "tools": self.tools })),
            "tools/call" => self.tool_call(id, params),
            _ => error(id, METHOD_NOT_FOUND, &format!("Method not found: {method}")),
        }
    }

    /// The messages for the 2026-07-28 request `method` with `params`,
    /// under `id`: its response (none for `subscriptions/listen`, which
    /// lasts), and the notifications it led to.
    fn of_stateless(&mut self, id: &Value, method: &str, params: &Value) -> Vec<Value> {
        let complete = |message| of_2026(message, "complete");
        match method {
            "server/discover" => vec![complete(response(id, &discover()))],
            "tools/list" => {
                // A list that follows the session's folder may change, and
                // is this client's.
                let (ttl, scope) = if self.forwarding.is_some() {
                    (0, "private")
                } else {
                    (KEEP_MS, "public")
                };
                let listed = json!({ "tools": self.tools, "ttlMs": ttl, "cacheScope": scope });
                vec![complete(response(id, &listed))]
            }
            "tools/call" => {
                let mut out = Vec::new();
                if let Some(roots) = params["inputResponses"].get(ROOTS) {
                    out = self.rooted(roots);
                } else if !self.client.asked_roots
                    && params["_meta"][META_CAPABILITIES]["roots"].is_object()
                {
                    self.client.asked_roots = true;
                    let asked = json!({ "inputRequests": { ROOTS: { "method": "roots/list" } } });
                    return vec![of_2026(response(id, &asked), "input_required")];
                }
                if declares_tasks(params)
                    && let Some(task) = self.task_call(id, params)
                {
                    out.push(of_2026(task, "task"));
                    return out;
                }
                out.push(complete(self.tool_call(id, params)));
                out
            }
            "tasks/get" | "tasks/cancel" => vec![self.of_task(id, method, params)],
            "subscriptions/listen" => {
                let wants = params["notifications"]["toolsListChanged"] == true;
                if wants {
                    self.subscriptions.push(id.clone());
                }
                let agreed = if wants {
                    json!({ "toolsListChanged": true })
                } else {
                    json!({})
                };
                vec![
                    json!({ "jsonrpc": "2.0", "method": "notifications/subscriptions/acknowledged",
                             "params": { "_meta": { META_SUBSCRIPTION: id }, "notifications": agreed } }),
                ]
            }
            _ => vec![error(
                id,
                METHOD_NOT_FOUND,
                &format!("Method not found: {method}"),
            )],
        }
    }

    /// The notifications that the tools changed: the old protocol's, if a
    /// client said `initialize`, and one on each subscription that wants
    /// them.
    fn changed(&self) -> Vec<Value> {
        let old = json!({ "jsonrpc": "2.0", "method": "notifications/tools/list_changed" });
        let on = self.subscriptions.iter().map(|id| {
            json!({ "jsonrpc": "2.0", "method": "notifications/tools/list_changed",
                    "params": { "_meta": { META_SUBSCRIPTION: id } } })
        });
        self.client
            .initialized
            .then_some(old)
            .into_iter()
            .chain(on)
            .collect()
    }

    /// The client's roots, `listed` (a `roots/list` result): the first is
    /// the session's folder. If it is in another workspace than the
    /// server's, that workspace's server starts now, so its build runs
    /// while the agent works, and the server will list its tools.
    fn rooted(&mut self, listed: &Value) -> Vec<Value> {
        self.client.asked_roots = true;
        let Some(folder) = listed["roots"][0]["uri"].as_str().and_then(path_of_uri) else {
            return Vec::new();
        };
        let root = project_root(&folder);
        self.folder = Some(folder);
        if let Some(workspace) = self.forward_to(&root) {
            self.follow = Some(workspace);
            return self.polled();
        }
        // The server's own workspace: its own list, again if the folder
        // moved back from another one.
        self.follow = None;
        self.listing = None;
        if self.tools == tools() {
            return Vec::new();
        }
        self.tools = tools();
        self.changed()
    }

    /// The notifications the client gets when the tools it should list
    /// changed: once the followed server has answered `initialize`, its
    /// list, if it isn't the server's. Asks nothing of a server still
    /// building. On the old protocol it is one notification; on 2026-07-28
    /// one on each subscription that wants it.
    pub fn poll(&mut self) -> Vec<String> {
        self.polled().iter().map(Value::to_string).collect()
    }

    /// [`Self::poll`]'s messages, as JSON. The tasks whose hosts replied
    /// end first.
    fn polled(&mut self) -> Vec<Value> {
        self.advance();
        let Some(workspace) = self.follow.clone() else {
            return Vec::new();
        };
        // A server answering a task's call would hold the loop on its list
        // until the call ends: it is asked after.
        let busy = self.tasks.works().any(|work| {
            matches!(work, Work::There(away) if away.workspace == workspace && away.id.is_some())
        });
        if busy {
            return Vec::new();
        }
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
                self.changed()
            }
            Some(_) => {
                // The same list from another folder's server: its tools
                // go there now.
                if self.listing.is_some() {
                    self.listing = Some(workspace);
                }
                Vec::new()
            }
            None => Vec::new(),
        }
    }

    /// The response to `tasks/get` or `tasks/cancel` (`method`) with
    /// `params`, under `id`: the task as it stands, or an empty result once
    /// it is stopped. A client that didn't declare the extension is
    /// `-32021`, and a task the server doesn't know (or forgot, an hour
    /// after it ended) `-32602`.
    fn of_task(&mut self, id: &Value, method: &str, params: &Value) -> Value {
        if !declares_tasks(params) {
            return json!({ "jsonrpc": "2.0", "id": id, "error": {
                "code": MISSING_CAPABILITY,
                "message": "Missing required client capability",
                "data": { "requiredCapabilities": { "extensions": { EXTENSION: {} } } },
            } });
        }
        let Some(task) = params["taskId"].as_str() else {
            return error(id, INVALID_PARAMS, "taskId: a task's id");
        };
        self.advance();
        if !self.tasks.has(task) {
            return error(
                id,
                INVALID_PARAMS,
                "Failed to retrieve task: Task not found",
            );
        }
        let result = if method == "tasks/get" {
            self.tasks.detailed(task).unwrap_or_default()
        } else {
            self.cancel(task);
            json!({})
        };
        of_2026(response(id, &result), "complete")
    }

    /// The response to `tools/call` with `params`, under `id`, as a task:
    /// none if its tool isn't one ([`tasked`], `start` with `link` not
    /// either). A call another workspace's server answers is a task too,
    /// sent once that server is ready. The task exists before its answer is
    /// sent.
    fn task_call(&mut self, id: &Value, params: &Value) -> Option<Value> {
        let (op, session) = tool(params["name"].as_str()?)?;
        let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
        if !tasked(op) || (session && op != Op::Start) || !arguments["link"].is_null() {
            return None;
        }
        let (arguments, root) = self.absolute(op, session, arguments);
        if let Some(workspace) = self.forward_to(&root) {
            let state = self.away_task(workspace, op, &root, &arguments)?;
            let task = self.tasks.make(op, (self.now)(), state);
            return self.tasks.json(&task).map(|task| response(id, &task));
        }
        let prepared = if op == Op::Start {
            self.end_session_elsewhere(None);
            self.start_prepared(arguments.clone())
        } else {
            self.prepared(op, arguments.clone())
        };
        let state = match prepared {
            Prepared::Answered(answer, png) => State::Completed(tool_result(&answer, png)),
            Prepared::Waits(mut pending) => {
                pending.arguments = arguments;
                pending.root = root;
                match pending.host.send(&pending.request) {
                    Ok(sent) => {
                        pending.sent = sent;
                        State::Working(Work::Here(pending))
                    }
                    Err(why) => self.completed(pending, Err(why)),
                }
            }
        };
        let task = self.tasks.make(op, (self.now)(), state);
        self.tasks.json(&task).map(|task| response(id, &task))
    }

    /// The state of a task of the tool of `op` with `arguments`, on the
    /// project at `root`, that the server of `workspace` answers: working
    /// until that server has answered. A `start` there ends the session's
    /// game anywhere, and the session has none until the task completes.
    fn away_task(
        &mut self,
        workspace: PathBuf,
        op: Op,
        root: &Path,
        arguments: &Value,
    ) -> Option<State<Work<'a>>> {
        let mut workspace = workspace;
        if op == Op::Start {
            self.end_session_elsewhere(Some(&workspace));
            // Its game there is replaced by this one.
            self.session_at = None;
            // A start there that was building was cancelled with its server.
            workspace = self.forward_to(root)?;
        }
        let mut away = Away::new(workspace, op, arguments);
        Some(match poll_away(&mut self.forwarded, &mut away, false) {
            Some(reply) => ended_as(self.away_ended(away, reply)),
            None => State::Working(Work::There(away)),
        })
    }

    /// Ends each working task whose host or server has replied, with its
    /// result (a host that answers only as it waits is called now, and a
    /// server once it is ready), and forgets the tasks that ended an hour
    /// ago.
    fn advance(&mut self) {
        for task in self.tasks.working(None) {
            let reply = match self.tasks.work(&task) {
                Some(Work::Here(pending)) => {
                    let reply = if pending.sent {
                        pending.host.reply()
                    } else {
                        pending.sent = true;
                        Some(pending.host.call(&pending.request))
                    };
                    reply.map(Replied::Host)
                }
                Some(Work::There(away)) => {
                    poll_away(&mut self.forwarded, away, true).map(Replied::Server)
                }
                None => None,
            };
            let Some(reply) = reply else {
                continue;
            };
            let state = match (self.tasks.end(&task, (self.now)(), State::Cancelled), reply) {
                (Some(Work::Here(pending)), Replied::Host(reply)) => self.completed(pending, reply),
                (Some(Work::There(away)), Replied::Server(reply)) => {
                    ended_as(self.away_ended(away, reply))
                }
                _ => continue,
            };
            self.tasks.ended(&task, state);
        }
        self.tasks.expire((self.now)());
    }

    /// How the task of `pending` ended on its host's `reply`: completed
    /// with the result `tools/call` would have answered, or failed with
    /// its JSON-RPC error. If the engine moved under the server, the call
    /// goes to a fresh server of its own workspace, and the task works on
    /// until that one answers.
    fn completed(&mut self, pending: Pending<'a>, reply: Result<Reply, String>) -> State<Work<'a>> {
        let (op, root, arguments) = (pending.op, pending.root.clone(), pending.arguments.clone());
        let (answer, png) = self.finish(pending, reply);
        let Some(workspace) = self.restart_to(op, &root, &answer) else {
            return ended_as(response(&Value::Null, &tool_result(&answer, png)));
        };
        let mut away = Away::new(workspace, op, &arguments);
        away.fallback = Some((answer, png));
        match poll_away(&mut self.forwarded, &mut away, false) {
            Some(reply) => ended_as(self.away_ended(away, reply)),
            None => State::Working(Work::There(away)),
        }
    }

    /// Stops task `id` if it works: its host is dropped, which ends it
    /// (the game's tool program and its build are killed), or the server
    /// that answers it, which is started again on the next call.
    fn cancel(&mut self, id: &str) {
        if let Some(Work::There(away)) = self.tasks.end(id, (self.now)(), State::Cancelled) {
            self.lost(&away.workspace);
        }
    }

    /// Forgets the server of `workspace`, which stops it, and the session
    /// that ran there: the next call starts it again.
    fn lost(&mut self, workspace: &Path) {
        self.forwarded.remove(workspace);
        if self.session_at.as_deref() == Some(workspace) {
            self.session_at = None;
        }
    }

    /// The response (under no id) to `away`'s call on its server's
    /// `reply`: a `start` that answered `ok` makes the session that
    /// server's. A server that stopped is forgotten, and its call answers
    /// [`codes::GAME_PROGRAM`] with why, or, for the restart's call, this
    /// server's own answer with why the fresh one didn't start.
    fn away_ended(&mut self, away: Away, reply: Result<Value, String>) -> Value {
        match reply {
            Ok(mut answered) => {
                answered["id"] = Value::Null;
                if away.fallback.is_some() && self.follow.is_none() && self.listing.is_none() {
                    self.follow = Some(away.workspace.clone());
                }
                if away.op == Op::Start && is_ok(&answered) {
                    self.session_at = Some(away.workspace);
                }
                answered
            }
            Err(why) => {
                self.lost(&away.workspace);
                let (answer, png) = match away.fallback {
                    Some((answer, png)) => {
                        self.restarted = false;
                        (not_restarted(answer, &why), png)
                    }
                    None => (failed(away.op, codes::GAME_PROGRAM, why), None),
                };
                response(&Value::Null, &tool_result(&answer, png))
            }
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
    ) -> Result<Value, String> {
        let Some(server) = self.forwarded.get_mut(workspace) else {
            return Err(NOT_RUNNING.to_owned());
        };
        // On the old protocol whatever the client speaks: the answer is
        // made a 2026-07-28 one, if it must be, as the server's own are.
        let params = json!({ "name": name, "arguments": arguments });
        match server.request("tools/call", &params) {
            Ok(mut answer) => {
                answer["id"] = id.clone();
                Ok(answer)
            }
            Err(why) => {
                self.lost(workspace);
                Err(why)
            }
        }
    }

    /// The response to `tools/call` with `params`, under `id`.
    fn tool_call(&mut self, id: &Value, params: &Value) -> Value {
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
            // A `start` task that completed elsewhere makes the session there.
            self.advance();
            if let Some(workspace) = self.session_at.clone() {
                return self
                    .forwarded_call(&workspace, id, name, &arguments)
                    .unwrap_or_else(|why| gone(op, why));
            }
            let (answer, png) = self.call(op, session, arguments);
            return response(id, &tool_result(&answer, png));
        }
        if op == Op::Start && !arguments["link"].is_null() {
            // Attached here, whatever the folder's workspace: the link is
            // the same wire in every engine of its version.
            self.end_session_elsewhere(None);
            let (answer, png) = self.call(op, session, arguments);
            return response(id, &tool_result(&answer, png));
        }
        let (arguments, root) = self.absolute(op, session, arguments);
        if let Some(workspace) = self.forward_to(&root) {
            return self.forwarded_tool(workspace, &root, id, op, &arguments);
        }
        if op == Op::Start {
            self.end_session_elsewhere(None);
        }
        let (answer, png) = self.call(op, session, arguments.clone());
        self.answered(id, op, &root, &arguments, answer, png)
    }

    /// The response under `id` to the tool of `op` with `arguments`, on
    /// the project at `root`, that this server answered `answer` and
    /// `png`: if the engine moved under the server, a fresh server of its
    /// own workspace answers it instead, and every later call.
    fn answered(
        &mut self,
        id: &Value,
        op: Op,
        root: &Path,
        arguments: &Value,
        answer: Value,
        png: Option<String>,
    ) -> Value {
        let Some(workspace) = self.restart_to(op, root, &answer) else {
            return response(id, &tool_result(&answer, png));
        };
        let reply = self.forwarded_call(&workspace, &Value::Null, op.name(), arguments);
        let mut away = Away::new(workspace, op, arguments);
        away.fallback = Some((answer, png));
        let mut answered = self.away_ended(away, reply);
        answered["id"] = id.clone();
        answered
    }

    /// The workspace of the fresh server that answers the tool of `op` on
    /// the project at `root`, which this server answered `answer`, when
    /// the engine moved under the server (the answer's contract isn't the
    /// server's) and the project is in the server's own workspace: from now
    /// on, every call of that workspace goes there. None otherwise.
    fn restart_to(&mut self, op: Op, root: &Path, answer: &Value) -> Option<PathBuf> {
        let moved = self.forwarding.is_some()
            && !self.restarted
            && stale(answer, VERSION, "").is_some()
            && self
                .workspace_of(root)
                .is_some_and(|ws| Some(ws) == self.workspace_of(&self.cwd.clone()));
        if !moved {
            return None;
        }
        self.restarted = true;
        if op == Op::Start {
            self.game = None;
        }
        let workspace = self.forward_to(root);
        if workspace.is_none() {
            self.restarted = false;
        }
        workspace
    }

    /// The response of the server of `workspace` to the tool of `op` (not
    /// a session tool, or `start`) with `arguments`, on the project at
    /// `root`, under `id`; a `start` there ends the session's game anywhere
    /// else.
    fn forwarded_tool(
        &mut self,
        workspace: PathBuf,
        root: &Path,
        id: &Value,
        op: Op,
        arguments: &Value,
    ) -> Value {
        let mut workspace = workspace;
        if op == Op::Start {
            self.end_session_elsewhere(Some(&workspace));
            // A start there that was building was cancelled with its server.
            match self.forward_to(root) {
                Some(again) => workspace = again,
                None => return error(id, INTERNAL_ERROR, NOT_RUNNING),
            }
        }
        let reply = self.forwarded_call(&workspace, &Value::Null, op.name(), arguments);
        let mut answered = self.away_ended(Away::new(workspace, op, arguments), reply);
        answered["id"] = id.clone();
        answered
    }

    /// Stops the session's game unless it runs in the server of `at`
    /// (this server's own if none): `start` is starting one there.
    fn end_session_elsewhere(&mut self, at: Option<&Path>) {
        if let Some(workspace) = self.session_at.take_if(|w| Some(w.as_path()) != at)
            && let Some(server) = self.forwarded.get_mut(&workspace)
        {
            let _ = server.request("tools/call", &json!({ "name": "stop", "arguments": {} }));
        }
        if at.is_some() {
            self.attached = None;
            // A `start` still building, here or in another workspace's
            // server, would take the session back.
            for task in self.tasks.working(Some(Op::Start)) {
                self.cancel(&task);
            }
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
    /// itself if left out, and `new`'s `dir` and `answers` absolute too;
    /// and the root of that project (the folder itself outside one).
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
            if op == Op::New {
                // Its folder and its answers file, from the session's
                // folder, whichever server answers.
                for field in ["dir", "answers"] {
                    if let Some(Value::String(path)) = arguments.get(field) {
                        let path = folder.join(path).display().to_string();
                        arguments.insert(field.to_owned(), json!(path));
                    }
                }
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
        match self.prepared(op, arguments) {
            Prepared::Answered(answer, png) => (answer, png),
            Prepared::Waits(pending) => self.waited(pending),
        }
    }

    /// The answer of `pending`'s host, waited for.
    fn waited(&mut self, mut pending: Pending<'a>) -> (Value, Option<String>) {
        let reply = pending.host.call(&pending.request);
        self.finish(pending, reply)
    }

    /// The answer of `pending`'s call on its host's `reply`. `start` keeps
    /// the host as the session's game, unless it answered nothing or
    /// neither ran nor played.
    fn finish(
        &mut self,
        pending: Pending<'a>,
        reply: Result<Reply, String>,
    ) -> (Value, Option<String>) {
        let op = pending.op;
        let (mut answer, png) = match reply {
            Ok(reply) => answer_of(op, reply, &pending.command),
            Err(e) => return (failed(op, codes::GAME_PROGRAM, e), None),
        };
        if op == Op::Start {
            // A session record that played differently has `ok` false and
            // a result: the game stands on that frame, to look at.
            if answer["ok"] == true || !answer["result"].is_null() {
                self.game = Some(pending.host);
                self.command = pending.command;
            }
            if let Some(list) = answer["limits"].as_array_mut() {
                list.extend(pending.limits.into_iter().map(Value::String));
            }
        }
        (answer, png)
    }

    /// The operation `op` (one of [`Op::ALL`]) with `arguments` up to its
    /// host's reply: answered here if it runs no game, or none started.
    fn prepared(&self, op: Op, arguments: Value) -> Prepared<'a> {
        let answered = |answer| Prepared::Answered(answer, None);
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
                return answered(failed(op, codes::WRONG_ARGUMENTS, error));
            }
        };
        if folder.is_some() && !takes_project(op, false) {
            let error = format!("{} takes no project", op.name());
            return answered(failed(op, codes::WRONG_ARGUMENTS, error));
        }
        let project = self.project(folder.as_deref());
        let runs_in_game = crate::contract::cli::runs_the_game(op.name())
            && project
                .as_ref()
                .is_ok_and(|project| project.config.game.is_some());
        let cwd = self.folder().to_path_buf();
        if !runs_in_game {
            return answered(one_off(op, arguments, &cwd, &project, None));
        }
        let Ok(project) = project else {
            return answered(one_off(op, arguments, &cwd, &project, None));
        };
        let host = match self.launch.launch(&project) {
            Ok(host) => host,
            Err((code, e)) => return answered(failed(op, code, e)),
        };
        Prepared::Waits(Pending {
            op,
            host,
            request: Request {
                op,
                input: arguments,
            },
            sent: false,
            command: project.config.command().to_owned(),
            limits: Vec::new(),
            arguments: Value::Null,
            root: project.root,
        })
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
        match self.start_prepared(arguments) {
            Prepared::Answered(answer, _) => answer,
            Prepared::Waits(pending) => self.waited(pending).0,
        }
    }

    /// `start` up to its host's reply: the game that runs stopped (and a
    /// `start` still building cancelled), and a host of the project's game
    /// launched; answered here if attached over the link, or if it can't
    /// start.
    fn start_prepared(&mut self, arguments: Value) -> Prepared<'a> {
        let op = Op::Start;
        let answered = |answer| Prepared::Answered(answer, None);
        let input: StartInput = match serde_json::from_value(if arguments.is_null() {
            json!({})
        } else {
            arguments
        }) {
            Ok(input) => input,
            Err(e) => return answered(failed(op, codes::WRONG_ARGUMENTS, format!("input: {e}"))),
        };
        let mut limits = Vec::new();
        for task in self.tasks.working(Some(Op::Start)) {
            self.cancel(&task);
            limits.push(format!(
                "the start that was building (task `{task}`) was cancelled"
            ));
        }
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
        if self.attached.take().is_some() {
            limits.push("the build that was attached was left running".to_owned());
        }
        if let Some(to) = &input.link {
            if !input.start.is_empty() || input.session.is_some() {
                let error = "start: a build attached over the link runs with its own options";
                return answered(failed(op, codes::WRONG_ARGUMENTS, error));
            }
            let (attached, mut answer) = Attached::start(to);
            self.attached = attached;
            if let Some(list) = answer["limits"].as_array_mut() {
                list.extend(limits.into_iter().map(Value::String));
            }
            return answered(answer);
        }
        let project = match self.project(input.project.as_deref()) {
            Ok(project) => project,
            Err((code, e)) => return answered(failed(op, code, e)),
        };
        if project.config.game.is_none() {
            let error = format!(
                "{} names no game tool program (`game`), and start runs the game",
                crate::project::FILE
            );
            return answered(failed(op, codes::PROJECT_FILE, error));
        }
        let host = match self.launch.launch(&project) {
            Ok(host) => host,
            Err((code, e)) => return answered(failed(op, code, e)),
        };
        Prepared::Waits(Pending {
            op,
            host,
            request: Request {
                op,
                input: json!({ "start": input.start, "session": input.session }),
            },
            sent: false,
            command: project.config.command().to_owned(),
            limits,
            arguments: Value::Null,
            root: project.root,
        })
    }

    /// Session tool `op` with `arguments`, on the session's game: none
    /// while `start`'s task builds it, which the answer names.
    fn in_session(&mut self, op: Op, arguments: Value) -> (Value, Option<String>) {
        self.advance();
        if let Some(attached) = self.attached.as_mut() {
            let answered = attached.call(op, arguments);
            if attached.ended() {
                self.attached = None;
            }
            return answered;
        }
        let Some(game) = self.game.as_mut() else {
            if let Some(task) = self.tasks.working(Some(Op::Start)).first() {
                let why = format!(
                    "no game is running yet: `start` is building it, as task `{task}`; call this \
                     again once `tasks/get` says the task completed"
                );
                return (failed(op, codes::NO_SESSION, why), None);
            }
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
fn is_ok(answered: &Value) -> bool {
    answered["result"]["isError"] == false
}

/// Whether the 2026-07-28 request of `params` declares the tasks
/// extension in its client's capabilities: only then may it be answered a
/// task.
fn declares_tasks(params: &Value) -> bool {
    params["_meta"][META_CAPABILITIES]["extensions"]
        .get(EXTENSION)
        .is_some()
}

/// `server/discover`'s result, before its `resultType` and `serverInfo`:
/// the versions a request may name in `_meta`, the tools, the tasks
/// extension, what the server is for, and that it keeps for the process's
/// life.
fn discover() -> Value {
    json!({
        "supportedVersions": [STATELESS],
        "capabilities": { "tools": { "listChanged": true }, "extensions": { EXTENSION: {} } },
        "instructions": INSTRUCTIONS,
        "ttlMs": KEEP_MS,
        "cacheScope": "public",
    })
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

/// What a host's `answer` says when its engine speaks another contract
/// than `version`, the server's: both contracts and which side is newer.
/// None when they speak the same one: a change of the engine that changes
/// no schema leaves the server's tools right.
fn stale_diagnosis(answer: &Value, version: u32) -> Option<String> {
    let theirs = answer.get("contract")?.as_u64()?;
    (theirs != u64::from(version)).then(|| {
        let newer = if theirs > u64::from(version) {
            "the server was built before the engine changed"
        } else {
            "the server is newer than the game's engine (a checkout with an older pin?)"
        };
        format!("the game's engine speaks contract {theirs} and this server {version}: {newer}")
    })
}

/// This server's own `answer` when the fresh server started for it, once
/// the engine moved, stopped with `why`: the limit keeps the diagnosis
/// alone, since what stopped the fresh server is the fix, not a
/// reconnect, and says why.
fn not_restarted(mut answer: Value, why: &str) -> Value {
    let diagnosis = stale_diagnosis(&answer, VERSION);
    if let Some(limits) = answer["limits"].as_array_mut() {
        if let Some(diagnosis) = diagnosis {
            limits.retain(|l| l.as_str().is_none_or(|l| !l.starts_with(&diagnosis)));
            limits.push(Value::String(diagnosis));
        }
        limits.push(Value::String(format!(
            "a fresh server of this checkout didn't start: {why}"
        )));
    }
    answer
}

/// The limit a host's `answer` gets, where no fresh server is started for
/// it, when its engine speaks another contract than `version`, the
/// server's (its two contracts and the newer side), and what fixes it: for an older
/// server, a reconnect (in Claude Code: /mcp) starts a fresh one; for a
/// newer one, a reconnect doesn't, and the project's own `shuyi` must be
/// the MCP entry (`command mcp install`) if the entry runs one installed
/// for every project. A server that forwards first hands the call to a
/// fresh server of its own workspace, and writes the diagnosis alone,
/// beside why that one didn't start.
pub fn stale(answer: &Value, version: u32, command: &str) -> Option<String> {
    let diagnosis = stale_diagnosis(answer, version)?;
    let theirs = answer.get("contract")?.as_u64()?;
    Some(if theirs > u64::from(version) {
        format!("{diagnosis}; reconnect it (in Claude Code: /mcp) to start a fresh one")
    } else {
        format!(
            "{diagnosis}; if the MCP entry runs a `shuyi` installed for every project, register \
             the project's own (`{command} mcp install`)"
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
        "serverInfo": server_info(),
        "instructions": INSTRUCTIONS,
    })
}

#[cfg(test)]
mod tests;
