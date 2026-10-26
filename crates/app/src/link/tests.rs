//! The listener over a real loopback socket, on the toy game and the
//! headless backend (no window): a client's hello, play and reads, the
//! lines that close it, a second client, a frame that never waits, and
//! when the build listens at all.

use std::io::{BufRead, BufReader};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use shuyi_platform::headless::HeadlessPlatform;
use shuyi_platform::link::{Request, Step, VERSION};
use shuyi_ui::toy::{self, Toy};

use super::*;

const TOKEN: &str = "0123456789abcdef0123456789abcdef";

/// A frame's length.
const DT: f32 = 1.0 / 60.0;

/// The toy game, running, with a listener on a free port.
struct Host {
    app: App<Toy>,
    platform: HeadlessPlatform,
    listener: Listener,
}

impl Host {
    fn new() -> Self {
        Self {
            app: App::running(toy::game()),
            platform: HeadlessPlatform::new(),
            listener: Listener::bind(0, TOKEN.to_owned(), "Toy".to_owned()).unwrap(),
        }
    }

    fn port(&self) -> u16 {
        self.listener.socket.local_addr().unwrap().port()
    }

    /// One frame as `run` does it, the picture a fake one.
    fn frame(&mut self) {
        let mut input = Vec::new();
        self.listener.before_frame(&self.app, &mut input);
        self.platform.push_input(&input);
        let _ = self.app.frame_after(&mut self.platform, DT);
        self.listener.after_frame(&self.app);
        if self.listener.wants_screenshot() {
            self.listener.screenshot(b"png", 2, 1);
        }
    }

    /// Runs frames until `client` has finished (at most ten seconds), and
    /// gives what it returned.
    fn serve<T>(&mut self, client: JoinHandle<T>) -> T {
        let until = Instant::now() + Duration::from_secs(10);
        while !client.is_finished() {
            assert!(Instant::now() < until, "the client didn't finish");
            self.frame();
            thread::sleep(Duration::from_millis(1));
        }
        client.join().unwrap()
    }
}

/// A connected client's two ends.
fn connect(port: u16) -> (BufReader<TcpStream>, TcpStream) {
    let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    (BufReader::new(stream.try_clone().unwrap()), stream)
}

fn send(stream: &mut TcpStream, request: &Request) {
    stream
        .write_all(format!("{}\n", request.to_line()).as_bytes())
        .unwrap();
}

fn answer(reader: &mut BufReader<TcpStream>) -> Answer {
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    Answer::from_line(&line).unwrap()
}

fn hello() -> Request {
    Request::Hello {
        token: TOKEN.to_owned(),
        link: VERSION,
    }
}

/// Whether the connection is closed with nothing more read: the end of
/// the stream, or a reset.
fn closed(reader: &mut BufReader<TcpStream>) -> bool {
    let mut rest = Vec::new();
    match reader.read_to_end(&mut rest) {
        Ok(_) => rest.is_empty(),
        Err(e) => matches!(
            e.kind(),
            ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted
        ),
    }
}

#[test]
fn a_client_says_hello_plays_and_reads_over_a_real_socket() {
    let mut host = Host::new();
    let port = host.port();
    let client = thread::spawn(move || {
        let (mut reader, mut stream) = connect(port);
        send(&mut stream, &hello());
        let hello = answer(&mut reader);
        let steps = vec![Step::Keys("f".to_owned())];
        send(&mut stream, &Request::Play { steps });
        send(&mut stream, &Request::Status);
        send(&mut stream, &Request::Frame);
        send(&mut stream, &Request::Screenshot);
        send(&mut stream, &Request::Log { since: 0 });
        let answers: Vec<Answer> = (0..5).map(|_| answer(&mut reader)).collect();
        let _ = stream.shutdown(Shutdown::Both);
        (hello, answers)
    });
    let (hello, answers) = host.serve(client);
    let game = "Toy".to_owned();
    assert_eq!(
        hello,
        Answer::Hello {
            link: VERSION,
            game
        }
    );
    let [played, status, frame, shot, log] = answers.as_slice() else {
        panic!("{answers:?}");
    };
    assert!(
        matches!(played, Answer::Played(p) if p.frames == 2),
        "{played:?}"
    );
    let Answer::Status(status) = status else {
        panic!("{status:?}");
    };
    assert_eq!(status.screens, ["home", "menu"]);
    assert_eq!(status.top_screen, "menu");
    let snapshot = host
        .app
        .game()
        .map(|g| g.buffer().to_snapshot(&g.ctx().palette));
    assert!(
        matches!(frame, Answer::Frame { frame, limit: None } if Some(frame) == snapshot.as_ref())
    );
    assert!(matches!(shot, Answer::Screenshot(s) if s.png == "cG5n" && s.width == 2));
    assert!(matches!(log, Answer::Log(_)), "{log:?}");
    // The client hung up: the link is free for the next.
    let until = Instant::now() + Duration::from_secs(10);
    while host.listener.client.is_some() {
        assert!(Instant::now() < until, "the client stayed attached");
        host.frame();
    }
}

#[test]
fn a_bad_first_line_closes_the_link_and_nothing_after_it_is_read() {
    let wrong = Request::Hello {
        token: "fedcba9876543210fedcba9876543210".to_owned(),
        link: VERSION,
    };
    let long = "x".repeat(MAX_LINE + 10);
    for first in [wrong.to_line(), "GET / HTTP/1.1".to_owned(), long] {
        let mut host = Host::new();
        let port = host.port();
        let client = thread::spawn(move || {
            let (mut reader, mut stream) = connect(port);
            let after = Request::Status.to_line();
            let _ = stream.write_all(format!("{first}\n{after}\n").as_bytes());
            closed(&mut reader)
        });
        assert!(host.serve(client), "an answer came");
        assert!(host.listener.client.is_none());
        // The next client is heard.
        let client = thread::spawn(move || {
            let (mut reader, mut stream) = connect(port);
            send(&mut stream, &hello());
            answer(&mut reader)
        });
        assert!(matches!(host.serve(client), Answer::Hello { .. }));
    }
}

#[test]
fn a_client_of_another_link_version_hears_the_builds_and_is_closed() {
    let mut host = Host::new();
    let port = host.port();
    let client = thread::spawn(move || {
        let (mut reader, mut stream) = connect(port);
        let other = Request::Hello {
            token: TOKEN.to_owned(),
            link: VERSION + 1,
        };
        send(&mut stream, &other);
        (answer(&mut reader), closed(&mut reader))
    });
    let (hello, closed) = host.serve(client);
    assert!(matches!(hello, Answer::Hello { link: VERSION, .. }));
    assert!(closed);
}

#[test]
fn a_second_client_while_one_is_attached_hears_busy_and_is_closed() {
    let mut host = Host::new();
    let port = host.port();
    // The first says hello and is answered, frames running meanwhile.
    let first = thread::spawn(move || {
        let (mut reader, mut stream) = connect(port);
        send(&mut stream, &hello());
        let hello = answer(&mut reader);
        (reader, stream, hello)
    });
    let (mut reader, mut stream, said) = host.serve(first);
    assert!(matches!(said, Answer::Hello { .. }), "{said:?}");
    let second = thread::spawn(move || {
        let (mut reader, mut stream) = connect(port);
        send(&mut stream, &hello());
        (answer(&mut reader), closed(&mut reader))
    });
    let (busy, closed) = host.serve(second);
    assert_eq!(busy, Answer::Busy);
    assert!(closed);
    // The first is still attached.
    let status = thread::spawn(move || {
        send(&mut stream, &Request::Status);
        answer(&mut reader)
    });
    assert!(matches!(host.serve(status), Answer::Status(_)));
}

#[test]
fn a_frame_never_waits_on_the_socket() {
    let mut host = Host::new();
    let quick = |host: &mut Host| {
        let started = Instant::now();
        for _ in 0..10 {
            host.frame();
        }
        assert!(
            started.elapsed() < Duration::from_millis(500),
            "{:?}",
            started.elapsed()
        );
    };
    // No client.
    quick(&mut host);
    // A client that says nothing, then half a line.
    let (_reader, mut stream) = connect(host.port());
    quick(&mut host);
    assert!(host.listener.client.is_some());
    stream.write_all(b"{\"op\":\"stat").unwrap();
    quick(&mut host);
    assert!(host.listener.client.is_some());
}

#[test]
fn the_build_listens_only_when_asked_with_debug_tools_on_a_port() {
    assert_eq!(decide(None, true), Decision::Off);
    assert_eq!(decide(None, false), Decision::Off);
    assert_eq!(decide(Some("0"), true), Decision::Listen(0));
    assert_eq!(decide(Some(" 7357 "), true), Decision::Listen(7357));
    assert_eq!(decide(Some("7357"), false), Decision::NoDebugTools);
    assert_eq!(
        decide(Some("yes"), true),
        Decision::BadPort("yes".to_owned())
    );
    assert_eq!(
        decide(Some("70000"), true),
        Decision::BadPort("70000".to_owned())
    );
}

#[test]
fn the_token_is_chosen_or_128_random_bits_in_hex() {
    let never = || -> Option<[u8; 16]> { panic!("asked for random bits") };
    assert_eq!(token(Some(TOKEN), never), Ok(TOKEN.to_owned()));
    assert_eq!(
        token(Some("sixteen-chars-ok"), never),
        Ok("sixteen-chars-ok".to_owned())
    );
    for short in ["", "fifteen-chars-x", "sixteen chars ok"] {
        let error = token(Some(short), never).unwrap_err();
        assert!(error.contains(TOKEN_VAR) && error.contains("16"), "{error}");
    }
    let bits = [
        0x00, 0x01, 0xab, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10, 0x7f,
    ];
    let random = token(None, || Some(bits)).unwrap();
    assert_eq!(random, "0001abff00000000000000000000107f");
    assert_eq!(random.len(), 32);
    assert!(token(None, || None).is_err());
    assert_eq!(hex(&[]), "");
    // The system's bits differ from call to call.
    assert_ne!(random_bits(), random_bits());
}
