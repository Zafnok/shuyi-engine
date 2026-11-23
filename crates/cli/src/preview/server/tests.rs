//! The server over workers on threads, each running the toy in its lobby
//! (Options, Key bindings, Talk, Quit): an action allowed and one refused,
//! each limit, a deadline, an expired session, and the requests nothing
//! answers, over a real connection too.

use std::io::{BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr};

use serde_json::Value;

use super::*;
use crate::preview::worker;
use crate::toy::ToyTools;

/// This test's address, and another's.
const HERE: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
const THERE: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));

/// Workers on threads, each the toy started in its lobby.
fn toy() -> Launch {
    Box::new(|| {
        let (from_server, to_worker) = std::io::pipe()?;
        let (from_worker, to_server) = std::io::pipe()?;
        let args: Vec<String> = ["--frames", "60", "--lobby"].map(str::to_owned).to_vec();
        thread::spawn(move || {
            worker::serve(&ToyTools, &args, BufReader::new(from_server), to_server)
        });
        Ok(Worker::over(Box::new(to_worker), from_worker, None))
    })
}

/// A server of the toy that keeps `limits`.
fn server(limits: Limits) -> Server {
    Server::new(limits, toy())
}

/// The status and the JSON body of `method` on `path` with `body`, from
/// `address`.
fn ask(server: &Server, address: IpAddr, method: &str, path: &str, body: &str) -> (u16, Value) {
    let request = Request {
        method: method.to_owned(),
        path: path.to_owned(),
        body: body.as_bytes().to_vec(),
    };
    let answer = server.answer(&request, address);
    let body = serde_json::from_slice(&answer.body).unwrap_or(Value::Null);
    (answer.status, body)
}

/// A new session's path, from `address`.
fn start(server: &Server, address: IpAddr) -> String {
    let (status, body) = ask(server, address, "POST", "/session", "");
    assert_eq!(status, 200, "{body}");
    format!("/session/{}", body["session"].as_str().unwrap())
}

/// The status and body of `action` pressed in the session at `path`.
fn press(server: &Server, path: &str, action: &str) -> (u16, Value) {
    let body = serde_json::json!({ "action": action }).to_string();
    ask(server, HERE, "POST", path, &body)
}

/// Limits with no rate to wait for.
fn quick() -> Limits {
    Limits {
        rate: 1_000,
        ..Limits::default()
    }
}

#[test]
fn a_session_starts_on_a_frame_and_an_allowed_action_answers_the_next() {
    let server = server(quick());
    let (status, first) = ask(&server, HERE, "POST", "/session", "");
    assert_eq!(status, 200);
    let mut keys: Vec<&str> = first
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["actions", "ended", "png", "session", "text"]);
    let id = first["session"].as_str().unwrap();
    assert_eq!(id_of(&format!("/{id}")), Some(id));
    assert!(first["text"].as_str().unwrap().contains("Key bindings"));
    let path = format!("/session/{id}");
    let (status, next) = press(&server, &path, "CursorDown");
    assert_eq!(status, 200, "{next}");
    assert_eq!(next["session"], first["session"]);
    assert_ne!(next["png"], first["png"]);
    // Ended by the visitor, it is gone.
    assert_eq!(ask(&server, HERE, "DELETE", &path, "").0, 204);
    assert_eq!(press(&server, &path, "CursorDown").0, 404);
    assert_eq!(ask(&server, HERE, "DELETE", &path, "").0, 404);
}

#[test]
fn an_unknown_action_or_body_is_refused_and_the_session_goes_on() {
    let server = server(quick());
    let path = start(&server, HERE);
    for action in ["Debug", "Jump", "cursordown"] {
        let (status, body) = press(&server, &path, action);
        assert_eq!(status, 400, "{action}");
        let error = body["error"].as_str().unwrap();
        assert!(
            error.starts_with(&format!("no action `{action}`")),
            "{error}"
        );
    }
    for body in [
        "",
        "{}",
        "{\"action\": 1}",
        "{\"action\": \"Confirm\", \"frames\": 9}",
    ] {
        assert_eq!(ask(&server, HERE, "POST", &path, body).0, 400, "{body}");
    }
    assert_eq!(press(&server, &path, "CursorDown").0, 200);
    // Quitting ends the session.
    for _ in 0..2 {
        assert_eq!(press(&server, &path, "CursorDown").0, 200);
    }
    let (status, quit) = press(&server, &path, "Confirm");
    assert_eq!((status, &quit["ended"]), (200, &Value::Bool(true)));
    assert_eq!(press(&server, &path, "CursorDown").0, 404);
}

#[test]
fn sessions_at_once_and_per_address_are_bounded() {
    let limits = Limits {
        sessions: 2,
        per_address: 1,
        ..quick()
    };
    let server = server(limits);
    let mine = start(&server, HERE);
    let (status, body) = ask(&server, HERE, "POST", "/session", "");
    assert_eq!(status, 429, "{body}");
    start(&server, THERE);
    let (status, body) = ask(
        &server,
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3)),
        "POST",
        "/session",
        "",
    );
    assert_eq!(status, 503, "{body}");
    // Ending one makes room.
    assert_eq!(ask(&server, HERE, "DELETE", &mine, "").0, 204);
    start(&server, HERE);
}

#[test]
fn an_address_starts_so_many_sessions_a_minute() {
    let server = server(Limits {
        starts: 2,
        ..quick()
    });
    for _ in 0..2 {
        let path = start(&server, HERE);
        assert_eq!(ask(&server, HERE, "DELETE", &path, "").0, 204);
    }
    let request = Request {
        method: "POST".to_owned(),
        path: "/session".to_owned(),
        body: Vec::new(),
    };
    let refused = server.answer(&request, HERE);
    assert_eq!(refused.status, 429);
    assert!(
        refused
            .headers
            .iter()
            .any(|(name, _)| *name == "Retry-After")
    );
    start(&server, THERE);
}

#[test]
fn a_session_presses_so_many_actions_so_fast() {
    let server = server(Limits {
        actions: 2,
        ..quick()
    });
    let path = start(&server, HERE);
    for _ in 0..2 {
        assert_eq!(press(&server, &path, "CursorDown").0, 200);
    }
    let (status, body) = press(&server, &path, "CursorDown");
    assert_eq!(status, 429);
    assert_eq!(body["error"], "the session pressed its 2 actions");
    assert_eq!(press(&server, &path, "CursorDown").0, 404);
    let server = self::server(Limits {
        rate: 1,
        ..Limits::default()
    });
    let path = start(&server, HERE);
    assert_eq!(press(&server, &path, "CursorDown").0, 200);
    let (status, body) = press(&server, &path, "CursorDown");
    assert_eq!(status, 429);
    assert_eq!(body["error"], "a session may press 1 actions each second");
}

#[test]
fn a_session_uses_so_much_of_the_games_time_and_sends_so_much() {
    let server = server(Limits {
        cpu: Duration::ZERO,
        ..quick()
    });
    let path = start(&server, HERE);
    let (status, body) = press(&server, &path, "CursorDown");
    assert_eq!(status, 429);
    assert_eq!(
        body["error"],
        "the session used its 0 seconds of the game's time"
    );
    let server = self::server(Limits {
        egress: 64 << 10,
        ..quick()
    });
    let path = start(&server, HERE);
    let mut pressed = 0;
    let body = loop {
        let (status, body) = press(&server, &path, "CursorDown");
        if status != 200 {
            assert_eq!(status, 429);
            break body;
        }
        pressed += 1;
        assert!(pressed < 100, "64 KB is a few frames of the toy");
    };
    assert_eq!(body["error"], "the session's answers sent their 64 KB");
    assert_eq!(press(&server, &path, "CursorDown").0, 404);
    assert_eq!(
        (size(3 << 20), size(1_536)),
        ("3 MB".to_owned(), "1 KB".to_owned())
    );
}

#[test]
fn a_worker_that_doesnt_answer_in_time_ends_its_session() {
    // A start frame, then nothing: the writer stays open, unwritten.
    let launch: Launch = Box::new(|| {
        let (from_worker, mut to_server) = std::io::pipe()?;
        let shown = worker::Visit::start(&ToyTools, &[], 30, 1).unwrap().shown();
        writeln!(
            to_server,
            "{REPLY}{}",
            serde_json::to_string(&shown).unwrap()
        )?;
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(5));
            drop(to_server);
        });
        Ok(Worker::over(Box::new(std::io::sink()), from_worker, None))
    });
    let limits = Limits {
        deadline: Duration::from_millis(200),
        ..quick()
    };
    let server = Server::new(limits, launch);
    let path = start(&server, HERE);
    let (status, body) = press(&server, &path, "CursorDown");
    assert_eq!(status, 500);
    let error = body["error"].as_str().unwrap();
    assert!(
        error.starts_with("the game stopped: the game took longer than"),
        "{error}"
    );
    assert_eq!(press(&server, &path, "CursorDown").0, 404);
    // One that doesn't start in time never is a session.
    let server = Server::new(
        Limits {
            deadline: Duration::from_millis(200),
            ..quick()
        },
        Box::new(|| {
            let (from_worker, to_server) = std::io::pipe()?;
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(5));
                drop(to_server);
            });
            Ok(Worker::over(Box::new(std::io::sink()), from_worker, None))
        }),
    );
    assert_eq!(ask(&server, HERE, "POST", "/session", "").0, 500);
    assert!(lock(&server.table).live.is_empty());
}

#[test]
fn a_session_idle_or_old_past_its_limit_expires() {
    let server = server(Limits {
        idle: Duration::from_millis(100),
        ..quick()
    });
    let path = start(&server, HERE);
    server.reap();
    assert_eq!(press(&server, &path, "CursorDown").0, 200);
    thread::sleep(Duration::from_millis(150));
    server.reap();
    assert_eq!(press(&server, &path, "CursorDown").0, 404);
    let server = self::server(Limits {
        lifetime: Duration::from_millis(100),
        ..quick()
    });
    let path = start(&server, HERE);
    for _ in 0..3 {
        thread::sleep(Duration::from_millis(50));
        assert_eq!(press(&server, &path, "CursorDown").0, 200, "idle it isn't");
    }
    server.reap();
    assert_eq!(press(&server, &path, "CursorDown").0, 404);
}

#[test]
fn a_bucket_refills_at_its_rate_up_to_its_size() {
    let now = Instant::now();
    let mut bucket = Bucket::new(2, Duration::from_secs(1), now);
    assert_eq!((bucket.take(now), bucket.take(now)), (Ok(()), Ok(())));
    assert_eq!(bucket.take(now), Err(Duration::from_millis(500)));
    let later = now + Duration::from_millis(250);
    assert_eq!(bucket.take(later), Err(Duration::from_millis(250)));
    assert_eq!(bucket.take(now + Duration::from_millis(500)), Ok(()));
    let much_later = now + Duration::from_secs(60);
    assert!(bucket.full(much_later));
    for _ in 0..2 {
        assert_eq!(bucket.take(much_later), Ok(()));
    }
    assert!(bucket.take(much_later).is_err());
}

#[test]
fn the_memory_a_worker_holds_is_read_where_the_system_says() {
    let mine = resident(std::process::id());
    if cfg!(target_os = "linux") {
        assert!(mine.unwrap() > 1 << 20, "{mine:?}");
    } else {
        assert_eq!(mine, None);
    }
}

/// The status and the body of `method` on `path`, over a connection to
/// `address`.
fn over_http(address: SocketAddr, method: &str, path: &str, body: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(address).unwrap();
    let length = body.len();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: preview\r\nContent-Length: {length}\r\n\r\n{body}"
    )
    .unwrap();
    let mut text = String::new();
    stream.read_to_string(&mut text).unwrap();
    let (head, body) = text.split_once("\r\n\r\n").unwrap();
    (head[9..12].parse().unwrap(), body.to_owned())
}

#[test]
fn nothing_but_the_sessions_is_served_over_a_connection() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let address = listener.local_addr().unwrap();
    let server = Arc::new(server(quick()));
    let serving = Arc::clone(&server);
    thread::spawn(move || serve(&listener, &serving));
    let (status, first) = over_http(address, "POST", "/session", "");
    assert_eq!(status, 200, "{first}");
    let first: Value = serde_json::from_str(&first).unwrap();
    let session = format!("/session/{}", first["session"].as_str().unwrap());
    let unserved = [
        "/",
        "/index.html",
        "/scene",
        "/snapshot",
        "/log",
        "/events",
        "/script",
        "/frames",
        "/frame_png",
        "/mcp",
        "/files/keymap.ron",
        "/game.wasm",
        "/game.pack",
        "/src/lib.rs",
        "/.git/config",
        "/sessions",
        "/session/",
        "/session/../files",
        &format!("{session}/scene"),
        &format!("{session}/snapshot"),
        &format!("{session}/log"),
        &session.to_uppercase(),
    ];
    for path in unserved {
        for method in ["GET", "POST", "PUT", "DELETE"] {
            let (status, body) = over_http(address, method, path, "{\"action\": \"Confirm\"}");
            assert_eq!(status, 404, "{method} {path}: {body}");
        }
    }
    for method in ["GET", "PUT", "PATCH"] {
        assert_eq!(over_http(address, method, &session, "").0, 405, "{method}");
        assert_eq!(
            over_http(address, method, "/session", "").0,
            405,
            "{method}"
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
    let mut broken = TcpStream::connect(address).unwrap();
    broken
        .write_all(b"POST /session HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n")
        .unwrap();
    let mut text = String::new();
    broken.read_to_string(&mut text).unwrap();
    assert!(text.starts_with("HTTP/1.1 501 "), "{text}");
    assert_eq!(lock(&server.table).live.len(), 1);
}
