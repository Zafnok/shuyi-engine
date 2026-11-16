//! `cargo xtask trim-targets`: frees the disk of this repository's idle
//! worktrees' `target/`, over `shuyi_cli::worktrees::trim` (ticket 2076,
//! ADR-0125), and its run at the end of a passing `cargo xtask gates`.

use std::path::Path;

use shuyi_cli::worktrees::trim::{self, Options};

/// The folders under `target/` whose profile folders lose their
/// `incremental` folder: the workspace's own, and the gates' one-off
/// builds (from before ticket 2074 turned their cache off).
const ENGINE_ROOTS: [&str; 4] = ["", "gates/games", "tutorial", "skill-examples*"];

/// The options of `trim-targets [--idle-days N] [--dry-run]
/// [--stale-hashes] [--merged]`, or `None` on any other argument.
pub fn parse(args: &[String]) -> Option<Options> {
    let mut options = Options::new(ENGINE_ROOTS.map(str::to_owned).to_vec());
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--idle-days" => options.idle_days = rest.next()?.parse().ok()?,
            "--dry-run" => options.dry_run = true,
            "--stale-hashes" => options.stale_hashes = true,
            "--merged" => options.merged = true,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| (*w).to_owned()).collect()
    }

    #[test]
    fn the_flags_set_the_options() {
        let defaults = parse(&[]).unwrap();
        assert_eq!(defaults.idle_days, trim::DEFAULT_IDLE_DAYS);
        assert!(!defaults.dry_run && !defaults.stale_hashes && !defaults.merged);
        assert_eq!(defaults.roots, ENGINE_ROOTS);
        let all = parse(&args(&[
            "--idle-days",
            "7",
            "--dry-run",
            "--stale-hashes",
            "--merged",
        ]))
        .unwrap();
        assert_eq!(all.idle_days, 7);
        assert!(all.dry_run && all.stale_hashes && all.merged);
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
