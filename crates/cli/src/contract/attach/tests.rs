//! `attach`: its arguments, and the operation against a test host that
//! runs the link's server on the toy game in a thread, as a backend's
//! listener does.

use std::io::Read as _;
use std::net::{Ipv4Addr, TcpListener};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use shuyi_platform::App;
use shuyi_platform::headless::HeadlessPlatform;
use shuyi_platform::link::{Handled, Server};
use shuyi_ui::toy;

use super::*;

const TOKEN: &str = "0123456789abcdef0123456789abcdef";

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-attach-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The toy game running behind the link's server on a loopback port, in
/// a thread: one client at a time, a second told `busy`, frames run
/// about every millisecond, and a fake picture.
struct Host {
    port: u16,
    stop: Arc<AtomicBool>,
    /// Whether a client is attached.
    attached: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

/// A client attached to the host: its socket, server and what it sent.
type Attached = (TcpStream, Server, Vec<u8>);

impl Host {
    fn start() -> Self {
        let socket = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        socket.set_nonblocking(true).unwrap();
        let port = socket.local_addr().unwrap().port();
        let stop = Arc::new(AtomicBool::new(false));
        let attached = Arc::new(AtomicBool::new(false));
        let (stopping, attaching) = (Arc::clone(&stop), Arc::clone(&attached));
        let thread = thread::spawn(move || serve(&socket, &stopping, &attaching));
        Self {
            port,
            stop,
            attached,
            thread: Some(thread),
        }
    }

    fn address(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    /// Waits until the last client has gone (at most ten seconds), as the
    /// next `attach` of a person or an agent comes a while after.
    fn let_go(&self) {
        let until = std::time::Instant::now() + Duration::from_secs(10);
        while self.attached.load(Ordering::Relaxed) {
            assert!(std::time::Instant::now() < until, "the client stayed attached");
            thread::sleep(Duration::from_millis(1));
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The host's loop, until `stop`.
fn serve(socket: &TcpListener, stop: &AtomicBool, attached: &AtomicBool) {
    let mut app = App::running(toy::game());
    let mut platform = HeadlessPlatform::new();
    let mut client: Option<Attached> = None;
    let mut refused = Vec::new();
    while !stop.load(Ordering::Relaxed) {
        if let Ok((mut stream, _)) = socket.accept() {
            // A client that hung up an instant ago makes way.
            if client
                .as_ref()
                .is_some_and(|(old, _, _)| match old.peek(&mut [0]) {
                    Ok(n) => n == 0,
                    Err(e) => !waiting(e.kind()),
                })
            {
                client = None;
            }
            if client.is_some() {
                // Half closed and kept, so the client reads the line.
                let _ = stream.write_all(format!("{}\n", Line::Busy.to_line()).as_bytes());
                let _ = stream.shutdown(std::net::Shutdown::Write);
                refused.push(stream);
            } else {
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_millis(1)))
                    .unwrap();
                let server = Server::new(TOKEN.to_owned(), "Toy".to_owned());
                client = Some((stream, server, Vec::new()));
            }
        }
        let mut out = String::new();
        let mut close = false;
        let mut input = Vec::new();
        if let Some((stream, server, read)) = &mut client {
            let mut chunk = [0; 4096];
            match stream.read(&mut chunk) {
                Ok(0) => close = true,
                Ok(n) => read.extend_from_slice(&chunk[..n]),
                Err(e) => close = !waiting(e.kind()),
            }
            while let Some(end) = read.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = read.drain(..=end).collect();
                match server.handle(String::from_utf8_lossy(&line).trim_end(), &app) {
                    Handled::Answer(answer) => out.push_str(&format!("{answer}\n")),
                    Handled::Close(last) => {
                        out.push_str(&last.map(|l| format!("{l}\n")).unwrap_or_default());
                        close = true;
                        break;
                    }
                    _ => {}
                }
            }
            server.input(&app, &mut input);
        }
        platform.push_input(&input);
        let _ = app.frame_after(&mut platform, 1.0 / 60.0);
        if let Some((stream, server, _)) = &mut client {
            for answer in server.after_frame(&app) {
                out.push_str(&format!("{answer}\n"));
            }
            if server.wants_screenshot() {
                out.push_str(&format!("{}\n", server.screenshot(b"\x89PNG", 2, 1)));
            }
            if stream.write_all(out.as_bytes()).is_err() || close {
                client = None;
            }
        }
        attached.store(client.is_some(), Ordering::Relaxed);
        thread::sleep(Duration::from_millis(1));
    }
}

/// Whether a read failed only because nothing came in time.
fn waiting(kind: ErrorKind) -> bool {
    matches!(kind, ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted)
}

/// A host that answers any hello with another link version.
fn other_version() -> (String, JoinHandle<()>) {
    let socket = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let address = socket.local_addr().unwrap().to_string();
    let thread = thread::spawn(move || {
        let (mut stream, _) = socket.accept().unwrap();
        let mut hello = String::new();
        let _ = BufReader::new(stream.try_clone().unwrap()).read_line(&mut hello);
        let answer = Line::Hello {
            link: link::VERSION + 1,
            game: "Toy".to_owned(),
        };
        let _ = stream.write_all(format!("{}\n", answer.to_line()).as_bytes());
    });
    (address, thread)
}

fn input(address: &str, steps: Vec<AttachStep>) -> AttachInput {
    let mut input = AttachInput::default();
    input.address = address.to_owned();
    input.token = TOKEN.to_owned();
    input.steps = steps;
    input
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
        (parsed.frames, parsed.screenshot.as_deref(), parsed.log_since),
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
        (vec!["a:1", "--token", "t", "--nope"], "unknown option: --nope"),
        (vec!["a:1", "--token", "t", "--keys", "Nope"], "--keys: unknown key"),
        (vec!["a:1", "--token", "t", "--wait", "x"], "--wait: not a number"),
        (vec!["a:1", "--token", "t", "--wait", "99"], "--wait: 99 is not"),
        (vec!["a:1", "--token", "t", "--act", " "], "--act: no action named"),
        (vec!["a:1", "--token", "t", "--act"], "--act needs a value"),
        (vec!["a:1", "--token", "t", "--frames", "-1"], "--frames: a whole number"),
        (vec!["a:1", "--token", "t", "--log-since", "x"], "--log-since: a whole"),
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
    assert_eq!((output.top_screen.as_str(), output.phase.as_str()), ("menu", "running"));
    assert!(output.focused && !output.suspended && !output.quit);
    assert!(output.frame.contains("--- "), "{}", output.frame);
    let shot = output.screenshot.clone().unwrap_or_else(|| panic!("{output:?}"));
    assert_eq!((shot.path.as_str(), shot.width, shot.height), ("shot.png", 2, 1));
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
    let answer = attach(&input(&host.address(), vec![AttachStep::Act("Confirm".into())]), &dir);
    assert_eq!(answer.result.unwrap().screens, ["home", "menu"]);
    host.let_go();
    let answer = attach(&input(&host.address(), vec![AttachStep::Act("Menu".into())]), &dir);
    assert_eq!(answer.code.as_deref(), Some(codes::WRONG_ARGUMENTS.0), "{answer:?}");
    let error = answer.error.unwrap();
    assert!(error.contains("Menu") && error.contains("neither"), "{error}");
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
    assert!(error.contains(&format!("version {}", link::VERSION + 1)), "{error}");
}

#[test]
fn a_build_with_a_client_attached_is_busy() {
    let host = Host::start();
    let mut first = TcpStream::connect((Ipv4Addr::LOCALHOST, host.port)).unwrap();
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
