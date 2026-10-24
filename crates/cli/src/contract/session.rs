//! A game running headless, for the operations that play it (`run`,
//! `scene`) and the MCP server's session ([`crate::mcp`]): [`Session`],
//! which any game gives as a [`Headless`] over its [`Harness`].
//!
//! The engine drives the session and reads it; the game says only how its
//! Harness starts and, if it has them, what its scene is
//! ([`Headless::with_scene`]), what happened in its rules
//! ([`Headless::with_events`]) and how its frame looks
//! ([`Headless::with_picture`]). The modules its game installed are read
//! from the game itself.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_ui::GameData;
use shuyi_ui::audio::{AudioRequest, MusicCommand};
use shuyi_ui::log::LogLine;
use shuyi_ui::module::InstalledModule;

use crate::frame_png::{Image, Script};
use crate::game::unsupported;

/// The scene a stage shows, as text and as data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// A view a game builds with `Scene::new`: it may grow (ADR-0039).
#[non_exhaustive]
pub struct Scene {
    /// The screen it is on.
    pub screen: String,
    /// What kind of scene: `grid map`, or the game's own word.
    pub kind: String,
    /// The scene as text: for a grid map, its tiles, units and cursor in
    /// the scene's text form (`MapScene::to_text`).
    pub text: String,
    /// What the game says of it as data: its stage's state.
    pub data: serde_json::Value,
}

impl Scene {
    /// The scene of kind `kind` on the screen `screen`, as the text `text`,
    /// with no data (`null`; assign [`data`](Self::data)).
    pub fn new(
        screen: impl Into<String>,
        kind: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            screen: screen.into(),
            kind: kind.into(),
            text: text.into(),
            data: serde_json::Value::Null,
        }
    }
}

/// Something that happened while a session ran, with its frame (frames
/// count from 1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Event {
    /// A screen opened.
    Opened {
        /// The frame.
        frame: u32,
        /// Its name.
        screen: String,
    },
    /// A screen closed.
    Closed {
        /// The frame.
        frame: u32,
        /// Its name.
        screen: String,
    },
    /// A music command to the platform.
    Music {
        /// The frame.
        frame: u32,
        /// `load`, `start`, `gain` or `stop`.
        command: String,
        /// The track's cue.
        cue: String,
    },
    /// A voiced line asked to play.
    Voice {
        /// The frame.
        frame: u32,
        /// The line's id.
        line: String,
    },
    /// The game asked to quit.
    Quit {
        /// The frame.
        frame: u32,
    },
    /// Something in the game's rules, in the game's words (it gives no
    /// frame).
    Game {
        /// What happened.
        text: String,
    },
}

/// A sound cue asked to play.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Sound {
    /// The frame.
    pub frame: u32,
    /// The cue.
    pub cue: String,
}

/// A line of the game's log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An answer's, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct LogEntry {
    /// The frame it was given in (frames count from 1; 0 is before the
    /// first).
    pub frame: u32,
    /// How serious it is: `error`, `warn`, `info` or `debug`.
    pub level: String,
    /// What it is about: `assets`, `audio`, `input`, `screens`, `storage`,
    /// `platform` or `game`.
    pub category: String,
    /// What it says, in English.
    pub text: String,
}

impl From<&LogLine> for LogEntry {
    fn from(line: &LogLine) -> Self {
        Self {
            frame: line.frame,
            level: line.level.name().to_owned(),
            category: line.category.name().to_owned(),
            text: line.text.clone(),
        }
    }
}

/// A module installed in the game, with what it declared (ADR-0040).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An answer's, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct ModuleEntry {
    /// Its name.
    pub name: String,
    /// The hooks the game calls on it, in the frame loop: `before_update`
    /// (before the screens' update), `draw_over` (over their drawing).
    pub hooks: Vec<String>,
    /// The types of the state it put on the context.
    pub states: Vec<String>,
    /// Its entries' names on the debug menu, as they read at install, in
    /// the order declared; the menu lists them after the game's.
    pub debug_entries: Vec<String>,
}

impl From<&InstalledModule> for ModuleEntry {
    fn from(module: &InstalledModule) -> Self {
        Self {
            name: module.name.to_owned(),
            hooks: module.hooks.iter().map(|h| h.name().to_owned()).collect(),
            states: module.states.iter().map(|&s| s.to_owned()).collect(),
            debug_entries: module.debug_entries.clone(),
        }
    }
}

/// A game running headless.
pub trait Session {
    /// Plays `script`'s steps, in order.
    fn play(&mut self, script: &Script);

    /// Runs `frames` frames with no input.
    fn idle(&mut self, frames: u32);

    /// How many frames have run.
    fn frame_count(&self) -> u32;

    /// The open screens' names, bottom first.
    fn screens(&self) -> Vec<String>;

    /// The frame the game asked to quit in, if it has.
    fn quit_frame(&self) -> Option<u32>;

    /// The current frame in the snapshot text format.
    fn snapshot(&self) -> String;

    /// What happened, in frame order: screens opening and closing, music,
    /// voices, the quit, then the game's own events.
    fn events(&self) -> Vec<Event>;

    /// The sound cues asked for, in order.
    fn sounds(&self) -> Vec<Sound>;

    /// The scene on screen, or why there is none.
    fn scene(&self) -> Result<Scene, String>;

    /// The game's log, oldest first: its last lines (none by default).
    fn log(&self) -> Vec<LogEntry> {
        Vec::new()
    }

    /// How many older lines the log dropped (0 by default).
    fn log_dropped(&self) -> u32 {
        0
    }

    /// The modules installed in the game, in install order (none by
    /// default).
    fn modules(&self) -> Vec<ModuleEntry> {
        Vec::new()
    }

    /// The current frame as the game paints it, each console pixel
    /// `scale` × `scale` (the game's choice if `None`). By default the
    /// game offers none.
    fn picture(&self, scale: Option<u32>) -> Result<Image, String> {
        let _ = scale;
        Err(unsupported(
            "a picture of a session's frame",
            "session, with Headless::with_picture",
        ))
    }
}

/// What a game says its scene is, from its Harness.
type SceneOf<G> = Box<dyn Fn(&Harness<G>) -> Result<Scene, String>>;

/// How a game paints its Harness's frame, at a scale if one is given.
type PictureOf<G> = Box<dyn Fn(&Harness<G>, Option<u32>) -> Result<Image, String>>;

/// What a game says happened in its rules, from its Harness.
type EventsOf<G> = Box<dyn Fn(&Harness<G>) -> Vec<String>>;

/// A [`Session`] over a game's [`Harness`].
pub struct Headless<G: GameData> {
    harness: Harness<G>,
    scene: SceneOf<G>,
    events: EventsOf<G>,
    picture: Option<PictureOf<G>>,
}

impl<G: GameData> Headless<G> {
    /// A session of `harness`, with no scene and no events of the game's.
    pub fn new(harness: Harness<G>) -> Self {
        Self {
            harness,
            scene: Box::new(|h| {
                Err(format!(
                    "the game's tools give no scene (the top screen is `{}`)",
                    h.top_screen()
                ))
            }),
            events: Box::new(|_| Vec::new()),
            picture: None,
        }
    }

    /// This session, its scene from `scene`.
    #[must_use]
    pub fn with_scene(
        mut self,
        scene: impl Fn(&Harness<G>) -> Result<Scene, String> + 'static,
    ) -> Self {
        self.scene = Box::new(scene);
        self
    }

    /// This session, the game's events from `events`.
    #[must_use]
    pub fn with_events(mut self, events: impl Fn(&Harness<G>) -> Vec<String> + 'static) -> Self {
        self.events = Box::new(events);
        self
    }

    /// This session, its frame painted by `picture` (most simply the
    /// game's `frame-png` painter over the Harness's frame).
    #[must_use]
    pub fn with_picture(
        mut self,
        picture: impl Fn(&Harness<G>, Option<u32>) -> Result<Image, String> + 'static,
    ) -> Self {
        self.picture = Some(Box::new(picture));
        self
    }

    /// The Harness it drives.
    pub fn harness(&self) -> &Harness<G> {
        &self.harness
    }
}

/// The screens that closed and opened between stacks `before` and
/// `after` (bottom first): those above what they share, the closed from
/// the top down, then the opened from the bottom up.
fn changes(before: &[&str], after: &[&str]) -> (Vec<String>, Vec<String>) {
    let shared = before.iter().zip(after).take_while(|(a, b)| a == b).count();
    let closed = before[shared..].iter().rev().map(|s| (*s).to_owned());
    let opened = after[shared..].iter().map(|s| (*s).to_owned());
    (closed.collect(), opened.collect())
}

/// A music command's name and cue.
fn music(command: &MusicCommand) -> (&'static str, &str) {
    match command {
        MusicCommand::Load { cue } => ("load", cue),
        MusicCommand::Start { cue } => ("start", cue),
        MusicCommand::Gain { cue, .. } => ("gain", cue),
        MusicCommand::Stop { cue } => ("stop", cue),
    }
}

impl<G: GameData> Session for Headless<G> {
    fn play(&mut self, script: &Script) {
        script.play(&mut self.harness);
    }

    fn idle(&mut self, frames: u32) {
        for _ in 0..frames {
            self.harness.frame(&[], FRAME_DT);
        }
    }

    fn frame_count(&self) -> u32 {
        self.harness.frame_count()
    }

    fn screens(&self) -> Vec<String> {
        self.harness
            .screens()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    fn quit_frame(&self) -> Option<u32> {
        self.harness.quit_frame()
    }

    fn snapshot(&self) -> String {
        self.harness.snapshot()
    }

    fn events(&self) -> Vec<Event> {
        let mut timed: Vec<(u32, Event)> = Vec::new();
        for pair in self.harness.screen_log().windows(2) {
            let ((_, before), (frame, after)) = (&pair[0], &pair[1]);
            let frame = *frame;
            let (closed, opened) = changes(before, after);
            timed.extend(
                closed
                    .into_iter()
                    .map(|screen| (frame, Event::Closed { frame, screen })),
            );
            timed.extend(
                opened
                    .into_iter()
                    .map(|screen| (frame, Event::Opened { frame, screen })),
            );
        }
        for (frame, command) in self.harness.music_log() {
            let (command, cue) = music(command);
            timed.push((
                frame,
                Event::Music {
                    frame,
                    command: command.to_owned(),
                    cue: cue.to_owned(),
                },
            ));
        }
        for (frame, request) in self.harness.audio_log() {
            if let AudioRequest::PlayVoice { line, .. } = request {
                let line = line.to_string();
                timed.push((frame, Event::Voice { frame, line }));
            }
        }
        if let Some(frame) = self.harness.quit_frame() {
            timed.push((frame, Event::Quit { frame }));
        }
        // A stable sort: within a frame, screens, then music, voices and
        // the quit.
        timed.sort_by_key(|(frame, _)| *frame);
        let game = (self.events)(&self.harness)
            .into_iter()
            .map(|text| Event::Game { text });
        timed
            .into_iter()
            .map(|(_, event)| event)
            .chain(game)
            .collect()
    }

    fn sounds(&self) -> Vec<Sound> {
        self.harness
            .audio_log()
            .into_iter()
            .filter_map(|(frame, request)| match request {
                AudioRequest::PlaySound { cue, .. } => Some(Sound {
                    frame,
                    cue: cue.clone(),
                }),
                _ => None,
            })
            .collect()
    }

    fn scene(&self) -> Result<Scene, String> {
        (self.scene)(&self.harness)
    }

    fn log(&self) -> Vec<LogEntry> {
        self.harness.log().lines().map(LogEntry::from).collect()
    }

    fn log_dropped(&self) -> u32 {
        self.harness.log().dropped()
    }

    fn modules(&self) -> Vec<ModuleEntry> {
        let modules = self.harness.game().modules();
        modules.iter().map(ModuleEntry::from).collect()
    }

    fn picture(&self, scale: Option<u32>) -> Result<Image, String> {
        match &self.picture {
            Some(picture) => picture(&self.harness, scale),
            None => Err(unsupported(
                "a picture of a session's frame",
                "session, with Headless::with_picture",
            )),
        }
    }
}

#[cfg(test)]
mod tests;
