//! `doctor`: checks the toolchain a game on the engine builds with: Rust
//! and Cargo, rustup's targets, MSYS2's linker on the Windows GNU
//! toolchain (`AGENTS.md`), on Linux the C linker, `pkg-config` and the
//! system libraries the backend links and opens, and the web target.
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
pub struct DoctorInput {}

/// One check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Check {
    /// What it checks (`rust`, `wasm-target`…).
    pub name: String,
    /// Whether it passed.
    pub ok: bool,
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
}

/// A check that passed, having found `found`.
fn pass(name: &str, found: impl Into<String>) -> Check {
    Check {
        name: name.to_owned(),
        ok: true,
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
        found,
        help: Some(help.into()),
    }
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
    DoctorOutput { host, checks }
}

/// `doctor` on the machine `probe` looks at.
pub fn doctor(probe: &dyn Probe, _input: &DoctorInput) -> Answer<DoctorOutput> {
    let output = checks(probe);
    let ok = output.checks.iter().all(|c| c.ok);
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

/// What `doctor` found, for people.
pub fn text(output: &DoctorOutput) -> String {
    let mut out = format!(
        "doctor: host {}\n",
        output.host.as_deref().unwrap_or("unknown")
    );
    for check in &output.checks {
        let mark = if check.ok { "ok  " } else { "FAIL" };
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
    }

    impl Fake {
        pub(super) fn with(mut self, command: &str, output: &str) -> Self {
            self.programs.insert(command.to_owned(), output.to_owned());
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
    }

    pub(super) fn rust(host: &str) -> Fake {
        Fake::default()
            .with(
                "rustc -vV",
                &format!("rustc 1.98.1 (abc 2026-09-01)\nbinary: rustc\nhost: {host}\n"),
            )
            .with("cargo --version", "cargo 1.98.1 (def 2026-09-01)\n")
    }

    fn names(output: &DoctorOutput, ok: bool) -> Vec<&str> {
        output
            .checks
            .iter()
            .filter(|c| c.ok == ok)
            .map(|c| c.name.as_str())
            .collect()
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
        assert_eq!(names(&output, false), ["msys2-gcc", "wasm-target"]);
        let msys = &output.checks[3];
        assert!(msys.help.as_deref().unwrap().contains("MSYS2.MSYS2"));
        let wasm = output.checks.last().unwrap();
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
        assert_eq!(checks(&rust("x86_64-pc-windows-msvc")).checks.len(), 4);
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
