//! The game played through the engine's Harness, and through its tools
//! (the tool contract). Key names here are the Harness's test-script
//! names, input to a test, not game code.

use std::path::Path;

use game::title::Title;
use shuyi::prelude::audio::{AudioConfig, format_problem, playable};
use shuyi::prelude::test::{Harness, Stamps};
use shuyi::prelude::text::{StyledText, TextRole};
use shuyi::prelude::{Cell, GameData, GlyphBuffer, Rgb, Screen, UiAction};
use shuyi::ui::frame::color::KEY;
use shuyi_cli::prelude::{
    BasicTools, CheckInput, Event, GameTools, PlaytestInput, Project, RunInput, check,
    engine_profiles, keys, run, targets,
};

/// The engine's tools for the game: `game::stages::tools` gives its
/// stages' own on top of them.
fn tools() -> BasicTools<UiAction, game::stages::Own> {
    BasicTools::new(game::game())
}

#[test]
fn the_title_shows_the_games_name_and_cancel_leaves() {
    let mut h = game::game().harness();
    assert_eq!(h.screens(), ["title"]);
    let view = Title::view(h.ctx());
    assert!(h.snapshot().contains(&view.title), "{}", h.snapshot());
    assert!(!view.help.is_empty(), "{:?}", view.help);
    assert_eq!(view.play.is_some(), !game::stages::names().is_empty());
    h.keys("Backspace");
    assert!(h.quit_requested());
}

/// The text of `line`'s key spans, and of the cells of row `y` of `buf`
/// painted in `key`: the same, if the line's keys stand out.
fn keys_and_key_cells(line: &StyledText, buf: &GlyphBuffer, y: i32, key: Rgb) -> (String, String) {
    let keys = line
        .spans()
        .iter()
        .filter(|span| matches!(span.role, Some(TextRole::Key | TextRole::Arrows)))
        .map(|span| span.text.as_str())
        .collect();
    let cells = (0..i32::from(buf.width()))
        .filter_map(|x| buf.get(x, y))
        .filter(|cell| cell.fg == key)
        .map(|cell| cell.glyph)
        .collect();
    (keys, cells)
}

#[test]
fn the_titles_help_and_play_paint_their_keys_in_the_palettes_key_colour() {
    let h = game::game().harness();
    let ctx = h.ctx();
    let key = ctx.palette.lookup(KEY).expect("the palette names `key`");
    let view = Title::view(ctx);
    let black = Rgb::new(0, 0, 0);
    let mut buf = GlyphBuffer::new(60, 24, Cell::new(' ', black, black));
    Title.draw(ctx, &mut buf);
    let (keys, cells) = keys_and_key_cells(&view.help, &buf, 23, key);
    assert!(!keys.is_empty(), "{:?}", view.help);
    assert_eq!(cells, keys);
    if let Some(play) = &view.play {
        let (keys, cells) = keys_and_key_cells(play, &buf, 15, key);
        assert!(!keys.is_empty(), "{play:?}");
        assert_eq!(cells, keys);
    }
}

#[test]
fn the_title_quits_only_where_the_game_can() {
    let profiles = engine_profiles().unwrap();
    let mut h = game::game().harness().as_target(&profiles["web"].profile);
    assert!(Title::view(h.ctx()).help.is_empty());
    h.keys("Backspace");
    assert!(!h.quit_requested());
    assert_eq!(h.screens(), ["title"]);

    let mut h = game::game()
        .harness()
        .as_target(&profiles["desktop"].profile);
    assert!(!Title::view(h.ctx()).help.is_empty());
    h.keys("Backspace");
    assert!(h.quit_requested());
}

#[test]
fn confirm_starts_the_first_stage_if_there_is_one() {
    let mut h = game::game().harness();
    h.keys("Enter");
    match game::stages::names().first() {
        Some(_) => assert_eq!(h.screens().len(), 2, "{:?}", h.screens()),
        None => assert_eq!(h.screens(), ["title"]),
    }
}

#[test]
fn every_sound_the_menus_play_is_in_the_manifest_and_plays() {
    let data = game::game().data().unwrap_or_else(|e| panic!("{e:?}"));
    let bundle = game::game().bundle();
    let config = AudioConfig::DEFAULT;
    let cues = [
        config.move_cue,
        config.select_cue,
        config.cancel_cue,
        config.denied_cue,
        config.cursor_cue,
    ];
    for cue in cues {
        let sound = data.audio_manifest().sounds.get(cue);
        let sound = sound.unwrap_or_else(|| panic!("files/audio.ron has no cue `{cue}`"));
        assert!(!sound.files.is_empty(), "{cue}");
        for file in &sound.files {
            // What the platform plays: a sound effect rendered to a WAV.
            let bytes = playable(file, bundle.bytes(file));
            let bytes = bytes.unwrap_or_else(|| panic!("{cue}: no file {file}"));
            assert_eq!(format_problem(file, &bytes), None, "{cue}: {file}");
        }
    }
}

#[test]
fn check_finds_no_error_in_the_files() {
    let answer = check(&tools(), &CheckInput::default());
    assert!(answer.ok, "{answer:?}");
    let answer = check(&game::stages::tools(), &CheckInput::default());
    assert!(answer.ok, "{answer:?}");
    // The text is checked against the font atlas too.
    assert!(
        !answer.limits.iter().any(|l| l.contains("font atlas")),
        "{:?}",
        answer.limits
    );
}

#[test]
fn run_says_the_game_quit_and_the_title_has_no_scene() {
    let mut input = RunInput::default();
    input.script = keys("Backspace");
    input.frames = 3;
    let answer = run(&tools(), &input);
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.as_ref().unwrap();
    assert_eq!(out.quit, Some(1));
    assert!(out.events.contains(&Event::Quit { frame: 1 }));
    assert!(out.scene.is_none());
    assert!(answer.limits.iter().any(|l| l.starts_with("no scene")));
    assert!(
        tools()
            .playtest_plan(None, &PlaytestInput::default())
            .is_err()
    );
}

#[test]
fn every_screen_declares_its_actions_and_they_fit_every_target() {
    let game = game::game();
    let mut ctx = game.ctx().unwrap_or_else(|e| panic!("{e}"));
    let screens = game.every_screen(&mut ctx);
    let names: Vec<&str> = screens.iter().map(|s| s.name()).collect();
    assert_eq!(names.first(), Some(&"title"), "{names:?}");
    assert!(screens.iter().all(|s| s.actions().is_some()));
    // The targets shuyi.ron lists, as `cargo shuyi check --target all`
    // checks them.
    let tools = game::stages::tools();
    let controls = tools.controls().unwrap().unwrap_or_else(|e| panic!("{e}"));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = Project::load(&root).unwrap_or_else(|e| panic!("{e}"));
    let profiles = engine_profiles().unwrap_or_else(|e| panic!("{e:?}"));
    assert!(
        !project.config.targets.is_empty(),
        "{:?}",
        project.config.targets
    );
    for id in project.config.targets.keys() {
        assert_eq!(controls.fit(&profiles[id].profile), [], "{id}");
    }
    let answer = targets(&project.config, &project.root, &all_targets(), Some(&tools));
    assert!(answer.ok, "{answer:?}");
    assert!(
        !answer.limits.iter().any(|l| l.contains("controls aren't")),
        "{:?}",
        answer.limits
    );
}

#[test]
fn a_session_recorded_from_the_real_start_plays_through_the_tools() {
    let mut h = Harness::start_recording(game::game().start()).unwrap_or_else(|e| panic!("{e}"));
    h.keys("Enter Down Up").wait(0.1);
    let record = h.session().unwrap_or_else(|| panic!("nothing recorded"));
    let mut session = game::stages::tools()
        .session_from(record, Stamps::Check)
        .unwrap_or_else(|e| panic!("{e}"));
    let played = session.play_record().unwrap_or_else(|| panic!("no record"));
    assert_eq!(played.first_difference, None);
    assert_eq!(played.frames, played.of);
    assert_eq!(session.screens(), h.screens());
}

/// `check --target all`'s input.
fn all_targets() -> CheckInput {
    let mut input = CheckInput::default();
    input.target = Some("all".to_owned());
    input
}
