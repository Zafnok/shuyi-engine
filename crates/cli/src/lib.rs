//! Shuyi Engine's tools, `shuyi`: `frame-png`, `font-atlas`, `glyph-tiles`,
//! `check-keys`, `check-text`, `lang-status`, `web` and `private-assets`,
//! each a module of library functions with a thin wrapper here.
//!
//! They run on a project: the folder holding a `shuyi.ron`
//! ([`project`]), which says what each command scans and builds. Two
//! commands run the game itself, `frame-png` and `lang-status`; a game
//! gives them what they need ([`GameTools`]) in a program of its own that
//! calls [`main_with`], and `shuyi` runs that program (`shuyi.ron`'s
//! `game`) for them.
//!
//! A CLI tool prints to stdout by design, so `print_stdout` is allowed here.
#![allow(clippy::print_stdout)]

pub mod check_keys;
pub mod check_text;
pub mod codec;
pub mod font_atlas;
pub mod frame_png;
pub mod game;
pub mod glyph_tiles;
pub mod lang_status;
pub mod private_assets;
pub mod project;
pub mod test_card;
#[cfg(feature = "toy")]
pub mod toy;
pub mod web;

use std::path::{Path, PathBuf};

pub use game::{GameArg, GameLang, GameOption, GameTools};
pub use project::Project;

/// The usage text, with `command` as the tool's name.
pub fn usage(command: &str) -> String {
    format!(
        "\
usage: {command} <command>

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

It runs on the project in the first folder from here up that holds a
{file}.",
        file = project::FILE
    )
}

/// Runs the command `args` on the project of the working folder, with the
/// game's tools if this is the game's tool program, and returns the exit
/// code: 0 for success, 1 for a failed check or step, 2 for wrong
/// arguments or a project file that lacks what the command needs.
pub fn main_with(args: impl Iterator<Item = String>, game: Option<&dyn GameTools>) -> u8 {
    let args: Vec<String> = args.collect();
    let here = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match Project::find(&here) {
        Ok(project) => run(&project, &args, game),
        Err(e) => {
            eprintln!("{e}");
            eprintln!("{}", usage("shuyi"));
            2
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
            eprintln!("unknown command: {other}");
            eprintln!("{}", usage(command));
            Ok(2)
        }
    };
    outcome.unwrap_or_else(|missing| {
        eprintln!("{missing}");
        2
    })
}

/// A command's exit code, or the error that the project file lacks its
/// part.
type Outcome = Result<u8, String>;

fn check_keys(project: &Project, args: &[String]) -> Outcome {
    let rules = project.part(&project.config.check_keys, "check-keys", "check_keys")?;
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
    let rules = project.part(&project.config.check_text, "check-text", "check_text")?;
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
    let rules = project.part(&project.config.font_atlas, "font-atlas", "font_atlas")?;
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
    let rules = project.part(&project.config.glyph_tiles, "glyph-tiles", "glyph_tiles")?;
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
    match frame_png::run(&project.root, &options, game) {
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

fn web(project: &Project, args: &[String]) -> Outcome {
    let rules = project.part(&project.config.web, "web", "web")?;
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
    let rules = project.part(
        &project.config.private_assets,
        "private-assets",
        "private_assets",
    )?;
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
