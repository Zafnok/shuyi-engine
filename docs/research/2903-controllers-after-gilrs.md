# Controllers after gilrs: SDL3 for desktop pads, the web keeps its own

Research for ticket 2903, checked **2026-11-22 to 2026-11-24**. The
pads were tried on Nick's Windows 11 machine on 2026-11-24, with Steam
not running. This research adds no dependency and changes no code.
**Nick's answer** is in [decisions](../decisions.md) (2026-11-24), and
[§ The answer](#the-answer-and-what-follows) says what follows from it.

**Recommendation: read native pads through SDL3's gamepad API, as Godot
4.5 does, and keep the web's own `gamepad.js`.** On Nick's pads, SDL3
read every button of an Xbox pad and a DualSense correctly, the logo
button, gyro, touchpad and light included, and rumbled both. gilrs read
the Xbox pad but not its logo button, didn't list the DualSense at all,
and listed a Switch 2 Pro pad that gave no input. Our own code, written
from documents the makers publish, would reach about what gilrs does on
Windows: the parts SDL adds (the logo button, Sony's gyro and touchpad,
the Switch 2 pads) come from protocols no maker documents. SDL3 is
Zlib, released every month, and brings a C library built with CMake.
One pad worked with neither library: SDL reads the wired Switch 2 Pro pad
only when built with libusb, which is LGPL-2.1, so the engine can't ship
it.

**Nick chose SDL3 with a fix for Switch** (2026-11-24). The fix was then
found and tried: one start-up command that switchbrew.org's protocol
page documents, sent by our own code, made the Switch 2 pad report every
button. [ADR-0146](../adr/0146-desktop-pads-through-sdl3-with-our-own-start-up-for-switch-2-pads.md)
records the plan.

## What the engine needs from pads

From `crates/app/src/pads/` (the native source, gilrs, and the web
source), `shuyi-ui`'s `Pads` (`crates/ui/src/input/pad.rs`) and the
open tickets that name pads. **R** is needed now or by a planned ticket;
**L** is later work that may want it.

| Need | Where it comes from |
| --- | --- |
| R: the 16 buttons of the standard layout, by position | `PadState`, `Button::ALL[..16]` |
| R: hot-plugging, and an id stable while the pad stays plugged in | `Pads::update`, `PadRemoved`; 2406 (seats, parked) gives pads to players by it |
| R: the pad's maker, for button names and the Switch-style swap | `PadKind::from_ids` (USB vendor and product ids) |
| R: the Steam Deck's controls as `PadKind::SteamDeck` | 3012 (Valve's vendor id `28de`, or the virtual Xbox pad Steam Input gives) |
| R: sticks and triggers as values | 1121 (parked) |
| R: two-motor rumble on desktop; the browser's `vibrationActuator` on the web | 2408 (parked) |
| R: never block or panic in the frame | the frame reads pads every frame on the main thread |
| R: the web reads the browser's Gamepad API itself: `gamepad.js` through macroquad's loader today, `web-sys` once the web build moves to `wasm-bindgen` (3331, ADR-0136) | `crates/cli/web/gamepad.js`, `crates/app/src/pads/web.rs` |
| L: the logo button | not read today; a game's pause menu, Steam Deck (3012 says never read the Steam button) |
| L: gyro, touchpad, lights, battery | 2402's Out: "write a ticket when a game asks" |
| L: Android and iOS pads; the Switch's own pads | 3015 (parked); 3014 (parked, a private backend) |

## The options, with the facts of the day

Checked on crates.io, GitLab, GitHub and Codeberg on 2026-11-22.

| | gilrs (today) | SDL3 through `sdl3-sys` | Our own code (2402, 2403) |
| --- | --- | --- | --- |
| Version | 0.11.2 (2026-05-30); 0.11.1 (2026-01-12) | SDL 3.4.20 (2026-11-03); `sdl3-sys` 0.7.3 (2026-11-04) | none |
| License | MIT OR Apache-2.0 | SDL: Zlib; `sdl3-sys`: Zlib; `sdl3` (the safe wrapper): MIT | MIT OR Apache-2.0 |
| Brings | `gilrs-core`, `uuid`, `windows` on Windows; `libudev-sys`, `nix`, `inotify`, `vec_map` on Linux; `objc2-io-kit` on macOS | SDL's C source (`sdl3-src`), built by CMake and a C compiler; or a system SDL | `windows`, `rustix`, `objc2-game-controller`; SDL's mapping file as data |
| Windows | Windows.Gaming.Input by default (needs a focused window), XInput as a feature | XInput, RawInput, HIDAPI (Sony, Nintendo, Valve pads), WGI and GameInput off by default | XInput and `RawGameController` (2402) |
| Linux, Steam Deck | evdev; needs `libudev` to build | evdev, HIDAPI over hidraw (the Deck's own controls), udev loaded at run time | evdev, `inotify` (2403) |
| macOS | IOKit; no rumble | the Game Controller framework and IOKit | the Game Controller framework (2403) |
| Rumble | Windows and Linux (its README); macOS no | `SDL_RumbleGamepad`, and trigger motors where the driver has them | XInput on Windows; evdev's force feedback on Linux; none on macOS (2402, 2403) |
| Android, iOS | no; an iOS merge request open since 2026-09-06 | yes (SDL's own platforms) | not planned |
| The web | `wasm-bindgen` only | Emscripten only | ours already (`gamepad.js`) |
| Used by | Bevy 0.20 (`bevy_gilrs`), 128 crates on crates.io | Godot 4.5 and later on desktop ("Since Godot 4.5, the engine relies on SDL 3 for controller support"), 47 crates use `sdl3` | |

### gilrs's health

From its GitLab project (`gitlab.com/gilrs-project/gilrs`) on 2026-11-22:

- **One maintainer.** Every merge request of the last two years was
  merged by one account (Mateusz Sieczko), who also cuts every release.
  Merge requests wait a day to five weeks. Three are open (an iOS
  backend, Steam controllers on macOS, event times).
- **Active, with fixes waiting for a release.** Commits came on
  2026-11-15, 11-09, 11-07 and 11-06. Since 0.11.2 (May), master has
  fixed a crash on the Steam Deck through Proton (#177), a crash with
  Steam Input on (#176) and a panic in `Gilrs::gamepad` seen in a
  released game (#206). None of these is released yet.
- **77 open issues** of 209; their median age is 3.7 years, 29 have no
  reply, and five were opened in 2026. Open and relevant here:
  `next_event()` sometimes blocks the thread on Windows after sleep
  (#196); `RawGameController::NonRoamableId` fails and drops events on
  Windows (#208, 2026-10-20).
- 90 stars on GitLab, 211 forks; 10.4 million downloads on crates.io.

### SDL3's health

SDL (`github.com/libsdl-org/SDL`): 17,048 stars, a push on
2026-11-21, a release each month (3.4.14 on 2026-08-03, 3.4.16 on
09-02, 3.4.18 on 10-02, 3.4.20 on 11-03). Its 3.4 notes add Switch 2
pads "when built with libusb" and a Steam Controller type (3.4.18).
`sdl3-sys` follows each SDL release within a day or two; it is one
maintainer's crate on Codeberg (19 stars, 6 open issues, updated
2026-11-15), and its bindings are generated from SDL's headers.
`sdl3`, the safe wrapper (`github.com/vhspace/sdl3-rs`), had 0.22.1 on
2026-11-09.

## What the pads did

Each library was run as a black box (ADR-0021 §2) by a scratch program
that asked for each button by its position, the sticks, the triggers,
the two motors, and an unplug and replug, and logged what the library
reported. gilrs 0.11.2 with its default features; SDL 3.4.20 built from
source, static, with its default drivers and no libusb. Every pad by USB
cable, one at a time, on Windows 11 (Nick's machine), 2026-11-24.

| Pad (USB ids) | gilrs 0.11.2 | SDL 3.4.20 |
| --- | --- | --- |
| Xbox Series X pad (`045e:0b12`) | 16 of 16 buttons right; **the logo button reported nothing** (Windows opened Game Bar); sticks and triggers analog (97 levels a trigger); both motors felt; replugged with the same id | 16 of 16 right **and the logo button**; type `xboxone`, reported as `045e:02ff`; triggers 128–194 levels; both motors felt; trigger motors "not supported"; replugged with a new id |
| DualSense (`054c:0ce6`) | **not listed at all**, though Windows saw it | 17 of 17 right; type `ps5`; gyro, accelerometer, a touchpad, the light; triggers 172–176 levels; both motors felt; replugged with a new id |
| Switch 2 Pro (`057e:2069`) | listed as "Nintendo Switch 2 Pro Controller", **no button reported** | **not listed** (needs libusb) |
| Steam Deck | not tried: left for a follow-up (ticket 2414) | not tried (2414) |

At rest, gilrs gave exact zeros (it filters small values), SDL raw values
up to 0.04. gilrs's sticks are up-positive and SDL's down-positive; the
engine flips gilrs's today. On both, a stick's corners reached about 1.2,
so the engine's 4-way rule must keep treating each axis on its own, as it
does.

What wasn't tried: Bluetooth; the Switch 2 pad's other interfaces; a
Switch (1) Pro pad, an 8BitDo or another third-party pad; Linux and
macOS. The Xbox pad turned on over Bluetooth for a moment while unplugged
(`045e:0b13`), and both libraries listed it, so both handle a pad that
changes how it's connected.

## What each choice costs

**A. SDL3 for desktop pads — recommended.** Godot made this move in
4.5, saying its own controller code had made some pads misbehave, and
uses SDL for input only, not its window or sound, as we would.

- **Pads:** the widest support of the three, measured above, and SDL's
  own drivers for Sony, Nintendo (Switch 1) and Valve's pads. The logo
  button, gyro, touchpads, lights and per-pad types are there when a game
  asks (2402's Out), and Android and iOS pads come with it if 3015 is
  unparked.
- **Build:** SDL's C source is built once by CMake (about a minute, then
  cached), so a new game's machine needs CMake beside the C compiler it
  already needs. On Nick's machine that was one MSYS2 package
  (`mingw-w64-x86_64-cmake`); `scripts/setup-linux.sh` and `shuyi doctor`
  would add it. A system SDL instead (`use-pkg-config`) would put a DLL
  beside every game. Static linking keeps one file and adds about
  1.8 MB (measured below).
- **Licenses:** SDL and `sdl3-sys` are Zlib, on ADR-0002 §8's list.
  libusb (LGPL-2.1) must stay off, so wired Switch 2 pads stay unread
  until SDL reads them without it.
- **The seam:** `shuyi-app`'s pad source changes; `Pads` and everything
  above it stay. The web keeps its own reading (`gamepad.js`, then
  `web-sys` with 3331). The game sees no API change.
- **Effort:** about one ticket of 400–800 lines (the source, its build
  in CI on three systems, `shuyi doctor`'s check, rumble if 2408 is
  done), against two `xhigh` tickets for our own code.

**B. Keep gilrs.** Nothing to build now. The engine keeps a library with
one maintainer, crash fixes unreleased for six months, and the gaps
above (no DualSense on Windows over USB, no logo button through WGI). It
stays a two-way door: the next look is whenever a game hits one.

**C. Our own code (2402 and 2403 as written).** No outside library, the
cleanest license, and a crate that can leave for its own repository
(2008). But ADR-0021 §2 allows only documents the makers publish:

- Windows: XInput has no documented logo button, and WGI's
  `RawGameController` hands raw buttons that need SDL's mapping file;
- Sony and Nintendo publish nothing about their pads' reports, so the
  gyro, touchpad, lights and the Switch 2 pad's start-up are out of
  reach without someone else's reverse engineering;
- Linux and macOS are well documented (evdev; the Game Controller
  framework), so 2403 stands either way.

So C reaches about what gilrs reaches on Windows, for the most work and
every hardware bug ours. It suits a later day when the engine wants no C
at all.

| Choice | Work | Pads on Windows (measured or expected) | New outside code |
| --- | --- | --- | --- |
| A: SDL3 | 1 ticket, 400–800 lines | Xbox and DualSense complete; Switch 2 no | SDL (C, Zlib), `sdl3-sys` |
| B: gilrs | none | Xbox without logo; DualSense no; Switch 2 no input | none new |
| C: our own | 2 tickets, `xhigh`, 3,000–6,000 lines | about gilrs's; logo, gyro, touchpad no | `windows`, `rustix`, `objc2-*` |

The line counts are engineering estimates, not measurements.

## Measurements

Release builds, stripped, of the scratch program (macroquad plus one
library), cold, on Nick's machine (24 cores), 2026-11-24:

| Build | Cold build | Size |
| --- | --- | --- |
| macroquad + gilrs 0.11.2 | 15 s | 3,527,680 bytes |
| macroquad + SDL 3.4.20, every SDL subsystem | 154 s | 6,596,096 bytes |
| macroquad + SDL 3.4.20, input parts only (no audio, video, render, GPU, camera, dialog, tray) | 132 s | 5,285,888 bytes |

So SDL's input parts add about 1.8 MB to a game, and its C build about
two minutes the first time; cargo then caches it, so a game's rebuild
doesn't pay it again. The C build needed `CMAKE_GENERATOR=Ninja` and a
short target directory: under the session's long scratch path, CMake's
object paths passed Windows's 260-character limit.

## The answer and what follows

Nick, 2026-11-24 (verbatim in [decisions](../decisions.md)): "either
sdl3 with a fix for switch, or write our own. if no fix for switch can
exist with sdl3, then write our own. make sure it is maintainable and
not a clone of existing stuff."

**The fix exists.** switchbrew.org's protocol page for the Switch 2 Pro
Controller (read 2026-11-24) documents an "Initialize" command: 16
bytes, command `0x03`, then `0x91`, sent over USB interface 1's bulk OUT
endpoint, after which the pad starts sending input. A scratch program
written from that page alone sent it through `nusb` (0.2.9, Apache-2.0
OR MIT, no C), and gilrs then read Nick's pad:

| Switch 2 Pro (`057e:2069`), after the start-up command | gilrs 0.11.2 |
| --- | --- |
| Buttons | 17 of 17 right, the Home button included |
| Sticks | analog, up-negative (inverted, as the page says), reaching about 0.8–0.9 of full |
| ZL, ZR | digital (two values), as the pad's are |
| Rumble | none: "force feedback is not supported" |
| Unplug and replug | seen, same id |

SDL3 still didn't list the pad, with its Switch 2 driver turned off by
`SDL_JOYSTICK_HIDAPI_SWITCH2=0`; other hints weren't tried with the pad
plugged in. So SDL3 is the answer, with a small fix of our own:

- [ADR-0146](../adr/0146-desktop-pads-through-sdl3-with-our-own-start-up-for-switch-2-pads.md)
  records it, resolving ADR-0021 §1's controllers choice;
- **2402** (parked) is rewritten: desktop pads through SDL3, gilrs
  removed, and, as Nick then asked, a toggle (on by default) so a game
  can leave controller support out on desktop and save the 1.8 MB;
- **2403** (parked) is rewritten: the Switch 2 start-up in `shuyi-pads`,
  and the pad read through SDL3 or by the crate itself;
- **2414** tries both on the Steam Deck, as Nick asked;
- **2408** (rumble, parked) and **1121** (analog, parked) read their
  either-order clauses against the SDL3 source.

## Sources

- gilrs: its README and issues #176, #177, #196, #206, #208 at
  `gitlab.com/gilrs-project/gilrs`; its crates.io page.
- SDL: [the 3.4.20 `WhatsNew.txt`](https://raw.githubusercontent.com/libsdl-org/SDL/release-3.4.20/WhatsNew.txt),
  its releases on GitHub, the `SDL_HINT_JOYSTICK_HIDAPI_STEAMDECK` and
  `SDL_HINT_JOYSTICK_HIDAPI_SWITCH2` hints; `sdl3-sys` and `sdl3` on
  crates.io.
- Godot: [Controllers, gamepads, and joysticks](https://docs.godotengine.org/en/4.5/tutorials/inputs/controllers_gamepads_joysticks.html)
  (4.5).
- libusb's license: `github.com/libusb/libusb` (LGPL-2.1).
- The Switch 2 Pro pad's protocol:
  [Switch 2: Pro Controller](https://switchbrew.org/wiki/Switch_2:_Pro_Controller)
  on switchbrew.org, read 2026-11-24. `nusb` 0.2.9 on crates.io
  (Apache-2.0 OR MIT).
- No library's source was read; the probe called each one's public API
  from its documentation and its bindings' declarations.
