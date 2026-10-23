//! `check [--baked]`: the game loads all its content through every
//! validator it has, and the answer lists every error found, each with its
//! file, line and message. With `--baked`, the game's sources are baked
//! into a pack too, and every asset read from the pack must equal the one
//! read from its source ([`bake::check_baked`]). It passes when there are
//! no errors.
//!
//! `check --target <id>` checks instead that the game fits target `id`,
//! or every target its `shuyi.ron` lists with `--target all`
//! ([`super::target`]). The display needs only the project file; the
//! controls, and a one-bit target's patterns, need the game's tools too,
//! and without them the answer says they weren't checked. It passes when
//! no target has an error; warnings don't fail it.

use std::fmt::Write as _;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_assets::target::Color;

use super::target::{self, Level, TargetReport};
use super::{Answer, Op, Problem, bake, codes};
use crate::game::{GameTools, code_of};
use crate::project::{Config, FILE};

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
/// its pack differs from its sources. A target is checked by [`targets`].
pub fn check(game: &dyn GameTools, input: &CheckInput) -> Answer<CheckOutput> {
    if input.target.is_some() {
        return Answer::failed(
            Op::Check,
            codes::WRONG_ARGUMENTS,
            "check --target is checked by check::targets, with the project file",
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
        Err(e) => Answer::failed(Op::Check, code_of(&e), e),
    }
}

/// `check --target`: how the game of `config` (the project at `root`)
/// fits the targets `input` asks for; its controls and a one-bit target's
/// patterns too with its tools, `game`.
pub fn targets(
    config: &Config,
    root: &Path,
    input: &CheckInput,
    game: Option<&dyn GameTools>,
) -> Answer<CheckOutput> {
    let Some(asked) = &input.target else {
        return Answer::failed(Op::Check, codes::WRONG_ARGUMENTS, "no target asked for");
    };
    let settings = match target::asked(config, asked) {
        Ok(settings) => settings,
        Err(e) => return Answer::failed(Op::Check, codes::TARGET, e),
    };
    let profiles = match target::profiles(config, root) {
        Ok(profiles) => profiles,
        Err(errors) => {
            let shown: Vec<String> = errors.iter().map(ToString::to_string).collect();
            return Answer::failed(
                Op::Check,
                codes::TARGET,
                format!("the target profiles don't load: {}", shown.join("; ")),
            );
        }
    };
    let mut reports = Vec::new();
    let mut limits = Vec::new();
    let controls = match game.map(GameTools::controls) {
        Some(Some(Ok(controls))) => Some(controls),
        Some(Some(Err(e))) => {
            let error = format!("the game's controls don't load: {e}");
            return Answer::failed(Op::Check, code_of(&e), error);
        }
        Some(None) => {
            limits.push(
                "the controls aren't checked: the game's tools don't offer them \
                 (GameTools::controls)"
                    .to_owned(),
            );
            None
        }
        None => {
            limits.push(format!(
                "the controls aren't checked: they need the game's tools, and {FILE} names no \
                 game tool program (`game`)"
            ));
            None
        }
    };
    for (id, setting) in settings {
        let Some(profile) = profiles.get(id) else {
            let known: Vec<&str> = profiles.keys().map(String::as_str).collect();
            return Answer::failed(
                Op::Check,
                codes::TARGET,
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
        let mut problems = target::check_target(id, setting, &profile.profile);
        if let Some(controls) = &controls {
            problems.extend(target::controls(id, controls.fit(&profile.profile)));
        }
        if profile.profile.display.color == Color::OneBit {
            match game.map(|game| game.one_bit(id)) {
                Some(Some(Ok(skin))) => problems.extend(target::one_bit(&skin)),
                Some(Some(Err(e))) => {
                    return Answer::failed(
                        Op::Check,
                        code_of(&e),
                        format!("the game's one-bit skin for {id} doesn't load: {e}"),
                    );
                }
                Some(None) => limits.push(format!(
                    "{id}'s one-bit patterns aren't checked: the game's tools don't offer them \
                     (GameTools::one_bit)"
                )),
                None => limits.push(format!(
                    "{id}'s one-bit patterns aren't checked: they need the game's tools, and \
                     {FILE} names no game tool program (`game`)"
                )),
            }
        }
        reports.push(TargetReport {
            target: id.to_owned(),
            profile: profile.file.clone(),
            problems,
        });
    }
    limits.push(
        "only the frame's fit, the text's size, the controls and one-bit patterns are checked yet"
            .to_owned(),
    );
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
    out.push_str(&codes::help_lines(&output.errors));
    out
}

#[cfg(test)]
mod tests {
    use shuyi_assets::ContentError;

    use shuyi_assets::onebit::{Pattern, PatternMapDef};
    use shuyi_assets::palette::PaletteDef;

    use super::*;
    use crate::contract::fake::Fake;
    use crate::contract::target::TargetCheck;
    use crate::game::OneBitSkin;

    #[test]
    fn check_passes_with_no_error_and_lists_each_with_where() {
        let answer = check(&Fake::default(), &CheckInput::default());
        assert!(answer.ok);
        assert_eq!(answer.op, Op::Check);
        assert!(answer.limits.is_empty(), "{:?}", answer.limits);
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
        let got = check(&lined, &CheckInput::default()).limits;
        assert!(got.is_empty(), "{got:?}");
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
        // Targets are `targets`'s.
        let answer = check(&Fake::default(), &on("web"));
        assert!(answer.error.unwrap().contains("check::targets"));
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
        let deck = targets(&config, root, &on("steam-deck"), None);
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
            [
                "the controls aren't checked: they need the game's tools, and shuyi.ron names no \
                 game tool program (`game`)",
                "only the frame's fit, the text's size, the controls and one-bit patterns are \
                 checked yet"
            ]
        );
        let all = targets(&config, root, &on("all"), None);
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
        assert_eq!(all.limits.len(), 4, "desktop takes any size");
        assert_eq!(
            all.limits[2],
            "playdate's one-bit patterns aren't checked: they need the game's tools, and \
             shuyi.ron names no game tool program (`game`)"
        );
        // A target not listed, or listed with no profile.
        assert!(targets(&config, root, &on("web"), None).error.is_some());
        assert!(
            targets(&config, root, &CheckInput::default(), None)
                .error
                .is_some()
        );
        let unknown = crate::project::Config::parse(
            r#"(targets: {"moon": (frame: (1, 1), scale: fit, text_px: 9)})"#,
        )
        .unwrap();
        let answer = targets(&unknown, root, &on("moon"), None);
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
        let answer = targets(&broken, root, &on("all"), None);
        assert!(
            answer
                .error
                .unwrap()
                .starts_with("the target profiles don't load: no-such-folder: ")
        );
    }

    #[test]
    fn with_the_games_tools_its_controls_are_checked_on_each_target() {
        let config = crate::project::Config::parse(
            r#"(targets: {
    "playdate": (frame: (400, 240), scale: integer, text_px: 16),
    "desktop": (frame: (400, 240), scale: integer, text_px: 16),
})"#,
        )
        .unwrap();
        let root = Path::new(".");
        let game = Fake {
            controls: Some(Ok(())),
            ..Fake::default()
        };
        let all = targets(&config, root, &on("all"), Some(&game));
        assert!(!all.ok, "the Playdate has no table");
        let out = all.result.as_ref().unwrap();
        let desktop = &out.targets[0];
        assert!(!desktop.failed(), "a keyboard reaches everything");
        assert_eq!(desktop.problems.len(), 1);
        let warning = &desktop.problems[0];
        assert_eq!(
            (warning.level, warning.check, warning.setting.as_str()),
            (Level::Warning, TargetCheck::Controls, "screen \"title\"")
        );
        assert_eq!(warning.screen.as_deref(), Some("title"));
        let playdate = &out.targets[1];
        assert_eq!(playdate.problems.len(), 2);
        let error = &playdate.problems[1];
        assert_eq!(
            (error.level, error.setting.as_str()),
            (Level::Error, "keymap targets.\"playdate\"")
        );
        assert!(
            text(out).contains(
                "  error: playdate: keymap targets.\"playdate\": the keymap file has no buttons \
                 for playdate, which has no keyboard"
            ),
            "{}",
            text(out)
        );
        assert!(!all.limits.iter().any(|l| l.contains("controls aren't")));
        // A game whose tools don't offer them: said, and the display alone.
        let answer = targets(&config, root, &on("playdate"), Some(&Fake::default()));
        assert!(answer.ok);
        assert_eq!(
            answer.limits[0],
            "the controls aren't checked: the game's tools don't offer them (GameTools::controls)"
        );
        // A game whose controls don't load fails.
        let broken = Fake {
            controls: Some(Err("keymap.ron: bad".to_owned())),
            ..Fake::default()
        };
        let answer = targets(&config, root, &on("playdate"), Some(&broken));
        assert!(!answer.ok);
        assert_eq!(
            answer.error.as_deref(),
            Some("the game's controls don't load: keymap.ron: bad")
        );
    }

    /// A skin of three teams, two of them on one pattern, and text in a
    /// dither.
    fn teams() -> OneBitSkin {
        let palette = PaletteDef::from_source(
            "files/palette.ron",
            r##"{"text": "#ffffff", "team.player": "#0000ff", "team.enemy": "#ff0000"}"##,
            &[],
        )
        .unwrap();
        let map = PatternMapDef::from_source(
            "files/palette@onebit.ron",
            r#"(
                roles: {
                    "text": (fg: Dither75, bg: Black),
                    "team.player": (fg: Dither50),
                    "team.enemy": (fg: Dither50),
                },
                distinct: [["team.player", "team.enemy"]],
                text: ["text"],
            )"#,
        )
        .unwrap();
        OneBitSkin {
            palette_file: "files/palette.ron".to_owned(),
            palette,
            map_file: "files/palette@onebit.ron".to_owned(),
            map,
        }
    }

    #[test]
    fn on_a_one_bit_target_the_roles_that_tell_things_apart_must_stay_distinct() {
        let config = crate::project::Config::parse(
            r#"(targets: {
    "playdate": (frame: (400, 240), scale: integer, text_px: 16),
    "desktop": (frame: (400, 240), scale: integer, text_px: 16),
})"#,
        )
        .unwrap();
        let root = Path::new(".");
        let game = Fake {
            one_bit: Some(Ok(teams())),
            ..Fake::default()
        };
        let all = targets(&config, root, &on("all"), Some(&game));
        assert!(!all.ok, "two teams look alike on the Playdate");
        let out = all.result.as_ref().unwrap();
        assert!(out.targets[0].problems.is_empty(), "the desktop has colour");
        let playdate = &out.targets[1].problems;
        assert_eq!(playdate.len(), 2, "{playdate:?}");
        let distinct = &playdate[0];
        assert_eq!(
            (distinct.level, distinct.check, distinct.setting.as_str()),
            (
                Level::Error,
                TargetCheck::OneBit,
                "files/palette@onebit.ron"
            )
        );
        assert_eq!(distinct.roles, ["team.player", "team.enemy"]);
        assert_eq!(
            (playdate[1].level, playdate[1].roles.as_slice()),
            (Level::Warning, ["text".to_owned()].as_slice())
        );
        assert!(
            text(out).contains(
                "  error: playdate: files/palette@onebit.ron: roles \"team.player\" and \
                 \"team.enemy\" must stay distinct, but their foregrounds look the same in one \
                 bit (both Dither50)"
            ),
            "{}",
            text(out)
        );
        assert!(
            !all.limits
                .iter()
                .any(|l| l.contains("one-bit patterns aren't"))
        );
        // Kept apart, it passes; the text's warning doesn't fail it.
        let mut apart = teams();
        apart.map.roles.get_mut("team.player").unwrap().fg = Pattern::WHITE;
        let game = Fake {
            one_bit: Some(Ok(apart)),
            ..Fake::default()
        };
        assert!(targets(&config, root, &on("playdate"), Some(&game)).ok);
        // A game whose tools don't offer them: said.
        let answer = targets(&config, root, &on("playdate"), Some(&Fake::default()));
        assert!(answer.ok);
        assert_eq!(
            answer.limits[1],
            "playdate's one-bit patterns aren't checked: the game's tools don't offer them \
             (GameTools::one_bit)"
        );
        // A skin that doesn't load fails.
        let broken = Fake {
            one_bit: Some(Err("palette@onebit.ron: bad".to_owned())),
            ..Fake::default()
        };
        let answer = targets(&config, root, &on("playdate"), Some(&broken));
        assert_eq!(
            answer.error.as_deref(),
            Some("the game's one-bit skin for playdate doesn't load: palette@onebit.ron: bad")
        );
        // The desktop never asks.
        assert!(targets(&config, root, &on("desktop"), Some(&broken)).ok);
    }
}
