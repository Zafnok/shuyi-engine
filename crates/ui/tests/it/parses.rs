//! No frame parses a text (ticket 2266): the toy's language learns each
//! text's form when it loads, and every fill of a screen's text reads it.
//! The count is the thread's, so tests beside it don't count here.

use shuyi_assets::message::parse_count;
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_ui::frame::GlyphBuffer;
use shuyi_ui::frame::color::EngineColor;
use shuyi_ui::input::PadKind;
use shuyi_ui::text::{Args, Label};
use shuyi_ui::toy::{self, Action, Toy, ToyCtx};
use shuyi_ui::{FrameInput, Game, GameHooks, Screen, Transition};

/// A screen of the toy's texts of each kind: plain, keys, a plural and a
/// select on `device`, by each of `Ctx`'s fills.
struct Texts;

impl Screen<Toy> for Texts {
    fn name(&self) -> &'static str {
        "texts"
    }

    fn update(&mut self, _: &mut ToyCtx, _: &FrameInput<Action>) -> Transition<Toy> {
        Transition::None
    }

    fn draw(&self, ctx: &ToyCtx, buf: &mut GlyphBuffer) {
        let (fg, bg) = (
            ctx.palette.engine(EngineColor::Text),
            ctx.palette.engine(EngineColor::Background),
        );
        let n: &Args<'_> = &[("n", &8)];
        let lines = [
            ctx.text_styled("menu.title", &[]).plain(),
            ctx.text_with("home.help", &[]),
            ctx.text_styled("profile.message.too_many", n).plain(),
            ctx.text_with("toy.turn", &[]),
            ctx.label(&Label::key("profile.numbered").with_arg("n", 3)),
            ctx.label_styled(&Label::key("toy.turn")).plain(),
        ];
        for (y, line) in (0..).zip(lines) {
            buf.print(0, y, &line, fg, bg);
        }
    }
}

#[test]
fn no_frame_of_the_toys_screens_parses_a_text() {
    let mut h = Harness::new(Game::new(toy::ctx(), Box::new(Texts), GameHooks::default()));
    parse_count::take();
    h.wait(FRAME_DT * 3.0);
    let snapshot = h.snapshot();
    for text in ["Menu", "8 profiles", "Turn with", "Player 3"] {
        assert!(snapshot.contains(text), "{text}: {snapshot}");
    }
    h.use_pad(PadKind::Playdate)
        .pad("DpadDown")
        .wait(FRAME_DT * 3.0);
    assert!(h.snapshot().contains("Crank with"), "{}", h.snapshot());
    assert_eq!(parse_count::take(), 0, "a frame parsed a text");
    // The engine's screens: the options, the Key bindings screen, a talk.
    let mut h = Harness::new(toy::lobby());
    parse_count::take();
    h.keys("f Down Down d Down f d d Down Down f f f f d");
    h.wait(FRAME_DT * 3.0);
    assert_eq!(parse_count::take(), 0, "a frame parsed a text");
}
