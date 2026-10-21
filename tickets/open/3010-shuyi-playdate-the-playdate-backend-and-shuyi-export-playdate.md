---
id: "3010"
title: "shuyi-playdate: the Playdate backend (update callback, d-pad, A, B and crank, 1-bit display with changed rows, storage) and shuyi export playdate"
type: feature
milestone: After MVP
model: opus-5.5
effort: xhigh
status: todo
blocked_by: ["3002", "3003", "3004", "3005", "3006", "2204", "3045", "3040", "3048", "3053"]
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
- the frame becomes 1-bit (3006);
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
  Pick one rule and write it in the docs.
- **Storage, memory and the panic handler** are in the game's
  `src/storage.rs` (a `no_std` copy of `shuyi-app`'s native storage),
  `src/pd/allocator.rs` and `src/pd/panic.rs`.

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
  - **Input.** `getButtonState` for the d-pad and A and B (`East` and
    `West`). The A+B chord comes from the engine's chord handling. The
    crank: `getCrankChange` becomes `CrankForward` and `CrankBack` once
    per detent. The detent defaults to 45° and is set in the keymap
    file; the float is turned into integer 1/256 turns here, at the
    boundary, and never travels further. `isCrankDocked` is reported, and
    when a required action is on the crank and the crank is docked, the
    engine shows a "use the crank" hint.
  - **The system menu.** Optional actions bound to `SystemMenu(n)`
    (3002) become `addMenuItem` entries. Their choices reach the game as
    `SystemMenu(n)` button presses through `poll_input`, never through
    `App::game_mut` (`Game::act`), so they stay in a session record
    (2518).
  - **The display.** `present` writes the `OneBitFrame` (3006) into
    `getFrame()`, and marks only `changed_rows` with `markUpdatedRows`.
  - **Storage.** Save slots and settings through the SDK's file API, in
    the game's data folder, written in the order `FileStorage` uses
    after 3045: the temporary file written and flushed, the last good
    copy kept as `.bak`, then the rename.
  - **Memory.** The global allocator goes through the SDK's `realloc`.
    **Either order with 2368 (parked):** if 2368 is done, the debug
    build wraps this allocator in 3008's counting allocator, implements
    `Platform::memory`, and gives the `playdate` profile's
    `peak_heap_bytes` as its budget, as 2368 says; if not, 2368 does it.
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
  - **The system's language.** Either order with 2349: if 2349 is done,
    `facts()` also fills `PlatformFacts::languages` (where 2349 and 3053
    put the system's languages, whichever lands second) from the SDK's
    `system->getLanguage()` (English gives `en`, Japanese `ja`,
    anything else none); if not, 2349 does it.
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
     the steps it does, written out; check its license);
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
  6. it runs the SDK's `pdc`;
  7. it reports the `.pdx` size against the profile's `package_bytes`.

  A missing SDK gives the setup steps above.

  **Either order with 3050:** if 3050 is done, the `playdate` exporter
  fills `Exporter::host()`, `container()` and `ci()`. It is
  `check-only`, with the SDK's license as `why`, or `build` on the host
  CI can use if the license lets CI download the SDK. If not, 3050
  fills them.
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
- **One example on the Playdate.** The smallest example (the game's
  1118 example game) gets a `playdate` layout, a pattern map and a frame
  that fits, and it exports.
- **Docs.** `docs/targets/playdate.md` gets how to build, sideload
  (Panic's help page *Sideloading*) and what is known not to work yet.
- **The agent eval.** If 2524 is done: add `playdate` to the `targets`
  of the turn-based tasks in `evals/tasks/` and drop their `unmeasured`
  Playdate entry, if their references pass `cargo shuyi check --target
  playdate`. If a reference doesn't, keep the entry and write a ticket
  for what stops it. The end goal names a Playdate build.

**Out (do not do):**
- **Audio**: 3029 (written early by the game's friction sweep, game#1153,
  because the game's front end has working code to lift).
- **The game's own stack**: 3028.
- the crank as an analog axis (it waits for parked 1121);
- MIDI sequences through the SDK's own synth (ADR-0014 §3). They wait
  for parked 2205 and 2206; once those are done, write the follow-up
  ticket;
- the accelerometer;
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
- [ ] Host tests with the fake SDK:
  - [ ] a crank turn of 90° gives two `CrankForward`s;
  - [ ] only changed rows are marked;
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
