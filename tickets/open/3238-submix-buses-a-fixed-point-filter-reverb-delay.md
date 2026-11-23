---
id: "3238"
title: The mixer has submix buses with a fixed-point filter, then reverb and delay
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3238 — The mixer has submix buses with a fixed-point filter, then reverb and delay

## Context

The engine's own mixer (`shuyi-sound`, ticket 3030,
[ADR-0073](../../docs/adr/0073-the-mixers-seams-commands-as-words-in-a-safe-ring-streams-through-a-workers-ring-and-a-bank-of-sounds.md))
adds every voice straight into one `i32` block and clamps it to `i16`
(`crates/sound/src/mix.rs`, `Mixer::render` at line 585, `clamp` at
line 616). There is no bus and no place to put an effect. Sound groups
([ADR-0091](../../docs/adr/0091-sound-groups-the-players-volumes-as-a-tree-of-groups-on-a-loudness-curve-worked-out-before-any-backend.md),
2342) and ducking
([ADR-0099](../../docs/adr/0099-ducking-rules-in-the-audio-manifest-a-voice-the-device-reports-and-a-straight-line-in-whole-milliseconds-in-shuyi-ui.md),
2343) are gains worked out in `shuyi-ui` and multiplied into each
voice's volume before it plays. ADR-0091 §5 says "a backend gets one
gain per cue, never the tree", and §6 says effects on a group wait
for a mixer of our own.

That mixer exists now. 3030's **Out** said "Effects (reverb, filters,
a limiter) and buses for groups: write a ticket when a game asks", and
2342's **Out** said the same. No such ticket was written. A search of
`tickets/` and `docs/adr` for `low-pass`, `lowpass`, `biquad` and
`submix` finds nothing; `reverb` is only in **Out** lists (2206, 2363)
and in the parked research 2911. This is that ticket. It came from a
blind-spot audit of the engine (2026-11-23), not from a game.

The job, as a player would hear it: the world goes muffled under the
pause menu or underwater, a cave echoes, a voice on the radio sounds
thin. A farming game muffles the music when the player walks indoors; a
platformer adds an echo in a cave level. Godot (bus effects), GameMaker
(bus effects), LÖVE (`setFilter`, `setEffect`) and Unity (`AudioMixer`)
all have this.

Why now: the in-house mixer is about to become the default (3084), and
the first release (2005) makes its public items something a game builds
on; Nick ruled out breaking runs after the MVP (2026-11-14, in 3084's
scope). The audit said a bus graph added later would break the API. That
is mostly not so: `Params` and `Config` are `#[non_exhaustive]` with
constructors (ADR-0039), `AudioFrame` is `#[non_exhaustive]` and built
with `AudioFrame::new`, and a new `Backend` method can have a default
body. So the seam can be additive. The ADR this ticket writes checks
that, item by item, before any code.

## Nick input

None.

## Scope

**In:**
- **An ADR on the bus seam**, written first (the `write-adr` skill). It
  amends ADR-0091 §5 and §6 and ADR-0073. It settles:
  - a bus per sound group: each voice plays on its cue's group's bus,
    and each bus adds into its parent's, up to master (the tree of
    `shuyi_assets::audio::groups::BUILT_IN` and a game's own groups);
  - group gains stay where they are (worked out in `shuyi-ui`, multiplied
    into each voice): a bus carries effects, not volumes, so the
    loudness curve and ducking don't change;
  - each bus has a fixed chain of slots (filter, then delay, then
    reverb), with all memory taken in `Mixer::new`; `Mixer::render`
    still takes no lock and allocates nothing;
  - an effect is set and faded through the command ring, so a change
    never clicks;
  - quad-snd and every other backend without buses play the dry mix,
    and say so (a backend's own doc and `docs/platform-backends.md`);
  - which new public items are additive, and that none breaks.
- **Buses in `shuyi-sound`.** `Config::with_buses(count)`,
  `Params::with_bus(bus)`, a bus's parent, and `render` adding each bus
  into its parent, deepest first, then master into `out`. A mixer with
  one bus (the default) gives exactly today's samples: the golden rows
  `script: mono output` and `script: stereo output` don't change.
- **A fixed-point filter.** Low-pass, high-pass and band-pass, as a
  biquad (direct form I): coefficients in Q2.30, sums in `i64`, worked
  out from cutoff and Q by integer arithmetic over a sine table of
  `shuyi-sound`'s own (it depends on no engine crate, `tools/layers.ron`,
  and denies `clippy::float_arithmetic`). Cutoff in whole hertz, Q in
  hundredths. The ADR may choose a state-variable filter instead, if
  the reasons are written down.
- **Effects as data in the audio manifest.** An optional `effects` list
  in `AudioManifest` (`crates/assets/src/audio.rs`): each effect has an
  id, a group, its kind and settings, a fade in whole milliseconds, and
  whether it starts on. It is checked like `ducks` (an unknown group, a
  cutoff past half the output rate, a Q of 0). A manifest without the
  list sounds as before.
- **Turning an effect on from a screen.** `Ctx::set_effect(id, on)` in
  `shuyi-ui`, faded in a straight line in whole milliseconds as ducking
  is (ADR-0099). `AudioFrame` gets a `Copy` view of each effect's level
  (`AudioFrame::with_effects`), and the shared player (ADR-0063) hands
  changes to a new `Backend` method whose default does nothing.
- **The in-house backend** (`crates/app/src/audio/in_house.rs`) makes
  one bus per group at start-up, plays each cue on its group's bus, and
  sends effect changes through `Remote`.
- **The headless `Speaker`** records each effect's level, so a test
  reads "the sfx bus is filtered" from state, not from samples.
- **Then reverb and delay** (steps 8–10): a delay line with feedback
  and a wet level, its longest delay in `Config`; and a small reverb of
  our own writing on the shape of Freeverb (comb and all-pass filters,
  lengths scaled to the output rate), integer only, its memory taken in
  `Mixer::new` for the buses `Config` names.
- The new public types follow ADR-0039: an effect's settings and the
  manifest's effect entry are `#[non_exhaustive]` with constructors; the
  new fields on `Config`, `Params`, `AudioFrame` and `AudioManifest` are
  additive.
- No new dependency. Freeverb is a public-domain design; nothing is
  copied from its code, and the ADR says so.

**Out (do not do):**
- The master limiter and the count of clipped frames: 3239's.
- A compressor, a sidechain, sends between buses, snapshots, chorus,
  EQ with more bands, and a game's own effect code: each a ticket when
  a game asks.
- Positional sound, occlusion and reverb zones (2363 and 2911 are
  parked, and leave reverb zones out themselves).
- Effects on quad-snd, macroquad's audio, the web's older path or the
  Playdate's own audio: they play the dry mix.
- Saving an effect's state in the settings: it is the game's state, not
  the player's.
- An option row for effects.

**Estimated size:** about 2,600 lines (code 1,300, tests 950, docs 350).

**Split:** past 1,000 lines, split off steps 8–10 (the delay, the reverb
and their goldens) into a new ticket, After MVP, blocked by this one.

**Either order with 3084:** if 3084 is done, the in-house backend is the
default and effects sound in a default build: say so in the ADR and in
`docs/platform-backends.md`, and note that a build on quad-snd
(`default-features = false`) plays the dry mix; if not, effects sound
only with `shuyi-app`'s `in-house-sound` feature, and nothing in 3084
changes (its matrix compares the dry mix, which this ticket keeps
sample-for-sample).

**Either order with 3239:** if 3239 is done, the master bus's chain
ends at its limiter: the buses add into master, then the limiter runs,
then the clamp, and 3239's clipped-frame count still counts the final
sum; if not, nothing to do here (3239 puts its limiter after the master
sum this ticket leaves in `render`).

## Implementation steps

1. **The ADR.** Write it (the `write-adr` skill) with the decisions in
   the Scope. List each public item it adds and why each is additive
   (`cargo xtask api` shows none removed). Amend ADR-0091 §5–6 and
   ADR-0073 with a line each pointing to it.
2. **Buses.** In `crates/sound/src/mix.rs`: a `Bus` id type, a bus
   table in `Mixer` (each with its parent and its own `i32` block, all
   allocated in `Mixer::new`), `Config::with_buses`, `Params::with_bus`
   (default the master bus). `render` mixes each voice into its bus's
   block, then adds buses into their parents deepest first, then clamps
   master as now. Keep `mix_alloc.rs` passing: nothing allocated in
   `render`.
3. **The filter.** A new `crates/sound/src/mix/effect.rs`: the sine
   table, `Filter` (kind, cutoff in hertz, Q in hundredths), its
   coefficients in Q2.30, and its state per channel. A bus's filter slot
   is empty by default and costs nothing then.
4. **Commands.** In `crates/sound/src/mix/queue.rs`: `Command`s to set a
   bus's filter and to fade an effect's wet level over whole
   milliseconds (as `FadeTo` does), encoded as words like the others
   (ADR-0073); `Remote` methods for them; `Mixer` methods of the same
   names for a caller on the audio thread.
5. **The manifest.** In `crates/assets/src/audio.rs` (or a new
   `audio/effects.rs` beside `groups.rs`): the `effects` list, its
   entry type, and its checks, each with a problem code. Update
   `docs/formats.md`'s audio manifest part.
6. **Screens and the frame.** In `shuyi-ui`: effect levels faded like
   `DuckState` (`crates/ui/src/audio/duck.rs`), `Ctx::set_effect`, and
   the levels in `FrameOutput`. In `shuyi-platform`: `AudioFrame`'s
   effects view and `with_effects`, the player handing a change to a new
   `Backend` method with a default body, and the headless `Speaker`
   recording levels.
7. **The in-house backend.** In `crates/app/src/audio/in_house.rs`:
   one bus per group from the manifest's tree, each cue played on its
   group's bus, and the `Backend` method sending the commands.
8. **Delay.** A delay slot: delay in whole milliseconds up to
   `Config`'s longest, feedback and wet level in 1/4096, its line
   allocated in `Mixer::new`.
9. **Reverb.** A reverb slot on the buses `Config` names: comb and
   all-pass filters in integers, room size, damping and wet level in
   1/4096. Its memory is taken in `Mixer::new`.
10. **Delay and reverb as data.** Their kinds in the manifest's
    `effects` list, with checks.
11. **Docs.** The audio chapter of the book (2802),
    `docs/platform-backends.md` (which backends play effects), and a
    worked example in the audio manifest's docs: a low-pass on `sfx` and
    `music` turned on by a pause screen.

## Acceptance criteria

- [ ] The ADR is merged in this PR, lists every public item added, and
      ADR-0091 and ADR-0073 point to it.
- [ ] A mixer with one bus and no effects renders exactly today's
      samples: the golden rows `script: mono output` and
      `script: stereo output` in `crates/sound/tests/golden/mix.ron` are
      unchanged.
- [ ] A voice on a child bus is heard through its parent's effect: a
      low-pass on `sfx` filters a cue in `ui` (which is under `sfx`).
- [ ] A low-pass at 1,000 hertz on a 48,000 hertz mix lowers a 4,000
      hertz sine by at least 18 dB and passes a 200 hertz sine within
      1 dB; a high-pass at 1,000 hertz does the reverse; a band-pass at
      1,000 hertz passes 1,000 hertz within 1 dB and lowers 200 and
      5,000 hertz by at least 10 dB.
- [ ] The filter's output stays within one step of a float reference
      (in the test, where floats are allowed) over a noise input, at
      cutoffs from 20 hertz to just under half the output rate.
- [ ] Turning an effect on with a fade of 200 ms moves its wet level in
      a straight line over 200 ms of frames, with no step bigger than
      one block's share.
- [ ] `render` allocates nothing with every slot on (`mix_alloc.rs`).
- [ ] A manifest whose effect names an unknown group, a cutoff at or
      past half the output rate, or a Q of 0 fails `shuyi check` with
      its file, line and problem code.
- [ ] A manifest without `effects` loads and sounds as before.
- [ ] A screen that calls `Ctx::set_effect("muffle", true)` leaves the
      headless `Speaker` holding the effect's level rising to full over
      its fade.
- [ ] The quad-snd backend builds and plays unchanged; its doc says
      effects are not played there.
- [ ] Delay: an impulse comes back after the set delay, to the frame,
      lowered by the feedback each time round.
- [ ] Reverb: an impulse's tail decays below -60 dB within a bound the
      room size sets, and never grows (the sum of squares per block
      falls once the input stops).
- [ ] The golden table has rows for the buses with a filter, the delay
      and the reverb (`mixer: …`), checked on wasm32 by CI's
      `determinism` job.
- [ ] `cargo xtask api` shows no item removed or changed, only added.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the sine table against a float reference; coefficients for each
  kind; bus order with a three-deep tree; a filter slot left empty
  costs nothing (same samples as no bus); commands encoded and decoded.
- Property: any filter settings the checks allow give a stable filter
  (no output past full scale grows from a bounded input over 10 seconds
  of noise); any bus tree adds into master exactly once.
- Snapshot / integration: golden rows in `mix_golden.rs`; the manifest's
  checks with problem codes; a `shuyi-ui` test of `Ctx::set_effect`
  through the headless `Speaker`; `mix_alloc.rs` with every slot on;
  the sound parity speed cell (`crates/xtask/src/sound_parity/mixer.rs`)
  records a block's render time with reverb on beside one without.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
