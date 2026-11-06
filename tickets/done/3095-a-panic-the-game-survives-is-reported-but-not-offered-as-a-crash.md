---
id: "3095"
title: "A panic the game survives is reported, but not offered to the game as a crash"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["3046"]
nick_input: none
completed: 2026-11-05
---

# 3095 — A panic the game survives is not offered as a crash

## Context

3046 installs a panic hook in `shuyi-app` (`crates/app/src/crash.rs`,
[ADR-0079](../../docs/adr/0079-crash-reports-one-panic-hook-per-backend-writing-through-the-games-storage-offered-once-and-the-games-record-added-on-native.md))
that writes a `CrashReport` for every panic not run inside `crash::quiet`,
on any thread, so a panic on the music worker is reported too. At the next
start `App::load` offers the newest report to the game
(`Setup::last_crash`, `shuyi_platform::crash::offer_last_crash`).

CI's `render` job (3046's crash step) showed a case where that misleads. On
a Linux machine with no sound card, quad-snd 0.2.8's own audio threads
panic ("Can't open PCM device", `quad-snd/src/alsa_snd.rs:30`) as the game
starts. The game runs on, silent, but each start writes reports, and the
next start tells the game that it crashed, so a game that shows a notice
shows a false one every time.

A report of such a panic is still worth keeping: it explains why the game
had no sound. What is wrong is offering it as a crash.

## Nick input

None.

## Scope

**In:**
- `CrashReport` gains `fatal: bool`, read as `true` when the text has no
  such field (a `#[serde(default = …)]` function), since an older report
  was offered as a crash. It is `true` when the panic ended the game: on native, the panic
  `run`'s `catch_unwind` caught (set it in `crash::add_record`'s rewrite,
  or a sibling call before it), and a panic on the main thread outside
  that catch; on the web, every panic on the main thread (it aborts).
- `offer_last_crash` offers the newest **fatal** report newer than the one
  `crash-offered` names, and skips the rest.
- `docs/crash-reports.md`: the field, and that a panic the game survives
  (a sound library's thread without a sound device) is kept but not
  offered.
- A new ADR that extends ADR-0079 §6 (accepted ADRs aren't edited).

**Out (do not do):**
- Stopping quad-snd from panicking without a sound device: that is the
  sound replacement's (ADR-0068), not this ticket's.
- Anything else of ADR-0079.

## Implementation steps

1. Read `crates/platform/src/crash.rs`, `crates/app/src/crash.rs` and
   `frame_or_crash` in `crates/app/src/lib.rs`.
2. Add `fatal`, set it where the process ends, and filter the offer.
3. Tests, docs and the ADR.

## Acceptance criteria

- [x] A storage holding a fatal report and a newer report that isn't
      offers the fatal one, once.
- [x] `crates/app/tests/crash_hook.rs`: a panic on another thread writes
      a report with `fatal: false`; the panic `run` catches is `fatal:
      true` (through `add_record` or its sibling).
- [x] CI's `render` step checks `fatal: true` in the report of the panic
      on purpose.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-platform`): the offer skips reports that aren't fatal; an
  old report without the field reads.
- Integration (`shuyi-app`): as above.

## Completion notes

- `shuyi_platform::crash::CrashReport` has `fatal: bool`, read as `true`
  from a report without it. `offer_last_crash` offers the newest fatal
  report newer than `crash-offered`'s, skips the rest (not fatal, or
  unreadable), and marks the newest report as offered either way.
- `shuyi_app::crash::install` remembers its thread, the game's: the hook
  writes `fatal: true` for a panic there and `false` on any other thread.
  ADR-0083 extends ADR-0079 §6.
- CI's `render` step checks `fatal: true` in the report of the panic on
  purpose, and that every other report there (quad-snd's threads) isn't.
- Docs: `docs/crash-reports.md`, `docs/platform-backends.md` (*Panics*),
  and the book's *When a player's game crashes*.

Deviations:
- The fatal flag comes from the thread, not from `add_record`'s rewrite:
  on the game's thread a panic not in `quiet` always ends the game (the
  frame's catch exits 101, one outside it ends the process, the web
  aborts), so one rule covers all three. `add_record` keeps the hook's
  `true`; the test checks it after the rewrite.
- `audio.rs`'s sound-effect decode caught quad-snd's panic on the game's
  thread without `crash::quiet`, so a broken effect wrote a report; under
  the new rule it would have been offered as a crash. It now runs in
  `quiet`, as the music's decode already did.
- 3076 (the Playdate's report) got a **From 3095** line: only fatal
  reports are offered, and `CrashReport::new` sets it.

No follow-up tickets.

No break: `CrashReport` is `#[non_exhaustive]`, so the new field is an
addition.
