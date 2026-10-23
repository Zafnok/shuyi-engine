//! The projects' own servers (ticket 2546): a call whose project is in
//! another cargo workspace than the server's goes to a `shuyi mcp` built
//! from that workspace's engine, a **forwarded server**, so an agent in a
//! worktree plays and checks the worktree's code, whichever checkout the
//! client started the server in.
//!
//! A forwarded server is the project's `command` and `mcp` ([`words`]),
//! run in the project's root with pipes to its stdin, stdout and stderr,
//! and [`FORWARDED`] set, so it answers every call itself. It gets an
//! `initialize` without roots as it starts, and its build runs while the
//! server goes on. It ends when its stdin closes: when the server drops it,
//! or ends, or is killed.
//!
//! [`Forwarding`] is the seam the server starts them and looks up
//! workspaces through ([`Processes`] for real), so its tests run no cargo.

use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use super::child::keep_tail;
use super::server::PROTOCOL_VERSIONS;
use crate::project::Project;

/// The environment variable a forwarded server is started with: it hands
/// nothing on, so a server is never forwarded twice.
pub const FORWARDED: &str = "SHUYI_MCP_FORWARDED";

/// The id of the `initialize` request a forwarded server gets.
const INITIALIZE: u64 = 0;

/// A project's own server that the server hands calls to.
pub trait Forward {
    /// Whether it has answered `initialize`: none while it builds (or,
    /// with `wait`, once it has answered or stopped), or why it never will.
    fn ready(&mut self, wait: bool) -> Option<Result<(), String>>;

    /// Its response to the request `method` with `params`, once it is
    /// ready, or why there is none: it stopped, with its last output.
    fn request(&mut self, method: &str, params: &Value) -> Result<Value, String>;
}

/// Looks up workspaces and starts forwarded servers.
pub trait Forwarding<'a> {
    /// The root of the cargo workspace `folder` is in, none outside one.
    /// Paths of one folder compare equal.
    fn workspace(&self, folder: &Path) -> Option<PathBuf>;

    /// A forwarded server: `words` run in `root`.
    fn spawn(&self, words: &[String], root: &Path) -> Box<dyn Forward + 'a>;
}

/// The words that start the server of the project at `root`, in the
/// workspace `workspace`, and the folder they run in:
///
/// - the project's `command` (`shuyi.ron`) and `mcp`, in its root, if it
///   names one;
/// - else the `shuyi` entry of the workspace's `.mcp.json` (the engine's
///   own repository has one and no `shuyi.ron` at its root), in the
///   workspace;
/// - else `shuyi mcp` in the project's root.
///
/// None outside a project with no entry: the server answers itself.
pub fn words(root: &Path, workspace: &Path) -> Option<(Vec<String>, PathBuf)> {
    let project = Project::find(root).ok();
    if let Some(project) = &project
        && let Some(command) = &project.config.command
    {
        return Some((with_mcp(command), project.root.clone()));
    }
    if let Some(words) = entry(&workspace.join(".mcp.json")) {
        return Some((words, workspace.to_path_buf()));
    }
    project.map(|project| (with_mcp(project.config.command()), project.root))
}

/// `command`'s words, then `mcp`.
fn with_mcp(command: &str) -> Vec<String> {
    command
        .split_whitespace()
        .chain(["mcp"])
        .map(str::to_owned)
        .collect()
}

/// The `shuyi` server's command and args in the `.mcp.json` at `path`.
fn entry(path: &Path) -> Option<Vec<String>> {
    let file: Value = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    let server = &file["mcpServers"]["shuyi"];
    let command = server["command"].as_str()?;
    let args = server["args"].as_array().map_or(&[][..], Vec::as_slice);
    std::iter::once(Some(command))
        .chain(args.iter().map(Value::as_str))
        .map(|word| word.map(str::to_owned))
        .collect()
}

/// The folder a `file://` URI names (a client's root), or none if it
/// names none: its percent-escapes decoded, and a Windows drive's slash
/// dropped (`file:///C:/x` is `C:/x`).
pub fn path_of_uri(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let path = rest.strip_prefix("localhost").unwrap_or(rest);
    if !path.starts_with('/') {
        return None;
    }
    let text = String::from_utf8(decoded(path)?).ok()?;
    let b = text.as_bytes();
    let text = if b.len() >= 3 && b[1].is_ascii_alphabetic() && b[2] == b':' {
        &text[1..]
    } else {
        &text
    };
    if cfg!(windows) {
        Some(PathBuf::from(text.replace('/', "\\")))
    } else {
        Some(PathBuf::from(text))
    }
}

/// `text` with each `%XX` made its byte, or none if one is broken.
fn decoded(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len());
    let mut bytes = text.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let hex = [bytes.next()?, bytes.next()?];
            out.push(u8::from_str_radix(std::str::from_utf8(&hex).ok()?, 16).ok()?);
        } else {
            out.push(byte);
        }
    }
    Some(out)
}

/// `path` without the `\\?\` before a Windows drive that canonicalizing
/// gives, which some programs don't read.
fn plain(path: PathBuf) -> PathBuf {
    match path.to_str().and_then(|text| text.strip_prefix(r"\\?\")) {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => path,
    }
}

/// Looks up workspaces with cargo and starts forwarded servers as
/// processes.
#[derive(Debug, Clone, Copy, Default)]
pub struct Processes;

impl<'a> Forwarding<'a> for Processes {
    fn workspace(&self, folder: &Path) -> Option<PathBuf> {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
        let output = Command::new(cargo)
            .args(["locate-project", "--workspace", "--message-format", "plain"])
            .current_dir(folder)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let manifest = PathBuf::from(String::from_utf8(output.stdout).ok()?.trim());
        let root = manifest.parent()?;
        Some(plain(
            fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf()),
        ))
    }

    fn spawn(&self, words: &[String], root: &Path) -> Box<dyn Forward + 'a> {
        match Process::start(words, root) {
            Ok(process) => Box::new(process),
            Err(why) => Box::new(Failed(why)),
        }
    }
}

/// A forwarded server that never started: each call says why.
struct Failed(String);

impl Forward for Failed {
    fn ready(&mut self, _: bool) -> Option<Result<(), String>> {
        Some(Err(self.0.clone()))
    }

    fn request(&mut self, _: &str, _: &Value) -> Result<Value, String> {
        Err(self.0.clone())
    }
}

/// A forwarded server: its stdin, the lines of its stdout, and the last
/// of its stderr.
pub struct Process {
    /// What it is, for errors: `the project's own server ("…" in …)`.
    named: String,
    child: Option<Child>,
    stdin: Option<Box<dyn Write + Send>>,
    lines: Receiver<String>,
    tail: Arc<Mutex<Vec<u8>>>,
    /// Hears when its stderr has been copied to the end.
    copied: Option<Receiver<()>>,
    /// Whether it answered `initialize`, once known.
    state: Option<Result<(), String>>,
    /// The id of its next request.
    next: u64,
}

impl Process {
    /// A forwarded server called `named`, writing to `stdin` and answering
    /// on `lines`, its stderr's last bytes in `tail`; sent `initialize` now.
    pub fn new(
        named: String,
        stdin: Box<dyn Write + Send>,
        lines: Receiver<String>,
        tail: Arc<Mutex<Vec<u8>>>,
    ) -> Self {
        let mut process = Self {
            named,
            child: None,
            stdin: Some(stdin),
            lines,
            tail,
            copied: None,
            state: None,
            next: INITIALIZE + 1,
        };
        process.send(&json!({
            "jsonrpc": "2.0",
            "id": INITIALIZE,
            "method": "initialize",
            "params": {
                "protocolVersion": PROTOCOL_VERSIONS[0],
                "capabilities": {},
                "clientInfo": { "name": "shuyi", "version": env!("CARGO_PKG_VERSION") },
            },
        }));
        process
    }

    /// `words` run in `root` as a forwarded server, or why it didn't
    /// start.
    fn start(words: &[String], root: &Path) -> Result<Self, String> {
        let named = format!(
            "the project's own server (\"{}\" in {})",
            words.join(" "),
            root.display()
        );
        let Some((program, args)) = words.split_first() else {
            return Err(format!("{named} has no command"));
        };
        eprintln!("mcp: {} is answered by {named}", root.display());
        let spawned = Command::new(program)
            .args(args)
            .current_dir(root)
            .env(FORWARDED, "1")
            // So `cargo` picks the toolchain of the project's checkout.
            .env_remove("RUSTUP_TOOLCHAIN")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match spawned {
            Ok(child) => child,
            Err(e) => return Err(format!("{named} didn't start: {e}")),
        };
        let (Some(stdin), Some(stdout), Some(stderr)) =
            (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            let _ = child.kill();
            return Err(format!("{named} started without its pipes"));
        };
        let (sent, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if sent.send(line).is_err() {
                    break;
                }
            }
        });
        let tail = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&tail);
        let (ended, copied) = channel();
        std::thread::spawn(move || {
            keep_tail(stderr, &kept);
            let _ = ended.send(());
        });
        let mut process = Self::new(named, Box::new(stdin), lines, tail);
        process.child = Some(child);
        process.copied = Some(copied);
        Ok(process)
    }

    /// Writes `message` on a line of its stdin; false if it is closed.
    fn send(&mut self, message: &Value) -> bool {
        let sent = self
            .stdin
            .as_mut()
            .map(|stdin| writeln!(stdin, "{message}").and_then(|()| stdin.flush()));
        matches!(sent, Some(Ok(())))
    }

    /// Why it answers nothing: it stopped, with its exit code and the
    /// last of its output. It is stopped for good.
    fn stopped(&mut self) -> String {
        if let Some(Err(why)) = &self.state {
            return why.clone();
        }
        self.stdin = None;
        let status = self
            .child
            .as_mut()
            .and_then(|child| child.wait().ok())
            .and_then(|status| status.code());
        // Its stderr closes as it ends: let the copy catch up, briefly.
        if let Some(copied) = &self.copied {
            let _ = copied.recv_timeout(Duration::from_secs(1));
        }
        let tail = self
            .tail
            .lock()
            .map(|tail| String::from_utf8_lossy(&tail).trim().to_owned())
            .unwrap_or_default();
        let mut out = format!("{} stopped before it answered", self.named);
        if let Some(code) = status {
            let _ = write!(out, " (exit code {code})");
        }
        if !tail.is_empty() {
            out.push_str(": ");
            out.push_str(&tail);
        }
        self.state = Some(Err(out.clone()));
        out
    }

    /// Reads its lines until the response to `id`, waiting for each with
    /// `wait`: `Err` if it stops first, none if `wait` is false and it
    /// hasn't come yet.
    fn response(&mut self, id: u64, wait: bool) -> Result<Option<Value>, String> {
        loop {
            let next = if wait {
                self.lines.recv().ok()
            } else {
                match self.lines.try_recv() {
                    Ok(line) => Some(line),
                    Err(TryRecvError::Empty) => return Ok(None),
                    Err(TryRecvError::Disconnected) => None,
                }
            };
            // Its stdout closed.
            let Some(line) = next else {
                return Err(self.stopped());
            };
            // A notification, or a line that isn't JSON: not the answer.
            let Ok(message) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if message.get("id") == Some(&json!(id)) {
                return Ok(Some(message));
            }
        }
    }
}

impl Forward for Process {
    fn ready(&mut self, wait: bool) -> Option<Result<(), String>> {
        if self.state.is_none() {
            let state = match self.response(INITIALIZE, wait) {
                Ok(None) => return None,
                Err(why) => Err(why),
                Ok(Some(answer)) if answer.get("result").is_some() => {
                    self.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
                    Ok(())
                }
                Ok(Some(answer)) => Err(format!(
                    "{} refused to start: {}",
                    self.named, answer["error"]
                )),
            };
            self.state = Some(state);
        }
        self.state.clone()
    }

    fn request(&mut self, method: &str, params: &Value) -> Result<Value, String> {
        if let Some(Err(why)) = self.ready(true) {
            return Err(why);
        }
        let id = self.next;
        self.next += 1;
        let request = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        if !self.send(&request) {
            return Err(self.stopped());
        }
        self.response(id, true)?
            .ok_or_else(|| format!("{} answered nothing", self.named))
    }
}

impl Drop for Process {
    /// Closes its stdin, which ends it, and makes sure it has ended.
    fn drop(&mut self) {
        self.stdin = None;
        if let Some(child) = self.child.as_mut() {
            // A moment to end by itself, as a server does when its stdin
            // closes; then it is ended.
            let _ = self
                .copied
                .as_ref()
                .map(|copied| copied.recv_timeout(Duration::from_millis(500)));
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(test)]
mod tests;
