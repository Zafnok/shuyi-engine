//! What the commands that run the game need from it: [`GameTools`].
//!
//! The `shuyi` binary knows no game. A game gives its tools in a program
//! of its own, which runs [`crate::main_with`] with them; `shuyi.ron`
//! names that program ([`crate::project::GameProgram`]) and `shuyi` runs
//! it with `cargo run` for the commands that run the game: `frame-png`,
//! `lang-status`, and the contract's `check`, `run`, `scene`, `replay`,
//! `bots`, `playtest`, `autobalance`, `import` and `bake`.

use std::collections::BTreeMap;
use std::process::Command;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_assets::ContentError;
use shuyi_assets::bundle::Bundle;
use shuyi_assets::format::Formats;
use shuyi_assets::lang::Lang;
use shuyi_assets::onebit::PatternMapDef;
use shuyi_assets::pack::Bake;
use shuyi_assets::palette::PaletteDef;
use shuyi_ui::input::fit::Controls;

use shuyi_core::bots::playtest::{Check, PlaytestReport, Tried, Tries};
use shuyi_core::{BalanceSettings, Balanced, Knob, autobalance_by};

use crate::contract::autobalance::AutobalanceInput;
use crate::contract::bots::{BotsInput, BotsOutput};
use crate::contract::codes::{self, Code};
use crate::contract::import::Importers;
use crate::contract::playtest::{DEFAULT_TRIES, PlaytestInput, Summary};
use crate::contract::replay::{Record, ReplayRun};
use crate::contract::session::Session;
use crate::frame_png::{Script, Shot};
use crate::project::Project;

/// An option a game adds to `frame-png`. Build it with
/// [`GameOption::new`] (ADR-0039).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct GameOption {
    /// The flag (`--light`).
    pub flag: &'static str,
    /// What its value is called in the usage text (`NAME`), if it takes
    /// one.
    pub value: Option<&'static str>,
    /// What it does, for the usage text (lines after the first are
    /// indented under it).
    pub help: &'static str,
}

impl GameOption {
    /// The flag `flag`, explained by `help`, taking no value
    /// ([`with_value`](Self::with_value) names one).
    #[must_use]
    pub const fn new(flag: &'static str, help: &'static str) -> Self {
        Self {
            flag,
            value: None,
            help,
        }
    }

    /// The option with its value called `value` in the usage text.
    #[must_use]
    pub const fn with_value(self, value: Option<&'static str>) -> Self {
        Self { value, ..self }
    }
}

/// A game option as given.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GameArg {
    /// Its flag.
    pub flag: String,
    /// Its value, if it takes one.
    #[serde(default)]
    pub value: Option<String>,
}

/// A game's languages, for `lang-status`. Build it with
/// [`GameLang::new`] (ADR-0039).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GameLang {
    /// The game's English and its packs, with the data's and dialogue's
    /// English.
    pub lang: Lang,
    /// The variants of a pack's line, in the pack files' order (`text_m`,
    /// `text_f`: `["m", "f"]`).
    pub variants: Vec<String>,
}

impl GameLang {
    /// The game's languages `lang`, a pack's line in the variants
    /// `variants`.
    pub fn new(lang: Lang, variants: Vec<String>) -> Self {
        Self { lang, variants }
    }
}

/// What a game bakes into its pack ([`GameTools::bake`]). Build it with
/// [`GameBake::new`] (ADR-0039).
#[derive(Debug)]
#[non_exhaustive]
pub struct GameBake {
    /// Which assets of which kinds, and the game's stamp.
    pub bake: Bake,
    /// The bundle of its sources: the layers it ships for the target.
    pub bundle: Bundle,
    /// The formats its loaders read the sources with (the pack's own are
    /// added to them).
    pub formats: Formats,
}

impl GameBake {
    /// The bake `bake` of the sources `bundle`, read with `formats`.
    pub fn new(bake: Bake, bundle: Bundle, formats: Formats) -> Self {
        Self {
            bake,
            bundle,
            formats,
        }
    }
}

/// A skin's one-bit look ([`GameTools::one_bit`]): its palette of roles
/// and the pattern map beside it (ADR-0008 §4). Build it with
/// [`OneBitSkin::new`] (ADR-0039).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct OneBitSkin {
    /// The palette's file, as messages show it.
    pub palette_file: String,
    /// The palette: each role's colour.
    pub palette: PaletteDef,
    /// The pattern map's file, as messages show it
    /// (`files/palette@onebit.ron`).
    pub map_file: String,
    /// The pattern map.
    pub map: PatternMapDef,
}

impl OneBitSkin {
    /// The palette `palette` from the file `palette_file`, and the pattern
    /// map `map` from `map_file`.
    pub fn new(
        palette_file: impl Into<String>,
        palette: PaletteDef,
        map_file: impl Into<String>,
        map: PatternMapDef,
    ) -> Self {
        Self {
            palette_file: palette_file.into(),
            palette,
            map_file: map_file.into(),
            map,
        }
    }
}

/// What `playtest` plays on one of the game's stages
/// ([`GameTools::playtest_plan`]): its entrants, its checks, and how one
/// try is played. The command picks the entrants asked for, plays their
/// tries on every core, times them and judges them once. Build it with
/// [`PlaytestPlan::new`] and assign the rest (ADR-0039).
#[non_exhaustive]
pub struct PlaytestPlan {
    /// Shown at the top of the report: the stage and its tier, say.
    pub title: String,
    /// Every entrant, in report order: its name and a label (a mode), if
    /// any.
    pub entrants: Vec<(String, Option<String>)>,
    /// Named groups of entrants (`"all"`), which `--entrant` takes too.
    pub groups: BTreeMap<String, Vec<String>>,
    /// The entrants that play when `--entrant` isn't given (all of them
    /// if empty).
    pub default_entrants: Vec<String>,
    /// The checks over the entrants that play, given their names: a
    /// check naming an entrant who doesn't play is left out.
    pub checks: PlaytestChecks,
    /// One try: of the entrant at that index of
    /// [`entrants`](Self::entrants), from that seed, stopped at that turn
    /// if it hasn't ended. Tries run on several threads at once, in any
    /// order; most simply [`play_tries`](shuyi_core::bots::playtest::play_tries)
    /// over the one seed.
    pub play: PlayTry,
    /// Each entrant's summary items, in [`entrants`](Self::entrants)
    /// order, printed on its line: `won 83/100 · turns median 12`. An
    /// entrant past the end, or with none, has no summary.
    pub summary: Vec<Vec<Summary>>,
}

impl PlaytestPlan {
    /// The plan titled `title`: `entrants` judged by `checks`, a try played
    /// by `play`. It has no [`groups`](Self::groups) and no
    /// [`summary`](Self::summary), and every entrant plays unless
    /// `--entrant` says ([`default_entrants`](Self::default_entrants)
    /// empty).
    pub fn new(
        title: impl Into<String>,
        entrants: Vec<(String, Option<String>)>,
        checks: PlaytestChecks,
        play: PlayTry,
    ) -> Self {
        Self {
            title: title.into(),
            entrants,
            groups: BTreeMap::new(),
            default_entrants: Vec::new(),
            checks,
            play,
            summary: Vec::new(),
        }
    }
}

/// A plan's checks ([`PlaytestPlan::checks`]): given the names of the
/// entrants that play, the checks over them.
pub type PlaytestChecks = Box<dyn Fn(&[&str]) -> Vec<Check> + Send + Sync>;

/// A plan's one try ([`PlaytestPlan::play`]): entrant index, seed, the
/// turn it stops at.
pub type PlayTry = Box<dyn Fn(usize, u64, u32) -> Result<Tried, String> + Send + Sync>;

/// What `autobalance` searches on one of the game's stages
/// ([`GameTools::autobalance_plan`]): who plays it and how they are judged,
/// as a [`PlaytestPlan`] says, and the stage it may change, its type the
/// game's ([`Tunable`]). The command picks the entrants asked for and runs
/// the search (`shuyi_core::autobalance_by`), each setting's tries played
/// on every core and judged once. Build it with [`AutobalancePlan::new`]
/// and assign the rest (ADR-0039).
#[non_exhaustive]
pub struct AutobalancePlan {
    /// Shown at the top of the answer: the stage and its tier, say.
    pub title: String,
    /// Every entrant, in report order: its name and a label (a mode), if
    /// any.
    pub entrants: Vec<(String, Option<String>)>,
    /// Named groups of entrants (`"all"`), which `--entrant` takes too.
    pub groups: BTreeMap<String, Vec<String>>,
    /// The entrants that play when `--entrant` isn't given (all of them
    /// if empty).
    pub default_entrants: Vec<String>,
    /// The checks over the entrants that play, given their names: the
    /// bands the search lands them in.
    pub checks: PlaytestChecks,
    /// The stage as authored, its knobs, how a try is played on a changed
    /// stage and how one is written: most simply a boxed [`Tunable`].
    pub stage: Box<dyn TunableStage>,
}

impl AutobalancePlan {
    /// The plan titled `title`: `entrants` judged by `checks` on `stage`.
    /// It has no [`groups`](Self::groups), and every entrant plays unless
    /// `--entrant` says ([`default_entrants`](Self::default_entrants)
    /// empty).
    pub fn new(
        title: impl Into<String>,
        entrants: Vec<(String, Option<String>)>,
        checks: PlaytestChecks,
        stage: Box<dyn TunableStage>,
    ) -> Self {
        Self {
            title: title.into(),
            entrants,
            groups: BTreeMap::new(),
            default_entrants: Vec::new(),
            checks,
            stage,
        }
    }
}

/// The stage an [`AutobalancePlan`] may change, of the game's stage
/// definition type `S`: the command never sees `S`, only what the search
/// found ([`TunableStage`]). Build it with [`Tunable::new`] (ADR-0039).
#[non_exhaustive]
pub struct Tunable<S> {
    /// The stage as authored.
    pub stage: S,
    /// What the search may change, in the order it tries them.
    pub knobs: Vec<Box<dyn Knob<S>>>,
    /// One try on a stage: the stage (as authored, or changed by the
    /// knobs), the entrant at that index of the plan's entrants, the seed,
    /// the turn it stops at. Tries run on several threads at once, in any
    /// order; most simply [`play_tries`](shuyi_core::bots::playtest::play_tries)
    /// over the one seed.
    pub play: StagePlay<S>,
    /// Writes a changed stage back to the game's files (`--apply`): the
    /// files written, as the game shows them.
    pub write: StageWrite<S>,
}

impl<S> Tunable<S> {
    /// The stage `stage`, changed by `knobs`, a try on it played by `play`
    /// and a changed one written by `write`.
    pub fn new(
        stage: S,
        knobs: Vec<Box<dyn Knob<S>>>,
        play: StagePlay<S>,
        write: StageWrite<S>,
    ) -> Self {
        Self {
            stage,
            knobs,
            play,
            write,
        }
    }
}

/// A [`Tunable`]'s one try: the stage, entrant index, seed, the turn it
/// stops at.
pub type StagePlay<S> = Box<dyn Fn(&S, usize, u64, u32) -> Result<Tried, String> + Send + Sync>;

/// A [`Tunable`]'s writer: the files it wrote of a changed stage.
pub type StageWrite<S> = Box<dyn Fn(&S) -> Result<Vec<String>, String>>;

/// How the search has a setting played: given one try on the stage of
/// that setting (entrant index, seed, the turn it stops at) and the tries
/// to play, their report, judged. `autobalance` plays them on its threads.
pub type Evaluate<'a> = dyn FnMut(
        &(dyn Fn(usize, u64, u32) -> Result<Tried, String> + Sync),
        &Tries,
    ) -> Result<PlaytestReport, String>
    + 'a;

/// A [`Tunable`] of any stage type, as `autobalance` runs it.
pub trait TunableStage {
    /// The search over the stage (`shuyi_core::autobalance_by`) as
    /// `settings` say, each setting played by `evaluate`, asking
    /// `should_stop` before each run; with `apply`, the stage found is
    /// written if it changed.
    fn search(
        &self,
        settings: &BalanceSettings,
        evaluate: &mut Evaluate<'_>,
        should_stop: &mut dyn FnMut() -> bool,
        apply: bool,
    ) -> Result<Searched, String>;
}

/// What [`TunableStage::search`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Searched {
    /// The changes, the reports before and after and why it stopped; the
    /// stage itself stays the game's.
    pub balanced: Balanced<()>,
    /// With `apply`: the files the game wrote (none if nothing changed),
    /// or why it couldn't.
    pub written: Option<Result<Vec<String>, String>>,
}

impl<S: Clone + Sync> TunableStage for Tunable<S> {
    fn search(
        &self,
        settings: &BalanceSettings,
        evaluate: &mut Evaluate<'_>,
        should_stop: &mut dyn FnMut() -> bool,
        apply: bool,
    ) -> Result<Searched, String> {
        let knobs: Vec<&dyn Knob<S>> = self.knobs.iter().map(AsRef::as_ref).collect();
        let play = &self.play;
        let found = autobalance_by(
            &self.stage,
            &knobs,
            settings,
            |stage, tries| {
                let one = |e: usize, seed: u64, turns: u32| play(stage, e, seed, turns);
                evaluate(&one, tries)
            },
            should_stop,
        )?;
        let written = apply.then(|| {
            if found.changes.is_empty() {
                Ok(Vec::new())
            } else {
                (self.write)(&found.stage)
            }
        });
        Ok(Searched {
            balanced: Balanced {
                stage: (),
                changes: found.changes,
                before: found.before,
                after: found.after,
                stop: found.stop,
                evaluations: found.evaluations,
            },
            written,
        })
    }
}

/// What a game gives the commands that run it. Only `frame-png`'s and
/// `lang-status`'s parts are required; a contract operation whose part a
/// game leaves out answers that the game's tools don't offer it.
pub trait GameTools {
    /// The options the game adds to `frame-png`, which also start its
    /// game for `run`, `scene` and `bots` (none: `&[]`).
    fn frame_options(&self) -> &[GameOption];

    /// What else `frame-png --help` says about the game's pictures.
    fn frame_notes(&self) -> &'static str {
        ""
    }

    /// Starts the game in its Harness as `options` say, plays `script`
    /// and paints the frame, each console pixel `scale` × `scale` (the
    /// game's choice if `None`).
    fn frame(
        &self,
        options: &[GameArg],
        script: &Script,
        scale: Option<u32>,
    ) -> Result<Shot, String>;

    /// `frame-png --target <id>` on a one-bit target: as
    /// [`frame`](Self::frame), but painted as the target's display shows
    /// it, most simply with [`to_one_bit`](shuyi_ui::frame::onebit::to_one_bit)
    /// and [`one_bit_picture`](crate::frame_png::one_bit_picture), each of
    /// its pixels `scale` × `scale` (1 if `None`).
    fn frame_one_bit(
        &self,
        target: &str,
        options: &[GameArg],
        script: &Script,
        scale: Option<u32>,
    ) -> Result<Shot, String> {
        let _ = (target, options, script, scale);
        Err(unsupported(
            "frame-png --target on a one-bit target",
            "frame_one_bit",
        ))
    }

    /// The game's languages.
    fn lang(&self) -> Result<GameLang, String>;

    /// `check`: every error of every validator of the game's content,
    /// each with its file and, when known, its line.
    fn check(&self) -> Result<Vec<ContentError>, String> {
        Err(unsupported("check", "check"))
    }

    /// `run` and `scene`: the game started headless as `options` say
    /// (its [`frame_options`](Self::frame_options)), most simply a
    /// [`Headless`](crate::contract::session::Headless) over its Harness.
    fn session(&self, options: &[GameArg]) -> Result<Box<dyn Session>, String> {
        let _ = options;
        Err(unsupported("run and scene", "session"))
    }

    /// `replay`: `record`, in the game's record format, re-run by its
    /// rules (for a turn game, [`turns`](crate::contract::replay::turns)).
    fn replay(&self, record: &Record) -> Result<ReplayRun, String> {
        let _ = record;
        Err(unsupported("replay", "replay"))
    }

    /// `bots`: the games `input` asks for, played by the game's bots and
    /// summed up ([`tally`](crate::contract::bots::tally)).
    fn bots(&self, input: &BotsInput) -> Result<BotsOutput, String> {
        let _ = input;
        Err(unsupported("bots", "bots"))
    }

    /// `playtest --stage`: the ids of the game's playtest stages, in the
    /// order `--every-stage` plays them. Empty, the default: one unnamed
    /// stage.
    fn playtest_stages(&self) -> Vec<String> {
        Vec::new()
    }

    /// `playtest`: the plan of `stage` (one of
    /// [`playtest_stages`](Self::playtest_stages), `None` if there are
    /// none), started as `input`'s game options say. Its numbers are those
    /// the tries are played with, the game's defaults filled in.
    fn playtest_plan(
        &self,
        stage: Option<&str>,
        input: &PlaytestInput,
    ) -> Result<PlaytestPlan, String> {
        let _ = (stage, input);
        Err(unsupported("playtest", "playtest_plan"))
    }

    /// `playtest`: how many tries, from which seed, stopped at which turn,
    /// when the input doesn't say. The engine's defaults
    /// ([`DEFAULT_TRIES`]) unless the game has its own.
    fn playtest_defaults(&self) -> Tries {
        DEFAULT_TRIES
    }

    /// `autobalance`: the plan of `stage` (one of
    /// [`playtest_stages`](Self::playtest_stages), `None` if there are
    /// none), started as `input`'s game options say. Its numbers are those
    /// the search runs with, the defaults filled in.
    fn autobalance_plan(
        &self,
        stage: Option<&str>,
        input: &AutobalanceInput,
    ) -> Result<AutobalancePlan, String> {
        let _ = (stage, input);
        Err(unsupported("autobalance", "autobalance_plan"))
    }

    /// `import`: the kinds and formats to read a file with. The engine's
    /// standard ones unless the game adds its own.
    fn importers(&self) -> Importers {
        Importers::standard()
    }

    /// `check --target`: the game's keymap and every screen it can show,
    /// for the check that its controls fit each target (ADR-0008 §3), most
    /// simply a [`GameControls`](shuyi_ui::input::fit::GameControls).
    /// `None`, the default, doesn't offer them: `check --target` then
    /// checks the display alone and says so. An error (the game's files
    /// don't load) fails the check.
    fn controls(&self) -> Option<Result<Box<dyn Controls>, String>> {
        None
    }

    /// `check --target` on a one-bit target: the skin's palette and its
    /// pattern map for `target`, for the check that every role has
    /// patterns and the roles that tell things apart stay distinct
    /// (ADR-0008 §4; `shuyi_assets::onebit::check`). `None`, the default,
    /// doesn't offer them, and `check --target` says they weren't checked.
    /// An error (the files don't load) fails the check.
    fn one_bit(&self, _target: &str) -> Option<Result<OneBitSkin, String>> {
        None
    }

    /// `bake` and `check --baked`: what the game bakes for `target`
    /// (`None` for its default bundle), from which bundle, read with which
    /// formats. The target is the game's to pick layers and kinds by.
    fn bake(&self, target: Option<&str>) -> Result<GameBake, String> {
        let _ = target;
        Err(unsupported("bake and check --baked", "bake"))
    }
}

/// What [`unsupported`]'s errors start with, which
/// [`is_unsupported`] tells them by.
const UNSUPPORTED: &str = "the game's tools don't offer ";

/// What a game whose tools don't offer `op` answers: they lack the
/// method `method`.
pub fn unsupported(op: &str, method: &str) -> String {
    format!("{UNSUPPORTED}{op} (GameTools::{method})")
}

/// Whether `error`, from a [`GameTools`] method, is [`unsupported`]'s:
/// the game left the method at its default.
pub fn is_unsupported(error: &str) -> bool {
    error.starts_with(UNSUPPORTED)
}

/// The problem code of `error`, from a [`GameTools`] method:
/// [`codes::NOT_OFFERED`] if [`is_unsupported`], else
/// [`codes::GAME_ERROR`].
pub fn code_of(error: &str) -> Code {
    if is_unsupported(error) {
        codes::NOT_OFFERED
    } else {
        codes::GAME_ERROR
    }
}

/// Runs `command` with `args` in the game's tool program of `project`
/// and returns its exit code; 3 if there is none, or cargo doesn't start
/// ([`codes::PROJECT_FILE`], [`codes::TOOLCHAIN`]).
pub fn run_in_game_program(project: &Project, command: &str, args: &[String]) -> u8 {
    let Some(program) = &project.config.game else {
        return crate::contract::cli::no_game(command, args);
    };
    let cargo = crate::cargo();
    match Command::new(cargo)
        .args(program.cargo_args(command, args))
        .current_dir(&project.root)
        .status()
    {
        Ok(status) => status
            .code()
            .and_then(|code| u8::try_from(code).ok())
            .unwrap_or(1),
        Err(e) => crate::contract::cli::setup_failed(
            command,
            args,
            codes::TOOLCHAIN,
            &format!("cannot run cargo: {e}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shuyi_core::bots::playtest::Band;

    /// A game that adds nothing to `frame-png`.
    struct Plain;

    impl GameTools for Plain {
        fn frame_options(&self) -> &[GameOption] {
            &[]
        }

        fn frame(&self, _: &[GameArg], _: &Script, _: Option<u32>) -> Result<Shot, String> {
            Err("no frame".to_owned())
        }

        fn lang(&self) -> Result<GameLang, String> {
            Ok(GameLang {
                lang: Lang::default(),
                variants: Vec::new(),
            })
        }
    }

    #[test]
    fn a_game_adds_no_options_or_notes_unless_it_says_so() {
        assert_eq!(Plain.frame_options(), &[]);
        assert_eq!(Plain.frame_notes(), "");
        let got = Plain.lang().unwrap().variants;
        assert!(got.is_empty(), "{got:?}");
        assert!(Plain.frame(&[], &Script::default(), None).is_err());
    }

    #[test]
    fn a_game_offers_no_contract_operation_unless_it_says_so() {
        assert_eq!(
            Plain.check().unwrap_err(),
            "the game's tools don't offer check (GameTools::check)"
        );
        let session = Plain.session(&[]).err().unwrap();
        assert_eq!(
            session,
            "the game's tools don't offer run and scene (GameTools::session)"
        );
        let record = Record {
            file: "a.ron".to_owned(),
            text: String::new(),
        };
        assert!(
            Plain
                .replay(&record)
                .unwrap_err()
                .contains("GameTools::replay")
        );
        let bots = Plain.bots(&BotsInput::default()).unwrap_err();
        assert!(bots.contains("GameTools::bots"));
        let playtest = Plain
            .playtest_plan(None, &PlaytestInput::default())
            .err()
            .unwrap();
        assert!(playtest.contains("GameTools::playtest_plan"));
        let balance = Plain
            .autobalance_plan(None, &AutobalanceInput::default())
            .err()
            .unwrap();
        assert_eq!(
            balance,
            "the game's tools don't offer autobalance (GameTools::autobalance_plan)"
        );
        let got = Plain.playtest_stages();
        assert!(got.is_empty(), "{got:?}");
        assert_eq!(Plain.playtest_defaults(), DEFAULT_TRIES);
        assert_eq!(Plain.importers().kinds(), Importers::standard().kinds());
        assert!(Plain.controls().is_none());
        assert!(Plain.one_bit("playdate").is_none());
        assert_eq!(
            Plain
                .frame_one_bit("playdate", &[], &Script::default(), None)
                .unwrap_err(),
            "the game's tools don't offer frame-png --target on a one-bit target \
             (GameTools::frame_one_bit)"
        );
        assert_eq!(
            Plain.bake(Some("desktop")).unwrap_err(),
            "the game's tools don't offer bake and check --baked (GameTools::bake)"
        );
    }

    #[test]
    fn a_method_left_at_its_default_is_told_from_the_games_own_errors() {
        let error = Plain.bots(&BotsInput::default()).unwrap_err();
        assert!(is_unsupported(&error));
        assert_eq!(code_of(&error), codes::NOT_OFFERED);
        assert!(!is_unsupported("records/a.ron: no turn 4"));
        assert_eq!(code_of("records/a.ron: no turn 4"), codes::GAME_ERROR);
    }

    #[test]
    fn with_no_game_tool_program_a_command_that_runs_the_game_exits_3() {
        let project = Project {
            root: std::path::PathBuf::from("."),
            config: crate::project::Config::default(),
        };
        assert_eq!(run_in_game_program(&project, "frame-png", &[]), 3);
    }

    #[test]
    fn each_new_gives_the_literal_it_replaces() {
        const SEED: GameOption = GameOption::new("--seed", "seed it").with_value(Some("N"));
        let literal = GameOption {
            flag: "--seed",
            value: Some("N"),
            help: "seed it",
        };
        assert_eq!(SEED, literal);
        let plain = GameOption::new("--hard", "harder");
        assert_eq!(
            (plain.flag, plain.value, plain.help),
            ("--hard", None, "harder")
        );
        let lang = GameLang::new(Lang::default(), vec!["m".to_owned()]);
        assert_eq!(lang.variants, ["m"]);
        let skin = OneBitSkin::new(
            "p.ron",
            PaletteDef::default(),
            "m.ron",
            PatternMapDef::default(),
        );
        let literal = OneBitSkin {
            palette_file: "p.ron".to_owned(),
            palette: PaletteDef::default(),
            map_file: "m.ron".to_owned(),
            map: PatternMapDef::default(),
        };
        assert_eq!(skin, literal);
        let made = GameBake::new(Bake::new("v1"), Bundle::new(Vec::new()), Formats::new());
        let literal = GameBake {
            bake: Bake::new("v1"),
            bundle: Bundle::new(Vec::new()),
            formats: Formats::new(),
        };
        assert_eq!(format!("{made:?}"), format!("{literal:?}"));
    }

    #[test]
    fn a_plan_starts_with_no_groups_and_every_entrant_by_default() {
        let entrants = vec![
            ("a".to_owned(), None),
            ("b".to_owned(), Some("hard".to_owned())),
        ];
        let plan = PlaytestPlan::new(
            "stage",
            entrants.clone(),
            Box::new(|names| vec![Check::rate(names[0], Vec::new(), Band { min: 0, max: 100 })]),
            Box::new(|_, _, _| Err("no try".to_owned())),
        );
        assert_eq!(plan.title, "stage");
        assert_eq!(plan.entrants, entrants);
        assert!(plan.groups.is_empty() && plan.default_entrants.is_empty());
        assert_eq!((plan.checks)(&["a"]).len(), 1);
        assert_eq!((plan.play)(0, 0, 1).unwrap_err(), "no try");
        let tunable = Tunable::new(
            7_u32,
            Vec::new(),
            Box::new(|stage: &u32, _, _, _| Err(format!("stage {stage}"))),
            Box::new(|_: &u32| Ok(vec!["s.ron".to_owned()])),
        );
        assert_eq!(tunable.stage, 7);
        assert!(tunable.knobs.is_empty());
        assert_eq!((tunable.play)(&3, 0, 0, 1).unwrap_err(), "stage 3");
        assert_eq!((tunable.write)(&3).unwrap(), ["s.ron"]);
        let plan = AutobalancePlan::new(
            "tuned",
            entrants.clone(),
            Box::new(|_| Vec::new()),
            Box::new(tunable),
        );
        assert_eq!(plan.title, "tuned");
        assert_eq!(plan.entrants, entrants);
        assert!(plan.groups.is_empty() && plan.default_entrants.is_empty());
        let got = (plan.checks)(&["a"]);
        assert!(got.is_empty(), "{got:?}");
    }
}
