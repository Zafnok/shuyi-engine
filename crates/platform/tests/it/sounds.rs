//! Sounds started with a handle, in the Harness: they belong to the
//! screen that started them, a pause menu holds the ones below it, and
//! the headless backend's `Speaker` holds them as playing until they
//! stop, and the audio while the system suspends the game.

use shuyi_platform::SystemEvent;
use shuyi_platform::harness::Harness;
use shuyi_ui::audio::AudioRequest;
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::screen::{FrameInput, Screen, ScreenStack, Transition};
use shuyi_ui::toy::{self, Action, Home, Toy, ToyCtx};
use shuyi_ui::{Game, GameHooks};

/// A screen with a loop: Confirm starts the toy's `hum`, Cancel pops,
/// Select pushes another, Menu replaces it with another, and up and down
/// pause the sounds and resume them.
struct Looper;

impl Screen<Toy> for Looper {
    fn name(&self) -> &'static str {
        "looper"
    }

    fn actions(&self) -> Option<Vec<Action>> {
        Some(vec![
            Action::Confirm,
            Action::Cancel,
            Action::Select,
            Action::Menu,
            Action::CursorUp,
            Action::CursorDown,
        ])
    }

    fn update(&mut self, ctx: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        for &action in &input.actions {
            match action {
                Action::Confirm => {
                    ctx.audio.start_sound("hum");
                }
                Action::Cancel => return Transition::Pop,
                Action::Select => return Transition::Push(Box::new(Self)),
                Action::Menu => return Transition::Replace(Box::new(Self)),
                Action::CursorUp => ctx.audio.pause_sounds(),
                Action::CursorDown => ctx.audio.resume_sounds(),
                _ => {}
            }
        }
        Transition::None
    }

    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

/// The toy's home with a [`Looper`] over it.
fn looper() -> Harness<Toy> {
    let mut stack = ScreenStack::new(Box::new(Home::default()));
    stack.push(Box::new(Looper));
    Harness::new(Game::with_stack(toy::ctx(), stack, GameHooks::default()))
}

/// The ids of the sounds the `Speaker` holds as playing.
fn playing(h: &Harness<Toy>) -> Vec<u32> {
    h.sounds_playing().iter().map(|p| p.id.get()).collect()
}

/// The ids the last frame stopped.
fn stopped(h: &Harness<Toy>) -> Vec<u32> {
    h.last_frame_audio()
        .iter()
        .filter_map(|r| match r {
            AudioRequest::StopSound { id } => Some(id.get()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_loop_plays_until_its_screen_pops_and_stops_in_that_frame() {
    let mut h = looper();
    h.act(Action::Confirm);
    assert_eq!(playing(&h), [1]);
    assert_eq!(
        h.sounds_playing().first().map(|p| p.cue.as_str()),
        Some("hum")
    );
    assert_eq!(h.sounds(), ["hum"], "a started cue is a sound");
    h.act(Action::Cancel);
    assert_eq!(h.screens(), ["home"]);
    assert_eq!(stopped(&h), [1]);
    let got = h.sounds_playing();
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn a_replaced_screen_takes_its_loop_with_it() {
    let mut h = looper();
    h.act(Action::Confirm);
    h.act(Action::Menu);
    assert_eq!(h.screens(), ["home", "looper"]);
    assert_eq!(stopped(&h), [1]);
    let got = h.sounds_playing();
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn a_loop_under_a_pushed_screen_plays_on() {
    let mut h = looper();
    h.act(Action::Confirm);
    h.act(Action::Select);
    h.act(Action::Confirm);
    assert_eq!(h.screens(), ["home", "looper", "looper"]);
    assert_eq!(playing(&h), [1, 2]);
    // The top one leaves with its own loop; the one below keeps its.
    h.act(Action::Cancel);
    assert_eq!(stopped(&h), [2]);
    assert_eq!(playing(&h), [1]);
}

/// The ids of the sounds the `Speaker` holds as paused.
fn paused(h: &Harness<Toy>) -> Vec<u32> {
    h.sounds_playing()
        .iter()
        .filter(|p| p.paused)
        .map(|p| p.id.get())
        .collect()
}

/// How many `PauseSounds` and `ResumeSounds` the last frame sent.
fn pauses(h: &Harness<Toy>) -> (usize, usize) {
    let audio = h.last_frame_audio();
    let count =
        |want: fn(&AudioRequest<toy::Voice>) -> bool| audio.iter().filter(|r| want(r)).count();
    (
        count(|r| matches!(r, AudioRequest::PauseSounds)),
        count(|r| matches!(r, AudioRequest::ResumeSounds)),
    )
}

#[test]
fn a_menu_that_pauses_holds_the_loop_below_and_its_pop_resumes_it() {
    let mut h = looper();
    h.act(Action::Confirm);
    // The menu pauses on its first frame, then starts a loop of its own.
    h.act(Action::Select).act(Action::CursorUp);
    assert_eq!(paused(&h), [1]);
    h.act(Action::Confirm);
    assert_eq!(playing(&h), [1, 2]);
    assert_eq!(paused(&h), [1], "the menu's own loop plays");
    // Popping it stops its loop and ends its pause, with no resume.
    h.act(Action::Cancel);
    assert_eq!(stopped(&h), [2]);
    assert_eq!(pauses(&h), (0, 1));
    assert_eq!(playing(&h), [1]);
    let got = paused(&h);
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn two_pauses_send_one_pause_and_one_resume_when_the_last_ends() {
    let mut h = looper();
    h.act(Action::Confirm);
    h.act(Action::Select).act(Action::CursorUp);
    assert_eq!(pauses(&h), (1, 0));
    h.act(Action::CursorUp);
    assert_eq!(pauses(&h), (0, 0), "pausing again every frame is fine");
    h.act(Action::Select).act(Action::CursorUp);
    assert_eq!(pauses(&h), (0, 0));
    h.act(Action::Cancel);
    assert_eq!(pauses(&h), (0, 0), "the menu below still pauses");
    assert_eq!(paused(&h), [1]);
    h.act(Action::CursorDown);
    assert_eq!(pauses(&h), (0, 1));
    let got = paused(&h);
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn a_suspend_holds_the_audio_until_the_resume_and_a_second_changes_nothing() {
    let mut h = looper();
    h.act(Action::Confirm);
    assert!(!h.platform().speaker().suspended());
    h.system(SystemEvent::Suspend);
    assert!(h.platform().speaker().suspended());
    h.system(SystemEvent::Suspend);
    assert!(h.platform().speaker().suspended());
    assert_eq!(playing(&h), [1]);
    h.system(SystemEvent::Resume);
    assert!(!h.platform().speaker().suspended());
    assert_eq!(playing(&h), [1]);
    let got = paused(&h);
    assert!(got.is_empty(), "{got:?}");
}
