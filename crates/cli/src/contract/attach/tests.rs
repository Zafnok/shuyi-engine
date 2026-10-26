//! `attach`: its arguments, and the operation against a test host that
//! runs the link's server on the toy game in a thread, as a backend's
//! listener does (`crate::toy::link`).

use std::net::{Ipv4Addr, TcpListener};
use std::path::PathBuf;

use super::*;
use crate::toy::link::{LinkHost as Host, TOKEN, other_version};

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-attach-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn input(address: &str, steps: Vec<AttachStep>) -> AttachInput {
    AttachInput {
        address: address.to_owned(),
        token: TOKEN.to_owned(),
        steps,
        ..AttachInput::default()
    }
}

#[test]
fn its_arguments_are_an_address_a_token_steps_and_what_to_read() {
    let parsed = parse_args(&args(&[
        "127.0.0.1:7357",
        "--token",
        TOKEN,
        "--keys",
        "Enter Up",
        "--act",
        "Confirm",
        "--pad",
        "South+East",
        "--wait",
        "0.5",
        "--frames",
        "3",
        "--screenshot",
        "shot.png",
        "--log-since",
        "12",
    ]))
    .unwrap();
    assert_eq!(parsed.address, "127.0.0.1:7357");
    assert_eq!(parsed.token, TOKEN);
    assert_eq!(
        parsed.steps,
        [
            AttachStep::Keys("Enter Up".into()),
            AttachStep::Act("Confirm".into()),
            AttachStep::Pad("South+East".into()),
            AttachStep::Wait(0.5),
        ]
    );
    assert_eq!(
        (
            parsed.frames,
            parsed.screenshot.as_deref(),
            parsed.log_since
        ),
        (3, Some("shot.png"), 12)
    );
    assert_eq!(
        parsed.link_steps(),
        [
            link::Step::Keys("Enter Up".into()),
            link::Step::Act("Confirm".into()),
            link::Step::Pad("South+East".into()),
            link::Step::Wait(30),
            link::Step::Wait(3),
        ]
    );
    // `--script` is `--keys`, as for `run`.
    let script = parse_args(&args(&["h:1", "--token", "t", "--script", "f"])).unwrap();
    assert_eq!(script.steps, [AttachStep::Keys("f".into())]);
    for (bad, said) in [
        (vec!["--token", TOKEN], "address is missing"),
        (vec!["127.0.0.1:1"], "--token is missing"),
        (vec!["a:1", "b:2", "--token", "t"], "one address"),
        (
            vec!["a:1", "--token", "t", "--nope"],
            "unknown option: --nope",
        ),
        (
            vec!["a:1", "--token", "t", "--keys", "Nope"],
            "--keys: unknown key",
        ),
        (
            vec!["a:1", "--token", "t", "--wait", "x"],
            "--wait: not a number",
        ),
        (
            vec!["a:1", "--token", "t", "--wait", "99"],
            "--wait: 99 is not",
        ),
        (
            vec!["a:1", "--token", "t", "--act", " "],
            "--act: no action named",
        ),
        (vec!["a:1", "--token", "t", "--act"], "--act needs a value"),
        (
            vec!["a:1", "--token", "t", "--frames", "-1"],
            "--frames: a whole number",
        ),
        (
            vec!["a:1", "--token", "t", "--log-since", "x"],
            "--log-since: a whole",
        ),
    ] {
        let error = parse_args(&args(&bad)).unwrap_err();
        assert!(error.contains(said), "{bad:?}: {error}");
    }
}

#[test]
fn a_wait_is_the_builds_frames_at_60_a_second() {
    assert_eq!(frames_of(0.0), 0);
    assert_eq!(frames_of(1.0 / 60.0), 1);
    assert_eq!(frames_of(0.5), 30);
    assert_eq!(frames_of(2.0), 120);
    assert_eq!(frames_of(-1.0), 0);
}

#[test]
fn it_plays_the_steps_in_the_running_build_and_reads_what_it_shows() {
    let host = Host::start();
    let dir = scratch("plays");
    let mut asked = input(&host.address(), vec![AttachStep::Keys("f".into())]);
    asked.frames = 2;
    asked.screenshot = Some("shot.png".to_owned());
    let answer = attach(&asked, &dir);
    assert!(answer.ok, "{answer:?}");
    let output = answer.result.unwrap();
    assert_eq!((output.link, output.game.as_str()), (link::VERSION, "Toy"));
    assert_eq!(output.screens, ["home", "menu"]);
    assert_eq!(
        (output.top_screen.as_str(), output.phase.as_str()),
        ("menu", "running")
    );
    assert!(output.focused && !output.suspended && !output.quit);
    assert!(output.frame.contains("--- "), "{}", output.frame);
    let shot = output
        .screenshot
        .clone()
        .unwrap_or_else(|| panic!("{output:?}"));
    assert_eq!(
        (shot.path.as_str(), shot.width, shot.height),
        ("shot.png", 2, 1)
    );
    assert_eq!(fs::read(dir.join("shot.png")).unwrap(), b"\x89PNG");
    assert!(
        answer.limits.iter().any(|l| l.contains("own frames")),
        "{:?}",
        answer.limits
    );
    // Reading the log from where it ended gives nothing new.
    host.let_go();
    let mut again = input(&host.address(), Vec::new());
    again.log_since = output.log_next;
    let answer = attach(&again, &dir);
    let output = answer.result.unwrap();
    assert_eq!(output.log, []);
    assert_eq!(output.screens, ["home", "menu"]);
    assert_eq!(answer.limits, Vec::<String>::new());
    let text = text(&output);
    assert!(text.starts_with("attach: Toy on frame "), "{text}");
    assert!(text.contains("top screen: menu (home > menu)"), "{text}");
}

#[test]
fn an_action_is_pressed_by_its_key_and_one_it_cant_press_is_named() {
    let host = Host::start();
    let dir = scratch("act");
    let answer = attach(
        &input(&host.address(), vec![AttachStep::Act("Confirm".into())]),
        &dir,
    );
    assert_eq!(answer.result.unwrap().screens, ["home", "menu"]);
    host.let_go();
    let answer = attach(
        &input(&host.address(), vec![AttachStep::Act("Menu".into())]),
        &dir,
    );
    assert_eq!(
        answer.code.as_deref(),
        Some(codes::WRONG_ARGUMENTS.0),
        "{answer:?}"
    );
    let error = answer.error.unwrap();
    assert!(
        error.contains("Menu") && error.contains("neither"),
        "{error}"
    );
}

#[test]
fn no_build_listening_is_a_setup_to_fix() {
    let free = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let address = free.local_addr().unwrap().to_string();
    drop(free);
    let answer = attach(&input(&address, Vec::new()), &scratch("none"));
    assert_eq!(answer.code.as_deref(), Some(codes::NO_BUILD.0));
    assert_eq!(answer.exit_code(), 3);
    assert!(answer.error.unwrap().contains(&address));
}

#[test]
fn a_wrong_token_and_another_link_version_have_their_codes() {
    let host = Host::start();
    let mut wrong = input(&host.address(), Vec::new());
    wrong.token = "fedcba9876543210fedcba9876543210".to_owned();
    let answer = attach(&wrong, &scratch("token"));
    assert_eq!(answer.code.as_deref(), Some(codes::LINK_TOKEN.0));
    assert_eq!(answer.exit_code(), 2);
    assert!(answer.error.unwrap().contains("token is wrong"));

    let (address, thread) = other_version();
    let answer = attach(&input(&address, Vec::new()), &scratch("version"));
    thread.join().unwrap();
    assert_eq!(answer.code.as_deref(), Some(codes::LINK_VERSION.0));
    assert_eq!(answer.exit_code(), 3);
    let error = answer.error.unwrap();
    assert!(
        error.contains(&format!("version {}", link::VERSION + 1)),
        "{error}"
    );
}

#[test]
fn a_build_with_a_client_attached_is_busy() {
    let host = Host::start();
    let mut first = TcpStream::connect((Ipv4Addr::LOCALHOST, host.port())).unwrap();
    let hello = Request::Hello {
        token: TOKEN.to_owned(),
        link: link::VERSION,
    };
    first
        .write_all(format!("{}\n", hello.to_line()).as_bytes())
        .unwrap();
    let mut reader = BufReader::new(first.try_clone().unwrap());
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let answer = attach(&input(&host.address(), Vec::new()), &scratch("busy"));
    assert_eq!(answer.code.as_deref(), Some(codes::LINK_LOST.0));
    assert!(answer.error.unwrap().contains("another client"));
}

#[test]
fn a_wrong_input_is_refused_before_connecting() {
    let mut asked = input("127.0.0.1:1", Vec::new());
    asked.token = String::new();
    let answer = attach(&asked, &scratch("refused"));
    assert_eq!(answer.code.as_deref(), Some(codes::WRONG_ARGUMENTS.0));
    let mut asked = input("127.0.0.1:1", vec![AttachStep::Wait(-1.0)]);
    asked.frames = MAX_IDLE_FRAMES + 1;
    assert_eq!(attach(&asked, &scratch("refused")).exit_code(), 2);
}
