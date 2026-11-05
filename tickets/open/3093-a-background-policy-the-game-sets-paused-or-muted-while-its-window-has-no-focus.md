---
id: "3093"
title: "A background policy the game sets: paused or muted while its window has no focus"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: ["3042"]
nick_input: none
completed:
---

# 3093 — A background policy the game sets: paused or muted while its window has no focus

## Context

Split out of 3042 by its size rule: 3042's PR would have passed about 600
changed lines with the policy in it, so the policy (its types, the pause
and the mute, with their tests) moved here, as 3042's scope said.

**From 3042 (done):** the macroquad backend sends `SystemEvent::FocusLost`
and `FocusGained` on Windows, macOS, Linux and the web
(`crates/app/src/focus.rs`, ADR-0076); `App::on_system_event`
(`crates/platform/src/app.rs`) sets `focused` on them, and on `FocusLost`
lets go of held input (`Game::forget_held_input`) before the game's hook;
`App::is_focused()` says the state; the pads are ignored while unfocused
(`crates/app/src/pads.rs`). What 3042 did not build is a way for the game
to say what happens while it has no focus: today it keeps running and
keeps its sound, and a game that wants to stop or go quiet writes it in
its `SystemHook`.

The engines compared on 2026-10-20 each have one: Unity's
`Application.runInBackground`, Godot's
`input_devices/joypads/ignore_joypad_on_unfocused_application` and its
focus notifications, Bevy's `WinitSettings::unfocused_mode`. The end goal
(2026-10-19, `docs/decisions.md`): an agent's game gets the pause and the
mute from one constant, not a hook of its own.

Rules it keeps: the pause is "no update", so replays and hashes don't
change (ADR-0002); `shuyi-ui`'s settings stay `no_std`.

## Nick input

None. The engine's default keeps today's behaviour (keep running, keep
the sound), so the game gets nothing new until it opts in. Whether
Visions of Shuyi pauses or mutes in the background is the game's choice,
made in its own repository.

## Scope

**In:**
- **The policy**, in `shuyi-ui` (`crates/ui/src/settings.rs`, `no_std`):
  - `Background { pub pause: bool, pub audio: BackgroundAudio }`, and
    `BackgroundAudio::Keep` and `Mute`, both `#[non_exhaustive]` (ADR-0039)
    so 3043's frame cap and a later duck can join; a const
    `Background::KEEP_RUNNING` (no pause, `Keep`) and const builders
    `paused(self)` and `muted(self)`, so a game writes
    `Background::KEEP_RUNNING.paused().muted()`;
  - `GameSettings::BACKGROUND: Background`, an associated const that
    defaults to `Background::KEEP_RUNNING`, so every game compiles and
    behaves as before, beside `DEFAULT_FRAME_CAP` and `FRAME_CAP_CHOICES`.
- **`App` applies it while unfocused** (not while suspended, which
  already stops everything; sound on `Suspend` is 2394's). `App` keeps the
  policy in a private field set from `<G::Settings as
  GameSettings>::BACKGROUND` when it is made, so its own tests (a child
  module) can set another policy on the toy:
  - **pause:** while the game runs and is unfocused, no update: the input
    is drained and dropped, no fixed steps, the screens see no time, and
    the game's last frame is presented again each frame. The loading card
    and the load go on as before. `FocusGained` restarts the clock as
    `Resume` does, so no catch-up steps. `QuitRequested` still quits.
    `App::is_paused()` says so;
  - **mute:** while unfocused, every frame's `AudioFrame` carries its
    requests as usual with the music, sound and voice volumes at 0, so
    the music keeps its place and a sound started meanwhile is silent
    rather than lost. The music and a voice already playing go quiet at
    once; a sound already playing plays out, since a sound's volume is
    set when it starts (`crates/app/src/audio.rs`). While paused and
    muted, `App` still calls `audio().play` each frame with no requests
    and the volumes at 0. The first frame after `FocusGained` sends the
    player's volumes again;
  - **Either order with 2342:** if 2342 is done, mute hands the frame
    `Gains::muted()` instead of three volumes at 0; if not, 2342 carries
    this over (its clause says so);
  - the game's hook still hears `FocusLost` and `FocusGained` after
    `App`'s part.
- **Either order with 2739:** if 2739 is done, `examples/snake` and the
  `stage-arcade-grid` part of `templates/shuyi-default/` set
  `GameSettings::BACKGROUND` to `Background::KEEP_RUNNING.paused()`; if
  not, nothing to do (2739 does it when it is written).
- **ADR-0076** gains no edit: write a short ADR (the `write-adr` skill)
  extending it with the policy and why the default changes nothing, and
  name it in ADR-0076's row of `docs/adr/README.md` ("extended by").
- **`docs/platform-backends.md`:** the `FocusLost` and `FocusGained` row
  of the `system_events` table says `App` applies the game's policy;
  *What one frame does* gains the paused branch.
- **What changes for the game:** additive only (`GameSettings::BACKGROUND`
  with a default, `App::is_paused`, the `Background` types). The game's
  PR that moves the pin follows this one.

**Out (do not do):**
- the player's Mute in background setting and fewer frames in the
  background: 3043, which builds on `Background`;
- holding sound on `Suspend`, and looped sounds that follow the mute:
  2394;
- a pause menu that opens on focus loss: a game's hook can push one;
- a background policy for the other templates and examples: they keep
  the default (the real-time grid's is the clause on 2739 above).

## Implementation steps

1. Read `crates/platform/src/app.rs` (`on_system_event`, `run`, `step`,
   `present_again`) and its tests (`crates/platform/src/app/tests.rs`),
   `crates/ui/src/settings.rs`, ADR-0024 §5 and ADR-0076. Check whether
   2342 and 2739 are done.
2. `Background`, `BackgroundAudio` and `GameSettings::BACKGROUND`, with
   tests of the builders.
3. `App`: the policy field, the pause and the mute, `is_paused`, with
   tests through `HeadlessPlatform`.
4. The ADR and `docs/platform-backends.md`, then the gates.

## Acceptance criteria

- [ ] `Background` builder tests: `KEEP_RUNNING` keeps running with its
      sound; `paused()` and `muted()` each set only their part, in either
      order.
- [ ] `App` tests with the policy set to `paused()`: after `FocusLost`,
      frames run no update and no steps and present the last frame; after
      `FocusGained` the first frame counts no time, and a 10-minute gap
      steps at most once; `QuitRequested` while paused quits.
- [ ] `App` tests with `muted()`: while unfocused the `AudioFrame` volumes
      are 0 and its requests are still passed; paused and muted, `play` is
      called with no requests at volume 0; after `FocusGained` the
      player's volumes come back.
- [ ] With the default policy, every existing Harness, snapshot and
      replay test passes unchanged.
- [ ] The ADR is written and ADR-0076's row names it.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Background`'s builders.
- Integration: `App` through `HeadlessPlatform` for the pause and the
  mute.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
