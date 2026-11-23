//! `shuyi preview` on Hello, Grid, the engine's turn game, as a visitor
//! plays it over HTTP: the server runs in the game's tool program, which
//! `shuyi` builds with cargo, and each session's game in a worker process
//! of its own. The visitor gets pictures and text, and never the program,
//! its files or its state.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;
use shuyi_cli::codec::decode_png;
use shuyi_platform::link::base64::decode;

/// The status and the JSON body (`null` if none) of `method` on `path`
/// with `body`, from the server at `address`.
fn http(address: &str, method: &str, path: &str, body: &str) -> (u16, Value) {
    let mut stream = TcpStream::connect(address).unwrap_or_else(|e| panic!("{address}: {e}"));
    let length = body.len();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: {address}\r\nContent-Length: {length}\r\n\r\n{body}"
    )
    .unwrap();
    let mut text = String::new();
    stream.read_to_string(&mut text).unwrap();
    let (head, body) = text.split_once("\r\n\r\n").unwrap();
    let status = head[9..12].parse().unwrap();
    (status, serde_json::from_str(body).unwrap_or(Value::Null))
}

/// The picture of `frame`, decoded: its width, height and pixels.
fn picture(frame: &Value) -> (u32, u32, Vec<u8>) {
    let png = decode(frame["png"].as_str().unwrap()).unwrap();
    decode_png(&png).unwrap()
}

#[test]
fn a_visitor_plays_hello_grid_as_pictures_and_text() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello-grid");
    let args = ["preview", "--port", "0", "--until-stdin-closes", "--stage"];
    let mut server = Command::new(env!("CARGO_BIN_EXE_shuyi"))
        .args(args)
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn shuyi preview: {e}"));
    let stdout = server.stdout.take().unwrap();
    let mut lines = BufReader::new(stdout).lines();
    let address = loop {
        let line = lines.next().expect("the server's address").unwrap();
        if let Some(url) = line.strip_prefix("preview: listening on http://") {
            break url.trim_end_matches('/').to_owned();
        }
    };

    let (status, first) = http(&address, "POST", "/session", "");
    assert_eq!(status, 200, "{first}");
    // The player's six actions: the developer's Debug key is no visitor's.
    let actions = ["Up", "Down", "Left", "Right", "Confirm", "Cancel"];
    assert_eq!(first["actions"], serde_json::json!(actions));
    // The stage, 48 x 24 cells of 8 x 16 pixels, with both pieces.
    let (width, height, pixels) = picture(&first);
    assert_eq!((width, height), (384, 384));
    let text = first["text"].as_str().unwrap();
    assert!(text.contains('@') && text.contains('&'), "{text}");
    let session = format!("/session/{}", first["session"].as_str().unwrap());

    for refused in ["Debug", "Jump"] {
        let body = format!("{{\"action\": \"{refused}\"}}");
        assert_eq!(http(&address, "POST", &session, &body).0, 400, "{refused}");
    }
    for unserved in [
        "/",
        "/scene",
        "/files/stage.map",
        &format!("{session}/snapshot"),
    ] {
        assert_eq!(http(&address, "GET", unserved, "").0, 404, "{unserved}");
    }

    // Your piece picked up, steered up and right, and sent: it walks, and
    // the foe steps.
    let mut last = first.clone();
    for action in ["Confirm", "Up", "Right", "Confirm"] {
        let body = format!("{{\"action\": \"{action}\"}}");
        let (status, frame) = http(&address, "POST", &session, &body);
        assert_eq!(status, 200, "{action}: {frame}");
        assert_eq!(frame["ended"], false);
        last = frame;
    }
    assert_ne!(picture(&last).2, pixels);
    assert_ne!(last["text"], first["text"]);

    assert_eq!(http(&address, "DELETE", &session, "").0, 204);
    assert_eq!(http(&address, "DELETE", &session, "").0, 404);
    // Its standard input closed, the server stops.
    drop(server.stdin.take());
    assert!(server.wait().unwrap().success());
}
