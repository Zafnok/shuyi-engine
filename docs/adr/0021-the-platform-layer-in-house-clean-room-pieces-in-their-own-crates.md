# ADR-0021: The platform layer in-house: written from specifications, in crates of their own, over raw OS bindings

- **Status:** Accepted
- **Date:** 2026-10-19
- **Related tickets:** 2210–2215, 2402, 2403, 3030–3034, 2008
- **Extends:** ADR-0002 §8 (licenses) and §9 (`unsafe`), ADR-0008 §2
  (platform backends)

## Context

Nick decided on 2026-10-19 (`docs/decisions.md`) that the platform layer
goes in-house: sound first, then controllers, then web saves and the
embedding of a game's files, then macroquad. Later, each piece moves to a
repository of its own. All of it is parked until the MVP.

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

Three forces shape the work:

- **"Absolutely do not steal source code line by line"** (Nick). The
  engine's code must be ours, so that its license is clean and the pieces
  can stand alone later.
- **Each piece becomes its own repository later.** It must be able to
  leave without dragging the engine along.
- **The web build uses macroquad's loader, not `wasm-bindgen`** (game
  ADR-0034, `AGENTS.md`). Whatever replaces it on the web reaches the
  browser through our own JS, as the gamepad plugin
  (`crates/cli/web/gamepad.js`) already does.

## Decision

### 1. Written from specifications, never copied

A replacement is written from public specifications and from the
operating system's or the browser's own documentation:

- Ogg: RFC 3533. Vorbis: the Xiph *Vorbis I specification*. WAV: the
  RIFF and `WAVE_FORMAT_EXTENSIBLE` documents from Microsoft, and the
  IMA ADPCM description from the IMA.
- Windows: Microsoft Learn (WASAPI, Windows.Gaming.Input, XInput, Raw
  Input and HID).
- Apple: Apple's developer documentation (Core Audio, the Game
  Controller framework, Metal).
- Linux: the kernel's documentation (evdev, `input-event-codes.h`), and
  the ALSA library's API reference.
- The web: MDN and the W3C specifications (Web Audio, the Gamepad API,
  Web Storage, WebGL 2, WebGPU).

The rules for whoever works a ticket:

- **Never open another implementation's source** for the piece being
  written, whatever its license: not lewton, stb_vorbis, libvorbis,
  quad-snd, cpal, gilrs, SDL, miniquad or winit.
- **Other implementations may be run as black boxes.** For example,
  decoding a test file once to make an expected-output fixture, or
  comparing behaviour on a real pad. The tool runs; we never read its
  code.
- **A ticket's completion notes list the documents used**, by name and
  link, and the PR body says the code was written from them.
- **Data under a permitted license may be used as data**, with its notice
  in `THIRD_PARTY_ASSETS.md`. For example, SDL's controller mapping
  database (`gamecontrollerdb.txt`, Zlib) is read by our own parser.

### 2. Raw OS bindings are not frameworks

A replacement may depend on crates that only declare an operating
system's or a driver's API, with no behaviour of their own, under ADR-0002
§8's licenses. Checked 2026-10-19:

| API | Crate | License |
| --- | ----- | ------- |
| Win32, WASAPI, XInput, Windows.Gaming.Input, Direct3D 12 | `windows` (Microsoft) | MIT OR Apache-2.0 |
| Apple frameworks (Core Audio, Game Controller, Metal, AppKit) | `objc2-*` | Zlib OR Apache-2.0 OR MIT |
| ALSA | `alsa-sys` | MIT |
| Linux system calls (evdev, inotify) | `rustix` | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| Vulkan | `ash` | MIT OR Apache-2.0 |
| OpenGL, GLES, WebGL | `glow` | MIT OR Apache-2.0 OR Zlib |

A ticket re-checks the license on the day it adds the crate, from the
crate's `Cargo.toml` and repository, and `cargo deny` checks it in CI.
Libraries that do the job themselves (cpal, rodio, symphonia, gilrs, SDL,
winit, wgpu) are not bindings: the job is what we write.

### 3. Each piece is a crate of its own

Each replacement is a crate in `crates/` that depends on no other
`shuyi-*` crate, so it can move to its own repository later (2008)
without changes:

| Crate | What | `no_std` |
| ----- | ---- | -------- |
| `shuyi-sound` | the WAV, Ogg and Vorbis readers, the mixer | yes (with `alloc`), so the Playdate and the GBA can use it |
| `shuyi-sound-out` | sound output: WASAPI, Core Audio, ALSA, Web Audio | no |
| `shuyi-pads` | controllers: Windows, Linux, macOS; the mapping database | no |

3034 names the crates for the window and the graphics.

- **The engine adapts them.** `shuyi-assets` registers the readers as
  formats (ADR-0007, ADR-0019). `shuyi-app` (or the backend crate that
  3003 leaves) implements the `Platform` trait's parts on them.
- **Their APIs are plain**, documented and tested on their own: a decoder
  takes bytes and gives samples; the pads crate gives a list of pads with
  standard buttons and axes. They use their own types, never the engine's
  `Button` or actions, and the engine maps them (`AGENTS.md` rule 8).
- **They are MIT OR Apache-2.0**, with the workspace's lints and gates.

### 4. The old crate leaves in the same PR as its last replacement

When the last ticket of a piece lands (3032 for sound, 2403 for
controllers, 3033, 2215), it removes the old crate from `Cargo.toml`,
and `cargo tree` shows it gone. Until then, the old and the new run side
by side behind the same seam. A platform not yet replaced keeps the old
crate.

### 5. `unsafe` in the platform crates only, the way `shuyi-app` does it

ADR-0002 §9 forbids `unsafe` outside `shuyi-app`. Calling an operating
system's API needs it, so the crates that talk to the system get
`shuyi-app`'s exception and its rules:

- **The platform crates**: `shuyi-sound-out`, `shuyi-pads`, and the window
  and graphics crates 3034 names. Each sets `unsafe_code = "deny"` instead
  of `forbid` in its `[lints]`, and keeps a test that its lints are the
  workspace's except that one, as `crates/app/src/lib.rs` does.
- **`unsafe` lives only in a `sys` module** of each crate, under one
  `#[allow(unsafe_code)]` on that module. The rest of the crate wraps it in
  safe types.
- **Every `unsafe` block has a `// SAFETY:` comment** saying why it holds
  (clippy's `undocumented_unsafe_blocks`, turned on in those crates).
- **`shuyi-sound` stays `forbid`.** The decoders and the mixer are plain
  code, and need no `unsafe`.

## Consequences

- No source code from another project, so the license is clean, and the
  pieces can be published on their own.
- Writing a Vorbis decoder, sound output on three systems and controllers
  on three systems is real work: about fourteen tickets before macroquad
  is even started. It is parked until the MVP, so the game doesn't pay for
  it now.
- We own the bugs. Hardware differences (pads, sound cards) that gilrs and
  quad-snd already handle will come back as bug reports. Testing on real
  devices is part of each output and controller ticket.
- `shuyi-sound` being `no_std` means one decoder and one mixer for every
  target, the Playdate (3029) and the GBA (3027) included.
- The web keeps working without `wasm-bindgen`, through our own JS
  plugins, as today.

## Alternatives considered

- **Swap one library for another** (cpal and symphonia for quad-snd,
  winit and wgpu for macroquad). Lost: it is not in-house, which was
  Nick's call. symphonia is MPL-2.0 and denied anyway. winit and wgpu
  need `wasm-bindgen` on the web.
- **Fork the old crates and maintain them.** Lost: forking is copying
  their source, and we would own code we didn't write.
- **Port a small public-domain decoder (stb_vorbis).** Lost: a line-by-line
  translation is exactly what Nick ruled out, whatever the license.
- **Put the new code inside `shuyi-app`.** Lost: it couldn't move to its
  own repository later, and the Playdate and the GBA couldn't share the
  decoder and the mixer.
