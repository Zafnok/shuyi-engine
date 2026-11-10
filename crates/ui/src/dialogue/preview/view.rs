//! What the dialogue preview shows, as plain data: no cells, glyphs,
//! colours or positions. The screen builds a [`PreviewView`] each frame
//! ([`DialoguePreview::view`](super::DialoguePreview::view)) and a skin
//! paints it ([`super::glyph::paint`] today). Tests of what happened read
//! it.

use crate::prelude::*;

/// A tab of the preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Tab {
    /// The scenes of the folder's files.
    Scenes,
    /// Who is there, the game's switches and the strip's.
    Switches,
    /// The scene's text boxes, and the line box.
    Lines,
    /// The files' problems.
    Problems,
}

impl Tab {
    /// Every tab, in the order shown.
    pub const ALL: [Tab; 4] = [Tab::Scenes, Tab::Switches, Tab::Lines, Tab::Problems];

    /// The text key of its name.
    pub const fn key(self) -> &'static str {
        match self {
            Tab::Scenes => "preview.tab.scenes",
            Tab::Switches => "preview.tab.switches",
            Tab::Lines => "preview.tab.lines",
            Tab::Problems => "preview.tab.problems",
        }
    }

    /// Its place in [`Tab::ALL`].
    pub const fn index(self) -> usize {
        match self {
            Tab::Scenes => 0,
            Tab::Switches => 1,
            Tab::Lines => 2,
            Tab::Problems => 3,
        }
    }
}

/// The whole screen, as it is this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PreviewView {
    /// Whether the tabs show; else the scene plays under the strip.
    pub open: bool,
    /// The tab shown.
    pub tab: Tab,
    /// The tabs' names, in [`Tab::ALL`] order.
    pub tabs: Vec<String>,
    /// The focused row of the tab shown: on the Lines tab, 0 is the line
    /// box and `n` is `lines[n - 1]`.
    pub focus: usize,
    /// The Scenes tab: every scene, by file and in file order.
    pub scenes: Vec<SceneRow>,
    /// The Switches tab, top to bottom.
    pub switches: Vec<SwitchRow>,
    /// The line box, the Lines tab's first row.
    pub go_to: GoToView,
    /// The Lines tab's text boxes, in the order they play.
    pub lines: Vec<LineRow>,
    /// The Problems tab, by file and line.
    pub problems: Vec<ProblemRow>,
    /// A note on the tab shown, when it has nothing to list.
    pub note: Option<String>,
    /// The scene picked, playing or over.
    pub playing: Option<PlayingView>,
    /// The strip over the scene playing, when it shows.
    pub strip: Option<StripView>,
    /// The help line: what the keys do here.
    pub help: String,
}

impl PreviewView {
    /// The tab `tab` shown, its tabs named `tabs`, closed, and nothing in
    /// it.
    pub fn new(tab: Tab, tabs: Vec<String>) -> Self {
        Self {
            open: true,
            tab,
            tabs,
            focus: 0,
            scenes: Vec::new(),
            switches: Vec::new(),
            go_to: GoToView::new(String::new()),
            lines: Vec::new(),
            problems: Vec::new(),
            note: None,
            playing: None,
            strip: None,
            help: String::new(),
        }
    }

    /// The text box the scene plays, if it plays.
    pub fn current_line(&self) -> Option<&LineRow> {
        self.lines.iter().find(|l| l.current)
    }

    /// The switch named `name`, if it shows.
    pub fn switch(&self, name: &str) -> Option<&SwitchRow> {
        self.switches.iter().find(|s| s.name == name)
    }
}

/// A scene of the Scenes tab.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SceneRow {
    /// Its file in the folder.
    pub file: String,
    /// Its id.
    pub id: String,
    /// Whether its file has problems.
    pub problems: bool,
    /// Whether it is the scene picked.
    pub picked: bool,
}

impl SceneRow {
    /// The scene `id` of `file`, its file with no problem, not picked.
    pub fn new(file: String, id: String) -> Self {
        Self {
            file,
            id,
            problems: false,
            picked: false,
        }
    }
}

/// A switch of the Switches tab.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SwitchRow {
    /// Its name: a character's id, or the name the game gave.
    pub name: String,
    /// Its value, in words.
    pub value: String,
    /// Whether it is on (the character there).
    pub on: bool,
}

impl SwitchRow {
    /// The switch `name`, showing `value`.
    pub fn new(name: String, value: String, on: bool) -> Self {
        Self { name, value, on }
    }
}

/// The line box: a line of the file to play the scene from.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct GoToView {
    /// Its label.
    pub label: String,
    /// The digits in it.
    pub digits: String,
    /// Whether it is open, taking digits.
    pub typing: bool,
}

impl GoToView {
    /// The closed box labelled `label`.
    pub fn new(label: String) -> Self {
        Self {
            label,
            digits: String::new(),
            typing: false,
        }
    }
}

/// A text box of the Lines tab.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LineRow {
    /// The line of the file it starts on.
    pub line: u32,
    /// Who says it (a character's id), `None` for narration.
    pub speaker: Option<String>,
    /// Its words, as written.
    pub words: String,
    /// Whether the scene plays it now.
    pub current: bool,
}

impl LineRow {
    /// The narration `words` of line `line`, not playing.
    pub fn new(line: u32, words: String) -> Self {
        Self {
            line,
            speaker: None,
            words,
            current: false,
        }
    }
}

/// A problem of the Problems tab.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProblemRow {
    /// Its file.
    pub file: String,
    /// Its line, if known.
    pub line: Option<u32>,
    /// What is wrong.
    pub message: String,
    /// What to do about it, where the message says.
    pub fix: Option<String>,
}

impl ProblemRow {
    /// `message` about `file`, at no line, with no fix.
    pub fn new(file: String, message: String) -> Self {
        Self {
            file,
            line: None,
            message,
            fix: None,
        }
    }
}

/// The scene picked.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PlayingView {
    /// Its file.
    pub file: String,
    /// Its id.
    pub scene: String,
    /// The text box it is at, 0 the first (the last it was at, once over).
    pub text_box: usize,
    /// Whether it is over.
    pub finished: bool,
}

impl PlayingView {
    /// The scene `scene` of `file`, at its first text box.
    pub fn new(file: String, scene: String) -> Self {
        Self {
            file,
            scene,
            text_box: 0,
            finished: false,
        }
    }
}

/// The strip over the scene playing: where its line is, and how it is
/// said.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct StripView {
    /// The scene's file.
    pub file: String,
    /// The line of the file the text box starts on.
    pub line: u32,
    /// How the line is said, if the script says.
    pub direction: Option<String>,
    /// What brings the tabs back.
    pub help: String,
}

impl StripView {
    /// The strip at `line` of `file`, with no direction.
    pub fn new(file: String, line: u32) -> Self {
        Self {
            file,
            line,
            direction: None,
            help: String::new(),
        }
    }
}
