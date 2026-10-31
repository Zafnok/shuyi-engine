# ADR-0062: The Playdate backend: a C shim, a one-bit look in the frame's `Look`, and a game's library built as a static library

- **Status:** Accepted; extends ADR-0008 §2 and ADR-0024
- **Date:** 2026-10-30
- **Related tickets:** 3010 (this), 3006 (one-bit frames), 3007 (the
  pack's one-bit pictures), 3011 (the device), 3028 (the stack), 3029
  (audio), 2410 (the buttons' names)

## Context

ADR-0008 §2 makes the Playdate a backend crate "over the MIT `playdate`
crates". Ticket 3010 builds it, and leaves three choices open:

- **The bindings.** On 2026-10-15 the game's ADR-0070 found boozook's crates
  unusable. Checked again on 2026-10-30: the last commit is of 2026-07, and
  the prebuilt bindings still stop at SDK 2.6.2, where the SDK is at 3.1.2.
  The game's own C shim, compiled with the SDK's headers, works today.
- **How the one-bit frame reaches the backend.** `Show::Game` carries the
  colour `GlyphBuffer` and a `Look` of a font and pictures. Drawing it in one
  bit also needs a `PatternMap`, the font's one-bit rows and one-bit pictures.
- **How a game becomes a `.pdx`.** The SDK loads a game linked from a static
  library, the shim and its `setup.c`, with no `std`. A game's package is a
  library for its tests and tools and a program for the window.

## Decision

1. **A C shim, not the `playdate` crates.** `crates/playdate/shim/shim.c` is
   the SDK's entry point and one small wrapper per call, compiled against the
   installed SDK's `pd_api.h`, so the C compiler checks each call against the
   real API. No nightly Rust, no bindgen, no SDK header in the repository. This
   changes ADR-0008 §2's "over the MIT `playdate` crates".
2. **A fake behind a small trait.** Everything but the C calls is written
   against `shuyi_playdate::sdk::Sdk`. `Driver` is the update callback's body,
   so the host tests run the whole loop on a fake.
3. **The one-bit look is part of `Look`.** `Look` gains an optional
   `one_bit: Option<OneBitLook>`: `map`, `font` and `images` as
   `render_with` takes them. The game's `look` function gives it
   (`Look::with_one_bit`), and its load makes it from its pack. The backend
   draws, not `App`: it redraws only what changed into a frame of its own
   (`render_changed_with`), then copies and marks the rows that differ from the
   SDK's frame buffer. A one-bit backend given no `one_bit` says so on screen.
4. **`--cfg shuyi_playdate_sdk`, not a cargo feature.** The allocator, the panic
   handler and the entry points are behind a cfg the exporter sets, so
   `--all-features` builds, which have `std`, never see them.
5. **A game's library, built as a static library.** A game adds a `playdate`
   feature and `shuyi_playdate::main!(start)`. The exporter runs `cargo rustc
   --lib --crate-type staticlib --no-default-features --features playdate`, with
   the profile's settings (`panic = abort`, LTO) in Cargo's variables, so a game
   needs no extra crate or profile. `shuyi` gains the default features `std` and
   `app`, so a game turns off the backend it can't build there.
6. **Input and the crank.** A is `East` and B is `West`, as the profile lists.
   `getCrankChange`'s float becomes whole steps, 256 to the turn, at the
   boundary, what rounds away carried to the next frame so a slow turn still gets there
   (`from_crank_steps`). The notch defaults to 45°. `SystemEvent::CrankDocked`
   and `CrankUndocked` tell `App`, and the game draws the hint itself, over any
   screen whose required action is only on the crank.
7. **The system's events run at once where they must.** The SDK stops calling
   the update callback while the system menu is open or the device locked, so a
   lock, a pause, a terminate and low power run one frame in the event itself:
   the game's hook saves before the updates stop.
8. **A panic on the Simulator leaves the game's call for good.** `error` comes
   back there, on the thread the window runs on, so spinning would hang the
   window with the message unread. The shim jumps out of the game's call and
   makes no call again.

## Consequences

- A game on the Playdate has its library free of `std`: it embeds no files,
  and reads a pack. Hello, Grid shows how, in one file and a feature.
- CI can't run the export: the SDK's license doesn't let it be handed on. It
  builds the backend and the example's library for the device, without linking.
- `CrankDef::default` moved from 30° to 45°, which changes the golden hash of a
  keymap that names no `crank`.
- The shim's copy in the exporter is `include_str!`, as `shuyi-app` embeds the
  target profiles: the two can't drift.

## Alternatives considered

- **The `playdate` crates** — unusable today (above). Worth re-checking when
  their bindings reach the current SDK.
- **`App` makes the `OneBitFrame`** — it would draw a whole frame every frame
  and hand over a copy, where the backend redraws only what changed and has
  nowhere else to keep it. Every other backend would carry the code for nothing.
- **A separate entry crate per game** — what the game has now. Another crate for
  every game, where a feature does it.
- **A cargo feature for the SDK's parts** — `--all-features` builds would break
  on two panic handlers.
