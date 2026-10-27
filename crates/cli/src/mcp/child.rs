//! The game's tool program as the server's host: `cargo run` of the
//! program `shuyi.ron` names, as `shuyi mcp-host`, with pipes to its
//! stdin and stdout. What it writes to stderr (the compiler's errors, a
//! panic) goes on to the server's stderr, and its last lines go into the
//! error when it stops before it answers.

use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::host::{REPLY, Reply, Request};
use super::server::{Launch, Link};
use crate::contract::codes::{self, Code};
use crate::project::{FILE, Project};

/// How much of the program's last output an error keeps, in bytes.
const TAIL: usize = 6000;

/// Launches the game's tool program with cargo (`$CARGO`, or `cargo`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Cargo;

impl<'a> Launch<'a> for Cargo {
    fn launch(&self, project: &Project) -> Result<Box<dyn Link + 'a>, (Code, String)> {
        let Some(program) = &project.config.game else {
            return Err((
                codes::PROJECT_FILE,
                format!("{FILE} names no game tool program (`game`), and the tool runs the game"),
            ));
        };
        let cargo = crate::cargo();
        let mut child = Command::new(cargo)
            .args(program.cargo_args("mcp-host", &[]))
            .current_dir(&project.root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| (codes::TOOLCHAIN, format!("cannot run cargo: {e}")))?;
        let (Some(stdin), Some(stdout), Some(stderr)) =
            (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            let _ = child.kill();
            return Err((
                codes::TOOLCHAIN,
                "cargo started without its pipes".to_owned(),
            ));
        };
        let tail = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&tail);
        let (ended, copied) = channel();
        std::thread::spawn(move || {
            keep_tail(stderr, &kept);
            let _ = ended.send(());
        });
        Ok(Box::new(Program {
            child,
            stdin: Some(stdin),
            stdout: BufReader::new(stdout),
            tail,
            copied,
        }))
    }
}

/// Copies `from` to this process's stderr, keeping its last [`TAIL`]
/// bytes in `tail`.
pub(super) fn keep_tail(mut from: impl Read, tail: &Mutex<Vec<u8>>) {
    let mut buf = [0u8; 4096];
    while let Ok(n) = from.read(&mut buf) {
        if n == 0 {
            break;
        }
        let _ = std::io::stderr().write_all(&buf[..n]);
        if let Ok(mut tail) = tail.lock() {
            tail.extend_from_slice(&buf[..n]);
            let over = tail.len().saturating_sub(TAIL);
            tail.drain(..over);
        }
    }
}

/// A running game tool program.
struct Program {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    tail: Arc<Mutex<Vec<u8>>>,
    /// Hears when its stderr has been copied to the end.
    copied: Receiver<()>,
}

impl Program {
    /// Why it answered nothing: it stopped, with its exit code and the
    /// last of its output.
    fn stopped(&mut self) -> String {
        self.stdin = None;
        let status = self.child.wait().ok().and_then(|status| status.code());
        // Its stderr closes as it ends: let the copy catch up, briefly.
        let _ = self.copied.recv_timeout(Duration::from_secs(1));
        let tail = self
            .tail
            .lock()
            .map(|tail| String::from_utf8_lossy(&tail).trim().to_owned())
            .unwrap_or_default();
        let mut out = "the game's tool program stopped before it answered".to_owned();
        if let Some(code) = status {
            let _ = write!(out, " (exit code {code})");
        }
        if !tail.is_empty() {
            out.push_str(": ");
            out.push_str(&tail);
        }
        out
    }
}

impl Link for Program {
    fn call(&mut self, request: &Request) -> Result<Reply, String> {
        let line = serde_json::to_string(request).map_err(|e| e.to_string())?;
        let sent = self
            .stdin
            .as_mut()
            .map(|stdin| writeln!(stdin, "{line}").and_then(|()| stdin.flush()));
        if !matches!(sent, Some(Ok(()))) {
            return Err(self.stopped());
        }
        let mut line = String::new();
        loop {
            line.clear();
            match self.stdout.read_line(&mut line) {
                Ok(0) | Err(_) => return Err(self.stopped()),
                Ok(_) => {}
            }
            // A line without the mark is the game's own printing.
            if let Some(reply) = line.trim_end().strip_prefix(REPLY) {
                return serde_json::from_str(reply)
                    .map_err(|e| format!("the game's tool program sent a broken reply ({e})"));
            }
        }
    }
}

impl Drop for Program {
    /// Closes its stdin and ends it: nothing it would still say is
    /// wanted.
    fn drop(&mut self) {
        self.stdin = None;
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests;
