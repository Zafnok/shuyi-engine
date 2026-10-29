//! The sounds started with a handle: [`AudioQueue`]'s ids, their fades
//! and the screens that own them.

use crate::prelude::*;

use super::{AudioQueue, AudioRequest, SoundId, clamp_volume, secs};
use crate::input::frame_seconds;

/// The owner of code outside the screen stack, such as the debug view's
/// pause (2316): no pop releases it, so its pause lasts until it resumes.
pub(crate) const OUTSIDE_THE_STACK: usize = usize::MAX;

/// A sound started with a handle and not yet stopped.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Handle {
    /// The depth on the screen stack of the screen that started it.
    owner: usize,
    /// Its volume, 0–1.
    volume: f32,
    /// Its fade to silence, once it is stopped with one.
    fade: Option<Fade>,
}

/// A volume falling in a straight line from `from` to 0 over `secs`
/// seconds, `elapsed` of them gone.
#[derive(Debug, Clone, PartialEq)]
struct Fade {
    from: f32,
    secs: f32,
    elapsed: f32,
}

impl<V> AudioQueue<V> {
    /// Starts sound `cue` at its own volume, playing until it is stopped:
    /// looped if its cue says so, else once. The handle lives until
    /// [`stop_sound`](Self::stop_sound), or until the screen that started
    /// it leaves the stack. A sound nothing needs to stop is
    /// [`play_sound`](Self::play_sound)'s.
    pub fn start_sound(&mut self, cue: &str) -> SoundId {
        self.start_sound_at(cue, 1.0)
    }

    /// [`start_sound`](Self::start_sound) at `volume` (0–1, clamped) times
    /// the cue's own volume.
    pub fn start_sound_at(&mut self, cue: &str, volume: f32) -> SoundId {
        // Skips 0 if it ever wraps: an id is never 0.
        self.last_id = self.last_id.wrapping_add(1).max(1);
        let id = SoundId(self.last_id);
        let volume = clamp_volume(volume);
        let handle = Handle {
            owner: self.owner,
            volume,
            fade: None,
        };
        self.handles.insert(id, handle);
        self.requests.push(AudioRequest::StartSound {
            id,
            cue: cue.to_owned(),
            volume,
        });
        id
    }

    /// Stops the sound started as `id`: at once with a `fade_ms` of 0,
    /// else after its volume falls in a straight line to silence over
    /// `fade_ms` milliseconds ([`advance`](Self::advance)). A sound already
    /// fading keeps its fade, unless this stops it at once. An unknown or
    /// stopped id does nothing.
    pub fn stop_sound(&mut self, id: SoundId, fade_ms: u32) {
        let Some(handle) = self.handles.get_mut(&id) else {
            return;
        };
        if fade_ms == 0 {
            self.stop_now(id);
        } else if handle.fade.is_none() {
            handle.fade = Some(Fade {
                from: handle.volume,
                secs: secs(fade_ms),
                elapsed: 0.0,
            });
        }
    }

    /// Sets the volume of the sound started as `id` (0–1, clamped). An
    /// unknown or stopped id, or a sound fading out, does nothing.
    pub fn set_sound_volume(&mut self, id: SoundId, volume: f32) {
        let Some(handle) = self.handles.get_mut(&id).filter(|h| h.fade.is_none()) else {
            return;
        };
        handle.volume = clamp_volume(volume);
        self.requests.push(AudioRequest::SoundVolume {
            id,
            volume: handle.volume,
        });
    }

    /// Whether `id` was started and hasn't been stopped (a sound fading
    /// out still counts). A cue that plays once counts until it is
    /// stopped, as the queue knows no lengths.
    pub fn sound_is_live(&self, id: SoundId) -> bool {
        self.handles.contains_key(&id)
    }

    /// Advances the fades by `dt` seconds: each sends its volume, or the
    /// stop on the frame it ends. [`Game`](crate::Game) calls it every
    /// frame before it takes the requests. Nonsense frame times pass no
    /// time.
    pub fn advance(&mut self, dt: f32) {
        let dt = frame_seconds(dt);
        if dt <= 0.0 {
            return;
        }
        let mut ended = Vec::new();
        for (&id, handle) in &mut self.handles {
            let Some(fade) = &mut handle.fade else {
                continue;
            };
            fade.elapsed += dt;
            if fade.elapsed >= fade.secs {
                ended.push(id);
            } else {
                handle.volume = fade.from * (1.0 - fade.elapsed / fade.secs);
                self.requests.push(AudioRequest::SoundVolume {
                    id,
                    volume: handle.volume,
                });
            }
        }
        for id in ended {
            self.stop_now(id);
        }
    }

    /// Pauses every sound started with a handle that is playing now, for
    /// a pause menu: one started after it plays, so the menu's own sounds
    /// are heard. Sound cues played once, the music and the voice go on.
    /// The pause is the calling screen's and lasts until it calls
    /// [`resume_sounds`](Self::resume_sounds) or leaves the stack. A
    /// screen pauses at most once, so calling it every frame is fine, and
    /// several screens pausing don't stack further: the queue sends
    /// [`AudioRequest::PauseSounds`] when the first pause begins and
    /// [`AudioRequest::ResumeSounds`] when the last ends.
    pub fn pause_sounds(&mut self) {
        if self.pauses.is_empty() {
            self.requests.push(AudioRequest::PauseSounds);
        }
        self.pauses.insert(self.owner);
    }

    /// Ends the calling screen's [`pause_sounds`](Self::pause_sounds). A
    /// screen that isn't pausing does nothing.
    pub fn resume_sounds(&mut self) {
        self.end_pause(self.owner);
    }

    /// Whether some screen is pausing the sounds.
    pub fn sounds_paused(&self) -> bool {
        !self.pauses.is_empty()
    }

    /// Makes the screen at `depth` on the stack the owner of the sounds
    /// started from now on: the stack calls it before it runs a screen.
    /// [`OUTSIDE_THE_STACK`] is the owner of code outside the stack.
    pub(crate) fn set_owner(&mut self, depth: usize) {
        self.owner = depth;
    }

    /// Stops at once the sounds of the screen at `depth`, which is leaving
    /// the stack, and ends its pause; one fading out finishes its fade.
    pub(crate) fn release(&mut self, depth: usize) {
        debug_assert_ne!(depth, OUTSIDE_THE_STACK, "no pop releases it");
        let owned: Vec<SoundId> = self
            .handles
            .iter()
            .filter(|(_, h)| h.owner == depth && h.fade.is_none())
            .map(|(&id, _)| id)
            .collect();
        for id in owned {
            self.stop_now(id);
        }
        self.end_pause(depth);
    }

    fn end_pause(&mut self, owner: usize) {
        if self.pauses.remove(&owner) && self.pauses.is_empty() {
            self.requests.push(AudioRequest::ResumeSounds);
        }
    }

    /// The live sounds and the depth of the screen that owns each, in id
    /// order.
    #[cfg(test)]
    pub(crate) fn owners(&self) -> Vec<(SoundId, usize)> {
        self.handles.iter().map(|(&id, h)| (id, h.owner)).collect()
    }

    fn stop_now(&mut self, id: SoundId) {
        if self.handles.remove(&id).is_some() {
            self.requests.push(AudioRequest::StopSound { id });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toy::Voice;

    type AudioQueue = super::AudioQueue<Voice>;
    type AudioRequest = super::AudioRequest<Voice>;

    const FRAME: f32 = 1.0 / 60.0;

    fn start(id: u32, volume: f32) -> AudioRequest {
        AudioRequest::StartSound {
            id: SoundId(id),
            cue: "hum".into(),
            volume,
        }
    }

    #[test]
    fn ids_count_from_one_and_are_the_same_on_every_run() {
        let run = || {
            let mut audio = AudioQueue::default();
            let ids = [audio.start_sound("hum"), audio.start_sound("hum")];
            audio.stop_sound(ids[0], 0);
            (ids, audio.start_sound("hum"), audio.take())
        };
        let (ids, third, requests) = run();
        assert_eq!(ids.map(SoundId::get), [1, 2]);
        assert_eq!(third.get(), 3);
        assert_eq!(
            requests,
            [
                start(1, 1.0),
                start(2, 1.0),
                AudioRequest::StopSound { id: SoundId(1) },
                start(3, 1.0),
            ]
        );
        assert_eq!(run().2, requests);
    }

    #[test]
    fn volumes_are_clamped() {
        let mut audio = AudioQueue::default();
        let loud = audio.start_sound_at("hum", 3.0);
        audio.start_sound_at("hum", f32::NAN);
        audio.set_sound_volume(loud, -1.0);
        assert_eq!(
            audio.take(),
            [
                start(1, 1.0),
                start(2, 0.0),
                AudioRequest::SoundVolume {
                    id: loud,
                    volume: 0.0
                },
            ]
        );
    }

    #[test]
    fn a_stop_without_a_fade_sends_one_stop_and_the_id_is_dead_after() {
        let mut audio = AudioQueue::default();
        let id = audio.start_sound("hum");
        audio.take();
        audio.stop_sound(id, 0);
        audio.stop_sound(id, 0);
        audio.stop_sound(id, 500);
        audio.set_sound_volume(id, 0.5);
        audio.advance(1.0);
        assert_eq!(audio.take(), [AudioRequest::StopSound { id }]);
        assert!(!audio.sound_is_live(id));
    }

    #[test]
    fn an_unknown_id_does_nothing() {
        let mut audio = AudioQueue::default();
        audio.stop_sound(SoundId(9), 0);
        audio.stop_sound(SoundId(9), 100);
        audio.set_sound_volume(SoundId(9), 0.5);
        audio.advance(1.0);
        let got = audio.take();
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn a_fade_sends_falling_volumes_then_one_stop_on_the_frame_it_ends() {
        let mut audio = AudioQueue::default();
        let id = audio.start_sound_at("hum", 0.8);
        audio.take();
        audio.stop_sound(id, 500);
        // The volume can't change on its way out.
        audio.set_sound_volume(id, 1.0);
        let mut volumes = Vec::new();
        let mut frames = 0;
        let stopped = loop {
            frames += 1;
            audio.advance(FRAME);
            let requests = audio.take();
            if let [AudioRequest::SoundVolume { volume, .. }] = requests[..] {
                volumes.push(volume);
            } else {
                break requests;
            }
            assert!(frames < 100, "the fade never ends");
        };
        assert_eq!(stopped, [AudioRequest::StopSound { id }]);
        // 0.5 s of 1/60 s frames: the 30th frame, give or take the
        // rounding of their sum.
        assert!((30..=31).contains(&frames), "{frames}");
        assert!(volumes.windows(2).all(|w| w[1] < w[0]), "{volumes:?}");
        assert!(volumes[0] < 0.8 && volumes[0] > 0.75, "{volumes:?}");
        audio.advance(FRAME);
        let got = audio.take();
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn bad_frame_times_pass_no_time() {
        let mut audio = AudioQueue::default();
        let id = audio.start_sound("hum");
        audio.stop_sound(id, 100);
        audio.take();
        for dt in [f32::NAN, -1.0, f32::INFINITY, 0.0] {
            audio.advance(dt);
        }
        let got = audio.take();
        assert!(got.is_empty(), "{got:?}");
        assert!(audio.sound_is_live(id));
        audio.advance(0.05);
        assert_eq!(
            audio.take(),
            [AudioRequest::SoundVolume { id, volume: 0.5 }]
        );
    }

    #[test]
    fn a_release_stops_its_owners_sounds_and_lets_a_fade_finish() {
        let mut audio = AudioQueue::default();
        let below = audio.start_sound("hum");
        audio.set_owner(1);
        let [on_top, fading] = [audio.start_sound("hum"), audio.start_sound("hum")];
        audio.stop_sound(fading, 100);
        audio.take();
        audio.release(1);
        assert_eq!(audio.take(), [AudioRequest::StopSound { id: on_top }]);
        assert_eq!(audio.owners(), [(below, 0), (fading, 1)]);
        audio.advance(0.1);
        assert_eq!(audio.take(), [AudioRequest::StopSound { id: fading }]);
        assert!(audio.sound_is_live(below));
    }

    #[test]
    fn a_screen_pausing_twice_pauses_once_and_its_resume_ends_it() {
        let mut audio = AudioQueue::default();
        audio.set_owner(1);
        audio.pause_sounds();
        audio.pause_sounds();
        assert!(audio.sounds_paused());
        // Another screen's resume ends no pause of this one's.
        audio.set_owner(0);
        audio.resume_sounds();
        assert_eq!(audio.take(), [AudioRequest::PauseSounds]);
        audio.set_owner(1);
        audio.resume_sounds();
        audio.resume_sounds();
        assert_eq!(audio.take(), [AudioRequest::ResumeSounds]);
        assert!(!audio.sounds_paused());
    }

    #[test]
    fn pauses_by_two_owners_send_one_pause_and_one_resume_when_the_last_ends() {
        let mut audio = AudioQueue::default();
        audio.set_owner(1);
        audio.pause_sounds();
        audio.set_owner(OUTSIDE_THE_STACK);
        audio.pause_sounds();
        audio.set_owner(1);
        audio.resume_sounds();
        assert_eq!(audio.take(), [AudioRequest::PauseSounds]);
        assert!(audio.sounds_paused(), "the outside pause holds");
        audio.set_owner(OUTSIDE_THE_STACK);
        audio.resume_sounds();
        assert_eq!(audio.take(), [AudioRequest::ResumeSounds]);
    }

    #[test]
    fn a_release_ends_the_leaving_screens_pause_after_stopping_its_sounds() {
        let mut audio = AudioQueue::default();
        audio.set_owner(1);
        audio.pause_sounds();
        let menu = audio.start_sound("hum");
        audio.take();
        audio.release(2);
        let got = audio.take();
        assert!(got.is_empty(), "{got:?}");
        audio.release(1);
        assert_eq!(
            audio.take(),
            [
                AudioRequest::StopSound { id: menu },
                AudioRequest::ResumeSounds
            ]
        );
        assert!(!audio.sounds_paused());
    }
}
