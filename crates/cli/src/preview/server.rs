//! The preview's server: its routes, its sessions and their limits, and
//! the workers their games run in.
//!
//! A request is answered on a thread of its own, at most
//! [`Limits::connections`] at once. A session's actions are answered one
//! at a time: a second while one runs is refused (`409`). Each limit of a
//! session is checked before its next action, but a worker's memory,
//! which is read after each answer, and a call's deadline, past which the
//! worker is killed. A thread ends the sessions idle or old past their
//! limits.

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, TryLockError};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;

use super::Limits;
use super::http::{self, Answer, Request};
use super::worker::{Act, REPLY, Reply};

/// How long a connection may take to send its request.
const REQUEST_TIME: Duration = Duration::from_secs(10);

/// How long one read or write of a connection may wait.
const SOCKET_WAIT: Duration = Duration::from_secs(5);

/// The longest reply line a worker may send, in bytes.
const MAX_REPLY: u64 = 32 << 20;

/// What a worker that stopped answers.
const STOPPED: &str = "the game's worker stopped";

/// How a session's worker starts.
pub(crate) type Launch = Box<dyn Fn() -> io::Result<Worker> + Send + Sync>;

/// A session's game in a worker: a process of its own, or a thread in a
/// test.
pub(crate) struct Worker {
    input: Option<Box<dyn Write + Send>>,
    replies: Receiver<Vec<u8>>,
    child: Option<Child>,
}

impl Worker {
    /// `program` started with `args` in `dir`, its stdin and stdout piped
    /// to the server, its stderr the server's.
    pub(crate) fn process(program: &Path, args: &[String], dir: &Path) -> io::Result<Self> {
        let mut child = Command::new(program)
            .args(args)
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let (Some(input), Some(output)) = (child.stdin.take(), child.stdout.take()) else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other("the worker started without its pipes"));
        };
        Ok(Self::over(Box::new(input), output, Some(child)))
    }

    /// A worker whose requests go to `input` and whose replies come from
    /// `output`: the process `child`, if it is one.
    pub(crate) fn over(
        input: Box<dyn Write + Send>,
        output: impl Read + Send + 'static,
        child: Option<Child>,
    ) -> Self {
        let (sender, replies) = channel();
        thread::spawn(move || read_replies(output, &sender));
        Self {
            input: Some(input),
            replies,
            child,
        }
    }

    /// The reply to `action`, or with none to its start, within `deadline`;
    /// or why the game can't go on. Past the deadline the worker is ended.
    fn call(&mut self, action: Option<&str>, deadline: Duration) -> Result<Reply, String> {
        if let Some(action) = action {
            let act = Act {
                action: action.to_owned(),
            };
            let line = serde_json::to_string(&act).map_err(|e| e.to_string())?;
            let sent = self
                .input
                .as_mut()
                .map(|input| writeln!(input, "{line}").and_then(|()| input.flush()));
            if !matches!(sent, Some(Ok(()))) {
                return Err(STOPPED.to_owned());
            }
        }
        match self.replies.recv_timeout(deadline) {
            Ok(line) => serde_json::from_slice(&line)
                .map_err(|e| format!("the game's worker sent a broken reply ({e})")),
            Err(RecvTimeoutError::Timeout) => {
                self.stop();
                Err(format!(
                    "the game took longer than {} seconds",
                    deadline.as_secs()
                ))
            }
            Err(RecvTimeoutError::Disconnected) => Err(STOPPED.to_owned()),
        }
    }

    /// The memory it holds now, in bytes, where the system says.
    fn resident(&self) -> Option<u64> {
        resident(self.child.as_ref()?.id())
    }

    /// Ends it: its stdin closed, its process killed.
    fn stop(&mut self) {
        self.input = None;
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Sends each reply line `output` writes, after its mark, to `sender`,
/// until it closes: a line without the mark is the game's own printing.
fn read_replies(output: impl Read, sender: &Sender<Vec<u8>>) {
    let mut output = BufReader::new(output);
    loop {
        let mut line = Vec::new();
        match (&mut output).take(MAX_REPLY).read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        if let Some(reply) = line.strip_prefix(REPLY.as_bytes())
            && sender.send(reply.to_vec()).is_err()
        {
            return;
        }
    }
}

/// The memory process `pid` holds, in bytes: its resident set, as Linux's
/// `/proc` says; `None` on a system that doesn't say it there.
pub(crate) fn resident(pid: u32) -> Option<u64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let kb = status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))?;
    let kb: u64 = kb.trim().trim_end_matches("kB").trim().parse().ok()?;
    Some(kb.saturating_mul(1_024))
}

/// A rate, kept as a bucket of tokens: `per` a window, and at most `per`
/// at once. Its level counts tokens times the window's milliseconds.
#[derive(Debug, Clone)]
struct Bucket {
    per: u64,
    window: u64,
    level: u64,
    at: Instant,
}

impl Bucket {
    /// A full bucket of `per` tokens a `window`.
    fn new(per: u32, window: Duration, now: Instant) -> Self {
        let per = u64::from(per.max(1));
        let window = u64::try_from(window.as_millis()).unwrap_or(u64::MAX).max(1);
        Self {
            per,
            window,
            level: per.saturating_mul(window),
            at: now,
        }
    }

    /// Takes a token at `now`, or says how long until there is one.
    fn take(&mut self, now: Instant) -> Result<(), Duration> {
        let ms = now.saturating_duration_since(self.at).as_millis();
        let ms = u64::try_from(ms).unwrap_or(u64::MAX);
        self.at = now;
        let most = self.per.saturating_mul(self.window);
        self.level = self
            .level
            .saturating_add(ms.saturating_mul(self.per))
            .min(most);
        if self.level >= self.window {
            self.level -= self.window;
            return Ok(());
        }
        Err(Duration::from_millis(
            (self.window - self.level).div_ceil(self.per),
        ))
    }

    /// Whether nothing has been taken for a window: it is full again.
    fn full(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.at).as_millis() >= u128::from(self.window)
    }
}

/// A session.
struct Live {
    worker: Worker,
    born: Instant,
    last: Instant,
    actions: u32,
    busy: Duration,
    sent: u64,
    peak: Option<u64>,
    rate: Bucket,
    allowed: Vec<String>,
}

impl Live {
    /// What it has spent of one of `limits`, if it has: why it ends.
    fn spent(&self, limits: &Limits) -> Option<String> {
        if self.actions >= limits.actions {
            return Some(format!(
                "the session pressed its {} actions",
                limits.actions
            ));
        }
        if self.busy >= limits.cpu {
            let seconds = limits.cpu.as_secs();
            return Some(format!(
                "the session used its {seconds} seconds of the game's time"
            ));
        }
        (self.sent >= limits.egress)
            .then(|| format!("the session's answers sent their {}", size(limits.egress)))
    }

    /// The answer of session `id` to its worker's `reply`, and whether the
    /// session goes on.
    fn answered(
        &mut self,
        id: &str,
        reply: Result<Reply, String>,
        limits: &Limits,
    ) -> (Answer, bool) {
        let reply = match reply {
            Ok(reply) => reply,
            Err(e) => return (Answer::error(500, format!("the game stopped: {e}")), false),
        };
        if let Some(refused) = reply.refused {
            return (Answer::error(400, refused), true);
        }
        let Some(shown) = reply.frame else {
            let failed = reply
                .failed
                .unwrap_or_else(|| "it sent no frame".to_owned());
            return (
                Answer::error(500, format!("the game failed: {failed}")),
                false,
            );
        };
        if let Some(resident) = self.worker.resident() {
            self.peak = self.peak.max(Some(resident));
            if resident > limits.memory {
                let error = format!("the game held more than its {}", size(limits.memory));
                return (Answer::error(503, error), false);
            }
        }
        let answer = Answer::json(
            200,
            &Frame {
                session: id,
                png: &shown.png,
                text: &shown.text,
                actions: &shown.actions,
                ended: shown.ended,
            },
        );
        self.sent += u64::try_from(answer.to_bytes().len()).unwrap_or(u64::MAX);
        self.allowed = shown.actions;
        (answer, !shown.ended)
    }

    /// What it did, for the server's log.
    fn summary(&self) -> String {
        let peak = self.peak.map_or_else(
            || "memory unread".to_owned(),
            |b| format!("{} MB held at most", b >> 20),
        );
        format!(
            "{} actions, {} ms of the game's time, {} KB sent, {peak}",
            self.actions,
            self.busy.as_millis(),
            self.sent >> 10
        )
    }
}

/// `bytes` for people: whole megabytes in MB, else kilobytes.
fn size(bytes: u64) -> String {
    if bytes.is_multiple_of(1 << 20) {
        format!("{} MB", bytes >> 20)
    } else {
        format!("{} KB", bytes >> 10)
    }
}

/// A frame as a visitor's answer: its session's id and what it shows.
#[derive(Serialize)]
struct Frame<'a> {
    session: &'a str,
    png: &'a str,
    text: &'a str,
    actions: &'a [String],
    ended: bool,
}

/// A session in the table: its address, kept outside its lock.
struct Entry {
    address: IpAddr,
    live: Arc<Mutex<Live>>,
}

/// The sessions, the addresses of those starting, and each address's
/// starts.
#[derive(Default)]
struct Table {
    live: HashMap<String, Entry>,
    starting: Vec<IpAddr>,
    starts: HashMap<IpAddr, Bucket>,
}

/// `mutex` locked, even after a panic while it was held.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A new session's id: 128 bits from the system's random source, in hex.
fn new_id() -> Result<String, String> {
    match (getrandom::u64(), getrandom::u64()) {
        (Ok(a), Ok(b)) => Ok(format!("{a:016x}{b:016x}")),
        _ => Err("the system's random source failed".to_owned()),
    }
}

/// The session id the rest of a path after `/session` names: `/` and 32
/// lowercase hex digits.
fn id_of(rest: &str) -> Option<&str> {
    let id = rest.strip_prefix('/')?;
    let hex = |b: u8| b.is_ascii_digit() || (b'a'..=b'f').contains(&b);
    (id.len() == 32 && id.bytes().all(hex)).then_some(id)
}

/// The preview's server: its limits, how a session's worker starts, and
/// its sessions.
pub(crate) struct Server {
    limits: Limits,
    launch: Launch,
    table: Mutex<Table>,
    connections: AtomicUsize,
}

impl Server {
    /// A server of no sessions yet, which keeps `limits` and starts each
    /// session's worker with `launch`.
    pub(crate) fn new(limits: Limits, launch: Launch) -> Self {
        Self {
            limits,
            launch,
            table: Mutex::new(Table::default()),
            connections: AtomicUsize::new(0),
        }
    }

    /// The answer to `request`, from `address`.
    pub(crate) fn answer(&self, request: &Request, address: IpAddr) -> Answer {
        let nothing = || Answer::error(404, "nothing is served here");
        let Some(rest) = request.path.strip_prefix("/session") else {
            return nothing();
        };
        match (request.method.as_str(), rest, id_of(rest)) {
            ("POST", "", _) => self.start(address),
            ("POST", _, Some(id)) => self.act(id, &request.body),
            ("DELETE", _, Some(id)) => self.end(id),
            (_, "", _) | (_, _, Some(_)) => Answer::error(
                405,
                "a session is started and played with POST, and ended with DELETE",
            ),
            _ => nothing(),
        }
    }

    /// A new session for `address`, and its first frame.
    fn start(&self, address: IpAddr) -> Answer {
        let now = Instant::now();
        {
            let limits = &self.limits;
            let mut table = lock(&self.table);
            let starts = table
                .starts
                .entry(address)
                .or_insert_with(|| Bucket::new(limits.starts, Duration::from_secs(60), now));
            if let Err(wait) = starts.take(now) {
                let error = format!("an address may start {} sessions a minute", limits.starts);
                return Answer::error(429, error).retry_after(wait.as_secs());
            }
            if table.live.len() + table.starting.len() >= limits.sessions {
                return Answer::error(503, "every session is taken: try again later")
                    .retry_after(30);
            }
            let starting = table.starting.iter().filter(|a| **a == address).count();
            let live = table.live.values().filter(|e| e.address == address).count();
            if starting + live >= limits.per_address {
                let error = format!(
                    "an address may have {} sessions at once: end one first",
                    limits.per_address
                );
                return Answer::error(429, error);
            }
            table.starting.push(address);
        }
        let opened = self.open();
        let mut table = lock(&self.table);
        if let Some(at) = table.starting.iter().position(|a| *a == address) {
            table.starting.swap_remove(at);
        }
        match opened {
            Ok((id, live, answer)) => {
                let live = Arc::new(Mutex::new(live));
                table.live.insert(id, Entry { address, live });
                eprintln!("preview: a session started ({} open)", table.live.len());
                answer
            }
            Err(answer) => answer,
        }
    }

    /// A new session's id, the session and its first frame; or the answer
    /// that says why it didn't start.
    fn open(&self) -> Result<(String, Live, Answer), Answer> {
        let id = new_id().map_err(|e| Answer::error(500, e))?;
        let born = Instant::now();
        let mut worker = (self.launch)()
            .map_err(|e| Answer::error(500, format!("the game's worker didn't start: {e}")))?;
        let reply = worker.call(None, self.limits.deadline);
        let mut live = Live {
            worker,
            born,
            last: Instant::now(),
            actions: 0,
            busy: born.elapsed(),
            sent: 0,
            peak: None,
            rate: Bucket::new(self.limits.rate, Duration::from_secs(1), born),
            allowed: Vec::new(),
        };
        match live.answered(&id, reply, &self.limits) {
            (answer, true) => Ok((id, live, answer)),
            (answer, false) => Err(answer),
        }
    }

    /// Action `body` pressed in session `id`, and the frame after it.
    fn act(&self, id: &str, body: &[u8]) -> Answer {
        let Ok(act) = serde_json::from_slice::<Act>(body) else {
            return Answer::error(400, "an action's body is {\"action\": \"<name>\"}");
        };
        let Some(live) = lock(&self.table).live.get(id).map(|e| Arc::clone(&e.live)) else {
            return gone();
        };
        let mut live = match live.try_lock() {
            Ok(live) => live,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => {
                return Answer::error(409, "the session is busy with an action").retry_after(1);
            }
        };
        if let Some(why) = live.spent(&self.limits) {
            drop(live);
            self.remove(id, &why);
            return Answer::error(429, why);
        }
        if let Err(wait) = live.rate.take(Instant::now()) {
            let error = format!(
                "a session may press {} actions each second",
                self.limits.rate
            );
            return Answer::error(429, error).retry_after(wait.as_secs());
        }
        if !live.allowed.contains(&act.action) {
            let allowed = live.allowed.join(", ");
            return Answer::error(
                400,
                format!("no action `{}`: a visitor's are {allowed}", act.action),
            );
        }
        live.actions += 1;
        let began = Instant::now();
        let reply = live.worker.call(Some(&act.action), self.limits.deadline);
        live.busy += began.elapsed();
        live.last = Instant::now();
        let (answer, goes_on) = live.answered(id, reply, &self.limits);
        drop(live);
        if !goes_on {
            self.remove(id, "the game ended, or failed");
        }
        answer
    }

    /// Session `id` ended by its visitor.
    fn end(&self, id: &str) -> Answer {
        if self.remove(id, "ended by its visitor") {
            Answer {
                status: 204,
                headers: Vec::new(),
                body: Vec::new(),
            }
        } else {
            gone()
        }
    }

    /// Ends session `id`, which ends its worker, and logs `why` and what
    /// it did: whether there was one.
    fn remove(&self, id: &str, why: &str) -> bool {
        let Some(entry) = lock(&self.table).live.remove(id) else {
            return false;
        };
        let summary = lock(&entry.live).summary();
        eprintln!("preview: a session ended ({why}): {summary}");
        true
    }

    /// Ends the sessions idle or old past their limits, and forgets the
    /// starts' buckets full again.
    fn reap(&self) {
        let now = Instant::now();
        let ended: Vec<(String, &str)> = {
            let mut table = lock(&self.table);
            table.starts.retain(|_, bucket| !bucket.full(now));
            let limits = &self.limits;
            let past = |live: &Live| {
                if now.saturating_duration_since(live.last) >= limits.idle {
                    Some("idle")
                } else if now.saturating_duration_since(live.born) >= limits.lifetime {
                    Some("its time was up")
                } else {
                    None
                }
            };
            table
                .live
                .iter()
                .filter_map(|(id, entry)| {
                    // A session busy with an action isn't idle.
                    let live = entry.live.try_lock().ok()?;
                    past(&live).map(|why| (id.clone(), why))
                })
                .collect()
        };
        for (id, why) in ended {
            self.remove(&id, why);
        }
    }

    /// Reads one request from `stream` and writes its answer.
    fn handle(&self, mut stream: TcpStream) {
        let _ = stream.set_read_timeout(Some(SOCKET_WAIT));
        let _ = stream.set_write_timeout(Some(SOCKET_WAIT));
        let address = stream
            .peer_addr()
            .map_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED), |a| a.ip());
        let answer = match http::read(&mut stream, Instant::now() + REQUEST_TIME) {
            Ok(request) => self.answer(&request, address),
            Err(refused) => refused,
        };
        let _ = answer.write_to(&mut stream);
    }
}

/// The answer to a session that isn't there.
fn gone() -> Answer {
    Answer::error(404, "no such session: it ended, or never was")
}

/// Serves `listener`'s connections with `server`, each on a thread of its
/// own, and ends the sessions past their time; never returns.
pub(crate) fn serve(listener: &TcpListener, server: &Arc<Server>) {
    let limits = &server.limits;
    let tick = (limits.idle.min(limits.lifetime) / 4)
        .clamp(Duration::from_millis(20), Duration::from_secs(1));
    let reaper = Arc::downgrade(server);
    thread::spawn(move || {
        while let Some(server) = reaper.upgrade() {
            server.reap();
            drop(server);
            thread::sleep(tick);
        }
    });
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
            continue;
        };
        let server = Arc::clone(server);
        if server.connections.fetch_add(1, Ordering::SeqCst) >= server.limits.connections {
            server.connections.fetch_sub(1, Ordering::SeqCst);
            let _ = stream.set_write_timeout(Some(SOCKET_WAIT));
            let busy = Answer::error(503, "too many connections: try again later");
            let _ = busy.retry_after(5).write_to(&mut stream);
            continue;
        }
        thread::spawn(move || {
            server.handle(stream);
            server.connections.fetch_sub(1, Ordering::SeqCst);
        });
    }
}

#[cfg(test)]
mod tests;
