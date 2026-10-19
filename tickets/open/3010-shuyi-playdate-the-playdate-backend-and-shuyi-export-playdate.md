---
id: "3010"
title: "shuyi-playdate: the Playdate backend (update callback, d-pad, A, B and crank, 1-bit display with changed rows, storage) and shuyi export playdate"
type: feature
milestone: After MVP
model: opus-5.5
effort: xhigh
status: todo
blocked_by: ["3002", "3003", "3004", "3005", "3006", "2204"]
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
- the controls check guards the inputs (3002).

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
    (3002) become `addMenuItem` entries.
  - **The display.** `present` writes the `OneBitFrame` (3006) into
    `getFrame()`, and marks only `changed_rows` with `markUpdatedRows`.
  - **Storage.** Save slots and settings through the SDK's file API, in
    the game's data folder, written atomically (write a temporary file,
    then rename).
  - **Memory.** The global allocator goes through the SDK's `realloc`.
  - **Suspend.** The SDK's lock and pause events become `Suspend` and
    `Resume`.
- **Assets.** The game's baked pack for `playdate` (3007) is placed in
  the `.pdx`, and is read through the SDK's file API. Check what the SDK's
  `pdc` does with files it doesn't know, and record it.
- **`shuyi export playdate`** (an `Exporter`, 3004):
  1. it runs `check --target playdate`;
  2. it builds for `thumbv7em-none-eabihf` (with `cargo-playdate`, or
     the steps it does, written out; check its license);
  3. it writes `pdxinfo` from the game manifest;
  4. it bakes the pack;
  5. it runs the SDK's `pdc`;
  6. it reports the `.pdx` size against the profile's `package_bytes`.

  A missing SDK gives the setup steps above.
- **A Simulator build**, if it builds on Windows with the GNU toolchain.
  If it needs MSVC, it is built only in CI, or skipped and recorded (never
  require MSVC locally, `AGENTS.md`).
- **One example on the Playdate.** The smallest example (the game's
  1118 example game) gets a `playdate` layout, a pattern map and a frame
  that fits, and it exports.
- **Docs.** `docs/targets/playdate.md` gets how to build, sideload
  (Panic's help page *Sideloading*) and what is known not to work yet.

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
- Panic's Catalog submission (the game's decision, game 1025).

## Implementation steps

1. Read the current Playdate SDK C API docs, and the `playdate` crates'
   docs and licenses. Note any fact in ADR-0008 that turned out wrong
   and fix it in the PR. If a fact changes the decision, write a new ADR.
2. Write the backend, method by method, with a fake SDK behind a small
   internal trait. That fake is what the host tests use.
3. Write the exporter.
4. Export the example, run it in the Simulator if one builds, and fix
   what's found.

## Acceptance criteria

- [ ] `shuyi export playdate` on the example produces a `.pdx`, and its
      size is in the report.
- [ ] Host tests with the fake SDK:
  - [ ] a crank turn of 90° gives two `CrankForward`s;
  - [ ] only changed rows are marked;
  - [ ] a save survives a simulated crash mid-write.
- [ ] The example opens and plays in the Playdate Simulator, if it
      builds (screenshot in the PR), or the reason it doesn't is
      recorded.
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
