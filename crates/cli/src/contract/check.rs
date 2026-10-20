//! `check [--baked]`: the game loads all its content through every
//! validator it has, and the answer lists every error found, each with its
//! file, line and message. With `--baked`, the game's sources are baked
//! into a pack too, and every asset read from the pack must equal the one
//! read from its source ([`bake::check_baked`]). It passes when there are
//! no errors.
//!
//! `check --target <id>` checks instead that the game fits target `id`,
//! or every target its `shuyi.ron` lists with `--target all`
//! ([`super::target`]). It needs only the project file, not the game's
//! tools, and passes when no target has an error; warnings don't fail it.

use std::fmt::Write as _;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::target::{self, Level, TargetReport};
use super::{Answer, Op, Problem, bake};
use crate::game::GameTools;
use crate::project::Config;

/// What `check` checks: everything the game loads (with `baked`, its pack
/// too), or with `target`, how the game fits a target.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckInput {
    /// The target to check the game against, by its profile's id, or
    /// `all` for every target in the game's `shuyi.ron`. Without it, the
    /// game's content is checked.
    #[serde(default)]
    pub target: Option<String>,
    /// Whether to bake the game's pack too, and check every asset read
    /// from it against the one read from its source.
    #[serde(default)]
    pub baked: bool,
}

/// What `check` found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckOutput {
    /// Every error of the game's content, in the order the game's
    /// validators found them (none with a target).
    pub errors: Vec<Problem>,
    /// Each target checked, with its problems (none without a target).
    #[serde(default)]
    pub targets: Vec<TargetReport>,
}

/// `check`: every error of `game`'s content, and with `baked`, every way
/// its pack differs from its sources. A target is checked by [`targets`],
/// on the project file; the game's tools can't.
pub fn check(game: &dyn GameTools, input: &CheckInput) -> Answer<CheckOutput> {
    if input.target.is_some() {
        return Answer::failed(
            Op::Check,
            "check --target runs on the project file, not in the game's tools",
        );
    }
    let found = game.check().and_then(|mut errors| {
        if input.baked {
            errors.extend(bake::check_baked(game)?);
        }
        Ok(errors)
    });
    match found {
        Ok(errors) => {
            let errors: Vec<Problem> = errors.iter().map(Problem::from).collect();
            let mut limits = Vec::new();
            if errors.iter().any(|e| e.line.is_none()) {
                limits.push(
                    "some errors are about a whole file: their validator gives no line".to_owned(),
                );
            }
            let output = CheckOutput {
                errors,
                targets: Vec::new(),
            };
            Answer::new(Op::Check, output.errors.is_empty(), output, limits)
        }
        Err(e) => Answer::failed(Op::Check, e),
    }
}

/// `check --target`: how the game of `config` (the project at `root`)
/// fits the targets `input` asks for.
pub fn targets(config: &Config, root: &Path, input: &CheckInput) -> Answer<CheckOutput> {
    let Some(asked) = &input.target else {
        return Answer::failed(Op::Check, "no target asked for");
    };
    let settings = match target::asked(config, asked) {
        Ok(settings) => settings,
        Err(e) => return Answer::failed(Op::Check, e),
    };
    let profiles = match target::profiles(config, root) {
        Ok(profiles) => profiles,
        Err(errors) => {
            let shown: Vec<String> = errors.iter().map(ToString::to_string).collect();
            return Answer::failed(
                Op::Check,
                format!("the target profiles don't load: {}", shown.join("; ")),
            );
        }
    };
    let mut reports = Vec::new();
    let mut limits = Vec::new();
    for (id, setting) in settings {
        let Some(profile) = profiles.get(id) else {
            let known: Vec<&str> = profiles.keys().map(String::as_str).collect();
            return Answer::failed(
                Op::Check,
                format!(
                    "no target profile \"{id}\"; the profiles are {}",
                    known.join(", ")
                ),
            );
        };
        if profile.profile.display.sizes.is_empty() {
            limits.push(format!(
                "{id}'s display takes any size: its frame fits, and its text is checked at scale 1"
            ));
        }
        reports.push(TargetReport {
            target: id.to_owned(),
            profile: profile.file.clone(),
            problems: target::check_target(id, setting, &profile.profile),
        });
    }
    limits.push("only the frame's fit and the text's size are checked yet".to_owned());
    let ok = !reports.iter().any(TargetReport::failed);
    let output = CheckOutput {
        errors: Vec::new(),
        targets: reports,
    };
    Answer::new(Op::Check, ok, output, limits)
}

/// `check`'s arguments (`--json` taken out): none, `--baked`, or
/// `--target <id>`.
pub fn parse_args(args: &[String]) -> Result<CheckInput, String> {
    match args {
        [] => Ok(CheckInput::default()),
        [flag] if flag == "--baked" => Ok(CheckInput {
            baked: true,
            ..CheckInput::default()
        }),
        [flag, target] if flag == "--target" => Ok(CheckInput {
            target: Some(target.clone()),
            ..CheckInput::default()
        }),
        _ => Err("no arguments, --baked, or --target <id | all>".to_owned()),
    }
}

/// `check`'s usage, with `command` as the tool's name.
pub fn usage(command: &str) -> String {
    format!("usage: {command} check [--baked | --target <id | all>] [--json]")
}

/// What `check --target` found, for people: a line a target, then its
/// problems.
fn targets_text(reports: &[TargetReport]) -> String {
    let mut out = String::new();
    for report in reports {
        let count = |level| report.problems.iter().filter(|p| p.level == level).count();
        let (errors, warnings) = (count(Level::Error), count(Level::Warning));
        // Writing to a `String` can't fail.
        let _ = if errors + warnings == 0 {
            writeln!(
                out,
                "check --target {}: OK ({})",
                report.target, report.profile
            )
        } else {
            writeln!(
                out,
                "check --target {}: {errors} error(s), {warnings} warning(s) ({})",
                report.target, report.profile
            )
        };
        for problem in &report.problems {
            let level = match problem.level {
                Level::Error => "error",
                Level::Warning => "warning",
            };
            let _ = writeln!(
                out,
                "  {level}: {}: {}: {}",
                report.target, problem.setting, problem.message
            );
        }
    }
    out
}

/// What `check` found, for people.
pub fn text(output: &CheckOutput) -> String {
    if !output.targets.is_empty() {
        return targets_text(&output.targets);
    }
    if output.errors.is_empty() {
        return "check: OK\n".to_owned();
    }
    let mut out = format!("check: {} error(s)\n", output.errors.len());
    for error in &output.errors {
        // Writing to a `String` can't fail.
        let _ = writeln!(out, "  {error}");
    }
    out
}

#[cfg(test)]
mod tests {
    use shuyi_assets::ContentError;

    use super::*;
    use crate::contract::fake::Fake;

    #[test]
    fn check_passes_with_no_error_and_lists_each_with_where() {
        let answer = check(&Fake::default(), &CheckInput::default());
        assert!(answer.ok);
        assert_eq!(answer.op, Op::Check);
        assert!(answer.limits.is_empty());
        assert_eq!(text(answer.result.as_ref().unwrap()), "check: OK\n");
        let game = Fake {
            check: Ok(vec![
                ContentError::new("a.ron", "bad").at(2, Some(5)),
                ContentError::new("b.map", "missing"),
            ]),
            ..Fake::default()
        };
        let answer = check(&game, &CheckInput::default());
        assert!(!answer.ok);
        assert_eq!(answer.exit_code(), 1);
        let out = answer.result.as_ref().unwrap();
        assert_eq!(out.errors[0].line, Some(2));
        assert_eq!(
            answer.limits,
            ["some errors are about a whole file: their validator gives no line"]
        );
        assert_eq!(
            text(out),
            "check: 2 error(s)\n  a.ron:2:5: bad\n  b.map: missing\n"
        );
        // Errors that all have a line need no limit.
        let lined = Fake {
            check: Ok(vec![ContentError::new("a.ron", "bad").at(1, None)]),
            ..Fake::default()
        };
        assert!(check(&lined, &CheckInput::default()).limits.is_empty());
    }

    #[test]
    fn a_game_that_cant_check_says_why() {
        let game = Fake {
            check: Err("no data".to_owned()),
            ..Fake::default()
        };
        let answer = check(&game, &CheckInput::default());
        assert_eq!(answer.error.as_deref(), Some("no data"));
    }

    fn on(target: &str) -> CheckInput {
        CheckInput {
            target: Some(target.to_owned()),
            ..CheckInput::default()
        }
    }

    #[test]
    fn check_takes_no_argument_baked_or_a_target() {
        let args =
            |items: &[&str]| -> Vec<String> { items.iter().map(|s| (*s).to_owned()).collect() };
        assert_eq!(parse_args(&[]), Ok(CheckInput::default()));
        assert_eq!(parse_args(&args(&["--target", "web"])), Ok(on("web")));
        let baked = CheckInput {
            baked: true,
            ..CheckInput::default()
        };
        assert_eq!(parse_args(&args(&["--baked"])), Ok(baked));
        assert!(parse_args(&args(&["--target"])).is_err());
        assert!(parse_args(&args(&["web"])).is_err());
        assert!(parse_args(&args(&["--targets", "web"])).is_err());
        assert!(parse_args(&args(&["--baked", "--target", "web"])).is_err());
        assert_eq!(
            usage("shuyi"),
            "usage: shuyi check [--baked | --target <id | all>] [--json]"
        );
        // The game's tools check content, not targets.
        let answer = check(&Fake::default(), &on("web"));
        assert!(answer.error.unwrap().contains("project file"));
    }

    #[test]
    fn targets_are_checked_one_or_all_and_warnings_dont_fail() {
        let config = crate::project::Config::parse(
            r#"(targets: {
    "playdate": (frame: (800, 512), scale: integer, text_px: 16),
    "steam-deck": (frame: (800, 512), scale: integer, text_px: 10),
    "desktop": (frame: (800, 512), scale: integer, text_px: 16),
})"#,
        )
        .unwrap();
        let root = Path::new(".");
        let deck = targets(&config, root, &on("steam-deck"));
        assert!(deck.ok, "a warning passes");
        let out = deck.result.as_ref().unwrap();
        assert_eq!(out.targets.len(), 1);
        assert_eq!(out.targets[0].profile, "targets/steam-deck.ron");
        assert_eq!(
            text(out),
            "check --target steam-deck: 0 error(s), 1 warning(s) (targets/steam-deck.ron)\n  \
             warning: steam-deck: targets.\"steam-deck\".text_px: text 10 px at scale 1 is 10 px \
             on 1280 × 800, below the 12 px the target recommends\n"
        );
        assert_eq!(
            deck.limits,
            ["only the frame's fit and the text's size are checked yet"]
        );
        let all = targets(&config, root, &on("all"));
        assert!(!all.ok, "the Playdate fails");
        let out = all.result.as_ref().unwrap();
        let ids: Vec<&str> = out.targets.iter().map(|r| r.target.as_str()).collect();
        assert_eq!(ids, ["desktop", "playdate", "steam-deck"]);
        assert!(
            text(out).starts_with(
                "check --target desktop: OK (targets/desktop.ron)\ncheck --target playdate: 1 \
                 error(s), 0 warning(s)"
            ),
            "{}",
            text(out)
        );
        assert_eq!(all.limits.len(), 2, "desktop takes any size");
        // A target not listed, or listed with no profile.
        assert!(targets(&config, root, &on("web")).error.is_some());
        assert!(
            targets(&config, root, &CheckInput::default())
                .error
                .is_some()
        );
        let unknown = crate::project::Config::parse(
            r#"(targets: {"moon": (frame: (1, 1), scale: fit, text_px: 9)})"#,
        )
        .unwrap();
        let answer = targets(&unknown, root, &on("moon"));
        assert!(
            answer
                .error
                .unwrap()
                .starts_with("no target profile \"moon\"; the profiles are android, desktop")
        );
        let broken = crate::project::Config {
            target_profiles: Some("no-such-folder".to_owned()),
            ..config
        };
        let answer = targets(&broken, root, &on("all"));
        assert!(
            answer
                .error
                .unwrap()
                .starts_with("the target profiles don't load: no-such-folder: ")
        );
    }
}
