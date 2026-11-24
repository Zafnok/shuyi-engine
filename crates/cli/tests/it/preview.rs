//! `shuyi preview-worker` and `shuyi preview` on Hello, Grid, the engine's
//! turn game: one visitor's game in a process of its own, the game's tool
//! program that `shuyi` builds with cargo, played by action over its pipes,
//! and the server in front of such workers, played over HTTP. Each answers
//! with a picture and the text of each frame, refuses what isn't a player's
//! action, and ends when its input closes.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;

use serde_json::{Value, json};
use shuyi_cli::codec::decode_png;
use shuyi_platform::link::base64::decode;

/// The mark before each reply line.
const REPLY: &str = "shuyi-preview-reply ";

/// The next reply on `stdout`, past the lines the game prints itself.
fn reply(stdout: &mut BufReader<ChildStdout>) -> Value {
    let mut line = String::new();
    loop {
        line.clear();
        let read = (stdout.read_line(&mut line)).unwrap_or_else(|e| panic!("a reply: {e}"));
        assert_ne!(read, 0, "the worker stopped");
        if let Some(reply) = line.trim_end().strip_prefix(REPLY) {
            return serde_json::from_str(reply).unwrap_or_else(|e| panic!("{e}: {reply}"));
        }
    }
}

/// The reply to `line`.
fn ask(stdin: &mut ChildStdin, stdout: &mut BufReader<ChildStdout>, line: &str) -> Value {
    let sent = writeln!(stdin, "{line}").and_then(|()| stdin.flush());
    sent.unwrap_or_else(|e| panic!("send {line}: {e}"));
    reply(stdout)
}

/// The pixels of `frame`'s picture, and its size.
fn picture(frame: &Value) -> (u32, u32, Vec<u8>) {
    let png = frame["png"].as_str().map(decode);
    let png = png
        .and_then(Result::ok)
        .unwrap_or_else(|| panic!("no picture: {frame}"));
    decode_png(&png).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn a_visitor_plays_hello_grid_in_a_worker_of_its_own() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello-grid");
    let mut worker = Command::new(env!("CARGO_BIN_EXE_shuyi"))
        .args(["preview-worker", "--stage"])
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn shuyi preview-worker: {e}"));
    let mut stdin = worker.stdin.take().unwrap();
    let mut stdout = BufReader::new(worker.stdout.take().unwrap());

    let first = reply(&mut stdout)["frame"].clone();
    // The player's six actions: the developer's Debug key is no visitor's.
    let actions = ["Up", "Down", "Left", "Right", "Confirm", "Cancel"];
    assert_eq!(first["actions"], json!(actions));
    // The stage, 48 x 24 cells of 8 x 16 pixels, with both pieces.
    let (width, height, pixels) = picture(&first);
    assert_eq!((width, height), (384, 384));
    let text = first["text"].as_str().unwrap();
    assert!(text.starts_with(" Turn 1"), "{text}");
    assert!(text.contains('@') && text.contains('&'), "{text}");

    let refused = [
        r#"{"action": "Debug"}"#,
        r#"{"script": "F2"}"#,
        r#"{"action": "Up", "frames": 900}"#,
    ];
    for line in refused {
        let answer = ask(&mut stdin, &mut stdout, line);
        assert!(answer["refused"].is_string(), "{line}: {answer}");
    }
    // Your piece picked up, steered up and right, and sent: it walks, and
    // the foe steps.
    let mut last = Value::Null;
    for action in ["Confirm", "Up", "Right", "Confirm"] {
        let line = json!({ "action": action }).to_string();
        last = ask(&mut stdin, &mut stdout, &line)["frame"].clone();
        assert_eq!(last["ended"], false, "{action}");
    }
    let text = last["text"].as_str().unwrap();
    assert!(text.starts_with(" Turn 2"), "{text}");
    assert_ne!(picture(&last).2, pixels);

    // Its input closed, it ends.
    drop(stdin);
    assert!(worker.wait().unwrap().success());
}

/// The status and the JSON body (`null` if none) of `method` on `path`
/// with `body`, from the server at `address`.
fn http(address: &str, method: &str, path: &str, body: &str) -> (u16, Value) {
    let mut stream = TcpStream::connect(address).unwrap_or_else(|e| panic!("{address}: {e}"));
    let waits = stream.set_read_timeout(Some(Duration::from_secs(120)));
    waits.unwrap_or_else(|e| panic!("{e}"));
    let length = body.len();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {address}\r\nContent-Length: {length}\r\n\r\n{body}"
    );
    let mut text = String::new();
    let answered =
        (stream.write_all(request.as_bytes())).and_then(|()| stream.read_to_string(&mut text));
    answered.unwrap_or_else(|e| panic!("{method} {path}: {e}"));
    let (head, body) = (text.split_once("\r\n\r\n")).unwrap_or_else(|| panic!("{text}"));
    let status = head[9..12]
        .parse()
        .unwrap_or_else(|e| panic!("{e}: {head}"));
    (status, serde_json::from_str(body).unwrap_or(Value::Null))
}

#[test]
fn a_visitor_plays_hello_grid_over_http_as_pictures_and_text() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello-grid");
    let args = "preview --port 0 --deadline 60 --until-stdin-closes --stage";
    let mut server = Command::new(env!("CARGO_BIN_EXE_shuyi"))
        .args(args.split(' '))
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn shuyi preview: {e}"));
    // Its log, and its workers', read as it goes.
    let mut stderr = server.stderr.take().unwrap();
    let log = std::thread::spawn(move || {
        let mut log = String::new();
        let _ = stderr.read_to_string(&mut log);
        log
    });
    let mut lines = BufReader::new(server.stdout.take().unwrap()).lines();
    let address = loop {
        let line = lines.next().expect("the server's address").unwrap();
        if let Some(url) = line.strip_prefix("preview: listening on http://") {
            break url.trim_end_matches('/').to_owned();
        }
    };

    let (status, first) = http(&address, "POST", "/session", "");
    assert_eq!(status, 200, "{first}");
    // The player's six actions, on the stage with both pieces.
    let actions = ["Up", "Down", "Left", "Right", "Confirm", "Cancel"];
    assert_eq!(first["actions"], json!(actions));
    let (width, height, pixels) = picture(&first);
    assert_eq!((width, height), (384, 384));
    let text = first["text"].as_str().unwrap();
    assert!(text.starts_with(" Turn 1"), "{text}");
    let session = format!("/session/{}", first["session"].as_str().unwrap());

    // The developer's Debug key, an unknown action and what isn't one are
    // refused, and the session goes on; nothing but sessions is served.
    let refused = [
        r#"{"action": "Debug"}"#,
        r#"{"action": "Jump"}"#,
        r#"{"script": "F2"}"#,
    ];
    for body in refused {
        assert_eq!(http(&address, "POST", &session, body).0, 400, "{body}");
    }
    let snapshot = format!("{session}/snapshot");
    for unserved in ["/", "/scene", "/files/stage.map", &snapshot] {
        assert_eq!(http(&address, "GET", unserved, "").0, 404, "{unserved}");
    }
    // Your piece picked up, steered up and right, and sent: it walks, and
    // the foe steps.
    let mut last = Value::Null;
    for action in ["Confirm", "Up", "Right", "Confirm"] {
        let body = json!({ "action": action }).to_string();
        let (status, frame) = http(&address, "POST", &session, &body);
        assert_eq!((status, &frame["ended"]), (200, &json!(false)), "{action}");
        last = frame;
    }
    let text = last["text"].as_str().unwrap();
    assert!(text.starts_with(" Turn 2"), "{text}");
    assert_ne!(picture(&last).2, pixels);

    assert_eq!(http(&address, "DELETE", &session, "").0, 204);
    assert_eq!(http(&address, "DELETE", &session, "").0, 404);
    // Its standard input closed, the server ends every worker and stops.
    drop(server.stdin.take());
    assert!(server.wait().unwrap().success());
    // Its log says why each session ended and what it spent, and whether
    // a worker's memory is kept here.
    let log = log.join().unwrap();
    let ended = "preview: a session ended (ended by its visitor): 4 actions, ";
    assert!(log.contains(ended), "{log}");
    let unkept = "--memory isn't kept here";
    if cfg!(target_os = "linux") {
        assert!(log.contains(" held at most; 0 open"), "{log}");
        assert!(!log.contains(unkept), "{log}");
    } else {
        assert!(log.contains("its memory unread; 0 open"), "{log}");
        assert!(log.contains(unkept), "{log}");
    }
}
