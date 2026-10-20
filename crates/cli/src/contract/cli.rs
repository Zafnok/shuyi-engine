//! The contract on the command line: `shuyi <op> [args] [--json]`. Each
//! op's arguments become its input, its answer is printed for people or,
//! with `--json`, as one JSON document ([`Answer::to_json`]), and the exit
//! code is 0 if it passed, 1 if not, 2 for wrong arguments.
//!
//! `new`, `doctor` and `schema` need no project. The others run on the
//! project's game, in its tool program; `import` without one reads only
//! the engine's standard kinds.

use std::fs;
use std::path::Path;

use serde::Serialize;

use super::{Answer, Op, bots, check, doctor, import, new, replay, run, schemas};
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

/// Says that `op`'s arguments are wrong (`error`), with its `usage`, as
/// `json` says; the exit code is 2.
fn wrong(json: bool, op: Op, error: &str, usage: &str) -> u8 {
    if json {
        println!("{}", Answer::<()>::failed(op, error).to_json());
    } else {
        eprintln!("{}: {error}\n\n{usage}", op.name());
    }
    2
}

/// Says that op `name` (one that [`runs_the_game`]) can't run: the
/// project names no game tool program. The exit code is 2.
pub fn no_game(name: &str, args: &[String]) -> u8 {
    let (json, _) = split_json(args);
    let error = format!(
        "{} names no game tool program (`game`), and {name} runs the game",
        crate::project::FILE
    );
    match Op::named(name) {
        Some(op) if json => {
            println!("{}", Answer::<()>::failed(op, error).to_json());
        }
        _ => eprintln!("{name}: {error}"),
    }
    2
}

/// Runs `name` with `args` if it is an op that needs no project (`new`,
/// `doctor`, `schema`), in the working folder `cwd`; `None` if it isn't.
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
                eprintln!("schema: no schema {name} (`{command} schema` lists them)");
                2
            }
        }
        _ => {
            eprintln!("usage: {command} schema [<name> | --out <dir>]");
            2
        }
    }
}

/// Whether op `name` with `args` checks targets (`check --target`), which
/// runs on the project file alone, without the game's tools.
pub fn checks_targets(name: &str, args: &[String]) -> bool {
    name == "check" && args.iter().any(|a| a == "--target")
}

/// `check --target` with `args` on `project`.
pub fn check_targets(project: &Project, args: &[String]) -> u8 {
    let (json, args) = split_json(args);
    match check::parse_args(&args) {
        Ok(input) => print(
            json,
            &check::targets(&project.config, &project.root, &input),
            check::text,
        ),
        Err(e) => wrong(json, Op::Check, &e, &check::usage(project.config.command())),
    }
}

/// Whether `name` is an op that runs on the project's game.
pub fn runs_the_game(name: &str) -> bool {
    matches!(
        name,
        "check" | "run" | "scene" | "replay" | "bots" | "import"
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
                &check::targets(&project.config, &project.root, &input),
                check::text,
            ),
            Ok(input) => print(json, &check::check(game, &input), check::text),
            Err(e) => wrong(json, Op::Check, &e, &check::usage(command)),
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
        for op in ["check", "run", "scene", "replay", "bots", "import"] {
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
    fn a_project_with_no_game_program_says_so() {
        assert_eq!(no_game("check", &args(&["--json"])), 2);
        assert_eq!(no_game("check", &[]), 2);
        assert_eq!(no_game("frame-png", &args(&["--json"])), 2);
    }

    #[test]
    fn the_ops_without_a_project_are_new_doctor_and_schema() {
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
        assert_eq!(fs::read_dir(dir.join("s")).unwrap().count(), 16);
        // A file where the folder should be.
        fs::write(dir.join("f"), "").unwrap();
        assert_eq!(
            without_project("schema", &args(&["--out", "f"]), &dir, "shuyi"),
            Some(1)
        );
        assert_eq!(
            without_project("doctor", &args(&["--json"]), &dir, "shuyi").map(|c| c <= 1),
            Some(true)
        );
        assert_eq!(
            without_project("new", &args(&["blank", "g", "--json"]), &dir, "shuyi"),
            Some(0)
        );
        assert_eq!(
            without_project("new", &args(&["blank", "g"]), &dir, "shuyi"),
            Some(1)
        );
        assert_eq!(without_project("new", &[], &dir, "shuyi"), Some(2));
        assert_eq!(
            without_project("doctor", &args(&["x"]), &dir, "shuyi"),
            Some(2)
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
