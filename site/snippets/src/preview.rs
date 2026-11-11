//! Chapter 6, a preview for your writer: the scenes of a folder's `.dlg`
//! files played in the game's own dialogue screen, read again when one is
//! saved, through the engine's `DialoguePreview`.

use std::path::Path;

use shuyi::app::run_dialogue_preview;
use shuyi::assets::dialogue::{DialogueLimits, NoDirective, NoTokens, SceneRules, Tokens};
use shuyi::prelude::*;
use shuyi::ui::dialogue::preview::{Play, PreviewGame, PreviewScreen, Showing};
use shuyi::ui::dialogue::text_box::glyph::{self, BoxPlace};
use shuyi::ui::dialogue::{DialoguePlayer, Plain, Step, TextBox, TextView};
use shuyi::ui::frame::glyph_buffer::Rect;

use crate::{Data, game};

/// Where the text box sits: the console's bottom seven rows.
const BOX: BoxPlace = BoxPlace::new(Rect::new(0, 17, 48, 7), 2, 19);

// ANCHOR: talk
/// The game's dialogue screen: a scene read in the engine's text box.
/// Confirm shows the whole page, then the next.
pub struct Talk {
    player: DialoguePlayer<Plain>,
    text: TextBox,
}

impl Talk {
    /// Characters a line of the box holds, and lines a page.
    pub const SIZE: (usize, usize) = (44, 3);

    /// The scene `player` plays, from where it is.
    pub fn new(ctx: &Ctx<Data>, player: DialoguePlayer<Plain>) -> Self {
        let words = player.current().text.unwrap_or_default();
        let text = TextBox::new(&words, Self::SIZE.0, Self::SIZE.1, ctx.widths());
        Self { player, text }
    }
}

impl Screen<Data> for Talk {
    fn name(&self) -> &'static str {
        "talk"
    }

    fn actions(&self) -> Option<Vec<UiAction>> {
        Some(vec![UiAction::Confirm])
    }

    fn update(&mut self, ctx: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        for &action in &input.actions {
            if action == UiAction::Confirm && self.text.confirm() == Step::Done {
                self.player.advance();
                if self.player.is_finished() {
                    return Transition::Pop;
                }
                *self = Self::new(ctx, self.player.clone());
            }
        }
        self.text
            .type_on(input.dt(), ctx.settings().text_speed.chars_per_s());
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        let view = TextView::new(self.text.page_lines().to_vec(), self.text.shown());
        glyph::paint(ctx, buf, BOX, &view, None);
    }

    fn is_overlay(&self) -> bool {
        true
    }
}
// ANCHOR_END: talk

// ANCHOR: preview
/// What the text box holds: the checks keep every line to it.
const LIMITS: DialogueLimits = DialogueLimits {
    box_width: Talk::SIZE.0,
    box_lines: Talk::SIZE.1,
    max_text: 200,
    max_option: 30,
    min_options: 2,
    max_options: 4,
    max_reaction_texts: 3,
};

/// The rules the format's checks hold a scene to.
pub struct Rules;

impl SceneRules<NoDirective> for Rules {
    fn limits(&self) -> &DialogueLimits {
        &LIMITS
    }

    fn tokens(&self) -> &dyn Tokens {
        &NoTokens
    }
}

/// What the game hands its preview: its rules and its dialogue screen.
pub struct GardenPreview;

impl PreviewGame<Data> for GardenPreview {
    type Directive = NoDirective;
    type Rules = Rules;

    fn rules(&self, _: &Ctx<Data>) -> Rules {
        Rules
    }

    fn play(
        &self,
        ctx: &mut Ctx<Data>,
        play: &Play<'_, NoDirective>,
    ) -> Box<dyn PreviewScreen<Data>> {
        Box::new(Talk::new(ctx, play.player(Plain)))
    }
}

impl PreviewScreen<Data> for Talk {
    fn showing(&self) -> Showing {
        Showing::of(&self.player)
    }
}

// ANCHOR_END: preview

// ANCHOR: main
/// The preview's binary (`src/bin/preview.rs`), the program the writer
/// runs: the scenes of the folder named on its command line, else
/// `scenes/`, in the game's window.
pub fn main() {
    let folder = std::env::args_os()
        .nth(1)
        .unwrap_or_else(|| "scenes".into());
    run_dialogue_preview(game(), Path::new(&folder), GardenPreview);
}
// ANCHOR_END: main

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use shuyi::assets::bundle::Layer;
    use shuyi::assets::dialogue::{Present, parse_dlg};
    use shuyi::ui::dialogue::preview::DialoguePreview;

    use super::*;

    // ANCHOR: test
    #[test]
    fn a_scene_plays_the_other_part_with_the_gardener_gone() {
        let scene = "@scene dusk\n\
                     @left gardener calm\n\
                     @if gardener\n\
                     gardener: The moon is up.\n\
                     @else\n\
                     > Nobody is in the garden.\n\
                     @endif\n\
                     @end\n";
        let files = BTreeMap::from([("dusk.dlg".to_owned(), scene.as_bytes().to_vec())]);
        let folder = Layer::owned("scenes".to_owned(), files).unwrap();
        let mut h = game().preview_harness(folder, GardenPreview);
        // Play it, then the Switches tab: send the gardener away.
        h.keys("Enter Backspace Right Enter");
        let preview = h.screen::<DialoguePreview<Data, GardenPreview>>().unwrap();
        let view = preview.view(h.ctx());
        assert!(!view.switch("gardener").unwrap().on);
        assert_eq!(view.lines[0].words, "Nobody is in the garden.");
    }
    // ANCHOR_END: test

    /// The talk of a scene of two lines, its first on screen.
    fn talk() -> (Ctx<Data>, Talk) {
        let source = "@scene dusk\n> The moon is up.\n> The garden sleeps.\n@end\n";
        let (scenes, errors) = parse_dlg::<NoDirective>("dusk.dlg", source);
        assert_eq!(errors, []);
        let ctx = game().ctx().unwrap();
        let player = DialoguePlayer::new(&scenes[0].scene, Plain, &Present::Everyone);
        let talk = Talk::new(&ctx, player);
        (ctx, talk)
    }

    fn press(talk: &mut Talk, ctx: &mut Ctx<Data>, action: UiAction) -> bool {
        let input = FrameInput::new(vec![action], 0.0, vec![]);
        matches!(talk.update(ctx, &input), Transition::Pop)
    }

    #[test]
    fn the_talk_reads_on_with_confirm_and_pops_after_its_last_line() {
        let (mut ctx, mut talk) = talk();
        assert_eq!(talk.name(), "talk");
        assert_eq!(talk.actions(), Some(vec![UiAction::Confirm]));
        assert!(talk.is_overlay());
        // Another action does nothing; Confirm shows the page, then goes on.
        assert!(!press(&mut talk, &mut ctx, UiAction::Down));
        assert!(!press(&mut talk, &mut ctx, UiAction::Confirm));
        assert_eq!(talk.showing().line, talk.player.line().cloned());
        assert_eq!(
            talk.player.current().text.as_deref(),
            Some("The moon is up.")
        );
        assert!(!press(&mut talk, &mut ctx, UiAction::Confirm));
        assert_eq!(
            talk.player.current().text.as_deref(),
            Some("The garden sleeps.")
        );
        assert!(!press(&mut talk, &mut ctx, UiAction::Confirm));
        assert!(press(&mut talk, &mut ctx, UiAction::Confirm));
    }

    #[test]
    fn the_talk_paints_its_page_in_the_box() {
        let (mut ctx, mut talk) = talk();
        let _ = press(&mut talk, &mut ctx, UiAction::Confirm);
        let mut buf = GlyphBuffer::new(48, 24, ctx.game.blank_cell(&ctx.palette));
        talk.draw(&ctx, &mut buf);
        let row: String = (0..48)
            .map(|x| buf.get(x, 19).map_or(' ', |c| c.glyph))
            .collect();
        assert!(row.contains("The moon is up."), "{row}");
    }
}
