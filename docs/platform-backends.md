# Writing a platform backend

A **backend** puts a game on a device: a window and a sound card on a
computer, the SDK on a Playdate, a console's own libraries. Everything
else (the screens, the input rules, the frame, the simulation) is the
engine's and is the same everywhere. This page says what a backend must
do. The decisions behind it are
[ADR-0008](adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§2 and
[ADR-0024](adr/0024-the-platform-boundary-app-runs-a-frame-time-in-milliseconds-storage-handed-over-and-the-harness-on-the-headless-backend.md).

| Backend | Crate | Where |
| ------- | ----- | ----- |
| macroquad: Windows, Linux, macOS, the web, the Steam Deck | `shuyi-app` | here, `crates/app` |
| headless: tests, tools, benchmarks | `shuyi_platform::headless` | here, `crates/platform` |
| Playdate | `shuyi-playdate` | here, from ticket 3010 |
| a closed console | its own crate | **a private repository**, never here |

## The rules

- **The backend owns the loop.** macroquad's async loop, the Playdate
  SDK's update callback, a console's main loop: whichever it is, it calls
  [`App::frame`](../crates/platform/src/app.rs) once per frame with the
  time on its clock. The engine never loops and has no clock of its
  own: in debug builds `App` measures frames with the one the backend
  hands it (`clock`, below).
- **Only a backend touches a device** (`AGENTS.md` rule 5): windows,
  files, audio devices, controllers, the clock. `shuyi-platform` itself
  touches none, and `cargo xtask check-layers` (CI's `layers` job) keeps
  macroquad, gilrs and audio crates out of it.
- **Nothing of a game in a backend** (`AGENTS.md` rule 4). What a game
  wants of its backend (a card, a voice folder) it says through `Start`
  and the backend's config.
- **Console backends stay private.** The SDKs of Nintendo, Sony and
  Microsoft, and the code written against them, are under non-disclosure
  agreements: a console backend lives in a private repository under that
  console's agreement, depends on the engine's crates like any game, and
  is never merged here, not even behind a feature. The engine's part is
  to keep `Platform` small, stable and documented.
- **`Platform` stays small** (ADR-0040 §7). A backend implements the
  seven methods below, and may give the eighth and ninth, `clock` and
  `memory`, which have defaults. Every method added to `Platform` later has a
  default, and a device service a backend may lack returns an `Option`,
  as `text_input` does, so a backend written today (a private console's)
  keeps building. `crates/platform/tests/it/bare.rs` implements only the
  seven and fails to compile if one without a default is added.
- **Licenses.** A public backend's dependencies follow ADR-0002 §8, like
  the rest of the engine.
- **Small devices are `no_std`.** `shuyi-platform`, `shuyi-ui`,
  `shuyi-core` and `shuyi-assets` build without `std` (their default
  `std` feature off; ticket 3005 gates it), so a backend for a device
  without a standard library can use them.

## The loop

```rust
let mut platform = MyPlatform::new(/* the device's handles */);
let mut app = shuyi_platform::App::new(start); // the game's `Start`
loop {
    let now = Millis(device_clock_ms());
    let outcome = app.frame(&mut platform, now);
    for line in app.take_new_log() {
        device_log(line.level, line.category.name(), &line.text);
    }
    if outcome == FrameOutcome::Quit {
        break; // or keep calling `frame`, which shows the last frame
    }
    wait_for_next_frame();
}
```

`app.take_new_log()` gives each line of the game's log once, after the
frame it was logged in: a `shuyi_ui::log::LogLine` with its frame, its
level (`Error`, `Warn`, `Info`, `Debug`), its category and its English
text. Hand each to the device's own log (`shuyi-app` prints them with
macroquad's macros). `App` keeps the last 500 itself (`app.log()`), for
tests and tools, and `app.set_log_filter` says which levels it keeps
(`Info` and more serious by default).

`App::frame` is synchronous. Anything the device can only do
asynchronously, or only once the game's data exists (decoding its sounds,
reading its stick thresholds), the backend does **between** frames, once
`app.game()` is `Some` (`shuyi-app`'s `run` and `Macroquad::start` do).

A backend whose system says how long the last frame took, rather than
what time it is, calls `App::frame_after(&mut platform, seconds)`
instead. A backend uses one or the other.

## What one frame does

`App` calls the platform in this order:

1. `system_events`: a `Resume` restarts the clock before it is read
   (in a frame it measures, `App` first reads the time on `clock`'s
   clock: the frame's start);
2. the clock: the time since the last frame, and the fixed steps due
   (60 a second, at most 4 a frame);
3. while suspended, `poll_input` (dropped) and nothing else: no update,
   nothing presented;
4. `poll_input`;
5. `text_input`: whether there is a system keyboard, and its answer if a
   screen's ask waits for one;
6. `audio().music_playing(now)`, for the screens that keep time with the
   music;
7. the game's update (`shuyi_ui::Game::frame`);
8. `audio().play(&frame, now)` with that frame's audio, every frame;
9. `text_input().open(&ask)` if a screen asked for the system keyboard;
10. `audio().stop_all()` if the game quit in that update;
11. `present(&frame)` (timed, in a frame `App` measures, and the
    frame's end read after it).

The game's first frames are different: frame 1 presents the loading card;
frame 2 calls `display` and `storage` for the game's `Setup`, runs the
game's load (seconds, on a slow device) and presents the game's first
frame; frames after run the game. A failed load presents its errors until
the player quits.

## Each method

### `display(&self) -> DisplayInfo`

The display as it is now, in device pixels, and whether it shows colour
(`Color::Rgba8`) or black and white (`OneBit`): the same
`shuyi_assets::target::Color` a target profile's display names (ADR-0008
§1). `App` hands it to the game's load (`Setup::display`), where a game
may pick its console to fit.

### `present(&mut self, frame: &Frame)`

Show the frame as soon as possible and keep it on the screen until the
next. `frame.show` is one of:

- `Show::Card(&Card)`: the loading card, while the game's data loads.
  Paint it with what the backend has of its own, since the game's font
  isn't loaded: `Card::Text` is a line centred in the backend's font, in
  `ink` on `background`; `Card::Picture` is an image from the game's
  files by path, centred at the largest whole scale that fits.
- `Show::Errors(&str)`: why the game couldn't start. Show the text (the
  backend's font, wrapped). `App` logs it once, at `Error`, so the
  backend needn't.
- `Show::Game { buffer, background, look, .. }`: the game's console, a
  `GlyphBuffer` of glyph cells and sprite items in full colour, on
  `background` (also the letterbox's). `look` holds the game's font atlas
  and image table: make the backend's textures (or 1-bit glyphs) from
  them the first time and keep them. If they can't be made, show why
  instead, as for `Errors`.

`frame.fullscreen` is the player's Fullscreen setting: a windowed backend
switches when it changes; a device that is always full screen ignores it.
`Show`, `Frame` and `DisplayInfo` are `#[non_exhaustive]`: match with a
`_` arm, since later tickets add to them (1-bit frames, ticket 3006; more
than one screen, 3023).

### `poll_input(&mut self, out: &mut Vec<RawInputEvent>)`

Add the input since the last call, in the order it happened:

- keys as `Down(Chord)` and `Up(Key)` (the engine's `Key` names, with
  Shift in the chord), and typed characters as `Text(char)` after the
  key's `Down`;
- controller buttons by position: read every pad's state and give it to
  `shuyi_ui::input::Pads`, then `RawInputEvent::from_pads`, which gives
  releases then presses and holds the rules (sticks as directions, the
  Switch-style swap, several pads as one);
- a crank through `RawInputEvent::from_crank`.

Translate only: which action a key means is the player's keymap's, never
the backend's (`AGENTS.md` rule 8). Keys the system owns (a console's home
button, the Steam button, the Playdate's Menu) never reach the game.

### `system_events(&mut self, out: &mut Vec<SystemEvent>)`

Add what the system said since the last call:

| Event | When | What `App` does |
| ----- | ---- | --------------- |
| `Suspend` | the device sleeps or locks, the home menu opens, the app goes to the background | the game's hook saves; no update and nothing presented until `Resume` |
| `Resume` | the game runs again | the time away is forgotten: the next frame counts none, so no catch-up steps |
| `FocusLost`, `FocusGained` | an overlay or another window takes the player's attention, and gives it back | the game's hook hears it (and may pause) |
| `QuitRequested` | the player closes the game (a window's close button) | the game's hook saves, then `FrameOutcome::Quit` |
| `LowBattery` | the battery is nearly empty | the game's hook may save |
| `ToggleFullscreen` | the window's own fullscreen shortcut (F11, Alt+Enter) | the Fullscreen setting flips |

A device that freezes the process without a word (the Steam Deck asleep)
sends `Resume` itself when it sees a long gap between frames (ticket
3012).

### `storage(&mut self) -> Box<dyn Storage>`

Hand over the game's storage: its saves, settings and key bindings, by
key (`^[a-z0-9_-]{1,64}$`, which a backend may turn straight into a file
name). `App` calls it once, as the game loads, and the game's context
keeps it. It must pass `shuyi_ui::storage::conformance_suite` (the
`storage-conformance` feature), in the backend's tests and, where there is
a debug build on the device, on the device's real files.

### `audio(&mut self) -> &mut dyn Audio`

The sound device:

- `music_playing(now)`: the music cue sounding and the seconds since it
  really started (once its file loaded), or `None`;
- `play(&frame, now)`: one frame's audio. Set the volumes first
  (`music_volume` and `voice_volume` reach what is already playing), then
  play the `SoundRequest`s in order (a sound cue at its manifest volume
  times its own and the sound volume; a voice clip by its file in the
  game's voice folder; stop the voice; the clips coming, to load ahead),
  then apply the `MusicCommand`s (load, start, fade, stop), and check on
  what is still loading. It is called every frame, with nothing to play
  too;
- `stop_all()`: the game quit.

Until ticket 3029 moves the rules of playing (variants, fades, the music
clock) into a player every backend shares, they are in `shuyi-app`'s
`audio` module; `shuyi_platform::Silent` plays nothing, for a backend
with no sound yet.

### `text_input(&mut self) -> Option<&mut dyn TextInput>`

The system's own keyboard for text (a console's, Steam's on-screen one),
if the device has one; `None` where the player types on keys, which come
through `poll_input`. `open(&ask)` shows it with the ask's prompt, text
and length, or is `false` if it can't now; `poll()` gives the answer
(`Typed(text)` or `Cancelled`) once the player closes it.

### `clock(&self) -> Option<PerfClock>` (has a default)

A clock to measure frames with, in debug builds
([ADR-0046](adr/0046-perf-traces-measured-by-the-backend-kept-by-app-beside-the-log-and-read-by-agents-as-json-lines.md)):
`PerfClock::Real(now_us)`, where `now_us` gives whole microseconds from
any fixed start and never goes back (`shuyi-app`'s reads macroquad's
`get_time`). `App` asks for it once, on its first frame, and only in a
build with debug tools (`shuyi_ui::DEBUG_TOOLS`): it times the game's
load, moves the clock into the game's context, and keeps a
`shuyi_ui::perf::FrameRecord` of each frame while the game's
`debug_tools` is on (`app.perf()`, `app.take_new_perf()`). A release
build never reads it. A backend without a clock leaves the default,
`None`, and nothing is measured. Never reset the device's timer in the
loop: a record's times are differences of its reads.

### `memory(&mut self) -> Option<MemoryReading>` (has a default)

The device's memory now, in debug builds (ADR-0046):
`MemoryReading::new(HeapCounts::new(live_bytes, peak_bytes, allocs))`,
the heap's live bytes, their peak, and the allocations and reallocations
since the last call, in bytes asked for (never the allocator's own
overhead). `App` asks once a frame, at the end of a frame it keeps a
record of (so only with a clock, and only while `debug_tools` is on),
and puts the heap's counts in the record's `heap`. Count with
`shuyi_alloc_count::CountingAlloc` around the device's allocator, in a
debug build only: `shuyi-app` reads the global counter of
`shuyi_alloc_count`'s `global` feature (`global::reading()`), which
counts the whole process, every thread's allocations. A backend that
doesn't count leaves the default, `None`, and the records have no heap.

### `memory_scopes(&self) -> Option<MemoryScopes>` (has a default)

The heap counter's memory tags, in debug builds (ADR-0046 §6), so the
heap is counted by what it is for: `MemoryScopes::new(tag, enter,
leave)`, three plain functions over the counter's tags. `App` asks for
them with the clock, once, on its first frame and only with debug tools
on, and moves them into the game's context when it runs. Then the game's
frame tags what it allocates `update` and `draw`, each span its own
name, and `App` tags `load`, `paint`, `perf` and `app`. A counter with
tags is `shuyi_alloc_count::CountingAlloc::with_tags` (a header in front
of each block), and its functions are `shuyi_alloc_count::tags`'s `tag`,
`enter` and `leave`: `shuyi-app` gives them when `global::COUNTING`. A
backend whose counter has no tags leaves the default, `None`.

A backend with tags also tags each asset decode with its kind, so the
bytes a game keeps of each kind of asset count under it. When it starts,
before the game's load, it sets the registry's one decode watch,
`shuyi_assets::format::watch_decodes(DecodeWatch::new(before, after))`:
`before(kind)` enters the tag of the kind (`TagClass::Asset`) and gives
back the tag before as its token, and `after(token, decoded)` leaves it.
`crates/app/src/backend.rs` has the two functions to copy.

### `heap_snapshot(&mut self) -> Option<HeapSnapshot>` (has a default)

What holds the heap now, by memory tag, in debug builds (ADR-0046 §6):
the heap's live bytes and peak, and each tag's row (its class and name,
live bytes and blocks, bytes ever allocated, peak), its `frame` left 0.
`App::heap_snapshot` stamps the frame, only while debug tools are on,
and only when a tool or the trace file asks (every 600 frames), never
every frame: taking one allocates the rows. A counter with tags reads
its rows (`CountingAlloc::rows`) and hands them, with its counts and
`tags::overflowed()`, to `shuyi_platform::heap::snapshot(counts, &rows,
overflowed)` (the platform's `heap` feature), which `shuyi-app` and the
tool program share. A backend whose counter has no tags leaves the
default, `None`, and no snapshot is taken.

## The live link (optional)

A backend may offer the live link
([ADR-0047](adr/0047-the-live-link-json-lines-over-loopback-tcp-to-a-running-debug-build-read-only-but-a-players-input.md)),
so `shuyi attach` reaches a debug build running on it. The wire and its
server are `shuyi_platform::link` (the `link` feature); the backend
carries the lines and takes the picture. `shuyi-app`'s listener
(`crates/app/src/link.rs`, its own `link` feature) is the one to read.

- **Only behind a cargo feature, and only with debug tools**
  (`shuyi_ui::DEBUG_TOOLS`) and the variable `SHUYI_LINK` set; otherwise
  log why and don't listen. Bind `127.0.0.1`, make a token (128 random
  bits, or `SHUYI_LINK_TOKEN`), and log `shuyi link: listening on
  <address>, token <token>`.
- **One `link::Server` per connection**, one connection at a time; tell
  a second one `Answer::Busy`, half-close it, and read it until it hangs
  up.
- **Before `App::frame`:** read what has come (non-blocking, never
  waiting), hand each line to `Server::handle` and send what it answers
  now, then put what `Server::input` gives in a list your `poll_input`
  adds **after the devices' own events**, so it goes where a player's
  input goes.
- **After `App::frame`:** send what `Server::after_frame` gives. If
  `Server::wants_screenshot`, read the frame just presented, before the
  device shows the next, and send `Server::screenshot(png, width,
  height)`: the window's pixels, upright, as a PNG.
- On `Handled::Close`, send its line if it has one, then close and read
  nothing more. Drop a client that leaves more than `link::MAX_UNSENT`
  bytes unread, or sends a line longer than `link::MAX_LINE`.

## The headless backend, as a reference

`shuyi_platform::headless::HeadlessPlatform` is the smallest complete
backend: it plays input and system events queued by a test, keeps the
last frame presented, hands over a memory storage, and stands in for the
music player (`Speaker`). A test hands it a clock
(`with_clock(PerfClock::fake())`, `set_clock`), whose fake reads give
the same records on every run, and a heap: a fake one that moves by
fixed amounts with the frames presented (`with_fake_memory`), or a real
counter's reading (`set_memory`); `memory_calls` counts `App`'s asks. It
gives memory tags only when a test hands it some (`with_scopes`,
`set_scopes`): fakes that note what is entered. The test Harness (`shuyi_platform::harness`, the
`harness` feature) runs every game's tests on it, through the same
`App::frame` a device runs. Read it before writing a backend, and test a
new backend's translation the same way: a fake of the device's SDK on
one side, the engine's types on the other.

## Checklist for a new backend

- [ ] A crate of its own, depending on `shuyi-platform` (and `shuyi-ui`,
      `shuyi-assets`), with the device's SDK behind it.
- [ ] Each method above, and the loop calling `App::frame`.
- [ ] Its storage passes the conformance suite.
- [ ] Its input tests: the device's buttons to `RawInputEvent`s, releases
      before presses.
- [ ] A target profile (`targets/<id>.ron`, ADR-0008 §1) naming it, and
      the target's device ticket for the numbers (ADR-0008 §6).
- [ ] For a closed console: a private repository, and nothing of it here.
