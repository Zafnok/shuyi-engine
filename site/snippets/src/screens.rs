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

    /// Confirm starts the garden, and Cancel leaves the game where it may.
    fn actions(&self) -> Option<Vec<UiAction>> {
        Some(vec![UiAction::Confirm, UiAction::Cancel])
    }

    fn update(&mut self, ctx: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        if input.actions.contains(&UiAction::Cancel) && ctx.platform.can_quit() {
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
    use shuyi::ui::input::help_keys::key_name;

    use super::*;
    use crate::assets::formats;
    use crate::game;

    #[test]
    fn each_screen_declares_the_actions_it_reads() {
        use UiAction::{Cancel, Confirm, Left, Right};
        let mut ctx = game().ctx().unwrap();
        let garden = GardenScreen::new(&ctx.game.own);
        let screens: Vec<Box<dyn Screen<Data>>> =
            vec![Box::new(Title), Box::new(garden), Box::new(Leave)];
        let declared: Vec<_> = screens.iter().map(|s| s.actions()).collect();
        assert_eq!(
            declared,
            [
                Some(vec![Confirm, Cancel]),
                Some(vec![Left, Right, Confirm, Cancel]),
                Some(vec![Confirm, Cancel]),
            ]
        );
        // Confirm once the garden is over goes back to the title.
        let mut over = GardenScreen {
            garden: Garden::new(&Plan::new(1, 1, 100), 0),
        };
        let confirm = || FrameInput::new(vec![Confirm], 0.0, vec![]);
        assert!(matches!(
            over.update(&mut ctx, &confirm()),
            Transition::None
        ));
        assert!(over.garden.outcome().is_some());
        assert!(matches!(over.update(&mut ctx, &confirm()), Transition::Pop));
    }

    #[test]
    fn left_walks_back() {
        let mut h = game().harness();
        h.keys("Enter Right Right Left");
        let garden = h.screen::<GardenScreen>().unwrap();
        assert_eq!(garden.garden.at(), 1);
    }

    #[test]
    fn the_title_and_the_question_are_where_they_should_be() {
        let mut h = game().harness();
        let frame = h.snapshot();
        let lines: Vec<&str> = frame.lines().collect();
        assert_eq!(lines[9].trim(), "Moon Garden", "{frame}");
        assert_eq!(lines[11].trim(), "made with Shuyi Engine", "{frame}");
        assert_eq!(lines[23].trim(), "Enter plant · Backspace leave", "{frame}");
        h.keys("Enter Backspace");
        let frame = h.snapshot();
        let lines: Vec<&str> = frame.lines().collect();
        // The question's box, 32 cells wide from column 8, over the garden,
        // which shows below it.
        assert_eq!(lines[9].find('┌'), Some(8), "{frame}");
        assert_eq!(lines[9].trim_end().chars().count(), 40, "{frame}");
        assert_eq!(
            lines[10].trim().trim_matches('│').trim(),
            "Leave the garden?"
        );
        assert!(lines[14].contains("9 seeds left"), "{frame}");
    }

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

    // ANCHOR: as_target
    #[test]
    fn on_the_web_cancel_on_the_title_doesnt_quit() {
        let profiles = shuyi_cli::contract::target::engine_profiles().unwrap();
        let mut h = game().harness().as_target(&profiles["web"].profile);
        h.keys("Backspace");
        assert!(!h.quit_requested());
        let mut h = game().harness().as_target(&profiles["desktop"].profile);
        h.keys("Backspace");
        assert!(h.quit_requested());
    }
    // ANCHOR_END: as_target

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
        // The keys come from the game's keymap file.
        assert_eq!(key_name(ctx.help_keys(), UiAction::Confirm), "Enter");
    }
}
