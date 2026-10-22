//! The contract on the command line: `shuyi <op> [args] [--json]`. Each
//! op's arguments become its input, its answer is printed for people or,
//! with `--json`, as one JSON document ([`Answer::to_json`]). An error
//! carries a problem code ([`codes`]), and the exit code is 0 if it
//! passed, 1 if not, 2 for wrong arguments (`SH0nnn`), and 3 for a setup
//! to fix before running it again (`SH1nnn`: no project, a project file
//! that lacks what it needs, the toolchain).
//!
//! `new`, `doctor`, `explain` and `schema` need no project. The others run
//! on the project's game, in its tool program; `import` without one reads
//! only the engine's standard kinds.

use std::fs;
use std::path::Path;

use serde::Serialize;

use super::codes::{self, Code};
use super::{
    Answer, Op, bake, bots, check, doctor, explain, import, new, playtest, replay, run, schemas,
};
use crate::game::GameTools;
use crate::project::Project;

/// Whether `args` ask for JSON, and the rest of them.
pub fn split_json(args: &[String]) -> (bool, Vec<String>) {
    let json = args.iter().any(|a| a == "--json");
    (
        json,
        args.iter().filter(|a| *a != "--json").cloned().collect(),
    )
}

/// Prints `answer` as `json` says, `text` writing its result for people,
/// and returns its exit code.
fn print<T: Serialize>(json: bool, answer: &Answer<T>, text: impl FnOnce(&T) -> String) -> u8 {
    if json {
        println!("{}", answer.to_json());
    } else if answer.error.is_some() {
        eprint!("{}", answer.to_text(text));
    } else {
        print!("{}", answer.to_text(text));
    }
    answer.exit_code()
}

/// Says that `op`'s arguments are wrong (`error`), with its `usage` as
/// the help, as `json` says; the exit code is 2 ([`codes::WRONG_ARGUMENTS`]).
fn wrong(json: bool, op: Op, error: &str, usage: &str) -> u8 {
    let answer = Answer::<()>::failed(op, codes::WRONG_ARGUMENTS, error).with_help(usage);
    print(json, &answer, |()| String::new())
}

/// Says that command `name` can't run with `args`: its setup is wrong
/// (`error`, of setup code `code`). An op with `--json` answers a failed
/// [`Answer`]; anything else says it for people. Returns the code's exit
/// code, 3.
pub fn setup_failed(name: &str, args: &[String], code: Code, error: &str) -> u8 {
    let (json, _) = split_json(args);
    match Op::named(name) {
        Some(op) if json => println!("{}", Answer::<()>::failed(op, code, error).to_json()),
        _ => eprint!(
            "{}",
            codes::said(name, code.0, error, codes::help_of(code.0))
        ),
    }
    codes::exit_code(Some(code.0))
}

/// Says that command `name` (one that runs the game) can't run with
/// `args`: the project names no game tool program
/// ([`codes::PROJECT_FILE`]). The exit code is 3.
pub fn no_game(name: &str, args: &[String]) -> u8 {
    let error = format!(
        "{} names no game tool program (`game`), and {name} runs the game",
        crate::project::FILE
    );
    setup_failed(name, args, codes::PROJECT_FILE, &error)
}

/// Runs `name` with `args` if it is an op that needs no project (`new`,
/// `doctor`, `explain`, `schema`), in the working folder `cwd`; `None` if
/// it isn't.
pub fn without_project(name: &str, args: &[String], cwd: &Path, command: &str) -> Option<u8> {
    let (json, args) = split_json(args);
    Some(match name {
        "new" => match new::parse_args(&args) {
            Ok(input) => print(json, &new::new(cwd, &input), new::text),
            Err(e) => wrong(json, Op::New, &e, &new::usage(command)),
        },
        "doctor" => {
            if !args.is_empty() {
                return Some(wrong(
                    json,
                    Op::Doctor,
                    "no arguments",
                    &format!("usage: {command} doctor [--json]"),
                ));
            }
            let input = doctor::DoctorInput::default();
            print(
                json,
                &doctor::doctor(&doctor::ThisMachine, &input),
                doctor::text,
            )
        }
        "explain" => match explain::parse_args(&args) {
            Ok(input) => print(json, &explain::explain(&input), explain::text),
            Err(e) => wrong(json, Op::Explain, &e, &explain::usage(command)),
        },
        "schema" => schema(&args, cwd, command),
        _ => return None,
    })
}

/// `shuyi schema`: the schemas' names; `schema <name>`, one of them;
/// `schema --out <dir>`, every one written into `dir`.
fn schema(args: &[String], cwd: &Path, command: &str) -> u8 {
    let files = schemas();
    match args {
        [] => {
            for file in &files {
                println!("{}", file.name.trim_end_matches(".json"));
            }
            0
        }
        [flag, dir] if flag == "--out" => {
            let dir = cwd.join(dir);
            for file in &files {
                if let Err(e) = fs::create_dir_all(&dir)
                    .and_then(|()| fs::write(dir.join(&file.name), &file.text))
                {
                    eprintln!("schema: {}: {e}", dir.join(&file.name).display());
                    return 1;
                }
            }
            println!(
                "schema: {} file(s) in {}, fingerprint {}",
                files.len(),
                dir.display(),
                super::fingerprint(&files)
            );
            0
        }
        [name] => {
            let file = files
                .iter()
                .find(|f| f.name == *name || f.name == format!("{name}.json"));
            if let Some(file) = file {
                print!("{}", file.text);
                0
            } else {
                let help = format!("`{command} schema` lists them");
                let error = format!("no schema {name}");
                eprint!(
                    "{}",
                    codes::said("schema", codes::WRONG_ARGUMENTS.0, &error, Some(&help))
                );
                2
            }
        }
        _ => {
            let usage = format!("usage: {command} schema [<name> | --out <dir>]");
            let error = "a schema's name, --out <dir>, or nothing";
            eprint!(
                "{}",
                codes::said("schema", codes::WRONG_ARGUMENTS.0, error, Some(&usage))
            );
            2
        }
    }
}

/// Whether op `name` with `args` checks targets (`check --target`), which
/// runs on the project file alone, without the game's tools.
pub fn checks_targets(name: &str, args: &[String]) -> bool {
    name == "check" && args.iter().any(|a| a == "--target")
}

/// `check --target` with `args` on `project`, with the game's tools
/// `game` if they are at hand.
pub fn check_targets(project: &Project, args: &[String], game: Option<&dyn GameTools>) -> u8 {
    let (json, args) = split_json(args);
    match check::parse_args(&args) {
        Ok(input) => print(
            json,
            &check::targets(&project.config, &project.root, &input, game),
            check::text,
        ),
        Err(e) => wrong(json, Op::Check, &e, &check::usage(project.config.command())),
    }
}

/// Whether `name` is an op that runs on the project's game.
pub fn runs_the_game(name: &str) -> bool {
    matches!(
        name,
        "check" | "run" | "scene" | "replay" | "bots" | "playtest" | "import" | "bake"
    )
}

/// Runs op `name` (one that [`runs_the_game`]) with `args` on `project`'s
/// `game`.
pub fn with_game(project: &Project, name: &str, args: &[String], game: &dyn GameTools) -> u8 {
    let (json, args) = split_json(args);
    let command = project.config.command();
    let options = game.frame_options();
    match name {
        "check" => match check::parse_args(&args) {
            Ok(input) if input.target.is_some() => print(
                json,
                &check::targets(&project.config, &project.root, &input, Some(game)),
                check::text,
            ),
            Ok(input) => print(json, &check::check(game, &input), check::text),
            Err(e) => wrong(json, Op::Check, &e, &check::usage(command)),
        },
        "bake" => match bake::parse_args(&args) {
            Ok(input) => print(json, &bake::bake(game, project, &input), bake::text),
            Err(e) => wrong(json, Op::Bake, &e, &bake::usage(command)),
        },
        "run" | "scene" => {
            let op = if name == "run" { Op::Run } else { Op::Scene };
            match run::parse_args(&args, options) {
                Ok(input) if op == Op::Run => print(json, &run::run(game, &input), run::text),
                Ok(input) => print(json, &run::scene(game, &input), run::scene_output_text),
                Err(e) => wrong(json, op, &e, &run::usage(command, op, options)),
            }
        }
        "replay" => {
            let input = match args.as_slice() {
                [record] => replay::ReplayInput {
                    record: record.clone(),
                    other: None,
                },
                [record, other] => replay::ReplayInput {
                    record: record.clone(),
                    other: Some(other.clone()),
                },
                _ => {
                    let usage = format!("usage: {command} replay <record> [<other>] [--json]");
                    return wrong(json, Op::Replay, "one or two records", &usage);
                }
            };
            print(
                json,
                &replay::replay(game, &project.root, &input),
                replay::text,
            )
        }
        "bots" => match bots::parse_args(&args, options) {
            Ok(input) => print(json, &bots::bots(game, &input), bots::text),
            Err(e) => wrong(json, Op::Bots, &e, &bots::usage(command, options)),
        },
        "playtest" => match playtest::parse_args(&args, options) {
            Ok(input) => print(json, &playtest::playtest(game, &input), playtest::text),
            Err(e) => wrong(json, Op::Playtest, &e, &playtest::usage(command, options)),
        },
        _ => import(json, &args, &game.importers(), &project.root, None, command),
    }
}

/// `import` with `args` (no `--json`) by `importers`, from `root`, with
/// `limit` added to its answer if it is given.
pub fn import(
    json: bool,
    args: &[String],
    importers: &import::Importers,
    root: &Path,
    limit: Option<&str>,
    command: &str,
) -> u8 {
    match import::parse_args(args) {
        Ok(input) => {
            let mut answer = import::import(importers, root, &input);
            if let Some(limit) = limit {
                answer = answer.limited(limit);
            }
            print(json, &answer, import::text)
        }
        Err(e) => wrong(
            json,
            Op::Import,
            &e,
            &format!("usage: {command} import <file> [--kind KIND] [--print] [--json]"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::contract::fake::{Fake, FakeSession};
    use crate::project::Config;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    fn project(root: PathBuf) -> Project {
        Project {
            root,
            config: Config::default(),
        }
    }

    #[test]
    fn json_is_a_flag_anywhere() {
        assert_eq!(
            split_json(&args(&["a", "--json", "b"])),
            (true, args(&["a", "b"]))
        );
        assert_eq!(split_json(&args(&["a"])), (false, args(&["a"])));
    }

    #[test]
    fn the_ops_that_run_the_game_are_named() {
        for op in [
            "check", "run", "scene", "replay", "bots", "playtest", "import", "bake",
        ] {
            assert!(runs_the_game(op), "{op}");
        }
        for other in ["new", "doctor", "schema", "frame-png", "check-keys"] {
            assert!(!runs_the_game(other), "{other}");
        }
    }

    #[test]
    fn each_op_answers_with_its_exit_code() {
        let dir = std::env::temp_dir().join(format!("shuyi-cli-ops-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.ron"), "h0\nh1").unwrap();
        fs::write(dir.join("b.ron"), "h0\nhx").unwrap();
        let p = project(dir.clone());
        let game = Fake::default();
        let code = |items: &[&str]| {
            let (name, rest) = items.split_first().unwrap();
            with_game(&p, name, &args(rest), &game)
        };
        assert_eq!(code(&["check"]), 0);
        assert_eq!(code(&["check", "--json"]), 0);
        assert_eq!(code(&["check", "x"]), 2);
        assert_eq!(code(&["check", "x", "--json"]), 2);
        assert_eq!(code(&["check", "--baked"]), 0);
        assert_eq!(code(&["bake", "--out", "p.shuyipak"]), 0);
        assert!(dir.join("p.shuyipak").is_file());
        assert_eq!(code(&["bake", "--bogus"]), 2);
        assert_eq!(code(&["run", "--script", "Enter", "--frames", "2"]), 0);
        assert_eq!(code(&["run", "--bogus"]), 2);
        assert_eq!(code(&["scene", "--json"]), 0);
        assert_eq!(code(&["scene", "--frames"]), 2);
        assert_eq!(code(&["replay", "a.ron"]), 0);
        assert_eq!(code(&["replay", "a.ron", "b.ron"]), 1);
        assert_eq!(code(&["replay"]), 2);
        assert_eq!(code(&["replay", "a", "b", "c"]), 2);
        assert_eq!(code(&["replay", "missing.ron"]), 1);
        assert_eq!(code(&["bots", "3"]), 0);
        assert_eq!(code(&["bots"]), 2);
        assert_eq!(code(&["playtest", "--tries", "3"]), 0);
        assert_eq!(code(&["playtest", "3"]), 2);
        assert_eq!(code(&["import", "a.ron"]), 1);
        assert_eq!(code(&["import"]), 2);
        // A scene that isn't there fails the op, not its arguments.
        let mut none = FakeSession::on("title", "");
        none.scene = Err("no stage".into());
        let blank = Fake {
            session: Ok(none),
            ..Fake::default()
        };
        assert_eq!(with_game(&p, "scene", &[], &blank), 1);
        assert_eq!(with_game(&p, "run", &[], &blank), 0);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn check_with_a_target_runs_on_the_project_file_in_the_game_program_too() {
        let mut p = project(PathBuf::from("."));
        p.config = Config::parse(
            r#"(targets: {"desktop": (frame: (800, 512), scale: integer, text_px: 16)})"#,
        )
        .unwrap();
        // The fake game offers no controls: an answer of 0 is the
        // project's.
        assert_eq!(
            with_game(&p, "check", &args(&["--target", "all"]), &Fake::default()),
            0
        );
        assert_eq!(
            check_targets(&p, &args(&["--target", "all", "--json"]), None),
            0
        );
        assert_eq!(check_targets(&p, &args(&["--target", "web"]), None), 1);
        assert_eq!(check_targets(&p, &args(&["--target"]), None), 2);
        // With its controls, a warning: its title declares nothing.
        let game = Fake {
            controls: Some(Ok(())),
            ..Fake::default()
        };
        assert_eq!(
            check_targets(&p, &args(&["--target", "all"]), Some(&game)),
            0
        );
        assert!(checks_targets("check", &args(&["--target", "all"])));
        assert!(!checks_targets("check", &args(&["--json"])));
        assert!(!checks_targets("run", &args(&["--target"])));
    }

    #[test]
    fn a_project_with_no_game_program_says_so() {
        assert_eq!(no_game("check", &args(&["--json"])), 3);
        assert_eq!(no_game("check", &[]), 3);
        assert_eq!(no_game("frame-png", &args(&["--json"])), 3);
        assert_eq!(
            setup_failed("run", &[], codes::TOOLCHAIN, "cannot run cargo"),
            3
        );
    }

    #[test]
    fn the_ops_without_a_project_are_new_doctor_explain_and_schema() {
        let dir = std::env::temp_dir().join(format!("shuyi-cli-free-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(without_project("check", &[], &dir, "shuyi"), None);
        assert_eq!(without_project("schema", &[], &dir, "shuyi"), Some(0));
        assert_eq!(
            without_project("schema", &args(&["check.input"]), &dir, "shuyi"),
            Some(0)
        );
        assert_eq!(
            without_project("schema", &args(&["check.input.json"]), &dir, "shuyi"),
            Some(0)
        );
        assert_eq!(
            without_project("schema", &args(&["--out", "s"]), &dir, "shuyi"),
            Some(0)
        );
        assert_eq!(fs::read_dir(dir.join("s")).unwrap().count(), 22);
        // A file where the folder should be.
        fs::write(dir.join("f"), "").unwrap();
        assert_eq!(
            without_project("schema", &args(&["--out", "f"]), &dir, "shuyi"),
            Some(1)
        );
        assert!(matches!(
            without_project("doctor", &args(&["--json"]), &dir, "shuyi"),
            Some(0 | 3)
        ));
        assert_eq!(
            without_project("new", &args(&["blank", "g", "--json"]), &dir, "shuyi"),
            Some(0)
        );
        assert_eq!(
            without_project("new", &args(&["blank", "g"]), &dir, "shuyi"),
            Some(2)
        );
        assert_eq!(without_project("new", &[], &dir, "shuyi"), Some(2));
        assert_eq!(
            without_project("doctor", &args(&["x"]), &dir, "shuyi"),
            Some(2)
        );
        assert_eq!(without_project("explain", &[], &dir, "shuyi"), Some(0));
        assert_eq!(
            without_project("explain", &args(&["SH2003", "--json"]), &dir, "shuyi"),
            Some(0)
        );
        assert_eq!(
            without_project("explain", &args(&["SH9999"]), &dir, "shuyi"),
            Some(2)
        );
        assert_eq!(
            without_project("explain", &args(&["a", "b"]), &dir, "shuyi"),
            Some(2)
        );
        assert_eq!(
            without_project("schema", &args(&["nope"]), &dir, "shuyi"),
            Some(2)
        );
        assert_eq!(
            without_project("schema", &args(&["a", "b", "c"]), &dir, "shuyi"),
            Some(2)
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
