//! Pictures `frame-png` makes that a game keeps checked in: its shots, and
//! what they share with its [`preview`](super::preview).
//!
//! A game's shots are the still frames the book shows: `shuyi.ron`'s
//! `shots` part names each and holds the `frame-png` arguments that make it
//! ([`crate::project::Shot`]), shot `<name>` is always `shots/<name>.png` in
//! the project's root ([`file()`]), `frame-png --shot <name>` writes one and
//! `frame-png --shots` all of them. [`check_shots`] fails when a file is no
//! longer what its arguments make, or when `shots/` holds a file no shot
//! names: a test of the game calls it, so a change to what the game shows
//! fails until the shots are remade.
//!
//! Both checks compare decoded pixels, never bytes: the same picture
//! written with another compression passes. Each problem with a file ends
//! with the command that remakes it; a file no shot names says
//! to remove it ([`orphan_problem`]), which no tool does, as it may be the
//! game's own.

use std::fs;

use crate::codec::decode_png;
use crate::contract;
use crate::game::{GameOption, GameTools};
use crate::project::{Project, Shot};

use super::{Image, Options, parse_args};

/// The folder of the shots, in the project's root.
pub const FOLDER: &str = "shots";

/// The largest shot, in bytes (256 KiB).
pub const MAX_SHOT_BYTES: u64 = 256 * 1024;

/// The part's name, as the errors give it.
const PART: &str = "shuyi.ron's `shots`";

/// The file of shot `name`, from the project's root: `shots/<name>.png`.
pub fn file(name: &str) -> String {
    format!("{FOLDER}/{name}.png")
}

/// What ends a problem with a kept file: `problem`, then the command that
/// remakes it, `frame-png` with `flag` (`--preview`, `--shot <name>`).
pub(crate) fn remake(project: &Project, flag: &str, problem: &str) -> String {
    format!(
        "{problem}\nremake it: {} frame-png {flag}",
        project.config.command()
    )
}

/// Whether kept file `file` of `len` bytes is within `cap`; the error gives
/// its size, the cap of `kind` (`a preview`) and how to make it smaller,
/// `advice`.
pub(crate) fn check_size(
    file: &str,
    len: u64,
    cap: u64,
    kind: &str,
    advice: &str,
) -> Result<(), String> {
    if len <= cap {
        return Ok(());
    }
    Err(format!(
        "{file} is {} KiB, more than the {} KiB {kind} may be: {advice}",
        len.div_ceil(1024),
        cap / 1024
    ))
}

/// The first pixel where `have` and `want`, of the same size, differ, as
/// its x and y; none when they are the same.
pub(crate) fn first_difference(have: &Image, want: &Image) -> Option<(usize, usize)> {
    let pixel = have
        .rgba
        .chunks(4)
        .zip(want.rgba.chunks(4))
        .position(|(a, b)| a != b)?;
    let width = usize::try_from(have.width).unwrap_or(usize::MAX).max(1);
    Some((pixel % width, pixel / width))
}

/// `frame-png`'s options for shot `name` of `project`, from its arguments
/// in the `shots` part with [`file()`] as the output path; `game` is the
/// game's own options. `--target` is allowed: a shot may show a target's
/// display.
///
/// # Errors
/// Each names `shuyi.ron`'s `shots`: a name no shot may have
/// ([`Shot::check_name`]), no such shot (with the names there are),
/// arguments that don't parse, an output path among them, or `--count` (a
/// shot is one frame).
pub fn shot_options(project: &Project, name: &str, game: &[GameOption]) -> Result<Options, String> {
    Shot::check_name(name).map_err(|e| format!("{PART}: {e}"))?;
    let shots = &project.config.shots;
    let Some(shot) = shots.get(name) else {
        let names: Vec<&str> = shots.keys().map(String::as_str).collect();
        let there = if names.is_empty() {
            "it has none".to_owned()
        } else {
            format!("it has {}", names.join(", "))
        };
        return Err(format!("{PART} has no `{name}`: {there}"));
    };
    let out = file(name);
    let mut args = vec![out.clone()];
    args.extend(shot.args.iter().cloned());
    let options = parse_args(&args, game).map_err(|e| {
        match e.strip_prefix("one output path only, not also ") {
            Some(path) => format!(
                "{PART}, `{name}`, names an output path, {path}: a shot is always {out}, in the \
                 project's root"
            ),
            None => format!("{PART}, `{name}`: {e}"),
        }
    })?;
    if options.capture.is_some() {
        return Err(format!(
            "{PART}, `{name}`: a shot is one frame: no --count (frames over time are the \
             preview's)"
        ));
    }
    Ok(options)
}

/// Whether `shots/<name>.png` in the root of `project` is what shot `name`
/// of its `shots` part makes with `game`: the same picture, pixel for
/// pixel, in at most [`MAX_SHOT_BYTES`].
///
/// # Errors
/// The shot's errors ([`shot_options`]), its target's or the game's, or the
/// first difference, named: no file, a file too large (its size), another
/// size, or the first pixel that differs (its x and y). Each of the file's
/// ends with the command that remakes it.
pub fn check_shot(project: &Project, game: &dyn GameTools, name: &str) -> Result<(), String> {
    let options = shot_options(project, name, game.frame_options())?;
    let target = match options.target.as_deref() {
        None => None,
        Some(id) => Some(
            contract::target::profile(&project.config, &project.root, id)
                .map_err(|(_, e)| format!("{PART}, `{name}`: {e}"))?
                .profile,
        ),
    };
    let want = super::frame_of(&options, game, target.as_ref())?.image;
    let shown = file(name);
    let remake = |problem: String| remake(project, &format!("--shot {name}"), &problem);
    let path = project.root.join(&shown);
    let bytes = fs::read(&path).map_err(|e| remake(format!("{}: {e}", path.display())))?;
    let len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    check_size(&shown, len, MAX_SHOT_BYTES, "a shot", "try --scale 1").map_err(remake)?;
    let (width, height, rgba) = decode_png(&bytes).map_err(|e| remake(format!("{shown}: {e}")))?;
    if (width, height) != (want.width, want.height) {
        return Err(remake(format!(
            "{shown} is stale: it is {width}×{height}, and {PART} makes `{name}` {}×{}",
            want.width, want.height
        )));
    }
    let have = Image {
        width,
        height,
        rgba,
    };
    match first_difference(&have, &want) {
        None => Ok(()),
        Some((x, y)) => Err(remake(format!(
            "{shown} is stale: its pixels differ from what {PART} makes of `{name}`, first at \
             ({x}, {y})"
        ))),
    }
}

/// Whether every shot of `project` is what its arguments make with `game`
/// ([`check_shot`]), and `shots/` holds nothing else ([`orphans`]).
///
/// # Errors
/// Every problem found, one after another, each ending with the command
/// that remakes its file.
pub fn check_shots(project: &Project, game: &dyn GameTools) -> Result<(), String> {
    let mut problems: Vec<String> = project
        .config
        .shots
        .keys()
        .filter_map(|name| check_shot(project, game, name).err())
        .collect();
    problems.extend(
        orphans(project)?
            .iter()
            .map(|orphan| orphan_problem(orphan)),
    );
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n\n"))
    }
}

/// What `shots/` in the root of `project` holds that no shot names, from
/// the root (`shots/old.png`), in order; none when there is no folder.
///
/// # Errors
/// The folder doesn't read.
pub fn orphans(project: &Project) -> Result<Vec<String>, String> {
    let dir = project.root.join(FOLDER);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{}: {e}", dir.display())),
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let shot = name
            .strip_suffix(".png")
            .is_some_and(|stem| project.config.shots.contains_key(stem));
        if !shot {
            found.push(format!("{FOLDER}/{name}"));
        }
    }
    found.sort();
    Ok(found)
}

/// What a file of `shots/` that no shot names is: `orphan` (from the
/// root), and what to do with it. No tool removes it: it may be the
/// game's own.
pub fn orphan_problem(orphan: &str) -> String {
    format!(
        "{orphan}: no shot of {PART} makes it: remove it, or name it in `shots`; {FOLDER}/ \
         holds the shots alone"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::project::Config;

    fn project(shots: &[(&str, &[&str])]) -> Project {
        let shots = shots
            .iter()
            .map(|(name, args)| {
                let args = args.iter().map(|a| (*a).to_owned()).collect();
                ((*name).to_owned(), Shot::new(args))
            })
            .collect();
        let config = Config {
            command: Some("cargo shuyi".to_owned()),
            shots,
            ..Config::default()
        };
        Project::new("game", config)
    }

    fn error(name: &str, args: &[&str]) -> String {
        shot_options(&project(&[("menu", args)]), name, &[]).unwrap_err()
    }

    #[test]
    fn a_shots_options_are_its_args_written_to_its_file() {
        let shots = project(&[(
            "menu",
            &["--keys", "Up", "--scale", "1", "--target", "playdate"],
        )]);
        let options = shot_options(&shots, "menu", &[]).unwrap();
        assert_eq!(options.out, std::path::Path::new("shots/menu.png"));
        assert_eq!(options.scale, Some(1));
        assert_eq!(options.target.as_deref(), Some("playdate"));
        assert_eq!(options.script.steps.len(), 1);
        assert_eq!(options.capture, None);
    }

    #[test]
    fn each_error_of_the_options_names_the_part_and_the_shot() {
        assert_eq!(
            error("Menu", &[]),
            "shuyi.ron's `shots`: `Menu` is no shot's name: lowercase letters, digits and - only"
        );
        assert_eq!(
            error("title", &[]),
            "shuyi.ron's `shots` has no `title`: it has menu"
        );
        assert_eq!(
            shot_options(&project(&[]), "title", &[]).unwrap_err(),
            "shuyi.ron's `shots` has no `title`: it has none"
        );
        assert_eq!(
            error("menu", &["--wait", "soon"]),
            "shuyi.ron's `shots`, `menu`: --wait: not a number of seconds: soon"
        );
        assert_eq!(
            error("menu", &["out.png"]),
            "shuyi.ron's `shots`, `menu`, names an output path, out.png: a shot is always \
             shots/menu.png, in the project's root"
        );
        assert!(
            error("menu", &["--every", "0.1", "--count", "2", "--apng"])
                .starts_with("shuyi.ron's `shots`, `menu`: a shot is one frame: no --count")
        );
    }

    #[test]
    fn the_size_is_checked_against_the_cap() {
        let advice = "try --scale 1";
        assert!(check_size("shots/a.png", 2048, 2048, "a shot", advice).is_ok());
        assert_eq!(
            check_size("shots/a.png", 2049, 2048, "a shot", advice).unwrap_err(),
            "shots/a.png is 3 KiB, more than the 2 KiB a shot may be: try --scale 1"
        );
        assert_eq!(MAX_SHOT_BYTES, 262_144);
    }

    #[test]
    fn the_first_difference_is_its_x_and_y() {
        let want = Image {
            width: 3,
            height: 2,
            rgba: vec![7; 24],
        };
        assert_eq!(first_difference(&want, &want), None);
        let mut have = want.clone();
        have.rgba[4 * 4 + 2] = 0;
        have.rgba[4 * 5] = 0;
        assert_eq!(first_difference(&have, &want), Some((1, 1)));
    }

    #[test]
    fn the_remake_line_ends_the_problem() {
        assert_eq!(
            remake(&project(&[]), "--shot menu", "shots/menu.png is stale"),
            "shots/menu.png is stale\nremake it: cargo shuyi frame-png --shot menu"
        );
    }
}
