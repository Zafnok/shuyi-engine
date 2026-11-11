# shuyi-script

Rules written as text. A game's modders write a small rule in
[Rhai](https://rhai.rs), such as a crop that grows faster in rain, a door
that opens when three switches are on, or a card's text. The game
compiles it once from its bundle, and its rules call it with a read-only
view and turn what it returns into effects the game already knows. A
script can't change the game's state, read a clock, draw a random number,
open a file or reach the network. Its maths is integers only, and every
run stops at an operation limit and size limits. The same script on the
same view gives the same values on every platform, so replays, rewinds
and lockstep stay exact. It serves turn games and fixed-tick games alike
([ADR-0093](../../docs/adr/0093-scripted-rules-rhai-integer-only-with-no-clock-no-luck-no-files-and-limits-views-in-and-values-out-by-serde.md)).

## What it is not for

- **The game's own rules.** Write those in Rust, in the game's rules
  crate. A script is for what modders or designers change without a
  build.
- **Changing state directly, or running on a clock.** A script returns
  values; the game decides what they do and applies them as commands or
  inputs. It joins no frame loop: the rules call it, as a library.
- **Floating-point maths.** A rule that needs fractions uses fixed-point
  integers, as the engine does.
- **A console without `std`.** It doesn't build for
  `thumbv7em-none-eabihf` (the Playdate): Rhai's `serde` bridge needs
  `std` there. A game that builds its core without `std` keeps scripting
  behind a feature of its own, on only where `std` is.
- **Visual scripting.** That is a later tool that writes these scripts
  (ticket 1135).

## What it costs

- **Size**, measured on 2026-11-08 with Hello, Grid calling a script,
  release builds: **+1,373,820 bytes** of `.wasm` (3,137,263 → 4,511,083)
  and **+1,389,056 bytes** of Windows `.exe`. The web build has no
  wasm-bindgen import, so it runs under macroquad's loader.
- **Dependencies:** `rhai` 1.27 (MIT OR Apache-2.0), `serde`, and about
  thirty crates Rhai brings, under MIT, Apache-2.0, CC0-1.0, BSD-2-Clause
  or Unicode-3.0 (checked with `cargo deny`). One
  exception: Rhai always depends on `smartstring` (MPL-2.0). This crate
  turns on Rhai's `compact_str` feature, which replaces every use of it,
  so its code is built but never linked into a program. The engine's
  `deny.toml` allows that one crate for that reason. A game that runs
  `cargo deny` needs the same exception:

  ```toml
  [licenses]
  exceptions = [{ crate = "smartstring@1.0.1", allow = ["MPL-2.0"] }]

  [advisories]
  ignore = [{ id = "RUSTSEC-2026-0249", reason = "smartstring, via rhai: built, never linked" }]
  ```

- **Speed:** about a third of a microsecond for a small rule called with
  a view, natively. Rhai walks a tree, so a script that calls its own
  functions deeply is several times slower than Rust. Keep rules short
  and call them per turn or per event, not per pixel.
- A `Runtime` isn't `Send`: a thread that runs scripts makes its own.

## How to start

Turn it on with `shuyi add script` (the `shuyi` crate's `script`
feature), or depend on `shuyi-script` directly. Then:

```rust
use serde::Serialize;
use shuyi::script::{Limits, Runtime};

/// What a script sees of a crop: the game's own view type, made from
/// its state, with nothing the script must not see.
#[derive(Serialize)]
struct CropView {
    growth: i64,
    rain: bool,
}

let runtime = Runtime::new(Limits::default());
// Compiled once, at load. The name is the file's display path, which
// every error names.
let script = runtime.compile(
    "mods/rain.rhai",
    "fn grows(crop) { if crop.rain { crop.growth * 2 } else { crop.growth } }",
)?;
// Called by the rules, with a view; the answer is read as the game's
// type (here `i64`), or as a `shuyi_script::Value`.
let grows: i64 = runtime.call(&script, "grows", &CropView { growth: 3, rain: true })?;
assert_eq!(grows, 6);
```

`Runtime::run` runs a script's top-level code instead, with the view as
the constant `view`. A script that fails to compile, fails while running
or stops at a limit gives a `ScriptError` naming its file and line, never
a panic: what that means for the game is the game's to decide.

The view is a struct of the game's own, made from its state for one side:
the luck to come (a `SimRng`) and what the side can't see are left out of
the type (ADR-0084 §4). It goes in through its `Serialize` derive, and
what comes back is read through the game's `Deserialize` types.

## Beside its alternatives

The other answer to *Scripted rules* is none: rules in Rust. A game moves
a rule from Rust to a script by writing its view type and the script, and
calling the script where the rule was; the effect stays the game's own
code. Moving back is the reverse. The two share a game freely: most rules
stay Rust, and only the ones meant for modding are scripts.
