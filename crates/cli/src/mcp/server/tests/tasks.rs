//! Long tools as tasks on 2026-07-28: a host whose reply waits until the
//! test lets it through, so `working` is seen, then `completed`; and one
//! that a cancel must stop.

use std::cell::Cell;
use std::time::{Duration, UNIX_EPOCH};

use super::*;

/// What a [`Building`] host waits on and says: whether its reply may come,
/// how many hosts were dropped (a cancel's kill), and whether its game
/// doesn't build.
#[derive(Clone, Default)]
struct Gate {
    open: Rc<Cell<bool>>,
    dropped: Rc<Cell<u32>>,
    broken: bool,
}

impl Gate {
    /// A gate already open.
    fn open() -> Self {
        let gate = Self::default();
        gate.open.set(true);
        gate
    }
}

/// A host whose reply to the request sent comes only once its gate is
/// open; one called waits for nothing.
struct Building {
    gate: Gate,
    asked: Option<Request>,
}

impl Building {
    /// Its reply to `request`: an answer of `ok` with no frames, or, if
    /// the game is broken, a stop with the compiler's error.
    fn answer(&self, request: &Request) -> Result<Reply, String> {
        if self.gate.broken {
            return Err(
                "the game's tool program stopped before it answered (exit code 101): \
                        error: could not compile `game`"
                    .into(),
            );
        }
        let answer =
            crate::contract::Answer::new(request.op, true, json!({ "frames": 0 }), Vec::new());
        Ok(Reply {
            answer: Some(serde_json::to_value(answer).unwrap()),
            ..Reply::default()
        })
    }
}

impl Link for Building {
    fn call(&mut self, request: &Request) -> Result<Reply, String> {
        self.answer(request)
    }

    fn send(&mut self, request: &Request) -> Result<bool, String> {
        self.asked = Some(request.clone());
        Ok(true)
    }

    fn reply(&mut self) -> Option<Result<Reply, String>> {
        if !self.gate.open.get() {
            return None;
        }
        let request = self.asked.take()?;
        Some(self.answer(&request))
    }
}

impl Drop for Building {
    fn drop(&mut self) {
        self.gate.dropped.set(self.gate.dropped.get() + 1);
    }
}

impl<'a> Launch<'a> for Gate {
    fn launch(&self, _: &Project) -> Result<Box<dyn Link + 'a>, (Code, String)> {
        Ok(Box::new(Building {
            gate: self.clone(),
            asked: None,
        }))
    }
}

/// A client's capabilities with the tasks extension.
fn with_tasks() -> Value {
    json!({ "extensions": { "io.modelcontextprotocol/tasks": {} } })
}

/// The response to the 2026-07-28 request `method` with `params`, from a
/// client with the tasks extension.
fn asked(server: &mut Server<'_>, method: &str, params: Value) -> Value {
    handled(server, &of_2026("t", method, params, &with_tasks()))
}

/// The result of calling tool `name` with `arguments` on 2026-07-28, from
/// a client with the tasks extension.
fn tasked_call(server: &mut Server<'_>, name: &str, arguments: &Value) -> Value {
    let params = json!({ "name": name, "arguments": arguments });
    asked(server, "tools/call", params)["result"].clone()
}

/// The result of `tasks/get` of `task`.
fn got(server: &mut Server<'_>, task: &str) -> Value {
    asked(server, "tasks/get", json!({ "taskId": task }))["result"].clone()
}

/// Whether `stamp` reads as ISO 8601 in UTC to the millisecond.
fn is_stamp(stamp: &Value) -> bool {
    let stamp = stamp.as_str().unwrap_or("");
    let b = stamp.as_bytes();
    b.len() == 24 && b[4] == b'-' && b[10] == b'T' && b[19] == b'.' && stamp.ends_with('Z')
}

#[test]
fn server_discover_declares_the_tasks_extension() {
    let dir = scratch("tasks-discover", GAME);
    let mut server = Server::new(dir.clone(), Box::new(Gate::open()));
    let found = asked(&mut server, "server/discover", json!({}));
    assert_eq!(
        found["result"]["capabilities"]["extensions"],
        json!({ "io.modelcontextprotocol/tasks": {} })
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_long_tool_answers_a_task_that_works_then_completes_with_its_result() {
    let dir = scratch("tasks-playtest", GAME);
    let gate = Gate::default();
    let mut server = Server::new(dir.clone(), Box::new(gate.clone()));
    let task = tasked_call(&mut server, "playtest", &json!({}));
    assert_eq!(task["resultType"], "task", "{task}");
    assert_eq!(server_info_of(&task)["name"], "shuyi");
    assert_eq!(
        (
            task["taskId"].as_str(),
            task["status"].as_str(),
            task["pollIntervalMs"].as_u64()
        ),
        (Some("shuyi-1"), Some("working"), Some(10_000))
    );
    assert!(task["ttlMs"].is_null(), "kept while it works: {task}");
    assert!(
        is_stamp(&task["createdAt"]) && is_stamp(&task["lastUpdatedAt"]),
        "{task}"
    );

    let working = got(&mut server, "shuyi-1");
    assert_eq!(
        (working["resultType"].as_str(), working["status"].as_str()),
        (Some("complete"), Some("working"))
    );
    assert!(working["result"].is_null(), "{working}");

    gate.open.set(true);
    let completed = got(&mut server, "shuyi-1");
    assert_eq!(completed["status"], "completed", "{completed}");
    // The result the call answers without tasks.
    let mut direct = Server::new(dir.clone(), Box::new(Gate::open()));
    assert_eq!(
        completed["result"],
        called(&mut direct, "playtest", &json!({}))
    );
    assert_eq!(completed["result"]["isError"], false);
    // Kept an hour from when it ended, which counts from when it was made.
    assert!(
        completed["ttlMs"].as_u64().unwrap() >= 3_600_000,
        "{completed}"
    );
    assert_eq!(completed["createdAt"], task["createdAt"]);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_call_without_the_extension_or_on_the_old_protocol_answers_as_it_did() {
    let dir = scratch("tasks-without", GAME);
    let gate = Gate::open();
    let mut server = Server::new(dir.clone(), Box::new(gate));
    let old = called(&mut server, "playtest", &json!({}));
    assert_eq!(old["isError"], false, "{old}");
    let call = json!({ "name": "playtest", "arguments": {} });
    let new = handled(&mut server, &of_2026("c", "tools/call", call, &json!({})));
    assert_eq!(new["result"]["resultType"], "complete", "{new}");
    assert_eq!(new["result"]["structuredContent"], old["structuredContent"]);
    // Nor do `tasks/get` and `tasks/cancel` answer such a client.
    for method in ["tasks/get", "tasks/cancel"] {
        let params = json!({ "taskId": "shuyi-1" });
        let refused = handled(&mut server, &of_2026("g", method, params, &json!({})));
        assert_eq!(refused["error"]["code"], -32_021, "{refused}");
        assert_eq!(
            refused["error"]["data"]["requiredCapabilities"]["extensions"],
            with_tasks()["extensions"]
        );
    }
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_tool_not_in_the_list_never_answers_a_task() {
    let dir = scratch("tasks-not", GAME);
    let mut server = Server::new(dir.clone(), Box::new(Gate::open()));
    for (name, arguments) in [
        ("docs", json!({ "query": "play a sound" })),
        ("scene", json!({})),
        ("run", json!({})),
        ("explain", json!({ "code": "SH0002" })),
        ("fly", json!({})),
    ] {
        let answered = asked(
            &mut server,
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        );
        assert_ne!(
            answered["result"]["resultType"], "task",
            "{name}: {answered}"
        );
    }
    // `start` attached over the link answers as it is too.
    let link = json!({ "link": { "address": "127.0.0.1:1", "token": "x" } });
    let attached = tasked_call(&mut server, "start", &link);
    assert_eq!(attached["resultType"], "complete", "{attached}");
    assert!(tasked(Op::Export) && tasked(Op::Bake) && tasked(Op::Autobalance));
    assert!(!tasked(Op::Step) && !tasked(Op::Run) && !tasked(Op::Docs));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_cancel_kills_starts_build_and_the_session_has_no_game() {
    let dir = scratch("tasks-cancel", GAME);
    let gate = Gate::default();
    let mut server = Server::new(dir.clone(), Box::new(gate.clone()));
    let task = tasked_call(&mut server, "start", &json!({}));
    assert_eq!(
        (task["status"].as_str(), task["pollIntervalMs"].as_u64()),
        (Some("working"), Some(2_000))
    );
    // While it builds, a session tool has no game, and names the task.
    let step = called(&mut server, "step", &json!({}));
    let answer = &step["structuredContent"];
    assert_eq!(answer["code"], "SH0002");
    assert!(
        answer["error"]
            .as_str()
            .unwrap()
            .contains("`start` is building it, as task `shuyi-1`"),
        "{answer}"
    );

    let cancelled = asked(&mut server, "tasks/cancel", json!({ "taskId": "shuyi-1" }));
    assert_eq!(cancelled["result"]["resultType"], "complete", "{cancelled}");
    assert_eq!(gate.dropped.get(), 1, "the build was killed");
    assert_eq!(got(&mut server, "shuyi-1")["status"], "cancelled");
    let step = called(&mut server, "step", &json!({}));
    assert_eq!(
        step["structuredContent"]["error"],
        "no game is running: call `start` first"
    );
    assert!(!server.has_game());

    // A cancel of a task that ended is empty, and changes nothing.
    gate.open.set(true);
    let task = tasked_call(&mut server, "start", &json!({}));
    assert_eq!(task["taskId"], "shuyi-2");
    assert_eq!(got(&mut server, "shuyi-2")["status"], "completed");
    let again = asked(&mut server, "tasks/cancel", json!({ "taskId": "shuyi-2" }));
    assert_eq!(again["result"]["resultType"], "complete");
    assert_eq!(got(&mut server, "shuyi-2")["status"], "completed");
    assert!(server.has_game());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_completed_start_is_the_sessions_game_once_the_server_looks() {
    let dir = scratch("tasks-plays", GAME);
    let gate = Gate::default();
    let mut server = Server::new(dir.clone(), Box::new(gate.clone()));
    tasked_call(&mut server, "start", &json!({}));
    assert_eq!(server.poll(), Vec::<String>::new());
    assert!(!server.has_game());
    gate.open.set(true);
    // The loop's poll ends it: no `tasks/get` is needed to play.
    assert_eq!(server.poll(), Vec::<String>::new());
    assert!(server.has_game());
    let step = called(&mut server, "step", &json!({}));
    assert_eq!(step["isError"], false, "{step}");
    assert_eq!(got(&mut server, "shuyi-1")["status"], "completed");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_second_start_cancels_the_first() {
    let dir = scratch("tasks-twice", GAME);
    let gate = Gate::default();
    let mut server = Server::new(dir.clone(), Box::new(gate.clone()));
    tasked_call(&mut server, "start", &json!({}));
    let second = tasked_call(&mut server, "start", &json!({}));
    assert_eq!(second["taskId"], "shuyi-2");
    assert_eq!(gate.dropped.get(), 1);
    assert_eq!(got(&mut server, "shuyi-1")["status"], "cancelled");
    gate.open.set(true);
    let done = got(&mut server, "shuyi-2");
    assert_eq!(
        done["result"]["structuredContent"]["limits"],
        json!(["the start that was building (task `shuyi-1`) was cancelled"]),
        "{done}"
    );
    // A start without tasks cancels one building too.
    gate.open.set(false);
    tasked_call(&mut server, "start", &json!({}));
    let direct = called(&mut server, "start", &json!({}));
    assert_eq!(got(&mut server, "shuyi-3")["status"], "cancelled");
    let limits = direct["structuredContent"]["limits"].to_string();
    assert!(limits.contains("task `shuyi-3`"), "{limits}");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_start_that_doesnt_build_completes_with_its_error_and_code() {
    let dir = scratch("tasks-broken", GAME);
    let gate = Gate {
        broken: true,
        ..Gate::open()
    };
    let mut server = Server::new(dir.clone(), Box::new(gate.clone()));
    tasked_call(&mut server, "start", &json!({}));
    let ended = got(&mut server, "shuyi-1");
    // A tool's error is its result: `failed` is a JSON-RPC error's.
    assert_eq!(ended["status"], "completed", "{ended}");
    assert_eq!(ended["result"]["isError"], true);
    let direct = called(
        &mut Server::new(dir.clone(), Box::new(gate)),
        "start",
        &json!({}),
    );
    assert_eq!(ended["result"], direct);
    assert_eq!(ended["result"]["structuredContent"]["code"], "SH3005");
    assert!(!server.has_game());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_task_answered_before_it_runs_is_completed_at_once() {
    // No game tool program: `playtest` says so without a host.
    let dir = scratch("tasks-no-game", "()");
    let mut server = Server::new(dir.clone(), Box::new(Gate::default()));
    let task = tasked_call(&mut server, "playtest", &json!({}));
    assert_eq!(task["status"], "completed", "{task}");
    let ended = got(&mut server, "shuyi-1");
    assert_eq!(ended["result"]["structuredContent"]["code"], "SH1002");
    // A host that can't start: the same.
    let mut server = Server::new(
        dir.clone(),
        Box::new(Launches {
            fails: 1,
            refuse: true,
        }),
    );
    fs::write(dir.join("shuyi.ron"), GAME).unwrap();
    tasked_call(&mut server, "start", &json!({}));
    let ended = got(&mut server, "shuyi-1");
    assert_eq!(
        ended["result"]["structuredContent"]["code"], "SH1003",
        "{ended}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_host_that_only_answers_as_it_waits_is_called_when_the_server_looks() {
    let dir = scratch("tasks-in-process", GAME);
    let game = Fake::default();
    let mut server = Server::new(dir.clone(), Box::new(InProcess::new(&game)));
    let task = tasked_call(&mut server, "start", &json!({}));
    assert_eq!(task["status"], "working", "{task}");
    let ended = got(&mut server, "shuyi-1");
    assert_eq!(ended["status"], "completed", "{ended}");
    assert_eq!(ended["result"]["isError"], false);
    assert!(server.has_game());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_unknown_task_or_none_named_is_invalid_params() {
    let dir = scratch("tasks-unknown", GAME);
    let mut server = Server::new(dir.clone(), Box::new(Gate::open()));
    for (method, params) in [
        ("tasks/get", json!({ "taskId": "shuyi-9" })),
        ("tasks/cancel", json!({ "taskId": "nope" })),
        ("tasks/get", json!({})),
    ] {
        let refused = asked(&mut server, method, params);
        assert_eq!(refused["error"]["code"], -32_602, "{method}: {refused}");
    }
    let refused = asked(&mut server, "tasks/get", json!({ "taskId": 1 }));
    assert_eq!(refused["error"]["message"], "taskId: a task's id");
    fs::remove_dir_all(&dir).unwrap();
}

thread_local! {
    /// The test's clock: seconds after the epoch.
    static CLOCK: Cell<u64> = const { Cell::new(1_000) };
}

/// The test's clock as a time.
fn clock() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(CLOCK.get())
}

#[test]
fn a_task_is_kept_an_hour_after_it_ends_then_forgotten() {
    let dir = scratch("tasks-kept", GAME);
    let gate = Gate::default();
    let mut server = Server::new(dir.clone(), Box::new(gate.clone()));
    server.now = clock;
    tasked_call(&mut server, "playtest", &json!({}));
    // Working, it is kept however long it runs.
    CLOCK.set(1_000 + 7_200);
    assert_eq!(got(&mut server, "shuyi-1")["status"], "working");
    gate.open.set(true);
    let ended = got(&mut server, "shuyi-1");
    assert_eq!(
        (
            ended["createdAt"].as_str(),
            ended["lastUpdatedAt"].as_str(),
            ended["ttlMs"].as_u64()
        ),
        (
            Some("1970-01-01T00:16:40.000Z"),
            Some("1970-01-01T02:16:40.000Z"),
            Some(7_200_000 + 3_600_000)
        )
    );
    CLOCK.set(1_000 + 7_200 + 3_600);
    assert_eq!(got(&mut server, "shuyi-1")["status"], "completed");
    CLOCK.set(1_000 + 7_200 + 3_601);
    let forgotten = asked(&mut server, "tasks/get", json!({ "taskId": "shuyi-1" }));
    assert_eq!(forgotten["error"]["code"], -32_602, "{forgotten}");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_start_in_another_workspace_cancels_one_building_here() {
    let own = scratch("tasks-fw-own", GAME);
    let theirs = scratch(
        "tasks-fw-theirs",
        r#"(command: "cargo shuyi", game: (package: "fake"))"#,
    );
    let gate = Gate::default();
    let workspaces = Workspaces::new(&[(&own, "own"), (&theirs, "theirs")]);
    let mut server =
        Server::new(own.clone(), Box::new(gate.clone())).forwarding(Box::new(workspaces));
    let task = tasked_call(&mut server, "start", &json!({}));
    assert_eq!(task["status"], "working", "{task}");
    // Forwarded, it answers as it is, and the session is there now.
    let project = json!({ "project": theirs.display().to_string() });
    let there = tasked_call(&mut server, "start", &project);
    assert_eq!(there["structuredContent"]["by"], "forwarded", "{there}");
    assert_eq!(gate.dropped.get(), 1);
    gate.open.set(true);
    assert_eq!(got(&mut server, "shuyi-1")["status"], "cancelled");
    assert!(server.game.is_none() && server.session_at.is_some());
    fs::remove_dir_all(&own).unwrap();
    fs::remove_dir_all(&theirs).unwrap();
}
