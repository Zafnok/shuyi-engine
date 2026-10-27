//! A game and a session made up for the contract's tests: each answer is
//! what the test sets.

use std::ops::RangeInclusive;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use shuyi_assets::ContentError;
use shuyi_assets::bundle::{Bundle, Layer};
use shuyi_assets::format::Formats;
use shuyi_assets::lang::Lang;
use shuyi_assets::names::Names;
use shuyi_assets::pack::Bake;
use shuyi_assets::palette::PaletteDef;
use shuyi_ui::input::KeymapDef;
use shuyi_ui::input::fit::{Controls, GameControls, ScreenActions};
use shuyi_ui::toy::Action;

use shuyi_core::bots::playtest::{Band, Check, Condition, Tried, Tries, Want};
use shuyi_core::{ContentHash, Knob, Made};

use super::autobalance::AutobalanceInput;
use super::bots::{BotsInput, BotsOutput, tally};
use super::perf::PerfRead;
use super::playtest::{DEFAULT_TRIES, PlaytestInput, Summary};
use super::replay::{Record, ReplayRun, ReplayStep};
use super::session::{Event, LogEntry, ModuleEntry, Scene, Session, Sound};
use crate::frame_png::{Image, Script, Shot, Step};
use crate::game::{
    AutobalancePlan, GameArg, GameBake, GameLang, GameOption, GameTools, OneBitSkin, PlaytestPlan,
    Tunable,
};
use shuyi_ui::perf::{FrameRecord, HeapSnapshot, PerfRing};

/// A session that counts frames as the Harness does (two a key, sixty a
/// second of waiting) and shows what it was given.
#[derive(Debug, Clone, PartialEq)]
pub struct FakeSession {
    /// Frames run.
    pub frames: u32,
    /// The open screens.
    pub screens: Vec<String>,
    /// The quit's frame.
    pub quit: Option<u32>,
    /// The scene, or why there is none.
    pub scene: Result<Scene, String>,
    /// The log's lines.
    pub log: Vec<LogEntry>,
    /// How many lines the log dropped.
    pub log_dropped: u32,
    /// The modules installed.
    pub modules: Vec<ModuleEntry>,
    /// The frames' records it measured, oldest first, numbered from 0.
    pub perf: Vec<FrameRecord>,
    /// Whether [`Session::set_perf`] turned perf on.
    pub perf_on: bool,
    /// The heap snapshots it takes, in turn: each
    /// [`Session::heap_snapshot`] takes the first left.
    pub snapshots: Vec<HeapSnapshot>,
}

impl FakeSession {
    /// A session on screen `top`, with a scene of `text`.
    pub fn on(top: &str, text: &str) -> Self {
        Self {
            frames: 0,
            screens: vec![top.to_owned()],
            quit: None,
            scene: Ok(Scene {
                screen: top.to_owned(),
                kind: "grid map".to_owned(),
                text: text.to_owned(),
                data: serde_json::json!({ "turn": 1 }),
            }),
            log: Vec::new(),
            log_dropped: 0,
            modules: Vec::new(),
            perf: Vec::new(),
            perf_on: false,
            snapshots: Vec::new(),
        }
    }
}

impl Session for FakeSession {
    fn play(&mut self, script: &Script) {
        for step in &script.steps {
            self.frames += match step {
                Step::Keys(keys) | Step::Pad(keys) => {
                    2 * u32::try_from(keys.split_whitespace().count()).unwrap_or(0)
                }
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                Step::Wait(seconds) => (seconds * 60.0).ceil() as u32,
            };
        }
    }

    fn idle(&mut self, frames: u32) {
        self.frames += frames;
    }

    fn frame_count(&self) -> u32 {
        self.frames
    }

    fn screens(&self) -> Vec<String> {
        self.screens.clone()
    }

    fn quit_frame(&self) -> Option<u32> {
        self.quit
    }

    fn snapshot(&self) -> String {
        "AB\n--- colours ---\nab\n".to_owned()
    }

    fn events(&self) -> Vec<Event> {
        vec![Event::Opened {
            frame: 1,
            screen: self.screens.last().cloned().unwrap_or_default(),
        }]
    }

    fn sounds(&self) -> Vec<Sound> {
        vec![Sound {
            frame: 1,
            cue: "click".to_owned(),
        }]
    }

    fn scene(&self) -> Result<Scene, String> {
        self.scene.clone()
    }

    fn log(&self) -> Vec<LogEntry> {
        self.log.clone()
    }

    fn log_dropped(&self) -> u32 {
        self.log_dropped
    }

    fn modules(&self) -> Vec<ModuleEntry> {
        self.modules.clone()
    }

    fn set_perf(&mut self, on: bool) {
        self.perf_on = on;
    }

    /// Its records, read from a ring that keeps them all.
    fn perf(&self, since: u64, max: u32) -> PerfRead {
        if self.perf.is_empty() {
            return PerfRead::none(since);
        }
        let mut ring = PerfRing::new(self.perf.len());
        for record in &self.perf {
            ring.push(record.clone());
        }
        PerfRead::of(&ring, since, max)
    }

    fn heap_snapshot(&mut self) -> Option<HeapSnapshot> {
        (!self.snapshots.is_empty()).then(|| self.snapshots.remove(0))
    }

    /// A picture one pixel high, a pixel wide per frame run, each
    /// `scale` × `scale`.
    fn picture(&self, scale: Option<u32>) -> Result<Image, String> {
        let scale = scale.unwrap_or(1);
        let (width, height) = (self.frames.max(1) * scale, scale);
        Ok(Image {
            width,
            height,
            rgba: vec![255; width as usize * height as usize * 4],
        })
    }
}

/// The stamp of engine `engine`, game `game` and content `content`.
pub fn stamp(engine: &str, game: &str, content: ContentHash) -> Made {
    let mut made = Made::new(game, content);
    made.engine = engine.to_owned();
    made
}

/// A game whose tools answer what the test sets.
pub struct Fake {
    /// `check`'s answer.
    pub check: Result<Vec<ContentError>, String>,
    /// `session`'s session, or why there is none.
    pub session: Result<FakeSession, String>,
    /// `replay`: a run of the record's text, a hash a line.
    pub replay: Result<(), String>,
    /// This build's stamp, or none.
    pub made: Option<Made>,
    /// `bots`'s plays, or why there are none.
    pub bots: Result<BotsOutput, String>,
    /// Its playtest, or why its plan doesn't load.
    pub playtest: Result<FakePlaytest, String>,
    /// The inputs its playtest plans were asked for with, in turn.
    pub planned: Mutex<Vec<PlaytestInput>>,
    /// Its autobalance, or why its plan doesn't load.
    pub balance: Result<FakeBalance, String>,
    /// The inputs its autobalance plans were asked for with, in turn.
    pub balanced: Mutex<Vec<AutobalanceInput>>,
    /// The stages its autobalance wrote, in turn.
    pub wrote: Arc<Mutex<Vec<u32>>>,
    /// The files `bake` bakes (its names table and palette are baked), or
    /// none if it doesn't offer `bake`.
    pub sources: Option<&'static [(&'static str, &'static [u8])]>,
    /// Whether it offers its controls (a keymap with no table of a
    /// target's, and a title screen that declares nothing), or why they
    /// don't load; `None`: it doesn't offer them.
    pub controls: Option<Result<(), String>>,
    /// Its one-bit skin, or why it doesn't load; `None`: it doesn't offer
    /// one.
    pub one_bit: Option<Result<OneBitSkin, String>>,
}

/// A game's sources: a names table, a palette and a picture.
pub static SOURCES: &[(&str, &[u8])] = &[
    ("data/names.ron", br#"{"hero": "Ana"}"#),
    ("data/palette.ron", br##"{"black": "#000000"}"##),
    ("images/a.png", b"not really a picture"),
];

impl Default for Fake {
    fn default() -> Self {
        Self {
            check: Ok(Vec::new()),
            session: Ok(FakeSession::on("stage", "map")),
            replay: Ok(()),
            made: None,
            bots: Ok(tally(Vec::new())),
            playtest: Ok(FakePlaytest::default()),
            planned: Mutex::new(Vec::new()),
            balance: Ok(FakeBalance::default()),
            balanced: Mutex::new(Vec::new()),
            wrote: Arc::new(Mutex::new(Vec::new())),
            sources: Some(SOURCES),
            controls: None,
            one_bit: None,
        }
    }
}

/// The fake's playtest. Its entrants are `walk` and `run` (labelled
/// `fast`), both in the group `all`. Its checks are a rate each, that
/// half its tries or more be won (floored there too), and that `run`'s
/// turns be lower than `walk`'s. Each try ends `outcome` (unfinished if
/// `None`), `walk`'s at the turn limit and `run`'s a turn sooner, won
/// unless its stage's name starts with `hard`; the try of seed `fail`
/// fails, and each takes `sleep`. `walk`'s summary is the sum of `won`
/// and the median of `turns`; `run` has none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakePlaytest {
    /// Its stages.
    pub stages: Vec<String>,
    /// Its default tries.
    pub defaults: Tries,
    /// The entrants that play when none is asked for.
    pub default_entrants: Vec<String>,
    /// How a try ends.
    pub outcome: Option<&'static str>,
    /// The seed whose try fails.
    pub fail: Option<u64>,
    /// How long each try takes.
    pub sleep: Duration,
}

impl Default for FakePlaytest {
    fn default() -> Self {
        Self {
            stages: Vec::new(),
            defaults: DEFAULT_TRIES,
            default_entrants: Vec::new(),
            outcome: Some("Won"),
            fail: None,
            sleep: Duration::ZERO,
        }
    }
}

/// The fake's autobalance: its playtest's entrants and checks on a stage
/// that is a number, `hard`, the try of seed `s` won when `s % 10`
/// reaches it. Over seeds in tens, `hard` 5 or less is in the checks'
/// band (half the tries or more won). Its knob, `ease`, lowers it a step
/// at a time; writing a stage records it in [`Fake::wrote`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeBalance {
    /// The stage as authored.
    pub hard: u32,
    /// Why writing fails, if it does.
    pub write_fails: Option<&'static str>,
    /// Whether every try reaches the turn limit unfinished (won or lost
    /// all the same).
    pub unfinished: bool,
}

impl Default for FakeBalance {
    /// Hard 8: two tries in ten won.
    fn default() -> Self {
        Self {
            hard: 8,
            write_fails: None,
            unfinished: false,
        }
    }
}

/// The fake's knob: a step lowers `hard` by one.
struct Ease {
    from: u32,
}

impl Knob<u32> for Ease {
    fn name(&self) -> &'static str {
        "ease"
    }

    fn range(&self) -> RangeInclusive<i32> {
        -i32::try_from(self.from).unwrap_or(0)..=0
    }

    fn apply(&self, hard: &u32, step: i32) -> Option<u32> {
        hard.checked_add_signed(step)
    }

    fn describe(&self, step: i32) -> String {
        format!("{} easier", step.unsigned_abs())
    }
}

/// The fake's checks over the entrants that play, `names`.
pub fn fake_checks(names: &[&str]) -> Vec<Check> {
    let mut checks: Vec<Check> = names
        .iter()
        .map(|&name| {
            Check::rate(
                name.to_owned(),
                vec![Condition::new("won").with_min(Some(1))],
                Band { min: 50, max: 100 },
            )
            .with_floor(Some(50))
        })
        .collect();
    if names.contains(&"walk") && names.contains(&"run") {
        checks.push(Check::median("run", "turns", Want::Lower, "walk"));
    }
    checks
}

/// The fake's start option.
pub const OPTIONS: [GameOption; 1] = [GameOption {
    flag: "--seed",
    value: Some("N"),
    help: "seed it\nwith N",
}];

impl GameTools for Fake {
    fn frame_options(&self) -> &[GameOption] {
        &OPTIONS
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

    fn check(&self) -> Result<Vec<ContentError>, String> {
        self.check.clone()
    }

    fn session(&self, _: &[GameArg]) -> Result<Box<dyn Session>, String> {
        let session = self.session.clone()?;
        Ok(Box::new(session))
    }

    /// Each line of the record is a step's hash; a line `refused` is a
    /// refused command. A first line `made <engine> <game> <content>` is
    /// its stamp ([`stamp`]).
    fn replay(&self, record: &Record) -> Result<ReplayRun, String> {
        self.replay.clone()?;
        let mut lines = record.text.lines().peekable();
        let made = lines.next_if(|line| line.starts_with("made ")).map(|line| {
            let parts: Vec<&str> = line.split(' ').collect();
            let content = parts.get(3).and_then(|c| c.parse().ok());
            stamp(
                parts[1],
                parts.get(2).unwrap_or(&""),
                content.unwrap_or_default(),
            )
        });
        let steps = (0..)
            .zip(lines)
            .map(|(step, line)| ReplayStep {
                step,
                command: (step > 0).then(|| format!("Step{step}")),
                hash: line.to_owned(),
                events: vec!["Moved".to_owned()],
                refused: (line == "refused").then(|| "no".to_owned()),
            })
            .collect();
        Ok(ReplayRun {
            record: record.file.clone(),
            steps,
            outcome: Some("Won".to_owned()),
            made,
        })
    }

    fn made(&self) -> Option<Made> {
        self.made.clone()
    }

    fn bots(&self, _: &BotsInput) -> Result<BotsOutput, String> {
        self.bots.clone()
    }

    fn playtest_stages(&self) -> Vec<String> {
        self.playtest
            .as_ref()
            .map(|p| p.stages.clone())
            .unwrap_or_default()
    }

    fn playtest_defaults(&self) -> Tries {
        self.playtest.as_ref().map_or(DEFAULT_TRIES, |p| p.defaults)
    }

    fn playtest_plan(
        &self,
        stage: Option<&str>,
        input: &PlaytestInput,
    ) -> Result<PlaytestPlan, String> {
        if let Ok(mut planned) = self.planned.lock() {
            planned.push(input.clone());
        }
        let fake = self.playtest.clone()?;
        let won = i64::from(!stage.is_some_and(|s| s.starts_with("hard")));
        let (outcome, fail, sleep) = (fake.outcome, fake.fail, fake.sleep);
        Ok(PlaytestPlan {
            title: format!("the fake on {}", stage.unwrap_or("its stage")),
            entrants: vec![
                ("walk".to_owned(), None),
                ("run".to_owned(), Some("fast".to_owned())),
            ],
            groups: [("all".to_owned(), vec!["walk".to_owned(), "run".to_owned()])].into(),
            default_entrants: fake.default_entrants,
            checks: Box::new(fake_checks),
            play: Box::new(move |entrant, seed, turns| {
                std::thread::sleep(sleep);
                if fail == Some(seed) {
                    return Err("refused".to_owned());
                }
                // `run` ends a turn sooner than `walk`.
                let turns = turns.saturating_sub(u32::try_from(entrant).unwrap_or(0));
                Ok(Tried {
                    seed,
                    outcome: outcome.map(str::to_owned),
                    turns,
                    measures: [("won".to_owned(), won), ("turns".to_owned(), turns.into())].into(),
                })
            }),
            summary: vec![vec![Summary::sum("won"), Summary::median("turns")]],
        })
    }

    fn autobalance_plan(
        &self,
        stage: Option<&str>,
        input: &AutobalanceInput,
    ) -> Result<AutobalancePlan, String> {
        if let Ok(mut balanced) = self.balanced.lock() {
            balanced.push(input.clone());
        }
        let fake = self.balance.clone()?;
        let (wrote, unfinished, fails) =
            (Arc::clone(&self.wrote), fake.unfinished, fake.write_fails);
        Ok(AutobalancePlan {
            title: format!("the fake on {}", stage.unwrap_or("its stage")),
            entrants: vec![
                ("walk".to_owned(), None),
                ("run".to_owned(), Some("fast".to_owned())),
            ],
            groups: [("all".to_owned(), vec!["walk".to_owned(), "run".to_owned()])].into(),
            default_entrants: Vec::new(),
            checks: Box::new(fake_checks),
            stage: Box::new(Tunable {
                stage: fake.hard,
                knobs: vec![Box::new(Ease { from: fake.hard })],
                play: Box::new(move |&hard, entrant, seed, turns| {
                    // `run` ends a turn sooner than `walk`.
                    let turns = turns.saturating_sub(u32::try_from(entrant).unwrap_or(0));
                    let won = i64::from(seed % 10 >= u64::from(hard));
                    let outcome = if won == 1 { "Won" } else { "Lost" };
                    Ok(Tried {
                        seed,
                        outcome: (!unfinished).then(|| outcome.to_owned()),
                        turns,
                        measures: [("won".to_owned(), won), ("turns".to_owned(), turns.into())]
                            .into(),
                    })
                }),
                write: Box::new(move |&hard| {
                    if let Some(e) = fails {
                        return Err(e.to_owned());
                    }
                    if let Ok(mut wrote) = wrote.lock() {
                        wrote.push(hard);
                    }
                    Ok(vec!["stage.ron".to_owned()])
                }),
            }),
        })
    }

    fn controls(&self) -> Option<Result<Box<dyn Controls>, String>> {
        let offered = self.controls.clone()?;
        if let Err(e) = offered {
            return Some(Err(e));
        }
        let title = ScreenActions::<Action> {
            screen: "title",
            actions: None,
        };
        Some(Ok(Box::new(
            GameControls::new(KeymapDef::default()).with(title),
        )))
    }

    fn one_bit(&self, _target: &str) -> Option<Result<OneBitSkin, String>> {
        self.one_bit.clone()
    }

    fn bake(&self, target: Option<&str>) -> Result<GameBake, String> {
        let sources = self.sources.ok_or("no bake")?;
        Ok(GameBake {
            bake: Bake::new(format!("fake for {}", target.unwrap_or("all")))
                .asset::<Names>("data/names")
                .asset::<PaletteDef>("data/palette"),
            bundle: Bundle::new(vec![Layer::table("base", sources)]),
            formats: Formats::standard(),
        })
    }
}
