//! The live link's listener (ADR-0047): with the `link` feature, in a
//! build with debug tools, `SHUYI_LINK=<port>` makes the game listen on
//! `127.0.0.1:<port>` (`0` takes a free port) for one client at a time.
//! The client's JSON lines go to a `shuyi_platform::link::Server`, which
//! answers them over the running `App`; the server's input reaches the
//! game after the keyboard's and the pads'.
//!
//! A frame never waits on the socket: it is non-blocking, read before the
//! frame and written after it. A line is at most
//! [`MAX_LINE`] bytes, and a client that leaves more than
//! [`MAX_UNSENT`] bytes of answers unread is dropped.
//!
//! Nothing here calls macroquad but [`start`]'s two log lines, so the
//! tests run it over the toy game with no window. The picture is read by
//! the loop ([`crate::readback`]).

use std::collections::VecDeque;
use std::io::{ErrorKind, Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};

use shuyi_platform::App;
use shuyi_platform::link::{Answer, Handled, MAX_LINE, MAX_UNSENT, Server};
use shuyi_ui::perf::HeapSnapshot;
use shuyi_ui::{GameData, RawInputEvent};

/// The variable that turns the link on: the port to listen on.
pub(crate) const PORT_VAR: &str = "SHUYI_LINK";
/// The variable that chooses the token, instead of a random one.
pub(crate) const TOKEN_VAR: &str = "SHUYI_LINK_TOKEN";
/// The shortest token [`TOKEN_VAR`] may give.
pub(crate) const MIN_TOKEN: usize = 16;

/// The most bytes read from the client in one frame.
const READ_PER_FRAME: usize = 1024 * 1024;

/// The most frames a connection told `busy` is kept, waiting for it to
/// hang up.
const REFUSED_FRAMES: u32 = 300;

/// What the build does about the link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Decision {
    /// No [`PORT_VAR`]: no link, and nothing said.
    Off,
    /// Listen on this port.
    Listen(u16),
    /// It was asked for, but the build has no debug tools.
    NoDebugTools,
    /// It was asked for with this, which is no port.
    BadPort(String),
}

/// What to do, given [`PORT_VAR`]'s value `port` and whether debug tools
/// are compiled in (`shuyi_ui::DEBUG_TOOLS`).
pub(crate) fn decide(port: Option<&str>, debug_tools: bool) -> Decision {
    let Some(port) = port else {
        return Decision::Off;
    };
    if !debug_tools {
        return Decision::NoDebugTools;
    }
    port.trim()
        .parse()
        .map_or_else(|_| Decision::BadPort(port.to_owned()), Decision::Listen)
}

/// The token: [`TOKEN_VAR`]'s value `chosen`, if it is
/// [`MIN_TOKEN`] characters or more with no space, else `random`'s 128
/// bits as hex.
pub(crate) fn token(
    chosen: Option<&str>,
    random: impl FnOnce() -> Option<[u8; 16]>,
) -> Result<String, String> {
    match chosen {
        Some(token)
            if token.chars().count() >= MIN_TOKEN && !token.contains(char::is_whitespace) =>
        {
            Ok(token.to_owned())
        }
        Some(_) => Err(format!(
            "{TOKEN_VAR} must be {MIN_TOKEN} characters or more, with no space"
        )),
        None => random()
            .map(|bytes| hex(&bytes))
            .ok_or_else(|| "the system gave no random bytes for the token".to_owned()),
    }
}

/// `bytes` as lowercase hex.
pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        // Writing to a `String` can't fail.
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// 128 random bits from the system.
fn random_bits() -> Option<[u8; 16]> {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).ok()?;
    Some(bytes)
}

/// The listener `SHUYI_LINK` asks for, if the build may listen, for the
/// game titled `game`: it says on what and with which token, or why it
/// doesn't listen.
pub(crate) fn start(game: &str) -> Option<Listener> {
    let port = std::env::var(PORT_VAR).ok();
    match decide(port.as_deref(), shuyi_ui::DEBUG_TOOLS) {
        Decision::Off => None,
        Decision::NoDebugTools => {
            macroquad::logging::warn!(
                "shuyi link: {} is set, but this build has no debug tools (a release build \
                 without the `debug-tools` feature), so it doesn't listen",
                PORT_VAR
            );
            None
        }
        Decision::BadPort(port) => {
            macroquad::logging::warn!(
                "shuyi link: {}={} is no port (0 to 65535; 0 takes a free one), so it doesn't \
                 listen",
                PORT_VAR,
                port
            );
            None
        }
        Decision::Listen(port) => {
            let chosen = std::env::var(TOKEN_VAR).ok();
            let listening = token(chosen.as_deref(), random_bits).and_then(|token| {
                Listener::bind(port, token, game.to_owned())
                    .map_err(|e| format!("can't listen on 127.0.0.1:{port}: {e}"))
            });
            match listening {
                Ok(listener) => {
                    macroquad::logging::info!(
                        "shuyi link: listening on {}, token {}",
                        listener.address(),
                        listener.token
                    );
                    Some(listener)
                }
                Err(e) => {
                    macroquad::logging::warn!("shuyi link: {}, so it doesn't listen", e);
                    None
                }
            }
        }
    }
}

/// The socket a client is attached on, and its server.
struct Client {
    stream: TcpStream,
    server: Server,
    /// What was read and isn't a whole line yet.
    read: Vec<u8>,
    /// The answers not written yet.
    unsent: VecDeque<u8>,
    /// Whether it is closed once `unsent` is written, reading nothing
    /// more.
    closing: bool,
}

impl Client {
    fn send(&mut self, line: &str) {
        self.unsent.extend(line.as_bytes());
        self.unsent.push_back(b'\n');
    }
}

/// The build's end of the link: a socket on loopback, and the client
/// attached, if one is.
pub(crate) struct Listener {
    socket: TcpListener,
    token: String,
    game: String,
    client: Option<Client>,
    /// The connections told `busy`, half closed, and the frames each is
    /// kept: what they send is read until they hang up, so closing them
    /// never resets the connection before they read the answer.
    refused: Vec<(TcpStream, u32)>,
}

impl Listener {
    /// Listens on `127.0.0.1:port` for clients with `token`, for the game
    /// titled `game`.
    pub(crate) fn bind(port: u16, token: String, game: String) -> std::io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            socket: listener,
            token,
            game,
            client: None,
            refused: Vec::new(),
        })
    }

    /// Where it listens.
    pub(crate) fn address(&self) -> String {
        self.socket
            .local_addr()
            .map_or_else(|_| "127.0.0.1".to_owned(), |a: SocketAddr| a.to_string())
    }

    /// Before `app`'s frame: takes a new client (another one while one is
    /// attached hears `busy` and is closed), reads its lines and answers
    /// what can be answered now, and puts the server's input for the frame
    /// in `input`.
    pub(crate) fn before_frame<G: GameData>(
        &mut self,
        app: &App<G>,
        input: &mut Vec<RawInputEvent>,
    ) {
        // A client that hung up is let go before a new one is taken.
        self.read(app);
        self.accept();
        self.refused.retain_mut(|(stream, left)| {
            *left = left.saturating_sub(1);
            *left > 0 && !drained(stream)
        });
        self.read(app);
        if let Some(client) = &mut self.client {
            client.server.input(app, input);
        }
        self.flush();
    }

    /// After `app`'s frame: sends the answers now due.
    pub(crate) fn after_frame<G: GameData>(&mut self, app: &App<G>) {
        if let Some(client) = &mut self.client {
            for line in client.server.after_frame(app) {
                client.send(&line);
            }
        }
        self.flush();
    }

    /// Whether the client waits for a picture of this frame.
    pub(crate) fn wants_screenshot(&self) -> bool {
        self.client
            .as_ref()
            .is_some_and(|client| client.server.wants_screenshot())
    }

    /// Answers the picture: `png`, `width` × `height` pixels.
    pub(crate) fn screenshot(&mut self, png: &[u8], width: u32, height: u32) {
        if let Some(client) = &mut self.client {
            let line = client.server.screenshot(png, width, height);
            client.send(&line);
        }
        self.flush();
    }

    /// Whether the client waits for a heap snapshot after this frame.
    pub(crate) fn wants_snapshot(&self) -> bool {
        self.client
            .as_ref()
            .is_some_and(|client| client.server.wants_snapshot())
    }

    /// Answers the frames' records asked for with `snapshot`, taken after
    /// `app`'s frame.
    pub(crate) fn snapshot<G: GameData>(&mut self, app: &App<G>, snapshot: Option<HeapSnapshot>) {
        if let Some(client) = &mut self.client {
            let line = client.server.snapshot(app, snapshot);
            client.send(&line);
        }
        self.flush();
    }

    /// Answers a picture that couldn't be taken with `error`.
    pub(crate) fn no_screenshot(&mut self, error: String) {
        if let Some(client) = &mut self.client {
            let _ = client.server.screenshot(&[], 0, 0);
            client.send(&Answer::Error { error }.to_line());
        }
        self.flush();
    }

    fn accept(&mut self) {
        loop {
            let accepted = self.socket.accept();
            // A client that hung up an instant ago makes way.
            if accepted.is_ok()
                && self
                    .client
                    .as_ref()
                    .is_some_and(|client| hung_up(&client.stream))
            {
                self.client = None;
            }
            match accepted {
                Ok((stream, _)) if self.client.is_some() => {
                    if let Some(stream) = refuse(stream) {
                        self.refused.push((stream, REFUSED_FRAMES));
                    }
                }
                Ok((stream, _)) => {
                    if stream.set_nonblocking(true).is_err() {
                        continue;
                    }
                    let _ = stream.set_nodelay(true);
                    self.client = Some(Client {
                        stream,
                        server: Server::new(self.token.clone(), self.game.clone()),
                        read: Vec::new(),
                        unsent: VecDeque::new(),
                        closing: false,
                    });
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(_) => return,
            }
        }
    }

    /// Reads what the client sent, and hands each whole line to its
    /// server. A line longer than [`MAX_LINE`] closes it, and so does
    /// one the server closes on.
    fn read<G: GameData>(&mut self, app: &App<G>) {
        let Some(client) = &mut self.client else {
            return;
        };
        let mut buffer = [0; 16 * 1024];
        let mut taken = 0;
        while !client.closing && taken < READ_PER_FRAME {
            match client.stream.read(&mut buffer) {
                Ok(0) => {
                    self.client = None;
                    return;
                }
                Ok(n) => {
                    taken += n;
                    client.read.extend_from_slice(&buffer[..n]);
                    lines(client, app);
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) if e.kind() == ErrorKind::WouldBlock => return,
                Err(_) => {
                    self.client = None;
                    return;
                }
            }
        }
    }

    /// Writes what it can of the answers; drops a client that has left
    /// too much unread, and closes one that is closing once it is
    /// written.
    fn flush(&mut self) {
        let Some(client) = &mut self.client else {
            return;
        };
        while !client.unsent.is_empty() {
            let (first, _) = client.unsent.as_slices();
            match client.stream.write(first) {
                Ok(0) => {
                    self.client = None;
                    return;
                }
                Ok(n) => drop(client.unsent.drain(..n)),
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(_) => {
                    self.client = None;
                    return;
                }
            }
        }
        if client.unsent.len() > MAX_UNSENT || (client.closing && client.unsent.is_empty()) {
            let _ = client.stream.shutdown(Shutdown::Both);
            self.client = None;
        }
    }
}

/// Hands each whole line `client` has read to its server, and queues the
/// answers it gives now.
fn lines<G: GameData>(client: &mut Client, app: &App<G>) {
    while let Some(end) = client.read.iter().position(|&b| b == b'\n') {
        let bytes: Vec<u8> = client.read.drain(..=end).collect();
        let line = String::from_utf8_lossy(&bytes);
        let line = line.trim_end_matches(['\n', '\r']);
        match client.server.handle(line, app) {
            Handled::Answer(answer) => client.send(&answer),
            Handled::Close(last) => {
                if let Some(last) = last {
                    client.send(&last);
                }
                client.closing = true;
                client.read.clear();
                return;
            }
            _ => {}
        }
    }
    if client.read.len() > MAX_LINE {
        client.closing = true;
        client.read.clear();
    }
}

/// Tells a second client the link is busy, and closes the link's side:
/// the connection, to keep until the client hangs up ([`drained`]).
fn refuse(mut stream: TcpStream) -> Option<TcpStream> {
    stream.set_nonblocking(true).ok()?;
    let mut line = Answer::Busy.to_line();
    line.push('\n');
    // A short line into an empty socket: it fits.
    stream.write_all(line.as_bytes()).ok()?;
    stream.shutdown(Shutdown::Write).ok()?;
    Some(stream)
}

/// Whether the client on `stream` has hung up, with nothing unread.
fn hung_up(stream: &TcpStream) -> bool {
    match stream.peek(&mut [0]) {
        Ok(0) => true,
        Ok(_) => false,
        Err(e) => !matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted),
    }
}

/// Reads and drops what a refused client sent: whether it has hung up
/// (or the connection broke).
fn drained(stream: &mut TcpStream) -> bool {
    let mut buffer = [0; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => return true,
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return e.kind() != ErrorKind::WouldBlock,
        }
    }
}

#[cfg(test)]
mod tests;
