//! The host on the fake game: the session's tools, the one-off
//! operations, and its lines in and out.

use std::fs;
use std::path::PathBuf;

use super::*;
use crate::contract::fake::{Fake, FakeSession};
use crate::project::Config;

/// A fresh empty folder under the system temp folder.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-mcp-host-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn project(root: PathBuf) -> Project {
    Project {
        root,
        config: Config::default(),
    }
}

/// The answer of `op` to `input` from `host`.
fn ask(host: &mut Host<'_>, op: Op, input: Value) -> Value {
    host.call(&Request { op, input }).answer.unwrap()
}

#[test]
fn the_session_starts_plays_looks_and_stops() {
    let game = Fake::default();
    let mut host = Host::new(&game, project(PathBuf::from(".")));
    let started = ask(
        &mut host,
        Op::Start,
        json!({ "start": [{ "flag": "--seed", "value": "3" }] }),
    );
    assert_eq!(started["op"], "start");
    assert_eq!(started["ok"], true);
    let result = &started["result"];
    assert_eq!(result["start"][0]["value"], "3");
    assert_eq!(result["options"][0]["flag"], "--seed");
    assert_eq!(result["options"][0]["value"], "N");
    assert_eq!(result["top_screen"], "stage");
    assert_eq!(result["frames"], 0);

    let stepped = ask(
        &mut host,
        Op::Step,
        json!({ "script": { "steps": [{ "keys": "Enter Up" }] }, "frames": 3 }),
    );
    assert_eq!(
        (stepped["op"].as_str(), stepped["ok"].as_bool()),
        (Some("step"), Some(true))
    );
    assert_eq!(stepped["result"]["frames"], 7);
    assert_eq!(stepped["result"]["scene"]["kind"], "grid map");
    assert!(stepped["result"]["log"].is_array());
    // It plays on from where it was.
    let stepped = ask(&mut host, Op::Step, json!({ "frames": 1 }));
    assert_eq!(stepped["result"]["frames"], 8);

    let scene = ask(&mut host, Op::Scene, Value::Null);
    assert_eq!(
        (scene["op"].as_str(), scene["ok"].as_bool()),
        (Some("scene"), Some(true))
    );
    assert_eq!(scene["result"]["frames"], 8);
    assert_eq!(scene["result"]["scene"]["text"], "map");

    let events = ask(&mut host, Op::Events, json!({}));
    assert_eq!(events["result"]["events"][0]["kind"], "opened");
    assert_eq!(events["result"]["sounds"][0]["cue"], "click");

    let reply = host.call(&Request {
        op: Op::FramePng,
        input: json!({ "scale": 2 }),
    });
    let answer = reply.answer.unwrap();
    assert_eq!(
        (
            answer["result"]["width"].as_u64(),
            answer["result"]["height"].as_u64()
        ),
        (Some(16), Some(2))
    );
    assert!(
        reply.png.unwrap().starts_with("iVBORw0KGgo"),
        "a PNG in base64"
    );

    let reset = ask(&mut host, Op::Reset, json!({}));
    assert_eq!(
        (reset["op"].as_str(), reset["ok"].as_bool()),
        (Some("reset"), Some(true))
    );
    assert_eq!(reset["result"]["start"][0]["value"], "3", "as it started");
    assert_eq!(ask(&mut host, Op::Scene, json!({}))["result"]["frames"], 0);

    let ended = ask(&mut host, Op::Stop, json!({}));
    assert_eq!(ended["result"]["frames"], 0);
    for op in [
        Op::Step,
        Op::Scene,
        Op::Events,
        Op::FramePng,
        Op::Reset,
        Op::Stop,
    ] {
        let answer = ask(&mut host, op, json!({}));
        assert_eq!(answer["code"], "SH0002", "{op:?}");
        assert_eq!(answer["op"], op.name());
    }
}

#[test]
fn a_session_tool_refuses_input_it_doesnt_take_and_keeps_its_game() {
    let game = Fake::default();
    let mut host = Host::new(&game, project(PathBuf::from(".")));
    assert_eq!(
        ask(&mut host, Op::Start, json!({ "nope": 1 }))["code"],
        "SH0001"
    );
    ask(&mut host, Op::Start, json!({}));
    for (op, input) in [
        (
            Op::Step,
            json!({ "script": { "steps": [{ "keys": "NoSuchKey" }] } }),
        ),
        (Op::Step, json!({ "frames": 100_001 })),
        (Op::Step, json!({ "frame": 1 })),
        (Op::Scene, json!({ "x": 1 })),
        (Op::Events, json!({ "since": 1 })),
        (Op::FramePng, json!({ "scale": 9 })),
        (Op::FramePng, json!({ "scale": 0 })),
        (Op::FramePng, json!({ "scale": "big" })),
        (Op::Reset, json!({ "x": 1 })),
        (Op::Stop, json!({ "x": 1 })),
    ] {
        let answer = ask(&mut host, op, input.clone());
        assert_eq!(answer["code"], "SH0001", "{op:?} {input}: {answer}");
        assert!(answer["help"].is_string());
    }
    assert_eq!(
        ask(&mut host, Op::Scene, json!({}))["ok"],
        true,
        "still running"
    );
}

#[test]
fn a_game_that_wont_start_or_paint_says_why() {
    let game = Fake {
        session: Err("no such stage".into()),
        ..Fake::default()
    };
    let mut host = Host::new(&game, project(PathBuf::from(".")));
    let answer = ask(&mut host, Op::Start, json!({}));
    assert_eq!(
        (answer["code"].as_str(), answer["error"].as_str()),
        (Some("SH3002"), Some("no such stage"))
    );
    assert_eq!(ask(&mut host, Op::Scene, json!({}))["code"], "SH0002");

    // A session with no scene isn't ok, and says why in its limits.
    let mut blank = FakeSession::on("title", "");
    blank.scene = Err("no stage is open".into());
    let game = Fake {
        session: Ok(blank),
        ..Fake::default()
    };
    let mut host = Host::new(&game, project(PathBuf::from(".")));
    ask(&mut host, Op::Start, json!({}));
    let scene = ask(&mut host, Op::Scene, json!({}));
    assert_eq!(scene["ok"], false);
    assert_eq!(scene["limits"][0], "no scene: no stage is open");
}

/// A session that says only what a session must, and paints nothing.
struct Unpainted;

impl Session for Unpainted {
    fn play(&mut self, _: &crate::frame_png::Script) {}
    fn idle(&mut self, _: u32) {}
    fn frame_count(&self) -> u32 {
        0
    }
    fn screens(&self) -> Vec<String> {
        Vec::new()
    }
    fn quit_frame(&self) -> Option<u32> {
        None
    }
    fn snapshot(&self) -> String {
        String::new()
    }
    fn events(&self) -> Vec<crate::contract::session::Event> {
        Vec::new()
    }
    fn sounds(&self) -> Vec<crate::contract::session::Sound> {
        Vec::new()
    }
    fn scene(&self) -> Result<crate::contract::session::Scene, String> {
        Err("none".to_owned())
    }
}

/// A game whose session is [`Unpainted`].
struct Plain;

impl GameTools for Plain {
    fn frame_options(&self) -> &[crate::game::GameOption] {
        &[]
    }
    fn frame(
        &self,
        _: &[GameArg],
        _: &crate::frame_png::Script,
        _: Option<u32>,
    ) -> Result<crate::frame_png::Shot, String> {
        Err("no frame".into())
    }
    fn lang(&self) -> Result<crate::game::GameLang, String> {
        Err("no lang".into())
    }
    fn session(&self, _: &[GameArg]) -> Result<Box<dyn Session>, String> {
        Ok(Box::new(Unpainted))
    }
}

#[test]
fn a_session_that_paints_nothing_answers_not_offered() {
    let mut host = Host::new(&Plain, project(PathBuf::from(".")));
    let started = ask(&mut host, Op::Start, json!({}));
    assert_eq!(started["result"]["top_screen"], "");
    let reply = host.call(&Request {
        op: Op::FramePng,
        input: json!({}),
    });
    let answer = reply.answer.unwrap();
    assert_eq!(answer["code"], "SH3001");
    assert!(reply.png.is_none());
}

#[test]
fn the_one_off_operations_answer_as_the_cli_does() {
    let dir = scratch("one-off");
    fs::write(dir.join("a.ron"), "h0\nh1").unwrap();
    let game = Fake::default();
    let mut host = Host::new(&game, project(dir.clone()));
    for (op, input, ok) in [
        (Op::Check, json!({}), true),
        (Op::Run, json!({ "frames": 2 }), true),
        (Op::Replay, json!({ "record": "a.ron" }), true),
        (Op::Bots, json!({ "games": 2 }), true),
        (Op::Playtest, json!({}), true),
        (Op::Import, json!({ "file": "a.ron" }), false),
        (Op::Explain, json!({ "code": "SH0002" }), true),
        (Op::Doctor, json!({}), true),
    ] {
        let answer = ask(&mut host, op, input);
        assert_eq!(answer["op"], op.name(), "{answer}");
        if ok && op != Op::Doctor {
            assert_eq!(answer["ok"], true, "{op:?}: {answer}");
        }
    }
    let answer = ask(&mut host, Op::Bots, json!({ "games": 0 }));
    assert_eq!(
        (answer["ok"].as_bool(), answer["code"].as_str()),
        (Some(false), Some("SH0001"))
    );
    assert_eq!(answer["error"], "games: 1 to 10000, not 0");
    let answer = ask(&mut host, Op::Bake, json!({ "nope": true }));
    assert_eq!(answer["code"], "SH0001");
    assert!(answer["error"].as_str().unwrap().starts_with("input: "));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn without_the_games_tools_or_a_project_an_operation_says_what_it_lacks() {
    let dir = scratch("lacks");
    let here: Result<Project, (Code, String)> = Ok(project(dir.clone()));
    let nowhere: Result<Project, (Code, String)> = Err((codes::NO_PROJECT, "no shuyi.ron".into()));
    for op in [
        Op::Check,
        Op::Run,
        Op::Scene,
        Op::Replay,
        Op::Bots,
        Op::Playtest,
        Op::Bake,
    ] {
        let answer = one_off(op, json!({}), &dir, &here, None);
        assert_eq!(answer["code"], "SH1002", "{op:?}");
        assert!(
            answer["error"]
                .as_str()
                .unwrap()
                .contains("names no game tool program")
        );
        let answer = one_off(op, json!({}), &dir, &nowhere, None);
        assert_eq!(answer["code"], "SH1001", "{op:?}");
    }
    // `check --target` needs the project file alone: here it lists no
    // target.
    let answer = one_off(Op::Check, json!({ "target": "all" }), &dir, &here, None);
    assert_eq!(answer["code"], "SH3003", "{answer}");
    let answer = one_off(Op::Check, json!({ "target": "all" }), &dir, &nowhere, None);
    assert_eq!(answer["code"], "SH1001");
    // `import` reads the engine's standard kinds, and says so.
    fs::write(dir.join("colors.ron"), "{ \"text\": \"#ffffff\" }").unwrap();
    for (project, said) in [
        (&here, "names no game tool program"),
        (&nowhere, "no project here"),
    ] {
        let answer = one_off(
            Op::Import,
            json!({ "file": "colors.ron" }),
            &dir,
            project,
            None,
        );
        assert_eq!(answer["ok"], true, "{answer}");
        assert!(answer["limits"].to_string().contains(said), "{answer}");
    }
    // `new` makes its folder from the working folder.
    let answer = one_off(
        Op::New,
        json!({ "template": "blank", "dir": "g" }),
        &dir,
        &nowhere,
        None,
    );
    assert_eq!(answer["ok"], true, "{answer}");
    assert!(dir.join("g/shuyi.ron").is_file());
    // A session tool is no operation.
    let answer = one_off(Op::Step, json!({}), &dir, &here, None);
    assert_eq!(answer["code"], "SH0001");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_host_answers_a_line_with_a_line() {
    let game = Fake::default();
    let input =
        "{\"op\":\"start\",\"input\":{}}\n\n{\"op\":\"scene\"}\nnot json\n{\"op\":\"teleport\"}\n";
    let mut output = Vec::new();
    assert_eq!(
        serve(
            &game,
            project(PathBuf::from(".")),
            input.as_bytes(),
            &mut output
        ),
        0
    );
    let replies: Vec<Reply> = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(replies.len(), 4);
    assert_eq!(replies[0].answer.as_ref().unwrap()["op"], "start");
    assert_eq!(replies[1].answer.as_ref().unwrap()["ok"], true);
    for refused in &replies[2..] {
        assert!(refused.answer.is_none());
        assert!(
            refused
                .error
                .as_ref()
                .unwrap()
                .contains("doesn't read the request")
        );
    }
}

#[test]
fn the_host_in_process_is_a_link() {
    let game = Fake::default();
    let launch = InProcess::new(&game);
    let mut link = launch.launch(&project(PathBuf::from("."))).unwrap();
    let reply = link
        .call(&Request {
            op: Op::Start,
            input: json!({}),
        })
        .unwrap();
    assert_eq!(reply.answer.unwrap()["ok"], true);
}
