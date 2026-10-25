---
id: "3010"
title: "shuyi-playdate: the Playdate backend (update callback, d-pad, A, B and crank, 1-bit display with changed rows, storage) and shuyi export playdate"
type: feature
milestone: After MVP
model: opus-5.5
effort: xhigh
status: todo
blocked_by: ["3002", "3003", "3004", "3005", "3006", "2204", "3007", "3045", "3040", "3048", "3053"]
nick_input: setup
completed:
---

# 3010 — The Playdate backend

## Context

Nick, 2026-10-15 (`docs/decisions.md`): "I want my visions of shuyi to
run on playdate". [ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§2 makes the Playdate a backend crate, `shuyi-playdate`, over the
`playdate` Rust crates (`github.com/boozook/playdate`, MIT on
2026-10-15; re-check each crate's license with `cargo deny`). This
ticket depends on the earlier ones:

- the crates are `no_std` (3005);
- `App::frame` drives the game (3003);
- `to_one_bit` makes the 1-bit frame (3006); how it reaches the backend
  is this ticket's (*The display*, below);
- the assets come as a baked pack (2204, 3007);
- the controls check guards the inputs (3002);
- saves survive a power cut, and a suite says what that means (3045);
- `App` keeps a log (3040);
- the game's name, id, version and icon are in `shuyi.ron` (3048);
- a backend reports its platform's facts (3053).

The device facts (ADR-0008, *Context*): 400 × 240 1-bit, refresh up to
50 Hz, a Cortex-M7 at about 168 MHz, 16 MB of RAM, the d-pad, A, B, the
crank, and the Menu button, which belongs to the system.

**From the game's first engine-friction sweep** (game#1153,
2026-10-19). The game already runs on the Playdate with its own front
end (`crates/playdate` in the game, about 1,700 lines, its ADR-0070):
working code for most of this ticket. Read it before step 1.

- **The bindings.** The game's ADR-0070 §2 tried boozook's crates on
  2026-10-15: `cargo-playdate` did not build on current nightly; with
  its pinned nightly its Cargo could not read edition-2024 manifests;
  its prebuilt bindings stop at SDK 2.6.2. The game uses a small C shim
  compiled with the SDK's own headers instead (`shim/shim.c`, no
  nightly, no bindgen). Re-check the crates in step 1, and take the
  shim if they are still unusable.
- **The buttons.** The game reports the SDK's A as `South` and B as
  `East` (`src/pads.rs`), where this ticket says `East` and `West`.
  Pick one rule and write it in the docs. **From 3002 (done):** `East`
  and `West` are already engine data: `targets/playdate.ron`'s buttons
  and A+B chord (3001), hello-grid's `targets."playdate"` table and
  3002's tests. Keeping them changes none of that, and the game moves
  its own table off `South` (3002's notes). `South` and `East` change
  all of them in this PR. Either way, a fake-SDK test checks that A and
  B report the buttons the profile lists.
- **Storage, memory and the panic handler** are in the game's
  `src/storage.rs` (a `no_std` copy of `shuyi-app`'s native storage),
  `src/pd/allocator.rs` and `src/pd/panic.rs`.
- **`pdc` crashes on a big unoptimized ELF** (game#1056, 2026-10-22;
  added from the game's retro of its last 50 PRs). The SDK's
  `link_map.ld` discards `.ARM.exidx` but not each function's own
  `.ARM.exidx.*`, and `--emit-relocs` keeps a `.rel.ARM.exidx.*` beside
  each. An unoptimized game of about 57,000 functions makes an ELF of
  more than 65,280 sections, which uses the ELF's extended section
  numbering (`e_shnum = 0`), and `pdc` 3.1.2 crashes reading it
  (`0xc0000005`, no message). The release build (LTO, one codegen unit)
  stays far under. The game's fix: after the device link of a debug
  build, `arm-none-eabi-objcopy --remove-section=.ARM.exidx*
  --remove-section=.rel.ARM.exidx*` (the game's
  `crates/xtask/src/playdate.rs`, `device_steps`). Any game built on
  this backend will hit it once it grows; the exporter's build step
  does the same for a debug build.

## Nick input

**Setup** (about 15 minutes, free):

1. Go to <https://play.date/dev/>, read Panic's SDK license, and download
   the Playdate SDK for Windows if you accept it. The license is Panic's,
   not ours, and the session will point out anything in it that matters
   for selling.
2. Run the installer with its defaults. It sets `PLAYDATE_SDK_PATH`.
3. Tell the session the SDK version it installed.

Nothing here costs money. A device is needed only for 3011.

## Scope

**In:**
- **The crate `shuyi-playdate`**, `no_std`, implementing `Platform`
  (3003):
  - **The loop.** The SDK's event handler sets the update callback, and
    each update calls `App::frame` with
    `getCurrentTimeMilliseconds()`. Use `display->setRefreshRate` from
    the profile (30).
    **Either order with 2321 (the frame cap):** if 2321 is done, the
    loop reports a `Pacing` through `App` each frame, as `shuyi-app`
    does, with the rate it runs at (the profile's 30) as the applied
    cap and the latest `frame_cap_serial`, so a game's
    `request_frame_cap` is `Applied`; if not, nothing to do (2321 does
    it, its clause says so).
  - **Input.** `getButtonState` for the d-pad and A and B (`East` and
    `West`). The A+B chord comes from the engine's chord handling. The
    crank: `getCrankChange` becomes `CrankForward` and `CrankBack` once
    per detent. The detent defaults to 45° and is set in the keymap
    file; the float is turned into integer 1/256 turns here, at the
    boundary, and never travels further. `isCrankDocked` is reported, and
    when a required action is on the crank and the crank is docked, the
    engine shows a "use the crank" hint.
    **Either order with 1121 (parked):** if 1121 is done, the crank's
    change in a frame is also the one-axis `Crank` input the profile
    lists, in the same integer 1/256 turns, which 1121's `Axis` actions
    bind like a stick's, with a fake-SDK test; if not, nothing to do
    (1121 does it).
  - **The system menu.** Optional actions bound to `Button::SystemMenu0`
    to `SystemMenu7` (3002; `Button::system_menu(n)` gives slot n)
    become `addMenuItem` entries, slot n as item n, up to the profile's
    `system_menu_slots`. A choice reaches the game as a press of that
    button (pressed and let go) through `poll_input`, never through
    `App::game_mut` (`Game::act`), so it stays in a session record
    (2518).
  - **The display.** `present` writes the `OneBitFrame` (3006) into
    `getFrame()`, and marks only `changed_rows` with `markUpdatedRows`.
    **From 3003 and 3006 (done):** the boundary has no 1-bit frame yet.
    `Show::Game` carries the colour `GlyphBuffer` and `Look { font,
    images }`. `to_one_bit` (`shuyi_ui::frame::onebit`, ADR-0026) also
    needs a `PatternMap` (`PatternMap::new` over the palette and the
    game's `palette@onebit.ron`), a one-bit `Font` (the atlas and its
    one-bit bits) and one-bit pictures (`Images`). This ticket decides
    how they reach the backend: for example `Look` gains an optional
    one-bit part that the game's load fills from its pack, or `App`
    makes the `OneBitFrame` when `DisplayInfo::color` is `OneBit`. It
    records the choice in an ADR (the `write-adr` skill), writes a
    `Breaking:` line if the public API changes, and fixes the words
    that still say 3006 does it: the docs of `Show::Game` and
    `DisplayInfo::color` (`crates/platform/src/platform.rs`) and
    "1-bit frames, ticket 3006" in `docs/platform-backends.md`.
  - **Storage.** Save slots and settings through the SDK's file API, in
    the game's data folder, written in the order `FileStorage` uses
    after 3045: the temporary file written and flushed, the last good
    copy kept as `.bak`, then the rename.
  - **Memory.** The global allocator goes through the SDK's `realloc`.
    **Either order with 3064:** if 3064 is done, the debug build wraps
    this allocator in its `CountingAlloc` (`shuyi-alloc-count`, without
    `std`) and implements `Platform::memory` from it; if not, 3064 does
    it (its own clause says so). **Either order with 2368 (parked):**
    if 2368 is done, that reading also gives the `playdate` profile's
    `peak_heap_bytes` as its budget, as 2368 says; if not, 2368 does it.
  - **The clock, for the perf traces** (3063, ADR-0046): the debug
    build implements `Platform::clock` with the SDK's high-resolution
    timer, as `PerfClock::Real` of whole microseconds (check its name
    and unit in the SDK's docs, and never reset it in the frame loop).
  - **Logs.** After each frame, the lines of `App::take_new_log()`
    (3040) go to the SDK's `system->logToConsole`, one per line, as
    `f<frame> <level> <category>: <text>`. The panic handler (the
    game's `src/pd/panic.rs`, lifted here) writes the panic's message
    there too.
  - **Panics.** If 3046 is done, the panic handler also writes a
    `shuyi_platform::crash::CrashReport` (message and place; no
    backtrace and no record) through the backend's storage before it
    stops, so the next start offers it as `Setup::last_crash`. If not,
    write a ticket for it then (the `write-ticket` skill).
  - **Suspend.** The SDK's lock and pause events become `Suspend` and
    `Resume`.
  - **The facts** (3053): `facts()` is `PlatformFacts::from_profile` of
    `targets/playdate.ron`: quit by the system (the Home item of the
    system menu), not windowed, `Keyboard::Absent`, no pointer. A
    game's Quit item then hides itself with `ctx.platform.can_quit()`,
    and the options screen has no Fullscreen row.
    The game starts with `ctx.device` set to
    `Device::Pad(PadKind::Playdate)` (3053's Out leaves this here), so
    help text names A and B from the first frame, before any press; a
    host test with the fake SDK reads the first frame's help.
  - **The system's language.** Either order with 2349: if 2349 is done,
    `facts()` also fills `PlatformFacts::languages` (where 2349 and 3053
    put the system's languages, whichever lands second) from the SDK's
    `system->getLanguage()` (English gives `en`, Japanese `ja`,
    anything else none); if not, 2349 does it.
- **The catalog and `check-modules`.** **From 2003 (done):** the check
  counts every crate under `crates/` other than the default six,
  `shuyi` and `xtask` as a module, and fails on one with no
  `[package.metadata.shuyi]`; `shuyi modules` and `shuyi add` offer
  every crate that has it. A backend answers no concern and is no
  feature of `shuyi`, so give `shuyi-playdate` no metadata, and teach
  the check a backend: a crate whose row is in `docs/modules.md`'s
  *Platform backends* table needs its README and that row, not
  metadata, with a fixture. Set that row (2003 wrote it `planned`) to
  `ready`. **Either order with 3021 and 3024 (both parked):** if
  either is done, it already taught the check this; keep it. If not,
  nothing more to do: their rows are in the same table.
- **Assets.** The game's baked pack for `playdate` (3007) is placed in
  the `.pdx`, and is read through the SDK's file API. Check what the SDK's
  `pdc` does with files it doesn't know, and record it. **Either order
  with 2906** (research: asset memory): if 2906 is done, read the pack
  as its ADR says (whole, or file by file through the SDK's file API);
  if not, read it whole into the heap, and a build ticket of 2906's
  changes that if its numbers call for it.
- **`shuyi export playdate`** (an `Exporter`, 3004):
  1. it runs `check --target playdate`;
  2. it builds for `thumbv7em-none-eabihf` (with `cargo-playdate`, or
     the steps it does, written out; check its license), through 3004's
     path remapping. As 3004's ADR says of every package, the exporter
     then runs 3004's path search over the ELF, and its library audit
     against a `playdate` list in `crates/cli/export/libraries.ron`
     (expected empty: a `no_std` game links everything statically);
  3. it writes `pdxinfo` from `shuyi.ron`'s `app` part (3048): `name`,
     `author` (the publisher), `bundleID` (the id), `version`,
     `buildNumber` (the build) and `imagePath`. The launcher's icon and
     card come from 3007's 1-bit bake of `app.icon`, or from files the
     game names in two optional fields this ticket adds to `AppPart`
     (`playdate_icon`, `playdate_card`), checked like 3048's other
     fields;
  4. it bakes the pack;
  5. it writes `NOTICES.txt` beside the pack, so the `.pdx` carries it,
     with 3039's `notices` module (the crates for
     `thumbv7em-none-eabihf`, the game's license texts and credits), as
     every 3004 package does;
  6. it runs the SDK's `pdc` (a debug build first drops the ELF's
     per-function unwind sections, as the *From the game's first
     engine-friction sweep* list above says);
  7. it reports the `.pdx` size against the profile's `package_bytes`.

  A missing SDK gives the setup steps above.

  **Either order with 3050:** if 3050 is done, the `playdate` exporter
  fills `Exporter::host()`, `container()` and `ci()`. It is
  `check-only`, with the SDK's license as `why`, or `build` on the host
  CI can use if the license lets CI download the SDK. If not, 3050
  fills them.

  **Either order with 3059** (one-language builds): if 3059 is done,
  `shuyi export playdate` takes its `--lang <tag>`. It bakes the
  `playdate` pack for that language, as `bake --target playdate` with
  `--lang` does, and names the `.pdx` per language as 3059's exports
  do. `pdxinfo` is per language too: `bundleID` is the id with the tag
  after a dot, in lower case (`com.example.game.ja`), and `name` is the
  name with the tag in brackets (`Game (ja)`). Check the SDK's docs for
  the characters a bundle id may hold, and record it. Builds for two
  languages then sit side by side on one device, each with its saves
  in its own data folder. A unit test reads the `pdxinfo` of a
  `--lang ja` export, and `docs/targets/playdate.md` says how to make
  one build per language. If not, nothing to do (3059 does it).
- **A Simulator build**, if it builds on Windows with the GNU toolchain.
  If it needs MSVC, it is built only in CI, or skipped and recorded (never
  require MSVC locally, `AGENTS.md`).
- **Run in the Simulator.** `shuyi export playdate --run` (3004's run
  step) opens the `.pdx` in the SDK's Simulator (`PlaydateSimulator`
  under `$PLAYDATE_SDK_PATH/bin`; check its command line in the SDK's
  docs on the day) when the Simulator build works. It prints the
  Simulator's console output if the SDK gives it to a calling program;
  record whether it does on Windows. Without a Simulator build it says
  why. Running on the device is 3011's.
- **One example on the Playdate.** The smallest example, `hello-grid`
  (the game's 1118 example game), already has its `targets."playdate"`
  button table in `files/keymap.ron` (3002) and its pattern map,
  `files/palette@onebit.ron` (3006). It gets a frame that fits
  400 × 240 (its frame is 384 × 384 today) and a `playdate` entry in
  `shuyi.ron`'s `targets`, the README's note on why the Playdate isn't
  listed goes, and it exports. **Either order with 2371:** if 2371 is
  done, the example's Options screen uses its one-bit painters (the
  one-bit look of the engine's screens and widgets); if not, 2371 adds
  that. **Either order with 2736 (parked):** if the match-3 it plans
  (`examples/match-three`, built by its part 2 ticket) is done,
  `shuyi export playdate` builds it too and its size is in the report;
  if not, nothing to do (its part 2 builds it with this exporter).
- **Docs.** `docs/targets/playdate.md` gets how to build, sideload
  (Panic's help page *Sideloading*) and what is known not to work yet.
- **The agent eval.** If 2524 is done: add `playdate` to the `targets`
  of the turn-based tasks in `evals/tasks/` and drop their `unmeasured`
  Playdate entry, if their references pass `cargo shuyi check --target
  playdate`. If a reference doesn't, keep the entry and write a ticket
  for what stops it. If 2524 isn't done, nothing to do: 2524's
  either-order clause does it. The end goal names a Playdate build.
- **Either order with 2739** (parked, the snake example): if 2739 is
  done, `shuyi export playdate` also builds `examples/snake`, whose
  frame fits the screen and whose `shuyi.ron` lists `playdate`, and the
  PR says whether it runs in the Simulator; and `evals/tasks/snake.ron`
  gets `playdate` in its `targets` and loses its `unmeasured` Playdate
  entry, if its reference passes `cargo shuyi check --target playdate`.
  If not, nothing to do (2739 does both).

**Out (do not do):**
- **Audio**: 3029 (written early by the game's friction sweep, game#1153,
  because the game's front end has working code to lift).
- **The game's own stack**: 3028.
- the crank as an analog axis while parked 1121 is open (see its clause
  under **Input**);
- MIDI sequences through the SDK's own synth (ADR-0014 §3): 3057
  (parked), after 2206 and 3029;
- the accelerometer;
- HTTP through the SDK's networking: a parked follow-up that 2601
  (parked) writes when it is unparked, blocked by this ticket;
- measuring on the device (3011);
- writing session records on the device: `App` records on any backend
  (2518), but only `shuyi-app` writes them (2519). Write a follow-up if
  a Playdate playtest needs one;
- Panic's Catalog submission (the game's decision, game 1025).

## Implementation steps

1. Read the current Playdate SDK C API docs, and the `playdate` crates'
   docs and licenses. Note any fact in ADR-0008 that turned out wrong
   and fix it in the PR. If a fact changes the decision, write a new ADR.
2. Write the backend, method by method, with a fake SDK behind a small
   internal trait. That fake is what the host tests use.
3. Write the exporter.
4. Export the example, run it in the Simulator with `--run` if one
   builds, and fix what's found.

## Acceptance criteria

- [ ] `shuyi export playdate` on the example produces a `.pdx`, and its
      size is in the report.
- [ ] The `.pdx`'s ELF passes 3004's path search (none of the build
      machine's paths) and its library audit against `playdate`'s list.
- [ ] Host tests with the fake SDK:
  - [ ] a crank turn of 90° gives two `CrankForward`s;
  - [ ] only changed rows are marked;
  - [ ] the first frame's help names the Playdate's buttons
        (`ctx.device` starts on `Device::Pad(PadKind::Playdate)`);
  - [ ] the storage passes `shuyi_ui::storage::power_cut_suite` (3045)
        over the fake SDK's files.
- [ ] The example opens and plays in the Playdate Simulator, if it
      builds (screenshot in the PR), or the reason it doesn't is
      recorded.
- [ ] `shuyi export playdate --run` opens the example in the Simulator,
      or the Completion notes say why it can't.
- [ ] `cargo deny check` passes with `shuyi-playdate`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the input mapping, crank detents, row marking and storage, all
  against the fake SDK.
- Integration: the export in CI, if the SDK's license allows CI to
  download it. Otherwise, on Nick's machine, and say so in the PR.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
