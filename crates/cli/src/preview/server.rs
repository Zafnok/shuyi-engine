//! The preview's server: its routes, its sessions, and the workers their
//! games run in.
//!
//! A request is answered on a thread of its own. A session's actions are
//! answered one at a time: a second while one runs is refused (`409`). A
//! worker that doesn't answer a start or an action within
//! [`Limits::deadline`] is killed, and its session ended.

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
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

/// A session: its game's worker, and the actions its last frame allows.
struct Live {
    worker: Worker,
    allowed: Vec<String>,
}

impl Live {
    /// The answer of session `id` to its worker's `reply`, and whether the
    /// session goes on.
    fn answered(&mut self, id: &str, reply: Result<Reply, String>) -> (Answer, bool) {
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
            let error = format!("the game failed: {failed}");
            return (Answer::error(500, error), false);
        };
        // A frame as a visitor's answer: its session's id and what it shows.
        let frame = json!({
            "session": id, "png": shown.png, "text": shown.text,
            "actions": shown.actions, "ended": shown.ended,
        });
        self.allowed = shown.actions;
        (Answer::json(200, &frame), !shown.ended)
    }
}

/// The sessions by id, and how many are starting.
#[derive(Default)]
struct Table {
    live: HashMap<String, Arc<Mutex<Live>>>,
    starting: usize,
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
}

impl Server {
    /// A server of no sessions yet, which keeps `limits` and starts each
    /// session's worker with `launch`.
    pub(crate) fn new(limits: Limits, launch: Launch) -> Self {
        Self {
            limits,
            launch,
            table: Mutex::new(Table::default()),
        }
    }

    /// The answer to `request`.
    pub(crate) fn answer(&self, request: &Request) -> Answer {
        let nothing = || Answer::error(404, "nothing is served here");
        let Some(rest) = request.path.strip_prefix("/session") else {
            return nothing();
        };
        match (request.method.as_str(), rest, id_of(rest)) {
            ("POST", "", _) => self.start(),
            ("POST", _, Some(id)) => self.act(id, &request.body),
            ("DELETE", _, Some(id)) => self.end(id),
            (_, "", _) => not_allowed("POST"),
            (_, _, Some(_)) => not_allowed("POST, DELETE"),
            _ => nothing(),
        }
    }

    /// A new session, and its first frame.
    fn start(&self) -> Answer {
        {
            let mut table = lock(&self.table);
            if table.live.len() + table.starting >= self.limits.sessions {
                return Answer::error(503, "every session is taken: try again later")
                    .retry_after(30);
            }
            table.starting += 1;
        }
        let opened = self.open();
        let mut table = lock(&self.table);
        table.starting -= 1;
        match opened {
            Ok((id, live, answer)) => {
                table.live.insert(id, Arc::new(Mutex::new(live)));
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
        let mut worker = (self.launch)()
            .map_err(|e| Answer::error(500, format!("the game's worker didn't start: {e}")))?;
        let reply = worker.call(None, self.limits.deadline);
        let mut live = Live {
            worker,
            allowed: Vec::new(),
        };
        match live.answered(&id, reply) {
            (answer, true) => Ok((id, live, answer)),
            (answer, false) => Err(answer),
        }
    }

    /// Action `body` pressed in session `id`, and the frame after it.
    fn act(&self, id: &str, body: &[u8]) -> Answer {
        let Ok(act) = serde_json::from_slice::<Act>(body) else {
            return Answer::error(400, "an action's body is {\"action\": \"<name>\"}");
        };
        let Some(session) = lock(&self.table).live.get(id).map(Arc::clone) else {
            return gone();
        };
        let mut live = match session.try_lock() {
            Ok(live) => live,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => {
                return Answer::error(409, "the session is busy with an action").retry_after(1);
            }
        };
        if !live.allowed.contains(&act.action) {
            let allowed = live.allowed.join(", ");
            let error = format!("no action `{}`: a visitor's are {allowed}", act.action);
            return Answer::error(400, error);
        }
        let reply = live.worker.call(Some(&act.action), self.limits.deadline);
        let (answer, goes_on) = live.answered(id, reply);
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

    /// Ends session `id`, which ends its worker, and logs `why`: whether
    /// there was one.
    fn remove(&self, id: &str, why: &str) -> bool {
        let (live, open) = {
            let mut table = lock(&self.table);
            (table.live.remove(id), table.live.len())
        };
        let Some(live) = live else {
            return false;
        };
        // Its worker ends when no action holds it any more.
        drop(live);
        eprintln!("preview: a session ended ({why}): {open} open");
        true
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
        let answer = match http::read(&mut stream, Instant::now() + REQUEST_TIME) {
            Ok(request) => self.answer(&request),
            Err(refused) => refused,
        };
        let _ = answer.write_to(&mut stream);
    }
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

/// Serves `listener`'s connections with `server`, each on a thread of its
/// own; never returns.
pub(crate) fn serve(listener: &TcpListener, server: &Arc<Server>) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else {
            // Out of handles for connections, say: wait, rather than spin.
            thread::sleep(Duration::from_millis(10));
            continue;
        };
        let server = Arc::clone(server);
        thread::spawn(move || server.handle(stream));
    }
}

#[cfg(test)]
mod tests;
