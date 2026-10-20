//! What a session reports of a game: here the toy's.

use super::*;

#[test]
fn a_change_of_stack_is_what_closed_from_the_top_then_what_opened() {
    let strings = |items: &[&str]| -> Vec<String> { items.iter().map(|s| (*s).into()).collect() };
    assert_eq!(
        changes(&["a", "b", "c"], &["a", "d"]),
        (strings(&["c", "b"]), strings(&["d"]))
    );
    assert_eq!(
        changes(&["a"], &["a", "b", "c"]),
        (Vec::new(), strings(&["b", "c"]))
    );
    assert_eq!(changes(&["a"], &["a"]), (Vec::new(), Vec::new()));
    assert_eq!(changes(&["a"], &[]), (strings(&["a"]), Vec::new()));
}

#[test]
fn a_music_command_is_named_with_its_cue() {
    let cue = || "theme".to_owned();
    assert_eq!(music(&MusicCommand::Load { cue: cue() }), ("load", "theme"));
    assert_eq!(
        music(&MusicCommand::Start { cue: cue() }),
        ("start", "theme")
    );
    assert_eq!(
        music(&MusicCommand::Gain {
            cue: cue(),
            gain: 0.5
        }),
        ("gain", "theme")
    );
    assert_eq!(music(&MusicCommand::Stop { cue: cue() }), ("stop", "theme"));
}

#[cfg(feature = "toy")]
mod toy {
    use shuyi_ui::toy;

    use super::*;
    use crate::frame_png::Step;

    fn session() -> Headless<toy::Toy> {
        Headless::new(Harness::new(toy::game()))
    }

    #[test]
    fn a_session_plays_scripts_and_idles() {
        let mut s = session();
        s.play(&Script {
            steps: vec![Step::Keys("f f".into()), Step::Wait(0.05)],
        });
        assert_eq!(s.frame_count(), 7);
        s.idle(3);
        assert_eq!(s.frame_count(), 10);
        assert_eq!(s.screens(), ["home", "menu"]);
        assert_eq!(s.snapshot(), s.harness().snapshot());
        assert_eq!(
            s.sounds(),
            [Sound {
                frame: 3,
                cue: "click".into()
            }]
        );
        assert_eq!(s.quit_frame(), None);
    }

    #[test]
    fn its_events_are_the_screens_the_quit_and_the_games_own() {
        let mut s = session().with_events(|h| vec![format!("{} frames", h.frame_count())]);
        s.play(&Script {
            steps: vec![Step::Keys("f d d".into())],
        });
        assert_eq!(s.quit_frame(), Some(5));
        assert_eq!(
            s.events(),
            [
                Event::Opened {
                    frame: 1,
                    screen: "menu".into()
                },
                Event::Closed {
                    frame: 3,
                    screen: "menu".into()
                },
                Event::Quit { frame: 5 },
                Event::Game {
                    text: "6 frames".into()
                },
            ]
        );
    }

    #[test]
    fn a_session_has_no_scene_unless_the_game_gives_one() {
        let s = session();
        assert_eq!(
            s.scene().unwrap_err(),
            "the game's tools give no scene (the top screen is `home`)"
        );
        let s = session().with_scene(|h| {
            Ok(Scene {
                screen: h.top_screen().to_owned(),
                kind: "toy".into(),
                text: "H".into(),
                data: serde_json::Value::Null,
            })
        });
        assert_eq!(s.scene().unwrap().screen, "home");
    }
}
