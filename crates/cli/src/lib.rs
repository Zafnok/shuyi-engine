//! Shuyi Engine's tools, `shuyi`: the tool contract's operations (`new`,
//! `check`, `lint`, `run`, `scene`, `replay`, `bots`, `playtest`, `autobalance`,
//! `import`, `bake`, `doctor`, `explain`, each with `--json`; [`contract`]), the contract as
//! an MCP server with a session (`mcp`, `mcp install`; [`mcp`]), and `frame-png`, `font-atlas`,
//! `glyph-tiles`, `check-keys`, `check-text`, `lang-status`, `web` and
//! `private-assets`, each a module of library functions with a thin
//! wrapper here; and the pool of threads the slow ones play on
//! ([`pool`]).
//!
//! They run on a project: the folder holding a `shuyi.ron`
//! ([`project`]), which says what each command scans and builds; `new`,
//! `doctor`, `explain` and `schema` need none. The commands that run the game itself
//! get what they need from it ([`GameTools`]) in a program of its own that
//! calls [`main_with`], and `shuyi` runs that program (`shuyi.ron`'s
//! `game`) for them. A game on the short path gives [`BasicTools`]
//! ([`basic`]).
//!
//! A CLI tool prints to stdout by design, so `print_stdout` is allowed here.
#![allow(clippy::print_stdout)]

pub mod basic;
pub mod check_keys;
pub mod check_text;
pub mod codec;
pub mod contract;
pub mod font_atlas;
pub mod frame_png;
pub mod game;
pub mod glyph_tiles;
pub mod lang_status;
pub mod mcp;
pub mod pool;
pub mod private_assets;
pub mod project;
pub mod test_card;
#[cfg(feature = "toy")]
pub mod toy;
pub mod web;

use std::path::{Path, PathBuf};

use shuyi_assets::target::TargetProfile;

use contract::Op;
use contract::codes;

pub use basic::BasicTools;
pub use game::{
    AutobalancePlan, GameArg, GameBake, GameLang, GameOption, GameTools, OneBitSkin, PlaytestPlan,
    Tunable,
};
pub use project::Project;
/// JSON, for the scenes a game gives ([`contract::session::Scene::data`]).
pub use serde_json;

/// The usage text, with `command` as the tool's name.
pub fn usage(command: &str) -> String {
    format!(
        "\
usage: {command} <command>

the tool contract (each also with --json; a wrong argument prints its usage):
  new <template> <dir>               make a new game from a template: grid, blank
  check [--baked]                    every error of the game's content, with file and line;
                                     --baked: and every way its pack differs from its sources
  check --target <id | all>          whether the frame, its text and the controls fit a target
  lint                               the game's code: check-keys, check-text and the rules
                                     crate's fence, each finding with file and line
  run [steps] [--frames N]           play a key script headless: screens, scene, events, sounds
  scene [steps]                      the scene on screen after a script
  replay <record> [<other>]          a state hash per step; where two runs part
  bots <games>                       the game's bots play: outcomes and measures
  playtest [--stage ID] [--entrant NAME] [--tries N]
                                     each named bot's tries, on every core, against the
                                     game's bands
  autobalance [--stage ID] [--budget N] [--time SECONDS] [--apply]
                                     change the stage until each named bot is in its band:
                                     the changes, before and after; --apply writes it
  import <file> [--print]            read a file through the format registry
  bake [--target <id>] [--out <file>]
                                     bake the game's sources into one pack: sizes and times
  doctor                             check the toolchain and say how to fix it
  explain [<code>]                   every problem code (SH2003), or one code's page
  schema [<name> | --out <dir>]      the contract's JSON schemas
  mcp                                serve the contract over MCP (stdio), with a session
  mcp --ready                        build the server and the game's tool program, then exit
  mcp install [--dry-run] [--yes]    register the server with the agents found here

commands:
  check-keys                         fail on keys hard-coded in game code or text
  check-text                         count screen text still written as literals,
                                     and fail on a screen that reads a data name itself
  lang-status <code> [--only ui|data|dialogue] [--file <dlg>]
                                     list a language pack's missing, stale and orphaned text
  font-atlas <font.bdf>... <out-dir> build the font atlas from BDF fonts
  glyph-tiles                        draw a tileset of the font's glyphs on colored squares
  frame-png <out.png> [steps]        render a scripted game frame to a PNG (frame-png --help)
  web [--release] [--<feature>...]   build and package the web (WASM) build
  private-assets [--library | --pin] fetch the private files into their checkout, or pin it
  mcp-host                           the game's side of `mcp`: JSON lines on stdin and stdout

It runs on the project in the first folder from here up that holds a
{file} (new, doctor, explain, schema and mcp need none). An error has a problem
code: `{command} explain <code>` says what it means. The exit code is 0
when it passed, 1 when not, 2 for a wrong command, and 3 for a setup to fix
({command} doctor, {file}) before running it again.",
        file = project::FILE
    )
}

/// Runs the command `args` on the project of the working folder, with the
/// game's tools if this is the game's tool program, and returns the exit
/// code: 0 for success, 1 for a failed check or step, 2 for wrong
/// arguments (`SH0nnn`), 3 for a setup to fix before running it again
/// (`SH1nnn`: no project, a project file that lacks what the command
/// needs, no game tool program, cargo that doesn't start).
pub fn main_with(args: impl Iterator<Item = String>, game: Option<&dyn GameTools>) -> u8 {
    let args: Vec<String> = args.collect();
    let here = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let project = Project::find(&here);
    if let Some((name, rest)) = args.split_first() {
        let command = project
            .as_ref()
            .map_or("shuyi", |project| project.config.command());
        if let Some(code) = contract::cli::without_project(name, rest, &here, command) {
            return code;
        }
        if name == "import" && project.is_err() {
            let (json, rest) = contract::cli::split_json(rest);
            let importers = game.map_or_else(contract::import::Importers::standard, |game| {
                game.importers()
            });
            let limit = "no project here: only the engine's standard kinds were tried";
            return contract::cli::import(json, &rest, &importers, &here, Some(limit), "shuyi");
        }
    }
    match project {
        Ok(project) => run(&project, &args, game),
        Err((code, e)) => {
            let (name, rest) = args
                .split_first()
                .map_or(("shuyi", &[][..]), |(name, rest)| (name.as_str(), rest));
            let exit = contract::cli::setup_failed(name, rest, code, &e);
            // An op asked for JSON gets the answer alone.
            if Op::named(name).is_none() || !contract::cli::split_json(rest).0 {
                eprintln!("{}", usage("shuyi"));
            }
            exit
        }
    }
}

/// Runs the command `args` on `project`, with the game's tools if they are
/// at hand (otherwise a command that runs the game runs the game's tool
/// program), and returns the exit code (see [`main_with`]).
pub fn run(project: &Project, args: &[String], game: Option<&dyn GameTools>) -> u8 {
    let command = project.config.command();
    let Some((name, args)) = args.split_first() else {
        println!("{}", usage(command));
        return 2;
    };
    if let Some(code) = contract::cli::without_project(name, args, &project.root, command) {
        return code;
    }
    if name == "mcp-host" {
        return match game {
            Some(game) => {
                if cfg!(windows) {
                    // The server's `start` relinks this program.
                    mcp::aside::this_program();
                }
                let stdin = std::io::stdin();
                mcp::host::serve(
                    game,
                    project.clone(),
                    stdin.lock(),
                    std::io::stdout().lock(),
                )
            }
            // It says so if the project names no game tool program.
            None => game::run_in_game_program(project, name, args),
        };
    }
    if name == "lint" {
        // On the project's files and cargo: in this program or the game's.
        return contract::cli::lint(project, args, &contract::lint::ThisMachine);
    }
    if contract::cli::checks_targets(name, args) {
        // The controls need the game's tools: in its tool program.
        return match game {
            Some(game) => contract::cli::check_targets(project, args, Some(game)),
            None if project.config.game.is_some() => game::run_in_game_program(project, name, args),
            None => contract::cli::check_targets(project, args, None),
        };
    }
    if contract::cli::runs_the_game(name) {
        return match game {
            Some(game) => contract::cli::with_game(project, name, args, game),
            None if name == "import" && project.config.game.is_none() => {
                let (json, rest) = contract::cli::split_json(args);
                let limit = format!(
                    "{} names no game tool program: only the engine's standard kinds were tried",
                    project::FILE
                );
                contract::cli::import(
                    json,
                    &rest,
                    &contract::import::Importers::standard(),
                    &project.root,
                    Some(&limit),
                    command,
                )
            }
            None if project.config.game.is_none() => contract::cli::no_game(name, args),
            None => game::run_in_game_program(project, name, args),
        };
    }
    let outcome = match name.as_str() {
        "check-keys" => check_keys(project, args),
        "check-text" => check_text(project, args),
        "lang-status" => match game {
            Some(game) => Ok(lang_status(project, args, game)),
            None => Ok(game::run_in_game_program(project, name, args)),
        },
        "font-atlas" => font_atlas(project, args),
        "glyph-tiles" => glyph_tiles(project, args),
        "frame-png" => match game {
            Some(game) => Ok(frame_png(project, args, game)),
            None => Ok(game::run_in_game_program(project, name, args)),
        },
        "web" => web(project, args),
        "private-assets" => private_assets(project, args),
        other => {
            let error = format!("unknown command: {other}");
            eprint!(
                "{}",
                codes::said(command, codes::WRONG_ARGUMENTS.0, &error, None)
            );
            eprintln!("{}", usage(command));
            Ok(2)
        }
    };
    outcome.unwrap_or_else(|missing| {
        contract::cli::setup_failed(name, args, codes::PROJECT_FILE, &missing)
    })
}

/// A command's exit code, or the error that the project file lacks its
/// part (a [`codes::PROJECT_FILE`] problem).
type Outcome = Result<u8, String>;

fn check_keys(project: &Project, args: &[String]) -> Outcome {
    let rules = project.part(&project.config.check_keys, "check_keys")?;
    if !args.is_empty() {
        eprintln!("usage: {} check-keys", project.config.command());
        return Ok(2);
    }
    let errors = check_keys::run(rules, &project.root);
    if errors.is_empty() {
        println!("check-keys: OK");
        return Ok(0);
    }
    eprintln!("check-keys: {} hard-coded key(s)", errors.len());
    for error in &errors {
        eprintln!("  {error}");
    }
    Ok(1)
}

fn check_text(project: &Project, args: &[String]) -> Outcome {
    let rules = project.part(&project.config.check_text, "check_text")?;
    if !args.is_empty() {
        eprintln!("usage: {} check-text", project.config.command());
        return Ok(2);
    }
    let (hits, mut errors) = check_text::run(rules, &project.root);
    print!(
        "{}",
        check_text::report(&rules.dir, &hits, rules.max_literals)
    );
    errors.extend(check_text::run_names(rules, &project.root));
    for error in &errors {
        eprintln!("check-text: {error}");
    }
    Ok(
        match check_text::verdict(&rules.hint, hits.len(), rules.max_literals) {
            Ok(note) => {
                if let Some(note) = note {
                    println!("{note}");
                }
                u8::from(!errors.is_empty())
            }
            Err(e) => {
                eprintln!("{e}");
                1
            }
        },
    )
}

/// Prints what a language pack lacks. Missing and stale keys are not a
/// failure: the exit code is 0 unless the game's languages don't load (1)
/// or there is no such pack (2).
fn lang_status(project: &Project, args: &[String], game: &dyn GameTools) -> u8 {
    let (code, options) = match lang_status::parse_args(args) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("lang-status: {e}");
            eprintln!(
                "usage: {} lang-status <code> [--only ui|data|dialogue] [--file <dlg>]",
                project.config.command()
            );
            return 2;
        }
    };
    let lang = match game.lang() {
        Ok(lang) => lang,
        Err(e) => {
            eprintln!("lang-status: {e}");
            return 1;
        }
    };
    match lang_status::report(&lang.lang, &lang.variants, &code, &options) {
        Ok(report) => {
            print!("{report}");
            0
        }
        Err(e) => {
            eprintln!("lang-status: {e}");
            2
        }
    }
}

fn font_atlas(project: &Project, args: &[String]) -> Outcome {
    let rules = project.part(&project.config.font_atlas, "font_atlas")?;
    let command = project.config.command();
    let Some((out_dir, fonts @ [_, ..])) = args.split_last() else {
        eprintln!("usage: {command} font-atlas <font.bdf>... <out-dir>");
        return Ok(2);
    };
    let fonts: Vec<PathBuf> = fonts.iter().map(PathBuf::from).collect();
    Ok(
        match font_atlas::run(rules, command, &fonts, Path::new(out_dir)) {
            Ok(summary) => {
                println!("{summary}");
                0
            }
            Err(e) => {
                eprintln!("font-atlas: {e}");
                1
            }
        },
    )
}

fn glyph_tiles(project: &Project, args: &[String]) -> Outcome {
    let rules = project.part(&project.config.glyph_tiles, "glyph_tiles")?;
    if !args.is_empty() {
        eprintln!("usage: {} glyph-tiles", project.config.command());
        return Ok(2);
    }
    Ok(match glyph_tiles::run(rules, &project.root) {
        Ok(summary) => {
            println!("{summary}");
            0
        }
        Err(e) => {
            eprintln!("glyph-tiles: {e}");
            1
        }
    })
}

fn frame_png(project: &Project, args: &[String], game: &dyn GameTools) -> u8 {
    let usage = || {
        frame_png::usage(
            project.config.command(),
            game.frame_options(),
            game.frame_notes(),
        )
    };
    if args.iter().any(|a| a == "--help") {
        println!("{}", usage());
        return 0;
    }
    let options = match frame_png::parse_args(args, game.frame_options()) {
        Ok(options) => options,
        Err(e) => {
            eprintln!("{e}\n\n{}", usage());
            return 2;
        }
    };
    let target = match options
        .target
        .as_deref()
        .map(|id| target_profile(project, id))
    {
        None => None,
        Some(Ok(profile)) => Some(profile),
        Some(Err((code, e))) => {
            eprintln!("frame-png: {e}");
            return code;
        }
    };
    match frame_png::run(&project.root, &options, game, target.as_ref()) {
        Ok(summary) => {
            println!("{summary}");
            0
        }
        Err(e) => {
            eprintln!("frame-png: {e}");
            1
        }
    }
}

/// The profile of target `id` (the engine's or the game's), or the exit
/// code and what is wrong: the profiles don't load (1), or none has that
/// id (2).
fn target_profile(project: &Project, id: &str) -> Result<TargetProfile, (u8, String)> {
    let profiles =
        contract::target::profiles(&project.config, &project.root).map_err(|errors| {
            let shown: Vec<String> = errors.iter().map(ToString::to_string).collect();
            (
                1,
                format!("the target profiles don't load: {}", shown.join("; ")),
            )
        })?;
    profiles.get(id).map(|p| p.profile.clone()).ok_or_else(|| {
        let known: Vec<&str> = profiles.keys().map(String::as_str).collect();
        (
            2,
            format!(
                "--target: no target profile \"{id}\"; the profiles are {}",
                known.join(", ")
            ),
        )
    })
}

fn web(project: &Project, args: &[String]) -> Outcome {
    let rules = project.part(&project.config.web, "web")?;
    let options = match web::parse_args(rules, project.config.command(), args) {
        Ok(options) => options,
        Err(e) => {
            eprintln!("{e}");
            return Ok(2);
        }
    };
    Ok(match web::run(rules, &project.root, &options) {
        Ok(summary) => {
            println!("{summary}");
            0
        }
        Err(e) => {
            eprintln!("web: {e}");
            1
        }
    })
}

fn private_assets(project: &Project, args: &[String]) -> Outcome {
    let rules = project.part(&project.config.private_assets, "private_assets")?;
    let command = project.config.command();
    let mode = match private_assets::parse_args(rules, command, args) {
        Ok(mode) => mode,
        Err(e) => {
            eprintln!("{e}");
            return Ok(2);
        }
    };
    Ok(
        match private_assets::run(rules, command, &project.root, &rules.url, mode) {
            Ok(summary) => {
                println!("{summary}");
                0
            }
            Err(e) => {
                eprintln!("private-assets: {e}");
                1
            }
        },
    )
}
