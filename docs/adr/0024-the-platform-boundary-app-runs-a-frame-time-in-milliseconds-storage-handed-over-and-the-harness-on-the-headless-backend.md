# ADR-0024: The platform boundary as built: App runs a frame, time in milliseconds, storage handed over, and the Harness on the headless backend

- **Status:** Accepted
- **Date:** 2026-10-20
- **Related tickets:** 3003; 3005 (`no_std`), 3006 (1-bit output), 3010
  (the Playdate backend), 3012 (the Steam Deck), 3023 (surfaces), 3029
  (audio for every backend), 1119 (tick simulation); game 0903 (Steam),
  game 1170 (the Playdate front end)
- **Extends:** ADR-0008 §2 (platform backends), ADR-0003 §2 (the frame
  driver's accumulator)

## Context

ADR-0008 §2 decided that a crate, `shuyi-platform`, sits between
`shuyi-ui` and the backends: a `Platform` trait the backend implements,
and `App::frame(&mut self, platform, now)`, which the backend's own loop
calls. It gave the trait's shape and left the names to ticket 3003.

What 3003 found in the code:

- **The run loop** was `shuyi_app::run`: it read the keyboard and the
  controllers, told the game what music sounded, called
  `shuyi_ui::Game::frame` with macroquad's frame time, switched the
  window to full screen, played the frame's audio and blitted its buffer.
  `Game` already owned the screen stack and the input state.
- **There was no tick driver.** Fixed simulation steps (ADR-0003 §2) are
  ticket 1119's, parked. 3003 still asks that a resume after ten minutes
  step the tick simulation at most once.
- **The game's look, sounds and stick thresholds are in its data**
  (`AppConfig::font`, `images`, the audio manifest, the keymap's
  `stick`), which only exists once the game has loaded.
- **macroquad decodes sounds asynchronously** on the web
  (`load_sound_from_bytes` waits frames), so loading them can't happen
  inside a synchronous `App::frame`.
- **The screens' audio requests are generic** over the game's voice clip
  variant (`AudioRequest<G::Voice>`); a trait object (`&mut dyn Audio`)
  can't take a generic method.
- **The game's context owns its storage** (`Ctx::storage`, a
  `ProfileStore` over a `Box<dyn Storage>`): screens save through it in
  their update.
- **The Harness** (`shuyi_ui::harness`) drove `Game::frame` itself, in
  frames of exactly 1/60 s, and its tests compare frame times to
  `f32::EPSILON`.
- **The game's Playdate front end** shows a loading card on its first
  frame, loads on the second, then runs, and restarts its frame clock
  after the load (game `crates/playdate/src/pd/entry.rs`).

## Decision

### 1. The crate and what moves

- `crates/platform`, package `shuyi-platform`, depends on `shuyi-assets`
  and `shuyi-ui` only. It touches no device. It is `no_std` + `alloc`
  without its default `std` feature (3005 gates that). CI's
  `platform-deps` job fails if `cargo tree` finds macroquad, miniquad, a
  `quad-*` crate, gilrs or an audio crate under it, on any target, with
  any feature.
- `App<G>` owns the game (`shuyi_ui::Game`, which keeps the screen stack
  and the input state) and adds what a backend needs around it: the
  clock and the tick accumulator, suspend and resume, the loading phase
  and the per-frame handshakes below.
- The test Harness moves to `shuyi_platform::harness` (the `harness`
  feature, which leaves `shuyi-ui`). It runs the game in an `App` on
  `shuyi_platform::headless::HeadlessPlatform`. Its script API is
  unchanged. `shuyi-ui`'s integration tests reach it through a
  dev-dependency, a cycle Cargo allows for tests.
- `shuyi-app` becomes the macroquad backend: `Macroquad` implements
  `Platform`; `shuyi_app::run(config, start)` calls `App::frame` once per
  macroquad frame on desktop and the web. `AppConfig` loses its game type
  and its `font` and `images` (they are the game's `Start`'s now) and
  gains `text_input`.

### 2. The trait

```rust
pub trait Platform {
    fn display(&self) -> DisplayInfo;                       // device pixels and colour depth
    fn present(&mut self, frame: &Frame<'_>);               // once per frame not suspended
    fn poll_input(&mut self, out: &mut Vec<RawInputEvent>); // since the last poll, in order
    fn system_events(&mut self, out: &mut Vec<SystemEvent>);
    fn storage(&mut self) -> Box<dyn Storage>;              // handed over once, at the load
    fn audio(&mut self) -> &mut dyn Audio;
    fn text_input(&mut self) -> Option<&mut dyn TextInput>;
}
```

- **`storage` hands the storage over** instead of lending it: the game's
  context owns it, and screens save through it mid-update. `App` calls it
  once, in the load. `Storage` stays in `shuyi-ui` (the context needs it);
  `shuyi-platform` re-exports it.
- **`Frame`** is `#[non_exhaustive]`: `show` (`Show::Card`, `Show::Errors`
  or `Show::Game { buffer, background, look }`) and `fullscreen`. A game
  frame carries its `Look` (the game's `FontAtlasDef` and `ImageTable`), so
  a backend makes its textures from the first one it sees: the look is in
  the game's data, which loads in a frame. The buffer is the full-colour
  `GlyphBuffer`; 3006 adds the 1-bit form. `DisplayInfo` and `Frame` are
  non-exhaustive so 3023 can add surfaces without breaking a backend.
  `DisplayInfo::color` is `shuyi_assets::target::Color`, the type a target
  profile's display names (3001), so a backend's display and a target's
  profile say colour depth the same way.
- **`SystemEvent`**: `Suspend`, `Resume`, `FocusLost`, `FocusGained`,
  `QuitRequested`, `LowBattery`, and `ToggleFullscreen` (the window's own
  shortcut, which flips the player's Fullscreen setting, as `shuyi-app`'s
  loop did). Non-exhaustive.
- **`Audio`** takes one frame at a time, without the game's types:

  ```rust
  pub trait Audio {
      fn music_playing(&self, now: Millis) -> Option<(&str, f64)>;
      fn play(&mut self, frame: &AudioFrame<'_>, now: Millis);
      fn stop_all(&mut self);
  }
  ```

  `AudioFrame` holds the frame's `SoundRequest`s (a sound cue, a voice
  clip by its file in the voice folder, stop the voice, the clips coming),
  its `MusicCommand`s and the three volumes. `App` turns each
  `AudioRequest<G::Voice>` into a `SoundRequest` with
  `shuyi_assets::voice::clip_file`, so every backend is the same for
  every game. The rules of playing stay in each backend's player until
  3029 moves them.
- **`TextInput`** is the system's own keyboard: `open(&TextAsk) -> bool`
  and `poll() -> Option<TextAnswer>`. A screen asks through
  `Ctx::system_keyboard` (`shuyi_ui::system_keyboard`, a one-frame
  handshake), `App` opens the keyboard after that frame and hands the
  answer back before a later one. Where `text_input` is `None` a screen
  reads typed keys, as before. `shuyi_app::AppConfig::text_input` is where
  a game's build gives one (Steam's, game 0903).

### 3. Time is whole milliseconds; the driver may bring its own

- `App::frame(&mut self, platform, now: Millis) -> FrameOutcome`.
  `Millis(u64)` is the backend's clock; only differences count, and a
  clock that goes backwards passes no time. The screens get the
  difference in seconds; the first frame, and the first after a load or a
  resume, count none.
- `App::frame_after(&mut self, platform, seconds: f32)` runs the same
  frame on a clock of the driver's own: the Harness's frames of exactly
  1/60 s, which whole milliseconds can't give, or a backend whose system
  says how long the last frame took. A driver uses one or the other.
- `FrameOutcome` is `Continue` or `Quit`. After a quit `App` presents the
  last frame in silence, for a backend (a web page) that can't close.

### 4. Fixed steps: the accumulator is `App`'s

- `shuyi_platform::tick::TickClock` turns each frame's time into steps:
  60 a second, at most 4 a frame (`MAX_STEPS`), the rest of a late frame
  dropped (ADR-0003 §2). It counts sixtieths of a nanosecond in a `u64`,
  so 16, 17, 17 ms frames give exactly 60 steps a second.
- The steps reach the top screen as `FrameInput::ticks()`, set by
  `Game::set_ticks` before each frame (like `Game::set_music_playing`).
  One accumulator for the whole game, not one per screen, so a resume
  resets it once. 1119's `TickDriver` reads `ticks()` instead of keeping
  an accumulator of its own.

### 5. Suspend, resume and the system's events

- **`Suspend`**: `App` stops stepping (no update, no input, nothing
  presented) until **`Resume`**, which restarts the clock and empties the
  accumulator: the first frame after counts no time, so a sleep never
  becomes catch-up steps.
- **`QuitRequested`**: `App` quits (`FrameOutcome::Quit`) in every
  phase. The macroquad backend sends it when the window's close button is
  pressed on native (`prevent_quit`), so a game can save on close.
- **The game's hook**, `fn(&mut Game<G>, SystemEvent)`, hears every event
  after `App` has done its part, until the game quits. `Suspend`,
  `QuitRequested` and `LowBattery` are where a game saves; `FocusLost` is
  where it may pause.
- The macroquad backend sends `ToggleFullscreen` and `QuitRequested`
  today. `Suspend` and `Resume` on the Steam Deck (a gap of more than a
  second between frames) are 3012's; Android's lifecycle is 3015's.

### 6. The loading phase

- A game starts as a `Start<G>`: a `Card` (a line of text, or a picture
  from the game's files, on a background), its `load:
  FnOnce(Setup) -> Result<Game<G>, String>`, its look (`fn(&G) -> Look`)
  and its hook.
- Frame 1 presents the card, in the backend's own font or picture
  decoder (the game's font isn't loaded yet). Frame 2 runs the load with
  the card on the screen, then presents the game's first frame, drawn
  before any update. Frames after run the game. A failed load shows its
  errors (`Show::Errors`) until the player quits.
- `Setup` is what the platform gives the load: the display, the storage,
  a seed from the clock (mixed, so two launches a millisecond apart
  differ), the voice manifest, whether there is a keyboard and whether to
  wait for a key. `Setup::ctx` puts it into the game's context, as
  `shuyi_app::Platform::setup` did. A backend adds its own part with
  `Start::before_load` (`shuyi-app` adds the voice manifest it read).
- What the macroquad backend can only do asynchronously (decoding
  sounds), and what needs the game's data (its stick thresholds), it does
  between frames once `App::game` is `Some`.

## Consequences

- A backend is the trait and a loop: the headless backend is about two
  hundred lines, the macroquad one about three hundred. A console's,
  in a private repository, implements the same seven methods.
- Every Harness test runs the frame a device runs: the same order of
  events, input, update, audio and present. Frame times in the Harness
  stay exact.
- **For the game** (at its next pin bump):
  - its `main` builds a `Start` (a card, its load over `Setup`, its look)
    and calls `shuyi_app::run(config, start)`; `AppConfig` has no type
    parameter, no `font` or `images`, and a `text_input` field;
  - its tests import `shuyi_platform::harness` instead of
    `shuyi_ui::harness`, and its `harness` feature is
    `shuyi-platform/harness`;
  - game ticket 1170 builds the Playdate front end on 3010's backend,
    which drives this `App`.
- The macroquad backend's music clock and frame times are whole
  milliseconds (they were `f64` seconds). The music clock still reads at
  most a frame old.
- Closing the window on native now takes one more frame: the game hears
  `QuitRequested` and `App` quits.
- A frame's audio requests are copied once more each frame (into
  `SoundRequest`s); a frame has a handful at most.

## Alternatives considered

- **A generic `Platform<V>` or `Audio<V>` over the game's voice type** —
  every backend would be generic over every game, and the headless one
  could no longer be one type. Naming the clip's file at the boundary
  costs one conversion.
- **`storage(&mut self) -> &mut dyn Storage`, as ADR-0008 sketched** —
  the context would have to borrow the storage each frame, and every
  screen that saves goes through the context. Handing it over once keeps
  `Ctx` as it is.
- **Floating-point seconds for `now`** — the tick accumulator would round
  differently from frame to frame. Whole milliseconds are what the
  Playdate's SDK gives, exact enough for screens, and exact for the steps.
- **Keeping the Harness in `shuyi-ui` with its own driver** — tests would
  check a loop no device runs; 3003 asks that they run `App::frame`.
- **An accumulator per tick screen (1119's first sketch)** — a resume
  would have to reach into every screen to empty it.
- **The game's look as a call on the backend after the load** (a
  `Platform::loaded` method) — an eighth method every backend writes; the
  frame carrying references to the look costs nothing.
- **Loading in the first frame, with no card** — a slow device shows
  nothing for seconds (3.7 s on the Playdate), which is why the game's
  Playdate front end drew one by hand.
