//! The server over workers on threads, each running the toy in its lobby
//! (Options, Key bindings, Talk, Quit), or stalled: actions allowed and
//! refused, the sessions at once, a deadline, closing, and the requests
//! nothing answers, over a real connection too.

use std::io::{BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr};

use serde_json::Value;

use super::*;
use crate::preview::worker;
use crate::toy::ToyTools;

/// A worker's arguments: the toy, started in its lobby.
fn lobby() -> Vec<String> {
    ["--frames", "60", "--lobby"].map(str::to_owned).to_vec()
}

/// Workers on threads, each the toy started in its lobby, which send
/// their exit code to `ended` as they end.
fn toy_ending(ended: Sender<u8>) -> Launch {
    Box::new(move || {
        let (from_server, to_worker) = std::io::pipe()?;
        let (from_worker, to_server) = std::io::pipe()?;
        let ended = ended.clone();
        thread::spawn(move || {
            let code = worker::serve(&ToyTools, &lobby(), BufReader::new(from_server), to_server);
            let _ = ended.send(code);
        });
        Ok(Worker::over(Box::new(to_worker), from_worker, None))
    })
}

/// A server of the toy that keeps `limits`.
fn server(limits: Limits) -> Server {
    Server::new(limits, toy_ending(channel().0))
}

/// Workers on threads that start the toy in its lobby, if `starts`, and
/// then answer nothing until the server closes their input.
fn stalled(starts: bool) -> Launch {
    Box::new(move || {
        let (from_server, to_worker) = std::io::pipe()?;
        let (from_worker, mut to_server) = std::io::pipe()?;
        thread::spawn(move || {
            if starts {
                // Its first frame alone: an input that ends at once.
                worker::serve(&ToyTools, &lobby(), &b""[..], &mut to_server);
            }
            BufReader::new(from_server).lines().for_each(drop);
            drop(to_server);
        });
        Ok(Worker::over(Box::new(to_worker), from_worker, None))
    })
}

/// The answer to `method` on `path` with `body`.
fn asked(server: &Server, method: &str, path: &str, body: &str) -> Answer {
    let request = Request {
        method: method.to_owned(),
        path: path.to_owned(),
        body: body.as_bytes().to_vec(),
    };
    server.answer(&request)
}

/// The status and the JSON body of `method` on `path` with `body`.
fn ask(server: &Server, method: &str, path: &str, body: &str) -> (u16, Value) {
    let answer = asked(server, method, path, body);
    let body = serde_json::from_slice(&answer.body).unwrap_or(Value::Null);
    (answer.status, body)
}

/// A new session's path.
fn start(server: &Server) -> String {
    let (status, body) = ask(server, "POST", "/session", "");
    assert_eq!(status, 200, "{body}");
    format!("/session/{}", body["session"].as_str().unwrap())
}

/// The status and body of `action` pressed in the session at `path`.
fn press(server: &Server, path: &str, action: &str) -> (u16, Value) {
    let body = serde_json::json!({ "action": action }).to_string();
    ask(server, "POST", path, &body)
}

/// The value of `answer`'s header `name`.
fn header<'a>(answer: &'a Answer, name: &str) -> Option<&'a str> {
    let mut headers = answer.headers.iter();
    headers.find(|(n, _)| *n == name).map(|(_, v)| v.as_str())
}

/// The sessions open.
fn open(server: &Server) -> usize {
    lock(&server.table).live.len()
}

#[test]
fn a_session_starts_on_a_frame_and_an_allowed_action_answers_the_next() {
    let server = server(Limits::default());
    let (status, first) = ask(&server, "POST", "/session", "");
    assert_eq!((status, &first["ended"]), (200, &Value::Bool(false)));
    let id = first["session"].as_str().unwrap();
    assert_eq!(id_of(&format!("/{id}")), Some(id));
    assert!(first["text"].as_str().unwrap().contains("Key bindings"));
    let path = format!("/session/{id}");
    let (status, next) = press(&server, &path, "CursorDown");
    assert_eq!(status, 200, "{next}");
    assert_eq!(next["session"], first["session"]);
    assert_ne!(next["png"], first["png"]);
    // An unknown action, or a body that isn't one, is refused.
    let (status, debug) = press(&server, &path, "Debug");
    let error = debug["error"].as_str().unwrap();
    assert!(
        error.starts_with("no action `Debug`: a visitor's are "),
        "{error}"
    );
    assert_eq!(
        (status, error.contains("CursorDown")),
        (400, true),
        "{error}"
    );
    for body in [
        "",
        "{}",
        r#"{"action": 1}"#,
        r#"{"action": "Up", "frames": 9}"#,
    ] {
        assert_eq!(ask(&server, "POST", &path, body).0, 400, "{body}");
    }
    // An action while one runs is refused.
    let session = Arc::clone(&lock(&server.table).live[id]);
    let held = lock(&session);
    let busy = asked(&server, "POST", &path, "{\"action\": \"CursorDown\"}");
    assert_eq!(busy.status, 409);
    assert_eq!(header(&busy, "Retry-After"), Some("1"));
    drop(held);
    // Another method says which a path answers.
    let allow = |path: &str| header(&asked(&server, "GET", path, ""), "Allow").map(str::to_owned);
    assert_eq!(allow("/session").as_deref(), Some("POST"));
    assert_eq!(allow(&path).as_deref(), Some("POST, DELETE"));
    // Ended by the visitor, it is gone.
    assert_eq!(ask(&server, "DELETE", &path, "").0, 204);
    assert_eq!(press(&server, &path, "CursorDown").0, 404);
    assert_eq!(ask(&server, "DELETE", &path, "").0, 404);
    // Another has another id, and ends when its game quits: down to Quit.
    let other = start(&server);
    assert_ne!(other, path);
    for action in ["CursorDown", "CursorDown", "CursorDown"] {
        assert_eq!(press(&server, &other, action).0, 200);
    }
    let (status, quit) = press(&server, &other, "Confirm");
    assert_eq!((status, &quit["ended"]), (200, &Value::Bool(true)));
    assert_eq!(press(&server, &other, "CursorDown").0, 404);
}

#[test]
fn the_sessions_at_once_are_bounded_those_starting_among_them() {
    let server = server(Limits {
        sessions: 2,
        ..Limits::default()
    });
    let first = start(&server);
    start(&server);
    let refused = asked(&server, "POST", "/session", "");
    assert_eq!(refused.status, 503);
    assert_eq!(header(&refused, "Retry-After"), Some("30"));
    // Ending one makes room, but not while another is starting.
    assert_eq!(ask(&server, "DELETE", &first, "").0, 204);
    lock(&server.table).starting = 1;
    assert_eq!(ask(&server, "POST", "/session", "").0, 503);
    lock(&server.table).starting = 0;
    start(&server);
    assert_eq!(open(&server), 2);
}

#[test]
fn a_worker_that_doesnt_answer_in_time_is_ended_with_its_session() {
    let limits = Limits {
        deadline: Duration::from_millis(200),
        ..Limits::default()
    };
    let server = Server::new(limits.clone(), stalled(true));
    let path = start(&server);
    let (status, body) = press(&server, &path, "CursorDown");
    assert_eq!(status, 500);
    let error = "the game stopped: the game took longer than 200ms";
    assert_eq!(body["error"], error);
    assert_eq!(press(&server, &path, "CursorDown").0, 404);
    // One that doesn't start in time never is a session.
    let server = Server::new(limits, stalled(false));
    assert_eq!(ask(&server, "POST", "/session", "").0, 500);
    assert_eq!((open(&server), lock(&server.table).starting), (0, 0));
}

#[test]
fn a_worker_process_past_its_deadline_is_killed_and_none_is_left() {
    // `sort` reads its whole input before it writes: it never answers.
    let worker = || Worker::process(Path::new("sort"), &[], Path::new(".")).unwrap();
    let mut late = worker();
    let reply = late.call(Some("Up"), Duration::from_millis(200));
    assert_eq!(reply, Err("the game took longer than 200ms".to_owned()));
    let child = late.child.as_mut().unwrap();
    assert!(child.try_wait().unwrap().is_some(), "it runs on");
    // One whose session ends is waited for: not even a zombie is left.
    let ended = worker();
    let pid = ended.child.as_ref().unwrap().id();
    drop(ended);
    // Linux's `/proc` says so.
    let left = cfg!(target_os = "linux") && Path::new(&format!("/proc/{pid}")).exists();
    assert!(!left, "{pid} is left");
}

#[test]
fn closing_the_server_ends_every_session_and_its_worker() {
    let (ended, workers) = channel();
    let server = Server::new(Limits::default(), toy_ending(ended));
    let paths = [start(&server), start(&server)];
    server.close();
    assert_eq!(open(&server), 0);
    for path in paths {
        let code = workers.recv_timeout(Duration::from_secs(10));
        assert_eq!(code, Ok(0), "{path}'s worker ended");
    }
}

/// `server` serving on a port of its own, and its address.
fn serving(server: &Arc<Server>) -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let address = listener.local_addr().unwrap();
    let server = Arc::clone(server);
    thread::spawn(move || serve(&listener, &server));
    address
}

/// The answer to `request`, over a connection to `address`.
fn over(address: SocketAddr, request: &str) -> String {
    let mut stream = TcpStream::connect(address).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut text = String::new();
    stream.read_to_string(&mut text).unwrap();
    text
}

/// The status and the body of `method` on `path` with `body`, over a
/// connection to `address`.
fn over_http(address: SocketAddr, method: &str, path: &str, body: &str) -> (u16, String) {
    let length = body.len();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: preview\r\nContent-Length: {length}\r\n\r\n{body}"
    );
    let text = over(address, &request);
    let (head, body) = text.split_once("\r\n\r\n").unwrap();
    (head[9..12].parse().unwrap(), body.to_owned())
}

#[test]
fn nothing_but_the_sessions_is_served_over_a_connection() {
    let server = Arc::new(server(Limits::default()));
    let address = serving(&server);
    let (status, first) = over_http(address, "POST", "/session", "");
    assert_eq!(status, 200, "{first}");
    let first: Value = serde_json::from_str(&first).unwrap();
    let id = first["session"].as_str().unwrap();
    let session = format!("/session/{id}");
    let files = "/ /index.html /scene /snapshot /log /events /script /frames /frame_png /mcp \
                 /files/keymap.ron /game.wasm /game.pack /src/lib.rs /.git/config /sessions \
                 /session/ /session/../files";
    let mut unserved: Vec<String> = files.split_whitespace().map(str::to_owned).collect();
    for more in ["/scene", "/snapshot", "/log", "0"] {
        unserved.push(format!("{session}{more}"));
    }
    unserved.push(format!("/session/{}", id.to_uppercase()));
    unserved.push(session.to_uppercase());
    for path in unserved {
        for method in ["GET", "POST", "PUT", "DELETE"] {
            let (status, body) = over_http(address, method, &path, "{\"action\": \"Confirm\"}");
            assert_eq!(status, 404, "{method} {path}: {body}");
        }
    }
    let other = [
        ("GET", session.as_str()),
        ("PUT", "/session"),
        ("PATCH", "/session"),
    ];
    for (method, path) in other {
        assert_eq!(
            over_http(address, method, path, "").0,
            405,
            "{method} {path}"
        );
    }
    let (status, next) = over_http(address, "POST", &session, "{\"action\": \"CursorDown\"}");
    assert_eq!(status, 200, "{next}");
    // The frame's fields alone: no colour, legend or overlay of its
    // snapshot, which name the files sprites are from.
    let next: Value = serde_json::from_str(&next).unwrap();
    let mut keys: Vec<&String> = next.as_object().unwrap().keys().collect();
    keys.sort_unstable();
    assert_eq!(keys, ["actions", "ended", "png", "session", "text"]);
    let text = next["text"].as_str().unwrap();
    assert!(text.contains("Key bindings"), "{text}");
    for internal in ["--- colours ---", "--- legend ---", "--- overlays ---"] {
        assert!(!text.contains(internal), "{internal} in {text}");
    }
    let chunked = "POST /session HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n";
    let refused = over(address, chunked);
    assert!(refused.starts_with("HTTP/1.1 501 "), "{refused}");
    assert_eq!(open(&server), 1);
}
