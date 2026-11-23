# ADR-0139: In-house sound on Linux and macOS: quad-snd's ALSA bindings are `alsa-sys`'s, one CPAL output for every system, and Linux's evidence on the same sink

- **Status:** Accepted
- **Date:** 2026-11-23
- **Related tickets:** 3032 (this), 3031, 3084, 3079, 3071, 3320, 3347
- **Amends:** ADR-0075 (its held `alsa-sys`)
- **Extends:** ADR-0066, ADR-0067, ADR-0068

## Context

ADR-0075 let CPAL and quad-snd share a build by patching `alsa-sys` to an empty crate
and building CPAL only for Windows, since each brings a crate that says
`links = "alsa"`, which Cargo allows once in a build on any target. Linux needs the
real ALSA bindings, so 3032 had to replace that first, and prove quad-snd unharmed.

The two crates are bindings of one C library, `libasound`, made by `bindgen` from its
headers: `quad-alsa-sys` 0.3.2 is a fork of `alsa-sys` 0.3 that links with
`-lasound` instead of `pkg-config` (its manifest says so), and `alsa-sys` 0.4.0,
which CPAL's `alsa` 0.11 needs, is the newer and the larger. Both are MIT.

3031's output was a thread that owns a CPAL stream and watches the default device,
and nothing in it was Windows' but the audio thread's priority (ticket 3089).

ADR-0067's Linux evidence is a PulseAudio null sink with ALSA routed to it, which a
new backend on Linux can use as it is.

## Decision

**quad-snd's bindings are `alsa-sys`'s.** The root `Cargo.toml` patches
`quad-alsa-sys` with `tools/sound-parity/quad-alsa-sys-held`: a crate of that name and
version, with no `links`, whose whole source is `pub use alsa_sys::*;` over the real
`alsa-sys` 0.4. quad-snd compiles against it unchanged (its calls are a subset), and
`alsa-sys` finds and links `libasound` for both, with `pkg-config`, which `doctor`'s
`lib-alsa` row already checks. Every build of the workspace links the same library as
before. quad-snd's own cells on Linux are the proof that nothing it does changed. As
with ADR-0075's patch, a game that turns `in-house-sound` on adds the same
`[patch.crates-io]` line, and 3079 removes the patch with quad-snd.

**One output for every system CPAL serves.** `shuyi-sound-out`'s `windows` module is
now `system`, built on Windows, Linux and macOS, with CPAL's default host on each:
WASAPI, ALSA and Core Audio. None of CPAL's features: no ASIO, JACK or PipeWire. The
same thread owns the stream, rebuilds it when it errs or the default device moves, and
mixes into nothing while there is none. On Linux the default is ALSA's `default`
device, behind which a sound server or a card stands, and a lost device is a stream
that errs. Only Windows raises the audio thread (`sys::raised`); elsewhere CPAL's own
thread is as CPAL makes it, which 3320 decides.

**Linux's evidence is the same sink.** `capture.py native --new-probe` records the
in-house probe beside quad-snd's on the one null sink: `linux-in-house` is a
`NewBackend` in `budget.ron`, every cell mandatory, `old-new` cells against quad-snd,
and a `device-loss` cell, for which the sink is unloaded while the probe plays and
loaded again, and the time until it is heard is held to a second.

**What a Linux build needs, measured, not assumed.** Both probes name
`libasound.so.2` in `DT_NEEDED` (a CI step fails otherwise): CPAL links ALSA, it doesn't
open it with `dlopen`, so neither backend starts where that library is missing. That
was so before, and the packages say they need ALSA. Whether a Linux game must start
silent without it is 3347's, and 3084 waits on it. `sound-linux-floor` builds both
probes in the Steam Linux Runtime's sniper SDK, where packages are built, and holds
them to glibc 2.31.

**macOS is built and tested, not recorded.** CI's `sound-macos` job runs the output's
and the backend's tests on Apple silicon and Intel runners, joins the probe's two
slices with `lipo`, and `mac-floor.sh` checks the oldest macOS it starts on: every
function it imports, not weakly, from the audio frameworks must be in the SDK's
headers no later than the `minos` rustc wrote in the program, or clang's
`-Werror=unguarded-availability` says which. CPAL's documentation names macOS 14.2;
this holds what the program itself asks for. Capture stays `Blocked` in `budget.ron`
until a route works on a hosted runner, so `port-ready macos` refuses, and 3084 can't
make the in-house backend macOS's default.

## Consequences

- One build holds both backends on every system, so the two can be recorded side by
  side on Linux, as on Windows and in the browser.
- quad-snd's Linux build now needs `pkg-config` to find ALSA, where it used `-lasound`
  alone. Linux setup already installs both, and `doctor` checks that way.
- A sound PR's CI gains two macOS jobs and a container build of a few minutes each;
  `calibrate.yml` leaves them out (`ports: false`), as they measure no timing.
- macOS's in-house backend has no recorded evidence; a person on a Mac is the only
  listener until a capture route works.

## Alternatives considered

- **`alsa-sys` held as `quad-alsa-sys`'s bindings:** `alsa` 0.11 is written against
  `alsa-sys` 0.4's newer bindings, which `quad-alsa-sys` 0.3 lacks; the larger set
  standing in for the smaller is the direction that compiles.
- **Copying `quad-alsa-sys`'s bindings without `links`:** nine thousand generated lines
  in the repository for the same declarations `alsa-sys` already has.
- **A raw ALSA or Core Audio output of our own:** ADR-0066 chose CPAL, and the ticket
  forbids one merely to follow the old outline.
- **macOS capture through a virtual driver from a third party:** the one in common use
  on runners is GPL-3.0, which ADR-0002 §8 denies, as 2573 ruled out denied drivers.
