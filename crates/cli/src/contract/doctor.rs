//! `doctor`: checks the toolchain a game on the engine builds with: Rust
//! and Cargo, rustup's targets, MSYS2's linker and `objcopy` on the
//! Windows GNU toolchain (`AGENTS.md`), on Linux the C linker, `pkg-config` and the
//! system libraries the backend links and opens, and the web target; and,
//! as optional checks that never fail it, `gh`, which only `shuyi repos`
//! needs, `ssh-keygen`, which it and `export --recipient` need, and, in a
//! git checkout, the disk its worktrees take (ticket 3402): whether git's
//! hooks are set to its `scripts/git-hooks`, and on Windows whether its
//! `target/` carries NTFS compression, which the `post-checkout` hook
//! gives a new worktree. `doctor` only reads that attribute: it never
//! compresses anything.
//! Each check says what it found and, if it failed, how to fix it (its
//! help): on Linux, the install command for the distribution's package
//! manager (`/etc/os-release`), and `scripts/setup-linux.sh`, which
//! installs the same list. It passes when
//! every check does; when one fails, it answers [`codes::TOOLCHAIN`] and
//! exits 3.
//!
//! What it runs and looks at goes through a [`Probe`], so its tests can
//! give it any machine.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::Command;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Answer, Op, codes};

pub(crate) mod linux;

/// The web build's target.
pub const WASM_TARGET: &str = "wasm32-unknown-unknown";

/// Where MSYS2 puts the mingw-w64 GCC that links on the Windows GNU
/// toolchain (the engine's `.cargo/config.toml` names it as the linker).
pub const MSYS2_GCC: &str = "C:/msys64/mingw64/bin/gcc.exe";

/// What `doctor` checks: everything (nothing to choose yet).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An input (ADR-0039, kind 4): built with `default()` and its fields
// assigned, it may grow.
#[non_exhaustive]
pub struct DoctorInput {}

/// One check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Check {
    /// What it checks (`rust`, `wasm-target`…).
    pub name: String,
    /// Whether it passed.
    pub ok: bool,
    /// Whether it is optional: a tool only one command needs, whose
    /// check failing doesn't fail `doctor`.
    #[serde(default)]
    pub optional: bool,
    /// What it found, if anything.
    pub found: Option<String>,
    /// How to fix it, if it failed.
    pub help: Option<String>,
}

/// What `doctor` found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DoctorOutput {
    /// The Rust host's target triple, if Rust was found.
    pub host: Option<String>,
    /// Every check, in order.
    pub checks: Vec<Check>,
}

/// How `doctor` looks at the machine.
pub trait Probe {
    /// What `program` with `args` prints on stdout, if it runs and
    /// succeeds.
    fn output(&self, program: &str, args: &[&str]) -> Option<String>;

    /// Whether the file at `path` exists.
    fn exists(&self, path: &Path) -> bool;

    /// The text of the file at `path`, if it can be read.
    fn read(&self, path: &Path) -> Option<String>;

    /// Whether `program` with `args` starts, whatever it then exits with:
    /// for a program with no flag that succeeds, as `ssh-keygen`. By
    /// default, whether it runs and succeeds ([`Probe::output`]).
    fn starts(&self, program: &str, args: &[&str]) -> bool {
        self.output(program, args).is_some()
    }

    /// Whether `path` is a folder. By default, never.
    fn is_folder(&self, path: &Path) -> bool {
        let _ = path;
        false
    }

    /// Whether the folder at `path` carries NTFS compression (Windows'
    /// `FILE_ATTRIBUTE_COMPRESSED`), or `None` when it can't be read or
    /// the system has no such attribute. By default, `None`.
    fn compressed(&self, path: &Path) -> Option<bool> {
        let _ = path;
        None
    }
}

/// The machine `doctor` runs on.
#[derive(Debug, Clone, Copy, Default)]
pub struct ThisMachine;

impl Probe for ThisMachine {
    fn output(&self, program: &str, args: &[&str]) -> Option<String> {
        let out = Command::new(program).args(args).output().ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    }

    fn exists(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn read(&self, path: &Path) -> Option<String> {
        fs::read_to_string(path).ok()
    }

    fn starts(&self, program: &str, args: &[&str]) -> bool {
        Command::new(program).args(args).output().is_ok()
    }

    fn is_folder(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn compressed(&self, path: &Path) -> Option<bool> {
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt as _;
            /// `FILE_ATTRIBUTE_COMPRESSED`, as Windows reports a file's
            /// attributes.
            const FILE_ATTRIBUTE_COMPRESSED: u32 = 0x800;
            fs::metadata(path)
                .ok()
                .map(|m| m.file_attributes() & FILE_ATTRIBUTE_COMPRESSED != 0)
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            None
        }
    }
}

/// A check that passed, having found `found`.
fn pass(name: &str, found: impl Into<String>) -> Check {
    Check {
        name: name.to_owned(),
        ok: true,
        optional: false,
        found: Some(found.into()),
        help: None,
    }
}

/// A check that failed: `found` (if anything) and how to fix it
/// (`help`).
fn fail(name: &str, found: Option<String>, help: impl Into<String>) -> Check {
    Check {
        name: name.to_owned(),
        ok: false,
        optional: false,
        found,
        help: Some(help.into()),
    }
}

/// `check`, made optional.
fn optional(check: Check) -> Check {
    Check {
        optional: true,
        ..check
    }
}

/// What the optional checks' help says first.
const ONLY_REPOS: &str = "optional: only `shuyi repos` needs it, to put the game on GitHub";

/// The optional checks: `gh`, which only `shuyi repos` needs, and
/// `ssh-keygen`, which it and `export --recipient` need.
fn repos_checks(probe: &dyn Probe) -> [Check; 2] {
    let gh = match probe.output("gh", &["--version"]) {
        Some(text) => pass("gh", first_line(&text)),
        None => fail(
            "gh",
            None,
            format!(
                "{ONLY_REPOS}. Install GitHub's CLI from https://cli.github.com (on Windows \
                 `winget install -e --id GitHub.cli`), then `gh auth login`"
            ),
        ),
    };
    let ssh_keygen = if probe.starts("ssh-keygen", &["-?"]) {
        pass("ssh-keygen", "on PATH")
    } else {
        fail(
            "ssh-keygen",
            None,
            "optional: only `shuyi repos` needs it, to make a game's deploy keys, and `shuyi \
             export --recipient`, to sign its ledger. It comes with OpenSSH: on Windows the \
             optional feature OpenSSH Client, or Git for Windows' usr/bin on PATH; on Linux the \
             package openssh-client"
                .to_owned(),
        )
    };
    [optional(gh), optional(ssh_keygen)]
}

/// The game's folder of git hooks, from its root (`shuyi repos` sets
/// `core.hooksPath` to it).
const HOOKS: &str = "scripts/git-hooks";

/// What a worktree check's help says first.
const ONLY_DISK: &str = "optional: it only saves disk, in each worktree";

/// The optional checks of the git checkout `doctor` runs in, if any: its
/// hooks, when it has [`HOOKS`], and on Windows its `target/`'s
/// compression.
fn worktree_checks(probe: &dyn Probe, windows: bool) -> Vec<Check> {
    let Some(top) = probe.output("git", &["rev-parse", "--show-toplevel"]) else {
        return Vec::new();
    };
    let root = Path::new(top.trim());
    let mut checks = Vec::new();
    if probe.is_folder(&root.join(HOOKS)) {
        let git_dir = root.to_string_lossy();
        let set = probe
            .output(
                "git",
                &["-C", &git_dir, "config", "--get", "core.hooksPath"],
            )
            .map(|path| path.trim().to_owned());
        checks.push(optional(match set {
            Some(path) if names_hooks(&path) => {
                pass("git-hooks", format!("core.hooksPath is {path}"))
            }
            found => fail(
                "git-hooks",
                found.map(|path| format!("core.hooksPath is {path}")),
                format!(
                    "{ONLY_DISK}: git runs {HOOKS}'s post-checkout, which compresses a new \
                     worktree's target/ on Windows, only once it is set. Set it once per clone, \
                     from the game's root: `git config core.hooksPath {HOOKS}`"
                ),
            ),
        }));
    }
    if windows {
        let target = root.join("target");
        let there = probe.is_folder(&target);
        let compact = "`compact /c /s:target /i /q` from the game's root (in Git Bash, \
                       `MSYS2_ARG_CONV_EXCL='*' compact.exe /c /s:target /i /q`)";
        let check = match (there, probe.compressed(&target)) {
            (true, Some(true)) => Some(pass("target-compression", "target/ is compressed")),
            (true, Some(false)) => Some(fail(
                "target-compression",
                Some("target/ is not compressed".to_owned()),
                format!(
                    "{ONLY_DISK}: NTFS compression takes about half a target/'s disk, at no \
                     measured cost in build time. {compact} compresses it, and every file a build \
                     writes there after"
                ),
            )),
            (false, _) => Some(fail(
                "target-compression",
                Some("no target/ yet".to_owned()),
                format!(
                    "{ONLY_DISK}: `mkdir target`, then {compact}, before the first build so \
                     that it compresses every file the build writes there"
                ),
            )),
            // An attribute it can't read: nothing to say.
            (true, None) => None,
        };
        checks.extend(check.map(optional));
    }
    checks
}

/// Whether a `core.hooksPath` names [`HOOKS`]: as it is, or by a path
/// ending in it (a worktree shares its main checkout's setting, which may
/// be that checkout's folder in full).
fn names_hooks(path: &str) -> bool {
    let path = path.replace('\\', "/");
    let path = path.trim_end_matches('/');
    path == HOOKS || path.ends_with(&format!("/{HOOKS}"))
}

/// The first line of `text`, trimmed.
fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or_default().trim().to_owned()
}

/// The `host:` line of `rustc -vV`'s output.
fn host_of(verbose: &str) -> Option<String> {
    verbose
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(|host| host.trim().to_owned())
}

/// `doctor`'s checks of the machine `probe` looks at.
pub fn checks(probe: &dyn Probe) -> DoctorOutput {
    let mut checks = Vec::new();
    let rustc = probe.output("rustc", &["-vV"]);
    let host = rustc.as_deref().and_then(host_of);
    checks.push(match &rustc {
        Some(text) => pass("rust", first_line(text)),
        None => fail(
            "rust",
            None,
            "install Rust with rustup: https://rustup.rs (on Windows choose the GNU toolchain, \
             x86_64-pc-windows-gnu, if MSVC isn't installed)",
        ),
    });
    checks.push(match probe.output("cargo", &["--version"]) {
        Some(text) => pass("cargo", first_line(&text)),
        None => fail(
            "cargo",
            None,
            "install Rust with rustup, which brings cargo",
        ),
    });
    let targets = probe.output("rustup", &["target", "list", "--installed"]);
    checks.push(match &targets {
        Some(text) => pass(
            "rustup",
            format!(
                "targets: {}",
                text.split_whitespace().collect::<Vec<_>>().join(", ")
            ),
        ),
        None => fail(
            "rustup",
            None,
            "install rustup (https://rustup.rs): the targets are added with it",
        ),
    });
    if host.as_deref().is_some_and(|h| h.ends_with("windows-gnu")) {
        let msys = Path::new(MSYS2_GCC);
        checks.push(if probe.exists(msys) {
            pass("msys2-gcc", MSYS2_GCC)
        } else {
            match probe.output("gcc", &["--version"]) {
                Some(text) => pass("msys2-gcc", format!("gcc on PATH: {}", first_line(&text))),
                None => fail(
                    "msys2-gcc",
                    None,
                    "the GNU toolchain links with MSYS2's mingw-w64 GCC: `winget install -e --id \
                     MSYS2.MSYS2`, then `C:\\msys64\\usr\\bin\\pacman.exe -S \
                     mingw-w64-x86_64-gcc`, and put C:\\msys64\\mingw64\\bin on PATH",
                ),
            }
        });
    }
    if host.as_deref().is_some_and(|h| h.contains("-linux-")) {
        checks.extend(linux::checks(probe));
    }
    // `export --release` strips the package's copy of the program with
    // binutils' objcopy. On Linux the C linker's packages (the `cc` row)
    // bring binutils.
    if host.as_deref().is_some_and(|h| h.ends_with("windows-gnu")) {
        checks.push(match probe.output("objcopy", &["--version"]) {
            Some(text) => pass("objcopy", first_line(&text)),
            None => fail(
                "objcopy",
                None,
                "`export --release` strips the package's program with binutils' objcopy, which \
                 comes with MSYS2's mingw-w64 GCC: `C:\\msys64\\usr\\bin\\pacman.exe -S \
                 mingw-w64-x86_64-binutils`, with C:\\msys64\\mingw64\\bin on PATH",
            ),
        });
    }
    let has_wasm = targets
        .as_deref()
        .is_some_and(|t| t.split_whitespace().any(|line| line == WASM_TARGET));
    checks.push(if has_wasm {
        pass("wasm-target", WASM_TARGET)
    } else {
        fail(
            "wasm-target",
            None,
            format!("for the web build: `rustup target add {WASM_TARGET}`"),
        )
    });
    checks.extend(repos_checks(probe));
    let windows = host.as_deref().is_some_and(|h| h.contains("-windows-"));
    checks.extend(worktree_checks(probe, windows));
    DoctorOutput { host, checks }
}

/// `doctor` on the machine `probe` looks at.
pub fn doctor(probe: &dyn Probe, _input: &DoctorInput) -> Answer<DoctorOutput> {
    let output = checks(probe);
    let ok = output.checks.iter().all(|c| c.ok || c.optional);
    let mut limits = Vec::new();
    if output.host.is_none() {
        limits.push(
            "without rustc the host is unknown: MSYS2 and the Linux libraries were not checked"
                .to_owned(),
        );
    }
    let answer = Answer::new(Op::Doctor, ok, output, limits);
    if ok {
        answer
    } else {
        answer
            .with_code(codes::TOOLCHAIN)
            .with_help("fix each failed check as its help says, then run doctor again")
    }
}

/// `doctor`'s usage, with `command` as the tool's name.
pub fn usage(command: &str) -> String {
    format!("usage: {command} doctor [--json]")
}

/// What `doctor` found, for people.
pub fn text(output: &DoctorOutput) -> String {
    let mut out = format!(
        "doctor: host {}\n",
        output.host.as_deref().unwrap_or("unknown")
    );
    for check in &output.checks {
        let mark = match (check.ok, check.optional) {
            (true, _) => "ok  ",
            (false, true) => "--  ",
            (false, false) => "FAIL",
        };
        // Writing to a `String` can't fail.
        let _ = write!(out, "  {mark} {}", check.name);
        if let Some(found) = &check.found {
            let _ = write!(out, ": {found}");
        }
        out.push('\n');
        if let Some(help) = &check.help {
            let _ = writeln!(out, "       help: {help}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use super::*;

    /// A machine with these programs' outputs, these files and these
    /// files' texts.
    #[derive(Default)]
    pub(super) struct Fake {
        pub(super) programs: BTreeMap<String, String>,
        pub(super) files: Vec<PathBuf>,
        texts: BTreeMap<PathBuf, String>,
        /// Its folders, each compressed or not (`None`: unreadable).
        folders: BTreeMap<PathBuf, Option<bool>>,
    }

    impl Fake {
        pub(super) fn with(mut self, command: &str, output: &str) -> Self {
            self.programs.insert(command.to_owned(), output.to_owned());
            self
        }

        /// The machine, with a folder at `path`, `compressed` or not.
        fn folder(mut self, path: &str, compressed: Option<bool>) -> Self {
            self.folders.insert(PathBuf::from(path), compressed);
            self
        }

        /// The machine, with `text` in the file at `path`.
        pub(super) fn reading(mut self, path: &str, text: &str) -> Self {
            self.texts.insert(PathBuf::from(path), text.to_owned());
            self
        }
    }

    impl Probe for Fake {
        fn output(&self, program: &str, args: &[&str]) -> Option<String> {
            let command = std::iter::once(program)
                .chain(args.iter().copied())
                .collect::<Vec<_>>()
                .join(" ");
            self.programs.get(&command).cloned()
        }

        fn exists(&self, path: &Path) -> bool {
            self.files.iter().any(|f| f == path)
        }

        fn read(&self, path: &Path) -> Option<String> {
            self.texts.get(path).cloned()
        }

        fn is_folder(&self, path: &Path) -> bool {
            self.folders.contains_key(path)
        }

        fn compressed(&self, path: &Path) -> Option<bool> {
            self.folders.get(path).copied().flatten()
        }
    }

    pub(super) fn rust(host: &str) -> Fake {
        Fake::default()
            .with(
                "rustc -vV",
                &format!("rustc 1.98.1 (abc 2026-09-01)\nbinary: rustc\nhost: {host}\n"),
            )
            .with("cargo --version", "cargo 1.98.1 (def 2026-09-01)\n")
    }

    /// The names of the checks that are `ok`, or not, but the optional
    /// ones.
    fn names(output: &DoctorOutput, ok: bool) -> Vec<&str> {
        output
            .checks
            .iter()
            .filter(|c| c.ok == ok && !c.optional)
            .map(|c| c.name.as_str())
            .collect()
    }

    #[test]
    fn gh_and_ssh_keygen_are_optional_and_never_fail_doctor() {
        let targets = "x86_64-pc-windows-msvc\nwasm32-unknown-unknown\n";
        let machine =
            rust("x86_64-pc-windows-msvc").with("rustup target list --installed", targets);
        let answer = doctor(&machine, &DoctorInput::default());
        assert!(answer.ok, "{answer:?}");
        let output = answer.result.unwrap();
        let optional: Vec<(&str, bool)> = output
            .checks
            .iter()
            .filter(|c| c.optional)
            .map(|c| (c.name.as_str(), c.ok))
            .collect();
        assert_eq!(optional, [("gh", false), ("ssh-keygen", false)]);
        let said = text(&output);
        assert!(
            said.contains("  --   gh\n       help: optional: only `shuyi repos` needs it"),
            "{said}"
        );
        assert!(said.contains("https://cli.github.com"), "{said}");
        assert!(said.contains("openssh-client"), "{said}");
        // Installed, they pass.
        let with = machine
            .with(
                "gh --version",
                "gh version 2.103.0 (2026-10-21)\nhttps://github.com/cli\n",
            )
            .with("ssh-keygen -?", "");
        let found = checks(&with);
        let gh = found.checks.iter().find(|c| c.name == "gh").unwrap();
        assert_eq!(
            (gh.ok, gh.found.as_deref()),
            (true, Some("gh version 2.103.0 (2026-10-21)"))
        );
        assert!(found.checks.iter().all(|c| c.ok));
        // The machine's own ssh-keygen starts or not; a program that
        // isn't there never does.
        assert!(!ThisMachine.starts("no-such-program-anywhere", &[]));
        assert!(ThisMachine.starts("cargo", &["--no-such-flag"]));
    }

    /// A Windows GNU machine with all it needs, in a git checkout at
    /// `D:/games/moon`.
    fn in_a_checkout() -> Fake {
        let mut machine = rust("x86_64-pc-windows-gnu")
            .with(
                "rustup target list --installed",
                "x86_64-pc-windows-gnu
wasm32-unknown-unknown
",
            )
            .with(
                "objcopy --version",
                "GNU objcopy (GNU Binutils) 2.45
",
            )
            .with(
                "git rev-parse --show-toplevel",
                "D:/games/moon
",
            );
        machine.files.push(PathBuf::from(MSYS2_GCC));
        machine
    }

    /// The optional checks of the worktree: name, ok, found and help.
    fn worktree(machine: &Fake) -> Vec<(String, bool, Option<String>, Option<String>)> {
        let answer = doctor(machine, &DoctorInput::default());
        // Never counted in the verdict.
        assert!(answer.ok, "{answer:?}");
        assert_eq!(answer.exit_code(), 0);
        let output = answer.result.unwrap();
        output
            .checks
            .into_iter()
            .filter(|c| c.name == "git-hooks" || c.name == "target-compression")
            .inspect(|c| assert!(c.optional, "{c:?}"))
            .map(|c| (c.name, c.ok, c.found, c.help))
            .collect()
    }

    #[test]
    fn set_hooks_and_a_compressed_target_pass() {
        let machine = in_a_checkout()
            .folder("D:/games/moon/scripts/git-hooks", None)
            .folder("D:/games/moon/target", Some(true))
            .with(
                "git -C D:/games/moon config --get core.hooksPath",
                "scripts/git-hooks
",
            );
        assert_eq!(
            worktree(&machine),
            [
                (
                    "git-hooks".to_owned(),
                    true,
                    Some("core.hooksPath is scripts/git-hooks".to_owned()),
                    None
                ),
                (
                    "target-compression".to_owned(),
                    true,
                    Some("target/ is compressed".to_owned()),
                    None
                ),
            ]
        );
        // A worktree's setting, shared with its main checkout, may name
        // that checkout's folder in full.
        let shared = machine.with(
            "git -C D:/games/moon config --get core.hooksPath",
            "D:\\games\\moon-main\\scripts\\git-hooks\\\n",
        );
        assert!(worktree(&shared)[0].1);
        assert!(!names_hooks("scripts/git-hooks-old"));
        assert!(!names_hooks("myscripts/git-hooks/x"));
    }

    #[test]
    fn unset_hooks_and_an_uncompressed_target_each_give_their_help() {
        let machine = in_a_checkout()
            .folder("D:/games/moon/scripts/git-hooks", None)
            .folder("D:/games/moon/target", Some(false));
        let found = worktree(&machine);
        assert_eq!(found.len(), 2);
        let (name, ok, seen, help) = &found[0];
        assert_eq!((name.as_str(), *ok, seen), ("git-hooks", false, &None));
        assert!(
            help.as_deref()
                .unwrap()
                .ends_with("`git config core.hooksPath scripts/git-hooks`"),
            "{help:?}"
        );
        let (name, ok, seen, help) = &found[1];
        assert_eq!(
            (name.as_str(), *ok, seen.as_deref()),
            (
                "target-compression",
                false,
                Some("target/ is not compressed")
            )
        );
        assert!(
            help.as_deref()
                .unwrap()
                .contains("`compact /c /s:target /i /q`"),
            "{help:?}"
        );
        let said = text(&doctor(&machine, &DoctorInput::default()).result.unwrap());
        assert!(
            said.contains(
                "  --   git-hooks
       help: optional: it only saves disk"
            ),
            "{said}"
        );
        // Another hooks path of the developer's own is named.
        let other = machine.with(
            "git -C D:/games/moon config --get core.hooksPath",
            ".githooks
",
        );
        assert_eq!(
            worktree(&other)[0].2.as_deref(),
            Some("core.hooksPath is .githooks")
        );
        // No target/ yet: the help makes one first.
        let bare = in_a_checkout().folder("D:/games/moon/scripts/git-hooks", None);
        let found = worktree(&bare);
        assert_eq!(found[1].2.as_deref(), Some("no target/ yet"));
        assert!(
            found[1]
                .3
                .as_deref()
                .unwrap()
                .contains("`mkdir target`, then")
        );
    }

    #[test]
    fn the_worktree_checks_are_skipped_where_they_mean_nothing() {
        // Outside a git checkout: neither.
        let mut outside = in_a_checkout();
        outside.programs.remove("git rev-parse --show-toplevel");
        assert_eq!(worktree(&outside), []);
        // A checkout with no hooks folder: no hooks check.
        let no_hooks = in_a_checkout().folder("D:/games/moon/target", Some(true));
        let names: Vec<String> = worktree(&no_hooks).into_iter().map(|c| c.0).collect();
        assert_eq!(names, ["target-compression"]);
        // Off Windows: no compression check.
        let linux = rust("x86_64-unknown-linux-gnu")
            .with(
                "git rev-parse --show-toplevel",
                "/home/you/moon
",
            )
            .folder("/home/you/moon/scripts/git-hooks", None)
            .folder("/home/you/moon/target", Some(false));
        let names: Vec<String> = checks(&linux)
            .checks
            .into_iter()
            .filter(|c| c.optional)
            .map(|c| c.name)
            .collect();
        assert_eq!(names, ["gh", "ssh-keygen", "git-hooks"]);
        // An attribute it can't read says nothing.
        let unread = in_a_checkout().folder("D:/games/moon/target", None);
        assert_eq!(worktree(&unread), []);
    }

    #[test]
    fn this_machine_reads_a_folders_compression() {
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(ThisMachine.is_folder(here));
        assert!(!ThisMachine.is_folder(&here.join("Cargo.toml")));
        let missing = ThisMachine.compressed(&here.join("no-such-folder"));
        assert_eq!(missing, None);
        // On Windows the attribute is read; elsewhere there is none.
        assert_eq!(ThisMachine.compressed(here).is_some(), cfg!(windows));
    }

    #[test]
    fn a_complete_linux_toolchain_passes() {
        let mut machine = rust("x86_64-unknown-linux-gnu")
            .with(
                "rustup target list --installed",
                "x86_64-unknown-linux-gnu\nwasm32-unknown-unknown\n",
            )
            .with("cc --version", "cc (Ubuntu 13.3.0-6ubuntu2) 13.3.0\n")
            .with("pkg-config --version", "1.8.1\n")
            .with("pkg-config --modversion alsa", "1.2.11\n")
            .with("pkg-config --modversion libudev", "255\n")
            .reading("/etc/os-release", "ID=ubuntu\n");
        for soname in [
            "libX11.so.6",
            "libXi.so.6",
            "libxkbcommon.so.0",
            "libGL.so.1",
        ] {
            machine
                .files
                .push(PathBuf::from(format!("/usr/lib/x86_64-linux-gnu/{soname}")));
        }
        let answer = doctor(&machine, &DoctorInput::default());
        assert!(answer.ok, "{answer:?}");
        assert_eq!((answer.code.as_deref(), answer.exit_code()), (None, 0));
        let output = answer.result.unwrap();
        assert_eq!(output.host.as_deref(), Some("x86_64-unknown-linux-gnu"));
        assert_eq!(
            names(&output, true),
            [
                "rust",
                "cargo",
                "rustup",
                "cc",
                "pkg-config",
                "lib-alsa",
                "lib-udev",
                "lib-x11",
                "lib-xi",
                "lib-xkbcommon",
                "lib-gl",
                "wasm-target"
            ]
        );
        assert_eq!(
            output.checks[3].found.as_deref(),
            Some("cc (Ubuntu 13.3.0-6ubuntu2) 13.3.0")
        );
        assert_eq!(
            output.checks[0].found.as_deref(),
            Some("rustc 1.98.1 (abc 2026-09-01)")
        );
        assert_eq!(
            output.checks[2].found.as_deref(),
            Some("targets: x86_64-unknown-linux-gnu, wasm32-unknown-unknown")
        );
        assert!(text(&output).contains("  ok   wasm-target: wasm32-unknown-unknown\n"));
    }

    #[test]
    fn windows_gnu_needs_msys2s_gcc_and_says_how_to_get_it() {
        let targets = "x86_64-pc-windows-gnu\n";
        let machine = rust("x86_64-pc-windows-gnu").with("rustup target list --installed", targets);
        let output = checks(&machine);
        assert_eq!(
            names(&output, false),
            ["msys2-gcc", "objcopy", "wasm-target"]
        );
        let msys = &output.checks[3];
        assert!(msys.help.as_deref().unwrap().contains("MSYS2.MSYS2"));
        let objcopy = &output.checks[4];
        assert!(
            objcopy
                .help
                .as_deref()
                .unwrap()
                .contains("mingw-w64-x86_64-binutils")
        );
        let with = machine.with("objcopy --version", "GNU objcopy (GNU Binutils) 2.45\n");
        let check = &checks(&with).checks[4];
        assert_eq!(
            check.found.as_deref(),
            Some("GNU objcopy (GNU Binutils) 2.45")
        );
        let wasm = output
            .checks
            .iter()
            .find(|c| c.name == "wasm-target")
            .unwrap();
        assert_eq!(
            wasm.help.as_deref(),
            Some("for the web build: `rustup target add wasm32-unknown-unknown`")
        );
        let said = text(&output);
        assert!(
            said.contains("  FAIL msys2-gcc\n       help: the GNU toolchain"),
            "{said}"
        );
        // At MSYS2's place, or on PATH, it passes.
        let mut installed = rust("x86_64-pc-windows-gnu");
        installed.files.push(PathBuf::from(MSYS2_GCC));
        assert!(checks(&installed).checks[3].ok);
        let on_path = rust("x86_64-pc-windows-gnu").with("gcc --version", "gcc (Rev1) 14.2.0\n");
        let check = &checks(&on_path).checks[3];
        assert_eq!(
            check.found.as_deref(),
            Some("gcc on PATH: gcc (Rev1) 14.2.0")
        );
    }

    #[test]
    fn without_rust_every_check_says_how_to_install_it() {
        let answer = doctor(&Fake::default(), &DoctorInput::default());
        assert!(!answer.ok);
        assert_eq!(answer.code.as_deref(), Some("SH1003"));
        assert_eq!(answer.exit_code(), 3);
        assert!(
            answer
                .help
                .as_deref()
                .unwrap()
                .contains("then run doctor again")
        );
        let said = answer.to_text(text);
        assert!(
            said.contains("doctor: error[SH1003]: A piece of the toolchain is missing\n"),
            "{said}"
        );
        let output = answer.result.as_ref().unwrap();
        assert_eq!(output.host, None);
        assert_eq!(
            names(output, false),
            ["rust", "cargo", "rustup", "wasm-target"]
        );
        assert!(output.checks.iter().all(|c| c.help.is_some()));
        assert_eq!(
            answer.limits,
            ["without rustc the host is unknown: MSYS2 and the Linux libraries were not checked"]
        );
        assert!(text(output).starts_with("doctor: host unknown\n"));
        // Another host's MSVC toolchain needs no MSYS2.
        let msvc = checks(&rust("x86_64-pc-windows-msvc"));
        assert_eq!(msvc.checks.iter().filter(|c| !c.optional).count(), 4);
    }

    #[test]
    fn this_machine_runs_programs_and_finds_files() {
        let cargo = ThisMachine.output("cargo", &["--version"]).unwrap();
        assert!(cargo.starts_with("cargo "), "{cargo}");
        assert_eq!(ThisMachine.output("cargo", &["--no-such-flag"]), None);
        assert_eq!(ThisMachine.output("no-such-program-anywhere", &[]), None);
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        assert!(ThisMachine.exists(&manifest));
        assert!(!ThisMachine.exists(&manifest.with_file_name("Nope.toml")));
        assert!(!ThisMachine.exists(Path::new(env!("CARGO_MANIFEST_DIR"))));
        let text = ThisMachine.read(&manifest).unwrap();
        assert!(text.contains("name = \"shuyi-cli\""), "{text}");
        assert_eq!(
            ThisMachine.read(&manifest.with_file_name("Nope.toml")),
            None
        );
    }
}
