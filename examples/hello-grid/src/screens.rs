//! The screens: the [`Title`], the scene before the stage ([`Talk`]), the
//! stage itself ([`stage::StageScreen`]), the question whether to leave it
//! ([`Leave`]) and how it ended ([`End`]), with the engine's options, Key
//! bindings and profiles screens opened from the title.
//! Each says what it shows as a view (plain data) and [`crate::skin`]
//! paints it.

pub mod stage;

use std::any::Any;

use shuyi_assets::dialogue::{Present, Side as Speaker};
use shuyi_ui::dialogue::{DialoguePlayer, Plain, Step, TextBox, TextView, WaitView};
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::screens::key_bindings::{KeyBindingsScreen, KeyRow, RowGroup};
use shuyi_ui::screens::options::{self, OptionRow, OptionsScreen, RowAction, RowValue};
use shuyi_ui::screens::profiles::ProfilesScreen;
use shuyi_ui::widgets::{Menu, MenuEvent, MenuItem};
use shuyi_ui::{Ctx, FrameInput, Screen, Transition};

pub use stage::StageScreen;

use crate::rules::Outcome;
use crate::settings::MapLook;
use crate::skin::{self, EndView, QuestionView, TitleView};
use crate::{Action, Hello};

/// The game's context.
pub type HelloCtx = Ctx<Hello>;

/// The Key bindings screen's rows: every action, each must keep a key.
pub const KEY_ROWS: [KeyRow<Action>; 6] = [
    (Action::Up, "key_bindings.action.up", RowGroup::Required),
    (Action::Down, "key_bindings.action.down", RowGroup::Required),
    (Action::Left, "key_bindings.action.left", RowGroup::Required),
    (
        Action::Right,
        "key_bindings.action.right",
        RowGroup::Required,
    ),
    (
        Action::Confirm,
        "key_bindings.action.confirm",
        RowGroup::Required,
    ),
    (
        Action::Cancel,
        "key_bindings.action.cancel",
        RowGroup::Required,
    ),
];

/// The options: the engine's text speed and the game's map look (the two
/// settings), the Key bindings screen, and Restore defaults.
pub fn option_rows(_: &HelloCtx) -> Vec<OptionRow<Hello>> {
    vec![
        OptionRow::text_speed(),
        OptionRow::new(
            "options.row.look",
            RowValue::Choice {
                values: |_| MapLook::ALL.iter().map(|&(_, key)| key).collect(),
                get: |ctx| ctx.settings().game.look.index(),
                set: |ctx, i| {
                    let look = MapLook::ALL.get(i).map_or(MapLook::Glyphs, |&(l, _)| l);
                    ctx.preview_settings(|s| s.game.look = look)
                },
            },
        ),
        OptionRow::new(
            "options.row.key_bindings",
            RowValue::Action {
                value: |_| None,
                confirm: |ctx| RowAction::Open(Box::new(KeyBindingsScreen::new(ctx, &KEY_ROWS))),
                help: |_| options::HELP_OPEN,
            },
        )
        .starting_group(),
        OptionRow::restore_defaults().starting_group(),
    ]
}

/// What an item of the title's menu stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleChoice {
    /// Starts a stage.
    Play,
    /// Opens the options.
    Options,
    /// Opens the player profiles.
    Profiles,
    /// Quits.
    Quit,
}

/// The title menu's items, by their text keys, top to bottom.
pub const TITLE_ITEMS: [(&str, TitleChoice); 4] = [
    ("title.play", TitleChoice::Play),
    ("title.options", TitleChoice::Options),
    ("title.profiles", TitleChoice::Profiles),
    ("title.quit", TitleChoice::Quit),
];

/// The title: the game's name and a menu of Play, Options, Profiles and
/// Quit.
#[derive(Debug, Default)]
pub struct Title {
    /// Which item has the focus.
    pub focus: usize,
}

impl Title {
    /// The title, on Play.
    pub fn new() -> Self {
        Self::default()
    }

    /// The menu: its labels are text keys, put in the player's language
    /// when the skin paints them.
    fn menu(&self) -> Menu<TitleChoice> {
        let items = TITLE_ITEMS
            .iter()
            .map(|&(key, choice)| MenuItem::keyed(key).with_value(choice));
        Menu::new(items.collect())
            .without_cancel()
            .focused(self.focus)
    }

    /// What it shows.
    pub fn view(&self, ctx: &HelloCtx) -> TitleView {
        TitleView {
            name: ctx.text("title.name").to_owned(),
            subtitle: ctx.text("title.subtitle").to_owned(),
            menu: self.menu().view(),
            help: ctx.text_with("title.help", &[]),
        }
    }
}

impl Screen<Hello> for Title {
    fn name(&self) -> &'static str {
        "title"
    }

    fn update(&mut self, ctx: &mut HelloCtx, input: &FrameInput<Action>) -> Transition<Hello> {
        if ctx.layout().is_none() {
            // A profile just switched to or added has no layout of its
            // own: the game's only one (until ticket 2401).
            ctx.use_layout(crate::LAYOUT);
        }
        let mut menu = self.menu();
        for &action in &input.actions {
            let event = menu.handle_with_sound(action, &mut ctx.audio);
            self.focus = menu.focus();
            match event {
                Some(MenuEvent::Chosen(TitleChoice::Play)) => {
                    return Transition::Push(Box::new(StageScreen::new(ctx)));
                }
                Some(MenuEvent::Chosen(TitleChoice::Options)) => {
                    return Transition::Push(Box::new(OptionsScreen::new(option_rows)));
                }
                Some(MenuEvent::Chosen(TitleChoice::Profiles)) => {
                    return Transition::Push(Box::new(ProfilesScreen::new(ctx)));
                }
                Some(MenuEvent::Chosen(TitleChoice::Quit)) => return Transition::Quit,
                Some(MenuEvent::Cancelled) | None => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &HelloCtx, buf: &mut GlyphBuffer) {
        skin::paint_title(ctx, &self.view(ctx), buf);
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}

/// A scene read in the engine's text box, over the screen below: Confirm
/// shows the whole page, then the next; holding it types faster; Cancel
/// skips the rest. Closes when the scene ends.
#[derive(Debug)]
pub struct Talk {
    /// The scene playing.
    pub player: DialoguePlayer<Plain>,
    /// Its text box.
    pub text: TextBox,
}

impl Talk {
    /// Characters a line of the box holds.
    const WIDTH: usize = 42;
    /// Lines a page holds.
    const LINES: usize = 3;

    /// The game's scene `id` (an empty one if it has none).
    pub fn new(ctx: &HelloCtx, id: &str) -> Self {
        let scene = ctx.game.dialogue.get(id).cloned().unwrap_or_default();
        let player = DialoguePlayer::new(&scene, Plain, &Present::Everyone);
        let text = Self::text_of(&player);
        Self { player, text }
    }

    fn text_of(player: &DialoguePlayer<Plain>) -> TextBox {
        let text = player.current().text.unwrap_or_default();
        TextBox::new(&text, Self::WIDTH, Self::LINES)
    }

    /// What its text box shows.
    pub fn view(&self, ctx: &HelloCtx) -> TextView {
        let waiting = self.text.is_revealed().then(|| WaitView {
            key: ctx.text_with("talk.next", &[]),
            log_hint: None,
            marker: self.text.marker_lit(),
        });
        TextView {
            narration: self.player.current().narration,
            lines: self.text.page_lines().to_vec(),
            shown: self.text.shown(),
            choices: None,
            skip: None,
            waiting,
        }
    }

    /// The name of whoever speaks now, if anyone does.
    pub fn speaker(&self, ctx: &HelloCtx) -> Option<String> {
        let view = self.player.current();
        let speaking = match view.speaker {
            Some(Speaker::Left) => view.left,
            Some(Speaker::Right) => view.right,
            None => None,
        };
        speaking.map(|p| ctx.text(&format!("character.{}", p.character.0)).to_owned())
    }
}

impl Screen<Hello> for Talk {
    fn name(&self) -> &'static str {
        "talk"
    }

    fn update(&mut self, ctx: &mut HelloCtx, input: &FrameInput<Action>) -> Transition<Hello> {
        for &action in &input.actions {
            match action {
                Action::Cancel => return Transition::Pop,
                // The last page read: the next line.
                Action::Confirm if self.text.confirm() == Step::Done => {
                    self.player.advance();
                    if self.player.is_finished() {
                        return Transition::Pop;
                    }
                    self.text = Self::text_of(&self.player);
                }
                _ => {}
            }
        }
        if self.player.is_finished() {
            return Transition::Pop;
        }
        self.text.hold(input.is_held(Action::Confirm), input.dt());
        self.text
            .type_on(input.dt(), ctx.settings().text_speed.chars_per_s());
        Transition::None
    }

    fn draw(&self, ctx: &HelloCtx, buf: &mut GlyphBuffer) {
        let view = self.view(ctx);
        skin::paint_talk(ctx, &view, self.speaker(ctx).as_deref(), buf);
    }

    fn is_overlay(&self) -> bool {
        true
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}

/// Whether to leave the stage, asked over it on the bottom row: Confirm
/// says yes and Cancel no. It pops with the answer, a `bool`, which the
/// stage reads when it is uncovered.
#[derive(Debug, Clone, Copy, Default)]
pub struct Leave;

impl Leave {
    /// What it shows.
    pub fn view(ctx: &HelloCtx) -> QuestionView {
        QuestionView {
            question: ctx.text("stage.leave").to_owned(),
            answers: ctx.text_with("stage.leave.answers", &[]),
        }
    }
}

impl Screen<Hello> for Leave {
    fn name(&self) -> &'static str {
        "leave"
    }

    fn update(&mut self, _: &mut HelloCtx, input: &FrameInput<Action>) -> Transition<Hello> {
        for &action in &input.actions {
            match action {
                Action::Confirm => return Transition::pop_with(true),
                Action::Cancel => return Transition::pop_with(false),
                _ => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &HelloCtx, buf: &mut GlyphBuffer) {
        skin::paint_question(&ctx.palette, &Self::view(ctx), buf);
    }

    fn is_overlay(&self) -> bool {
        true
    }
}

/// The end of a stage: how it ended, and a menu of Play again and Back to
/// the title.
#[derive(Debug)]
pub struct End {
    /// How the stage ended.
    pub outcome: Outcome,
    /// Which item has the focus.
    pub focus: usize,
}

/// What an item of the end's menu stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndChoice {
    /// Plays the stage again.
    Again,
    /// Goes back to the title.
    Title,
}

/// The end menu's items, by their text keys, top to bottom.
pub const END_ITEMS: [(&str, EndChoice); 2] = [
    ("end.again", EndChoice::Again),
    ("end.title", EndChoice::Title),
];

impl End {
    /// The end of a stage that ended so, on Play again.
    pub fn new(outcome: Outcome) -> Self {
        Self { outcome, focus: 0 }
    }

    /// The menu, of text keys like the title's.
    fn menu(&self) -> Menu<EndChoice> {
        let items = END_ITEMS
            .iter()
            .map(|&(key, choice)| MenuItem::keyed(key).with_value(choice));
        Menu::new(items.collect())
            .without_cancel()
            .focused(self.focus)
    }

    /// What it shows.
    pub fn view(&self, ctx: &HelloCtx) -> EndView {
        let (key, turn, won) = match self.outcome {
            Outcome::Won { turn } => ("end.won", turn, true),
            Outcome::Caught { turn } => ("end.caught", turn, false),
        };
        EndView {
            headline: ctx.text_with(key, &[("turn", &turn)]),
            won,
            menu: self.menu().view(),
            help: ctx.text_with("end.help", &[]),
        }
    }
}

impl Screen<Hello> for End {
    fn name(&self) -> &'static str {
        "end"
    }

    fn update(&mut self, ctx: &mut HelloCtx, input: &FrameInput<Action>) -> Transition<Hello> {
        let mut menu = self.menu();
        for &action in &input.actions {
            let event = menu.handle_with_sound(action, &mut ctx.audio);
            self.focus = menu.focus();
            match event {
                Some(MenuEvent::Chosen(EndChoice::Again)) => {
                    return Transition::Replace(Box::new(StageScreen::new(ctx)));
                }
                Some(MenuEvent::Chosen(EndChoice::Title)) => return Transition::Pop,
                Some(MenuEvent::Cancelled) | None => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &HelloCtx, buf: &mut GlyphBuffer) {
        skin::paint_end(ctx, &self.view(ctx), buf);
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}
