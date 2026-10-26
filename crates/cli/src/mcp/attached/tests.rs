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

    let (perf, _) = called(&mut server, "perf", &json!({}));
    assert_eq!(perf["code"], "SH3001");
    let error = perf["error"].as_str().unwrap();
    assert!(error.contains("SHUYI_PERF=<file>"), "{error}");
    assert!(server.has_game(), "a refused perf keeps the session");

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
