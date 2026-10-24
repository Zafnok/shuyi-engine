//! The talk: a scene of `files/talk.dlg` read one text box at a time in
//! the engine's text box, over the title. Confirm shows the whole page,
//! then the next, and picks the reply the cursor is on; Up and Down move
//! the cursor while the replies are up; Cancel leaves. It closes when the
//! scene ends.
//!
//! Its part of the game's data is [`Own`]: every scene of the file, read
//! and checked with the rest of the files ([`own`]). What it shows is a
//! [`TextView`], plain data, that the engine's glyph skin paints.

use std::any::Any;

use shuyi::assets::Formats;
use shuyi::assets::dialogue::{
    DialogueFile, DialogueLimits, DialogueTable, NoDirective, NoTokens, Present, SceneRules, Side,
    Tokens, scripts_from_parsed,
};
use shuyi::prelude::*;
use shuyi::ui::dialogue::text_box::glyph::{self, BoxPlace};
use shuyi::ui::dialogue::{DialoguePlayer, Plain, Step, TextBox, TextView, WaitView};
use shuyi::ui::widgets::menu::{Label, MenuItemView, MenuView};

use crate::Data;

/// The game's dialogue, in its files.
// check-text: not player text, a file name
pub const TALK: &str = "talk.dlg";

/// The scene the title starts.
// check-text: not player text, a scene id
pub const FIRST: &str = "intro";

/// Characters a line of the text box holds.
const WIDTH: usize = 42;

/// Lines a page of the text box holds.
const LINES: usize = 3;

/// Where the text box goes: the bottom seven rows.
const BOX: BoxPlace = BoxPlace::new(Rect::new(0, 17, 48, 7), 2, 19);

/// What the scenes keep to: the box's size, two to four replies, and no
/// tokens of the game's own.
const LIMITS: DialogueLimits = DialogueLimits {
    box_width: WIDTH,
    box_lines: LINES,
    max_text: WIDTH * LINES,
    max_option: WIDTH - 4,
    min_options: 2,
    max_options: 4,
    max_reaction_texts: 2,
};

/// The game's rules for its scenes ([`LIMITS`]).
struct Rules;

impl SceneRules<NoDirective> for Rules {
    fn limits(&self) -> &DialogueLimits {
        &LIMITS
    }

    fn tokens(&self) -> &dyn Tokens {
        &NoTokens
    }
}

/// The stage's part of the game's data: every scene of [`TALK`], by id.
#[derive(Debug)]
pub struct Own {
    /// The scenes.
    pub scenes: DialogueTable,
}

/// Reads the scenes from the game's files through the engine's `.dlg`
/// format, then checks them against the game's [`Rules`]: every problem,
/// with its file and line.
pub fn own(bundle: &Bundle) -> Result<Own, Vec<ContentError>> {
    let file = Formats::standard().load::<DialogueFile>(bundle, TALK)?;
    let scripts = scripts_from_parsed([file], &Rules)?;
    Ok(Own {
        scenes: scripts.table,
    })
}

/// The talk, as the title starts it.
pub fn first(ctx: &mut Ctx<Data>) -> Box<dyn Screen<Data>> {
    Box::new(Talk::new(ctx, FIRST))
}

/// Every screen the stage can show.
pub fn screens(ctx: &mut Ctx<Data>) -> Vec<Box<dyn Screen<Data>>> {
    vec![first(ctx)]
}

/// The talk screen.
#[derive(Debug)]
pub struct Talk {
    /// The scene playing.
    pub player: DialoguePlayer<Plain>,
    /// Its text box.
    pub text: TextBox,
    /// The reply the cursor is on, while the replies are up.
    pub reply: usize,
}

impl Talk {
    /// The game's scene `id`, played as it is for everyone (an empty one
    /// if the game has none).
    pub fn new(ctx: &Ctx<Data>, id: &str) -> Self {
        let scene = ctx.game.own.scenes.get(id).cloned().unwrap_or_default();
        let player = DialoguePlayer::new(&scene, Plain, &Present::Everyone);
        let text = Self::text_of(&player);
        Self {
            player,
            text,
            reply: 0,
        }
    }

    /// A text box of the player's text now: revealed at once while the
    /// replies are up, since that line was read already.
    fn text_of(player: &DialoguePlayer<Plain>) -> TextBox {
        let text = player.current().text.unwrap_or_default();
        let mut text = TextBox::new(&text, WIDTH, LINES);
        if player.is_choosing() {
            text.reveal();
        }
        text
    }

    /// How many replies are up: none unless a choice is open.
    fn replies(&self) -> usize {
        self.player.current().choices.map_or(0, |c| c.len())
    }

    /// What its text box shows.
    pub fn view(&self, ctx: &Ctx<Data>) -> TextView {
        let current = self.player.current();
        let mut view = TextView::new(self.text.page_lines().to_vec(), self.text.shown());
        view.narration = current.narration;
        if let Some(replies) = current.choices {
            let items = replies
                .into_iter()
                .enumerate()
                .map(|(i, reply)| MenuItemView {
                    label: Label::Text(reply.into_owned()),
                    suffix: None,
                    note: None,
                    enabled: true,
                    focused: i == self.reply,
                })
                .collect();
            view.choices = Some(MenuView::new(items));
        } else if self.text.is_revealed() {
            let next = ctx.text_with("talk.next", &[]);
            view.waiting = Some(WaitView::new(next, self.text.marker_lit()));
        }
        view
    }

    /// The name of whoever speaks now, from the game's text
    /// (`character.<id>`), if anyone does.
    pub fn speaker(&self, ctx: &Ctx<Data>) -> Option<String> {
        let current = self.player.current();
        let speaking = match current.speaker {
            Some(Side::Left) => current.left,
            Some(Side::Right) => current.right,
            None => None,
        };
        speaking.map(|p| ctx.text(&format!("character.{}", p.character.0)).to_owned())
    }
}

impl Screen<Data> for Talk {
    // check-text: not player text, the screen's id
    fn name(&self) -> &'static str {
        "talk"
    }

    /// Confirm reads on and picks a reply, Up and Down move between the
    /// replies, Cancel leaves.
    fn actions(&self) -> Option<Vec<UiAction>> {
        Some(vec![
            UiAction::Confirm,
            UiAction::Up,
            UiAction::Down,
            UiAction::Cancel,
        ])
    }

    fn update(&mut self, ctx: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        for &action in &input.actions {
            let replies = self.replies();
            match action {
                UiAction::Cancel => return Transition::Pop,
                UiAction::Up if replies > 0 => self.reply = (self.reply + replies - 1) % replies,
                UiAction::Down if replies > 0 => self.reply = (self.reply + 1) % replies,
                UiAction::Confirm if replies > 0 => {
                    self.player.choose(self.reply);
                    self.reply = 0;
                    self.text = Self::text_of(&self.player);
                }
                // The last page read: the next line.
                UiAction::Confirm if self.text.confirm() == Step::Done => {
                    self.player.advance();
                    self.text = Self::text_of(&self.player);
                }
                _ => {}
            }
            if self.player.is_finished() {
                return Transition::Pop;
            }
        }
        self.text.hold(input.is_held(UiAction::Confirm), input.dt());
        self.text
            .type_on(input.dt(), ctx.settings().text_speed.chars_per_s());
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        let view = self.view(ctx);
        glyph::paint(ctx, buf, BOX, &view, self.speaker(ctx).as_deref());
    }

    fn is_overlay(&self) -> bool {
        true
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}
