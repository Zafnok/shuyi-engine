//! A test host of the live link (ADR-0047): the toy game running behind
//! the link's server on a loopback port, in a thread, as a backend's
//! listener runs it. `attach`'s tests, MCP's session tests and the
//! scripted MCP client attach to it.
//!
//! It measures its frames with a fake clock and reads a fake heap, as the
//! toy's headless session does, so a perf read over the link has records,
//! heap counts and snapshots.

use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{Ipv4Addr, Shutdown, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use shuyi_platform::App;
use shuyi_platform::headless::HeadlessPlatform;
use shuyi_platform::link::{self, Answer, Handled, Server};
use shuyi_ui::perf::PerfClock;
use shuyi_ui::toy;

/// The host's token.
pub const TOKEN: &str = "0123456789abcdef0123456789abcdef";

/// The game the host's hello names.
pub const GAME: &str = "Toy";

/// The toy game behind the link's server, in a thread: one client at a
/// time, a second told `busy`, frames run about every millisecond, and a
/// picture of 2 × 1 pixels whose PNG is `\x89PNG`. It stops when dropped.
pub struct LinkHost {
    port: u16,
    stop: Arc<AtomicBool>,
    /// Whether a client is attached.
    attached: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

/// A client attached to the host: its socket, server and what it sent.
type Client = (TcpStream, Server, Vec<u8>);

impl LinkHost {
    /// The host, listening on a free loopback port.
    pub fn start() -> Self {
        Self::running(1)
    }

    /// The host running `frames` frames at a time, about every
    /// millisecond: with more than a perf read's 600, a reader that reads
    /// between them misses some.
    pub fn running(frames: u32) -> Self {
        let socket = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .unwrap_or_else(|e| panic!("bind a loopback port: {e}"));
        let port = socket
            .local_addr()
            .unwrap_or_else(|e| panic!("its address: {e}"))
            .port();
        socket
            .set_nonblocking(true)
            .unwrap_or_else(|e| panic!("a non-blocking socket: {e}"));
        let stop = Arc::new(AtomicBool::new(false));
        let attached = Arc::new(AtomicBool::new(false));
        let (stopping, attaching) = (Arc::clone(&stop), Arc::clone(&attached));
        let thread = thread::spawn(move || serve(&socket, frames, &stopping, &attaching));
        Self {
            port,
            stop,
            attached,
            thread: Some(thread),
        }
    }

    /// Its port.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Its address, `127.0.0.1:<port>`.
    pub fn address(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    /// Whether a client is attached.
    pub fn attached(&self) -> bool {
        self.attached.load(Ordering::Relaxed)
    }

    /// Waits until the last client has gone (at most ten seconds), as the
    /// next `attach` of a person or an agent comes a while after.
    pub fn let_go(&self) {
        let until = Instant::now() + Duration::from_secs(10);
        while self.attached() {
            assert!(Instant::now() < until, "the client stayed attached");
            thread::sleep(Duration::from_millis(1));
        }
    }
}

impl Drop for LinkHost {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The host's loop, until `stop`: `frames` frames at a time.
fn serve(socket: &TcpListener, frames: u32, stop: &AtomicBool, attached: &AtomicBool) {
    let mut app = App::running(toy::game());
    let mut platform = HeadlessPlatform::new()
        .with_clock(PerfClock::fake())
        .with_fake_memory();
    let mut client: Option<Client> = None;
    let mut refused = Vec::new();
    while !stop.load(Ordering::Relaxed) {
        if let Ok((stream, _)) = socket.accept() {
            accept(stream, &mut client, &mut refused);
        }
        let mut out = String::new();
        let mut close = false;
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
                    Handled::Answer(answer) => push_line(&mut out, &answer),
                    Handled::Close(last) => {
                        if let Some(last) = last {
                            push_line(&mut out, &last);
                        }
                        close = true;
                        break;
                    }
                    _ => {}
                }
            }
        }
        for _ in 0..frames {
            let mut input = Vec::new();
            if let Some((_, server, _)) = &mut client {
                server.input(&app, &mut input);
            }
            platform.push_input(&input);
            let _ = app.frame_after(&mut platform, 1.0 / 60.0);
            if let Some((_, server, _)) = &mut client {
                for answer in server.after_frame(&app) {
                    push_line(&mut out, &answer);
                }
                if server.wants_screenshot() {
                    push_line(&mut out, &server.screenshot(b"\x89PNG", 2, 1));
                }
                if server.wants_snapshot() {
                    let snapshot = app.heap_snapshot(&mut platform);
                    push_line(&mut out, &server.snapshot(&app, snapshot));
                }
            }
        }
        if let Some((stream, _, _)) = &mut client
            && (stream.write_all(out.as_bytes()).is_err() || close)
        {
            client = None;
        }
        attached.store(client.is_some(), Ordering::Relaxed);
        thread::sleep(Duration::from_millis(1));
    }
}

/// Takes the connection `stream`: the client, or one told `busy` and kept
/// in `refused`.
fn accept(mut stream: TcpStream, client: &mut Option<Client>, refused: &mut Vec<TcpStream>) {
    // A client that hung up an instant ago makes way.
    if client
        .as_ref()
        .is_some_and(|(old, _, _)| match old.peek(&mut [0]) {
            Ok(n) => n == 0,
            Err(e) => !waiting(e.kind()),
        })
    {
        *client = None;
    }
    if client.is_some() {
        // Half closed and kept, so the client reads the line.
        let _ = stream.write_all(format!("{}\n", Answer::Busy.to_line()).as_bytes());
        let _ = stream.shutdown(Shutdown::Write);
        refused.push(stream);
    } else if stream.set_nonblocking(false).is_ok()
        && stream
            .set_read_timeout(Some(Duration::from_millis(1)))
            .is_ok()
    {
        let server = Server::new(TOKEN.to_owned(), GAME.to_owned());
        *client = Some((stream, server, Vec::new()));
    }
}

/// `line` and a line break, added to `out`.
fn push_line(out: &mut String, line: &str) {
    out.push_str(line);
    out.push('\n');
}

/// Whether a read failed only because nothing came in time.
fn waiting(kind: ErrorKind) -> bool {
    matches!(
        kind,
        ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
    )
}

/// A host that answers one hello with another link version: its address,
/// and its thread, which ends once it has answered.
pub fn other_version() -> (String, JoinHandle<()>) {
    let socket = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .unwrap_or_else(|e| panic!("bind a loopback port: {e}"));
    let address = socket
        .local_addr()
        .unwrap_or_else(|e| panic!("its address: {e}"))
        .to_string();
    let thread = thread::spawn(move || {
        let Ok((mut stream, _)) = socket.accept() else {
            return;
        };
        let mut hello = String::new();
        if let Ok(read) = stream.try_clone() {
            let _ = BufReader::new(read).read_line(&mut hello);
        }
        let answer = Answer::Hello {
            link: link::VERSION + 1,
            game: GAME.to_owned(),
        };
        let _ = stream.write_all(format!("{}\n", answer.to_line()).as_bytes());
    });
    (address, thread)
}
