# ADR-0021: The platform layer in-house: written from specifications, in crates of their own, with wgpu and winit for graphics and the window

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related tickets:** 2902, 2903 (research first); 2210–2215, 2402, 2403,
  3030–3034, 2008
- **Extends:** ADR-0002 §8 (licenses) and §9 (`unsafe`), ADR-0008 §2
  (platform backends)

## Context

Nick decided on 2026-10-19 (`docs/decisions.md`) that macroquad and the
libraries around it leave the engine, in this order: sound, controllers,
web saves and the embedding of a game's files, then macroquad. Later,
each piece we write moves to a repository of its own. All of it is parked
until the MVP.

What the engine leans on today, checked on crates.io and the projects'
repositories on 2026-10-19:

| Piece | Crate | Last release | Brings |
| ----- | ----- | ------------ | ------ |
| Sound output and mixing | `quad-snd` 0.2.8 | 2024-05 | `audrey` 0.3 (2021-01), `lewton` 0.9 (older than its 0.10.2 of 2021-01), `hound`, `ogg`, `dasp` (2020), `smallvec` 0.6 and `maybe-uninit` (2019) |
| Controllers | `gilrs` 0.11.2 | 2026-05 | `gilrs-core`, `vec_map` (2020), `uuid`, `windows` |
| Web saves | `quad-storage` 0.1.3 | 2021-08 | `sapp-jsutils` |
| A game's files in its binary | `include_dir` 0.7.4 | 2024-06 | `include_dir_macros` |
| Window, input, drawing, run loop | `macroquad` 0.4.16 over `miniquad` 0.4.11 | 2026-07 | `fontdue`, `ttf-parser`, `image` 0.24, `png` 0.17, `glam` 0.27, `quad-rand` |

miniquad draws with OpenGL 2 and 3, GLES 2 and 3, WebGL 1 and Metal. Its
README rules out Vulkan and DirectX 12.

Bevy, the engine Nick names as the one to rival someday, uses winit 0.30
for its window, wgpu 30 for its graphics, gilrs 0.11 for controllers and
rodio 0.22 for sound (`bevy_winit`, `bevy_render`, `bevy_gilrs` and
`bevy_audio` 0.20 on crates.io, 2026-10-19).

The forces:

- **"Absolutely do not steal source code line by line"** (Nick). What we
  write must be ours, so that its license is clean and the pieces can
  stand alone later.
- **Each piece we write becomes its own repository later.** It must be
  able to leave without dragging the engine along.
- **Not everything is worth writing.** Nick: "I don't wanna rewrite
  graphics bindings... so I think wgpu is safe", and winit "since it's not
  really something I wanna reimplement".
- **Controllers and sound are two-way doors.** Nick: "this can be a 2 way
  door decision and we can make it much later in the future after
  additional research in a dedicated ticket. And for audio, we can look at
  our options as well when we get there."
- **The web build uses macroquad's loader, not `wasm-bindgen`** (game
  ADR-0034, `AGENTS.md`). wgpu and winit both need `wasm-bindgen` in the
  browser.

## Decision

### 1. What we use, what we write, and what research decides

| Piece | Decision | Tickets |
| ----- | -------- | ------- |
| Graphics | **wgpu**, with our own 2D renderer on top | 3034, then the tickets it writes |
| Window and its input | **winit** | the same |
| Run loop | ours (`App::frame`, 3003), driven by winit's event loop | the same |
| Web saves | ours: a `localStorage` plugin | 3033 |
| A game's files in its binary | ours: `shuyi-embed` | 2215 |
| Sound | quad-snd goes. **Research decides how** (our own code, as 2210–2214 and 3030–3032 plan, or a library) | 2902 first |
| Controllers | gilrs goes. **Research decides how** (our own code, as 2402 and 2403 plan, or a library) | 2903 first |

wgpu is lower level than macroquad: it opens no window, runs no loop,
plays no sound and draws no sprites. It's one interface to Vulkan, Metal,
Direct3D 12, OpenGL and GLES, WebGL 2 and WebGPU, with shaders written
once in WGSL (MIT OR Apache-2.0; 30.0.1 of 2026-08-22). winit opens
windows and reports their events on Windows, macOS, X11 and Wayland, the
web, Android and iOS, with high-DPI and IME (Apache-2.0; 0.30.13 of
2026-09-04; over 1 500 crates depend on it). Writing either ourselves
would cost more than the rest of the platform layer together, for nothing
a player would see.

Both make breaking releases (wgpu about every three months), so the engine
pins them and moves on purpose. On the web both reach the browser through
`wasm-bindgen`, so the web build moves to it with macroquad's departure
(3034 plans it).

The research tickets 2902 and 2903 may keep, change or drop the build
tickets that wait on them. Whatever they pick follows §2 to §6 if it's our
own code, and ADR-0002 §8's licenses if it's a library.

### 2. What we write is written from specifications, never copied

Code we write is written from public specifications and from the
operating system's or the browser's own documentation:

- Ogg: RFC 3533. Vorbis: the Xiph *Vorbis I specification*. WAV: the
  RIFF and `WAVE_FORMAT_EXTENSIBLE` documents from Microsoft, and the
  IMA ADPCM description from the IMA.
- Windows: Microsoft Learn (WASAPI, Windows.Gaming.Input, XInput, Raw
  Input and HID).
- Apple: Apple's developer documentation (Core Audio, the Game
  Controller framework).
- Linux: the kernel's documentation (evdev, `input-event-codes.h`), and
  the ALSA library's API reference.
- The web: MDN and the W3C specifications (Web Audio, the Gamepad API,
  Web Storage).

The rules for whoever works a ticket:

- **Never open another implementation's source** for the piece being
  written, whatever its license: not lewton, stb_vorbis, libvorbis,
  quad-snd, cpal, gilrs, SDL or miniquad.
- **Other implementations may be run as black boxes.** For example,
  decoding a test file once to make an expected-output fixture, or
  comparing behaviour on a real pad. The tool runs; we never read its
  code.
- **A library we use is read like any library.** wgpu's and winit's
  documentation and examples are fair to read, since we call them rather
  than reimplement them.
- **A ticket's completion notes list the documents used**, by name and
  link, and the PR body says the code was written from them.
- **Data under a permitted license may be used as data**, with its notice
  in `THIRD_PARTY_ASSETS.md`. For example, SDL's controller mapping
  database (`gamecontrollerdb.txt`, Zlib) read by our own parser.

### 3. What we write sits on raw OS bindings

Our own platform code may depend on crates that only declare an operating
system's or a driver's API, with no behaviour of their own, under ADR-0002
§8's licenses. Checked 2026-10-19:

| API | Crate | License |
| --- | ----- | ------- |
| Win32, WASAPI, XInput, Windows.Gaming.Input | `windows` (Microsoft) | MIT OR Apache-2.0 |
| Apple frameworks (Core Audio, Game Controller) | `objc2-*` | Zlib OR Apache-2.0 OR MIT |
| ALSA | `alsa-sys` | MIT |
| Linux system calls (evdev, inotify) | `rustix` | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |

A ticket re-checks the license on the day it adds the crate, from the
crate's `Cargo.toml` and repository, and `cargo deny` checks it in CI.

### 4. Each piece we write is a crate of its own

Each is a crate in `crates/` that depends on no other `shuyi-*` crate, so
it can move to its own repository later (2008) without changes. Planned,
if the research keeps them:

| Crate | What | `no_std` |
| ----- | ---- | -------- |
| `shuyi-sound` | the WAV, Ogg and Vorbis readers, the mixer | yes (with `alloc`), so the Playdate and the GBA can use it |
| `shuyi-sound-out` | sound output: WASAPI, Core Audio, ALSA, Web Audio | no |
| `shuyi-pads` | controllers: Windows, Linux, macOS; the mapping database | no |
| `shuyi-embed` | a game's files in its binary, from a build script | no (a build dependency) |

3034 names the crate for the 2D renderer over wgpu.

- **The engine adapts them.** `shuyi-assets` registers the readers as
  formats (ADR-0007, ADR-0019). `shuyi-app` (or the backend crate that
  3003 leaves) implements the `Platform` trait's parts on them.
- **Their APIs are plain**, documented and tested on their own: a decoder
  takes bytes and gives samples; the pads crate gives a list of pads with
  standard buttons and axes. They use their own types, never the engine's
  `Button` or actions, and the engine maps them (`AGENTS.md` rule 8).
- **They are MIT OR Apache-2.0**, with the workspace's lints and gates.

### 5. The old crate leaves in the same PR as its last replacement

When the last ticket of a piece lands, it removes the old crate from
`Cargo.toml`, and `cargo tree` shows it gone. Until then, the old and the
new run side by side behind the same seam. A platform not yet replaced
keeps the old crate.

### 6. `unsafe` in the platform crates only, the way `shuyi-app` does it

ADR-0002 §9 forbids `unsafe` outside `shuyi-app`. Calling an operating
system's API needs it, so our crates that talk to the system get
`shuyi-app`'s exception and its rules:

- **The platform crates**: `shuyi-sound-out` and `shuyi-pads`, if the
  research keeps them. Each sets `unsafe_code = "deny"` instead of
  `forbid` in its `[lints]`, and keeps a test that its lints are the
  workspace's except that one, as `crates/app/src/lib.rs` does.
- **`unsafe` lives only in a `sys` module** of each crate, under one
  `#[allow(unsafe_code)]` on that module. The rest of the crate wraps it in
  safe types.
- **Every `unsafe` block has a `// SAFETY:` comment** saying why it holds
  (clippy's `undocumented_unsafe_blocks`, turned on in those crates).
- **`shuyi-sound` and `shuyi-embed` stay `forbid`.** They are plain code.
- The renderer over wgpu and the window code over winit should need no
  `unsafe`. If 3034 finds otherwise, it says why in its ADR.

## Consequences

- What we write has no source code from another project, so the license
  is clean, and the pieces can be published on their own.
- The engine keeps two large outside libraries, wgpu and winit, like
  Bevy. In return it gets every graphics API and every window system
  without writing them. Their breaking releases are work we schedule.
- The web build moves to `wasm-bindgen` when macroquad goes. The plugins
  we write before then (3031, 3033) move with it.
- Sound and controllers stay open until their research. If it chooses our
  own code: a Vorbis decoder, sound output on three systems and
  controllers on three systems is real work, about a dozen tickets. We
  would own the hardware bugs that gilrs and quad-snd handle today, so
  testing on real devices is part of each output and controller ticket.
- If `shuyi-sound` is built, it's `no_std`, so one decoder and one mixer
  serve every target, the Playdate (3029) and the GBA (3027) included.

## Alternatives considered

- **Our own window and our own graphics backends** (`windows`, `objc2-*`
  and Wayland and X11 for windows; `ash`, `objc2-metal` and `glow` for
  graphics; our own shader translation). Lost: Nick doesn't want to
  reimplement them, and they would cost more than everything else here
  for the same pixels.
- **Decide sound and controllers now.** Lost: both are two-way doors, the
  work is after the MVP, and the libraries will have moved by then. The
  research tickets decide with the facts of their day.
- **Keep macroquad, and add wgpu beside it.** Lost: macroquad owns the
  window and the loop, so wgpu can't get at the window without fighting
  it, and the old GL path would stay.
- **Fork the old crates and maintain them.** Lost: forking is copying
  their source, and we would own code we didn't write.
- **Port a small public-domain decoder (stb_vorbis).** Lost: a line-by-line
  translation is exactly what Nick ruled out, whatever the license.
- **Put the new code inside `shuyi-app`.** Lost: it couldn't move to its
  own repository later, and the Playdate and the GBA couldn't share the
  decoder and the mixer.
