//! `cargo xtask book-pad [--out <dir>] [--no-build]`: the book served from
//! its Codespace (ticket 2840, ADR-0126 §1), so a reader follows the book
//! beside the editor of its project, and each step's game is the reader's
//! own build of it, made again when its files change.
//!
//! - It builds the site once as CI does (`site-data`, `errors.sh`,
//!   `copy-playground.sh`, `copy-previews.sh`, then `zola build`), into a
//!   folder outside the repository's tree, at the address the reader opens
//!   it at: the Codespace's forwarded port 4000, or `http://127.0.0.1:4000/`
//!   outside one.
//! - It serves that folder on `127.0.0.1:4000`, but each step's `play/`
//!   from the step's own web build (`site/tutorial/<step>/dist/web/`) and
//!   its playground's `files/` from the step's folder, uncached. `/` sends
//!   the reader to the last step page served (step 2's at first).
//! - Once a second it reads the modification times under each step's
//!   `crates/` and its `Cargo.toml`, and builds a step that changed
//!   (`cargo shuyi web` in its folder), one build at a time. A build made
//!   another way (the container's `build.sh`, or the reader's own `cargo
//!   shuyi web`) counts as one when the step's `dist/web/index.html`
//!   changes.
//! - `GET /pad/status` says a page is served from here (the public site
//!   answers it with a 404), and `GET /pad/build/<step>` gives the step's
//!   build: its number, the number of its last good one, its state and,
//!   failed, cargo's output. The step's page polls it (`static/book-pad.js`).
//!
//! It answers `GET` only, and refuses a request whose `Host` isn't this
//! server's (403), so another site open in the reader's browser can't read
//! it through a name of its own pointing here.

use std::collections::{BTreeMap, VecDeque};
use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead, BufReader, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use crate::devcontainers;
use crate::site_data;

#[cfg(test)]
mod tests;

/// The port the book's container forwards.
pub const PORT: u16 = 4000;
/// The steps' projects, from the repository's root.
const TUTORIAL: &str = "site/tutorial";
/// The project's pages in the site, each step's at `<step>/`.
const PROJECT: &str = "/learn/book/box-pusher/";
/// A step's web build, in its folder.
const DIST: &str = "dist/web";
/// The files a step's playground edits, in its folder (the project's
/// `extra.game_files`).
const GAME_FILES: &str = "crates/game/files";
/// The step served at `/` until the reader opens another.
const FIRST: &str = "02-board";
/// How long a request may take to arrive.
const READ_TIMEOUT: Duration = Duration::from_secs(10);
/// The longest request line and headers read.
const MAX_HEAD: usize = 16 * 1024;

/// `cargo xtask book-pad`'s options.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// `--out <dir>`: where the site is built; by default `book-site` in the
    /// home folder.
    pub out: Option<PathBuf>,
    /// `--no-build`: serve the site already built there.
    pub no_build: bool,
}

impl Options {
    /// The options in `args`, or `None` on anything else.
    pub fn parse(args: &[String]) -> Option<Self> {
        let mut options = Self::default();
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            match arg.as_str() {
                "--out" => options.out = Some(PathBuf::from(rest.next()?)),
                "--no-build" => options.no_build = true,
                _ => return None,
            }
        }
        Some(options)
    }
}

/// Where the site is served from, as the reader's browser names it: the
/// base URL the site is built with, and every `Host` a request may carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    /// The site's base URL, ending in `/`.
    pub base: String,
    /// The hosts a request may name.
    pub hosts: Vec<String>,
}

impl Address {
    /// The address from the environment `var` reads: the Codespace's
    /// forwarded port when `CODESPACE_NAME` and
    /// `GITHUB_CODESPACES_PORT_FORWARDING_DOMAIN` are set (read as the server
    /// starts: a prebuild's name isn't the reader's), else the loopback.
    pub fn from_env(var: impl Fn(&str) -> Option<String>) -> Self {
        let mut hosts = vec![format!("127.0.0.1:{PORT}"), format!("localhost:{PORT}")];
        let forwarded = match (
            var("CODESPACE_NAME").filter(|v| !v.is_empty()),
            var("GITHUB_CODESPACES_PORT_FORWARDING_DOMAIN").filter(|v| !v.is_empty()),
        ) {
            (Some(name), Some(domain)) => Some(format!("{name}-{PORT}.{domain}")),
            _ => None,
        };
        let base = match forwarded {
            Some(host) => {
                let base = format!("https://{host}/");
                hosts.push(host);
                base
            }
            None => format!("http://127.0.0.1:{PORT}/"),
        };
        Self { base, hosts }
    }
}

/// A step's build state, as `GET /pad/build/<step>` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Not built yet: no `dist/web/index.html`.
    None,
    /// Being built.
    Building,
    /// Its last build worked.
    Ok,
    /// Its last build failed; the last good one is still served.
    Failed,
}

impl State {
    fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Building => "building",
            Self::Ok => "ok",
            Self::Failed => "failed",
        }
    }
}

/// A step's builds since the server started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Build {
    /// Builds started (or seen made another way).
    pub number: u32,
    /// The number of the last good one: 0 is the one there at the start.
    pub good: u32,
    /// Where the last one is.
    pub state: State,
    /// A failed build's output.
    pub output: String,
}

/// What the server and the watcher share.
#[derive(Debug, Default)]
pub struct Pad {
    /// Each step's builds, by its folder's name.
    pub steps: BTreeMap<String, Build>,
    /// The step whose page was served last.
    pub last: String,
}

impl Pad {
    /// Each of `steps` in `tutorial`, built when its `dist/web/index.html`
    /// is there.
    pub fn new(tutorial: &Path, steps: &[String]) -> Self {
        let steps = steps
            .iter()
            .map(|step| {
                let made = tutorial.join(step).join(DIST).join("index.html").is_file();
                let build = Build {
                    number: 0,
                    good: 0,
                    state: if made { State::Ok } else { State::None },
                    output: String::new(),
                };
                (step.clone(), build)
            })
            .collect::<BTreeMap<_, _>>();
        let last = if steps.contains_key(FIRST) {
            FIRST.to_owned()
        } else {
            steps.keys().next().cloned().unwrap_or_default()
        };
        Self { steps, last }
    }
}

/// Locks `pad`, whatever a thread that panicked holding it left.
fn lock(pad: &Mutex<Pad>) -> MutexGuard<'_, Pad> {
    pad.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Builds a step: `Err` holds what the build printed.
pub trait Builder {
    /// Builds the step in the folder `step` of the project.
    fn build(&mut self, step: &str) -> Result<(), String>;
}

/// `cargo shuyi web` in the step's folder.
struct Cargo {
    tutorial: PathBuf,
}

impl Builder for Cargo {
    fn build(&mut self, step: &str) -> Result<(), String> {
        let output = Command::new("cargo")
            .args(["shuyi", "web"])
            .current_dir(self.tutorial.join(step))
            .output()
            .map_err(|e| format!("cargo shuyi web: {e}"))?;
        if output.status.success() {
            Ok(())
        } else {
            let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
            text.push_str(&String::from_utf8_lossy(&output.stderr));
            Err(text)
        }
    }
}

/// Each source of a step and its modification time and length: a change, a
/// new file or a removed one changes it.
type Stamp = Vec<(PathBuf, Option<SystemTime>, u64)>;

/// Watches the steps' sources and builds the ones that change.
pub struct Watcher {
    tutorial: PathBuf,
    /// Each step's sources as last read.
    sources: BTreeMap<String, Stamp>,
    /// Each step's `dist/web/index.html`'s time as last read.
    dists: BTreeMap<String, Option<SystemTime>>,
    /// The steps waiting for a build, in order.
    queue: VecDeque<String>,
}

impl Watcher {
    /// A watcher of `steps` in `tutorial`, as they are now: nothing to build.
    pub fn new(tutorial: &Path, steps: &[String]) -> Self {
        Self {
            tutorial: tutorial.to_path_buf(),
            sources: steps
                .iter()
                .map(|step| (step.clone(), stamp(&tutorial.join(step))))
                .collect(),
            dists: steps
                .iter()
                .map(|step| (step.clone(), dist_time(&tutorial.join(step))))
                .collect(),
            queue: VecDeque::new(),
        }
    }

    /// The steps waiting for a build.
    #[cfg(test)]
    pub fn queued(&self) -> Vec<&str> {
        self.queue.iter().map(String::as_str).collect()
    }

    /// Reads every step again, queues each one changed, and builds the
    /// first queued, if any. A change made during a build is read at the
    /// next tick, and queues that step once more.
    pub fn tick(&mut self, builder: &mut dyn Builder, pad: &Mutex<Pad>) {
        for (step, known) in &mut self.sources {
            let now = stamp(&self.tutorial.join(step));
            if now != *known {
                *known = now;
                if !self.queue.contains(step) {
                    self.queue.push_back(step.clone());
                }
            }
        }
        for (step, known) in &mut self.dists {
            let now = dist_time(&self.tutorial.join(step));
            if now != *known {
                *known = now;
                if now.is_some() {
                    // Built another way: a good build all the same.
                    let mut pad = lock(pad);
                    if let Some(build) = pad.steps.get_mut(step) {
                        build.number += 1;
                        build.good = build.number;
                        build.state = State::Ok;
                        build.output.clear();
                    }
                }
            }
        }
        let Some(step) = self.queue.pop_front() else {
            return;
        };
        if let Some(build) = lock(pad).steps.get_mut(&step) {
            build.number += 1;
            build.state = State::Building;
        }
        eprintln!("book-pad: {step} changed: building it");
        let started = Instant::now();
        let result = builder.build(&step);
        let seconds = started.elapsed().as_secs_f32();
        let mut pad = lock(pad);
        if let Some(build) = pad.steps.get_mut(&step) {
            match result {
                Ok(()) => {
                    eprintln!("book-pad: {step} built in {seconds:.1} s");
                    build.good = build.number;
                    build.state = State::Ok;
                    build.output.clear();
                }
                Err(output) => {
                    eprintln!("book-pad: {step}'s build failed in {seconds:.1} s");
                    build.state = State::Failed;
                    build.output = output;
                }
            }
        }
        // Its own build isn't one made another way.
        self.dists
            .insert(step.clone(), dist_time(&self.tutorial.join(&step)));
    }
}

/// The step in `folder`'s sources: every file under its `crates/`, and its
/// `Cargo.toml`, by path.
fn stamp(folder: &Path) -> Stamp {
    let mut found = Vec::new();
    walk(&folder.join("crates"), &mut found);
    let manifest = folder.join("Cargo.toml");
    if let Ok(meta) = fs::metadata(&manifest) {
        found.push((manifest, meta.modified().ok(), meta.len()));
    }
    found.sort();
    found
}

fn walk(dir: &Path, found: &mut Stamp) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.is_dir() {
            walk(&path, found);
        } else {
            found.push((path, meta.modified().ok(), meta.len()));
        }
    }
}

/// When the step in `folder`'s web build was last made, if it is there.
fn dist_time(folder: &Path) -> Option<SystemTime> {
    fs::metadata(folder.join(DIST).join("index.html"))
        .and_then(|meta| meta.modified())
        .ok()
}

/// A request, as far as the server reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// `GET`, say.
    pub method: String,
    /// The path, its query left out, not yet decoded.
    pub path: String,
    /// The `Host` header, if any.
    pub host: Option<String>,
}

/// Reads a request's line and headers from `input`; `None` on anything
/// that isn't one.
pub fn read_request(input: &mut impl BufRead) -> Option<Request> {
    let mut read = 0;
    let mut line = String::new();
    let mut next = |line: &mut String| -> Option<()> {
        line.clear();
        let n = input.by_ref().take(MAX_HEAD as u64).read_line(line).ok()?;
        read += n;
        (n > 0 && read <= MAX_HEAD).then_some(())
    };
    next(&mut line)?;
    let mut words = line.split_whitespace();
    let method = words.next()?.to_owned();
    let target = words.next()?;
    words.next().filter(|v| v.starts_with("HTTP/"))?;
    let path = target
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .to_owned();
    let mut host = None;
    loop {
        next(&mut line)?;
        let header = line.trim_end_matches(['\r', '\n']);
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.trim().eq_ignore_ascii_case("host")
        {
            host = Some(value.trim().to_owned());
        }
    }
    Some(Request { method, path, host })
}

/// An answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// The status code.
    pub status: u16,
    /// Headers besides the length, the cache's and the connection's.
    pub headers: Vec<(&'static str, String)>,
    /// The body.
    pub body: Vec<u8>,
}

impl Response {
    fn text(status: u16, text: &str) -> Self {
        Self {
            status,
            headers: vec![("Content-Type", "text/plain; charset=utf-8".to_owned())],
            body: text.as_bytes().to_vec(),
        }
    }

    fn json(value: &serde_json::Value) -> Self {
        Self {
            status: 200,
            headers: vec![("Content-Type", "application/json".to_owned())],
            body: value.to_string().into_bytes(),
        }
    }

    fn redirect(status: u16, to: &str) -> Self {
        Self {
            status,
            headers: vec![("Location", to.to_owned())],
            body: Vec::new(),
        }
    }

    /// The header named `name`, if any.
    #[cfg(test)]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// The whole answer, as sent.
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = format!("HTTP/1.1 {} {}\r\n", self.status, reason(self.status));
        for (name, value) in &self.headers {
            let _ = write!(out, "{name}: {value}\r\n");
        }
        let _ = write!(
            out,
            "Content-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
            self.body.len()
        );
        let mut bytes = out.into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        301 => "Moved Permanently",
        302 => "Found",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Internal Server Error",
    }
}

/// The server: what it serves, and from where.
#[derive(Clone)]
pub struct Server {
    /// The built site.
    pub site: PathBuf,
    /// The steps' projects.
    pub tutorial: PathBuf,
    /// Where the reader's browser reaches it.
    pub address: Address,
    /// What it shares with the watcher.
    pub pad: Arc<Mutex<Pad>>,
}

impl Server {
    /// The answer to `request`.
    pub fn answer(&self, request: &Request) -> Response {
        if !request.host.as_ref().is_some_and(|host| {
            self.address
                .hosts
                .iter()
                .any(|h| h.eq_ignore_ascii_case(host))
        }) {
            return Response::text(403, "This server answers its own address only.");
        }
        if request.method != "GET" {
            let mut response = Response::text(405, "This server answers GET only.");
            response.headers.push(("Allow", "GET".to_owned()));
            return response;
        }
        if request.path == "/" {
            let last = lock(&self.pad).last.clone();
            return Response::redirect(302, &format!("{PROJECT}{last}/"));
        }
        let Some(segments) = segments(&request.path) else {
            return Response::text(400, "That path isn't one this server reads.");
        };
        if let Some(pad) = request.path.strip_prefix("/pad/") {
            return self.pad_route(pad);
        }
        let parts: Vec<&str> = segments.iter().map(String::as_str).collect();
        let project: Vec<&str> = PROJECT.split('/').filter(|s| !s.is_empty()).collect();
        if let Some(rest) = parts.strip_prefix(project.as_slice())
            && let [step, kind, inner @ ..] = rest
            && lock(&self.pad).steps.contains_key(*step)
        {
            let from = match *kind {
                "play" => Some(self.tutorial.join(step).join(DIST)),
                "files" => Some(self.tutorial.join(step).join(GAME_FILES)),
                _ => None,
            };
            if let Some(from) = from {
                return self.file(&from, inner, &request.path);
            }
        }
        let response = self.file(&self.site, &parts, &request.path);
        if response.status == 200
            && let Some(rest) = parts.strip_prefix(project.as_slice())
            && let [step] | [step, "index.html"] = rest
        {
            (*step).clone_into(&mut lock(&self.pad).last);
        }
        response
    }

    /// `/pad/<route>`.
    fn pad_route(&self, route: &str) -> Response {
        if route == "status" {
            return Response::json(&serde_json::json!({ "pad": true }));
        }
        if let Some(step) = route.strip_prefix("build/")
            && let Some(build) = lock(&self.pad).steps.get(step)
        {
            return Response::json(&serde_json::json!({
                "step": step,
                "number": build.number,
                "good": build.good,
                "state": build.state.name(),
                "output": build.output,
            }));
        }
        Response::text(404, "No such route.")
    }

    /// The file at `parts` under `root`, a folder's `index.html`; `path` is
    /// the request's, for a folder named without its last `/`.
    fn file(&self, root: &Path, parts: &[&str], path: &str) -> Response {
        let mut file = root.to_path_buf();
        file.extend(parts);
        if file.is_dir() {
            if !path.ends_with('/') {
                return Response::redirect(301, &format!("{path}/"));
            }
            file.push("index.html");
        }
        match fs::read(&file) {
            Ok(body) => Response {
                status: 200,
                headers: vec![("Content-Type", content_type(&file).to_owned())],
                body,
            },
            Err(_) => self.not_found(),
        }
    }

    fn not_found(&self) -> Response {
        match fs::read(self.site.join("404.html")) {
            Ok(body) => Response {
                status: 404,
                headers: vec![("Content-Type", "text/html; charset=utf-8".to_owned())],
                body,
            },
            Err(_) => Response::text(404, "Not found."),
        }
    }

    /// Reads one request from `stream` and answers it.
    fn serve(&self, stream: TcpStream) {
        let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
        let Ok(mut writer) = stream.try_clone() else {
            return;
        };
        let mut reader = BufReader::new(stream);
        let response = match read_request(&mut reader) {
            Some(request) => self.answer(&request),
            None => Response::text(400, "That isn't a request this server reads."),
        };
        let _ = writer.write_all(&response.bytes());
        let _ = writer.flush();
        // Closed gracefully, never reset: Windows resets a socket closed
        // with anything unread.
        let _ = writer.shutdown(std::net::Shutdown::Write);
        let mut rest = [0; 512];
        while reader.get_mut().read(&mut rest).is_ok_and(|n| n > 0) {}
    }
}

/// The decoded segments of `path`, or `None` for one that could leave the
/// folder it names or isn't text: a `..` or `.`, a `\`, a `:` or a NUL.
pub fn segments(path: &str) -> Option<Vec<String>> {
    let path = path.strip_prefix('/')?;
    let mut out = Vec::new();
    for raw in path.split('/') {
        if raw.is_empty() {
            continue;
        }
        let segment = decode(raw)?;
        if segment == "." || segment == ".." || segment.contains(['/', '\\', ':', '\0']) {
            return None;
        }
        out.push(segment);
    }
    Some(out)
}

/// `text` with each `%XX` decoded, if that is UTF-8.
fn decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// The `Content-Type` of `file`, by its extension.
fn content_type(file: &Path) -> &'static str {
    match file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
    {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "wasm" => "application/wasm",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "xml" => "application/xml",
        "woff2" => "font/woff2",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "md" => "text/markdown; charset=utf-8",
        "txt" | "ron" | "map" | "dlg" | "toml" | "rs" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// Builds the site as CI does, at `base`, into `out`.
fn build_site(root: &Path, out: &Path, base: &str) -> Result<(), String> {
    site_data::run(
        root,
        &site_data::Options {
            site: root.join("site"),
        },
    )
    .map_err(|errors| errors.join("\n"))?;
    for script in [
        "site/errors.sh",
        "site/copy-playground.sh",
        "site/copy-previews.sh",
    ] {
        run(Command::new("bash").arg(script).current_dir(root), script)?;
    }
    let out = out.to_string_lossy();
    run(
        Command::new("zola")
            .args(["build", "--base-url", base, "--output-dir", &out, "--force"])
            .current_dir(root.join("site")),
        "zola build",
    )
}

fn run(command: &mut Command, name: &str) -> Result<(), String> {
    let status = command.status().map_err(|e| format!("{name}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{name} failed ({status})"))
    }
}

/// The home folder's `book-site`, or the temporary folder's.
fn default_out() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join("book-site")
}

/// Serves the book until stopped. A port already taken means the book is
/// served already: said, and not an error.
pub fn run_pad(root: &Path, options: &Options) -> Result<(), String> {
    let listener = match TcpListener::bind(("127.0.0.1", PORT)) {
        Ok(listener) => listener,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            eprintln!(
                "book-pad: port {PORT} is taken: the book is already served (the Ports tab opens it)"
            );
            return Ok(());
        }
        Err(e) => return Err(format!("127.0.0.1:{PORT}: {e}")),
    };
    let address = Address::from_env(|name| std::env::var(name).ok());
    let out = options.out.clone().unwrap_or_else(default_out);
    if !options.no_build {
        eprintln!(
            "book-pad: building the site at {} into {}",
            address.base,
            out.display()
        );
        build_site(root, &out, &address.base)?;
    }
    let tutorial = root.join(TUTORIAL);
    let steps = devcontainers::steps(&tutorial).map_err(|e| format!("{TUTORIAL}: {e}"))?;
    let pad = Arc::new(Mutex::new(Pad::new(&tutorial, &steps)));
    let mut watcher = Watcher::new(&tutorial, &steps);
    let mut builder = Cargo {
        tutorial: tutorial.clone(),
    };
    let shared = Arc::clone(&pad);
    thread::spawn(move || {
        loop {
            watcher.tick(&mut builder, &shared);
            thread::sleep(Duration::from_secs(1));
        }
    });
    let server = Server {
        site: out,
        tutorial,
        address,
        pad,
    };
    eprintln!(
        "book-pad: serving the book at {} (the Ports tab opens it); a step is built again when its files change",
        server.address.base
    );
    for stream in listener.incoming().flatten() {
        let server = server.clone();
        thread::spawn(move || server.serve(stream));
    }
    Ok(())
}
