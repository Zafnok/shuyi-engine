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

/// A session that says only what a session must.
struct Bare;

impl Session for Bare {
    fn play(&mut self, _: &Script) {}
    fn idle(&mut self, _: u32) {}
    fn frame_count(&self) -> u32 {
        0
    }
    fn screens(&self) -> Vec<String> {
        Vec::new()
    }
    fn quit_frame(&self) -> Option<u32> {
        None
    }
    fn snapshot(&self) -> String {
        String::new()
    }
    fn events(&self) -> Vec<Event> {
        Vec::new()
    }
    fn sounds(&self) -> Vec<Sound> {
        Vec::new()
    }
    fn scene(&self) -> Result<Scene, String> {
        Err("none".to_owned())
    }
}

#[test]
fn a_session_that_says_nothing_of_its_log_has_none() {
    let found = Bare.log();
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(Bare.log_dropped(), 0);
}

#[test]
fn a_session_that_paints_nothing_says_the_game_offers_no_picture() {
    let error = Bare.picture(Some(2)).unwrap_err();
    assert_eq!(
        error,
        "the game's tools don't offer a picture of a session's frame (GameTools::session, with Headless::with_picture)"
    );
    assert!(crate::game::is_unsupported(&error));
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

    #[test]
    fn a_session_paints_its_frame_if_the_game_says_how() {
        assert!(session().picture(None).is_err());
        let s = session().with_picture(|h, scale| {
            let side = scale.unwrap_or(1) * h.frame_count().max(1);
            Ok(Image {
                width: side,
                height: 1,
                rgba: vec![0; side as usize * 4],
            })
        });
        assert_eq!(s.picture(Some(3)).unwrap().width, 3);
        assert_eq!(s.picture(None).unwrap().width, 1);
    }

    #[test]
    fn a_session_logs_what_its_harness_logs() {
        use shuyi_ui::profiles::ProfileId;
        use shuyi_ui::settings::SETTINGS_KEY;
        use shuyi_ui::storage::{MemoryStorage, Storage};

        let mut storage = MemoryStorage::new();
        let key = ProfileId::FIRST.key(SETTINGS_KEY);
        storage.write(&key, "nonsense").unwrap();
        let ctx = toy::launch(Box::new(storage));
        let s = Headless::new(Harness::new(toy::game_on(ctx)));
        let log = s.log();
        assert_eq!(log.len(), 1, "{log:?}");
        let entry = &log[0];
        assert_eq!(
            (entry.frame, entry.level.as_str(), entry.category.as_str()),
            (0, "warn", "storage")
        );
        assert!(
            entry.text.starts_with("settings: unreadable: "),
            "{entry:?}"
        );
        assert_eq!(s.log_dropped(), 0);
        let found = session().log();
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_session_counts_the_lines_its_log_dropped() {
        use shuyi_ui::log::{LOG_LINES, LogCategory, LogLevel};

        let mut h = Harness::new(toy::game());
        for n in 0..=LOG_LINES {
            h.ctx_mut()
                .log(LogLevel::Info, LogCategory::Game, format!("{n}"));
        }
        h.frame(&[], FRAME_DT);
        let s = Headless::new(h);
        assert_eq!(s.log_dropped(), 1);
        assert_eq!(s.log().len(), LOG_LINES);
    }
}
