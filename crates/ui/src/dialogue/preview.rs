//! A dialogue preview: the program a game hands a writer who has no source.
//! It lists the scenes of the `.dlg` files in a folder, plays the one
//! picked in the game's own dialogue screen, and reads the files again
//! when one is saved.
//!
//! The game hands in what only it knows ([`PreviewGame`]): its own lines
//! (its [`Directive`] type), the rules the format's checks hold the files
//! to, its own checks, its own named switches (a hero named by the player
//! or not, say), and the screen that plays a scene ([`PreviewScreen`]). The
//! preview does the rest ([`DialoguePreview`]), in four tabs:
//!
//! - **Scenes**: every scene, by file and in file order, a file with
//!   problems marked. Confirm plays one from its first text box.
//! - **Switches**: one per character the scene's `@if` blocks test
//!   ([`Scene::conditions`]), all there at first; the game's own switches;
//!   and whether a strip over the scene shows the file, the line number and
//!   the line's direction ([`DialoguePlayer::direction`]). Changing one
//!   plays the scene again from the same text box.
//! - **Lines**: the scene's text boxes as they play for those there
//!   ([`ParsedScene::text_boxes`]), to play from one, and a box to type a
//!   line number of the file in ([`ParsedScene::text_box_at`]).
//! - **Problems**: every problem of the format's checks and the game's,
//!   with its file and line, and the fix where its message carries one.
//!
//! While a scene plays the game's screen takes the input, but Cancel, which
//! brings the tabs back over it. Every control is one of the
//! [`UiAction`]s a game maps its own actions onto ([`ActionSet::ui`]); the
//! line box takes typed digits too.
//!
//! **Reload.** The files come from a [`DialogueFolder`] module, which reads
//! them again when one changes on disk (`Module::reloaded`; a folder read
//! with `shuyi_app::read_watched_folder` is watched in every build,
//! ADR-0095). The scene playing goes on from the same text box if it still
//! has one, else from its start. A file that no longer parses keeps its
//! last good scenes and lists its problems.
//!
//! **The look is a skin.** [`DialoguePreview::view`] says what the screen
//! shows as plain data ([`PreviewView`]), and [`glyph::paint`] draws it.

pub mod folder;
pub mod glyph;
pub mod view;

use crate::prelude::*;
use alloc::collections::BTreeSet;
use core::any::Any;
use core::fmt;

#[cfg(feature = "std")]
pub use folder::drop_scenes;
pub use folder::{DialogueFiles, DialogueFolder};
pub use view::{
    GoToView, LineRow, PlayingView, PreviewView, ProblemRow, SceneRow, StripView, SwitchRow, Tab,
};

use shuyi_assets::ContentError;
use shuyi_assets::dialogue::{
    CharacterId, DialogueFile, Directive, LineId, ParsedScene, Present, Scene, SceneRules, Step,
    parse_dlg, scripts_from_parsed,
};
use shuyi_assets::error::codes;
use shuyi_assets::keymap::{ActionSet, UiAction};

use super::{DialogueHooks, DialoguePlayer};
use crate::audio::MenuSound;
use crate::ctx::{Ctx, GameData};
use crate::frame::glyph_buffer::GlyphBuffer;
use crate::input::fixed::{TextKey, text_key};
use crate::input::ui_actions;
use crate::screen::{FrameInput, Screen, Transition};

/// Text key ([`Ctx::text`]) of a character's switch while they are there.
pub const HERE: &str = "preview.switch.here";
/// Text key of a character's switch while they are gone.
pub const GONE: &str = "preview.switch.gone";
/// Text key of a switch of the game's that is on.
pub const ON: &str = "preview.switch.on";
/// Text key of a switch of the game's that is off.
pub const OFF: &str = "preview.switch.off";
/// Text key of the switch that shows the strip over the scene.
pub const STRIP_SWITCH: &str = "preview.switch.strip";
/// Text key of the line box's label.
pub const GO_TO: &str = "preview.lines.go_to";
/// Text key of the note on an empty Scenes tab.
pub const NO_SCENES: &str = "preview.note.no_scenes";
/// Text key of the note on an empty Scenes tab where a file dropped on the
/// window joins the folder ([`PlatformFacts::can_drop_files`]): the note
/// says to drop one. A game's text without it shows [`NO_SCENES`] there
/// too.
///
/// [`PlatformFacts::can_drop_files`]: crate::PlatformFacts::can_drop_files
pub const NO_SCENES_DROP: &str = "preview.note.no_scenes_drop";
/// Text key of the note on the Switches or Lines tab before a scene is
/// picked.
pub const NO_SCENE_PICKED: &str = "preview.note.no_scene";
/// Text key of the note on an empty Problems tab.
pub const NO_PROBLEMS: &str = "preview.note.no_problems";
/// Text key of the note once the scene playing is over.
pub const FINISHED: &str = "preview.note.finished";
/// Text key of the help line over the tabs.
pub const HELP: &str = "preview.help";
/// Text key of the help line while the line box is open.
pub const HELP_GO_TO: &str = "preview.help.go_to";
/// Text key of the help on the strip while a scene plays.
pub const HELP_PLAYING: &str = "preview.help.playing";

/// The most digits the line box takes.
const MAX_DIGITS: usize = 6;

/// What a game hands its dialogue preview: what only the game knows. No
/// method names anything of one game; a game implements it on a type of
/// its own (a unit struct, or one holding what its checks need).
///
/// A trait rather than a struct of functions: the game's lines and its
/// rules are types ([`Directive`](Self::Directive), [`Rules`](Self::Rules)),
/// which a struct of functions would carry as type parameters on every
/// use.
pub trait PreviewGame<G: GameData>: 'static {
    /// The game's own lines, as its `.dlg` files hold them.
    type Directive: Directive + 'static;

    /// The rules the format's checks hold the files to
    /// ([`scripts_from_parsed`]): the text box's size, the tokens, who may
    /// speak, and the game's own checks on each scene
    /// ([`SceneRules::scene_problems`]).
    type Rules: SceneRules<Self::Directive>;

    /// The rules, made from the game's data on `ctx`.
    fn rules(&self, ctx: &Ctx<G>) -> Self::Rules;

    /// The game's own checks over every scene that parsed, beyond each
    /// scene's ([`SceneRules::scene_problems`]): each problem with its file
    /// and line. None by default.
    fn problems(
        &self,
        _ctx: &Ctx<G>,
        _scenes: &[ParsedScene<Self::Directive>],
    ) -> Vec<ContentError> {
        Vec::new()
    }

    /// The game's own named switches, as the writer reads their names
    /// (in the player's language), each off at first: what the screen
    /// [`play`](Self::play) makes reads ([`Play::switches`]). A game with a
    /// hero the player names offers one to show the name the player gave.
    /// None by default.
    fn switches(&self, _ctx: &Ctx<G>) -> Vec<String> {
        Vec::new()
    }

    /// The game's dialogue screen playing `play`: its scene, for those
    /// there, from its text box, with the switches as they are.
    /// [`Play::player`] starts the player.
    fn play(&self, ctx: &mut Ctx<G>, play: &Play<'_, Self::Directive>)
    -> Box<dyn PreviewScreen<G>>;
}

/// A game's dialogue screen, as a preview plays it: a [`Screen`] that also
/// says what it shows. The preview calls its `update` and `draw` itself;
/// its `Pop` means the scene is over.
pub trait PreviewScreen<G: GameData>: Screen<G> {
    /// What it shows now.
    fn showing(&self) -> Showing;
}

/// What a game's dialogue screen shows ([`PreviewScreen::showing`]).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Showing {
    /// The id of the line in the text box, `None` once the scene is over.
    pub line: Option<LineId>,
    /// How that line is said ([`DialoguePlayer::direction`]).
    pub direction: Option<String>,
}

impl Showing {
    /// The line `line`, with no direction.
    pub fn new(line: Option<LineId>) -> Self {
        Self {
            line,
            direction: None,
        }
    }

    /// What `player` shows.
    pub fn of<H: DialogueHooks>(player: &DialoguePlayer<H>) -> Self {
        Self {
            line: player.line().cloned(),
            direction: player.direction().map(ToOwned::to_owned),
        }
    }
}

/// What the preview asks the game to play ([`PreviewGame::play`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Play<'a, D> {
    /// The scene, as written (with its `@if` blocks).
    pub scene: &'a Scene<D>,
    /// Who is there.
    pub present: &'a Present,
    /// The text box to start at: 0 is the first.
    pub text_box: usize,
    /// The game's own switches ([`PreviewGame::switches`]), on or off, in
    /// its order.
    pub switches: &'a [bool],
}

impl<'a, D> Play<'a, D> {
    /// `scene` for those `present`, from its first text box, with no
    /// switches.
    pub fn new(scene: &'a Scene<D>, present: &'a Present) -> Self {
        Self {
            scene,
            present,
            text_box: 0,
            switches: &[],
        }
    }

    /// Whether the game's switch `index` is on (`false` for one it hasn't).
    pub fn switch(&self, index: usize) -> bool {
        self.switches.get(index).copied().unwrap_or(false)
    }
}

impl<D: Directive> Play<'_, D> {
    /// The player of the scene, started at the text box
    /// ([`DialoguePlayer::from_text_box`]), each reply choice on the way
    /// taking its first reply, as the Lines tab lists them.
    pub fn player<H: DialogueHooks<Directive = D>>(&self, hooks: H) -> DialoguePlayer<H> {
        DialoguePlayer::from_text_box(self.scene, hooks, self.present, self.text_box, &[])
    }
}

/// One file's scenes: the last that parsed.
#[derive(Debug, Clone)]
struct FileScenes<D> {
    /// Its path in the folder.
    file: String,
    /// Its scenes, the last version that parsed.
    scenes: Vec<ParsedScene<D>>,
    /// Whether its last version didn't parse.
    broken: bool,
}

/// The scene picked: its file and id.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Picked {
    file: String,
    scene: String,
}

/// The dialogue preview screen, over the files of a [`DialogueFolder`]
/// module and the game's [`PreviewGame`]. A game's preview program makes it
/// its root screen.
pub struct DialoguePreview<G: GameData, P: PreviewGame<G>> {
    /// What the game hands in.
    game: P,
    /// The files read, in order.
    files: Vec<FileScenes<P::Directive>>,
    /// Every problem, by file and line.
    problems: Vec<ContentError>,
    /// The version of the files read ([`DialogueFiles::version`]).
    read: Option<u64>,
    /// Whether the tabs show (else the scene plays).
    open: bool,
    /// The tab shown.
    tab: Tab,
    /// The focused row of each tab, in [`Tab::ALL`] order.
    focus: [usize; 4],
    /// The characters switched off.
    gone: BTreeSet<CharacterId>,
    /// The game's switches: their names, and whether each is on.
    switches: Vec<(String, bool)>,
    /// Whether the strip shows over the scene.
    strip: bool,
    /// The scene picked, if one is.
    picked: Option<Picked>,
    /// The game's screen playing it, until it is over.
    playing: Option<Box<dyn PreviewScreen<G>>>,
    /// The text box it is at (the last it was at, once it is over).
    text_box: usize,
    /// The line box's digits, while it is open.
    go_to: Option<String>,
}

impl<G: GameData, P: PreviewGame<G>> fmt::Debug for DialoguePreview<G, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DialoguePreview")
            .field("open", &self.open)
            .field("tab", &self.tab)
            .field("picked", &self.picked)
            .field("text_box", &self.text_box)
            .finish_non_exhaustive()
    }
}

impl<G: GameData, P: PreviewGame<G>> DialoguePreview<G, P> {
    /// The preview of `game`, on the Scenes tab, with the files of the
    /// [`DialogueFolder`] module on `ctx` (none without it), the strip
    /// shown.
    pub fn new(ctx: &Ctx<G>, game: P) -> Self {
        let switches = game.switches(ctx).into_iter().map(|s| (s, false));
        let mut preview = Self {
            game,
            files: Vec::new(),
            problems: Vec::new(),
            read: None,
            open: true,
            tab: Tab::Scenes,
            focus: [0; 4],
            gone: BTreeSet::new(),
            switches: switches.collect(),
            strip: true,
            picked: None,
            playing: None,
            text_box: 0,
            go_to: None,
        };
        preview.take_files(ctx);
        preview
    }

    /// The game's preview.
    pub fn game(&self) -> &P {
        &self.game
    }

    /// The game's screen playing the scene, if one plays.
    pub fn playing(&self) -> Option<&dyn PreviewScreen<G>> {
        self.playing.as_deref()
    }

    /// Reads the files again if the module's version moved on, and plays
    /// the scene again if it changed.
    fn read_files(&mut self, ctx: &mut Ctx<G>) {
        let before = self.picked_scene().cloned();
        if !self.take_files(ctx) {
            return;
        }
        let Some(before) = before else { return };
        match self.picked_scene() {
            None => {
                self.picked = None;
                self.playing = None;
            }
            Some(now) if *now != before => {
                let count = now.text_boxes(&self.present(&now.scene), &[]).len();
                let from = if self.text_box < count {
                    self.text_box
                } else {
                    0
                };
                self.start(ctx, from);
            }
            Some(_) => {}
        }
    }

    /// Takes the files from the module if their version moved on: each
    /// parsed, the format's and the game's checks run. Whether it did.
    fn take_files(&mut self, ctx: &Ctx<G>) -> bool {
        let Some(files) = ctx.module_state::<DialogueFiles>() else {
            return false;
        };
        if self.read == Some(files.version()) {
            return false;
        }
        self.read = Some(files.version());
        let mut problems = Vec::new();
        let mut read = Vec::new();
        for (file, bytes) in files.files() {
            let (scenes, errors) = if let Ok(text) = core::str::from_utf8(bytes) {
                parse_dlg::<P::Directive>(file, text)
            } else {
                let error =
                    ContentError::new(file, "file is not valid UTF-8").with_code(codes::NOT_TEXT);
                (Vec::new(), vec![error])
            };
            let broken = !errors.is_empty();
            let scenes = if broken {
                // The last good version, if there was one.
                let last = self.files.iter().find(|f| f.file == file);
                last.map(|f| f.scenes.clone()).unwrap_or_default()
            } else {
                scenes
            };
            problems.extend(errors);
            read.push(FileScenes {
                file: file.to_owned(),
                scenes,
                broken,
            });
        }
        // The checks, over the files that parsed: a broken file's last
        // good scenes were checked when they were read.
        let parsed = read.iter().filter(|f| !f.broken).map(|f| DialogueFile {
            scenes: f.scenes.clone(),
        });
        let rules = self.game.rules(ctx);
        if let Err(errors) = scripts_from_parsed(parsed, &rules) {
            problems.extend(errors);
        }
        let scenes: Vec<ParsedScene<P::Directive>> = read
            .iter()
            .filter(|f| !f.broken)
            .flat_map(|f| f.scenes.iter().cloned())
            .collect();
        problems.extend(self.game.problems(ctx, &scenes));
        problems.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
        problems.dedup();
        self.files = read;
        self.problems = problems;
        self.clamp_focus();
        true
    }

    /// The scene picked, as last parsed.
    fn picked_scene(&self) -> Option<&ParsedScene<P::Directive>> {
        let picked = self.picked.as_ref()?;
        let file = self.files.iter().find(|f| f.file == picked.file)?;
        file.scenes.iter().find(|s| s.scene.id == picked.scene)
    }

    /// Who is there, for `scene`: everyone, but the characters switched
    /// off.
    fn present(&self, scene: &Scene<P::Directive>) -> Present {
        let conditions = scene.conditions();
        if conditions.iter().all(|c| !self.gone.contains(c)) {
            return Present::Everyone;
        }
        Present::only(conditions.into_iter().filter(|c| !self.gone.contains(c)))
    }

    /// Plays the scene picked from its text box `text_box`.
    fn start(&mut self, ctx: &mut Ctx<G>, text_box: usize) {
        let Some(parsed) = self.picked_scene().cloned() else {
            self.playing = None;
            return;
        };
        let present = self.present(&parsed.scene);
        let switches: Vec<bool> = self.switches.iter().map(|&(_, on)| on).collect();
        let mut play = Play::new(&parsed.scene, &present);
        play.text_box = text_box;
        play.switches = &switches;
        self.playing = Some(self.game.play(ctx, &play));
        self.text_box = text_box;
    }

    /// Notes the text box the game's screen is at.
    fn follow(&mut self) {
        let Some(line) = self.playing.as_ref().and_then(|s| s.showing().line) else {
            return;
        };
        let Some(parsed) = self.picked_scene() else {
            return;
        };
        let present = self.present(&parsed.scene);
        let boxes = parsed.text_boxes(&present, &[]);
        if let Some(i) = boxes.iter().position(|b| line_of(b.step) == Some(&line)) {
            self.text_box = i;
        }
    }

    /// The rows of the tab `tab`.
    fn rows(&self, tab: Tab) -> usize {
        match tab {
            Tab::Scenes => self.files.iter().map(|f| f.scenes.len()).sum(),
            Tab::Switches => self.condition_rows().len() + self.switches.len() + 1,
            Tab::Lines => 1 + self.text_boxes().len(),
            Tab::Problems => self.problems.len(),
        }
    }

    /// The characters of the scene picked's switches.
    fn condition_rows(&self) -> Vec<CharacterId> {
        self.picked_scene()
            .map(|p| p.scene.conditions())
            .unwrap_or_default()
    }

    /// The scene picked's text boxes as they play now: their source lines
    /// and steps.
    fn text_boxes(&self) -> Vec<(u32, Step<P::Directive>)> {
        let Some(parsed) = self.picked_scene() else {
            return Vec::new();
        };
        let present = self.present(&parsed.scene);
        let boxes = parsed.text_boxes(&present, &[]);
        boxes.iter().map(|b| (b.line, b.step.clone())).collect()
    }

    /// Keeps each tab's focus on one of its rows.
    fn clamp_focus(&mut self) {
        for (i, tab) in Tab::ALL.iter().enumerate() {
            let last = self.rows(*tab).saturating_sub(1);
            self.focus[i] = self.focus[i].min(last);
        }
    }

    /// The focused row of the tab shown.
    fn focused(&self) -> usize {
        self.focus[self.tab.index()]
    }

    /// The scene of row `row` of the Scenes tab: its file and id.
    fn scene_at(&self, row: usize) -> Option<Picked> {
        let mut scenes = self.files.iter().flat_map(|f| {
            f.scenes.iter().map(|s| Picked {
                file: f.file.clone(),
                scene: s.scene.id.clone(),
            })
        });
        scenes.nth(row)
    }

    /// One frame with the tabs shown.
    fn tabs(&mut self, ctx: &mut Ctx<G>, input: &FrameInput<G::Action>) {
        if self.go_to.is_some() {
            self.type_line(ctx, input);
            return;
        }
        for &action in &input.actions {
            match action.ui() {
                Some(UiAction::Left) => self.turn(ctx, false),
                Some(UiAction::Right) => self.turn(ctx, true),
                Some(UiAction::Up) => self.step_focus(ctx, false),
                Some(UiAction::Down) => self.step_focus(ctx, true),
                Some(UiAction::Confirm) => self.confirm(ctx),
                Some(UiAction::Cancel) if self.playing.is_some() => {
                    self.open = false;
                    ctx.audio.menu(MenuSound::Cancel);
                }
                _ => {}
            }
            if !self.open || self.go_to.is_some() {
                return;
            }
        }
    }

    /// Shows the next tab (`forward`) or the one before, going round.
    fn turn(&mut self, ctx: &mut Ctx<G>, forward: bool) {
        let count = Tab::ALL.len();
        let i = self.tab.index();
        let to = if forward { i + 1 } else { i + count - 1 };
        self.tab = Tab::ALL[to % count];
        ctx.audio.menu(MenuSound::Move);
    }

    /// Moves the focus one row down (`forward`) or up, stopping at the
    /// ends.
    fn step_focus(&mut self, ctx: &mut Ctx<G>, forward: bool) {
        let rows = self.rows(self.tab);
        let at = &mut self.focus[self.tab.index()];
        let to = if forward {
            (*at + 1).min(rows.saturating_sub(1))
        } else {
            at.saturating_sub(1)
        };
        if to != *at {
            *at = to;
            ctx.audio.menu(MenuSound::Move);
        }
    }

    /// Confirm on the focused row.
    fn confirm(&mut self, ctx: &mut Ctx<G>) {
        let row = self.focused();
        match self.tab {
            Tab::Scenes => {
                let Some(picked) = self.scene_at(row) else {
                    return ctx.audio.menu(MenuSound::Denied);
                };
                self.picked = Some(picked);
                self.focus[Tab::Lines.index()] = 0;
                self.play_from(ctx, 0);
            }
            Tab::Switches => self.switch(ctx, row),
            Tab::Lines if row == 0 => {
                if self.picked_scene().is_none() {
                    return ctx.audio.menu(MenuSound::Denied);
                }
                self.go_to = Some(String::new());
                ctx.audio.menu(MenuSound::Select);
            }
            Tab::Lines => self.play_from(ctx, row - 1),
            Tab::Problems => ctx.audio.menu(MenuSound::Denied),
        }
    }

    /// Plays the scene picked from text box `text_box`, the tabs closed.
    fn play_from(&mut self, ctx: &mut Ctx<G>, text_box: usize) {
        self.start(ctx, text_box);
        if self.playing.is_some() {
            self.open = false;
            ctx.audio.menu(MenuSound::Select);
        } else {
            ctx.audio.menu(MenuSound::Denied);
        }
    }

    /// Changes switch `row`: a character's, the game's, or the strip's.
    /// The scene plays again from its text box.
    fn switch(&mut self, ctx: &mut Ctx<G>, row: usize) {
        let characters = self.condition_rows();
        if let Some(character) = characters.get(row) {
            if !self.gone.remove(character) {
                self.gone.insert(character.clone());
            }
        } else if let Some((_, on)) = self.switches.get_mut(row - characters.len()) {
            *on = !*on;
        } else {
            self.strip = !self.strip;
            ctx.audio.menu(MenuSound::Select);
            return;
        }
        ctx.audio.menu(MenuSound::Select);
        if self.picked.is_some() {
            let count = self.text_boxes().len();
            let from = if self.text_box < count {
                self.text_box
            } else {
                0
            };
            self.start(ctx, from);
        }
        self.clamp_focus();
    }

    /// One frame of the line box: typed digits, the text box's fixed keys,
    /// and the cursor stepping the number.
    fn type_line(&mut self, ctx: &mut Ctx<G>, input: &FrameInput<G::Action>) {
        for &action in &input.actions {
            match action.ui() {
                Some(UiAction::Up | UiAction::Right) => self.step_line(ctx, true),
                Some(UiAction::Down | UiAction::Left) => self.step_line(ctx, false),
                Some(UiAction::Confirm) => return self.jump(ctx),
                Some(UiAction::Cancel) => return self.close_box(ctx),
                _ => {}
            }
        }
        for &c in input.text() {
            let Some(digits) = &mut self.go_to else {
                return;
            };
            if c.is_ascii_digit() && digits.len() < MAX_DIGITS {
                digits.push(c);
            } else {
                ctx.audio.menu(MenuSound::Denied);
            }
        }
        for &chord in input.pressed_chords() {
            match text_key(chord) {
                Some(TextKey::Done) => return self.jump(ctx),
                Some(TextKey::Cancel) => return self.close_box(ctx),
                Some(TextKey::Delete) if self.go_to.as_mut().and_then(String::pop).is_some() => {
                    ctx.audio.menu(MenuSound::Cancel);
                }
                Some(TextKey::Delete) | None => {}
            }
        }
    }

    /// The line box's number one up (`up`) or down, from 1.
    fn step_line(&mut self, ctx: &mut Ctx<G>, up: bool) {
        let Some(digits) = &mut self.go_to else {
            return;
        };
        let now: u32 = digits.parse().unwrap_or(0);
        let to = if up {
            now.saturating_add(1).min(999_999)
        } else {
            now.saturating_sub(1).max(1)
        };
        *digits = to.to_string();
        ctx.audio.menu(MenuSound::Move);
    }

    /// Closes the line box.
    fn close_box(&mut self, ctx: &mut Ctx<G>) {
        self.go_to = None;
        ctx.audio.menu(MenuSound::Cancel);
    }

    /// Plays the scene from the text box the line box's line falls in.
    fn jump(&mut self, ctx: &mut Ctx<G>) {
        let line = self.go_to.take().and_then(|d| d.parse::<u32>().ok());
        let found = line.and_then(|line| {
            let parsed = self.picked_scene()?;
            parsed.text_box_at(line, &self.present(&parsed.scene), &[])
        });
        match found {
            Some(text_box) => {
                self.focus[Tab::Lines.index()] = text_box + 1;
                self.play_from(ctx, text_box);
            }
            None => ctx.audio.menu(MenuSound::Denied),
        }
    }

    /// One frame of the scene playing: the game's screen takes the input,
    /// but Cancel, which shows the tabs.
    fn play_frame(&mut self, ctx: &mut Ctx<G>, input: &FrameInput<G::Action>) -> Transition<G> {
        let cancel = |a: &G::Action| a.ui() == Some(UiAction::Cancel);
        if input.actions.iter().any(cancel) {
            self.open = true;
            ctx.audio.menu(MenuSound::Select);
            return Transition::None;
        }
        let Some(screen) = &mut self.playing else {
            return Transition::None;
        };
        let declared = screen.actions().unwrap_or_else(|| G::Action::ALL.to_vec());
        let declared: Vec<G::Action> = declared.into_iter().filter(|a| !cancel(a)).collect();
        let (input, _) = input.declared(screen.name(), &declared);
        match screen.update(ctx, &input) {
            Transition::Pop | Transition::PopWith(_) => {
                self.playing = None;
                self.open = true;
                self.tab = Tab::Lines;
            }
            Transition::Quit => return Transition::Quit,
            // A screen it would open over the scene, or put in its place,
            // isn't the scene's.
            Transition::None | Transition::Push(_) | Transition::Replace(_) => {}
        }
        self.follow();
        Transition::None
    }

    /// What the screen shows.
    pub fn view(&self, ctx: &Ctx<G>) -> PreviewView {
        let tabs = Tab::ALL.iter().map(|t| ctx.text(t.key()).to_owned());
        let mut view = PreviewView::new(self.tab, tabs.collect());
        view.open = self.open;
        view.focus = self.focused();
        view.scenes = self.scene_rows();
        view.switches = self.switch_rows(ctx);
        let mut go_to = GoToView::new(ctx.text(GO_TO).to_owned());
        go_to.digits = self.go_to.clone().unwrap_or_default();
        go_to.typing = self.go_to.is_some();
        view.go_to = go_to;
        view.lines = self.line_rows();
        view.problems = self.problems.iter().map(problem_row).collect();
        view.playing = self.playing_view();
        view.strip = self.strip_view(ctx);
        view.note = self.note(ctx);
        let help = if self.go_to.is_some() {
            HELP_GO_TO
        } else {
            HELP
        };
        view.help = ctx.text_with(help, &[]);
        view
    }

    /// The Scenes tab's rows.
    fn scene_rows(&self) -> Vec<SceneRow> {
        let mut rows = Vec::new();
        for f in &self.files {
            let problems = self.problems.iter().any(|p| p.file == f.file);
            for s in &f.scenes {
                let mut row = SceneRow::new(f.file.clone(), s.scene.id.clone());
                row.problems = problems;
                row.picked = self
                    .picked
                    .as_ref()
                    .is_some_and(|p| p.file == f.file && p.scene == s.scene.id);
                rows.push(row);
            }
        }
        rows
    }

    /// The Switches tab's rows.
    fn switch_rows(&self, ctx: &Ctx<G>) -> Vec<SwitchRow> {
        let text = |key: &str| ctx.text(key).to_owned();
        let mut rows = Vec::new();
        for character in self.condition_rows() {
            let on = !self.gone.contains(&character);
            let value = text(if on { HERE } else { GONE });
            rows.push(SwitchRow::new(character.0.clone(), value, on));
        }
        for (name, on) in &self.switches {
            let value = text(if *on { ON } else { OFF });
            rows.push(SwitchRow::new(name.clone(), value, *on));
        }
        let value = text(if self.strip { ON } else { OFF });
        rows.push(SwitchRow::new(text(STRIP_SWITCH), value, self.strip));
        rows
    }

    /// The Lines tab's rows, after its line box.
    fn line_rows(&self) -> Vec<LineRow> {
        let boxes = self.text_boxes();
        let playing = self.playing.is_some();
        let mut rows = Vec::new();
        for (i, (line, step)) in boxes.iter().enumerate() {
            let words = step.text().unwrap_or_default().to_owned();
            let mut row = LineRow::new(*line, words);
            if let Step::Say { speaker, .. } = step {
                row.speaker = Some(speaker.0.clone());
            }
            row.current = playing && i == self.text_box;
            rows.push(row);
        }
        rows
    }

    /// What plays, if a scene is picked.
    fn playing_view(&self) -> Option<PlayingView> {
        let picked = self.picked.as_ref()?;
        let mut view = PlayingView::new(picked.file.clone(), picked.scene.clone());
        view.text_box = self.text_box;
        view.finished = self.playing.is_none();
        Some(view)
    }

    /// The strip over the scene, while it plays with the tabs closed and
    /// the strip switched on.
    fn strip_view(&self, ctx: &Ctx<G>) -> Option<StripView> {
        if self.open || !self.strip {
            return None;
        }
        let picked = self.picked.as_ref()?;
        let showing = self.playing.as_ref()?.showing();
        let line = self.text_boxes().get(self.text_box).map_or(0, |b| b.0);
        let mut strip = StripView::new(picked.file.clone(), line);
        strip.direction = showing.direction;
        strip.help = ctx.text_with(HELP_PLAYING, &[]);
        Some(strip)
    }

    /// The note for the tab shown, if it has nothing to list.
    fn note(&self, ctx: &Ctx<G>) -> Option<String> {
        let key = match self.tab {
            Tab::Scenes if self.files.iter().all(|f| f.scenes.is_empty()) => {
                let drops = ctx.platform.can_drop_files() && ctx.game.lang().has(NO_SCENES_DROP);
                if drops { NO_SCENES_DROP } else { NO_SCENES }
            }
            Tab::Switches | Tab::Lines if self.picked.is_none() => NO_SCENE_PICKED,
            Tab::Lines if self.playing.is_none() => FINISHED,
            Tab::Problems if self.problems.is_empty() => NO_PROBLEMS,
            _ => return None,
        };
        Some(ctx.text(key).to_owned())
    }
}

/// The id of the line a text box's step shows.
fn line_of<D>(step: &Step<D>) -> Option<&LineId> {
    match step {
        Step::Say { line, .. } | Step::Narrate { line, .. } => Some(line),
        _ => None,
    }
}

/// A problem's row: its message, and the fix after its first `; ` (the
/// checks' messages say what is wrong, then what to do).
fn problem_row(problem: &ContentError) -> ProblemRow {
    let (message, fix) = match problem.message.split_once("; ") {
        Some((message, fix)) => (message.to_owned(), Some(fix.to_owned())),
        None => (problem.message.clone(), None),
    };
    let mut row = ProblemRow::new(problem.file.clone(), message);
    row.line = problem.line;
    row.fix = fix;
    row
}

impl<G: GameData, P: PreviewGame<G>> Screen<G> for DialoguePreview<G, P> {
    fn name(&self) -> &'static str {
        "dialogue_preview"
    }

    /// The cursor, Confirm and Cancel, and the actions of the game's
    /// screen playing the scene.
    fn actions(&self) -> Option<Vec<G::Action>> {
        let mut actions = ui_actions(&[
            UiAction::Up,
            UiAction::Down,
            UiAction::Left,
            UiAction::Right,
            UiAction::Confirm,
            UiAction::Cancel,
        ]);
        if let Some(screen) = &self.playing {
            let theirs = screen.actions().unwrap_or_else(|| G::Action::ALL.to_vec());
            for action in theirs {
                if !actions.contains(&action) {
                    actions.push(action);
                }
            }
        }
        Some(actions)
    }

    fn update(&mut self, ctx: &mut Ctx<G>, input: &FrameInput<G::Action>) -> Transition<G> {
        self.read_files(ctx);
        if !self.open && self.playing.is_some() {
            return self.play_frame(ctx, input);
        }
        self.open = true;
        self.tabs(ctx, input);
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<G>, buf: &mut GlyphBuffer) {
        let view = self.view(ctx);
        glyph::paint_back(ctx, buf);
        if let Some(screen) = self.playing.as_ref().filter(|_| !view.open) {
            screen.draw(ctx, buf);
        }
        glyph::paint(ctx, &view, buf);
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }
}

#[cfg(test)]
mod tests;
