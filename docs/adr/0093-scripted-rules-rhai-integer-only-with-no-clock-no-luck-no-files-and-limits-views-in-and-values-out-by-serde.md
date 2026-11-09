# ADR-0093: Scripted rules: Rhai, integer only, with no clock, no luck, no files, and limits; views in and values out by serde

- **Status:** Accepted; extends ADR-0084 §7 (2101's part) and ADR-0009
  (the first module crate)
- **Date:** 2026-11-08
- **Related tickets:** 2101 (this), 2102 (its example), game#1017 (the
  game's scripted rules), 1135 (visual scripting), 2905, 2020

## Context

Nick asked for "a scripting DSL or something for easy modding support".
The game's rules will call scripts and turn what they return into effects
they already know (game#1017). The runtime must not break what the
engine promises:

- **Determinism** (ADR-0002 §2): the same script on the same view gives
  the same values on x64, ARM64 and wasm32. That means no clock, no host
  randomness, no floats in a simulation, and nothing that iterates in
  hash order.
- **A sandbox:** no file or network access, and a run that always ends.
  A modder's script is untrusted.
- **The web build** loads a bare `wasm32-unknown-unknown` module through
  macroquad's own loader (game ADR-0034). There is no wasm-bindgen, no
  emscripten and no C library. ADR-0004 §7 already names `mlua` as a trap
  for this reason.
- **Licenses** (ADR-0002 §8): permissive only.
- **ADR-0084:** a script reads a game's view types, never its state.
  Values go in and come out through serde, with no registry of the
  module's own.

### What was measured

All on 2026-11-08, on Nick's machine (Windows 11, the GNU toolchain,
Rust 1.99.1), in the engine's release profile (thin LTO, one codegen
unit, stripped). Each candidate ran the same rule ("twice the growth in
rain") behind the same exported function:

- **Standalone:** a crate that is both a `cdylib` (`.wasm`) and a binary
  (`.exe`). The empty harness is 271,360 bytes native and 67 bytes as
  wasm.
- **In a game:** Hello, Grid's web build calling the rule from `main`.
  Its baseline `.wasm` is 3,137,263 bytes.

"Runs on the web" means the `.wasm` builds with no import a bare module
can't satisfy: no `__wbindgen` import, no emscripten, no getrandom
backend set by a flag on the final build.

| Candidate | License (its `Cargo.toml`) | Native, added | `.wasm` alone | Hello, Grid `.wasm`, added | Runs on the web | Without `std` |
| --- | --- | --- | --- | --- | --- | --- |
| **Rhai 1.27**, the standard package | `MIT OR Apache-2.0`; needs `smartstring`, `MPL-2.0+` | +1,633,280 | 1,557,487 | +1,645,863 | yes | no (below) |
| **Rhai 1.27**, the packages §2 keeps | the same | +1,377,792 | 1,318,439; 1,303,503 with `compact_str` | +1,396,580 | yes | no |
| **`shuyi-script`** as built here | the same | +1,389,056 (in Hello, Grid) | — | **+1,373,820** | yes: 0 `__wbindgen` imports | no |
| wasmi 2.0 (WebAssembly modules) | `MIT/Apache-2.0` | +1,172,480 | 1,339,051 | +1,357,478 | yes | yes |
| Lua 5.4 through `mlua` 0.12 (C) | `MIT` (Lua's `lua-src`: `MIT`) | +489,984 | doesn't build: "don't know how to build Lua for wasm32-unknown-unknown" | — | no: emscripten only | no |
| A language of our own (a 150-line spike: integers, fields, `if`) | ours | +17,408 | 41,707 | +15,950 | yes | yes |
| Rune 0.14.2 | `MIT OR Apache-2.0` | +5,000,704 | 4,267,334 | — | yes | no: `memchr` brings `std` |
| Koto 0.16.1 | `MIT` | +1,626,624 | 1,917,622 | — | no: 4 `__wbindgen` imports (`instant`, `chrono`) | no |
| Starlark 0.14.2 (Python-like, Meta) | `Apache-2.0` | +5,155,840 | 4,199,271, only with a getrandom flag on the final build | — | no | no |
| piccolo 0.3.3 (Lua in Rust) | `MIT` | +446,976 | 493,120, only with a getrandom flag on the final build | — | no | no |
| Boa 0.22 (JavaScript) | `Unlicense OR MIT` | +7,499,264 | 5,652,067, only with a getrandom flag on the final build | — | no | no |
| RustPython 0.6 | `MIT`; `malachite` is `LGPL-3.0-only` | +17,378,816 | 12,640,866 | — | no: the license | no |

### Determinism and the sandbox, by candidate

- **Rhai:**
  - `no_float` takes floats out of the language: `1.5` doesn't compile.
  - Its one integer is a checked `i64`, so an overflow is an error.
  - `no_time` takes out its clock. It has no random numbers.
  - `no_module` takes out `import`, its one way to read a file.
  - Object maps are `BTreeMap`s, so they iterate in key order.
  - `set_max_operations` and the size limits stop any run.
- **wasmi:** deterministic and sandboxed by design: `Config::floats(false)`
  and fuel. But a modder writes Rust or Zig and compiles it, which is far
  from easy modding. Views would cross through linear memory by an ABI
  of our own.
- **Lua and CPython:** both are C, which the web build can't take. Lua
  also seeds its string hashing from the clock and memory addresses
  (`luai_makeseed`, `lstate.c`), so `pairs` changes order between runs,
  and its `/` is always a float. CPython has floats, string-hash
  randomization, and no way to cap a run.
- **Rune:**
  - Floats can't be removed.
  - Its objects iterate in hash order. The same script printed a
    different key order in a debug and a release build of one program.
    The research also found the order differs between x64 and wasm32.
  - In 0.14.2 its budget misses native iterators, and `-i64::MIN` wraps
    in release. Both are fixed only on the unreleased 0.15.
  - It breaks embedders at every 0.x release.
- **Koto:** its only limit is wall-clock time, and its `import` reads
  files.
- **piccolo:** experimental, no commits since 2025-07. Its tables hash
  with a random seed (`pairs` order changed on every run), and floats
  stay.
- **Boa:** all numbers are floats, and it has no total operation count.

### Speed, Rhai against Rune

Natively, median of five, the same scripts:

| | Rhai (§2's config) | Rune |
| --- | --- | --- |
| `fib(25)`, recursive | 79.8 ms | 10.3 ms |
| A loop of 1,000,000 | 132.1 ms | 105.7 ms |
| 100,000 calls, each with a view | 35.2 ms | 31.4 ms |
| Compiling 300 functions | 2.0 ms | 10.2 ms |

A rule called once a turn or per event costs about a third of a
microsecond either way. Rhai is several times slower on deep calls.

Bevy's scripting plugin (bevy_mod_scripting) swapped Rhai for Rune in
October 2026: its PR 640 and issue 518. The reason it measured was the
speed of its own reflection glue into Bevy's ECS. Its benchmarks moved
from 8–13× Lua's time to about 1×. This engine hands a script a plain
view and has no such glue. The plugin doesn't run on the web.

### Without `std`

Rhai's `no_std` feature builds for `thumbv7em-none-eabihf` on stable Rust
without `serde`. With `serde`, 1.27 fails to compile
(`src/serde/serialize.rs:85` names `Union::Shared` outside its `cfg`).
The serde bridge is how views go in (ADR-0084 §2), so the module needs
`std`.

### The one license exception

Rhai 1.27 always depends on `smartstring` 1.0.1, which is `MPL-2.0+` and
unmaintained (RUSTSEC-2026-0249). Rhai's maintainers replaced it behind
the `compact_str` feature (rhaiscript/rhai PR 1175, merged 2026-09-29,
partly for the license) and plan that as the default in 2.0, which has
no date. With `compact_str`, no Rhai code names `smartstring`. Neither
the Windows release build, the debug build nor the `.wasm` holds any of
its code. MPL-2.0 binds those who distribute its code (Mozilla's MPL 2.0
FAQ, Q5), so no game ships any. Not legal advice. Microsoft's
one-collect and TiKV make the same one-crate exception. Nick allowed it
(`docs/decisions.md`, 2026-11-08).

## Decision

### 1. The language is Rhai 1.27

`shuyi-script` (`crates/script`) is a module (ADR-0009 §4). It depends
on `rhai` with `default-features = false` and these features:

- `std`;
- `serde`: the bridge (§4);
- `no_float`: integers only;
- `no_time`: no clock;
- `no_module`: no `import`, so no files;
- `no_custom_syntax`;
- `only_i64`: one integer type;
- `compact_str`: no `smartstring` code (§7).

Default features stay off. They would bring `ahash`'s runtime random
seed, and getrandom with no web backend.

### 2. The language as configured

`Runtime::new` builds an `Engine::new_raw()` (no file resolver, no
output) and registers:

- `CorePackage`: arithmetic, strings and iterators;
- `LogicPackage`: comparisons, `min` and `max`;
- `BasicArrayPackage`;
- `BasicMapPackage`.

It drops the standard package's bit fields, byte blobs, extra string
functions and maths, about 240 KB of `.wasm`; a later ticket can add
one back. It also sets:

- **`print`, `debug` and `eval` disabled:** they don't compile;
- **strict variables:** a misspelt name doesn't compile;
- **a missing map property is an error**, not `()`.

### 3. Limits

`Limits` holds the limits, kind 4 of ADR-0039 (`#[non_exhaustive]`,
`Default`, fields assigned). Rhai takes 0 as "no limit", so a 0 here is
taken as 1. The defaults:

| Field | Default | What stops |
| --- | --- | --- |
| `operations` | 100,000 | a run's operations |
| `text_bytes` | 4,096 | a string's bytes |
| `list_items` | 1,024 | a list's items, nested lists counted |
| `map_entries` | 256 | a map's entries, nested maps counted |
| `variables` | 256 | the variables in scope |
| `call_depth` | 32 | how deep calls go |
| `expression_depth` | 64 | how deep an expression nests, at compile |

Memory is bounded by these together: each value's size is capped, and
the operations cap how many values a run can make. One gap: Rhai doesn't
check a map's size on `m[k] = v` (rhaiscript/rhai issue 1213). The
operation limit still stops it, and a test checks that.

### 4. The API

- **`Runtime::compile(name, source) -> Result<Script, ScriptError>`.**
  `name` is the bundle's display path of the file, and every error names
  it.
- **`Runtime::call(&script, function, &view) -> Result<R, ScriptError>`**
  calls one function with the view as its argument. The script's
  top-level code doesn't run.
- **`Runtime::run(&script, &view)`** runs the top-level code with the
  view as the constant `view`.
- **Views go in** through `rhai::serde::to_dynamic` from the game's
  `Serialize` view type (ADR-0084 §2, §4): a struct becomes a map, and
  an enum is externally tagged.
- **Values come out** through `rhai::serde::from_dynamic` into the
  game's `Deserialize` type, or into `Value`: unit, a bool, an `i64`, a
  string, a list or a map in key order. `Value`'s `Display` is the
  script's own form, which the golden table stores.
- **No type is registered**, so the runtime names none: a view is a map
  in the script, and `type_of(view)` is `"map"`. ADR-0084 §3's names
  apply where the runtime names a type. A later ticket that registers
  types names them as §3 says.
- **Errors are values.** A script that doesn't compile, fails, stops at
  a limit, or gives what doesn't read is a `ScriptError`: `script`,
  `line`, `column` and an `ErrorKind` (`Compile`, `Limit(Limit)`, `Run`,
  `View`, `Value`). Its `Display` is `file:line:column: what`. Nothing
  panics; the caller decides what an error means.
- A `Runtime` isn't `Send` (Rhai without its `sync` feature). A thread
  that runs scripts makes its own.

### 5. Determinism, checked

- **Unit tests:**
  - a float doesn't compile, and an overflow is an error;
  - `timestamp()`, `rand()`, `import`, `open()`, `http_get()`, `print`
    and `eval` each fail to compile or error;
  - a loop forever stops at `Limit::Operations`;
  - a map iterates in key order.
- **A property test:** two runtimes run one rule on generated views and
  must agree with each other and with a Rust oracle.
- **A golden table**, `crates/script/tests/golden/scripts.ron`
  (`shuyi_assets::golden`, ticket 2116), holds what three fixture
  scripts give on fixed views: a crop, a door and a card. CI's `test`
  job checks it on x64 and ARM64, and its `determinism` job on wasm32
  (`-p shuyi-script`).
- Rhai hashes function names with `ahash` under its compile-time seed.
  That only finds functions; no script sees a hash order.

### 6. A library, not a frame-loop module

The runtime installs no `Module`: a game's rules call it, as ADR-0040's
library-first default says. The `shuyi` crate's feature `script` turns it
on and re-exports it as `shuyi::script`, with no line in
`enabled_modules`. It needs `std`, so the feature turns `std` on. It
doesn't build without `std`. A game that builds its core without `std`
keeps scripting behind a feature of its own.

### 7. The license exception

`deny.toml` allows `smartstring@1.0.1` under MPL-2.0 alone, and ignores
RUSTSEC-2026-0249, each with why. A test, `smartstring_is_never_linked`,
fails if Rhai's map keys stop being `compact_str`. A game that runs
`cargo deny` adds the same exception, as the module's README shows. Both
go when Rhai drops `smartstring`.

### 8. The web build

`rhai` depends on `web-time` on wasm, and through it on wasm-bindgen.
With `no_time` nothing calls it, so the `.wasm` holds no `__wbindgen`
import (measured above). CI's `wasm` job builds the crate for
`wasm32-unknown-unknown`. 2102's example, built and opened in the
browser, shows the loader runs it.

## Consequences

- Modders get a small, safe language that reads like JavaScript and
  Rust, with integer maths, maps and lists, functions and closures, and
  errors that name a file, line and column.
- A game that turns it on pays about 1.37 MB on the web and 1.39 MB
  natively. A game that doesn't pays nothing.
- Replays, rewinds and lockstep stay exact with scripted rules, checked
  on three architectures.
- It doesn't work on the Playdate, or anywhere without `std`, until Rhai
  builds there with `serde`.
- Rhai is mostly one maintainer (81% of a year's commits). It is very
  active and MIT/Apache, so it can be forked. The engine's API names no
  Rhai type, so a swap would change the module, not a game's code. A
  game's scripts would still need rewriting.
- One MPL crate sits in the build graph until Rhai 2.0. A test and the
  exception's comment keep it from being linked by accident.
- A game reads values with its own types, or with `Value`.

## Alternatives considered

- **Rune** is faster on deep calls and has better error spans and an
  LSP. It lost on determinism: its maps iterate in an order that changes
  with the build and the architecture, and floats can't be removed. Its
  limit and overflow fixes are unreleased, it breaks embedders at each
  0.x, and it is 3.3× Rhai's size on the web.
- **Lua through `mlua`** is the most used in games. It is C, which the
  web build can't take: `lua-src` has no `wasm32-unknown-unknown` build,
  and mlua supports emscripten only. Making it work means a new web
  backend, and then patching Lua's hash seed. Its floats stay.
- **Lua in Rust (piccolo)** is small, but experimental and inactive.
  Its table order is random, and it needs a getrandom flag in every
  game's web build.
- **Python:** CPython is C, like Lua. RustPython is 12.6 MB, has an LGPL
  dependency and has no run limit. Starlark is hermetic and well kept,
  but has no operation budget, has 174 dependencies, and builds for the
  web only with a getrandom flag.
- **JavaScript (Boa)** is well maintained, but it is 5.6 MB, all floats,
  has no operation count, and needs the same getrandom flag.
- **Koto** needs wasm-bindgen on the web, reads files through `import`,
  and limits only wall-clock time.
- **WebAssembly modules (wasmi)** are deterministic, sandboxed,
  permissive and build without `std`. They lost on modding: a modder
  compiles Rust or Zig, and views need an ABI of our own. They stay the
  answer if mods ever need full languages.
- **A language of our own** is 16 KB and builds without `std`. Nick
  turned it down: "I rather not work on our own language as well as our
  own engine that's major feature bloat".
- **Waiting for Rhai 2.0** to drop `smartstring`: no date is announced,
  and the game's modding (game#1017) would wait with it.
- **A patched-out `smartstring`** (`[patch.crates-io]` to an empty
  crate) works in this workspace only. Patches don't reach a game that
  depends on the engine.
