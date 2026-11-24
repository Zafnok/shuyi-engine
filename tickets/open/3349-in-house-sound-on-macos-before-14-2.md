---
id: "3349"
title: "A program with the in-house sound starts on macOS older than 14.2, as one with quad-snd does"
type: bug
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3032"]
nick_input: none
completed:
---

# 3349 — The in-house sound on macOS before 14.2

## Context

3032 checked the oldest macOS a program built with `in-house-sound` starts on
(`tools/sound-parity/mac-floor.sh`, run by CI's `sound-macos` job): CPAL 0.18.2
imports `AudioHardwareDestroyProcessTap`, which the SDK's headers make macOS 14.2 and
later, and imports it strongly, not weakly. Every other audio function it imports is
older than the program's own `minos` (11.0 on Apple silicon, 10.12 on Intel, rustc's
defaults). CPAL's documentation names macOS 14.2 too; it uses process taps for
capturing a device's output, which no game of ours asks for.

So a game built with the in-house sound may not start on macOS 11 to 14.1, where the
same game with quad-snd does: with chained fixups (a `minos` of 12 and later, or a
newer linker's choice) dyld binds every non-weak import at launch and refuses a
missing one; with lazy binding it stops at the first call, which nothing here makes.
Which of the two a package gets is not yet measured. Shuyi names no macOS floor
(`docs/targets/desktop.md`), and the ticket forbade raising one silently
([ADR-0142](../../docs/adr/0142-in-house-sound-on-linux-and-macos-quad-snds-alsa-bindings-are-alsa-syss.md)).
3084 can't make the in-house sound macOS's default until this is settled.

## Nick input

None, unless the fix below can't be made: then whether macOS 14.2 may become the
floor is his (what players get), asked with `ask-nick`.

## Scope

**In:**
- Make the import weak, or absent, without a CPAL fork of ours, in this order:
  1. a CPAL release or feature that doesn't import process taps for output (check its
     license and graph as ADR-0066 did);
  2. a weak import: the program links CoreAudio weakly (`-weak_framework CoreAudio`),
     set where every macOS build of a game is made (`shuyi export macos`, the
     template's `.cargo/config.toml`), so a function the system lacks is null rather
     than a refusal to start.
- `mac-floor.sh --expect` in CI moves to the program's own `minos`, and its message
  and ADR-0142 follow.
- Proof on the oldest macOS a hosted runner offers, and the reasoning for older ones
  written down: dyld's binding of a weak import is documented by Apple.

**Out (do not do):**
- Raising Shuyi's macOS floor without Nick's word.
- Capturing on macOS (3348).

**Estimated size:** about 300 lines (code 100, tests and CI 100, docs 100).

## Implementation steps

1. Look for a CPAL version or feature without the import; else link CoreAudio weakly.
2. Change `mac-floor.sh`'s expectation in `.github/workflows/sound-parity.yml`.
3. Update `docs/targets/desktop.md` and ADR-0142's status line.

## Acceptance criteria

- [ ] `mac-floor.sh` finds no strong audio import newer than the program's `minos`, in
      CI, on both architectures.
- [ ] A game exported for macOS with the in-house sound starts on a hosted runner, and
      the oldest macOS it is claimed for is written in `docs/targets/desktop.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: `mac-floor.sh --expect <minos>` in `sound-macos`; the exported game
  started.

## Completion notes

*(Filled in by the session that completes the ticket.)*
