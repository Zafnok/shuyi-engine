//! The contract on the command line: `shuyi <op> [args] [--json]`. Each
//! op's arguments become its input, its answer is printed for people or,
//! with `--json`, as one JSON document ([`Answer::to_json`]). An error
//! carries a problem code ([`codes`]), and the exit code is 0 if it
//! passed, 1 if not, 2 for wrong arguments (`SH0nnn`), and 3 for a setup
//! to fix before running it again (`SH1nnn`: no project, a project file
//! that lacks what it needs, the toolchain).
//!
//! `new`, `doctor`, `explain`, `attach` and `schema` need no project. `lint`,
//! `modules` and `add` run on the project, not its game. The others run on
//! the project's game, in its tool program; `import` without one reads
//! only the engine's standard kinds.

use std::fs;
use std::path::Path;

use serde::Serialize;

use super::codes::{self, Code};
use super::{
    Answer, Op, add, attach, autobalance, bake, bots, check, doctor, explain, export, import, lint,
    modules, new, playtest, replay, run, schemas,
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

/// `answer`, also written to `path` as the JSON `--json` prints. If it
/// can't be, the answer keeps its result but fails with [`codes::FILE`],
/// naming the file as given, `file`.
fn also_written<T: Serialize>(mut answer: Answer<T>, path: &Path, file: &Path) -> Answer<T> {
    if let Err(e) = fs::write(
        path,
        format!(
            "{}
",
            answer.to_json()
        ),
    ) {
        answer.ok = false;
        answer.error = Some(format!("--report {}: {e}", file.display()));
        answer = answer.with_code(codes::FILE);
    }
    answer
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
/// `doctor`, `explain`, `attach`, `schema`, and `mcp`, which finds a project per
/// call), in the working folder `cwd`; `None` if it isn't.
pub fn without_project(name: &str, args: &[String], cwd: &Path, command: &str) -> Option<u8> {
    if name == "mcp" {
        return Some(crate::mcp::main(args, cwd));
    }
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
        "attach" => match attach::parse_args(&args) {
            Ok(input) => print(json, &attach::attach(&input, cwd), attach::text),
            Err(e) => wrong(json, Op::Attach, &e, &attach::usage(command)),
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

/// Whether op `name` with `args` checks targets (`check --target`, and
/// `export`, which starts with that check), which runs on the project
/// file alone, without the game's tools.
pub fn checks_targets(name: &str, args: &[String]) -> bool {
    name == "export" || (name == "check" && args.iter().any(|a| a == "--target"))
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

/// `export` with `args` on `project`, with the game's tools `game` if
/// they are at hand (for its check's controls).
pub fn export(project: &Project, args: &[String], game: Option<&dyn GameTools>) -> u8 {
    let (json, args) = split_json(args);
    match export::parse_args(&args) {
        Ok(input) => print(json, &export::export(project, &input, game), export::text),
        Err(e) => wrong(
            json,
            Op::Export,
            &e,
            &export::usage(project.config.command()),
        ),
    }
}

/// `lint` with `args` on `project`, running cargo with `cargo`.
pub fn lint(project: &Project, args: &[String], cargo: &dyn lint::Cargo) -> u8 {
    let (json, args) = split_json(args);
    if !args.is_empty() {
        let usage = lint::usage(project.config.command());
        return wrong(json, Op::Lint, "no arguments", &usage);
    }
    let input = lint::LintInput::default();
    print(json, &lint::lint(cargo, project, &input), lint::text)
}

/// `modules` or `add` (`name`) with `args` on `project`: on its files,
/// not its game.
pub fn modules_or_add(project: &Project, name: &str, args: &[String]) -> u8 {
    let (json, args) = split_json(args);
    let command = project.config.command();
    if name == "modules" {
        if !args.is_empty() {
            return wrong(json, Op::Modules, "no arguments", &modules::usage(command));
        }
        let input = modules::ModulesInput::default();
        return print(json, &modules::modules(project, &input), modules::text);
    }
    match add::parse_args(&args) {
        Ok(input) => print(json, &add::add(project, &input), add::text),
        Err(e) => wrong(json, Op::Add, &e, &add::usage(command)),
    }
}

/// `replay`'s input from `args` (without `--json`): one or two records,
/// and `--any-stamp` anywhere; `None` for anything else.
fn replay_args(args: &[String]) -> Option<replay::ReplayInput> {
    let any_stamp = args.iter().any(|a| a == "--any-stamp");
    let records: Vec<&String> = args.iter().filter(|a| *a != "--any-stamp").collect();
    let (record, other) = match records.as_slice() {
        [record] => (record, None),
        [record, other] => (record, Some((*other).clone())),
        _ => return None,
    };
    if records.iter().any(|a| a.starts_with("--")) {
        return None;
    }
    let mut input = replay::ReplayInput::new((*record).clone());
    input.other = other;
    input.any_stamp = any_stamp;
    Some(input)
}

/// Whether `name` is an op that runs on the project's game.
pub fn runs_the_game(name: &str) -> bool {
    matches!(
        name,
        "check"
            | "run"
            | "scene"
            | "replay"
            | "bots"
            | "playtest"
            | "autobalance"
            | "import"
            | "bake"
            | "export"
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
        "export" => match export::parse_args(&args) {
            Ok(input) => print(
                json,
                &export::export(project, &input, Some(game)),
                export::text,
            ),
            Err(e) => wrong(json, Op::Export, &e, &export::usage(command)),
        },
        "bake" => match bake::parse_args(&args) {
            Ok(input) => print(json, &bake::bake(game, project, &input), bake::text),
            Err(e) => wrong(json, Op::Bake, &e, &bake::usage(command)),
        },
        "run" | "scene" => {
            let op = if name == "run" { Op::Run } else { Op::Scene };
            match run::parse_args(&args, options) {
                Ok(input) if op == Op::Run => {
                    print(json, &run::run_at(game, &project.root, &input), run::text)
                }
                Ok(input) => print(json, &run::scene(game, &input), run::scene_output_text),
                Err(e) => wrong(json, op, &e, &run::usage(command, op, options)),
            }
        }
        "replay" => {
            let Some(input) = replay_args(&args) else {
                let usage =
                    format!("usage: {command} replay <record> [<other>] [--any-stamp] [--json]");
                return wrong(json, Op::Replay, "one or two records", &usage);
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
            Ok(parsed) => {
                let answer = playtest::kept(game, &project.root, &parsed.input);
                let answer = match &parsed.report {
                    Some(file) => also_written(answer, &project.root.join(file), file),
                    None => answer,
                };
                if parsed.brief {
                    print(json, &answer, |outputs| playtest::brief(outputs))
                } else {
                    print(json, &answer, |outputs| playtest::text(outputs))
                }
            }
            Err(e) => wrong(json, Op::Playtest, &e, &playtest::usage(command, options)),
        },
        "autobalance" => match autobalance::parse_args(&args, options) {
            Ok(input) => print(
                json,
                &autobalance::autobalance(game, &input),
                autobalance::text,
            ),
            Err(e) => wrong(
                json,
                Op::Autobalance,
                &e,
                &autobalance::usage(command, options),
            ),
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
    fn replay_takes_one_or_two_records_and_any_stamp_anywhere() {
        let one = replay_args(&args(&["a.ron"])).unwrap();
        assert_eq!(one, replay::ReplayInput::new("a.ron"));
        let two = replay_args(&args(&["--any-stamp", "a.ron", "b.ron"])).unwrap();
        assert_eq!(
            (two.record.as_str(), two.other.as_deref(), two.any_stamp),
            ("a.ron", Some("b.ron"), true)
        );
        let after = replay_args(&args(&["a.ron", "--any-stamp"])).unwrap();
        assert!(after.any_stamp && after.other.is_none(), "{after:?}");
        for wrong in [
            &[][..],
            &["--any-stamp"],
            &["a", "b", "c"],
            &["a.ron", "--bogus"],
        ] {
            assert_eq!(replay_args(&args(wrong)), None, "{wrong:?}");
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
            "check",
            "run",
            "scene",
            "replay",
            "bots",
            "playtest",
            "autobalance",
            "import",
            "bake",
            "export",
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
        assert_eq!(code(&["playtest", "--tries", "3", "--report", "r.json"]), 0);
        let written = fs::read_to_string(dir.join("r.json")).unwrap();
        let written: Answer<Vec<playtest::PlaytestOutput>> =
            serde_json::from_str(&written).unwrap();
        assert!(written.ok);
        assert_eq!(written.result.unwrap()[0].report.entrants[0].tries.len(), 3);
        // Each run was kept in the project's history; a brief one isn't.
        let kept = dir.join("target/playtest-history/stage-walk+run.jsonl");
        assert_eq!(fs::read_to_string(&kept).unwrap().lines().count(), 2);
        assert_eq!(
            code(&["playtest", "--tries", "3", "--brief", "--no-history"]),
            0
        );
        assert_eq!(fs::read_to_string(&kept).unwrap().lines().count(), 2);
        assert_eq!(
            code(&["playtest", "--report", "no/such/r.json", "--json"]),
            1
        );
        assert_eq!(code(&["autobalance", "--tries", "10", "--quick", "10"]), 0);
        assert_eq!(code(&["autobalance", "--tries", "10", "--budget", "2"]), 1);
        assert_eq!(code(&["autobalance", "3"]), 2);
        assert_eq!(code(&["autobalance", "--quick", "0", "--json"]), 2);
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
    fn a_report_file_holds_the_json_the_answer_prints() {
        let dir = std::env::temp_dir().join(format!("shuyi-cli-report-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let answer = Answer::new(Op::Playtest, true, 7u8, vec!["a limit".to_owned()]);
        let file = Path::new("r.json");
        assert_eq!(also_written(answer.clone(), &dir.join(file), file), answer);
        let written = fs::read_to_string(dir.join(file)).unwrap();
        assert_eq!(
            written,
            format!(
                "{}
",
                answer.to_json()
            )
        );
        // A file that can't be written fails the answer, which keeps its
        // result.
        let file = Path::new("missing/r.json");
        let failed = also_written(answer, &dir.join(file), file);
        assert!(!failed.ok);
        assert_eq!(failed.result, Some(7));
        assert_eq!(failed.code.as_deref(), Some("SH3004"));
        assert!(
            failed
                .error
                .unwrap()
                .starts_with("--report missing/r.json: ")
        );
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

    /// A cargo that runs and finds nothing.
    struct QuietCargo;

    impl lint::Cargo for QuietCargo {
        fn run(&self, _root: &Path, _args: &[&str]) -> Result<lint::Ran, String> {
            Ok(lint::Ran {
                success: true,
                ..lint::Ran::default()
            })
        }
    }

    #[test]
    fn lint_answers_with_its_exit_code() {
        let dir = std::env::temp_dir().join(format!("shuyi-cli-lint-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        let mut p = project(dir.clone());
        p.config.rules = Some("r".to_owned());
        assert_eq!(lint(&p, &[], &QuietCargo), 0);
        assert_eq!(lint(&p, &args(&["--json"]), &QuietCargo), 0);
        assert_eq!(lint(&p, &args(&["x"]), &QuietCargo), 2);
        fs::write(
            dir.join("src/a.rs"),
            "fn f() {\n    let _ = Key::Space;\n}\n",
        )
        .unwrap();
        p.config.check_keys = Some(crate::project::CheckKeys {
            rust: vec!["src".to_owned()],
            ..crate::project::CheckKeys::default()
        });
        assert_eq!(lint(&p, &[], &QuietCargo), 1);
        assert_eq!(lint(&p, &args(&["--json"]), &QuietCargo), 1);
        fs::remove_dir_all(&dir).unwrap();
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
    fn the_ops_without_a_project_are_new_doctor_explain_attach_and_schema() {
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
        assert_eq!(fs::read_dir(dir.join("s")).unwrap().count(), 51);
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
        assert_eq!(without_project("attach", &[], &dir, "shuyi"), Some(2));
        assert_eq!(
            without_project("attach", &args(&["127.0.0.1:1", "--json"]), &dir, "shuyi"),
            Some(2)
        );
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
