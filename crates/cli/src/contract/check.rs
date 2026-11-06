//! `check [--baked]`: the game loads all its content through every
//! validator it has, and the answer lists every error found, each with its
//! file, line and message. With `--baked`, the game's sources are baked
//! into a pack too, and every asset read from the pack must equal the one
//! read from its source ([`bake::check_baked`]). Once the content loads
//! without errors, the game's text is checked against its font atlas
//! ([`GameTools::font_atlas`], `shuyi_assets::font::undrawn`): a character
//! the atlas can't draw is an error. It passes when there are no errors.
//!
//! `check --target <id>` checks instead that the game fits target `id`,
//! or every target its `shuyi.ron` lists with `--target all`
//! ([`super::target`]). The display and who the game is (`shuyi.ron`'s
//! `app`) need only the project file; the
//! controls, and a one-bit target's patterns, need the game's tools too,
//! and without them the answer says they weren't checked. It passes when
//! no target has an error; warnings don't fail it.

use std::fmt::Write as _;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_assets::ContentError;
use shuyi_assets::font;
use shuyi_assets::target::Color;

use super::codes::Code;
use super::target::{self, Level, TargetReport};
use super::{Answer, Op, Problem, bake, codes};
use crate::app_info;
use crate::game::{GameTools, code_of};
use crate::project::{Config, FILE};

/// What `check` checks: everything the game loads (with `baked`, its pack
/// too), or with `target`, how the game fits a target.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An input: built with `default()` and its fields assigned, it may grow
// (ADR-0039).
#[non_exhaustive]
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
        let loaded = errors.is_empty();
        if input.baked {
            errors.extend(bake::check_baked(game)?);
        }
        Ok((errors, loaded))
    });
    match found {
        Ok((mut errors, loaded)) => {
            let mut atlas_limit = None;
            if loaded {
                match undrawn(game) {
                    Ok(Some(found)) => errors.extend(found),
                    Ok(None) => atlas_limit = Some(ATLAS_NOT_OFFERED),
                    Err((code, e)) => return Answer::failed(Op::Check, code, e),
                }
            } else {
                atlas_limit = Some(ATLAS_AFTER_ERRORS);
            }
            let errors: Vec<Problem> = errors.iter().map(Problem::from).collect();
            let mut limits = Vec::new();
            if errors.iter().any(|e| e.line.is_none()) {
                limits.push(
                    "some errors are about a whole file: their validator gives no line".to_owned(),
                );
            }
            limits.extend(atlas_limit.map(str::to_owned));
            let output = CheckOutput {
                errors,
                targets: Vec::new(),
            };
            Answer::new(Op::Check, output.errors.is_empty(), output, limits)
        }
        Err(e) => Answer::failed(Op::Check, code_of(&e), e),
    }
}

/// `check`'s limit when the game's tools don't offer its font atlas.
const ATLAS_NOT_OFFERED: &str = "the text isn't checked against the font atlas: the game's tools \
                                 don't offer it (GameTools::font_atlas)";

/// `check`'s limit when the content has errors.
const ATLAS_AFTER_ERRORS: &str =
    "the text is checked against the font atlas once the content loads without errors";

/// Every character of `game`'s text its font atlas can't draw
/// ([`font::undrawn`]); `None` if its tools don't offer the atlas, and the
/// code and message to fail with if the atlas or the text doesn't load.
fn undrawn(game: &dyn GameTools) -> Result<Option<Vec<ContentError>>, (Code, String)> {
    let atlas = match game.font_atlas() {
        None => return Ok(None),
        Some(Ok(atlas)) => atlas,
        Some(Err(e)) => {
            return Err((
                code_of(&e),
                format!("the game's font atlas doesn't load: {e}"),
            ));
        }
    };
    let lang = game
        .lang()
        .map_err(|e| (code_of(&e), format!("the game's text doesn't load: {e}")))?;
    Ok(Some(font::undrawn(&atlas, &lang.lang)))
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
    let app = config
        .app
        .as_ref()
        .map(|part| app_info::problems(part, root, config.command()));
    for (id, setting) in settings {
        let profile = match target::profile(config, root, id) {
            Ok(profile) => profile,
            Err((code, why)) => return Answer::failed(Op::Check, code, why),
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
        if let Some(app) = &app {
            problems.extend(target::app(app.clone()));
        }
        reports.push(TargetReport {
            target: id.to_owned(),
            profile: profile.file.clone(),
            problems,
        });
    }
    if app.is_none() {
        limits.push(format!("the identity isn't checked: {FILE} has no `app`"));
    }
    limits.push(
        "only the frame's fit, the text's size, the controls, one-bit patterns and the identity \
         are checked yet"
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
    use shuyi_assets::font::FontAtlasDef;
    use shuyi_assets::lang::Lang;
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
            [
                "some errors are about a whole file: their validator gives no line",
                ATLAS_AFTER_ERRORS
            ]
        );
        assert_eq!(
            text(out),
            "check: 2 error(s)\n  a.ron:2:5: bad\n  b.map: missing\n"
        );
        // Errors that all have a line need no limit of their own.
        let lined = Fake {
            check: Ok(vec![ContentError::new("a.ron", "bad").at(1, None)]),
            ..Fake::default()
        };
        let got = check(&lined, &CheckInput::default()).limits;
        assert_eq!(
            got,
            ["the text is checked against the font atlas once the content loads without errors"]
        );
    }

    /// English's `menu.title`, and the pack `ru`, whose `menu.title` is
    /// written `title`.
    fn russian(title: &str) -> Lang {
        one_pack("ru", r#"(name: "Russkij", made_by: Machine)"#, title)
    }

    /// English's `menu.title`, and the pack `code` described by `info` (its
    /// `lang.ron`), whose `menu.title` is written `title`.
    fn one_pack(code: &str, info: &str, title: &str) -> Lang {
        use shuyi_assets::lang::{LangFile, PackRules, from_files};
        let ui = format!(r#"[(key: "menu.title", source: "Start", text: "{title}")]"#);
        let (info_path, ui_path) = (format!("{code}/lang.ron"), format!("{code}/ui.ron"));
        let files = [
            ("en/ui.ron", r#"{"menu.title": "Start"}"#),
            (info_path.as_str(), info),
            (ui_path.as_str(), ui.as_str()),
        ]
        .map(|(path, source)| LangFile {
            path,
            source: Some(source),
        });
        let rules = PackRules {
            data_problem: |_, _| None,
            tokens: &shuyi_assets::dialogue::NoTokens,
            limits: shuyi_assets::dialogue::DialogueLimits {
                box_width: 40,
                box_lines: 3,
                max_text: 200,
                max_option: 30,
                min_options: 2,
                max_options: 4,
                max_reaction_texts: 3,
            },
            widths: &shuyi_assets::font::CellWidths::NARROW,
            variants: &[],
        };
        from_files(&files, None, str::to_owned, &rules).unwrap()
    }

    /// An atlas of ASCII's printable characters, and `extra`.
    fn ascii_and(extra: &str) -> FontAtlasDef {
        FontAtlasDef {
            cell_w: 8,
            cell_h: 16,
            columns: 16,
            glyphs: (' '..='~').chain(extra.chars()).zip(0..).collect(),
            ..FontAtlasDef::default()
        }
    }

    #[test]
    fn check_counts_a_wide_glyph_as_drawn_and_fails_on_one_the_atlas_lacks() {
        let game = Fake {
            lang: Ok(russian("日a")),
            font_atlas: Some(Ok(ascii_and(""))),
            ..Fake::default()
        };
        let answer = check(&game, &CheckInput::default());
        assert!(!answer.ok);
        let errors = &answer.result.as_ref().unwrap().errors;
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0]
                .message
                .starts_with("'日' (U+65E5) isn't in the font atlas")
        );
        let mut atlas = ascii_and("");
        atlas.wide.insert('日', 96);
        let drawn = Fake {
            font_atlas: Some(Ok(atlas)),
            ..game
        };
        let answer = check(&drawn, &CheckInput::default());
        assert!(answer.ok, "{:?}", answer.result);
    }

    #[test]
    fn check_fails_on_a_wide_character_the_set_a_pack_names_lacks() {
        // 简 is in the atlas, in the Chinese set, but not in the Japanese
        // set the pack draws from.
        let mut atlas = ascii_and("");
        atlas.wide_sets = [
            ("ja".to_owned(), [('直', 96)].into()),
            ("zh-Hans".to_owned(), [('直', 98), ('简', 100)].into()),
        ]
        .into();
        let info =
            |set: &str| format!(r#"(name: "Pack", made_by: Machine, wide_font: Some("{set}"))"#);
        let game = |set: &str| Fake {
            lang: Ok(one_pack("ja", &info(set), "直简")),
            font_atlas: Some(Ok(atlas.clone())),
            ..Fake::default()
        };
        let answer = check(&game("ja"), &CheckInput::default());
        assert!(!answer.ok);
        let errors = &answer.result.as_ref().unwrap().errors;
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].file, "ja/ui.ron");
        assert!(
            errors[0]
                .message
                .starts_with("'简' (U+7B80) isn't in the font atlas's wide set \"ja\""),
            "{errors:?}"
        );
        // Drawn from the set that has both, it passes.
        let answer = check(&game("zh-Hans"), &CheckInput::default());
        assert!(answer.ok, "{:?}", answer.result);
    }

    /// The errors of a game's tips, `title` the one tip's title, whose
    /// loader measures them with `atlas`'s widths, as a game's does when
    /// its atlas loads before its text.
    fn tip_errors(title: &str, atlas: &FontAtlasDef) -> Vec<ContentError> {
        use shuyi_assets::font::CellWidths;
        use shuyi_assets::tip::{TipLimits, TipRules, from_source};
        let source =
            format!(r#"(tips: [(id: "a", trigger: "jumped", title: {title:?}, text: "x")])"#);
        let widths = CellWidths::of(atlas);
        let rules = TipRules {
            limits: TipLimits {
                text_lines: 1,
                line_chars: 10,
                title_chars: 6,
            },
            widths: &widths,
            placeholder: &|_| true,
            face_problem: &|_| None,
            default_face: "calm",
        };
        from_source::<String>("tips.ron", &source, &rules)
            .err()
            .unwrap_or_default()
    }

    #[test]
    fn check_measures_the_text_with_the_font_atlass_wide_glyphs() {
        // Four characters, under the title's 6; but 8 cells drawn wide.
        let title = "日本語だ";
        let narrow = ascii_and(title);
        let mut wide = ascii_and("");
        wide.wide = title.chars().zip(96..).collect();
        for (atlas, ok) in [(narrow, true), (wide, false)] {
            let game = Fake {
                check: Ok(tip_errors(title, &atlas)),
                lang: Ok(russian(title)),
                font_atlas: Some(Ok(atlas)),
                ..Fake::default()
            };
            let answer = check(&game, &CheckInput::default());
            assert_eq!(answer.ok, ok, "{:?}", answer.result);
            let errors = &answer.result.as_ref().unwrap().errors;
            let messages: Vec<&str> = errors.iter().map(|e| e.message.as_str()).collect();
            let over = ["tip \"a\": the title must be 1 to 6 characters"];
            assert_eq!(messages, if ok { &[][..] } else { &over[..] });
        }
    }

    #[test]
    fn check_fails_on_a_character_the_font_atlas_cant_draw() {
        let game = Fake {
            lang: Ok(russian("Жa")),
            font_atlas: Some(Ok(ascii_and(""))),
            ..Fake::default()
        };
        let answer = check(&game, &CheckInput::default());
        assert!(!answer.ok);
        let errors = &answer.result.as_ref().unwrap().errors;
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].file, "ru/ui.ron");
        assert_eq!(errors[0].line, None);
        assert!(
            errors[0]
                .message
                .starts_with("'Ж' (U+0416) isn't in the font atlas (menu.title)"),
            "{}",
            errors[0].message
        );
        assert_eq!(
            answer.limits,
            ["some errors are about a whole file: their validator gives no line"]
        );
        // With the glyph, it passes.
        let drawn = Fake {
            font_atlas: Some(Ok(ascii_and("Ж"))),
            ..game
        };
        let answer = check(&drawn, &CheckInput::default());
        assert!(answer.ok, "{:?}", answer.result);
        assert!(answer.limits.is_empty(), "{:?}", answer.limits);
    }

    #[test]
    fn a_game_that_doesnt_offer_its_atlas_passes_and_says_so() {
        let game = Fake {
            lang: Ok(russian("Жa")),
            font_atlas: None,
            ..Fake::default()
        };
        let answer = check(&game, &CheckInput::default());
        assert!(answer.ok);
        assert_eq!(
            answer.limits,
            [
                "the text isn't checked against the font atlas: the game's tools don't offer it \
                 (GameTools::font_atlas)"
            ]
        );
    }

    #[test]
    fn an_atlas_or_a_text_that_doesnt_load_fails_the_check() {
        let atlas = Fake {
            font_atlas: Some(Err("font/atlas.ron: bad".to_owned())),
            ..Fake::default()
        };
        let answer = check(&atlas, &CheckInput::default());
        assert_eq!(
            answer.error.as_deref(),
            Some("the game's font atlas doesn't load: font/atlas.ron: bad")
        );
        let text = Fake {
            lang: Err("lang/en/ui.ron: bad".to_owned()),
            ..Fake::default()
        };
        let answer = check(&text, &CheckInput::default());
        assert_eq!(
            answer.error.as_deref(),
            Some("the game's text doesn't load: lang/en/ui.ron: bad")
        );
        // With errors in the content, neither is asked for.
        let both = Fake {
            check: Ok(vec![ContentError::new("a.ron", "bad").at(1, None)]),
            ..atlas
        };
        assert_eq!(check(&both, &CheckInput::default()).error, None);
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
                "the identity isn't checked: shuyi.ron has no `app`",
                "only the frame's fit, the text's size, the controls, one-bit patterns and the \
                 identity are checked yet"
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
        assert_eq!(all.limits.len(), 5, "desktop takes any size");
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
    fn the_app_part_is_checked_on_every_target() {
        let with = |app: &str| {
            crate::project::Config::parse(&format!(
                r#"(app: {app}, targets: {{
    "desktop": (frame: (400, 240), scale: integer, text_px: 16),
    "web": (frame: (400, 240), scale: integer, text_px: 16),
}})"#
            ))
            .unwrap()
        };
        let root = Path::new(".");
        let bad = with(r#"(name: "Moon Garden", id: "moon")"#);
        let answer = targets(&bad, root, &on("all"), None);
        assert!(!answer.ok, "a bad id fails");
        for report in &answer.result.as_ref().unwrap().targets {
            let found: Vec<(Level, TargetCheck, &str)> = report
                .problems
                .iter()
                .map(|p| (p.level, p.check, p.setting.as_str()))
                .collect();
            assert_eq!(found, [(Level::Error, TargetCheck::App, "app.id")]);
        }
        assert!(!answer.limits.iter().any(|l| l.contains("identity isn't")));
        let placeholder = with(r#"(name: "Moon Garden", id: "com.example.moon-garden")"#);
        let answer = targets(&placeholder, root, &on("web"), None);
        assert!(answer.ok, "the placeholder is a warning");
        let out = answer.result.as_ref().unwrap();
        assert_eq!(out.targets[0].problems[0].level, Level::Warning);
        assert!(
            text(out)
                .contains("  warning: web: app.id: \"com.example.moon-garden\" is a placeholder"),
            "{}",
            text(out)
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
