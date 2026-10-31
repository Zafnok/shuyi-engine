---
id: "3003"
title: "The platform boundary: a shuyi-platform crate with the Platform trait and App::frame, and shuyi-app as its macroquad backend"
type: feature
milestone: After MVP
model: reasoning
effort: xhigh
status: done
blocked_by: ["game#1131"]
nick_input: none
completed: 2026-10-20
---

# 3003 — The platform boundary

## Context

[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§2. Today `shuyi-app` (macroquad) owns both the devices and the run loop:
input mapping, the screen stack, the tick driver, and drawing. The
Playdate (3010) can't use macroquad, and closed console backends will
live in private repositories. So the run loop has to move into a crate
that every backend calls, and the devices have to sit behind a small
trait.

Each backend owns its loop:

- macroquad's async loop;
- the Playdate SDK's update callback;
- a console's main loop.

The engine provides `App::frame`, which the backend calls once per
frame with the time.

## Nick input

None. The game keeps depending on `shuyi-app`, and only its `main`
changes (a few lines).

## Scope

**In:**
- **A new crate, `shuyi-platform`**, between `shuyi-ui` and the
  backends:
  - the `Platform` trait, with `display`, `present`, `poll_input`,
    `system_events`, `storage`, `audio` and `text_input` (ADR-0008 §2
    gives the shape; fix the names here);
  - `Storage`, `Audio` and `TextInput`, and `SystemEvent` (`Suspend`,
    `Resume`, `FocusLost`, `FocusGained`, `QuitRequested`, `LowBattery`);
  - `App<G>`, which holds the screen stack, input state and tick driver
    moved from `shuyi-app`, with `App::frame(&mut self, p: &mut impl
    Platform, now: Millis) -> FrameOutcome` (`Continue` or `Quit`).
- **Suspend and resume.** On `Suspend`, `App` saves through the game's
  hook and stops stepping. On `Resume`, it resets the tick accumulator,
  so that a sleep doesn't become a burst of catch-up steps (ADR-0003 §2's
  cap of 4 still holds).
- **`shuyi-app` becomes the macroquad backend.** It implements
  `Platform` with macroquad, gilrs and the audio it already uses, and
  `shuyi_app::run(app)` drives `App::frame` from macroquad's loop on
  desktop and the web.
- **A headless backend.** `shuyi_platform::headless::HeadlessPlatform`
  records presented frames and plays scripted input. The Harness uses it
  instead of its own driver, so tests run the same `App::frame` the
  devices do.
- **A loading phase** (from the game's friction sweep, game#1153): a
  game's first load can take seconds on a slow device (3.7 s on the
  Playdate), and a backend shows nothing until the first frame. `App`
  starts in a loading state: its first frame paints a card the game
  gives (a picture or a line of text), the next frames run the game's
  load, then the title. The game's Playdate front end does this by hand
  (`src/pd/entry.rs`, `Step::ShowCard`, `Load`, `Running`).
- **Docs.** `docs/platform-backends.md` explains how to write a backend:
  the trait, what each method must do, the headless backend as a
  reference, and the rule that console backends stay private.
- **Rules.** ADR-0008 §2 already adds `shuyi-platform` to ADR-0002 §1's
  crates (ADR-0002 stays unedited; its index row says "extended by
  ADR-0008"). `AGENTS.md` rule 5 already says "platform backends". Check
  that both still match what was built.

**Out (do not do):**
- `no_std` (3005);
- the Playdate backend (3010);
- the 1-bit output (3006); `present` takes the RGBA frame for now;
- Steam's keyboard behind `TextInput` (the game's 0903);
- more than one screen (the parked 3023, ADR-0012 §5). Keep
  `DisplayInfo` and `present` open to a list of surfaces later.

## Implementation steps

1. Create `shuyi-platform` and move the run loop out of `shuyi-app`
   without changing behaviour. The existing Harness and snapshot tests
   must pass unchanged at each step.
2. Define the trait and `App::frame`. Implement it for macroquad in
   `shuyi-app` and for the headless backend.
3. Point the Harness at `HeadlessPlatform`.
4. Add the suspend and resume handling, with tests.
5. Update the examples' `main`s. Write the game's matching change as a
   game ticket and link it here.
6. Write `docs/platform-backends.md`.

## Acceptance criteria

- [x] `shuyi-platform` doesn't depend on macroquad, gilrs or any audio
      crate (`cargo tree` check in CI).
- [x] Every example runs on desktop and the web as before (the web
      build gate and a manual run).
- [x] All existing Harness, snapshot and replay tests pass unchanged.
- [x] A test: `Suspend`, then `Resume` 10 minutes later, steps the tick
      simulation at most once, not 36 000 times.
- [x] The game's ticket for its `main` change exists and is linked.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `App::frame` with the headless backend: input, then actions,
  then a step, then a presented frame.
- Integration: every example's Harness tests through
  `HeadlessPlatform`; suspend and resume.

## Completion notes

**Done** (the decisions are
[ADR-0024](../../docs/adr/0024-the-platform-boundary-app-runs-a-frame-time-in-milliseconds-storage-handed-over-and-the-harness-on-the-headless-backend.md)):

- **`shuyi-platform`** (`crates/platform`), depending on `shuyi-assets`
  and `shuyi-ui` only:
  - `Platform` with `display`, `present`, `poll_input`,
    `system_events`, `storage`, `audio` and `text_input`; `Millis`,
    `DisplayInfo`, `Frame`/`Show`/`Look`, `SystemEvent` (the six asked
    for, and `ToggleFullscreen`), `Audio`/`AudioFrame`/`SoundRequest`,
    `TextInput`, and `Storage` re-exported from `shuyi-ui`;
  - `App<G>`: `App::frame(&mut self, p, now: Millis) -> FrameOutcome`,
    the loading phase (`Start`, `Card`, `Setup`), suspend and resume,
    the game's hook for the system's events, and `tick::TickClock` (60
    steps a second, at most 4 a frame, in integers);
  - `headless::HeadlessPlatform` (scripted input and events, the last
    frame kept, memory storage, the `Speaker` music stand-in, a scripted
    keyboard), and the Harness over it (`shuyi_platform::harness`).
- **`shuyi-app`** is the macroquad backend: `Macroquad` implements
  `Platform`; `shuyi_app::run(config, start)` calls `App::frame` from
  macroquad's loop. It decodes the game's sounds and starts the pads
  between frames once the game has loaded, and sends `QuitRequested`
  when the window's close button is pressed on native.
- **Suspend and resume:** `Suspend` saves through the game's hook and
  stops stepping, presenting and taking input; `Resume` forgets the time
  away. `app::tests::suspended_ten_minutes_the_game_steps_at_most_once_after_resuming`
  checks it through `App::frame`, and hello-grid's `suspend` test through
  its Harness.
- **The loading phase:** frame 1 presents the card, frame 2 runs the load
  and presents the game's first frame, and the game's clock starts after
  the load. hello-grid and both templates show their name on black.
- **CI:** a `platform-deps` job runs `cargo tree` on `shuyi-platform`
  (every target, every feature) and fails on macroquad, miniquad,
  `quad-*`, gilrs or an audio crate. The `run-gates` skill has the
  command.
- **Docs:** `docs/platform-backends.md`; the README's crate table;
  `docs/carve.md`. AGENTS.md rule 5 already says "platform backends" and
  ADR-0002's index row already says "extended by ADR-0008": both match.
  ADR-0008's row now also names ADR-0024. The `work-ticket` skill's
  layer line says "platform backends".
- **Manual runs:** hello-grid on Windows opened its window, ran, and quit
  with code 0 when its window was closed (the new `QuitRequested` path).
  Its web build (`cargo shuyi web --release`) ran in the browser pane:
  the title drew, and each Up press moved the menu one row in the frame
  of the press. The pane was hidden, so frames were driven from the
  page's JavaScript (`wasm_exports.frame()`).

**Deviations:**

- **`App` owns `shuyi_ui::Game`**, which keeps the screen stack and the
  input state; they didn't move out of `shuyi-ui`. There was no tick
  driver in `shuyi-app` to move: `TickClock` is new, and the steps reach
  the top screen as `FrameInput::ticks()` (set by `Game::set_ticks`).
  Ticket 1119's text now says its `TickDriver` reads them.
- **`storage` hands the storage over** (`Box<dyn Storage>`, once, at the
  load) instead of lending `&mut dyn Storage`: the game's context owns it.
- **The audio at the boundary names voice clips by file**
  (`SoundRequest`), so `Platform` isn't generic over the game's voice
  type. `shuyi-app`'s audio player takes them; its tests pass the
  screens' requests through a small test-only `hear`. The player's rules
  are unchanged (3029 moves them).
- **`App::frame_after(p, seconds)`**, besides `frame`: the Harness's
  frames are exactly 1/60 s, which whole milliseconds can't give, and its
  tests compare frame times to `f32::EPSILON`.
- **The Harness moved to `shuyi_platform::harness`**, and `shuyi-ui`'s
  `harness` feature is gone. Every existing test passes with only its
  `use` line changed; no snapshot changed. `shuyi-ui`'s integration tests
  reach it through a dev-dependency (a cycle Cargo allows for tests).
- **`TextInput` is wired, not just declared:** a screen asks through
  `Ctx::system_keyboard` (new, `shuyi_ui::system_keyboard`), `App` opens
  the keyboard and hands the answer back. `shuyi_app::AppConfig` has a
  `text_input` slot for a game's (Steam's is game 0903).
- **`shuyi-platform` builds without `std`** already (checked for
  `thumbv7em-none-eabihf` here); the CI gate stays 3005's.
- The ADR is **0024**: 0023 is taken by the open PR for 2204.

**For the game:** `shuyi_app::run` takes a `Start`; `AppConfig` has no
type parameter, no `font` or `images`, and a `text_input` field;
`Platform::setup` is `Setup::ctx`; the Harness is
`shuyi_platform::harness` with `shuyi-platform`'s `harness` feature; the
game's `cargo xtask engine` needs `shuyi-platform` in its `CRATES`.
Game ticket 1175 says all of it
([`Zafnok/visions-of-shuyi#374`](https://github.com/Zafnok/visions-of-shuyi/pull/374)
adds it, and puts it in 1170's `blocked_by`). The game PR that moves
the pin is 1175's, not opened yet; this engine PR merges first.

**After 2204 and 3001 landed** (merged into this branch): nothing in
them changes the boundary. 2204's packs are read by a game's own load
(`Hello::from_pack` fits inside `Start`'s load), and pictures and sounds
stay files the backend reads. 3001's target profiles name a display's
colour as `shuyi_assets::target::Color`, so `DisplayInfo::color` uses that
type instead of a `ColorDepth` of its own. A profile's `backend` names the
crate that implements `Platform` (`shuyi-app`).

**Follow-ups:** none new. The macroquad backend sends no `Suspend`,
`Resume` or focus events yet: the Steam Deck's are 3012's, Android's
3015's.

**For Nick:** nothing to do. A game now shows its name on black for a
moment while it loads, and closing its window lets it save first.
