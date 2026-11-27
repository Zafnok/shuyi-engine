---
id: "3337"
title: A native crash leaves a crash report, with a minidump on Windows, and an ADR decides the release builds' Windows toolchain
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3337 — A native crash leaves a crash report, with a minidump on Windows, and an ADR decides the release builds' Windows toolchain

## Context

Found by the engine blind-spot audit of 2026-11-23 (no friction review:
a missing feature). Today a crash that is not a Rust panic writes
nothing.

What there is (read 2026-11-23):

- **ADR-0079** (3046, done): one panic hook per backend. It writes a
  `CrashReport` (`crates/platform/src/crash.rs`) through the game's
  storage, and `App::load` offers the newest one once
  (`Setup::last_crash`, `offer_last_crash`). Its *Consequences* say:
  stack overflows, aborts, running out of memory and faults in native
  code "end the process without a panic hook: they leave no report. No
  minidumps." `docs/crash-reports.md` (*What makes no report*) says the
  same to game makers.
- **2598** (done): `CrashReport::frames` holds the panicking thread's
  stack as `CodeFrame { offset }`, taken by the system's unwinder
  (`crates/app/src/crash/frames.rs`: `RtlCaptureStackBackTrace` on
  Windows through `winapi`, glibc's `backtrace()` on Linux, the
  executable's range read once from `/proc/self/maps` by
  `note_executable`). `capture` returns a `Vec`, so it allocates.
- **3095** (done, ADR-0083): `CrashReport::fatal`; only fatal reports
  are offered.
- **3041** (done): the log file. `crates/app/src/log_sink.rs` writes
  `latest.log` in the storage's `logs` folder and moves the last three
  runs' files to `previous-1.log` … `previous-3.log` when it opens.
- **The storage's folder** on a desktop is `StorageAt::dir()`
  (`crates/app/src/storage/mod.rs`); each key is a `<key>.ron` text file
  (`FileStorage::path`, `storage/native.rs`). A minidump is binary, so it
  can't be a storage key; it goes in a folder beside the keys, as the
  logs do.
- **2520** (done): `shuyi crash` reads a report
  (`crates/cli/src/contract/crash.rs`). **3441** (done) names its
  frames from the build's debug file with `addr2line`, which reads
  DWARF only. Its *Out* leaves Windows PDB files out "until one asks".
- **The Windows toolchain.** Nick's machine builds with GNU and has no
  MSVC (`AGENTS.md`, *Environment*). `shuyi export windows` builds
  `x86_64-pc-windows-gnu` only (`crates/cli/src/export/desktop.rs`,
  line 29), and CI's `export.yml` sets that target up with MSYS2's
  mingw-w64 GCC. CI's `test` job runs on `windows-latest` with rustup's
  default host there, MSVC, so the workspace already compiles and tests
  on MSVC. A GNU build's debug info is DWARF inside the `.exe` (or its
  `.debug` file, 3004), which WinDbg, Visual Studio and services that
  read Windows minidumps can't read; they want a PDB. Steamworks
  (1144, 3037: neither has confirmed the GNU link), Epic Online
  Services (2920: the same question) and Microsoft's GDK (ADR-0008:
  "Microsoft's GDK needs MSVC"; 3016) all lean on MSVC. No ADR says
  which toolchain a release build uses, or why.
- **The crates for it** (read in the local registry on 2026-11-23):
  `windows-sys` 0.61 (MIT OR Apache-2.0, already in `Cargo.lock` at
  0.61.2) declares `MiniDumpWriteDump` (`dbghelp.dll`, feature
  `Win32_System_Diagnostics_Debug`), `SetUnhandledExceptionFilter`
  (`Win32_System_Diagnostics_Debug`) and `SetThreadStackGuarantee`, and
  links them through `windows-link`, so no import library is needed on
  GNU. `winapi` 0.3.9, which `shuyi-app` uses today, has the filter but
  no `MiniDumpWriteDump`. On Linux and macOS, `sigaction` and
  `sigaltstack` come from `libc` (MIT OR Apache-2.0, in `Cargo.lock`).
  No minidump crate is needed: an in-process Linux minidump needs a
  second process to `ptrace` the crashed one, which this ticket leaves
  out.

Other engines: Unreal's Crash Reporter Client, Unity's Diagnostics and
Steam's own crash collection all work from minidumps. Godot writes a
backtrace for a native fault in release builds. On desktop the crashes
players hit most are native: GPU drivers, audio devices, the window
system. Without this, a store release's worst bugs are invisible to
the game and to `shuyi crash`.

**The game:** it turns reports on with
`CrashReports::new(APP.version).with_record_in_release()`
(`crates/app/src/main.rs`; `gh search code --repo
Zafnok/visions-of-shuyi 'CrashReports'`, pin `50472bbc`). It gets
native-fault reports with no change. `gh search code --repo
Zafnok/visions-of-shuyi 'minidump'` finds nothing, so minidumps are its
choice to turn on.

## Nick input

None. The toolchain ADR is technical and costs nothing (CI's
`windows-latest` runners have MSVC). It never makes Nick's machine need
MSVC.

## Scope

**In:**
- **A native fault becomes a report.** `shuyi_app::crash::install`
  also installs a native-fault handler, on desktop only (Windows, Linux,
  macOS; nothing on the web). It is on whenever crash reports are on
  (`CrashReports::off()` installs neither). It catches:
  - Windows: an access violation, an illegal instruction, a division by
    zero, an in-page error and a stack overflow, through
    `SetUnhandledExceptionFilter`. The filter returns
    `EXCEPTION_CONTINUE_SEARCH`, so Windows Error Reporting still sees
    the crash;
  - Linux and macOS: `SIGSEGV`, `SIGBUS`, `SIGILL`, `SIGFPE` and
    `SIGABRT` (so an abort, a failed allocation and `panic = "abort"`
    are caught too), through `sigaction` with `SA_ONSTACK`. The handler
    keeps each action it replaced, and, when done, puts it back and
    raises the signal again, so std's "has overflowed its stack"
    line and the system's core dump stay.
- **What the handler may do.** A fault can come at any moment, with
  locks held and the heap broken, so the handler never allocates,
  never locks and never formats with `core::fmt`:
  - Linux and macOS: it writes a short ASCII **fault note** with
    `open`, `write` and `close` to a path built at `install` (in a
    fixed buffer), with integers written by hand. The note holds: the
    signal, the fault address (`si_addr`), the time
    (`clock_gettime(CLOCK_REALTIME)`), whether it was the game's thread,
    the frame (`set_frame`'s value, kept in an `AtomicU32`), the top
    screen (kept as an atomic pointer and length beside today's
    `Mutex`), and the frames as offsets, through a non-allocating
    `frames::capture_into(&mut [CodeFrame; FRAMES]) -> usize` (Linux
    only, `backtrace()` called once at `install` so its library is
    loaded before a fault). macOS gets no frames, as for a panic.
  - Windows: a **dump thread** started at `install` waits on an event.
    The filter stores the exception record's pointer and the faulting
    thread's id in statics, sets the event and waits (5 seconds at
    most). The dump thread writes the fault note (as above, with the
    exception code and address) and, when asked, the minidump. A stack
    overflow leaves little stack, so the filter does nothing else. In
    every other case it captures the frames first, on its own stack,
    with `capture_into`. `install` calls `SetThreadStackGuarantee` on
    the game's thread so the filter has room to run after an overflow.
- **The minidump (Windows).** `CrashReports::with_minidumps()`
  (a `const fn`, off by default) makes the dump thread call
  `MiniDumpWriteDump` with `MiniDumpNormal` (the threads' stacks and
  the modules, no heap) and the exception information, into
  `crash-dumps/native-<when>.dmp` in the storage's folder. It is off by
  default because a stack holds whatever the game had in it, which
  ADR-0079 §5 keeps out of reports. The newest `keep` dumps are kept,
  as reports are. Linux and macOS write no minidump.
- **The note becomes a report at the next start.** `install` looks for
  fault notes (`crash-dumps/native-<when>.note`) before it writes
  anything. Each one becomes a `CrashReport` written through the
  storage at `crash_key(when)`, then the note is deleted. So
  `offer_last_crash` offers it once, like a panic's, and 3047's
  sending would treat it like any other report. The report has:
  - `message`: the fault in words, such as `native fault: access
    violation writing 0x0000000000000010` or `native fault: SIGSEGV at
    0x10`, and `fatal: true`;
  - `frames` from the note; `frame`, `top_screen`, `engine`, `game` and
    `target` as the hook fills them (the versions are written into the
    note at `install`);
  - `log`: the last `LOG_LINES` lines of the faulted run's log file
    (`latest.log` or `previous-1.log`, whichever was that run's, as
    `log_sink`'s rotation leaves them), empty when there is none;
  - two new fields, `#[serde(default)]`: `fault: Option<NativeFault>`
    and `dump: Option<String>` (the minidump's file name, if one was
    written). `NativeFault { kind: String, code: u32, address:
    Option<u64> }` is `#[non_exhaustive]` with a constructor (ADR-0039).
    `CrashReport` is `#[non_exhaustive]` already, so the new fields are
    additive and need no `Breaking:` line. `REPORT_VERSION` stays 1:
    an older report reads with both fields `None`.
  - A note that doesn't parse (cut short by the crash) still gives a
    report, with what parsed and `native fault: the note was
    incomplete` as its message. The reader keeps ADR-0007 §5: a size
    limit (64 KiB), errors, never a panic.
- **On purpose, for the tests.** `SHUYI_FAULT_AT_FRAME=<n>[,stack|abort]`,
  read where `SHUYI_PANIC_AT_FRAME` is read and in the same builds,
  faults at frame `n`: a write through a null pointer, unbounded
  recursion, or `std::process::abort`. Its `unsafe` sits in
  `crates/app/src/crash/fault.rs` with the reason (game ADR-0034).
- **An ADR on native faults**, the next free number, `Amends:
  ADR-0079` (its *Consequences* line "No minidumps"). ADR-0079 gains
  only an "amended by" line in its status, as the `write-adr` skill
  says. It records the handler's rules above, what still leaves no
  report, and why there is no out-of-process reporter and no Linux
  minidump.
- **An ADR on the release builds' Windows toolchain**, the next free
  number after it. It decides whether a game's release build for
  Windows stays `x86_64-pc-windows-gnu`, or may be built
  `x86_64-pc-windows-msvc` on CI's runners, or either at the game's
  choice. It weighs, with what this ticket found:
  - what reads each one's crash: 3441's `addr2line` reads DWARF only
    (a PDB would need another reader, such as the `pdb` crate), and a
    minidump of a GNU build has no symbols WinDbg or Visual Studio
    reads;
  - what needs MSVC: Steamworks (1144, 3037), Epic Online Services
    (2920) and the GDK (3016), with what each of those tickets has
    found when this one is worked, or "not known yet";
  - that a local `shuyi export windows` on Nick's machine is GNU and
    must stay so, so an MSVC release would differ from the local build;
  - what CI already proves: the `test` job builds and tests the
    workspace on MSVC.
  It names the follow-up tickets its answer needs (an exporter option,
  a PDB reader in `shuyi crash`), and this ticket writes them in its PR
  (After MVP, blocked by this one). If the answer is "stay GNU", none.
- **Docs:** `docs/crash-reports.md` replaces *What makes no report*
  with what a native fault writes, what still writes nothing, and
  `with_minidumps`, with what a dump can hold. `docs/adr/README.md`
  gains both ADRs' rows.
- **From 2520 (done):** `CrashOutput` (`crates/cli/src/contract/crash.rs`,
  `#[non_exhaustive]`) also carries the report's `fault` and `dump`,
  `crash::text` shows them, and the `crash` schemas are regenerated
  with their line in `versions.json` (`docs/contract.md`, *The
  schemas*); added answer fields keep `VERSION` (ADR-0150).

**Out (do not do):**
- Sending a report or a minidump anywhere: 3047 (parked).
- A minidump on Linux or macOS, and a separate reporter process
  (Unreal's): a follow-up if a game asks.
- An abort on Windows: Rust's `abort` uses `__fastfail`, which no
  in-process handler sees. The docs say Windows Error Reporting keeps
  those.
- Switching `shuyi export` to MSVC, or a PDB reader: the toolchain ADR's
  follow-ups, if it picks MSVC.
- The Steamworks and EOS link checks themselves: 1144, 3037 and 2920
  (all parked) do them.
- The Playdate's, the GBA's and the DS's crashes (3076 and their
  backends), and the web.
- Any change to the game.

**Estimated size:** about 1,400 lines (code 650, tests 450, docs 300).

**Split:** past 1,000 lines, split off the Linux and macOS signal
handler (step 4) into a new ticket, blocked by this one.

## Implementation steps

1. Read ADR-0079, ADR-0083, `crates/app/src/crash.rs`,
   `crates/app/src/crash/frames.rs`, `crates/app/src/log_sink.rs`,
   `crates/app/src/storage/mod.rs` and `crates/platform/src/crash.rs`,
   and `docs/crash-reports.md`.
2. In `shuyi_platform::crash`: `NativeFault`, the two fields, the fault
   note's text and its reader (`FaultNote::parse`, never a panic), and
   `FaultNote::into_report`.
3. In `crates/app/src/crash/frames.rs`: `capture_into`, with `capture`
   built on it.
4. `crates/app/src/crash/native.rs` with an `os` module per system, as
   `frames.rs` has: the Unix handler (`libc`), the Windows filter and
   dump thread (`windows-sys` 0.61, features
   `Win32_System_Diagnostics_Debug`, `Win32_System_Threading` and
   `Win32_Foundation`; check `cargo deny check` and that `Cargo.lock`
   keeps one 0.61), and the note's writer over a fixed buffer.
5. `install`: turn the notes into reports, keep the newest `keep`
   dumps, then install the handler. `CrashReports::with_minidumps`.
6. `crates/app/src/crash/fault.rs`: `SHUYI_FAULT_AT_FRAME`.
7. Write both ADRs, then the docs.
8. On the machine at hand (Windows GNU): run hello-grid with
   `SHUYI_FAULT_AT_FRAME=3` and with `3,stack`, with minidumps on, start
   it again, and put the report's text and the dump's size in the PR
   description.
9. Run the gates.

## Acceptance criteria

- [ ] A `shuyi-app` test starts its own test binary as a child with
      `SHUYI_FAULT_AT_FRAME` set for a null write: the child dies by
      the fault, not exit code 101, and leaves a fault note. On Linux
      the child's exit is by `SIGSEGV`.
- [ ] Reading that note at the next `install` writes one fatal report
      whose `fault` names the fault, whose message names it in words,
      whose `frame` is the frame it faulted at, and deletes the note.
      `offer_last_crash` offers it once.
- [ ] On Windows and Linux the report's `frames` is not empty and its
      first offsets fall inside the executable.
- [ ] The same with `,stack` writes a report on Windows and Linux.
      On Linux std's "has overflowed its stack" line is still printed.
- [ ] With `,abort` on Linux and macOS a report is written; on Windows
      the test expects none, as the docs say.
- [ ] On Windows, with `with_minidumps()`, `crash-dumps/native-<when>.dmp`
      exists, starts with `MDMP`, and the report's `dump` names it;
      without it, no dump is written.
- [ ] A note cut short at each byte reads as a report and never panics.
- [ ] A report written before this ticket reads with `fault` and `dump`
      `None`.
- [ ] `CrashReports::off()` installs no handler: the child of the first
      case leaves no note.
- [ ] The native-fault ADR amends ADR-0079, whose status line says so.
- [ ] The toolchain ADR states its choice and the facts above, and its
      follow-up tickets, if any, are in `tickets/open/`.
- [ ] `cargo deny check` passes with no change to `deny.toml`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the note's writer and reader round-trip each field; integers
  written by hand match `format!`; a cut note; the message for each
  fault kind; the newest-`keep` sweep of dumps.
- Property: any bytes up to 64 KiB given to `FaultNote::parse` return
  without a panic.
- Integration: the child-process cases above, compiled on Windows,
  Linux and macOS (CI's `test` matrix), each in its own process.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
