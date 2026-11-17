//! Where `repos` runs its programs ([`Host`]): the machine's `gh`, `git`
//! and `ssh-keygen` ([`ThisMachine`]), or a test's script. And the
//! temporary folder that holds a key or the private repository's first
//! commit, removed whatever happens ([`TempDir`]).

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::project::Project;

/// What a program that started did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ran {
    /// Whether it exited with success.
    pub ok: bool,
    /// What it printed on stdout.
    pub stdout: String,
    /// What it printed on stderr.
    pub stderr: String,
}

impl Ran {
    /// What it said: stderr, else stdout, trimmed.
    pub fn said(&self) -> String {
        let err = self.stderr.trim();
        if err.is_empty() {
            self.stdout.trim().to_owned()
        } else {
            err.to_owned()
        }
    }
}

/// Where `repos` runs its programs.
pub trait Host {
    /// Runs `program` with `args` in `dir`, with `stdin` on its standard
    /// input (nothing otherwise). The error is that it didn't start: not
    /// installed, most often.
    fn run(
        &mut self,
        program: &str,
        args: &[&str],
        dir: &Path,
        stdin: Option<&str>,
    ) -> std::io::Result<Ran>;

    /// The folder `repos` makes its temporary folders in.
    fn temp_root(&self) -> PathBuf;

    /// Checks out the project's private files (`shuyi private-assets`):
    /// its one-line summary, or why it failed.
    fn fetch_private(&mut self, project: &Project) -> Result<String, String>;
}

/// The machine `repos` runs on.
#[derive(Debug, Clone, Copy, Default)]
pub struct ThisMachine;

impl Host for ThisMachine {
    fn run(
        &mut self,
        program: &str,
        args: &[&str],
        dir: &Path,
        stdin: Option<&str>,
    ) -> std::io::Result<Ran> {
        let mut command = Command::new(program);
        command
            .args(args)
            .current_dir(dir)
            // Neither ever stops to ask on the terminal.
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GH_PROMPT_DISABLED", "1")
            .env("GH_NO_UPDATE_NOTIFIER", "1")
            .stdin(if stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn()?;
        if let (Some(text), Some(mut pipe)) = (stdin, child.stdin.take()) {
            // A key or a ruleset: far less than a pipe holds, so writing it
            // all before reading can't stall. Dropping the pipe closes it. A
            // program that stopped before reading it says why, and its exit
            // code is the answer.
            let _ = pipe.write_all(text.as_bytes());
        }
        let output = child.wait_with_output()?;
        Ok(Ran {
            ok: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    fn temp_root(&self) -> PathBuf {
        std::env::temp_dir()
    }

    fn fetch_private(&mut self, project: &Project) -> Result<String, String> {
        let rules = project.part(&project.config.private_assets, "private_assets")?;
        crate::private_assets::run(
            rules,
            project.config.command(),
            &project.root,
            &rules.url,
            crate::private_assets::Mode::Fetch { library: false },
        )
    }
}

/// A temporary folder, removed with everything in it when dropped: after
/// a success, an error or a panic.
#[derive(Debug)]
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// A new empty folder in `root`, named from `what`.
    pub fn new(root: &Path, what: &str) -> Result<Self, String> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos());
        let path = root.join(format!("shuyi-repos-{what}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path)
            .map_err(|e| format!("can't make a temporary folder {}: {e}", path.display()))?;
        Ok(Self { path })
    }

    /// Its path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
