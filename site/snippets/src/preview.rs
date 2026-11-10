//! Chapter 6, a preview for your writer: the scenes of a folder's `.dlg`
//! files played in the game's own dialogue screen, read again when one is
//! saved, through the engine's `DialoguePreview`.

use shuyi::assets::bundle::Layer;
use shuyi::assets::dialogue::{DialogueLimits, NoDirective, NoTokens, SceneRules, Tokens};
use shuyi::prelude::*;
use shuyi::ui::dialogue::preview::{
    DialogueFolder, DialoguePreview, Play, PreviewGame, PreviewScreen, Showing,
};
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

/// The preview over the folder read as `folder`: the game's data from its
/// own files, and the scenes from the folder's.
pub fn preview(folder: Layer) -> Result<Game<Data>, String> {
    let ctx = game().ctx()?;
    let root = DialoguePreview::new(&ctx, GardenPreview);
    Game::new(ctx, Box::new(root), GameHooks::default())
        .with_debug_screens(false)
        .with_module(DialogueFolder::new(folder))
        .map_err(|e| e.to_string())
}
// ANCHOR_END: preview

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use shuyi::platform::harness::Harness;

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
        let mut h = Harness::new(preview(folder).unwrap());
        // Play it, then the Switches tab: send the gardener away.
        h.keys("Enter Backspace Right Enter");
        let preview = h.screen::<DialoguePreview<Data, GardenPreview>>().unwrap();
        let view = preview.view(h.ctx());
        assert!(!view.switch("gardener").unwrap().on);
        assert_eq!(view.lines[0].words, "Nobody is in the garden.");
    }
    // ANCHOR_END: test
}
