//! A game and a session made up for the contract's tests: each answer is
//! what the test sets.

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

use shuyi_core::bots::playtest::{
    Band, Check, EntrantTries, Found, Judged, PlaytestReport, RateVerdict, Tried,
};

use super::bots::{BotsInput, BotsOutput, tally};
use super::playtest::PlaytestInput;
use super::replay::{Record, ReplayRun, ReplayStep};
use super::session::{Event, LogEntry, Scene, Session, Sound};
use crate::frame_png::{Script, Shot, Step};
use crate::game::{GameArg, GameBake, GameLang, GameOption, GameTools, OneBitSkin};

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
}

/// A game whose tools answer what the test sets.
pub struct Fake {
    /// `check`'s answer.
    pub check: Result<Vec<ContentError>, String>,
    /// `session`'s session, or why there is none.
    pub session: Result<FakeSession, String>,
    /// `replay`: a run of the record's text, a hash a line.
    pub replay: Result<(), String>,
    /// `bots`'s plays, or why there are none.
    pub bots: Result<BotsOutput, String>,
    /// `playtest`'s report, or why there is none.
    pub playtest: Result<PlaytestReport, String>,
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
            bots: Ok(tally(Vec::new())),
            playtest: Ok(report(Some("Won"), false)),
            sources: Some(SOURCES),
            controls: None,
            one_bit: None,
        }
    }
}

/// A report of one try, unfinished if `outcome` is `None`, blocked if
/// `blocked`.
pub fn report(outcome: Option<&str>, blocked: bool) -> PlaytestReport {
    PlaytestReport {
        entrants: vec![EntrantTries {
            name: "walk".to_owned(),
            tries: vec![Tried {
                seed: 0,
                outcome: outcome.map(str::to_owned),
                turns: 3,
                measures: [("won".to_owned(), 1)].into(),
            }],
        }],
        checks: vec![Judged {
            check: Check::Rate {
                entrant: "walk".to_owned(),
                success: Vec::new(),
                band: Band { min: 0, max: 100 },
                floor: None,
            },
            found: Found::Rate {
                successes: 1,
                count: 1,
                verdict: RateVerdict::Within,
                blocked,
            },
        }],
        blocked,
    }
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
    /// refused command.
    fn replay(&self, record: &Record) -> Result<ReplayRun, String> {
        self.replay.clone()?;
        let steps = (0..)
            .zip(record.text.lines())
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
        })
    }

    fn bots(&self, _: &BotsInput) -> Result<BotsOutput, String> {
        self.bots.clone()
    }

    fn playtest(&self, _: &PlaytestInput) -> Result<PlaytestReport, String> {
        self.playtest.clone()
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
