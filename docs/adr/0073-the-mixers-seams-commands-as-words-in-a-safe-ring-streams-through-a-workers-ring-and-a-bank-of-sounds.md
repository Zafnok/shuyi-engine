# ADR-0073: The mixer's seams: commands as words in a safe ring, streams through a worker's ring, and a bank of sounds

- **Status:** Accepted
- **Date:** 2026-11-01
- **Related tickets:** 3030 (this), 3031, 3032, 2397, 3027, 2573
- **Extends:** ADR-0066, which chose the owned integer mixer

## Context

`shuyi-sound` forbids `unsafe` (ADR-0021 §6) and builds without `std`, and its
mixer is called from a system's audio callback, which may not take a lock, wait or
allocate. It needs commands from a game's thread, frames from a decoding worker,
and sounds it can hold without freeing them mid-mix. The usual tools for this are an
`UnsafeCell` ring or a mutex. Ticket 3030 asked for a safe queue if one could be
real-time safe, and otherwise for the queue to be left to 3031.

`Source` as 2210 left it had one way to say "nothing now": `0`, the end. A worker
that is late isn't the end of a stream.

## Decision

**A ring of `AtomicU32`s, with no `unsafe`.** A slot is words. The writer stores
them and then its head with `Release`; the reader loads the head with `Acquire` and
then the words; the tail is the same the other way. Types give each end one owner.
Only atomic loads and stores are used, which the smallest targets have, and memory
is taken once. So a command is plain data, encoded to 12 words, and the queue is real
time safe in safe code: 3031 uses it rather than writing one in `sys`.

**The game's side picks the voice.** `Remote` keeps which voices are busy, so `play`
answers `Full` at once and a command applied on the audio thread can't be refused.
A second ring brings back which voices ended, with room for every voice, so it never
overflows. The ids are the same as a mixer used directly.

**What can't be words goes ahead of time.** A `Pcm` or a boxed source can't cross as
words in safe code. The mixer holds a fixed bank of `SharedPcm`s named by `SoundId`,
and fixed stream slots named by `StreamId`, filled with `add_sound` and `add_stream`
before it moves to the audio thread. Neither allocates or frees, so a backend with
a hand-over of its own in its `sys` module can call them on the audio thread too.
`remove_sound` and `take_finished_stream` give an object back for its caller to free:
`render` never frees one. So `play` takes a `SoundId`, not the `Pcm` the ticket's
sketch had. A stale id plays as a sound with no frames, which keeps `play`'s answer
`Result<VoiceId, Full>` and commands infallible.

**Streams go through a worker's ring.** `feed` makes a `Feeder` and a `Feed`, a ring
of frames as words. The worker decodes ahead; `render` only reads the `Feed`, so it
does no decoder work. A read of an empty ring is `Error::Underrun`, a new variant:
the voice stays where it is and sounds nothing, so a late worker delays a stream and
loses and repeats nothing, and `Mixer::underrun_frames` counts it. `0` still means
the end, after `Feeder::close`.

**A loop's way back is the worker's, ready before the join.** `LoopedRegion` wraps a
decoder on the worker: it trims at the region's end, restarts through a closure,
reads and drops the frames before the region, and goes on, so the ring simply holds
the frames in the order they play. `Source::rewind_to`, a provided method saying
`Error::NoRewind`, is what the mix itself asks at a join when a source has its
frames ready in memory; a `Feed` answers it by doing nothing, its frames being next
already. So `Params::region` belongs on a source that can go back, and a `Feed`'s
region belongs on its worker.

**Exact positions with no division a frame.** A position is 1/65536 of a source
frame. A step in those units is `source rate * rate / output rate`, whose remainder
is carried as a fraction of the output rate. Nothing drifts, a length comes out
exact, and a frame costs additions. A rate is held between 1/256 and 256, so a voice
always ends or comes round and a block's cost stays bounded.

**Speed is a share of the sound's length.** quad-snd has no offline mix, so nothing
is the same work to compare with. The probe renders 32 busy voices in release builds
on each captured platform, the gate holds that to a share of the sound's length, and
an injected slowdown must show, as for the decoders. What a stream holds decoded is
measured the same over short and long streams, against a limit a stream decoded
whole fails. Expected samples are worked out in `xtask` in plain arithmetic that
shares no code with the mixer.

## Consequences

- 3031 and 3032 get a queue with no `unsafe`, and need `sys` only for a run-time
  hand-over of new sounds and sources.
- A command is 48 bytes and a frame in a ring takes 4 whether it is mono or stereo:
  simple, and small next to what they carry.
- A stream that underruns is late from then on. A backend that wants to catch up
  has to choose to, by stopping and starting it.
- Fades and pauses hold still in an underrun, so a late stream's fade ends late with
  it.
- Without shared atomic counts (`thumbv4t`, the GBA) `SharedPcm` is an `Rc`, so the
  mixer isn't `Send` there. That target has one thread and calls the mixer directly.

## Alternatives considered

- **A mutex with `try_lock`, or `std::sync::mpsc`:** needs `std`, and only moves the
  promise of never waiting into a library's internals.
- **An `UnsafeCell` ring in `shuyi-sound-out`:** the ticket's fallback. Not needed:
  it would add `unsafe` for a smaller command and nothing else.
- **`Arc<Pcm>` in each command, through an `AtomicPtr`:** `unsafe`, and the audio
  thread could free the last share mid-mix.
- **An extra `Source` result type for "not yet":** breaks every source. A new
  `Error` variant on a `#[non_exhaustive]` enum breaks none.
- **A sinc or polyphase resampler:** the ticket keeps linear until a game hears the
  difference.
