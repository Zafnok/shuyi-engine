//! Hello, Grid's dialogue preview through the Harness: its scene played in
//! the game's text box, the guide sent away, a jump to a typed line, and
//! a file changed through the headless backend's script with debug tools
//! off, as an optimized build that watches its folder has them.

use std::borrow::Cow;
use std::collections::BTreeMap;

use hello_grid::preview::{self, HelloPreview};
use hello_grid::screens::Talk;
use hello_grid::tools::Start;
use hello_grid::{Action, Hello, bundle, ctx_from, file};
use shuyi::assets::bundle::Layer;
use shuyi::platform::harness::{FRAME_DT, Harness};
use shuyi::ui::Screen;
use shuyi::ui::dialogue::preview::{DialoguePreview, PreviewView, Tab};

/// The game's scene file.
fn intro() -> String {
    String::from_utf8_lossy(file("intro.dlg").unwrap_or_default()).into_owned()
}

/// The preview over a folder holding the game's scene, watched in every
/// build, in a Harness: its first frame read.
fn preview() -> (Harness<Hello>, Layer) {
    let files = BTreeMap::from([("intro.dlg".to_owned(), intro().into_bytes())]);
    let folder = Layer::owned("files".to_owned(), files).unwrap_or_else(|e| panic!("{e:?}"));
    let ctx = ctx_from(&bundle(Some("desktop")), "desktop", 7);
    let game = preview::game(ctx, folder).unwrap_or_else(|e| panic!("{e:?}"));
    let mut h = Harness::new(game);
    h.watch_files(true);
    h.wait(FRAME_DT);
    (h, folder)
}

/// What the preview shows.
fn view(h: &Harness<Hello>) -> PreviewView {
    let preview = h.screen::<DialoguePreview<Hello, HelloPreview>>();
    preview
        .unwrap_or_else(|| panic!("no preview"))
        .view(h.ctx())
}

/// The text the game's text box shows.
fn shown(h: &Harness<Hello>) -> String {
    let preview = h.screen::<DialoguePreview<Hello, HelloPreview>>();
    let talk = preview
        .and_then(|p| p.playing())
        .and_then(|s| s.as_any()?.downcast_ref::<Talk>());
    let text = talk.and_then(|t| t.player.current().text.map(Cow::into_owned));
    text.unwrap_or_default()
}

/// Each action in turn, a frame each.
fn act(h: &mut Harness<Hello>, actions: &[Action]) {
    for &action in actions {
        h.act(action);
    }
}

/// From the scene playing, the Lines tab's line box, open.
fn open_line_box(h: &mut Harness<Hello>) {
    act(
        h,
        &[
            Action::Cancel,
            Action::Right,
            Action::Right,
            Action::Confirm,
        ],
    );
    let view = view(h);
    assert_eq!(view.tab, Tab::Lines);
    assert!(view.go_to.typing);
}

#[test]
fn the_scene_is_listed_and_plays_in_the_games_text_box() {
    let (mut h, _) = preview();
    let scenes = view(&h).scenes;
    assert_eq!(scenes.len(), 1);
    assert_eq!(
        (scenes[0].file.as_str(), scenes[0].id.as_str()),
        ("intro.dlg", "intro")
    );
    assert!(!scenes[0].problems, "{:?}", view(&h).problems);
    h.act(Action::Confirm);
    assert!(shown(&h).starts_with("A small grid"));
    let strip = view(&h).strip.unwrap_or_else(|| panic!("no strip"));
    assert_eq!((strip.file.as_str(), strip.line), ("intro.dlg", 9));
}

#[test]
fn with_the_guide_sent_away_the_scene_plays_its_else_line() {
    let (mut h, _) = preview();
    h.act(Action::Confirm);
    // The Switches tab: the guide, there, is its first row.
    act(&mut h, &[Action::Cancel, Action::Right]);
    let guide = view(&h).switch("guide").cloned();
    assert!(guide.is_some_and(|g| g.on));
    h.act(Action::Confirm);
    assert!(
        !view(&h)
            .switch("guide")
            .unwrap_or_else(|| panic!("no switch"))
            .on
    );
    // On to the fourth text box, the @if block's.
    act(
        &mut h,
        &[Action::Right, Action::Down, Action::Down, Action::Down],
    );
    act(&mut h, &[Action::Down, Action::Confirm]);
    let view = view(&h);
    let line = view
        .current_line()
        .unwrap_or_else(|| panic!("nothing plays"));
    assert_eq!(
        line.words,
        "The foe trips now and then. Nobody wishes you luck."
    );
    assert_eq!((line.line, line.speaker.as_deref()), (15, None));
    assert_eq!(shown(&h), line.words);
}

#[test]
fn a_jump_to_a_typed_line_number_shows_the_text_box_it_falls_in() {
    let (mut h, _) = preview();
    h.act(Action::Confirm);
    open_line_box(&mut h);
    // Line 12 is the `@if`: it falls in the text box of line 11.
    h.type_text("12");
    h.act(Action::Confirm);
    let view = view(&h);
    assert!(!view.open);
    assert_eq!(view.playing.as_ref().map(|p| p.text_box), Some(2));
    let line = view
        .current_line()
        .unwrap_or_else(|| panic!("nothing plays"));
    assert_eq!(line.line, 11);
    assert!(shown(&h).starts_with("After each of your moves"));
}

#[test]
fn a_changed_file_is_played_on_the_next_frame_from_the_same_text_box() {
    let (mut h, folder) = preview();
    h.act(Action::Confirm);
    // The page shown whole, then the next text box.
    act(&mut h, &[Action::Confirm, Action::Confirm]);
    assert!(shown(&h).starts_with("Walk your piece"));
    let reworded = intro().replace("Walk your piece", "Walk your little piece");
    h.change_file(folder, "intro.dlg", reworded.into_bytes());
    h.wait(FRAME_DT);
    assert!(
        shown(&h).starts_with("Walk your little piece"),
        "{}",
        shown(&h)
    );
    assert_eq!(view(&h).playing.map(|p| p.text_box), Some(1));
    assert!(!h.ctx().debug_tools, "as a writer's optimized build");
}

#[test]
fn a_broken_file_keeps_the_last_good_scene_and_lists_its_problem_with_file_and_line() {
    let (mut h, folder) = preview();
    h.act(Action::Confirm);
    act(&mut h, &[Action::Confirm, Action::Confirm]);
    let broken = intro().replace("@endif\n", "");
    h.change_file(folder, "intro.dlg", broken.into_bytes());
    h.wait(FRAME_DT);
    assert!(
        shown(&h).starts_with("Walk your piece"),
        "the scene plays on"
    );
    let view = view(&h);
    assert!(view.scenes[0].problems);
    let problem = &view.problems[0];
    assert_eq!(problem.file, "intro.dlg");
    assert!(problem.line.is_some(), "{problem:?}");
    // Mended, the problem goes.
    h.change_file(folder, "intro.dlg", intro().into_bytes());
    h.wait(FRAME_DT);
    assert_eq!(self::view(&h).problems, []);
}

#[test]
fn the_tools_start_the_preview_over_the_games_scene() {
    let mut h = Start {
        preview: true,
        ..Start::default()
    }
    .harness();
    h.wait(FRAME_DT);
    assert_eq!(h.screens(), ["dialogue_preview"]);
    let scenes = view(&h).scenes;
    assert_eq!(scenes.len(), 1);
    assert_eq!(scenes[0].id, "intro");
}

#[test]
fn every_control_is_an_action_of_the_games() {
    let (mut h, _) = preview();
    h.act(Action::Confirm);
    // While the scene plays: the cursor, Confirm and Cancel; the text box
    // reads Confirm and Cancel too.
    assert_eq!(h.undeclared(), [] as [(&str, Action); 0]);
    let preview = h.screen::<DialoguePreview<Hello, HelloPreview>>();
    let mut declared = preview.and_then(Screen::actions).unwrap_or_default();
    declared.sort();
    let mut all = vec![
        Action::Up,
        Action::Down,
        Action::Left,
        Action::Right,
        Action::Confirm,
        Action::Cancel,
    ];
    all.sort();
    assert_eq!(declared, all);
}
