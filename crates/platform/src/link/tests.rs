//! The link's server on the toy game, on the headless backend: the wire's
//! lines, the hello, each step's frames, `press_for`, a play matching the
//! Harness's, the log by number, the picture, and the waits.

use alloc::collections::BTreeMap;

use proptest::prelude::*;
use shuyi_ui::input::{Key, RepeatDef};
use shuyi_ui::log::{LogCategory, LogLevel, LogLine};
use shuyi_ui::toy::{self, Toy};
use shuyi_ui::{Game, GameData};

use super::*;
use crate::harness::{FRAME_DT, Harness};
use crate::headless::HeadlessPlatform;
use crate::{Card, Look, Start, SystemEvent};

const TOKEN: &str = "0123456789abcdef0123456789abcdef";

/// A build of `app` on the headless backend with a server attached.
struct Build<G: GameData> {
    app: App<G>,
    platform: HeadlessPlatform,
    server: Server,
    /// Every answer sent, in order.
    sent: Vec<String>,
}

impl<G: GameData> Build<G> {
    fn new(app: App<G>) -> Self {
        let mut build = Self {
            app,
            platform: HeadlessPlatform::new(),
            server: Server::new(TOKEN.to_owned(), "Toy".to_owned()),
            sent: Vec::new(),
        };
        let hello = build.send(&hello(VERSION));
        assert!(matches!(hello, Handled::Answer(_)), "{hello:?}");
        build.sent.clear();
        build
    }

    /// Hands the server `request`, keeping an answer it gives now.
    fn send(&mut self, request: &str) -> Handled {
        let handled = self.server.handle(request, &self.app);
        if let Handled::Answer(answer) = &handled {
            self.sent.push(answer.clone());
        }
        handled
    }

    /// Runs one frame as a backend does: the server's input after the
    /// devices' (none), the frame, then the answers due.
    fn frame(&mut self) {
        let mut events = Vec::new();
        self.server.input(&self.app, &mut events);
        self.platform.push_input(&events);
        let _ = self.app.frame_after(&mut self.platform, FRAME_DT);
        let due = self.server.after_frame(&self.app);
        self.sent.extend(due);
    }

    /// Runs frames until an answer is sent (at most `MAX_FRAMES`), and
    /// takes it.
    fn answer(&mut self) -> Answer {
        for _ in 0..crate::harness::MAX_FRAMES {
            if !self.sent.is_empty() {
                return Answer::from_line(&self.sent.remove(0)).unwrap();
            }
            self.frame();
        }
        panic!("no answer");
    }

    /// Plays `steps` and waits for their answer.
    fn play(&mut self, steps: Vec<Step>) -> Answer {
        let _ = self.send(&Request::Play { steps }.to_line());
        self.answer()
    }

    fn game(&self) -> &Game<G> {
        self.app.game().unwrap()
    }
}

/// The toy game running as the Harness runs it: its screens get only the
/// actions they declare.
fn toy_build() -> Build<Toy> {
    build_of(toy::game())
}

/// `game` running as the Harness runs it.
fn build_of(mut game: Game<Toy>) -> Build<Toy> {
    game.guard_actions(true);
    Build::new(App::running(game))
}

fn hello(link: u32) -> String {
    Request::Hello {
        token: TOKEN.to_owned(),
        link,
    }
    .to_line()
}

fn keys(script: &str) -> Step {
    Step::Keys(script.to_owned())
}

fn pad(script: &str) -> Step {
    Step::Pad(script.to_owned())
}

fn act(name: &str) -> Step {
    Step::Act(name.to_owned())
}

#[test]
fn requests_and_answers_are_one_json_object_a_line() {
    let requests = [
        Request::Hello {
            token: "t".into(),
            link: 1,
        },
        Request::Status,
        Request::Frame,
        Request::Screenshot,
        Request::Play {
            steps: vec![
                keys("f Up"),
                pad("South+East"),
                act("Confirm"),
                Step::Wait(3),
            ],
        },
        Request::Log { since: 7 },
    ];
    for request in requests {
        let line = request.to_line();
        assert!(!line.contains('\n'), "{line}");
        assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), request);
    }
    assert_eq!(
        Request::Play {
            steps: vec![keys("f"), Step::Wait(2), act("Cancel")]
        }
        .to_line(),
        r#"{"op":"play","steps":[{"keys":"f"},{"wait":2},{"act":"Cancel"}]}"#
    );
    let read = |line: &str| serde_json::from_str::<Request>(line);
    assert_eq!(read(r#"{"op":"log"}"#).unwrap(), Request::Log { since: 0 });
    assert_eq!(read(r#"{"op":"status"}"#).unwrap(), Request::Status);
    assert!(read(r#"{"op":"hello","token":"t","link":1,"more":2}"#).is_err());
    assert!(read(r#"{"op":"reset"}"#).is_err());

    let answers = [
        Answer::Hello {
            link: 1,
            game: "Toy".into(),
        },
        Answer::Status(Status::of(&App::running(toy::game()))),
        Answer::Frame {
            frame: "ab\n".into(),
            limit: None,
        },
        Answer::Screenshot(Screenshot {
            png: "cG5n".into(),
            width: 2,
            height: 1,
            limit: Some("why".into()),
        }),
        Answer::Played(Played {
            frames: 4,
            waited_loading: true,
            waited_suspended: false,
        }),
        Answer::Log(log(&LogRing::new(2), 0)),
        Answer::Error { error: "no".into() },
        Answer::Busy,
    ];
    for answer in answers {
        let line = answer.to_line();
        assert!(!line.contains('\n'), "{line}");
        assert_eq!(Answer::from_line(&format!("{line}\n")).unwrap(), answer);
    }
    assert_eq!(Answer::Busy.to_line(), r#"{"answer":"busy"}"#);
    assert!(Answer::from_line("GET / HTTP/1.1").is_err());
}

#[test]
fn base64_is_rfc_4648s() {
    // RFC 4648 §10's test vectors.
    let vectors = [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ];
    for (plain, encoded) in vectors {
        assert_eq!(base64::encode(plain.as_bytes()), encoded);
        assert_eq!(base64::decode(encoded).unwrap(), plain.as_bytes());
    }
    let all: Vec<u8> = (0..=255).collect();
    assert_eq!(base64::decode(&base64::encode(&all)).unwrap(), all);
    for bad in ["Zg=", "Z===", "Zg==Zm9v", "Zm9*", "Zm 9"] {
        assert!(base64::decode(bad).is_err(), "{bad}");
    }
}

#[test]
fn the_first_line_must_be_a_hello_with_the_token_or_the_link_closes() {
    let app = App::running(toy::game());
    let server = || Server::new(TOKEN.to_owned(), "Toy".to_owned());
    let wrong = Request::Hello {
        token: "fedcba9876543210fedcba9876543210".into(),
        link: VERSION,
    }
    .to_line();
    let long = format!("{}{}", hello(VERSION), " ".repeat(MAX_LINE));
    for first in [
        wrong.as_str(),
        "GET / HTTP/1.1",
        "POST /rpc HTTP/1.1",
        r#"{"op":"status"}"#,
        "",
        long.as_str(),
    ] {
        let mut s = server();
        assert_eq!(s.handle(first, &app), Handled::Close(None), "{first:.40}");
    }
    // The right token and another version: the build's hello, then closed.
    let mut s = server();
    let Handled::Close(Some(line)) = s.handle(&hello(VERSION + 1), &app) else {
        panic!("not closed");
    };
    let built = Answer::Hello {
        link: VERSION,
        game: "Toy".into(),
    };
    assert_eq!(Answer::from_line(&line).unwrap(), built);
    // The right token and version: the hello's answer, then requests.
    let mut s = server();
    assert_eq!(
        s.handle(&hello(VERSION), &app),
        Handled::Answer(built.to_line())
    );
    let Handled::Answer(again) = s.handle(&hello(VERSION), &app) else {
        panic!("a second hello closed the link");
    };
    assert!(matches!(
        Answer::from_line(&again),
        Ok(Answer::Error { .. })
    ));
    let Handled::Answer(junk) = s.handle("not json", &app) else {
        panic!("junk after the hello closed the link");
    };
    assert!(matches!(Answer::from_line(&junk), Ok(Answer::Error { .. })));
    assert_eq!(s.handle(&long, &app), Handled::Close(None));
}

/// A keymap with Confirm on a key, Cancel on a button, Info on two buttons
/// together, and nothing for Menu.
fn keymap() -> Keymap<toy::Action> {
    use toy::Action;
    let pair = ButtonChord::pair(Button::South, Button::East).unwrap();
    Keymap::new(
        [(Chord::plain(Key::F), Action::Confirm)],
        RepeatDef::default(),
    )
    .with_pad([
        (ButtonChord::single(Button::East), Action::Cancel),
        (ButtonChord::single(Button::South), Action::Confirm),
        (pair, Action::Info),
    ])
}

#[test]
fn press_for_takes_the_primary_key_then_the_primary_button() {
    let keymap = keymap();
    let pair = ButtonChord::pair(Button::South, Button::East).unwrap();
    assert_eq!(
        press_for(&keymap, "Confirm"),
        Ok(Press::Keys(Chord::plain(Key::F)))
    );
    assert_eq!(
        press_for(&keymap, "Cancel"),
        Ok(Press::Pad(ButtonChord::single(Button::East)))
    );
    assert_eq!(press_for(&keymap, "Info"), Ok(Press::Pad(pair)));
    let neither = press_for(&keymap, "Menu").unwrap_err();
    assert!(
        neither.contains("Menu") && neither.contains("neither"),
        "{neither}"
    );
    let unknown = press_for(&keymap, "Jump").unwrap_err();
    assert!(
        unknown.contains("\"Jump\"") && unknown.contains("Confirm, Cancel"),
        "{unknown}"
    );
}

#[test]
fn each_step_becomes_frames_of_events() {
    let shift_space = Chord::shifted(Key::Space);
    let kind = PadKind::default();
    let frames = plan(
        &[
            keys("f Shift+Space"),
            pad("South East+DpadDown"),
            Step::Wait(0),
            Step::Wait(3),
            act("Info"),
            act("Confirm"),
        ],
        &keymap(),
    )
    .unwrap();
    let events = |e: &[RawInputEvent]| Planned::Events(e.to_vec());
    let expected = [
        events(&[RawInputEvent::Down(Chord::plain(Key::F))]),
        events(&[RawInputEvent::Up(Key::F)]),
        events(&[RawInputEvent::Down(shift_space)]),
        events(&[RawInputEvent::Up(Key::Space)]),
        events(&[RawInputEvent::PadDown(Button::South, kind)]),
        events(&[RawInputEvent::PadUp(Button::South)]),
        events(&[
            RawInputEvent::PadDown(Button::East, kind),
            RawInputEvent::PadDown(Button::DpadDown, kind),
        ]),
        events(&[
            RawInputEvent::PadUp(Button::East),
            RawInputEvent::PadUp(Button::DpadDown),
        ]),
        Planned::Idle(3),
        events(&[
            RawInputEvent::PadDown(Button::South, kind),
            RawInputEvent::PadDown(Button::East, kind),
        ]),
        events(&[
            RawInputEvent::PadUp(Button::South),
            RawInputEvent::PadUp(Button::East),
        ]),
        events(&[RawInputEvent::Down(Chord::plain(Key::F))]),
        events(&[RawInputEvent::Up(Key::F)]),
    ];
    assert_eq!(Vec::from(frames), expected);
    let error = plan(&[keys("f"), act("Menu")], &keymap()).unwrap_err();
    assert!(error.starts_with("step 2: act: Menu"), "{error}");
}

#[test]
fn a_step_that_cant_be_played_is_refused_at_once_naming_it() {
    let mut b = toy_build();
    for (steps, said) in [
        (vec![keys("f"), keys("f Nope")], "step 2: keys: unknown key"),
        (vec![pad("South+Nope")], "step 1: pad: "),
        (vec![act(" ")], "step 1: act: no action named"),
        (vec![Step::Wait(MAX_WAIT + 1)], "step 1: wait: at most"),
    ] {
        let Handled::Answer(line) = b.send(&Request::Play { steps }.to_line()) else {
            panic!("not answered at once");
        };
        let Answer::Error { error } = Answer::from_line(&line).unwrap() else {
            panic!("{line}");
        };
        assert!(error.starts_with(said), "{error}");
    }
    assert_eq!(b.app.frame_number(), 0);
}

/// `steps` played by the Harness, waits in whole frames of `FRAME_DT`.
fn harness_play(steps: &[Step]) -> Harness<Toy> {
    harness_play_on(toy::game(), steps)
}

/// `steps` played by the Harness on `game`.
fn harness_play_on(game: Game<Toy>, steps: &[Step]) -> Harness<Toy> {
    let mut h = Harness::new(game);
    for step in steps {
        match step {
            Step::Keys(script) => {
                h.keys(script);
            }
            Step::Pad(script) => {
                h.pad(script);
            }
            Step::Wait(n) => {
                for _ in 0..*n {
                    h.frame(&[], FRAME_DT);
                }
            }
            _ => unreachable!("the Harness has no actions by name"),
        }
    }
    h
}

/// Whether the server's play of `steps` ends as the Harness's does: the
/// same screens and frame text, in as many frames.
fn plays_as_the_harness(steps: &[Step]) {
    let h = harness_play(steps);
    let mut b = toy_build();
    let answer = b.play(steps.to_vec());
    let Answer::Played(played) = answer else {
        panic!("{answer:?}");
    };
    assert_eq!(played.frames, h.frame_count());
    assert_eq!(b.app.frame_number(), h.frame_count());
    assert_eq!(b.game().screens(), h.screens());
    let Answer::Frame { frame, limit } = frame(&b.app) else {
        unreachable!()
    };
    assert_eq!((frame, limit), (h.snapshot(), None));
}

#[test]
fn a_play_of_keys_buttons_and_waits_ends_as_the_harnesss() {
    plays_as_the_harness(&[keys("f"), Step::Wait(3), keys("Down f")]);
    plays_as_the_harness(&[pad("South"), Step::Wait(2), pad("DpadDown East")]);
    plays_as_the_harness(&[keys("f Down"), pad("South"), Step::Wait(5), keys("d")]);
}

/// A key or button step of the toy's, or a wait.
fn any_step() -> impl Strategy<Value = Step> {
    let key = prop::sample::select(vec![
        "f", "d", "j", "Up", "Down", "Left", "Right", "Space", "Enter", "Escape", "F2",
    ]);
    let button = prop::sample::select(vec![
        "South",
        "East",
        "North",
        "DpadUp",
        "DpadDown",
        "Start",
        "South+East",
    ]);
    prop_oneof![
        key.prop_map(keys),
        button.prop_map(pad),
        (0u32..6).prop_map(Step::Wait),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn any_script_plays_over_the_server_as_on_the_harness(
        steps in prop::collection::vec(any_step(), 0..10)
    ) {
        plays_as_the_harness(&steps);
    }
}

#[test]
fn act_presses_an_actions_key_or_its_button_and_names_one_it_cant() {
    // In the right-handed layout Confirm is on a key and Menu on nothing;
    // before a layout is picked, Cancel is on a button only.
    let keymap = toy::game().input().keymap().clone();
    let Ok(Press::Keys(confirm)) = press_for(&keymap, "Confirm") else {
        panic!("Confirm has no key");
    };
    assert!(press_for(&keymap, "Menu").is_err());
    let first = || toy::game_on(toy::first_launch());
    assert_eq!(
        press_for(first().input().keymap(), "Cancel"),
        Ok(Press::Pad(ButtonChord::single(Button::East)))
    );

    let mut b = toy_build();
    assert!(matches!(b.play(vec![act("Confirm")]), Answer::Played(_)));
    let h = harness_play(&[keys(confirm.key.name())]);
    assert_eq!(b.game().screens(), h.screens());
    assert_eq!(frame(&b.app), frame_of(&h));

    let mut b = build_of(first());
    assert!(matches!(b.play(vec![act("Cancel")]), Answer::Played(_)));
    let h = harness_play_on(first(), &[pad("East")]);
    assert_eq!(b.game().screens(), h.screens());
    assert_eq!(frame(&b.app), frame_of(&h));

    for (name, said) in [
        ("Menu", "neither a key nor a button"),
        ("Jump", "no action"),
    ] {
        let mut b = toy_build();
        let screens = b.game().screens();
        let Answer::Error { error } = b.play(vec![keys("f"), act(name)]) else {
            panic!("{name} played");
        };
        assert!(error.contains(name) && error.contains(said), "{error}");
        // Nothing of the play was played: one frame passed, with no input.
        assert_eq!(b.game().screens(), screens);
        assert_eq!(b.app.frame_number(), 1);
    }
}

/// The Harness's frame as the link answers it.
fn frame_of(h: &Harness<Toy>) -> Answer {
    Answer::Frame {
        frame: h.snapshot(),
        limit: None,
    }
}

#[test]
fn a_request_after_a_play_waits_for_its_answer() {
    let mut b = toy_build();
    assert_eq!(
        b.send(
            &Request::Play {
                steps: vec![keys("f")]
            }
            .to_line()
        ),
        Handled::Later
    );
    assert_eq!(b.send(&Request::Status.to_line()), Handled::Later);
    assert_eq!(b.send(&Request::Frame.to_line()), Handled::Later);
    assert!(matches!(
        b.answer(),
        Answer::Played(Played { frames: 2, .. })
    ));
    let Answer::Status(status) = b.answer() else {
        panic!("not the status");
    };
    assert_eq!(status.frames, 2);
    assert_eq!(status.screens, b.game().screens());
    assert!(matches!(b.answer(), Answer::Frame { limit: None, .. }));
    // A play of nothing is answered at once.
    let Handled::Answer(line) = b.send(&Request::Play { steps: vec![] }.to_line()) else {
        panic!("not answered");
    };
    assert!(matches!(
        Answer::from_line(&line),
        Ok(Answer::Played(Played { frames: 0, .. }))
    ));
    // One of no frames is answered after the next.
    let none = Request::Play {
        steps: vec![Step::Wait(0)],
    };
    assert_eq!(b.send(&none.to_line()), Handled::Later);
    assert!(matches!(
        b.answer(),
        Answer::Played(Played { frames: 0, .. })
    ));
}

#[test]
fn the_status_and_the_frame_say_where_the_build_is() {
    let mut b = toy_build();
    let _ = b.play(vec![keys("f")]);
    let status = Status::of(&b.app);
    let screens: Vec<String> = b.game().screens().iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(status.phase, Phase::Running);
    assert_eq!(status.top_screen, *screens.last().unwrap());
    assert_eq!(status.screens, screens);
    assert_eq!((status.frames, status.errors), (2, None));
    assert!(status.focused && !status.suspended && !status.quit);
    b.platform.push_system(SystemEvent::FocusLost);
    b.frame();
    assert!(!Status::of(&b.app).focused);
}

/// The toy's look, for a start: it draws with neither.
fn look(_: &Toy) -> Look<'_> {
    Look::new(&FONT, &IMAGES)
}

static FONT: shuyi_assets::font::FontAtlasDef = shuyi_assets::font::FontAtlasDef {
    cell_w: 0,
    cell_h: 0,
    columns: 0,
    glyphs: BTreeMap::new(),
};
static IMAGES: shuyi_assets::image::ImageTable =
    shuyi_assets::image::ImageTable::new(BTreeMap::new());

#[test]
fn the_steps_wait_while_the_game_loads_and_a_failed_load_ends_the_play() {
    let start = Start::new(Card::text("Toy"), look, |setup| {
        Ok(toy::game_on(setup.ctx(toy::ctx())))
    });
    let mut b = Build::new(App::new(start));
    assert_eq!(Status::of(&b.app).phase, Phase::Loading);
    let Answer::Frame { frame, limit } = frame(&b.app) else {
        unreachable!()
    };
    assert_eq!(frame, "");
    assert!(limit.unwrap().contains("loading card"));
    let _ = b.send(&Request::Screenshot.to_line());
    assert!(b.server.wants_screenshot());
    let line = b.server.screenshot(b"png", 2, 1);
    let Ok(Answer::Screenshot(shot)) = Answer::from_line(&line) else {
        panic!("{line}");
    };
    assert!(shot.limit.unwrap().contains("loading card"));
    let Answer::Played(played) = b.play(vec![Step::Wait(2)]) else {
        panic!("not played");
    };
    assert!(played.waited_loading && !played.waited_suspended);
    assert_eq!(played.frames, 2);
    // Two frames for the card and the load, then the play's two.
    assert_eq!(b.app.frame_number(), 4);

    let start: Start<Toy> = Start::new(Card::text("Toy"), look, |_| Err("no font".to_owned()));
    let mut b = Build::new(App::new(start));
    let Answer::Error { error } = b.play(vec![keys("f")]) else {
        panic!("a failed game played");
    };
    assert!(
        error.contains("didn't load") && error.contains("no font"),
        "{error}"
    );
    let status = Status::of(&b.app);
    assert_eq!(
        (status.phase, status.errors.as_deref()),
        (Phase::Failed, Some("no font"))
    );
}

#[test]
fn the_steps_wait_while_the_build_is_suspended() {
    let mut b = toy_build();
    b.platform.push_system(SystemEvent::Suspend);
    b.frame();
    assert!(b.app.is_suspended());
    let _ = b.send(
        &Request::Play {
            steps: vec![keys("f")],
        }
        .to_line(),
    );
    for _ in 0..5 {
        b.frame();
    }
    assert_eq!(b.sent, Vec::<String>::new());
    b.platform.push_system(SystemEvent::Resume);
    b.frame();
    let Answer::Played(played) = b.answer() else {
        panic!("not played");
    };
    assert!(played.waited_suspended && !played.waited_loading);
    assert_eq!(played.frames, 2);
    let h = harness_play(&[keys("f")]);
    assert_eq!(b.game().screens(), h.screens());
}

#[test]
fn a_screenshot_is_answered_after_the_frame_and_requests_behind_it_wait() {
    let mut b = toy_build();
    assert_eq!(b.send(&Request::Screenshot.to_line()), Handled::Screenshot);
    assert_eq!(b.send(&Request::Status.to_line()), Handled::Later);
    b.frame();
    assert!(b.sent.is_empty() && b.server.wants_screenshot());
    let line = b.server.screenshot(b"png", 2, 1);
    assert!(!b.server.wants_screenshot());
    assert_eq!(
        Answer::from_line(&line).unwrap(),
        Answer::Screenshot(Screenshot {
            png: "cG5n".into(),
            width: 2,
            height: 1,
            limit: None,
        })
    );
    assert!(matches!(b.answer(), Answer::Status(_)));
}

/// A ring keeping `capacity` lines, `pushed` pushed into it.
fn ring(capacity: usize, pushed: u32) -> LogRing {
    let mut ring = LogRing::new(capacity);
    for i in 0..pushed {
        ring.push(LogLine::new(
            i,
            LogLevel::Warn,
            LogCategory::Game,
            format!("line {i}"),
        ));
    }
    ring
}

#[test]
fn the_log_is_read_by_number_with_the_dropped_ones_counted() {
    let numbers = |answer: &LogAnswer| answer.lines.iter().map(|l| l.seq).collect::<Vec<_>>();
    // Nothing logged.
    let empty = log(&ring(3, 0), 0);
    assert_eq!((empty.lines.len(), empty.next, empty.dropped), (0, 0, 0));
    // Two kept of two: 0 gives both, numbered.
    let two = log(&ring(3, 2), 0);
    assert_eq!((numbers(&two), two.next, two.dropped), (vec![0, 1], 2, 0));
    let first = &two.lines[0];
    assert_eq!(
        (
            first.frame,
            first.level.as_str(),
            first.category.as_str(),
            first.text.as_str()
        ),
        (0, "warn", "game", "line 0")
    );
    // `next` gives none until a line is pushed, then that one.
    assert_eq!(log(&ring(3, 2), two.next).lines, []);
    assert_eq!(numbers(&log(&ring(3, 3), two.next)), vec![2]);
    // Five pushed, three kept: 0..2 are gone.
    let five = log(&ring(3, 5), 0);
    assert_eq!(
        (numbers(&five), five.next, five.dropped),
        (vec![2, 3, 4], 5, 2)
    );
    assert_eq!(five.lines[0].text, "line 2");
    let from_one = log(&ring(3, 5), 1);
    assert_eq!((numbers(&from_one), from_one.dropped), (vec![2, 3, 4], 1));
    let from_three = log(&ring(3, 5), 3);
    assert_eq!((numbers(&from_three), from_three.dropped), (vec![3, 4], 0));
    let later = log(&ring(3, 5), 9);
    assert_eq!((later.lines.len(), later.next, later.dropped), (0, 5, 0));
}

#[test]
fn the_log_request_reads_the_apps_ring() {
    let mut b = toy_build();
    let ctx = b.app.game_mut().unwrap().ctx_mut();
    ctx.log(LogLevel::Warn, LogCategory::Game, "hello");
    b.frame();
    let _ = b.send(&Request::Log { since: 0 }.to_line());
    let Answer::Log(answer) = b.answer() else {
        panic!("not the log");
    };
    let line = answer.lines.last().unwrap();
    assert_eq!((line.frame, line.text.as_str()), (1, "hello"));
    assert_eq!(answer.next, b.app.log().next_seq());
    assert_eq!(line.seq, answer.next - 1);
    let _ = b.send(&Request::Log { since: answer.next }.to_line());
    let Answer::Log(after) = b.answer() else {
        panic!("not the log");
    };
    assert_eq!(after.lines, []);
}
