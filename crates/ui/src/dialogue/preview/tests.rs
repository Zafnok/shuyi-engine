//! The preview over the toy game's dialogue screen ([`toy::Talk`]), with
//! files handed in as a layer: what its view lists and plays.

use alloc::borrow::Cow;
use alloc::collections::BTreeMap;
use core::cell::Cell as Seen;

use shuyi_assets::bundle::Layer;
use shuyi_assets::dialogue::{DialogueLimits, NoDirective, NoTokens, Tokens};

use super::*;
use crate::dialogue::{Plain, TextBox};
use crate::frame::color::EngineColor;
use crate::frame::glyph_buffer::Cell;
use crate::game::Game;
use crate::module::Module;
use crate::toy::{self, Action, Toy, ToyCtx};

/// A scene that tests whether the scout is there, and one of narration.
const GATE: &str = "\
@scene gate
@left mentor neutral
@right scout happy
mentor (quietly): Is the road clear?
scout: Not since noon.
@if scout
scout: I'll keep watch.
@else
> Nobody keeps watch.
@endif
mentor: Then we go.
@end
@scene second
> Only narration.
@end
";

/// A file with a problem a check finds, its message carrying a fix.
const LONELY: &str = "@scene lonely\nmentor: Hello?\n@end\n";

/// The text box's limits: wide enough for every line here.
const LIMITS: DialogueLimits = DialogueLimits {
    box_width: 34,
    box_lines: 3,
    max_text: 100,
    max_option: 30,
    min_options: 2,
    max_options: 4,
    max_reaction_texts: 3,
};

/// The rules: anyone speaks, no tokens.
struct Rules;

impl SceneRules<NoDirective> for Rules {
    fn limits(&self) -> &DialogueLimits {
        &LIMITS
    }

    fn tokens(&self) -> &dyn Tokens {
        &NoTokens
    }
}

/// The toy's preview: its `Talk` plays, with one switch of its own, whose
/// value it notes as its screen is made; its own check finds a problem
/// in more than three scenes.
#[derive(Default)]
struct ToyPreview {
    /// The switch's value when the screen was last made.
    switched: Seen<Option<bool>>,
}

impl PreviewGame<Toy> for ToyPreview {
    type Directive = NoDirective;
    type Rules = Rules;

    fn rules(&self, _: &ToyCtx) -> Rules {
        Rules
    }

    fn problems(&self, _: &ToyCtx, scenes: &[ParsedScene<NoDirective>]) -> Vec<ContentError> {
        let many = scenes.len() > 3;
        let error = ContentError::new("all", "too many scenes; keep three").at(1, None);
        if many { vec![error] } else { Vec::new() }
    }

    fn switches(&self, _: &ToyCtx) -> Vec<String> {
        vec!["hero named".to_owned()]
    }

    fn play(&self, ctx: &mut ToyCtx, play: &Play<'_, NoDirective>) -> Box<dyn PreviewScreen<Toy>> {
        self.switched.set(Some(play.switch(0)));
        let player = play.player(Plain);
        let text = player.current().text.unwrap_or_default();
        let text = TextBox::new(&text, 34, 3, ctx.widths());
        Box::new(toy::Talk { player, text })
    }
}

impl PreviewScreen<Toy> for toy::Talk {
    fn showing(&self) -> Showing {
        Showing::of(&self.player)
    }
}

type Preview = DialoguePreview<Toy, ToyPreview>;

/// The files as a folder's layer.
fn layer(files: &[(&str, &str)]) -> Layer {
    let files: BTreeMap<String, Vec<u8>> = files
        .iter()
        .map(|(path, text)| ((*path).to_owned(), text.as_bytes().to_vec()))
        .collect();
    Layer::owned("folder".to_owned(), files).unwrap()
}

/// The toy game with the folder's module over `files`, the module (to
/// tell it of changes), and the preview.
fn preview(files: &[(&str, &str)]) -> (Game<Toy>, DialogueFolder, Layer, Preview) {
    let layer = layer(files);
    let folder = DialogueFolder::new(layer);
    let game = toy::game().with_module(folder.clone()).unwrap();
    let preview = Preview::new(game.ctx(), ToyPreview::default());
    (game, folder, layer, preview)
}

fn press(p: &mut Preview, ctx: &mut ToyCtx, actions: &[Action]) {
    let input = FrameInput::new(actions.to_vec(), 0.0, vec![]);
    let _ = p.update(ctx, &input);
}

/// Each action in turn, a frame each.
fn presses(p: &mut Preview, ctx: &mut ToyCtx, actions: &[Action]) {
    for &action in actions {
        press(p, ctx, &[action]);
    }
}

/// Types `text` in one frame.
fn typed(p: &mut Preview, ctx: &mut ToyCtx, text: &str) {
    let input = FrameInput::new(vec![], 0.0, vec![]).with_typing(vec![], text.chars().collect());
    let _ = p.update(ctx, &input);
}

/// The text the game's screen shows.
fn shown(p: &Preview) -> String {
    let talk = p
        .playing()
        .and_then(|s| s.as_any()?.downcast_ref::<toy::Talk>());
    let text = talk.and_then(|t| t.player.current().text.map(Cow::into_owned));
    text.unwrap_or_default()
}

/// Reads the page, then goes on to the next text box.
fn read_on(p: &mut Preview, ctx: &mut ToyCtx) {
    presses(p, ctx, &[Action::Confirm, Action::Confirm]);
}

/// Changes `path` in the folder to `text`, and tells the module, as a save
/// on disk does (a layer's bytes change only with `std`).
#[cfg(feature = "std")]
fn save(folder: &mut DialogueFolder, layer: Layer, ctx: &mut ToyCtx, path: &str, text: &str) {
    assert!(layer.replace(path, text.as_bytes().to_vec()));
    Module::<Toy>::reloaded(folder, ctx, &[path.to_owned()]);
}

#[test]
fn the_scenes_are_listed_by_file_in_file_order_a_file_with_problems_marked() {
    let (game, _, _, p) = preview(&[("b/gate.dlg", GATE), ("a.dlg", LONELY)]);
    let view = p.view(game.ctx());
    let rows: Vec<(&str, &str, bool)> = view
        .scenes
        .iter()
        .map(|s| (s.file.as_str(), s.id.as_str(), s.problems))
        .collect();
    assert_eq!(
        rows,
        [
            ("a.dlg", "lonely", true),
            ("b/gate.dlg", "gate", false),
            ("b/gate.dlg", "second", false),
        ]
    );
    assert_eq!(view.tab, Tab::Scenes);
    assert!(view.open);
    assert_eq!(view.tabs, ["Scenes", "Switches", "Lines", "Problems"]);
}

#[test]
fn a_folder_with_no_scenes_says_so() {
    let (game, _, _, p) = preview(&[("notes.txt", "not a scene")]);
    let view = p.view(game.ctx());
    assert_eq!(view.scenes, []);
    assert_eq!(
        view.note.as_deref(),
        Some("No scenes: save a .dlg file in the folder")
    );
}

#[test]
fn confirm_plays_a_scene_in_the_games_screen_from_its_first_text_box() {
    let (mut game, _, _, mut p) = preview(&[("gate.dlg", GATE)]);
    let ctx = game.ctx_mut();
    press(&mut p, ctx, &[Action::Confirm]);
    let view = p.view(ctx);
    assert!(!view.open);
    let playing = view.playing.unwrap();
    assert_eq!((playing.scene.as_str(), playing.text_box), ("gate", 0));
    assert!(!playing.finished);
    assert_eq!(shown(&p), "Is the road clear?");
    assert!(view.scenes[0].picked);
    // The game's screen reads on with Confirm.
    read_on(&mut p, ctx);
    assert_eq!(shown(&p), "Not since noon.");
    assert_eq!(p.view(ctx).playing.unwrap().text_box, 1);
}

#[test]
fn the_strip_shows_the_file_the_line_and_its_direction_until_switched_off() {
    let (mut game, _, _, mut p) = preview(&[("gate.dlg", GATE)]);
    let ctx = game.ctx_mut();
    press(&mut p, ctx, &[Action::Confirm]);
    let strip = p.view(ctx).strip.unwrap();
    assert_eq!((strip.file.as_str(), strip.line), ("gate.dlg", 4));
    assert_eq!(strip.direction.as_deref(), Some("quietly"));
    assert_eq!(strip.help, ctx.text_with(HELP_PLAYING, &[]));
    read_on(&mut p, ctx);
    let strip = p.view(ctx).strip.unwrap();
    assert_eq!((strip.line, strip.direction), (5, None));
    // Off on the Switches tab, its last row.
    presses(&mut p, ctx, &[Action::Cancel, Action::CursorRight]);
    let last = p.view(ctx).switches.len() - 1;
    presses(&mut p, ctx, &vec![Action::CursorDown; last]);
    presses(&mut p, ctx, &[Action::Confirm, Action::Cancel]);
    let view = p.view(ctx);
    assert!(!view.open);
    assert_eq!(view.strip, None);
}

#[test]
fn cancel_shows_the_tabs_over_the_scene_and_cancel_again_goes_back_to_it() {
    let (mut game, _, _, mut p) = preview(&[("gate.dlg", GATE)]);
    let ctx = game.ctx_mut();
    press(&mut p, ctx, &[Action::Confirm]);
    read_on(&mut p, ctx);
    press(&mut p, ctx, &[Action::Cancel]);
    assert!(p.view(ctx).open);
    // The scene waits where it was.
    assert_eq!(shown(&p), "Not since noon.");
    press(&mut p, ctx, &[Action::Cancel]);
    assert!(!p.view(ctx).open);
    assert_eq!(shown(&p), "Not since noon.");
}

#[test]
fn a_switch_per_character_tested_all_there_at_first_then_the_games_and_the_strips() {
    let (mut game, _, _, mut p) = preview(&[("gate.dlg", GATE)]);
    let ctx = game.ctx_mut();
    assert_eq!(p.view(ctx).note.as_deref(), None, "the Scenes tab has rows");
    press(&mut p, ctx, &[Action::Confirm]);
    presses(&mut p, ctx, &[Action::Cancel, Action::CursorRight]);
    let view = p.view(ctx);
    assert_eq!(view.tab, Tab::Switches);
    let rows: Vec<(&str, &str, bool)> = view
        .switches
        .iter()
        .map(|s| (s.name.as_str(), s.value.as_str(), s.on))
        .collect();
    assert_eq!(
        rows,
        [
            ("scout", "Here", true),
            ("hero named", "Off", false),
            ("Strip: file, line, direction", "On", true),
        ]
    );
}

#[test]
fn a_character_switched_off_plays_the_else_part_again_from_the_same_text_box() {
    let (mut game, _, _, mut p) = preview(&[("gate.dlg", GATE)]);
    let ctx = game.ctx_mut();
    press(&mut p, ctx, &[Action::Confirm]);
    read_on(&mut p, ctx);
    read_on(&mut p, ctx);
    assert_eq!(shown(&p), "I'll keep watch.");
    // The scout goes: the same text box, the third, is now the @else line.
    presses(
        &mut p,
        ctx,
        &[Action::Cancel, Action::CursorRight, Action::Confirm],
    );
    let view = p.view(ctx);
    assert_eq!(view.switch("scout").unwrap().value, "Gone");
    assert_eq!(view.playing.as_ref().unwrap().text_box, 2);
    assert_eq!(shown(&p), "Nobody keeps watch.");
    let lines: Vec<u32> = view.lines.iter().map(|l| l.line).collect();
    assert_eq!(lines, [4, 5, 9, 11]);
    assert_eq!(view.current_line().unwrap().words, "Nobody keeps watch.");
}

#[test]
fn a_switch_of_the_games_reaches_its_screen() {
    let (mut game, _, _, mut p) = preview(&[("gate.dlg", GATE)]);
    let ctx = game.ctx_mut();
    press(&mut p, ctx, &[Action::Confirm]);
    assert_eq!(p.game().switched.get(), Some(false));
    presses(
        &mut p,
        ctx,
        &[
            Action::Cancel,
            Action::CursorRight,
            Action::CursorDown,
            Action::Confirm,
        ],
    );
    assert_eq!(p.view(ctx).switch("hero named").unwrap().value, "On");
    assert_eq!(p.game().switched.get(), Some(true), "played again");
}

#[test]
fn the_lines_tab_lists_the_text_boxes_and_plays_from_one() {
    let (mut game, _, _, mut p) = preview(&[("gate.dlg", GATE)]);
    let ctx = game.ctx_mut();
    press(&mut p, ctx, &[Action::Confirm]);
    presses(
        &mut p,
        ctx,
        &[Action::Cancel, Action::CursorLeft, Action::CursorLeft],
    );
    let view = p.view(ctx);
    assert_eq!(view.tab, Tab::Lines);
    let rows: Vec<(u32, Option<&str>, &str)> = view
        .lines
        .iter()
        .map(|l| (l.line, l.speaker.as_deref(), l.words.as_str()))
        .collect();
    assert_eq!(
        rows,
        [
            (4, Some("mentor"), "Is the road clear?"),
            (5, Some("scout"), "Not since noon."),
            (7, Some("scout"), "I'll keep watch."),
            (11, Some("mentor"), "Then we go."),
        ]
    );
    assert_eq!(view.go_to.label, "Go to line");
    // The fourth text box: the line box, then three down.
    presses(&mut p, ctx, &[Action::CursorDown; 4]);
    press(&mut p, ctx, &[Action::Confirm]);
    assert_eq!(shown(&p), "Then we go.");
    assert_eq!(p.view(ctx).playing.unwrap().text_box, 3);
}

#[test]
fn a_typed_line_number_plays_the_text_box_it_falls_in() {
    let (mut game, _, _, mut p) = preview(&[("gate.dlg", GATE)]);
    let ctx = game.ctx_mut();
    press(&mut p, ctx, &[Action::Confirm]);
    presses(
        &mut p,
        ctx,
        &[Action::Cancel, Action::CursorLeft, Action::CursorLeft],
    );
    press(&mut p, ctx, &[Action::Confirm]);
    assert!(p.view(ctx).go_to.typing);
    // Line 8 is the @else: in the text box of line 7.
    typed(&mut p, ctx, "8");
    assert_eq!(p.view(ctx).go_to.digits, "8");
    press(&mut p, ctx, &[Action::Confirm]);
    let view = p.view(ctx);
    assert!(!view.go_to.typing);
    assert!(!view.open);
    assert_eq!(view.playing.unwrap().text_box, 2);
    assert_eq!(shown(&p), "I'll keep watch.");
}

#[test]
fn the_cursor_steps_the_line_box_and_cancel_closes_it() {
    let (mut game, _, _, mut p) = preview(&[("gate.dlg", GATE)]);
    let ctx = game.ctx_mut();
    press(&mut p, ctx, &[Action::Confirm]);
    presses(
        &mut p,
        ctx,
        &[Action::Cancel, Action::CursorLeft, Action::CursorLeft],
    );
    press(&mut p, ctx, &[Action::Confirm]);
    presses(&mut p, ctx, &[Action::CursorUp; 5]);
    press(&mut p, ctx, &[Action::CursorDown]);
    assert_eq!(p.view(ctx).go_to.digits, "4");
    press(&mut p, ctx, &[Action::Cancel]);
    let view = p.view(ctx);
    assert!(!view.go_to.typing);
    assert!(view.open, "the tabs stay");
}

#[test]
fn the_line_box_needs_a_scene() {
    let (mut game, _, _, mut p) = preview(&[("gate.dlg", GATE)]);
    let ctx = game.ctx_mut();
    presses(
        &mut p,
        ctx,
        &[Action::CursorLeft, Action::CursorLeft, Action::Confirm],
    );
    let view = p.view(ctx);
    assert!(!view.go_to.typing);
    assert_eq!(view.note.as_deref(), Some("Pick a scene first"));
}

#[test]
fn the_scene_over_shows_the_lines_tab() {
    let (mut game, _, _, mut p) = preview(&[("gate.dlg", GATE)]);
    let ctx = game.ctx_mut();
    press(&mut p, ctx, &[Action::Confirm]);
    for _ in 0..4 {
        read_on(&mut p, ctx);
    }
    let view = p.view(ctx);
    assert!(view.open);
    assert_eq!(view.tab, Tab::Lines);
    assert!(view.playing.as_ref().unwrap().finished);
    assert_eq!(view.current_line(), None);
    assert_eq!(
        view.note.as_deref(),
        Some("The scene is over: pick a line to play it again")
    );
}

#[test]
fn the_problems_have_their_file_line_message_and_fix() {
    let (game, _, _, p) = preview(&[("gate.dlg", GATE), ("lonely.dlg", LONELY)]);
    let view = p.view(game.ctx());
    let rows: Vec<(&str, Option<u32>, &str, Option<&str>)> = view
        .problems
        .iter()
        .map(|p| {
            (
                p.file.as_str(),
                p.line,
                p.message.as_str(),
                p.fix.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [(
            "lonely.dlg",
            Some(2),
            "\"mentor\" speaks but is not on screen",
            Some("place them with @left or @right first"),
        )]
    );
}

#[test]
fn the_games_own_checks_are_listed_too() {
    let more = "@scene c\n> C.\n@end\n@scene d\n> D.\n@end\n";
    let (game, _, _, mut p) = preview(&[("gate.dlg", GATE), ("more.dlg", more)]);
    p.tab = Tab::Problems;
    let view = p.view(game.ctx());
    let rows: Vec<(&str, &str, Option<&str>)> = view
        .problems
        .iter()
        .map(|p| (p.file.as_str(), p.message.as_str(), p.fix.as_deref()))
        .collect();
    assert_eq!(rows, [("all", "too many scenes", Some("keep three"))]);
    assert_eq!(view.note, None);
}

#[test]
#[cfg(feature = "std")]
fn a_changed_file_plays_again_from_the_same_text_box() {
    let (mut game, mut folder, layer, mut p) = preview(&[("gate.dlg", GATE)]);
    let ctx = game.ctx_mut();
    press(&mut p, ctx, &[Action::Confirm]);
    read_on(&mut p, ctx);
    let reworded = GATE.replace("Not since noon.", "Not since dawn.");
    save(&mut folder, layer, ctx, "gate.dlg", &reworded);
    press(&mut p, ctx, &[]);
    assert_eq!(shown(&p), "Not since dawn.");
    assert_eq!(p.view(ctx).playing.unwrap().text_box, 1);
}

#[test]
#[cfg(feature = "std")]
fn a_scene_with_fewer_text_boxes_now_plays_from_its_start() {
    let (mut game, mut folder, layer, mut p) = preview(&[("gate.dlg", GATE)]);
    let ctx = game.ctx_mut();
    press(&mut p, ctx, &[Action::Confirm]);
    for _ in 0..3 {
        read_on(&mut p, ctx);
    }
    let short = "@scene gate\n> Short now.\n@end\n";
    save(&mut folder, layer, ctx, "gate.dlg", short);
    press(&mut p, ctx, &[]);
    assert_eq!(shown(&p), "Short now.");
    assert_eq!(p.view(ctx).playing.unwrap().text_box, 0);
}

#[test]
#[cfg(feature = "std")]
fn a_broken_file_keeps_its_last_good_scenes_and_lists_its_problems() {
    let (mut game, mut folder, layer, mut p) = preview(&[("gate.dlg", GATE)]);
    let ctx = game.ctx_mut();
    press(&mut p, ctx, &[Action::Confirm]);
    read_on(&mut p, ctx);
    let broken = GATE.replace("@endif\n", "");
    save(&mut folder, layer, ctx, "gate.dlg", &broken);
    press(&mut p, ctx, &[]);
    // The scene goes on as it was.
    assert_eq!(shown(&p), "Not since noon.");
    let view = p.view(ctx);
    assert!(view.scenes.iter().all(|s| s.problems));
    assert_eq!(view.scenes.len(), 2);
    let problem = &view.problems[0];
    assert_eq!(problem.file, "gate.dlg");
    assert!(problem.line.is_some(), "{problem:?}");
    // Mended: no problem left.
    save(&mut folder, layer, ctx, "gate.dlg", GATE);
    press(&mut p, ctx, &[]);
    assert_eq!(p.view(ctx).problems, []);
}

#[test]
#[cfg(feature = "std")]
fn a_change_to_a_file_that_isnt_a_scripts_reads_nothing_again() {
    let (mut game, mut folder, layer, mut p) = preview(&[("gate.dlg", GATE), ("a.ron", "()")]);
    let ctx = game.ctx_mut();
    let before = ctx.module_state::<DialogueFiles>().unwrap().version();
    save(&mut folder, layer, ctx, "a.ron", "(1)");
    assert_eq!(
        ctx.module_state::<DialogueFiles>().unwrap().version(),
        before
    );
    press(&mut p, ctx, &[]);
    assert_eq!(p.view(ctx).scenes.len(), 2);
}

#[test]
#[cfg(feature = "std")]
fn a_scene_taken_out_of_its_file_stops_playing() {
    let (mut game, mut folder, layer, mut p) = preview(&[("gate.dlg", GATE)]);
    let ctx = game.ctx_mut();
    press(&mut p, ctx, &[Action::Confirm]);
    save(
        &mut folder,
        layer,
        ctx,
        "gate.dlg",
        "@scene other\n> O.\n@end\n",
    );
    press(&mut p, ctx, &[]);
    let view = p.view(ctx);
    assert!(view.open);
    assert_eq!(view.playing, None);
    assert!(p.playing().is_none());
}

#[test]
fn it_declares_the_cursor_confirm_cancel_and_the_games_screens_actions() {
    let (mut game, _, _, mut p) = preview(&[("gate.dlg", GATE)]);
    let ctx = game.ctx_mut();
    let ui = [
        Action::CursorLeft,
        Action::CursorDown,
        Action::CursorUp,
        Action::CursorRight,
        Action::Confirm,
        Action::Cancel,
    ];
    let mut declared = p.actions().unwrap();
    declared.sort();
    let mut expected = ui.to_vec();
    expected.sort();
    assert_eq!(declared, expected);
    press(&mut p, ctx, &[Action::Confirm]);
    let mut declared = p.actions().unwrap();
    declared.sort();
    assert_eq!(
        declared, expected,
        "the toy's Talk reads Confirm and Cancel"
    );
}

#[test]
fn the_skin_paints_the_tabs_and_the_strip() {
    let (mut game, _, _, mut p) = preview(&[("gate.dlg", GATE), ("lonely.dlg", LONELY)]);
    let ctx = game.ctx_mut();
    let row = |buf: &GlyphBuffer, y: i32| -> String {
        (0..i32::from(buf.width()))
            .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
            .collect::<String>()
            .trim_end()
            .to_owned()
    };
    let colour = |e| ctx.palette.engine(e);
    let blank = Cell::new(
        ' ',
        colour(EngineColor::Text),
        colour(EngineColor::Background),
    );
    let mut buf = GlyphBuffer::new(40, 20, blank);
    p.draw(ctx, &mut buf);
    assert_eq!(row(&buf, 0), "  Scenes   Switches   Lines   Problems");
    assert_eq!(row(&buf, 2), "    gate.dlg: gate");
    assert_eq!(row(&buf, 3), "    gate.dlg: second");
    assert_eq!(row(&buf, 4), "  ! lonely.dlg: lonely");
    assert_eq!(row(&buf, 19).trim(), ctx.text_with(HELP, &[]));
    press(&mut p, ctx, &[Action::Confirm]);
    let mut buf = GlyphBuffer::new(40, 20, blank);
    p.draw(ctx, &mut buf);
    // The help at the right end, the file, line and direction at the left.
    let help = ctx.text_with(HELP_PLAYING, &[]);
    let strip = row(&buf, 0);
    assert!(strip.starts_with(" gate.dlg:4  (quietly) "), "{strip}");
    assert!(strip.ends_with(&format!(" {help}")), "{strip}");
    assert_eq!(strip.chars().count(), 39);
}
