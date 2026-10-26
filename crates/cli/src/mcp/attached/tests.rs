//! The session attached over the live link, through the server: each tool
//! on the toy behind a test host (`crate::toy::link`), the refusals and
//! `attach`'s codes.

use std::fs;
use std::net::{Ipv4Addr, TcpListener};
use std::path::PathBuf;

use serde_json::json;

use super::*;
use crate::contract::fake::Fake;
use crate::mcp::host::InProcess;
use crate::mcp::server::Server;
use crate::toy::link::{GAME, LinkHost, TOKEN, other_version};

/// A fresh folder holding a project whose game is the fake one.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-mcp-link-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("shuyi.ron"), r#"(game: (package: "fake"))"#).unwrap();
    dir
}

/// The answer of tool `name` called with `arguments`, and its picture.
fn called(server: &mut Server<'_>, name: &str, arguments: &Value) -> (Value, Option<String>) {
    let message = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                          "params": { "name": name, "arguments": arguments } });
    let written = server.handle(&message.to_string());
    let response: Value = serde_json::from_str(&written[0]).unwrap();
    let result = &response["result"];
    let png = result["content"][1]["data"].as_str().map(str::to_owned);
    (result["structuredContent"].clone(), png)
}

/// `start`'s arguments attaching to `address` with `token`.
fn link(address: &str, token: &str) -> Value {
    json!({ "link": { "address": address, "token": token } })
}

/// Whether `answer`'s limits hold one containing `part`.
fn limited(answer: &Value, part: &str) -> bool {
    answer["limits"]
        .as_array()
        .unwrap()
        .iter()
        .any(|l| l.as_str().unwrap().contains(part))
}

#[test]
fn each_session_tool_reads_or_plays_the_running_build() {
    let host = LinkHost::start();
    let dir = scratch("tools");
    let game = Fake::default();
    let mut server = Server::new(dir.clone(), Box::new(InProcess::new(&game)));

    let (started, _) = called(&mut server, "start", &link(&host.address(), TOKEN));
    assert_eq!(started["ok"], true, "{started}");
    let result = &started["result"];
    assert_eq!(
        (result["game"].as_str(), result["project"].as_str()),
        (Some(GAME), Some(""))
    );
    assert_eq!(result["options"], json!([]));
    assert_eq!(result["top_screen"], "home", "{result}");
    assert!(limited(&started, "stop leaves it running"), "{started}");
    assert!(server.has_game() && host.attached());

    // `f` opens the toy's menu, as `attach` plays it.
    let script = json!({ "script": { "steps": [{ "keys": "f" }] }, "frames": 2 });
    let (stepped, _) = called(&mut server, "step", &script);
    assert_eq!(stepped["ok"], true, "{stepped}");
    let result = &stepped["result"];
    assert_eq!(result["screens"], json!(["home", "menu"]));
    assert!(result["frame"].as_str().unwrap().contains("--- "));
    assert_eq!(
        (result["scene"].clone(), result["events"].clone()),
        (Value::Null, json!([]))
    );
    for limit in [NO_SCENE, NO_EVENTS, "own frames"] {
        assert!(limited(&stepped, limit), "{limit}: {stepped}");
    }

    let (picture, png) = called(&mut server, "frame_png", &json!({}));
    assert_eq!(picture["ok"], true, "{picture}");
    assert_eq!(
        (
            picture["result"]["width"].as_u64(),
            picture["result"]["height"].as_u64()
        ),
        (Some(2), Some(1))
    );
    let fake_png = shuyi_platform::link::base64::encode(b"\x89PNG");
    assert_eq!(png.as_deref(), Some(fake_png.as_str()));
    let (scaled, _) = called(&mut server, "frame_png", &json!({ "scale": 2 }));
    assert_eq!(scaled["code"], "SH0001");
    assert!(
        scaled["error"]
            .as_str()
            .unwrap()
            .contains("the window's size")
    );

    let (scene, _) = called(&mut server, "scene", &json!({}));
    assert_eq!(scene["ok"], true, "{scene}");
    let shown = &scene["result"]["scene"];
    assert_eq!(
        (shown["kind"].as_str(), shown["screen"].as_str()),
        (Some("frame"), Some("menu"))
    );
    assert!(shown["text"].as_str().unwrap().contains("--- "));
    assert!(limited(&scene, NO_SCENE), "{scene}");

    let (events, _) = called(&mut server, "events", &json!({}));
    assert_eq!(
        (
            events["result"]["events"].clone(),
            events["result"]["sounds"].clone()
        ),
        (json!([]), json!([]))
    );
    assert!(limited(&events, NO_EVENTS), "{events}");

    let (reset, _) = called(&mut server, "reset", &json!({}));
    assert_eq!(reset["code"], "SH0001");
    let error = reset["error"].as_str().unwrap();
    assert!(error.contains("never changes a running build"), "{error}");
    assert!(server.has_game(), "a refused reset keeps the session");

    // Stop detaches and leaves the build running; a second start attaches
    // again.
    let (detached, _) = called(&mut server, "stop", &json!({}));
    assert_eq!(detached["ok"], true, "{detached}");
    assert!(detached["result"]["frames"].as_u64().unwrap() > 0);
    assert!(!server.has_game());
    host.let_go();
    let (again, _) = called(&mut server, "start", &link(&host.address(), TOKEN));
    assert_eq!(again["ok"], true, "{again}");
    assert_eq!(again["result"]["top_screen"], "menu");
    let (stepped, _) = called(&mut server, "step", &json!({}));
    assert_eq!(stepped["ok"], true, "{stepped}");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn start_with_a_link_says_why_it_cant_attach_with_the_codes_of_attach() {
    let host = LinkHost::start();
    let dir = scratch("codes");
    let game = Fake::default();
    let mut server = Server::new(dir.clone(), Box::new(InProcess::new(&game)));
    let code = |server: &mut Server<'_>, arguments: &Value| {
        let (answer, _) = called(server, "start", arguments);
        assert_eq!(answer["ok"], false, "{answer}");
        answer["code"].as_str().unwrap().to_owned()
    };
    let wrong = "fedcba9876543210fedcba9876543210";
    assert_eq!(code(&mut server, &link(&host.address(), wrong)), "SH0004");
    let free = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let nowhere = free.local_addr().unwrap().to_string();
    drop(free);
    assert_eq!(code(&mut server, &link(&nowhere, TOKEN)), "SH1005");
    let (other, thread) = other_version();
    assert_eq!(code(&mut server, &link(&other, TOKEN)), "SH1006");
    thread.join().unwrap();
    assert_eq!(code(&mut server, &link("", TOKEN)), "SH0001");
    let mut options = link(&host.address(), TOKEN);
    options["start"] = json!([{ "flag": "--seed", "value": "1" }]);
    assert_eq!(code(&mut server, &options), "SH0001");
    assert!(!server.has_game());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_start_ends_the_session_before_and_a_lost_link_ends_this_one() {
    let host = LinkHost::start();
    let dir = scratch("ends");
    let game = Fake::default();
    let mut server = Server::new(dir.clone(), Box::new(InProcess::new(&game)));
    let (started, _) = called(&mut server, "start", &json!({}));
    assert_eq!(started["ok"], true, "{started}");
    let (attached, _) = called(&mut server, "start", &link(&host.address(), TOKEN));
    let stopped = "the game that was running was stopped";
    assert!(limited(&attached, stopped), "{attached}");
    let (headless, _) = called(&mut server, "start", &json!({}));
    assert!(
        limited(&headless, "attached was left running"),
        "{headless}"
    );
    host.let_go();

    let (attached, _) = called(&mut server, "start", &link(&host.address(), TOKEN));
    assert_eq!(attached["ok"], true, "{attached}");
    drop(host);
    let (lost, _) = called(&mut server, "events", &json!({}));
    assert_eq!(lost["code"], "SH3006", "{lost}");
    assert!(!server.has_game());
    let (after, _) = called(&mut server, "step", &json!({}));
    assert_eq!(after["code"], "SH0002");
    fs::remove_dir_all(&dir).unwrap();
}

/// The names of `answer`'s result's fields, sorted.
fn fields(answer: &Value) -> Vec<String> {
    let result = answer["result"].as_object();
    let mut names: Vec<String> = result.into_iter().flat_map(|r| r.keys().cloned()).collect();
    names.sort();
    names
}

/// The frames `perf`'s answer lists as kept.
fn kept(answer: &Value) -> Vec<u64> {
    let kept = answer["result"]["kept"].as_array();
    kept.into_iter()
        .flatten()
        .filter_map(Value::as_u64)
        .collect()
}

/// `perf` called on `server`'s session as an agent reads a trace: every
/// record, a snapshot, frames stepped, a second snapshot with the diff
/// from the first, a diff from a frame not kept, and a `max` too large.
fn perf_reads(server: &mut Server<'_>) -> Vec<Value> {
    let mut answers = Vec::new();
    let mut perf = |server: &mut Server<'_>, arguments: Value| {
        let (answer, _) = called(server, "perf", &arguments);
        answers.push(answer.clone());
        answer
    };
    called(server, "step", &json!({ "frames": 10 }));
    let all = perf(server, json!({ "since": 0 }));
    let next = all["result"]["next"].as_u64().unwrap();
    let first = perf(server, json!({ "since": next, "snapshot": true }));
    let from = first["result"]["snapshot"]["frame"].as_u64().unwrap();
    called(server, "step", &json!({ "frames": 5 }));
    perf(
        server,
        json!({ "since": next, "snapshot": true, "diff_from": from }),
    );
    perf(server, json!({ "diff_from": 99_999 }));
    perf(server, json!({ "max": 601 }));
    answers
}

#[test]
fn perf_on_an_attached_session_answers_as_on_a_headless_one() {
    let host = LinkHost::start();
    let dir = scratch("perf");
    let toy = crate::toy::ToyTools;
    let mut headless = Server::new(dir.clone(), Box::new(InProcess::new(&toy)));
    let (started, _) = called(&mut headless, "start", &json!({}));
    assert_eq!(started["ok"], true, "{started}");
    let game = Fake::default();
    let mut attached = Server::new(dir.clone(), Box::new(InProcess::new(&game)));
    let (started, _) = called(&mut attached, "start", &link(&host.address(), TOKEN));
    assert_eq!(started["ok"], true, "{started}");

    let alike = perf_reads(&mut headless);
    let read = perf_reads(&mut attached);
    for (headless, attached) in alike.iter().zip(&read) {
        assert_eq!(attached["ok"], headless["ok"], "{attached}\n{headless}");
        assert_eq!(attached["code"], headless["code"], "{attached}");
        assert_eq!(fields(attached), fields(headless), "{attached}");
        assert_eq!(attached["limits"], headless["limits"], "{attached}");
    }
    let [all, first, second, not_kept, too_many] = read.as_slice() else {
        panic!("{read:?}");
    };
    // Every record kept, one per frame, with its heap counts.
    let frames: Vec<u64> = all["result"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["frame"].as_u64())
        .collect();
    assert!(frames.len() >= 10, "{all}");
    assert!(frames.windows(2).all(|w| w[1] == w[0] + 1), "{frames:?}");
    assert!(all["result"]["records"][0]["heap"].is_object(), "{all}");
    assert_eq!(all["result"]["summary"]["frames"], frames.len());
    // The snapshots are kept on this side, by frame.
    let from = first["result"]["snapshot"]["frame"].as_u64().unwrap();
    let to = second["result"]["snapshot"]["frame"].as_u64().unwrap();
    assert_eq!((kept(first), kept(second)), (vec![from], vec![from, to]));
    let diff = &second["result"]["diff"];
    assert_eq!(
        (diff["from"].as_u64(), diff["to"].as_u64()),
        (Some(from), Some(to))
    );
    let error = not_kept["error"].as_str().unwrap();
    assert!(error.contains(&format!("kept: {from}, {to}")), "{error}");
    assert_eq!(too_many["code"], "SH0001");
    assert!(attached.has_game(), "a refused perf keeps the session");
    fs::remove_dir_all(&dir).unwrap();
}
