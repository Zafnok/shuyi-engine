//! `cargo xtask trim-targets`: frees the disk of this repository's idle
//! worktrees' `target/`, over `shuyi_cli::worktrees::trim` (ticket 2076,
//! ADR-0121), and its run at the end of a passing `cargo xtask gates`:
//! waited for after a full run, started in the background once a day after
//! `--fast` (ticket 3545, ADR-0167).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use shuyi_cli::worktrees::trim::{self, Options};

/// The folders under `target/` whose profile folders lose their
/// `incremental` folder: the workspace's own, and the gates' one-off
/// builds (from before ticket 2074 turned their cache off).
const ENGINE_ROOTS: [&str; 4] = ["", "gates/games", "tutorial", "skill-examples*"];

/// The options of `trim-targets [--idle-days N] [--dry-run] [--merged]
/// [--stale-hashes]`, or `None` on any other argument.
pub fn parse(args: &[String]) -> Option<Options> {
    let mut options = Options::new(ENGINE_ROOTS.map(str::to_owned).to_vec());
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--idle-days" => options.idle_days = rest.next()?.parse().ok()?,
            "--dry-run" => options.dry_run = true,
            "--merged" => options.merged = true,
            "--stale-hashes" => options.stale_hashes = true,
            _ => return None,
        }
    }
    Some(options)
}

/// Whether the gates trim at their end: not on CI (`ci`, the `CI`
/// variable) and not with `SHUYI_NO_TRIM` set to anything but `0` or empty.
pub fn wanted_after_gates(ci: Option<&std::ffi::OsStr>, no_trim: Option<&std::ffi::OsStr>) -> bool {
    ci.is_none() && no_trim.is_none_or(|v| v.is_empty() || v == "0")
}

/// The trim with its defaults, at the end of a passing full run of the
/// gates: one line on stderr, and never a failure.
pub fn after_gates(root: &Path) {
    let ci = std::env::var_os("CI");
    let no_trim = std::env::var_os("SHUYI_NO_TRIM");
    if !wanted_after_gates(ci.as_deref(), no_trim.as_deref()) {
        return;
    }
    let options = Options::new(ENGINE_ROOTS.map(str::to_owned).to_vec());
    match trim::run(root, &options, &mut std::io::sink()) {
        Ok(report) => eprintln!("== {}", report.summary()),
        Err(e) => eprintln!("warning: trim-targets did not run ({e}); the gates passed"),
    }
}

/// How often a passing `--fast` run starts the trim: at most once a day.
const TRIM_EVERY: Duration = Duration::from_hours(24);

/// Whether the trim is due, its stamp last written at `stamped` (`None`
/// when there is none), at `now`: a stamp in the future (a clock moved
/// back) counts as due, so it can't hold the trim off.
fn due(stamped: Option<SystemTime>, now: SystemTime) -> bool {
    stamped.is_none_or(|stamped| {
        now.duration_since(stamped)
            .map_or(true, |age| age >= TRIM_EVERY)
    })
}

/// The trim with its defaults, at the end of a passing `--fast` run (the
/// pre-push hook's, ticket 3545, ADR-0167): at most once a day, by the
/// stamp `shuyi-trim/stamp` under the repository's common `.git` folder,
/// and in the background, never waited for. On Nick's machine it takes
/// most of a minute over about a hundred worktrees. One line on stderr,
/// and never a failure.
pub fn after_fast_gates(root: &Path) {
    let ci = std::env::var_os("CI");
    let no_trim = std::env::var_os("SHUYI_NO_TRIM");
    if !wanted_after_gates(ci.as_deref(), no_trim.as_deref()) {
        return;
    }
    match start_daily(root) {
        Ok(Some(log)) => eprintln!(
            "== trim-targets: started in the background, at most once a day; its log: {}",
            log.display()
        ),
        Ok(None) => eprintln!("== trim-targets: ran in the last day, not started"),
        Err(e) => eprintln!("warning: trim-targets did not start ({e}); the gates passed"),
    }
}

/// Starts `trim-targets` in the background when its stamp is a day old,
/// and gives its log, or `None` when it isn't due.
///
/// It runs a copy of this program, so that a build of xtask in this
/// worktree can replace the program meanwhile (Windows refuses to while it
/// runs), with no handle to this terminal or the hook's pipes, so that git
/// and the push don't wait for it.
fn start_daily(root: &Path) -> Result<Option<PathBuf>, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !output.status.success() {
        return Err("git rev-parse --git-common-dir failed".to_owned());
    }
    let common = root.join(String::from_utf8_lossy(&output.stdout).trim());
    let dir = common.join("shuyi-trim");
    let stamp = dir.join("stamp");
    let stamped = std::fs::metadata(&stamp).and_then(|m| m.modified()).ok();
    if !due(stamped, SystemTime::now()) {
        return Ok(None);
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let program = dir.join(format!("xtask-trim{}", std::env::consts::EXE_SUFFIX));
    let this = std::env::current_exe().map_err(|e| e.to_string())?;
    // A copy still running holds its file: the trim is under way already.
    std::fs::copy(&this, &program).map_err(|e| format!("{}: {e}", program.display()))?;
    let log = dir.join("last.log");
    start_detached(root, &program, &log)?;
    std::fs::write(&stamp, b"").map_err(|e| format!("{}: {e}", stamp.display()))?;
    Ok(Some(log))
}

/// Starts `program trim-targets --log <log>` in `root`, detached: nothing
/// of this process's handles (the terminal, the hook's or a harness's
/// pipes) passes to it, so whatever reads this process's output doesn't
/// wait for it, and the terminal's Ctrl+C doesn't stop it.
///
/// On Windows a child inherits every inheritable handle of its parent,
/// whatever its own stdio, and xtask forbids the `unsafe` that would turn
/// that off: `Start-Process` starts it instead (through `ShellExecute`,
/// which passes no handle), from a PowerShell that exits at once.
/// Elsewhere it is a process group of its own, with no stdio.
fn start_detached(root: &Path, program: &Path, log: &Path) -> Result<(), String> {
    let program_text = program.display().to_string();
    let log_text = log.display().to_string();
    let mut command;
    if cfg!(windows) {
        command = Command::new("powershell");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!(
                "Start-Process -WindowStyle Hidden -FilePath {} -ArgumentList {}",
                powershell_quoted(&program_text),
                powershell_quoted(&format!("trim-targets --log \"{log_text}\"")),
            ),
        ]);
    } else {
        command = Command::new(program);
        command.args(["trim-targets", "--log", &log_text]);
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(&mut command, 0);
    }
    command
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if cfg!(windows) {
        let status = command.status().map_err(|e| format!("powershell: {e}"))?;
        if !status.success() {
            return Err(format!("powershell Start-Process: {status}"));
        }
    } else {
        command
            .spawn()
            .map_err(|e| format!("{}: {e}", program.display()))?;
    }
    Ok(())
}

/// `text` as one single-quoted word of PowerShell.
fn powershell_quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// `trim-targets`' own `--log <file>` taken out of `args`: the file its
/// lines go to instead of stdout (the background trim's, ADR-0167), and the
/// rest; `None` when `--log` has no file.
pub fn split_log(args: &[String]) -> Option<(Option<PathBuf>, Vec<String>)> {
    let mut log = None;
    let mut rest = Vec::new();
    let mut words = args.iter();
    while let Some(word) = words.next() {
        if word == "--log" {
            log = Some(PathBuf::from(words.next()?));
        } else {
            rest.push(word.clone());
        }
    }
    Some((log, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_background_trim_logs_to_its_file() {
        let words = |w: &[&str]| w.iter().map(|w| (*w).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            split_log(&words(&["--log", "a b.log", "--dry-run"])),
            Some((Some(PathBuf::from("a b.log")), words(&["--dry-run"])))
        );
        assert_eq!(
            split_log(&words(&["--merged"])),
            Some((None, words(&["--merged"])))
        );
        assert_eq!(split_log(&words(&["--log"])), None);
        assert_eq!(powershell_quoted("D:/it's/x.exe"), "'D:/it''s/x.exe'");
    }

    #[test]
    fn the_trim_after_fast_runs_is_due_once_a_day() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        assert!(due(None, now), "never stamped");
        assert!(!due(Some(now), now));
        assert!(!due(Some(now - TRIM_EVERY + Duration::from_secs(1)), now));
        assert!(due(Some(now - TRIM_EVERY), now));
        assert!(due(Some(now - 3 * TRIM_EVERY), now));
        assert!(
            due(Some(now + Duration::from_secs(60)), now),
            "a stamp in the future"
        );
    }

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| (*w).to_owned()).collect()
    }

    #[test]
    fn the_flags_set_the_options() {
        let defaults = parse(&[]).unwrap();
        assert_eq!(defaults.idle_days, trim::DEFAULT_IDLE_DAYS);
        assert!(!defaults.dry_run && !defaults.merged && !defaults.stale_hashes);
        assert_eq!(defaults.roots, ENGINE_ROOTS);
        let all = parse(&args(&[
            "--idle-days",
            "7",
            "--dry-run",
            "--merged",
            "--stale-hashes",
        ]))
        .unwrap();
        assert_eq!(all.idle_days, 7);
        assert!(all.dry_run && all.merged && all.stale_hashes);
        assert_eq!(parse(&args(&["--idle-days"])), None);
        assert_eq!(parse(&args(&["--idle-days", "three"])), None);
        assert_eq!(parse(&args(&["--force"])), None);
    }

    #[test]
    fn the_gates_trim_locally_unless_told_not_to() {
        assert!(wanted_after_gates(None, None));
        assert!(wanted_after_gates(None, Some("0".as_ref())));
        assert!(wanted_after_gates(None, Some("".as_ref())));
        assert!(!wanted_after_gates(None, Some("1".as_ref())));
        assert!(!wanted_after_gates(Some("true".as_ref()), None));
        assert!(
            !wanted_after_gates(Some("".as_ref()), None),
            "CI set, even empty"
        );
    }
}
