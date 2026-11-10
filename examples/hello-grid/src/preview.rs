//! Hello, Grid's dialogue preview: the program the game would hand a writer
//! (`hello-grid-preview`). It lists the scenes of the `.dlg` files in a
//! folder, plays the one picked in the game's own text box ([`Talk`]), and
//! reads a file again when it is saved, with the engine's preview screen
//! ([`DialoguePreview`]) and folder module ([`DialogueFolder`]). The game's
//! own data (its text, keys and font) comes from its embedded files: a
//! broken scene in the folder is listed as a problem, and never stops the
//! program.

use shuyi::assets::bundle::Layer;
use shuyi::assets::dialogue::{DialogueLimits, NoDirective, NoTokens, SceneRules, Tokens};
use shuyi::platform::{Card, Start};
use shuyi::ui::dialogue::Plain;
use shuyi::ui::dialogue::preview::{
    DialogueFolder, DialoguePreview, Play, PreviewGame, PreviewScreen, Showing,
};
use shuyi::ui::module::ModuleError;
use shuyi::ui::{Ctx, Game, GameHooks};

use crate::prelude::*;
use crate::screens::{HelloCtx, Talk};
use crate::{Hello, TITLE, bundle, ctx_from};

/// What the game's text box and reply menu hold: [`Talk`]'s box, a line of
/// text at most a page of it. The game has no reply choices; their limits
/// are the engine's usual ones.
pub const LIMITS: DialogueLimits = DialogueLimits {
    box_width: Talk::WIDTH,
    box_lines: Talk::LINES,
    max_text: Talk::WIDTH * Talk::LINES,
    max_option: 30,
    min_options: 2,
    max_options: 4,
    max_reaction_texts: 3,
};

/// The rules the format's checks hold a scene to: [`LIMITS`], and no
/// tokens. Every glyph of the game's font is one cell wide, the rules'
/// default widths.
#[derive(Debug, Clone, Copy, Default)]
pub struct HelloRules;

impl SceneRules<NoDirective> for HelloRules {
    fn limits(&self) -> &DialogueLimits {
        &LIMITS
    }

    fn tokens(&self) -> &dyn Tokens {
        &NoTokens
    }
}

/// What the game hands the preview: its rules, and its text box.
#[derive(Debug, Clone, Copy, Default)]
pub struct HelloPreview;

impl PreviewGame<Hello> for HelloPreview {
    type Directive = NoDirective;
    type Rules = HelloRules;

    fn rules(&self, _: &HelloCtx) -> HelloRules {
        HelloRules
    }

    fn play(
        &self,
        ctx: &mut HelloCtx,
        play: &Play<'_, NoDirective>,
    ) -> Box<dyn PreviewScreen<Hello>> {
        Box::new(Talk::playing(ctx, play.player(Plain)))
    }
}

impl PreviewScreen<Hello> for Talk {
    fn showing(&self) -> Showing {
        Showing::of(&self.player)
    }
}

/// The preview on `ctx`, over the folder read as `folder`, without the
/// debug screens: a writer's program.
///
/// # Errors
///
/// If the folder's module can't join the game (a module of its name is in
/// it already).
pub fn game(ctx: Ctx<Hello>, folder: Layer) -> Result<Game<Hello>, ModuleError> {
    let root = DialoguePreview::new(&ctx, HelloPreview);
    Game::new(ctx, Box::new(root), GameHooks::default())
        .with_debug_screens(false)
        .with_module(DialogueFolder::new(folder))
}

/// How the preview starts, over the folder read as `folder`
/// (`shuyi_app::read_watched_folder`, so a saved file is read again in an
/// optimized build too): the game's card, then the preview, with the
/// game's own files as the platform's target has them.
pub fn start(folder: Layer) -> Start<Hello> {
    Start::new(Card::text(TITLE), Hello::look, move |setup| {
        let target = setup.platform.target.clone();
        let ctx = setup.ctx(ctx_from(&bundle(Some(&target)), &target, 0));
        game(ctx, folder).map_err(|e| e.to_string())
    })
}
