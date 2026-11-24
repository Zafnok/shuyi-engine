//! The preview's server: its routes, its sessions and their limits, and
//! the workers their games run in.
//!
//! A request is answered on a thread of its own, at most
//! [`Limits::connections`] at once. A session's actions are answered one
//! at a time: a second while one runs is refused (`409`). A session's
//! actions, its game's time and the bytes it sent are checked before its
//! next action, its rate as it presses, and its worker's memory after
//! each answer. A worker that doesn't answer a start or an action within
//! [`Limits::deadline`] is killed, and its session ended. A thread ends
//! the sessions idle or old past their limits ([`Server::reap`]).

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, TryLockError};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

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

/// The window of an address's starts.
const MINUTE: Duration = Duration::from_secs(60);

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
                Err(format!("the game took longer than {deadline:?}"))
            }
            Err(RecvTimeoutError::Disconnected) => Err(STOPPED.to_owned()),
        }
    }

    /// The memory its process holds now, in bytes, where the system says.
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
/// `/proc/<pid>/status` says it (`VmRSS`); `None` on a system that doesn't
/// say it there.
pub(crate) fn resident(pid: u32) -> Option<u64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    rss_of(&status)
}

/// The resident set a `/proc/<pid>/status` says, in bytes.
fn rss_of(status: &str) -> Option<u64> {
    let kb = status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))?;
    let kb: u64 = kb.trim().strip_suffix("kB")?.trim().parse().ok()?;
    Some(kb.saturating_mul(1_024))
}

/// A rate, kept as a bucket of tokens: `per` a window, and at most `per`
/// at once. Its level counts tokens times the window's milliseconds.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Bucket {
    per: u64,
    window: u64,
    level: u64,
    at: Instant,
}

impl Bucket {
    /// A full bucket of `per` tokens a `window`, at `now`.
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
        self.at = self.at.max(now);
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

    /// Whether nothing has been taken for a window at `now`: it is full
    /// again.
    fn full(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.at).as_millis() >= u128::from(self.window)
    }
}

/// `wait` in whole seconds, rounded up: a `Retry-After` that is never
/// early.
fn seconds(wait: Duration) -> u64 {
    let ms = u64::try_from(wait.as_millis()).unwrap_or(u64::MAX);
    ms.div_ceil(1_000)
}

/// A session: its game's worker, the actions its last frame allows, and
/// what it has spent.
struct Live {
    worker: Worker,
    allowed: Vec<String>,
    /// When it started.
    born: Instant,
    /// When its last action, or its start, was answered.
    last: Instant,
    /// The actions it pressed.
    actions: u32,
    /// Its game's time: its calls' time summed.
    busy: Duration,
    /// The bytes its answers sent.
    sent: u64,
    /// The most memory its worker held, where the system says.
    peak: Option<u64>,
    /// Its actions' rate.
    rate: Bucket,
}

impl Live {
    /// What it has used up of `limits`, if it has: why it ends.
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

    /// The answer of session `id` to its worker's `reply`, and why the
    /// session ends, if it does: a worker past `limits`' memory ends it.
    fn answered(
        &mut self,
        id: &str,
        reply: Result<Reply, String>,
        limits: &Limits,
    ) -> (Answer, Option<String>) {
        let ending = |status: u16, error: String| (Answer::error(status, &*error), Some(error));
        let reply = match reply {
            Ok(reply) => reply,
            Err(e) => return ending(500, format!("the game stopped: {e}")),
        };
        if let Some(refused) = reply.refused {
            return (Answer::error(400, refused), None);
        }
        let Some(shown) = reply.frame else {
            let failed = reply
                .failed
                .unwrap_or_else(|| "it sent no frame".to_owned());
            return ending(500, format!("the game failed: {failed}"));
        };
        if let Some(resident) = self.worker.resident() {
            self.peak = self.peak.max(Some(resident));
            if past(resident, limits.memory) {
                return ending(
                    503,
                    format!("the game held more than its {}", size(limits.memory)),
                );
            }
        }
        // A frame as a visitor's answer: its session's id and what it shows.
        let frame = json!({
            "session": id, "png": shown.png, "text": shown.text,
            "actions": shown.actions, "ended": shown.ended,
        });
        let answer = Answer::json(200, &frame);
        let bytes = u64::try_from(answer.to_bytes().len()).unwrap_or(u64::MAX);
        self.sent = self.sent.saturating_add(bytes);
        self.allowed = shown.actions;
        let ended = shown.ended.then(|| "the game ended".to_owned());
        (answer, ended)
    }

    /// What it did, for the server's log.
    fn summary(&self) -> String {
        let peak = self.peak.map_or_else(
            || "its memory unread".to_owned(),
            |bytes| format!("{} held at most", size(bytes)),
        );
        format!(
            "{} actions, {} ms of the game's time, {} sent, {peak}",
            self.actions,
            self.busy.as_millis(),
            size(self.sent),
        )
    }
}

/// Whether `resident` bytes are past a bound of `most`: more than it.
fn past(resident: u64, most: u64) -> bool {
    resident > most
}

/// `bytes` for people: whole megabytes in MB, else kilobytes, rounded
/// down.
fn size(bytes: u64) -> String {
    if bytes >= 1 << 20 && bytes.is_multiple_of(1 << 20) {
        format!("{} MB", bytes >> 20)
    } else {
        format!("{} KB", bytes >> 10)
    }
}

/// Who a request is from, for its address's limits: the address, an IPv4
/// one written in IPv6 as itself, and an IPv6 one as its /64, the least a
/// host is given.
fn visitor(address: IpAddr) -> IpAddr {
    match address.to_canonical() {
        IpAddr::V6(v6) => {
            let net = u128::from(v6) & !u128::from(u64::MAX);
            IpAddr::V6(Ipv6Addr::from(net))
        }
        v4 @ IpAddr::V4(_) => v4,
    }
}

/// A session in the table, and its visitor's address kept outside its
/// lock.
struct Entry {
    address: IpAddr,
    live: Arc<Mutex<Live>>,
}

/// The sessions by id, the addresses of those starting, and each
/// address's starts.
#[derive(Default)]
struct Table {
    live: HashMap<String, Entry>,
    starting: Vec<IpAddr>,
    starts: HashMap<IpAddr, Bucket>,
}

impl Table {
    /// The sessions of `address`, those starting among them.
    fn of(&self, address: IpAddr) -> usize {
        let starting = self.starting.iter().filter(|a| **a == address).count();
        starting + self.live.values().filter(|e| e.address == address).count()
    }
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

/// The preview's server: its limits, how a session's worker starts, its
/// sessions, and the connections it serves now.
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
            ("POST", "", _) => self.start(visitor(address)),
            ("POST", _, Some(id)) => self.act(id, &request.body),
            ("DELETE", _, Some(id)) => self.end(id),
            (_, "", _) => not_allowed("POST"),
            (_, _, Some(_)) => not_allowed("POST, DELETE"),
            _ => nothing(),
        }
    }

    /// A new session for `address`, and its first frame.
    fn start(&self, address: IpAddr) -> Answer {
        let limits = &self.limits;
        {
            let mut table = lock(&self.table);
            if table.live.len() + table.starting.len() >= limits.sessions {
                return Answer::error(503, "every session is taken: try again later")
                    .retry_after(30);
            }
            if table.of(address) >= limits.per_address {
                let error = format!(
                    "an address may have {} sessions at once: end one first",
                    limits.per_address
                );
                return Answer::error(429, error);
            }
            let now = Instant::now();
            let starts = (table.starts.entry(address))
                .or_insert_with(|| Bucket::new(limits.starts, MINUTE, now));
            if let Err(wait) = starts.take(now) {
                let error = format!("an address may start {} sessions a minute", limits.starts);
                return Answer::error(429, error).retry_after(seconds(wait));
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
        let last = Instant::now();
        let mut live = Live {
            worker,
            allowed: Vec::new(),
            born,
            last,
            actions: 0,
            busy: last - born,
            sent: 0,
            peak: None,
            rate: Bucket::new(self.limits.rate, Duration::from_secs(1), last),
        };
        match live.answered(&id, reply, &self.limits) {
            (answer, None) => Ok((id, live, answer)),
            (answer, Some(why)) => {
                eprintln!(
                    "preview: a session didn't start ({why}): {}",
                    live.summary()
                );
                Err(answer)
            }
        }
    }

    /// Action `body` pressed in session `id`, and the frame after it.
    fn act(&self, id: &str, body: &[u8]) -> Answer {
        let Ok(act) = serde_json::from_slice::<Act>(body) else {
            return Answer::error(400, "an action's body is {\"action\": \"<name>\"}");
        };
        let Some(session) = lock(&self.table).live.get(id).map(|e| Arc::clone(&e.live)) else {
            return gone();
        };
        let mut live = match session.try_lock() {
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
            let error = format!("a session may press {} actions a second", self.limits.rate);
            return Answer::error(429, error).retry_after(seconds(wait));
        }
        if !live.allowed.contains(&act.action) {
            let allowed = live.allowed.join(", ");
            let error = format!("no action `{}`: a visitor's are {allowed}", act.action);
            return Answer::error(400, error);
        }
        live.actions += 1;
        let began = Instant::now();
        let reply = live.worker.call(Some(&act.action), self.limits.deadline);
        let answered = Instant::now();
        live.last = answered;
        live.busy += answered - began;
        let (answer, ends) = live.answered(id, reply, &self.limits);
        drop(live);
        if let Some(why) = ends {
            self.remove(id, &why);
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

    /// Ends session `id`, which ends its worker, and logs `why` and what it
    /// did: whether there was one.
    fn remove(&self, id: &str, why: &str) -> bool {
        let (entry, open) = {
            let mut table = lock(&self.table);
            (table.live.remove(id), table.live.len())
        };
        let Some(entry) = entry else {
            return false;
        };
        // Its worker ends when no action holds it any more.
        let summary = lock(&entry.live).summary();
        drop(entry);
        log_ended(why, &summary, open);
        true
    }

    /// Ends, at `now`, each session idle [`Limits::idle`] since its last
    /// action or [`Limits::lifetime`] old, but one busy with an action; and
    /// forgets the addresses whose starts are full again.
    fn reap(&self, now: Instant) {
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
        let mut ended = Vec::new();
        let open = {
            let mut table = lock(&self.table);
            table.starts.retain(|_, bucket| !bucket.full(now));
            table.live.retain(|_, entry| {
                let live = match entry.live.try_lock() {
                    Ok(live) => live,
                    Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
                    // A session busy with an action isn't idle.
                    Err(TryLockError::WouldBlock) => return true,
                };
                let Some(why) = past(&live) else {
                    return true;
                };
                ended.push((why, live.summary()));
                false
            });
            table.live.len()
        };
        for (why, summary) in ended {
            log_ended(why, &summary, open);
        }
    }

    /// Ends every session, and with it its worker: the server stops.
    pub(crate) fn close(&self) {
        let ids: Vec<String> = lock(&self.table).live.keys().cloned().collect();
        for id in ids {
            self.remove(&id, "the server stopped");
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

/// Says in the server's log that a session ended, `why`, what it did, and
/// the sessions still `open`.
fn log_ended(why: &str, summary: &str, open: usize) {
    eprintln!("preview: a session ended ({why}): {summary}; {open} open");
}

/// The answer to a session that isn't there.
fn gone() -> Answer {
    Answer::error(404, "no such session: it ended, or never was")
}

/// The answer to a method that isn't one of `allowed` on its path.
fn not_allowed(allowed: &str) -> Answer {
    let error = "a session is started and played with POST, and ended with DELETE";
    let mut answer = Answer::error(405, error);
    answer.headers.push(("Allow", allowed.to_owned()));
    answer
}

/// How often the reaper looks: a quarter of the shorter of `limits`' idle
/// time and lifetime, between 20 ms and a second, so that it ends a
/// session at most a second late.
fn tick(limits: &Limits) -> Duration {
    (limits.idle.min(limits.lifetime) / 4).clamp(Duration::from_millis(20), Duration::from_secs(1))
}

/// Serves `listener`'s connections with `server`, each on a thread of its
/// own and at most [`Limits::connections`] at once, while a thread ends
/// the sessions past their time; never returns.
pub(crate) fn serve(listener: &TcpListener, server: &Arc<Server>) {
    let every = tick(&server.limits);
    let reaper = Arc::downgrade(server);
    thread::spawn(move || {
        while let Some(server) = reaper.upgrade() {
            server.reap(Instant::now());
            drop(server);
            thread::sleep(every);
        }
    });
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
            // Out of handles for connections, say: wait, rather than spin.
            thread::sleep(Duration::from_millis(10));
            continue;
        };
        let held = server.connections.fetch_add(1, Ordering::SeqCst);
        if held >= server.limits.connections {
            server.connections.fetch_sub(1, Ordering::SeqCst);
            let _ = stream.set_write_timeout(Some(SOCKET_WAIT));
            let busy = Answer::error(503, "too many connections: try again later");
            let _ = busy.retry_after(5).write_to(&mut stream);
            continue;
        }
        let server = Arc::clone(server);
        thread::spawn(move || {
            server.handle(stream);
            server.connections.fetch_sub(1, Ordering::SeqCst);
        });
    }
}

#[cfg(test)]
mod tests;
