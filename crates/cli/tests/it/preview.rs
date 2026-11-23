//! `shuyi preview-worker` on Hello, Grid, the engine's turn game: one
//! visitor's game in a process of its own, the game's tool program that
//! `shuyi` builds with cargo, played by action over its pipes. It answers
//! with a picture and the text of each frame, refuses what isn't a player's
//! action, and ends when its input closes.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{ChildStdin, ChildStdout, Command, Stdio};

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
