//! The frame loop, driven like a platform drives it, on the toy game.

use super::*;
use alloc::rc::Rc;
use core::cell::RefCell;

use shuyi_assets::keymap::StickDef;

use crate::frame::color::EngineColor;
use crate::frame::glyph_buffer::Cell;
use crate::input::state::Device;
use crate::screen::{ScreenResult, Transition};
use crate::toy::{
    Action, Home, LEFT_HANDED, Menu, RIGHT_HANDED, Toy, ToyCtx, Voice, ctx, first_launch,
};

fn down(key: Key) -> RawInputEvent {
    RawInputEvent::Down(Chord::plain(key))
}

fn tap(game: &mut Game<Toy>, key: Key) -> bool {
    let quit = game.frame(&[down(key)], 0.0).quit;
    game.frame(&[RawInputEvent::Up(key)], 0.0);
    quit
}

fn pad_tap(game: &mut Game<Toy>, button: Button) -> bool {
    let quit = game
        .frame(&[RawInputEvent::PadDown(button, PadKind::Xbox)], 0.0)
        .quit;
    game.frame(&[RawInputEvent::PadUp(button)], 0.0);
    quit
}

/// The toy at its [`Home`], with no hooks.
fn home(ctx: ToyCtx) -> Game<Toy> {
    Game::new(ctx, Box::new(Home::default()), GameHooks::default())
}

/// Ticket 1109: a game of two screens, from the engine alone. Confirm
/// opens the menu over the home screen, Cancel closes it, Cancel again
/// quits; after that, frames do nothing.
#[test]
fn a_toy_game_of_two_screens_pushes_pops_and_quits() {
    let mut game = home(ctx());
    assert_eq!(game.screens(), ["home"]);
    assert_eq!(game.buffer().get(0, 0).map(|c| c.glyph), Some('H'));
    assert!(!tap(&mut game, Key::F));
    assert_eq!(game.screens(), ["home", "menu"]);
    assert_eq!(game.top_screen(), Some("menu"));
    // The menu is an overlay: the home screen shows under it.
    let row: String = (0..2)
        .filter_map(|x| game.buffer().get(x, 0))
        .map(|c| c.glyph)
        .collect();
    assert_eq!(row, "HM");
    // Its Confirm clicks, in that frame's output.
    let out = game.frame(&[down(Key::F)], 0.0);
    let click = AudioRequest::PlaySound {
        cue: "click".into(),
        volume: 1.0,
    };
    assert_eq!(out.audio, [click]);
    game.frame(&[RawInputEvent::Up(Key::F)], 0.0);
    assert!(!tap(&mut game, Key::D));
    assert_eq!(game.screens(), ["home"]);
    assert_eq!(game.buffer().get(1, 0).map(|c| c.glyph), Some(' '));
    assert!(!game.quit_requested());
    assert!(tap(&mut game, Key::D));
    assert!(game.quit_requested());
    // Quit: the frames that follow do nothing, and say so.
    let before = game.buffer().clone();
    // Only the top screen updates: home saw the press that opened the
    // menu, the release after the menu closed, and the press that quit.
    let frames = game.screen::<Home>().map(|h| h.frames);
    assert_eq!(frames, Some(3));
    assert!(
        game.frame(&[RawInputEvent::Up(Key::D), down(Key::F)], 0.0)
            .quit
    );
    assert_eq!(game.screens(), ["home"]);
    assert_eq!(game.buffer(), &before);
    assert_eq!(game.screen::<Home>().map(|h| h.frames), frames);
}

/// A test reaches into a screen on the stack by its type, the top-most
/// first; a screen that doesn't opt in is never found.
#[test]
fn a_screen_on_the_stack_is_found_by_its_type() {
    let mut game = home(ctx());
    tap(&mut game, Key::F);
    assert_eq!(game.screens(), ["home", "menu"]);
    if let Some(home) = game.screen_mut::<Home>() {
        home.frames = 40;
    }
    assert_eq!(game.screen::<Home>().map(|h| h.frames), Some(40));
    assert!(game.screen::<Menu>().is_none());
    assert!(game.screen_mut::<Menu>().is_none());
}

#[test]
fn popping_the_last_screen_quits() {
    let mut game = Game::new(ctx(), Box::new(Menu), GameHooks::default());
    assert!(tap(&mut game, Key::D));
    assert_eq!(game.top_screen(), None);
    let got = game.screens();
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn a_new_game_is_drawn_on_the_games_console_cleared_to_its_blank() {
    let game = home(ctx());
    let buffer = game.buffer();
    assert_eq!(
        buffer.spec(),
        crate::frame::console::ConsoleSpec::text(40, 20)
    );
    // The default blank: text on panel.
    let palette = &game.ctx().palette;
    let blank = Cell::new(
        ' ',
        palette.engine(EngineColor::Text),
        palette.engine(EngineColor::Panel),
    );
    assert_eq!(buffer.get(5, 5), Some(&blank));
    assert_eq!(game.ctx().game.blank_cell(palette), blank);
}

#[test]
fn a_redraw_shows_what_changed_outside_the_loop() {
    let mut game = home(ctx());
    game.ctx_mut().palette = crate::frame::color::Palette::new::<EngineColor>(
        game.ctx().game.theme_def(crate::toy::Theme::Night).unwrap(),
    )
    .unwrap();
    let night = game.ctx().palette.engine(EngineColor::Panel);
    assert_ne!(game.buffer().get(5, 5).map(|c| c.bg), Some(night));
    game.redraw();
    assert_eq!(game.buffer().get(5, 5).map(|c| c.bg), Some(night));
}

/// What a [`Spy`] saw, frame by frame.
type Seen = Rc<RefCell<Vec<FrameInput<Action>>>>;

/// Records what the screen saw.
struct Spy(Seen);

impl Screen<Toy> for Spy {
    fn name(&self) -> &'static str {
        "spy"
    }
    fn update(&mut self, _: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        self.0.borrow_mut().push(input.clone());
        Transition::None
    }
    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

fn spied() -> (Game<Toy>, Seen) {
    let seen = Rc::default();
    let game = Game::new(ctx(), Box::new(Spy(Rc::clone(&seen))), GameHooks::default());
    (game, seen)
}

#[test]
fn screens_see_actions_dt_and_held_keys() {
    let (mut game, seen) = spied();
    game.frame(&[down(Key::Right), down(Key::F)], 0.1);
    game.frame(&[RawInputEvent::Up(Key::F)], 0.0);
    let seen = seen.borrow();
    assert_eq!(seen[0].actions, [Action::CursorRight, Action::Confirm]);
    assert!((seen[0].dt() - 0.1).abs() < f32::EPSILON);
    assert!(seen[0].is_held(Action::CursorRight));
    assert!(seen[0].is_held(Action::Confirm));
    let got = &seen[1].actions;
    assert!(got.is_empty(), "{got:?}");
    assert!(seen[1].is_held(Action::CursorRight));
    assert!(!seen[1].is_held(Action::Confirm));
    assert!(game.input().is_held(Action::CursorRight));
    // Every press of the frame, in order; releases aren't presses.
    assert_eq!(
        seen[0].pressed_chords(),
        [Chord::plain(Key::Right), Chord::plain(Key::F)]
    );
    let got = seen[1].pressed_chords();
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn the_steps_due_reach_the_next_frame_only() {
    let (mut game, seen) = spied();
    game.frame(&[], 0.1);
    game.set_ticks(3);
    game.frame(&[], 0.05);
    // Told for one frame: the next has none unless told again.
    game.frame(&[], 0.0);
    game.set_ticks(1);
    // An update from outside the keys passes no time and no steps.
    game.act(Action::Confirm);
    game.frame(&[], 0.0);
    let ticks: Vec<u32> = seen.borrow().iter().map(FrameInput::ticks).collect();
    assert_eq!(ticks, [0, 3, 0, 0, 1]);
}

#[test]
fn screens_see_the_keys_pressed_and_the_text_typed() {
    let (mut game, seen) = spied();
    let events = [
        down(Key::A),
        RawInputEvent::Text('a'),
        RawInputEvent::Down(Chord::shifted(Key::Z)),
        // Control characters some platforms send for Backspace or Enter
        // are keys, not text.
        RawInputEvent::Text('\u{8}'),
        RawInputEvent::Text('\r'),
        RawInputEvent::Text('é'),
    ];
    game.frame(&events, 0.0);
    game.frame(&[RawInputEvent::Up(Key::A)], 0.0);
    let seen = seen.borrow();
    let pressed = [Chord::plain(Key::A), Chord::shifted(Key::Z)];
    assert_eq!(seen[0].pressed_chords(), pressed);
    assert_eq!(seen[0].text(), ['a', 'é']);
    let got = seen[1].pressed_chords();
    assert!(got.is_empty(), "{got:?}");
    let got = seen[1].text();
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn screens_see_the_buttons_that_went_down_and_up() {
    let (mut game, seen) = spied();
    let press = |b| RawInputEvent::PadDown(b, PadKind::Xbox);
    game.frame(&[press(Button::RightStickUp), press(Button::South)], 0.0);
    let ups = [
        RawInputEvent::PadUp(Button::South),
        RawInputEvent::PadUp(Button::RightStickUp),
    ];
    game.frame(&ups, 0.0);
    let seen = seen.borrow();
    assert_eq!(
        seen[0].pressed_buttons(),
        [Button::RightStickUp, Button::South]
    );
    assert_eq!(seen[0].actions, [Action::Confirm]);
    assert!(seen[0].is_held(Action::Confirm));
    let got = seen[0].released_buttons();
    assert!(got.is_empty(), "{got:?}");
    assert_eq!(
        seen[1].released_buttons(),
        [Button::South, Button::RightStickUp]
    );
    assert!(!seen[1].is_held(Action::Confirm));
}

/// Input is a controller's when a button went down, bound or not, or when
/// a controller gives an action with no button going down (a held
/// direction's repeat, a chord button's own action as it is let go); an
/// empty frame, or a key's action, is not.
#[test]
fn a_frame_is_pad_input_when_a_button_went_down_or_a_pad_acted() {
    let (mut game, seen) = spied();
    let press = |b| RawInputEvent::PadDown(b, PadKind::Xbox);
    // An unbound button: no action, but a button went down.
    game.frame(&[press(Button::RightStickPress)], 0.0);
    game.frame(&[RawInputEvent::PadUp(Button::RightStickPress)], 0.0);
    // Held down on the D-pad: it repeats with no button going down.
    game.frame(&[press(Button::DpadDown)], 0.0);
    game.frame(&[], 0.5);
    game.frame(&[RawInputEvent::PadUp(Button::DpadDown)], 0.0);
    // Held down on the keyboard: its repeat is the keyboard's.
    game.frame(&[down(Key::Down)], 0.0);
    game.frame(&[], 0.5);
    let seen = seen.borrow();
    let pad: Vec<bool> = seen.iter().map(FrameInput::pad_pressed).collect();
    let acted: Vec<bool> = seen.iter().map(|i| !i.actions.is_empty()).collect();
    assert_eq!(acted, [false, false, true, true, false, true, true]);
    assert_eq!(pad, [true, false, true, true, false, false, false]);
}

/// Screens find what was pressed last in `ctx.device`, already in the
/// frame of the press; a game can start on a pad.
#[test]
fn the_context_knows_the_device_pressed_last() {
    let mut game = home(ctx());
    assert_eq!(game.ctx().device, Device::Keyboard);
    let sony = PadKind::PlayStation;
    game.frame(&[RawInputEvent::PadDown(Button::DpadDown, sony)], 0.0);
    assert_eq!(game.ctx().device, Device::Pad(sony));
    game.frame(&[down(Key::Up)], 0.0);
    assert_eq!(game.ctx().device, Device::Keyboard);
    let mut on_pad = ctx();
    on_pad.device = Device::Pad(PadKind::Nintendo);
    let mut game = home(on_pad);
    game.frame(&[], 0.0);
    assert_eq!(game.ctx().device, Device::Pad(PadKind::Nintendo));
}

/// A key or a button press ends the wait for one; a release doesn't, and
/// a game started without the wait never waits.
#[test]
fn a_press_moves_the_key_prompt_on() {
    for press in [
        down(Key::Z),
        RawInputEvent::PadDown(Button::RightStickPress, PadKind::Xbox),
    ] {
        let mut waiting = ctx();
        waiting.key_prompt = KeyPrompt::Waiting;
        let mut game = home(waiting);
        game.frame(
            &[
                RawInputEvent::Up(Key::Z),
                RawInputEvent::PadUp(Button::South),
            ],
            0.0,
        );
        assert_eq!(game.ctx().key_prompt, KeyPrompt::Waiting);
        game.frame(&[press], 0.0);
        assert_eq!(game.ctx().key_prompt, KeyPrompt::Pressed);
    }
    let mut game = home(ctx());
    game.frame(&[down(Key::Z)], 0.0);
    assert_eq!(game.ctx().key_prompt, KeyPrompt::Off);
}

#[test]
fn the_clock_adds_up_frame_times() {
    let mut game = home(ctx());
    game.frame(&[], 0.5);
    game.frame(&[], 0.25);
    assert!((game.ctx().clock_s - 0.75).abs() < 1e-9);
    // Nonsense frame times don't count.
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        game.frame(&[], dt);
    }
    assert!((game.ctx().clock_s - 0.75).abs() < 1e-9);
}

#[test]
fn rebinding_keys_reaches_the_input_before_the_next_press() {
    let mut game = home(ctx());
    let mut b = game.ctx().layout_bindings(RIGHT_HANDED);
    assert_eq!(b.bind(Action::Confirm, 0, Chord::plain(Key::G)), Ok(None));
    game.ctx_mut().set_layout_bindings(RIGHT_HANDED, b).unwrap();
    // The old key does nothing; the new one confirms at once.
    tap(&mut game, Key::F);
    assert_eq!(game.screens(), ["home"]);
    tap(&mut game, Key::G);
    assert_eq!(game.screens(), ["home", "menu"]);
    assert_eq!(game.input().keymap(), &game.ctx().keymap);
}

/// The toy's layout picker: Confirm picks the right-handed layout, Cancel
/// or any controller button closes it without choosing.
struct Picker;

impl Screen<Toy> for Picker {
    fn name(&self) -> &'static str {
        "picker"
    }
    fn update(&mut self, ctx: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        if input.pad_pressed() {
            return Transition::Pop;
        }
        if input.actions.contains(&Action::Confirm) {
            let _ = ctx.choose_layout(RIGHT_HANDED);
            return Transition::Pop;
        }
        Transition::None
    }
    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
    fn is_overlay(&self) -> bool {
        true
    }
}

/// A debug tool: Cancel closes it.
struct Tool(&'static str);

impl Screen<Toy> for Tool {
    fn name(&self) -> &'static str {
        self.0
    }
    fn update(&mut self, _: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        if input.actions.contains(&Action::Cancel) {
            Transition::Pop
        } else if input.actions.contains(&Action::Confirm) {
            Transition::Push(Box::new(Tool("inspector")))
        } else {
            Transition::None
        }
    }
    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

fn hooks() -> GameHooks<Toy> {
    GameHooks {
        first_key: Some(ScreenHook {
            name: "picker",
            open: |_| Box::new(Picker),
        }),
        debug: Some(ScreenHook {
            name: "tools",
            open: |_| Box::new(Tool("tools")),
        }),
        debug_screens: &["tools", "inspector"],
    }
}

fn hooked(ctx: ToyCtx) -> Game<Toy> {
    Game::new(ctx, Box::new(Home::default()), hooks())
}

/// Until a layout is chosen, the first key (bound or not) opens the
/// first-key screen and does nothing else; the next keys are that
/// screen's, and no second one opens.
#[test]
fn with_no_layout_the_first_key_opens_the_first_key_screen() {
    let mut game = hooked(first_launch());
    // Releases, text and buttons don't.
    game.frame(&[RawInputEvent::Up(Key::F), RawInputEvent::Text('f')], 0.0);
    pad_tap(&mut game, Button::DpadDown);
    assert_eq!(game.screens(), ["home"]);
    // `f` would open the menu: it doesn't, and it is not held.
    game.frame(&[down(Key::F)], 0.0);
    assert_eq!(game.screens(), ["home", "picker"]);
    assert!(!game.input().is_held(Action::Confirm));
    assert_eq!(
        game.ctx().device,
        Device::Keyboard,
        "the key was the keyboard's"
    );
    game.frame(&[RawInputEvent::Up(Key::F)], 0.0);
    tap(&mut game, Key::Up);
    assert_eq!(game.screens(), ["home", "picker"]);
    tap(&mut game, Key::J);
    assert_eq!(game.screens(), ["home"]);
    assert_eq!(game.ctx().layout(), Some(RIGHT_HANDED));
    assert_eq!(game.input().keymap(), &game.ctx().keymap);
    // Chosen: keys act again and the picker stays away.
    tap(&mut game, Key::F);
    assert_eq!(game.screens(), ["home", "menu"]);
}

/// A controller button closes the first-key screen without choosing; the
/// next key asks again. A layout chosen, or no keyboard, and it never
/// opens.
#[test]
fn the_first_key_screen_opens_only_while_needed() {
    let mut game = hooked(first_launch());
    tap(&mut game, Key::Z);
    pad_tap(&mut game, Button::South);
    assert_eq!(game.screens(), ["home"]);
    tap(&mut game, Key::Z);
    assert_eq!(game.screens(), ["home", "picker"]);
    let mut game = hooked(ctx());
    tap(&mut game, Key::Z);
    assert_eq!(game.screens(), ["home"]);
    let mut console = first_launch();
    console.keyboard = Keyboard::Absent;
    let mut game = hooked(console);
    tap(&mut game, Key::Z);
    pad_tap(&mut game, Button::South);
    assert_eq!(game.screens(), ["home", "menu"], "the buttons work");
    assert_eq!(game.ctx().layout(), None);
    // Without the hook, a first launch's keys are the layout picker's.
    let mut game = home(first_launch());
    tap(&mut game, Key::J);
    assert_eq!(game.screens(), ["home", "menu"]);
}

/// The Debug key opens the debug screen with debug tools on, never over
/// one of the debug screens, and never without the tools.
#[test]
fn the_debug_key_opens_the_debug_screen_once() {
    let mut game = hooked(ctx());
    tap(&mut game, Key::F2);
    assert_eq!(game.screens(), ["home", "tools"]);
    tap(&mut game, Key::F2);
    assert_eq!(game.screens(), ["home", "tools"]);
    tap(&mut game, Key::F);
    assert_eq!(game.screens(), ["home", "tools", "inspector"]);
    tap(&mut game, Key::F2);
    assert_eq!(game.screens(), ["home", "tools", "inspector"]);
    tap(&mut game, Key::D);
    tap(&mut game, Key::D);
    assert_eq!(game.screens(), ["home"]);
    let mut game = hooked(ctx()).with_debug_screens(false);
    tap(&mut game, Key::F2);
    assert_eq!(game.screens(), ["home"]);
    assert!(!game.ctx().debug_tools);
    // Over the menu (no debug screen) it opens; without a hook, never.
    let mut game = hooked(ctx());
    tap(&mut game, Key::F);
    tap(&mut game, Key::F2);
    assert_eq!(game.screens(), ["home", "menu", "tools"]);
    let mut game = home(ctx()).with_debug_screens(true);
    tap(&mut game, Key::F2);
    assert_eq!(game.screens(), ["home"]);
}

/// A root screen that counts how often it was covered and uncovered.
struct Watcher(Rc<RefCell<(u32, u32)>>);

impl Screen<Toy> for Watcher {
    fn name(&self) -> &'static str {
        "watcher"
    }
    fn update(&mut self, _: &mut ToyCtx, _: &FrameInput<Action>) -> Transition<Toy> {
        Transition::None
    }
    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
    fn covered(&mut self, _: &mut ToyCtx) {
        self.0.borrow_mut().0 += 1;
    }
    fn uncovered(&mut self, _: &mut ToyCtx, _: Option<ScreenResult>) -> Transition<Toy> {
        self.0.borrow_mut().1 += 1;
        Transition::None
    }
}

/// Ticket 2306: the screens the game's hooks open cover the top screen
/// like any push, and closing them uncovers it.
#[test]
fn the_hooks_screens_cover_the_top_screen() {
    let seen = Rc::new(RefCell::new((0, 0)));
    let mut game = Game::new(ctx(), Box::new(Watcher(Rc::clone(&seen))), hooks());
    tap(&mut game, Key::F2);
    assert_eq!(game.screens(), ["watcher", "tools"]);
    assert_eq!(*seen.borrow(), (1, 0));
    tap(&mut game, Key::D);
    assert_eq!(game.screens(), ["watcher"]);
    assert_eq!(*seen.borrow(), (1, 1));
    let mut game = Game::new(first_launch(), Box::new(Watcher(Rc::clone(&seen))), hooks());
    tap(&mut game, Key::Z);
    assert_eq!(game.screens(), ["watcher", "picker"]);
    assert_eq!(*seen.borrow(), (2, 1));
}

/// The frame that opens the debug screen doesn't reach the screen below.
#[test]
fn the_debug_key_frame_goes_to_no_screen() {
    let seen = Rc::default();
    let mut game = Game::new(ctx(), Box::new(Spy(Rc::clone(&seen))), hooks());
    game.frame(&[down(Key::F2), down(Key::F)], 0.0);
    assert_eq!(game.screens(), ["spy", "tools"]);
    let got = seen.borrow();
    assert!(got.is_empty(), "{got:?}");
}

/// Plays `click` on Confirm, the track `drums` on Cancel, quits (with a
/// quieter click) on Up and asks for an unknown cue on Down.
struct Noisy;

impl Screen<Toy> for Noisy {
    fn name(&self) -> &'static str {
        "noisy"
    }
    fn update(&mut self, ctx: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        for action in &input.actions {
            match action {
                Action::Confirm => ctx.audio.play_sound("click"),
                Action::Cancel => ctx.audio.play_music("drums"),
                Action::CursorUp => {
                    ctx.audio.play_sound_at("click", 0.5);
                    return Transition::Quit;
                }
                Action::CursorDown => ctx.audio.play_sound("nope"),
                _ => {}
            }
        }
        Transition::None
    }
    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

/// A game on [`Noisy`] (the toy fades music in 0.5 s), `theme` asked for
/// already.
fn noisy() -> Game<Toy> {
    let mut ctx = ctx();
    ctx.audio.play_music("theme");
    Game::new(ctx, Box::new(Noisy), GameHooks::default())
}

fn click(volume: f32) -> AudioRequest<Voice> {
    AudioRequest::PlaySound {
        cue: "click".into(),
        volume,
    }
}

#[test]
fn frames_carry_the_audio_screens_asked_for() {
    let mut game = noisy();
    // A request made before the first frame goes out with it.
    let out = game.frame(&[], 0.0);
    let theme = AudioRequest::PlayMusic {
        cue: "theme".into(),
        switch: None,
    };
    assert_eq!(out.audio, [theme]);
    let load = MusicCommand::Load {
        cue: "theme".into(),
    };
    let start = MusicCommand::Start {
        cue: "theme".into(),
        gain: 1.0,
    };
    assert_eq!(out.music, [load, start]);
    let out = game.frame(&[down(Key::F)], 0.0);
    assert_eq!(out.audio, [click(1.0)]);
    assert!(out.music.is_empty(), "{:?}", out.music);
    // Each frame holds only its own requests.
    let out = game.frame(&[RawInputEvent::Up(Key::F)], 0.0);
    assert!(out.audio.is_empty(), "{:?}", out.audio);
    let got = game.ctx().audio.pending();
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn frames_carry_the_music_fade_of_the_manifest() {
    let mut game = noisy();
    game.frame(&[], 0.0);
    let out = game.frame(&[down(Key::D)], 0.25);
    let half = MusicCommand::Gain {
        cue: "theme".into(),
        gain: 0.5,
    };
    let load = MusicCommand::Load {
        cue: "drums".into(),
    };
    assert_eq!(out.music, [load, half]);
    assert_eq!(game.music().target(), Some("drums"));
    let out = game.frame(&[RawInputEvent::Up(Key::D)], 0.25);
    let stop = MusicCommand::Stop {
        cue: "theme".into(),
    };
    let start = MusicCommand::Start {
        cue: "drums".into(),
        gain: 1.0,
    };
    assert_eq!(out.music, [stop, start]);
    assert_eq!(game.music().current(), Some("drums"));
    // A one-second fade: a quarter of a second is a quarter of it.
    let mut ctx = ctx();
    ctx.game.audio.music_fade_ms = 1000;
    ctx.audio.play_music("theme");
    let mut game = Game::new(ctx, Box::new(Noisy), GameHooks::default());
    game.frame(&[], 0.0);
    let out = game.frame(&[down(Key::D)], 0.25);
    let gain = MusicCommand::Gain {
        cue: "theme".into(),
        gain: 0.75,
    };
    assert_eq!(out.music[1..], [gain]);
}

/// With the manifest's fade-in, the first track after silence starts at
/// gain 0 and rises once the platform says it sounds.
#[test]
fn the_first_track_fades_in_once_it_sounds_with_the_manifests_fade_in() {
    let mut ctx = ctx();
    ctx.game.audio.music_fade_in_ms = 1000;
    ctx.audio.play_music("theme");
    let mut game = Game::new(ctx, Box::new(Noisy), GameHooks::default());
    let out = game.frame(&[], 0.5);
    let load = MusicCommand::Load {
        cue: "theme".into(),
    };
    let start = MusicCommand::Start {
        cue: "theme".into(),
        gain: 0.0,
    };
    assert_eq!(out.music, [load, start]);
    game.set_music_playing(None);
    let out = game.frame(&[], 0.5);
    assert!(out.music.is_empty(), "{:?}", out.music);
    game.set_music_playing(Some(("theme", 0.0)));
    let out = game.frame(&[], 0.25);
    let quarter = MusicCommand::Gain {
        cue: "theme".into(),
        gain: 0.25,
    };
    assert_eq!(out.music, [quarter]);
}

#[test]
fn the_quitting_frame_still_sounds_and_later_ones_are_silent() {
    let mut game = noisy();
    let out = game.frame(&[down(Key::Up)], 0.0);
    assert!(out.quit);
    assert_eq!(out.audio[1..], [click(0.5)]);
    let out = game.frame(&[RawInputEvent::Up(Key::Up), down(Key::F)], 0.0);
    assert!(out.audio.is_empty(), "{:?}", out.audio);
    assert!(out.music.is_empty(), "{:?}", out.music);
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "audio request for a cue not in the audio manifest")]
fn an_unknown_cue_panics_in_debug_builds() {
    let mut game = noisy();
    game.frame(&[down(Key::Down)], 0.0);
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "play_sound of the looped cue \"hum\"")]
fn playing_a_looped_cue_without_a_handle_panics_in_debug_builds() {
    let mut ctx = ctx();
    ctx.audio.play_sound("hum");
    let mut game = Game::new(ctx, Box::new(Noisy), GameHooks::default());
    game.frame(&[], 0.0);
}

/// A sound's fade moves with the frames' time, in the frame's own
/// requests.
#[test]
fn a_sounds_fade_moves_with_the_frames() {
    let mut ctx = ctx();
    let id = ctx.audio.start_sound("hum");
    ctx.audio.stop_sound(id, 1000);
    let mut game = Game::new(ctx, Box::new(Noisy), GameHooks::default());
    let out = game.frame(&[], 0.25);
    let start = AudioRequest::StartSound {
        id,
        cue: "hum".into(),
        volume: 1.0,
    };
    let quarter = AudioRequest::SoundVolume { id, volume: 0.75 };
    assert_eq!(out.audio, [start, quarter]);
    let out = game.frame(&[], 0.75);
    assert_eq!(out.audio, [AudioRequest::StopSound { id }]);
}

#[test]
fn cues_must_be_of_the_right_kind() {
    let toy = Toy::new();
    let audio = &toy.audio;
    let sound = |cue: &str| AudioRequest::<Voice>::PlaySound {
        cue: cue.into(),
        volume: 1.0,
    };
    let play = |cue: &str| AudioRequest::<Voice>::PlayMusic {
        cue: cue.into(),
        switch: None,
    };
    assert!(is_known(audio, &sound("click")));
    assert!(!is_known(audio, &sound("theme")));
    assert!(is_known(audio, &play("theme")));
    assert!(!is_known(audio, &play("click")));
    let mut queue = crate::audio::AudioQueue::<Voice>::default();
    let id = queue.start_sound("hum");
    queue.start_sound("theme");
    queue.set_sound_volume(id, 0.5);
    queue.stop_sound(id, 0);
    let known: Vec<bool> = queue.take().iter().map(|r| is_known(audio, r)).collect();
    assert_eq!(known, [true, false, true, true]);
    let line = shuyi_assets::dialogue::LineId::new("x");
    for request in [
        AudioRequest::StopMusic,
        AudioRequest::PlayVoice {
            line: line.clone(),
            variant: Voice::Low,
        },
        AudioRequest::StopVoice,
        AudioRequest::PreloadVoices { lines: vec![] },
    ] {
        assert!(is_known(audio, &request), "{request:?}");
    }
}

/// What the platform reports becomes the clock screens read.
#[test]
fn the_music_the_platform_reports_becomes_the_clock() {
    // The toy's tracks loop and are 10 s long.
    let mut game = noisy();
    assert_eq!(game.ctx().music_clock, None);
    game.set_music_playing(Some(("drums", 12.5)));
    let clock = game.ctx().music_clock.clone().unwrap();
    assert_eq!(clock.cue, "drums");
    assert!((clock.position - 2.5).abs() < 1e-5, "{clock:?}");
    // A frame leaves it as reported.
    game.frame(&[], 0.5);
    assert_eq!(game.ctx().music_clock, Some(clock));
    // A cue the manifest lacks has no clock, and silence clears it.
    game.set_music_playing(Some(("nope", 1.0)));
    assert_eq!(game.ctx().music_clock, None);
    game.set_music_playing(Some(("theme", 1.0)));
    assert!(game.ctx().music_clock.is_some());
    game.set_music_playing(None);
    assert_eq!(game.ctx().music_clock, None);
}

/// The settings the platform applies go out every frame.
#[test]
fn frames_carry_the_volumes_and_fullscreen() {
    let mut game = home(ctx());
    let out = game.frame(&[], 0.0);
    assert!((out.music_volume - 0.8).abs() < 1e-6);
    assert!((out.sound_volume - 0.8).abs() < 1e-6);
    assert!((out.voice_volume - 0.8).abs() < 1e-6);
    assert!(!out.fullscreen);
    game.ctx_mut()
        .change_settings(|s| {
            s.music_volume = 25;
            s.sound_volume = 50;
        })
        .unwrap();
    game.ctx_mut().voice_volume = 3;
    let out = game.frame(&[], 0.0);
    assert!((out.music_volume - 0.25).abs() < 1e-6);
    assert!((out.sound_volume - 0.5).abs() < 1e-6);
    assert!((out.voice_volume - 0.3).abs() < 1e-6);
    // The window's own shortcut flips the setting.
    game.toggle_fullscreen();
    assert!(game.frame(&[], 0.0).fullscreen);
    assert!(game.ctx().settings().fullscreen);
    game.toggle_fullscreen();
    assert!(!game.frame(&[], 0.0).fullscreen);
}

/// The frame cap goes out every frame with its serial; the platform's
/// report comes back before the next, and confirms a request for a cap.
#[test]
fn a_request_for_a_frame_cap_is_confirmed_by_the_platforms_report() {
    use crate::frame_cap::{CapStatus, FrameCap, Pacing};
    let mut game = home(ctx());
    let out = game.frame(&[], 0.0);
    assert_eq!(
        (out.frame_cap, out.frame_cap_serial),
        (FrameCap::Fps(60), 0)
    );
    assert_eq!(game.ctx().pacing(), None);
    // A request is pending until the platform reports its serial.
    let request = game.ctx_mut().request_frame_cap(FrameCap::Off);
    assert_eq!(request.serial(), 1);
    assert_eq!(game.ctx().frame_cap_status(request), CapStatus::Pending);
    let out = game.frame(&[], 0.0);
    assert_eq!((out.frame_cap, out.frame_cap_serial), (FrameCap::Off, 1));
    // A report of an older serial doesn't confirm it.
    game.set_pacing(Pacing::new(FrameCap::Fps(60), 0));
    assert_eq!(game.ctx().frame_cap_status(request), CapStatus::Pending);
    let mut report = Pacing::new(FrameCap::Off, 1);
    report.fps = 240;
    game.set_pacing(report);
    assert_eq!(
        game.ctx().frame_cap_status(request),
        CapStatus::Applied(report)
    );
    assert_eq!(game.ctx().pacing(), Some(report));
    // The cap is saved like any setting.
    let key = crate::settings::SETTINGS_KEY;
    let saved = crate::storage::Storage::read(&game.ctx().storage, key);
    let saved = saved.unwrap().unwrap();
    assert!(saved.contains("frame_cap:Off"), "{saved}");
    // A second request before the report replaces the first: the first is
    // superseded, the second applied once its serial is reported.
    let first = game.ctx_mut().request_frame_cap(FrameCap::Fps(30));
    let second = game.ctx_mut().request_frame_cap(FrameCap::Fps(120));
    assert_eq!(game.ctx().frame_cap_status(first), CapStatus::Superseded);
    assert_eq!(game.ctx().frame_cap_status(second), CapStatus::Pending);
    let out = game.frame(&[], 0.0);
    assert_eq!(
        (out.frame_cap, out.frame_cap_serial),
        (FrameCap::Fps(120), 3)
    );
    let report = Pacing::new(FrameCap::Fps(120), 3);
    game.set_pacing(report);
    assert_eq!(game.ctx().frame_cap_status(first), CapStatus::Superseded);
    assert_eq!(
        game.ctx().frame_cap_status(second),
        CapStatus::Applied(report)
    );
    assert_eq!(game.ctx().frame_cap_status(request), CapStatus::Superseded);
    // Asking for the cap in force is confirmed all the same.
    let again = game.ctx_mut().request_frame_cap(FrameCap::Fps(120));
    assert_eq!(again.serial(), 4);
    assert_eq!(game.ctx().frame_cap_status(again), CapStatus::Pending);
    // Any change of the cap takes the next serial; others don't.
    game.ctx_mut()
        .change_settings(|s| s.music_volume = 1)
        .unwrap();
    assert_eq!(game.ctx().frame_cap_serial(), 4);
    game.ctx_mut()
        .change_settings(|s| s.frame_cap = FrameCap::Fps(60))
        .unwrap();
    assert_eq!(game.ctx().frame_cap_serial(), 5);
    assert_eq!(game.ctx().frame_cap_status(again), CapStatus::Superseded);
}

/// A profile switched to with another frame cap is a change of the cap.
#[test]
fn a_profile_with_another_frame_cap_takes_the_next_serial() {
    use crate::frame_cap::FrameCap;
    let mut game = home(ctx());
    let ctx = game.ctx_mut();
    let other = ctx.add_profile("Other").unwrap();
    let first = ctx.profiles().current();
    ctx.switch_profile(other).unwrap();
    assert_eq!(ctx.frame_cap_serial(), 0);
    ctx.request_frame_cap(FrameCap::Fps(30));
    assert_eq!(ctx.frame_cap_serial(), 1);
    ctx.switch_profile(first).unwrap();
    assert_eq!(ctx.settings().frame_cap, FrameCap::Fps(60));
    assert_eq!(ctx.frame_cap_serial(), 2);
}

#[test]
fn the_context_is_handed_back_with_its_storage() {
    let mut game = home(ctx());
    game.ctx_mut().choose_layout(LEFT_HANDED).unwrap();
    let ctx = game.into_ctx();
    assert_eq!(ctx.saved_layout(), Some(LEFT_HANDED));
}

#[test]
fn hooks_say_which_screens() {
    let none = GameHooks::<Toy>::default();
    assert!(none.first_key.is_none() && none.debug.is_none());
    assert!(none.debug_screens.is_empty(), "{:?}", none.debug_screens);
    let hook = hooks().debug.unwrap();
    let copy = hook;
    assert_eq!(format!("{copy:?}"), "ScreenHook(tools)");
    assert_eq!((hook.open)(&ctx()).name(), "tools");
}

#[test]
fn pad_changes_become_pad_events_with_the_pressing_pads_kind() {
    let mut pads = Pads::new(StickDef {
        press_percent: 50,
        release_percent: 35,
    });
    let holding = |buttons: &[Button]| PadState {
        buttons: buttons.iter().copied().collect(),
        ..PadState::default()
    };
    let pad = |id, kind, buttons: &[Button]| (id, kind, holding(buttons));
    let sony = PadKind::PlayStation;
    let both = [
        pad(3, PadKind::Xbox, &[Button::Start]),
        pad(7, sony, &[Button::South]),
    ];
    assert_eq!(
        RawInputEvent::from_pads(&mut pads, &both),
        [
            RawInputEvent::PadDown(Button::South, sony),
            RawInputEvent::PadDown(Button::Start, PadKind::Xbox),
        ]
    );
    // Releases come first and carry no kind.
    let next = [
        pad(3, PadKind::Xbox, &[Button::South]),
        pad(7, sony, &[Button::South, Button::North]),
    ];
    assert_eq!(
        RawInputEvent::from_pads(&mut pads, &next),
        [
            RawInputEvent::PadUp(Button::Start),
            RawInputEvent::PadDown(Button::North, sony),
        ]
    );
    assert_eq!(RawInputEvent::from_pads(&mut pads, &[]).len(), 2);
}

/// Ticket 1035: each notch of the crank is a press and a release of a
/// crank button, from the pad of the kind given; a docked crank gives
/// nothing and forgets a part notch.
#[test]
fn crank_notches_become_presses_and_releases_of_the_crank_buttons() {
    let mut crank = crate::input::Crank::new(shuyi_assets::keymap::CrankDef { notch_degrees: 30 });
    let kind = PadKind::Playdate;
    let back = [
        RawInputEvent::PadDown(Button::CrankBack, kind),
        RawInputEvent::PadUp(Button::CrankBack),
    ];
    assert_eq!(
        RawInputEvent::from_crank(&mut crank, -20.0, false, kind),
        []
    );
    assert_eq!(
        RawInputEvent::from_crank(&mut crank, -50.0, false, kind),
        [back, back].concat()
    );
    assert_eq!(
        RawInputEvent::from_crank(&mut crank, 40.0, false, kind),
        [
            RawInputEvent::PadDown(Button::CrankForward, kind),
            RawInputEvent::PadUp(Button::CrankForward),
        ]
    );
    // 20 wound forwards, then docked: forgotten.
    assert_eq!(RawInputEvent::from_crank(&mut crank, -25.0, true, kind), []);
    assert_eq!(RawInputEvent::from_crank(&mut crank, 20.0, false, kind), []);
}

/// Ticket 1035: an action from outside the keys (a platform's own menu)
/// reaches the top screen in an update of its own, as its key would.
#[test]
fn an_action_from_outside_the_keys_reaches_the_top_screen() {
    let mut game = home(ctx());
    game.act(Action::Confirm);
    assert_eq!(game.screens(), ["home", "menu"]);
    assert_eq!(game.screen::<Home>().map(|h| h.frames), Some(1));
    // The menu's click comes out with the next frame.
    game.act(Action::Confirm);
    let click = AudioRequest::PlaySound {
        cue: "click".into(),
        volume: 1.0,
    };
    assert_eq!(game.frame(&[], 0.0).audio, [click]);
    game.act(Action::Cancel);
    assert_eq!(game.screens(), ["home"]);
    game.act(Action::Cancel);
    assert!(game.quit_requested());
    // After a quit, nothing.
    game.act(Action::Confirm);
    assert_eq!(game.screens(), ["home"]);
}

/// With the guard on, the game's screens get only what they declare, and
/// the game says what was held back; off, nothing is.
#[test]
fn the_guard_holds_back_what_a_screen_does_not_declare() {
    let mut game = home(ctx());
    game.act(Action::Info);
    assert_eq!(game.undeclared_actions(), []);
    game.guard_actions(true);
    game.act(Action::Info);
    game.act(Action::Confirm);
    assert_eq!(game.undeclared_actions(), [("home", Action::Info)]);
    assert_eq!(
        game.screens(),
        ["home", "menu"],
        "Confirm still opens the menu"
    );
    game.guard_actions(false);
    game.act(Action::Info);
    assert_eq!(game.undeclared_actions().len(), 1);
}
