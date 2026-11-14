---
id: "2598"
title: "A crash report's frames as offsets into the executable, so a release's debug file names them"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-14
---

# 2598 — A crash report's frames as offsets into the executable

## Context

- **The game ticket:** game#0918
  (<https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0918-a-players-crash-report-reads-with-the-releases-debug-file.md>).
  The game keeps each release's `<package>.debug` (3004's `shuyi export
  --release`; game ticket 0917 keeps it on a private release), but a
  player's crash report holds nothing it can read.
- **The job:** a crash report from a player's stripped release build
  says where in the program each frame of the panicking thread was, so
  `addr2line` over that release's debug file names the functions, files
  and lines.
- **No friction review:** a missing feature.
- **The generic shape:** `CrashReport` (3046, `shuyi_platform::crash`)
  gains `frames: Vec<CodeFrame>`, `CodeFrame { offset: Option<u64> }`:
  each frame's return address minus the executable's load address, or
  `None` for a frame in another module (a system library). The hook in
  `crates/app/src/crash.rs` fills it on Windows and Linux. Today the
  report holds only `std::backtrace::Backtrace::force_capture()`'s text,
  which in a stripped build is `<unknown>` frames with no addresses; and
  the program loads at another address each run (ASLR), so an absolute
  address alone wouldn't read either.
- **Any game:** a farming game's player sends the newest
  `crash-*.ron`; the developer reads it against the debug file of the
  version it names.
- **2520** (open) planned these frames as its first part, taken with the
  `backtrace` crate, beside `shuyi crash` and `replay --crash`, and its
  **Size** clause lets the frames go to a ticket of their own. This is
  that ticket, with a cheaper capture: no new crate. `backtrace` 0.3
  brings `addr2line`, `gimli`, `object` and `rustc-demangle` into every
  game's build for a hook that never resolves a name, against the
  dependency budget (2080). The system's own unwinder does the job:
  - **Windows:** `RtlCaptureStackBackTrace` (kernel32) walks the stack
    by the `.pdata` unwind tables, which MSVC and the GNU toolchain both
    write and stripping keeps. `GetModuleHandleW(null)` is the
    executable's base, and `GetModuleHandleExW` with
    `GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS` says which module a frame
    is in. All three are in `winapi` 0.3.9 (features `winnt` and
    `libloaderapi`), already a Windows dependency of `shuyi-app`.
  - **Linux (glibc, the Steam Deck too):** glibc's `backtrace()`
    (`execinfo.h`) walks the stack by `.eh_frame`, which stripping keeps.
    The executable's mappings are the lines of `/proc/self/maps` whose
    path is `std::env::current_exe()`, read once by `crash::install`;
    the lowest start is the load address (Rust links PIE executables,
    whose first segment's address is 0, so the offset is the address
    the debug file uses).
  - **macOS and the web:** `frames` stays empty.

## Nick input

None.

## Scope

**In:**
- `shuyi_platform::crash::CodeFrame` (ADR-0039: `#[non_exhaustive]`,
  `CodeFrame::new(offset)`), `CrashReport::frames`
  (`#[serde(default)]`; `CrashReport` is `#[non_exhaustive]`, so the
  field is additive) and `FRAMES`, the most frames kept (128). The
  report's `version` stays 1: an older report reads with no frames.
- The hook (`crates/app/src/crash.rs`) fills `frames` from a new
  `crash/frames.rs`, the module's `unsafe` allowed there only (the
  `unsafe_code = "deny"` note in `crates/app/Cargo.toml`). `install`
  finds the executable's place on Linux; the hook never reads a file.
- The `/proc/self/maps` reading is a pure function of the file's text.
- `docs/crash-reports.md`: the `frames` row, and how a report's offsets
  read with `addr2line` and the debug file (Windows adds the image
  base; each address is a return address, so 1 is taken off).
- 2520: its frames bullet becomes "From 2598 (done)", and its Size
  clause no longer moves the frames.

**Out (do not do):**
- `shuyi crash`, `--symbols`, `replay --crash`: 2520.
- Frames on macOS and the web; MSVC's PDB files.
- Resolving names inside the hook.

## Implementation steps

1. `CodeFrame`, `FRAMES` and the field in `crates/platform/src/crash.rs`,
   with a round-trip test and an old report without the field.
2. `crates/app/src/crash/frames.rs`: `capture()` per OS, the maps
   parser, `note_executable()` for `install`.
3. The hook fills `report.frames`; `crash_hook.rs` checks them.
4. Docs, 2520, gates.

## Acceptance criteria

- [x] On Windows and Linux a report's `frames` hold at least one frame
      in the executable (`crash_hook.rs`); on macOS none.
- [x] A unit test captures frames inside an `#[inline(never)]` function
      and finds one whose offset is a little past that function's own
      offset (its address minus the executable's).
- [x] The maps parser finds the executable's range in a sample text and
      skips other files' lines.
- [x] A report without `frames` reads, with none.
- [x] No new crate (`Cargo.lock` gains no package).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the maps parser; the frame inside a known function; the
  report's round trip with frames and an old one without.
- Integration: `crash_hook.rs`'s frames.

## Completion notes

- `shuyi_platform::crash::CodeFrame` (`#[non_exhaustive]`,
  `CodeFrame::new`), `FRAMES` (128) and `CrashReport::frames`
  (`#[serde(default)]`, cut to `FRAMES` by `limit`). The report's
  `version` stays 1; a report without the field reads with no frames.
- `crates/app/src/crash/frames.rs` walks the stack with the system's
  unwinder: `RtlCaptureStackBackTrace` and `GetModuleHandleExW` on
  Windows (`winapi`'s `winnt` and `libloaderapi` features, no new
  crate), glibc's `backtrace()` on Linux with the executable's range from
  `/proc/self/maps`, read by `crash::install`. Elsewhere no frames. The
  hook fills `frames` before std's backtrace text.
- Tests: the maps parser over a sample (spaces in the path, other files,
  a bad line skipped); `offset_from`; a frame captured inside an
  `#[inline(never)]` function lands just past that function's own
  offset (Windows and Linux); `crash_hook.rs` finds a frame in the
  program (none on macOS); the report's round trip with frames, the cut,
  and an old report without them. The Linux and macOS code was
  type-checked locally on its own (libudev's build script stops a
  cross build of `shuyi-app` here); CI runs it.
- `docs/crash-reports.md`: the `frames` row and *Reading the frames*,
  the `addr2line` commands. The book's *When a player's game crashes*
  says what `frames` are and links there.
- Deviation from 2520's plan for the frames: the system's unwinder, not
  the `backtrace` crate (it would bring `addr2line`, `gimli`, `object`
  and `rustc-demangle` into every game for a hook that never names a
  frame). 2520's frames bullet is now "From 2598 (done)", its Size
  clause and acceptance no longer carry the frames, and it adds no
  `backtrace` crate either.
- No break: `CrashReport` is `#[non_exhaustive]`, so the field is
  additive; `CodeFrame` and `FRAMES` are new. Searched the game for
  `CrashReport` (`gh search code --repo Zafnok/visions-of-shuyi
  'CrashReport'`): it builds one only with `CrashReport::new` and
  `to_text`, which still work.

Behaviour: shuyi-app, shuyi-platform: a crash report written on Windows or Linux holds `frames`, the stack as offsets into the executable.

Game: game#0918 moves the pin to this change.
