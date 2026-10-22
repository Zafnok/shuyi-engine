//! `shuyi mcp install`: registers the server with the agents it finds, in
//! each one's project file of MCP servers, in the project's folder (or
//! the working folder outside one). It shows what changes and asks before
//! writing, and keeps each file it changes as `<file>.bak`. `--dry-run`
//! writes nothing and prints each file as it would write it.
//!
//! | Agent | File | Found by |
//! | ----- | ---- | -------- |
//! | Claude Code | `.mcp.json` | always |
//! | Cursor | `.cursor/mcp.json` | `.cursor/` here or in the home folder |
//! | VS Code | `.vscode/mcp.json` | `.vscode/` here, or `code` on `PATH` |
//! | Gemini CLI | `.gemini/settings.json` | `.gemini/` here or in the home folder |
//!
//! Claude Code's file is also what `claude --mcp-config <file>` reads.

use std::fmt::Write as _;
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::project::Project;

/// The server's name in each agent's file.
pub const NAME: &str = "shuyi";

/// An agent the server can be registered with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Agent {
    /// Its name, for people.
    pub name: &'static str,
    /// Its file of MCP servers, from the project's folder.
    pub file: &'static str,
    /// The key of the servers in it (`mcpServers`).
    pub key: &'static str,
    /// Whether an entry says its transport (`"type": "stdio"`).
    pub typed: bool,
    /// The folders, in the project or the home folder, that show it is
    /// used.
    pub folders: &'static [&'static str],
    /// The programs on `PATH` that show it is installed.
    pub programs: &'static [&'static str],
}

/// Every agent the server can be registered with, Claude Code first.
pub const AGENTS: [Agent; 4] = [
    Agent {
        name: "Claude Code",
        file: ".mcp.json",
        key: "mcpServers",
        typed: false,
        folders: &[],
        programs: &[],
    },
    Agent {
        name: "Cursor",
        file: ".cursor/mcp.json",
        key: "mcpServers",
        typed: false,
        folders: &[".cursor"],
        programs: &[],
    },
    Agent {
        name: "VS Code",
        file: ".vscode/mcp.json",
        key: "servers",
        typed: true,
        folders: &[".vscode"],
        programs: &["code"],
    },
    Agent {
        name: "Gemini CLI",
        file: ".gemini/settings.json",
        key: "mcpServers",
        typed: false,
        folders: &[".gemini"],
        programs: &[],
    },
];

/// Where agents are looked for on this machine.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Machine {
    /// The home folder.
    pub home: Option<PathBuf>,
    /// The folders on `PATH`.
    pub path: Vec<PathBuf>,
}

impl Machine {
    /// This machine's.
    pub fn this() -> Self {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from);
        let path = std::env::var_os("PATH")
            .map(|path| std::env::split_paths(&path).collect())
            .unwrap_or_default();
        Self { home, path }
    }

    /// Whether `program` is on `PATH` (as itself, or with an extension
    /// Windows runs).
    fn has_program(&self, program: &str) -> bool {
        self.path.iter().any(|dir| {
            ["", ".exe", ".cmd", ".bat"]
                .iter()
                .any(|ext| dir.join(format!("{program}{ext}")).is_file())
        })
    }
}

impl Agent {
    /// Whether it is used in `root` or on `machine`: Claude Code always.
    pub fn found(&self, root: &Path, machine: &Machine) -> bool {
        if self.folders.is_empty() && self.programs.is_empty() {
            return true;
        }
        let in_folder = self.folders.iter().any(|folder| {
            root.join(folder).is_dir()
                || machine
                    .home
                    .as_ref()
                    .is_some_and(|home| home.join(folder).is_dir())
        });
        in_folder || self.programs.iter().any(|p| machine.has_program(p))
    }

    /// Its entry for the server, run as `command`'s words and `mcp`.
    pub fn entry(&self, command: &str) -> Value {
        let mut words = command.split_whitespace().map(str::to_owned);
        let program = words.next().unwrap_or_else(|| NAME.to_owned());
        let mut args: Vec<String> = words.collect();
        args.push("mcp".to_owned());
        let mut entry = Map::new();
        if self.typed {
            entry.insert("type".to_owned(), json!("stdio"));
        }
        entry.insert("command".to_owned(), json!(program));
        entry.insert("args".to_owned(), json!(args));
        Value::Object(entry)
    }

    /// Its file with the server's entry for `command` set, from the file
    /// as it is (`old`, if there is one), everything else in it kept; or
    /// why it can't be (not a JSON object).
    pub fn file_text(&self, old: Option<&str>, command: &str) -> Result<String, String> {
        let mut file = match old.map(str::trim) {
            None | Some("") => Map::new(),
            Some(text) => match serde_json::from_str::<Value>(text) {
                Ok(Value::Object(file)) => file,
                Ok(_) => return Err(format!("{}: not a JSON object", self.file)),
                Err(e) => return Err(format!("{}: not JSON ({e})", self.file)),
            },
        };
        let servers = file
            .entry(self.key.to_owned())
            .or_insert_with(|| Value::Object(Map::new()));
        let Value::Object(servers) = servers else {
            return Err(format!("{}: `{}` is not an object", self.file, self.key));
        };
        servers.insert(NAME.to_owned(), self.entry(command));
        let mut text = serde_json::to_string_pretty(&Value::Object(file)).unwrap_or_default();
        text.push('\n');
        Ok(text)
    }
}

/// One file to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    /// The agent's name.
    pub agent: &'static str,
    /// The file.
    pub path: PathBuf,
    /// Its text now, if it is there.
    pub old: Option<String>,
    /// Its text with the server registered.
    pub new: String,
}

/// The file of each agent found in `root` or on `machine`, with the
/// server run as `command`'s words and `mcp`.
pub fn plan(root: &Path, machine: &Machine, command: &str) -> Result<Vec<Planned>, String> {
    AGENTS
        .iter()
        .filter(|agent| agent.found(root, machine))
        .map(|agent| {
            let path = root.join(agent.file);
            let old = fs::read_to_string(&path).ok();
            let new = agent.file_text(old.as_deref(), command)?;
            Ok(Planned {
                agent: agent.name,
                path,
                old,
                new,
            })
        })
        .collect()
}

/// The lines of `old` and `new`, each marked: `  ` both have it, `- `
/// only `old`, `+ ` only `new` (a longest common subsequence).
pub fn diff(old: &str, new: &str) -> String {
    let (a, b): (Vec<&str>, Vec<&str>) = (old.lines().collect(), new.lines().collect());
    // common[i][j]: the longest common subsequence of a[i..] and b[j..].
    let mut common = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            common[i][j] = if a[i] == b[j] {
                common[i + 1][j + 1] + 1
            } else {
                common[i + 1][j].max(common[i][j + 1])
            };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, String::new());
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            let _ = writeln!(out, "  {}", a[i]);
            (i, j) = (i + 1, j + 1);
        } else if i < a.len() && common[i + 1][j] == common[i][j] {
            // Leaving a[i] out keeps the most lines shared.
            let _ = writeln!(out, "- {}", a[i]);
            i += 1;
        } else {
            let _ = writeln!(out, "+ {}", b[j]);
            j += 1;
        }
    }
    out
}

/// What `install` was asked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// Write nothing; print each file as it would be written.
    pub dry_run: bool,
    /// Don't ask before writing.
    pub yes: bool,
    /// How to run `shuyi`, if given.
    pub command: Option<String>,
}

/// Reads `install`'s arguments.
pub fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--dry-run" => options.dry_run = true,
            "--yes" => options.yes = true,
            "--command" => {
                let command = iter.next().ok_or("--command needs a value")?;
                if command.trim().is_empty() {
                    return Err("--command: the words that run shuyi, not nothing".to_owned());
                }
                options.command = Some(command.clone());
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(options)
}

/// Installs as `options` say in `root`, on `machine`, the server run as
/// `command`; `ask` is asked whether to write the files it is shown.
/// Returns what to print, or the error.
pub fn install(
    root: &Path,
    options: &Options,
    machine: &Machine,
    command: &str,
    ask: &mut dyn FnMut(&str) -> bool,
) -> Result<String, String> {
    let planned = plan(root, machine, command)?;
    let (same, changed): (Vec<Planned>, Vec<Planned>) = planned
        .into_iter()
        .partition(|p| p.old.as_deref() == Some(p.new.as_str()));
    let mut out = String::new();
    let shown = |p: &Planned| shown_path(root, &p.path);
    for p in &same {
        let _ = writeln!(
            out,
            "mcp install: {} ({}) has the server already",
            shown(p),
            p.agent
        );
    }
    if options.dry_run {
        for p in &changed {
            let _ = writeln!(out, "mcp install: would write {} ({}):", shown(p), p.agent);
            out.push_str(&p.new);
        }
        return Ok(out);
    }
    if changed.is_empty() {
        return Ok(out);
    }
    let mut question = String::new();
    for p in &changed {
        let _ = writeln!(question, "{} ({}):", shown(p), p.agent);
        question.push_str(&diff(p.old.as_deref().unwrap_or(""), &p.new));
    }
    let _ = write!(question, "write {} file(s)? [y/N] ", changed.len());
    if !options.yes && !ask(&question) {
        out.push_str("mcp install: nothing written\n");
        return Ok(out);
    }
    for p in &changed {
        if let Some(dir) = p.path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let mut wrote = format!("mcp install: wrote {} ({})", shown(p), p.agent);
        if p.old.is_some() {
            let mut backup = p.path.clone().into_os_string();
            backup.push(".bak");
            let backup = PathBuf::from(backup);
            fs::copy(&p.path, &backup).map_err(|e| format!("{}: {e}", backup.display()))?;
            let _ = write!(wrote, ", its old text in {}", shown_path(root, &backup));
        }
        fs::write(&p.path, &p.new).map_err(|e| format!("{}: {e}", p.path.display()))?;
        let _ = writeln!(out, "{wrote}");
    }
    Ok(out)
}

/// Whether the line `input` gives says yes (`y`, `Y` or `yes`).
pub fn said_yes(input: &mut impl BufRead) -> bool {
    let mut answer = String::new();
    input.read_line(&mut answer).is_ok() && matches!(answer.trim(), "y" | "Y" | "yes")
}

/// `path` from `root`, for people.
fn shown_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// `shuyi mcp install` with `args` in the working folder `cwd`: in the
/// project's folder (or `cwd` outside one), the server run as
/// `--command`, or the project's `command`, or `shuyi`. Returns the exit
/// code.
pub fn main(args: &[String], cwd: &Path) -> u8 {
    let options = match parse_args(args) {
        Ok(options) => options,
        Err(e) => {
            eprintln!("mcp install: {e}\n\n{}", super::usage("shuyi"));
            return 2;
        }
    };
    let project = Project::find(cwd).ok();
    let root = project.as_ref().map_or(cwd, |p| p.root.as_path());
    let command = options.command.clone().unwrap_or_else(|| {
        project
            .as_ref()
            .map_or("shuyi", |p| p.config.command())
            .to_owned()
    });
    let mut ask = |question: &str| {
        print!("{question}");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        said_yes(&mut std::io::stdin().lock())
    };
    match install(root, &options, &Machine::this(), &command, &mut ask) {
        Ok(out) => {
            print!("{out}");
            0
        }
        Err(e) => {
            eprintln!("mcp install: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests;
