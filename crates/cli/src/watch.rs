//! `run --watch`: the game's window program, in a debug build (so with its
//! debug tools on), with the folder of its files on disk laid over the
//! embedded ones and watched (ADR-0071). It isn't an operation of the
//! contract: it plays nothing headless and answers nothing, and it runs
//! until the player closes the window.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use shuyi_platform::WATCH_VAR;

use crate::contract::codes;
use crate::export::package_and_program;
use crate::project::{self, Project};

/// The folder of a game's files when its project file doesn't say.
const DEFAULT_ASSETS: &str = "assets";

/// What `run --watch` starts.
#[derive(Debug, PartialEq, Eq)]
struct Plan {
    /// `cargo`'s arguments.
    args: Vec<String>,
    /// The folder of the game's files.
    assets: PathBuf,
}

/// What runs `bin` of `package` over the folder `assets` of the project at
/// `root`, or why not.
fn plan(root: &Path, assets: Option<&str>, package: &str, bin: &str) -> Result<Plan, String> {
    let named = assets.unwrap_or(DEFAULT_ASSETS);
    let assets = root.join(named);
    if !assets.is_dir() {
        return Err(format!(
            "no folder {named} here: `assets` in {} names the folder of the game's files",
            project::FILE
        ));
    }
    Ok(Plan {
        args: ["run", "-p", package, "--bin", bin]
            .map(str::to_owned)
            .to_vec(),
        assets: std::path::absolute(&assets).unwrap_or(assets),
    })
}

/// `run --watch` on `project`, `args` all it was given. Returns the exit
/// code: the game's, as 0 or 1; 2 for another argument, 3 for a project that
/// doesn't say what to run.
pub fn main(project: &Project, args: &[String]) -> u8 {
    let command = project.config.command();
    if args.iter().any(|arg| arg != "--watch") {
        let error = "--watch takes nothing else: it runs the game in its window";
        let help = format!("usage: {command} run --watch");
        eprint!(
            "{}",
            codes::said("run", codes::WRONG_ARGUMENTS.0, error, Some(&help))
        );
        return 2;
    }
    let config = &project.config;
    let planned = package_and_program(&project.root, config)
        .map_err(|e| e.message())
        .and_then(|(package, bin)| plan(&project.root, config.assets.as_deref(), &package, &bin));
    let plan = match planned {
        Ok(plan) => plan,
        Err(error) => {
            return crate::contract::cli::setup_failed("run", args, codes::PROJECT_FILE, &error);
        }
    };
    run(&plan, &project.root, crate::cargo())
}

/// Runs `plan` with `cargo` until the game quits.
fn run(plan: &Plan, root: &Path, cargo: OsString) -> u8 {
    println!(
        "watching {}: a file changed there reloads in the game",
        plan.assets.display()
    );
    let status = Command::new(cargo)
        .args(&plan.args)
        .current_dir(root)
        .env(WATCH_VAR, &plan.assets)
        .status();
    match status {
        Ok(status) if status.success() => 0,
        Ok(status) => {
            eprintln!(
                "run --watch: cargo {} exited with {status}",
                plan.args.join(" ")
            );
            1
        }
        Err(e) => {
            let error = format!("cargo doesn't start: {e}");
            eprint!(
                "{}",
                codes::said(
                    "run",
                    codes::TOOLCHAIN.0,
                    &error,
                    codes::help_of(codes::TOOLCHAIN.0)
                )
            );
            3
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::project::Config;

    /// An empty folder of its own for the test `name`.
    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("shuyi-watch-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e:?}"));
        dir
    }

    #[test]
    fn it_runs_the_program_over_the_folder_the_project_names() {
        let dir = folder("names");
        fs::create_dir(dir.as_path().join("files")).unwrap();
        let planned = plan(dir.as_path(), Some("files"), "game", "game").unwrap();
        assert_eq!(planned.args, ["run", "-p", "game", "--bin", "game"]);
        assert!(planned.assets.ends_with("files"), "{planned:?}");
        assert!(planned.assets.is_absolute(), "{planned:?}");
    }

    #[test]
    fn the_folder_is_assets_unless_the_project_says() {
        let dir = folder("default");
        let error = plan(dir.as_path(), None, "game", "game").unwrap_err();
        assert!(error.contains("no folder assets"), "{error}");
        assert!(error.contains("shuyi.ron"), "{error}");
        fs::create_dir(dir.as_path().join("assets")).unwrap();
        let planned = plan(dir.as_path(), None, "game", "game").unwrap();
        assert!(planned.assets.ends_with("assets"), "{planned:?}");
    }

    #[test]
    fn it_takes_no_other_argument_and_needs_a_program() {
        let dir = folder("args");
        let project = Project::new(dir.as_path(), Config::default());
        let args = ["--watch".to_owned(), "--frames".to_owned()];
        assert_eq!(main(&project, &args), 2);
        assert_eq!(main(&project, &["--watch".to_owned()]), 3);
    }

    #[test]
    fn a_game_that_fails_is_a_failure_and_no_cargo_a_setup_to_fix() {
        let dir = folder("fails");
        let planned = Plan {
            args: vec!["--no-such-flag".to_owned()],
            assets: dir.as_path().to_path_buf(),
        };
        assert_eq!(run(&planned, dir.as_path(), crate::cargo()), 1);
        let none = OsString::from("no-such-cargo-here");
        assert_eq!(run(&planned, dir.as_path(), none), 3);
    }
}
