//! The live link (ADR-0047): how a tool reaches a build that is already
//! running, in a debug build with the backend's `link` feature. The wire is
//! JSON lines, one UTF-8 JSON object per line: the client sends a
//! [`Request`] per line, and the build answers each with an [`Answer`], in
//! order. The first line must be a [`Request::Hello`] with the build's
//! token, or the connection is closed.
//!
//! This module touches no device. A backend carries the lines (TCP on
//! loopback in `shuyi-app`) and the [`Server`] turns each into its answer
//! over the [`App`] it runs: it reads the screens, the frame as text, the
//! window's picture (taken by the backend) and the log, and its only input
//! is raw device events the backend adds after the devices' own in
//! [`Platform::poll_input`](crate::Platform::poll_input), so they go where a
//! player's go. It changes nothing any other way.
//!
//! `shuyi attach` is the client, and shares these types with the build, so
//! the two ends can't drift; [`VERSION`] says which wire they speak.

pub mod base64;

use crate::alloc_prelude::*;
use alloc::collections::VecDeque;
use alloc::{format, vec};

use serde::{Deserialize, Serialize};
use shuyi_ui::input::{ActionSet, Button, ButtonChord, Chord, Keymap, PadKind};
use shuyi_ui::log::LogRing;
use shuyi_ui::{GameData, RawInputEvent};

use crate::app::App;

/// The link's version: a client and a build speak the same, or the build
/// closes the link after its hello.
pub const VERSION: u32 = 1;

/// The longest request line, in bytes: a longer one closes the link.
pub const MAX_LINE: usize = 64 * 1024;

/// The most bytes of answers a client may leave unread: past it, the
/// backend drops the client.
pub const MAX_UNSENT: usize = 64 * 1024 * 1024;

/// The most frames one [`Step::Wait`] may hold: about 28 minutes at 60 a
/// second.
pub const MAX_WAIT: u32 = 100_000;

/// A line a client sends, tagged by `op`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum Request {
    /// The first line: the build's token, and the link's version the
    /// client speaks.
    Hello {
        /// The token the build logged when it started listening.
        token: String,
        /// The client's [`VERSION`].
        link: u32,
    },
    /// Where the build is: [`Status`].
    Status,
    /// The frame as text, in the snapshot format.
    Frame,
    /// The window's picture, as a PNG, after the next frame is shown.
    Screenshot,
    /// Steps played as a player's input, frame by frame: answered when
    /// the last has run.
    Play {
        /// The steps, in order.
        steps: Vec<Step>,
    },
    /// The log's kept lines numbered `since` or more.
    Log {
        /// The first line's number: 0, the default, for every kept line;
        /// the last answer's `next` for the lines since.
        #[serde(default)]
        since: u64,
    },
}

impl Request {
    /// The request as one line, with no line break.
    pub fn to_line(&self) -> String {
        // An enum of strings and numbers: serializing can't fail.
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// One step of a [`Request::Play`], as the Harness's test scripts have
/// them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Step {
    /// Each whitespace-separated chord (`"Enter Shift+Space"`) pressed in
    /// one frame and let go in the next, as `Harness::keys` does.
    Keys(String),
    /// Each whitespace-separated controller button (`"South"`; buttons
    /// joined by `+` go down together) pressed on a generic pad in one
    /// frame and let go in the next, as `Harness::pad` does.
    Pad(String),
    /// The game's action of this name, pressed by its primary key or else
    /// its primary button ([`press_for`]), as a key or button step.
    Act(String),
    /// This many frames with no input.
    Wait(u32),
}

impl Step {
    /// Whether it can be played: names `Chord::parse` and `Button::parse`
    /// know, an action named, a wait of at most [`MAX_WAIT`] frames. An
    /// action's name is looked up when its play starts.
    pub fn check(&self) -> Result<(), String> {
        match self {
            Step::Keys(script) => {
                for token in script.split_whitespace() {
                    Chord::parse(token).map_err(|e| format!("keys: {e}"))?;
                }
            }
            Step::Pad(script) => {
                for token in script.split_whitespace() {
                    buttons(token).map_err(|e| format!("pad: {e}"))?;
                }
            }
            Step::Act(name) => {
                if name.trim().is_empty() {
                    return Err("act: no action named".to_owned());
                }
            }
            Step::Wait(frames) => {
                if *frames > MAX_WAIT {
                    return Err(format!(
                        "wait: at most {MAX_WAIT} frames in one step, not {frames}"
                    ));
                }
            }
        }
        Ok(())
    }
}

/// The buttons of one pad token, `"South"` or `"South+East"`.
fn buttons(token: &str) -> Result<Vec<Button>, String> {
    token.split('+').map(Button::parse).collect()
}

/// What presses an action: a key chord, or a button (or two together).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Press {
    /// The action's primary key chord.
    Keys(Chord),
    /// Its primary button, or button chord.
    Pad(ButtonChord),
}

/// What presses the game's action named `name` in `keymap`: its primary
/// key ([`Keymap::primary`]), else its primary button
/// ([`Keymap::primary_button`]). The one lookup of an action by name, so
/// every tool presses an action as a player would. An error names the
/// action: the game has none of that name, or it has neither a key nor a
/// button.
pub fn press_for<A: ActionSet>(keymap: &Keymap<A>, name: &str) -> Result<Press, String> {
    let Some(action) = A::from_name(name) else {
        let names: Vec<&str> = A::ALL.iter().map(|a| a.name()).collect();
        return Err(format!(
            "act: the game has no action {name:?}; its actions are {}",
            names.join(", ")
        ));
    };
    if let Some(chord) = keymap.primary(action) {
        return Ok(Press::Keys(chord));
    }
    keymap
        .primary_button(action)
        .map(Press::Pad)
        .ok_or_else(|| {
            format!("act: {name} has neither a key nor a button in the player's keymap now")
        })
}

/// Which part of its life the game is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Phase {
    /// Its loading card shows.
    Loading,
    /// It runs.
    Running,
    /// Its load failed: the errors show.
    Failed,
}

/// Where a running build is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
// An answer, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct Status {
    /// The frames run: [`App::frame_number`].
    pub frames: u32,
    /// Loading, running or failed.
    pub phase: Phase,
    /// Why the game didn't load, if it failed.
    pub errors: Option<String>,
    /// The open screens, bottom first.
    pub screens: Vec<String>,
    /// The top screen's name; empty with none open.
    pub top_screen: String,
    /// Whether the system has suspended the game.
    pub suspended: bool,
    /// Whether its window has the player's attention.
    pub focused: bool,
    /// Whether the game has quit.
    pub quit: bool,
}

impl Status {
    /// Where `app` is now.
    pub fn of<G: GameData>(app: &App<G>) -> Self {
        let phase = if app.is_loading() {
            Phase::Loading
        } else if app.errors().is_some() {
            Phase::Failed
        } else {
            Phase::Running
        };
        let screens: Vec<String> = app
            .game()
            .map(|game| game.screens().into_iter().map(str::to_owned).collect())
            .unwrap_or_default();
        Self {
            frames: app.frame_number(),
            phase,
            errors: app.errors().map(str::to_owned),
            top_screen: screens.last().cloned().unwrap_or_default(),
            screens,
            suspended: app.is_suspended(),
            focused: app.is_focused(),
            quit: app.has_quit(),
        }
    }
}

/// The window's picture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Screenshot {
    /// The PNG file, in [`base64`].
    pub png: String,
    /// Its width in pixels.
    pub width: u32,
    /// Its height in pixels.
    pub height: u32,
    /// What it doesn't show: the loading card or the errors instead of
    /// the game.
    pub limit: Option<String>,
}

/// What a [`Request::Play`] did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Played {
    /// The frames its steps took.
    pub frames: u32,
    /// Whether the steps waited for the game to load.
    pub waited_loading: bool,
    /// Whether the steps waited while the game was suspended.
    pub waited_suspended: bool,
}

/// One line of the log, with its number in the ring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct LogEntry {
    /// Its number: lines are numbered from 0 as they are logged.
    pub seq: u64,
    /// The frame it was given in.
    pub frame: u32,
    /// How serious it is: `error`, `warn`, `info` or `debug`.
    pub level: String,
    /// What it is about (`assets`, `platform`, `game`, …).
    pub category: String,
    /// What it says.
    pub text: String,
}

/// The log's lines a [`Request::Log`] asked for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct LogAnswer {
    /// The kept lines numbered `since` or more, oldest first.
    pub lines: Vec<LogEntry>,
    /// The number the next line gets: the next request's `since`.
    pub next: u64,
    /// How many lines numbered `since` or more the ring no longer keeps.
    pub dropped: u64,
}

/// A line the build sends, tagged by `answer`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "answer", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum Answer {
    /// The hello's answer: the build's link version (a client of another
    /// is closed after it) and its game, the window's title.
    Hello {
        /// The build's [`VERSION`].
        link: u32,
        /// The game: the window's title.
        game: String,
    },
    /// [`Request::Status`]'s.
    Status(Status),
    /// [`Request::Frame`]'s: the frame in the snapshot format, empty
    /// while the game isn't running, with a limit saying why.
    Frame {
        /// The frame as text.
        frame: String,
        /// Why there is none.
        limit: Option<String>,
    },
    /// [`Request::Screenshot`]'s.
    Screenshot(Screenshot),
    /// [`Request::Play`]'s, once its last frame has run.
    Played(Played),
    /// [`Request::Log`]'s.
    Log(LogAnswer),
    /// A request that couldn't be answered: why.
    Error {
        /// What was wrong.
        error: String,
    },
    /// Another client holds the link: this one is closed after this line.
    Busy,
}

impl Answer {
    /// The answer as one line, with no line break.
    pub fn to_line(&self) -> String {
        // Strings, numbers and lists: serializing can't fail.
        serde_json::to_string(self).unwrap_or_default()
    }

    /// The answer `line` holds.
    pub fn from_line(line: &str) -> Result<Self, String> {
        serde_json::from_str(line.trim_end()).map_err(|e| format!("not a link answer: {e}"))
    }

    fn error(error: impl Into<String>) -> String {
        Self::Error {
            error: error.into(),
        }
        .to_line()
    }
}

/// What the [`Server`] made of a line.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Handled {
    /// Its answer, to send now.
    Answer(String),
    /// Its answer comes from [`Server::after_frame`]: a play's when its
    /// last frame has run, or any request's behind a play or a picture.
    Later,
    /// The backend answers it after this frame is shown, with
    /// [`Server::screenshot`].
    Screenshot,
    /// Close the connection, after sending this line if there is one,
    /// and read nothing more: the first line wasn't a hello with the
    /// token, a line was longer than [`MAX_LINE`], or the client speaks
    /// another link version (the line is the hello's answer, saying the
    /// build's).
    Close(Option<String>),
}

/// One frame of a play: the events it gives, or frames with none.
#[derive(Debug, Clone, PartialEq)]
enum Planned {
    Events(Vec<RawInputEvent>),
    Idle(u32),
}

/// A play being played.
#[derive(Debug)]
struct Play {
    steps: Vec<Step>,
    /// Its frames, once its actions are looked up (the game has loaded).
    frames: Option<VecDeque<Planned>>,
    /// The frames given so far.
    run: u32,
    waited_loading: bool,
    waited_suspended: bool,
}

/// What the server is busy with: later requests wait for it.
#[derive(Debug)]
enum Busy {
    Idle,
    Play(Play),
    /// A picture the backend takes after this frame is shown.
    Screenshot {
        limit: Option<String>,
    },
}

/// The build's end of the link, for one client: it turns the client's
/// lines into answers over the [`App`] the backend runs, and gives the
/// backend a play's input frame by frame. The backend makes one per
/// connection.
///
/// Each frame the backend calls [`handle`](Self::handle) for each line
/// read and sends the answers; [`input`](Self::input) before
/// [`App::frame`], adding what it gives after the devices' input; then
/// [`after_frame`](Self::after_frame), sending what it gives; and, if
/// [`wants_screenshot`](Self::wants_screenshot), reads the window and
/// sends [`screenshot`](Self::screenshot)'s line.
#[derive(Debug)]
pub struct Server {
    token: String,
    game: String,
    hello: bool,
    busy: Busy,
    /// The request lines behind a play or a picture, in order.
    waiting: VecDeque<String>,
    /// The answers [`after_frame`](Self::after_frame) gives.
    due: Vec<String>,
}

impl Server {
    /// The server of a build whose token is `token`, of the game `game`
    /// (the window's title).
    pub fn new(token: String, game: String) -> Self {
        Self {
            token,
            game,
            hello: false,
            busy: Busy::Idle,
            waiting: VecDeque::new(),
            due: Vec::new(),
        }
    }

    /// What `line` (without its line break) asks of `app`.
    pub fn handle<G: GameData>(&mut self, line: &str, app: &App<G>) -> Handled {
        if line.len() > MAX_LINE {
            return Handled::Close(None);
        }
        if !self.hello {
            return self.hello(line);
        }
        if !matches!(self.busy, Busy::Idle) || !self.waiting.is_empty() {
            self.waiting.push_back(line.to_owned());
            return Handled::Later;
        }
        self.request(line, app)
    }

    /// The first line: a hello with the token and this link's version.
    fn hello(&mut self, line: &str) -> Handled {
        let Ok(Request::Hello { token, link }) = serde_json::from_str::<Request>(line) else {
            return Handled::Close(None);
        };
        if token != self.token {
            return Handled::Close(None);
        }
        let answer = Answer::Hello {
            link: VERSION,
            game: self.game.clone(),
        }
        .to_line();
        if link != VERSION {
            return Handled::Close(Some(answer));
        }
        self.hello = true;
        Handled::Answer(answer)
    }

    /// A request after the hello, with nothing before it waiting.
    fn request<G: GameData>(&mut self, line: &str, app: &App<G>) -> Handled {
        let request = match serde_json::from_str::<Request>(line) {
            Ok(request) => request,
            Err(e) => return Handled::Answer(Answer::error(format!("not a request: {e}"))),
        };
        let answer = match request {
            Request::Hello { .. } => Answer::error("the link has said hello already"),
            Request::Status => Answer::Status(Status::of(app)).to_line(),
            Request::Frame => frame(app).to_line(),
            Request::Log { since } => Answer::Log(log(app.log(), since)).to_line(),
            Request::Screenshot => {
                self.busy = Busy::Screenshot {
                    limit: shot_limit(app),
                };
                return Handled::Screenshot;
            }
            Request::Play { steps } => {
                for (i, step) in steps.iter().enumerate() {
                    if let Err(e) = step.check() {
                        return Handled::Answer(Answer::error(format!("step {}: {e}", i + 1)));
                    }
                }
                if steps.is_empty() {
                    Answer::Played(Played {
                        frames: 0,
                        waited_loading: false,
                        waited_suspended: false,
                    })
                    .to_line()
                } else {
                    self.busy = Busy::Play(Play {
                        steps,
                        frames: None,
                        run: 0,
                        waited_loading: false,
                        waited_suspended: false,
                    });
                    return Handled::Later;
                }
            }
        };
        Handled::Answer(answer)
    }

    /// This frame's input from the play being played, if one is: the next
    /// frame of its steps, put in `out` after what is there. While the
    /// game loads or is suspended the steps wait. A play whose game failed
    /// to load, or one of whose actions can't be pressed, ends with an
    /// error, having played nothing more.
    pub fn input<G: GameData>(&mut self, app: &App<G>, out: &mut Vec<RawInputEvent>) {
        let Busy::Play(play) = &mut self.busy else {
            return;
        };
        if let Some(errors) = app.errors() {
            let error = format!("the game didn't load, so the steps can't be played: {errors}");
            self.end_play(Answer::error(error));
            return;
        }
        if app.is_loading() {
            play.waited_loading = true;
            return;
        }
        if app.is_suspended() {
            play.waited_suspended = true;
            return;
        }
        let Some(game) = app.game() else {
            return;
        };
        if play.frames.is_none() {
            match plan(&play.steps, game.input().keymap()) {
                Ok(frames) if frames.is_empty() => {
                    let played = Played {
                        frames: 0,
                        waited_loading: play.waited_loading,
                        waited_suspended: play.waited_suspended,
                    };
                    self.end_play(Answer::Played(played).to_line());
                    return;
                }
                Ok(frames) => play.frames = Some(frames),
                Err(e) => {
                    self.end_play(Answer::error(e));
                    return;
                }
            }
        }
        let frames = play.frames.get_or_insert_with(VecDeque::new);
        match frames.front_mut() {
            Some(Planned::Events(events)) => {
                out.append(events);
                frames.pop_front();
            }
            Some(Planned::Idle(n)) => {
                *n = n.saturating_sub(1);
                if *n == 0 {
                    frames.pop_front();
                }
            }
            None => {}
        }
        play.run = play.run.saturating_add(1);
    }

    /// Ends the play with `answer`, due after this frame.
    fn end_play(&mut self, answer: String) {
        self.busy = Busy::Idle;
        self.due.push(answer);
    }

    /// The answers due now that `app`'s frame has run: a play's whose last
    /// frame it was, then those of the requests that waited behind it, up
    /// to the next play or picture.
    pub fn after_frame<G: GameData>(&mut self, app: &App<G>) -> Vec<String> {
        match &mut self.busy {
            Busy::Play(play) if play.frames.as_ref().is_some_and(VecDeque::is_empty) => {
                let answer = Answer::Played(Played {
                    frames: play.run,
                    waited_loading: play.waited_loading,
                    waited_suspended: play.waited_suspended,
                });
                self.end_play(answer.to_line());
            }
            Busy::Screenshot { limit } => *limit = shot_limit(app),
            Busy::Idle | Busy::Play(_) => {}
        }
        while matches!(self.busy, Busy::Idle)
            && let Some(line) = self.waiting.pop_front()
        {
            match self.request(&line, app) {
                Handled::Answer(answer) => self.due.push(answer),
                Handled::Later | Handled::Screenshot | Handled::Close(_) => {}
            }
        }
        core::mem::take(&mut self.due)
    }

    /// Whether the backend should read the window after this frame and
    /// answer with [`screenshot`](Self::screenshot).
    pub fn wants_screenshot(&self) -> bool {
        matches!(self.busy, Busy::Screenshot { .. })
    }

    /// The answer to a [`Request::Screenshot`]: `png`, the window's
    /// picture of `width` × `height` pixels. Requests behind it are
    /// answered from the next [`after_frame`](Self::after_frame).
    pub fn screenshot(&mut self, png: &[u8], width: u32, height: u32) -> String {
        let limit = match core::mem::replace(&mut self.busy, Busy::Idle) {
            Busy::Screenshot { limit } => limit,
            other => {
                self.busy = other;
                None
            }
        };
        Answer::Screenshot(Screenshot {
            png: base64::encode(png),
            width,
            height,
            limit,
        })
        .to_line()
    }
}

/// The frame `app` shows, as text.
fn frame<G: GameData>(app: &App<G>) -> Answer {
    match app.game() {
        Some(game) => Answer::Frame {
            frame: game.buffer().to_snapshot(&game.ctx().palette),
            limit: None,
        },
        None => Answer::Frame {
            frame: String::new(),
            limit: Some(not_running(app)),
        },
    }
}

/// Why `app` shows no game: its loading card, or the errors of its load.
fn not_running<G: GameData>(app: &App<G>) -> String {
    match app.errors() {
        Some(errors) => format!("the build's game didn't load: {errors}"),
        None => "the build shows its loading card".to_owned(),
    }
}

/// What a picture of `app`'s window doesn't show.
fn shot_limit<G: GameData>(app: &App<G>) -> Option<String> {
    app.game()
        .is_none()
        .then(|| format!("{}: the picture shows that, not the game", not_running(app)))
}

/// `ring`'s kept lines numbered `since` or more, each with its number.
fn log(ring: &LogRing, since: u64) -> LogAnswer {
    let next = ring.next_seq();
    let kept = u64::try_from(ring.lines().len()).unwrap_or(u64::MAX);
    let first = next.saturating_sub(kept);
    let lines = (since.max(first)..)
        .zip(ring.since(since))
        .map(|(seq, line)| LogEntry {
            seq,
            frame: line.frame,
            level: line.level.name().to_owned(),
            category: line.category.name().to_owned(),
            text: line.text.clone(),
        })
        .collect();
    LogAnswer {
        lines,
        next,
        dropped: first.saturating_sub(since),
    }
}

/// The frames `steps` give with `keymap`, waits of no frame left out; an
/// error names the step whose action can't be pressed.
fn plan<A: ActionSet>(steps: &[Step], keymap: &Keymap<A>) -> Result<VecDeque<Planned>, String> {
    let mut frames = VecDeque::new();
    for (i, step) in steps.iter().enumerate() {
        let at = |e: String| format!("step {}: {e}", i + 1);
        match step {
            Step::Keys(script) => {
                for token in script.split_whitespace() {
                    let chord = Chord::parse(token).map_err(|e| at(format!("keys: {e}")))?;
                    press_keys(&mut frames, chord);
                }
            }
            Step::Pad(script) => {
                for token in script.split_whitespace() {
                    let buttons = buttons(token).map_err(|e| at(format!("pad: {e}")))?;
                    press_pad(&mut frames, &buttons);
                }
            }
            Step::Act(name) => match press_for(keymap, name).map_err(at)? {
                Press::Keys(chord) => press_keys(&mut frames, chord),
                Press::Pad(chord) => {
                    let buttons = match chord.buttons() {
                        Some((a, b)) => vec![a, b],
                        None => vec![chord.first()],
                    };
                    press_pad(&mut frames, &buttons);
                }
            },
            Step::Wait(n) => {
                if *n > 0 {
                    frames.push_back(Planned::Idle(*n));
                }
            }
        }
    }
    Ok(frames)
}

/// `chord` down in one frame and its key up in the next.
fn press_keys(frames: &mut VecDeque<Planned>, chord: Chord) {
    frames.push_back(Planned::Events(vec![RawInputEvent::Down(chord)]));
    frames.push_back(Planned::Events(vec![RawInputEvent::Up(chord.key)]));
}

/// `buttons` down together on a generic pad in one frame, and up in the
/// next.
fn press_pad(frames: &mut VecDeque<Planned>, buttons: &[Button]) {
    let kind = PadKind::default();
    let down = buttons.iter().map(|&b| RawInputEvent::PadDown(b, kind));
    frames.push_back(Planned::Events(down.collect()));
    let up = buttons.iter().map(|&b| RawInputEvent::PadUp(b));
    frames.push_back(Planned::Events(up.collect()));
}

#[cfg(test)]
mod tests;
