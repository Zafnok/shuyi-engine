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

    /// A module that draws over the toy's screens.
    struct Shade;

    impl shuyi_ui::module::Module<toy::Toy> for Shade {
        fn name(&self) -> &'static str {
            "shade"
        }

        fn install(&mut self, install: &mut shuyi_ui::module::Install<'_, toy::Toy>) {
            install.hook(shuyi_ui::module::Hook::DrawOver);
            install.state(1_u8);
            install.debug_entry(|| {
                shuyi_ui::debug::DebugEntry::new("Shade more", |_| shuyi_ui::Transition::None)
            });
        }
    }

    #[test]
    fn a_session_lists_its_games_modules() {
        assert_eq!(session().modules(), []);
        let game = toy::game().with_module(Shade).unwrap();
        let s = Headless::new(Harness::new(game));
        let expected = ModuleEntry {
            name: "shade".into(),
            hooks: vec!["draw_over".into()],
            states: vec!["u8".into()],
            debug_entries: vec!["Shade more".into()],
        };
        assert_eq!(s.modules(), [expected]);
        assert_eq!(Bare.modules(), []);
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

#[test]
fn a_new_scene_has_no_data() {
    let literal = Scene {
        screen: "stage".to_owned(),
        kind: "grid map".to_owned(),
        text: "@ (1,1)".to_owned(),
        data: serde_json::Value::Null,
    };
    assert_eq!(Scene::new("stage", "grid map", "@ (1,1)"), literal);
}

#[test]
fn a_session_that_doesnt_measure_reads_nothing_measured() {
    let mut bare = Bare;
    bare.set_perf(true);
    let read = bare.perf(7, 10);
    assert!(read.records.is_empty() && !read.measured);
    assert_eq!(read.next, 7);
}

/// Perf on the toy: this machine's clock, or the platform's own.
#[cfg(feature = "toy")]
mod perf {
    use shuyi_platform::headless::HeadlessPlatform;
    use shuyi_ui::toy::{self, Toy};

    use super::*;

    /// The toy, headless, on `platform`.
    fn toy_on(platform: HeadlessPlatform) -> Headless<Toy> {
        Headless::new(Harness::on(toy::game(), platform))
    }

    #[test]
    fn set_perf_measures_with_this_machines_clock_and_stops_measuring() {
        let mut session = toy_on(HeadlessPlatform::new());
        session.idle(2);
        assert!(!session.perf(0, 600).measured);
        session.set_perf(true);
        session.idle(3);
        let read = session.perf(0, 600);
        assert!(read.measured);
        assert_eq!(read.records.len(), 3);
        assert_eq!(read.records[0].frame, 3);
        session.set_perf(false);
        session.idle(2);
        assert_eq!(session.perf(0, 600).next, 3);
    }

    /// The test binary is a debug build with `shuyi-alloc-count`'s
    /// `global`: it counts its own heap. Other test threads allocate too,
    /// so only that a heap was read, never its values.
    #[test]
    fn set_perf_reads_this_programs_heap_and_stops_reading_it() {
        let mut session = toy_on(HeadlessPlatform::new());
        session.set_perf(true);
        assert_eq!(
            session.counts_process_heap(),
            shuyi_alloc_count::global::COUNTING
        );
        session.idle(2);
        let records = session.perf(0, 600).records;
        let heaps: Vec<_> = records.iter().map(|r| r.heap).collect();
        if shuyi_alloc_count::global::COUNTING {
            assert!(heaps.iter().all(|h| h.is_some_and(|h| h.peak_bytes > 0)));
        } else {
            assert_eq!(heaps, [None, None]);
        }
        session.set_perf(false);
        assert!(!session.counts_process_heap());
        assert!(!session.harness().platform().has_memory());
    }

    /// The headless backend with a fake clock and a fake heap.
    fn fakes() -> HeadlessPlatform {
        HeadlessPlatform::new()
            .with_clock(PerfClock::fake())
            .with_fake_memory()
    }

    #[test]
    fn set_perf_keeps_a_clock_and_a_heap_the_platform_already_has() {
        let mut session = toy_on(fakes());
        session.set_perf(true);
        assert!(!session.counts_process_heap());
        session.idle(2);
        let twice = |session: &Headless<_>| session.perf(0, 600).records;
        let first = twice(&session);
        let mut again = toy_on(fakes());
        again.set_perf(true);
        again.idle(2);
        assert_eq!(first, twice(&again));
        assert_eq!(first[0].times.update_us, 300);
        let heap = first[0].heap.unwrap();
        assert_eq!((heap.live_bytes, heap.allocs), (1_064_960, 1));
        // Its clock stays: `false` takes away only a clock it gave.
        session.set_perf(false);
        session.idle(1);
        assert_eq!(session.perf(0, 600).next, 3);
    }
}
