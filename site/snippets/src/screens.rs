//! Chapter 2, a game and its screens: the title, the garden and the
//! question asked over it, each a [`Screen`] on the stack, and the long
//! form of a game's data for a game that outgrows the short path.

use core::any::Any;

use shuyi::assets::audio::AudioManifest;
use shuyi::assets::keymap::KeymapDef;
use shuyi::assets::lang::Lang;
use shuyi::assets::palette::PaletteDef;
use shuyi::core::TurnRules;
use shuyi::prelude::*;

use crate::Data;
use crate::assets::Plan;
use crate::frame::paint_garden;
use crate::rules::{Command, Garden};

/// The seed of the garden's luck. A game takes one from its start
/// options, or from the clock in its binary; a test gives its own.
pub const SEED: u64 = 7;

// ANCHOR: title
/// What the title shows: plain data, built from the game's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleView {
    /// The game's name.
    pub title: String,
    /// The line under it.
    pub subtitle: String,
    /// The help line, naming the player's keys.
    pub help: String,
}

/// The title, the game's first screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Title;

impl Title {
    /// What it shows.
    pub fn view(ctx: &Ctx<Data>) -> TitleView {
        TitleView {
            title: ctx.text("title.name").to_owned(),
            subtitle: ctx.text("title.subtitle").to_owned(),
            help: ctx.text_with("title.help", &[]),
        }
    }
}

impl Screen<Data> for Title {
    // check-text: not player text, the screen's id
    fn name(&self) -> &'static str {
        "title"
    }

    /// Confirm starts the garden, and Cancel leaves the game.
    fn actions(&self) -> Option<Vec<UiAction>> {
        Some(vec![UiAction::Confirm, UiAction::Cancel])
    }

    fn update(&mut self, ctx: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        if input.actions.contains(&UiAction::Cancel) {
            return Transition::Quit;
        }
        if input.actions.contains(&UiAction::Confirm) {
            return Transition::Push(Box::new(GardenScreen::new(&ctx.game.own)));
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        let c = |color| ctx.palette.engine(color);
        let bg = c(EngineColor::Background);
        buf.fill_rect(buf.bounds(), Cell::new(' ', bg, bg));
        let view = Self::view(ctx);
        print_centred(buf, 9, &view.title, c(EngineColor::Highlight), bg);
        print_centred(buf, 11, &view.subtitle, c(EngineColor::TextDim), bg);
        let last = i32::from(buf.height()) - 1;
        print_centred(buf, last, &view.help, c(EngineColor::TextDim), bg);
    }
}
// ANCHOR_END: title

// ANCHOR: garden
/// The garden: the player walks along the plots and plants seeds. Its
/// state is the rules' ([`Garden`]), changed only by commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GardenScreen {
    /// The rules' state.
    pub(crate) garden: Garden,
}

impl GardenScreen {
    /// A new garden laid out by the game's plan.
    pub fn new(plan: &Plan) -> Self {
        Self {
            garden: Garden::new(plan, SEED),
        }
    }
}

impl Screen<Data> for GardenScreen {
    // check-text: not player text, the screen's id
    fn name(&self) -> &'static str {
        "garden"
    }

    fn actions(&self) -> Option<Vec<UiAction>> {
        Some(vec![
            UiAction::Left,
            UiAction::Right,
            UiAction::Confirm,
            UiAction::Cancel,
        ])
    }

    fn update(&mut self, _: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        for &action in &input.actions {
            let command = match action {
                UiAction::Left => Command::Left,
                UiAction::Right => Command::Right,
                UiAction::Confirm if self.garden.outcome().is_some() => return Transition::Pop,
                UiAction::Confirm => Command::Plant,
                UiAction::Cancel => return Transition::Push(Box::new(Leave)),
                _ => continue,
            };
            // A refused command changes nothing, so there is nothing to undo.
            let _ = self.garden.apply(&command);
        }
        Transition::None
    }

    /// The question pops with the player's answer: yes leaves the garden.
    fn uncovered(&mut self, _: &mut Ctx<Data>, result: Option<ScreenResult>) -> Transition<Data> {
        if result.and_then(ScreenResult::take::<bool>) == Some(true) {
            Transition::Pop
        } else {
            Transition::None
        }
    }

    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        paint_garden(&self.view(ctx), ctx, buf);
    }

    /// So a test finds it on the stack.
    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}
// ANCHOR_END: garden

// ANCHOR: leave
/// "Leave the garden?", asked over it: it pops with the answer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Leave;

impl Screen<Data> for Leave {
    // check-text: not player text, the screen's id
    fn name(&self) -> &'static str {
        "leave"
    }

    /// The garden shows through.
    fn is_overlay(&self) -> bool {
        true
    }

    fn actions(&self) -> Option<Vec<UiAction>> {
        Some(vec![UiAction::Confirm, UiAction::Cancel])
    }

    fn update(&mut self, _: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        for &action in &input.actions {
            match action {
                UiAction::Confirm => return Transition::pop_with(true),
                UiAction::Cancel => return Transition::pop_with(false),
                _ => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        let c = |color| ctx.palette.engine(color);
        let panel = c(EngineColor::Panel);
        let area = Rect::new(8, 9, i32::from(buf.width()) - 16, 5);
        buf.fill_rect(area, Cell::new(' ', panel, panel));
        buf.draw_box(area, BoxStyle::Single, c(EngineColor::Border), panel);
        let question = ctx.text("leave.question");
        print_centred(buf, 10, question, c(EngineColor::Text), panel);
        let help = ctx.text_with("leave.help", &[]);
        print_centred(buf, 12, &help, c(EngineColor::TextDim), panel);
    }
}
// ANCHOR_END: leave

// ANCHOR: game_data
/// A game's data written by hand, for a game whose data outgrows
/// `BasicData`: its own sounds, voices, themes or settings. Each method
/// hands the engine a part it reads; the rest have defaults.
#[derive(Debug)]
pub struct OwnData {
    /// The text by key, in every language the game has.
    pub lang: Lang,
    /// The keymap file, over the game's actions.
    pub keymap: KeymapDef<UiAction>,
    /// The sounds and the music.
    pub audio: AudioManifest,
    /// The colors by name.
    pub palette: PaletteDef,
}

impl GameData for OwnData {
    type Action = UiAction;
    type Colors = EngineColor;
    type Voice = ();
    type Settings = ();

    fn lang(&self) -> &Lang {
        &self.lang
    }

    fn keymap_def(&self) -> &KeymapDef<UiAction> {
        &self.keymap
    }

    fn audio_manifest(&self) -> &AudioManifest {
        &self.audio
    }

    fn palette_def(&self) -> &PaletteDef {
        &self.palette
    }

    fn console(&self) -> ConsoleSpec {
        ConsoleSpec::text(48, 24)
    }
}
// ANCHOR_END: game_data

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use shuyi::assets::lang::English;

    use super::*;
    use crate::assets::formats;
    use crate::game;

    // ANCHOR: harness
    #[test]
    fn confirm_opens_the_garden_and_leaving_asks_first() {
        let mut h = game().harness();
        h.keys("Enter");
        assert_eq!(h.screens(), ["title", "garden"]);
        h.keys("Backspace");
        assert_eq!(h.screens(), ["title", "garden", "leave"]);
        // No: the question pops, and the garden stays.
        h.keys("Backspace");
        assert_eq!(h.screens(), ["title", "garden"]);
        // Yes: the garden pops with it.
        h.keys("Backspace Enter");
        assert_eq!(h.screens(), ["title"]);
    }
    // ANCHOR_END: harness

    #[test]
    fn the_long_form_reads_the_same_files() {
        let bundle = game().bundle();
        let formats = formats();
        let data = OwnData {
            lang: Lang::new(
                formats.load::<English>(&bundle, "en.ron").unwrap().0,
                BTreeMap::new(),
            ),
            keymap: formats.load(&bundle, "keymap.ron").unwrap(),
            audio: AudioManifest::default(),
            palette: formats.load(&bundle, "palette.ron").unwrap(),
        };
        let ctx = Ctx::new(data).unwrap();
        assert_eq!(ctx.text("title.name"), "Moon Garden");
    }
}
