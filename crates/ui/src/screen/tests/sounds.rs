//! A sound belongs to the screen that started it, and leaves with it.

use super::*;
use crate::audio::{AudioRequest, SoundId};
use crate::toy::Voice;

/// A screen that starts `hum` in each update, then answers as told.
struct Starter {
    next: Vec<Transition<Toy>>,
}

impl Starter {
    fn boxed(next: Vec<Transition<Toy>>) -> Box<dyn Screen<Toy>> {
        Box::new(Self { next })
    }
}

impl Screen<Toy> for Starter {
    fn name(&self) -> &'static str {
        "starter"
    }

    fn update(&mut self, ctx: &mut ToyCtx, _: &FrameInput<Action>) -> Transition<Toy> {
        ctx.audio.start_sound("hum");
        if self.next.is_empty() {
            Transition::None
        } else {
            self.next.remove(0)
        }
    }

    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

fn stops(requests: &[AudioRequest<Voice>]) -> Vec<u32> {
    requests
        .iter()
        .filter_map(|r| match r {
            AudioRequest::StopSound { id } => Some(id.get()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_screen_owns_what_it_starts_in_its_update() {
    let log = Log::default();
    // Built with `push`, which tells no one: the update still knows its
    // screen.
    let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
    stack.push(Starter::boxed(vec![Transition::Pop]));
    let mut ctx = ctx();
    assert!(!stack.update(&mut ctx, &input(&[])));
    assert_eq!(stops(&ctx.audio.take()), [1], "it left with its sound");
    let got = ctx.audio.owners();
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn a_pop_or_a_replace_stops_the_leaving_screens_sounds_and_a_push_keeps_them() {
    let log = Log::default();
    let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
    let mut ctx = ctx();
    let below = ctx.audio.start_sound("hum");
    stack.apply_in(&mut ctx, Transition::Push(Probe::boxed("b", true, &log)));
    let [menu, fading] = [ctx.audio.start_sound("hum"), ctx.audio.start_sound("hum")];
    ctx.audio.stop_sound(fading, 100);
    ctx.audio.take();
    stack.apply_in(&mut ctx, Transition::Pop);
    assert_eq!(stops(&ctx.audio.take()), [menu.get()]);
    assert_eq!(ctx.audio.owners(), [(below, 0), (fading, 1)]);
    stack.apply_in(
        &mut ctx,
        Transition::Replace(Probe::boxed("c", false, &log)),
    );
    assert_eq!(stops(&ctx.audio.take()), [below.get()]);
    // The new screen owns what is started after the replace.
    let next = ctx.audio.start_sound("hum");
    assert_eq!(ctx.audio.owners(), [(fading, 1), (next, 0)]);
}

#[test]
fn a_screen_uncovered_owns_what_it_starts_there() {
    let log = Log::default();
    let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
    stack.push(Probe::boxed("b", false, &log));
    let mut ctx = ctx();
    // Popping `b` uncovers `a`, which runs as the top screen.
    stack.apply_in(&mut ctx, Transition::Pop);
    let id = ctx.audio.start_sound("hum");
    assert_eq!(ctx.audio.owners(), [(id, 0)]);
}

/// One step of [`no_handle_outlives_its_screen`].
#[derive(Debug, Clone)]
enum Step {
    Start,
    /// Stops the n-th sound started (if any), with a fade or not.
    Stop(usize, u32),
    Volume(usize, f32),
    Advance(f32),
    /// The top screen pauses the sounds, or ends its pause.
    Pause,
    Resume,
    Push,
    Pop,
    Replace,
}

fn step() -> impl proptest::strategy::Strategy<Value = Step> {
    use proptest::prelude::*;
    prop_oneof![
        Just(Step::Start),
        (0usize..12, prop_oneof![Just(0u32), 1u32..400]).prop_map(|(n, ms)| Step::Stop(n, ms)),
        (0usize..12, 0.0f32..1.0).prop_map(|(n, v)| Step::Volume(n, v)),
        (0.0f32..0.2).prop_map(Step::Advance),
        Just(Step::Pause),
        Just(Step::Resume),
        Just(Step::Push),
        Just(Step::Pop),
        Just(Step::Replace),
    ]
}

proptest::proptest! {
    /// Whatever screens start, stop, fade, pause and resume, and however
    /// the stack changes through `apply_in`, once every fade has ended
    /// each handle got exactly one stop or is live, every live one belongs
    /// to a screen on the stack, `PauseSounds` and `ResumeSounds`
    /// alternate, and the sounds are paused only while a screen on the
    /// stack pauses them.
    #[test]
    fn no_handle_outlives_its_screen(steps in proptest::collection::vec(step(), 0..40)) {
        let log = Log::default();
        let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
        let mut ctx = ctx();
        let mut started: Vec<SoundId> = Vec::new();
        let mut requests = Vec::new();
        // The depths pausing, as the queue should see them.
        let mut pausing = alloc::collections::BTreeSet::new();
        for step in steps {
            let probe = Probe::boxed("p", false, &log);
            match step {
                Step::Start => started.push(ctx.audio.start_sound("hum")),
                Step::Stop(n, ms) => {
                    if let Some(&id) = started.get(n) {
                        ctx.audio.stop_sound(id, ms);
                    }
                }
                Step::Volume(n, v) => {
                    if let Some(&id) = started.get(n) {
                        ctx.audio.set_sound_volume(id, v);
                    }
                }
                Step::Advance(dt) => ctx.audio.advance(dt),
                Step::Pause => {
                    stack.own_top(&mut ctx);
                    ctx.audio.pause_sounds();
                    pausing.insert(stack.names().len() - 1);
                }
                Step::Resume => {
                    stack.own_top(&mut ctx);
                    ctx.audio.resume_sounds();
                    pausing.remove(&(stack.names().len() - 1));
                }
                Step::Push => {
                    stack.apply_in(&mut ctx, Transition::Push(probe));
                }
                // The last screen stays: popping it ends the game.
                Step::Pop if stack.names().len() > 1 => {
                    pausing.remove(&(stack.names().len() - 1));
                    stack.apply_in(&mut ctx, Transition::Pop);
                }
                Step::Pop => {}
                Step::Replace => {
                    pausing.remove(&(stack.names().len() - 1));
                    stack.apply_in(&mut ctx, Transition::Replace(probe));
                }
            }
            proptest::prop_assert_eq!(ctx.audio.sounds_paused(), !pausing.is_empty());
            requests.extend(ctx.audio.take());
        }
        let pauses: Vec<bool> = requests
            .iter()
            .filter_map(|r| match r {
                AudioRequest::PauseSounds => Some(true),
                AudioRequest::ResumeSounds => Some(false),
                _ => None,
            })
            .collect();
        for (n, &pause) in pauses.iter().enumerate() {
            proptest::prop_assert_eq!(pause, n % 2 == 0, "{:?}", pauses);
        }
        // Every fade is under 0.4 s.
        ctx.audio.advance(1.0);
        requests.extend(ctx.audio.take());
        let stopped = stops(&requests);
        let depth = stack.names().len();
        for id in started {
            let times = stopped.iter().filter(|&&s| s == id.get()).count();
            let live = ctx.audio.sound_is_live(id);
            proptest::prop_assert!(
                times + usize::from(live) == 1,
                "{id:?}: stopped {times} times, live {live}"
            );
        }
        for (id, owner) in ctx.audio.owners() {
            proptest::prop_assert!(owner < depth, "{id:?} belongs to {owner}, the stack is {depth}");
        }
    }
}
