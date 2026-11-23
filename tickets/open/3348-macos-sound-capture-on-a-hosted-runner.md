---
id: "3348"
title: "Record macOS's sound on a hosted runner, so the in-house backend's macOS cells can pass"
type: infra
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: todo
blocked_by: ["3032"]
nick_input: none
completed:
---

# 3348 — macOS's sound recorded in CI

## Context

2573's rule (ADR-0067) is that a port's new sound output can't support a parity
claim until its real output is recorded and judged, and 3032 was to make a route
work on a suitable runner "or remain blocked there with the old backend". 3032 built
and tested the in-house backend on Apple silicon and Intel runners and checked its
oldest macOS (`sound-macos` in `.github/workflows/sound-parity.yml`,
`tools/sound-parity/mac-floor.sh`), but recorded nothing: `macos` is `Blocked` in
`tools/sound-parity/budget.ron`, and `cargo xtask sound-parity port-ready macos`
refuses. 3084 can't make the in-house backend macOS's default until it passes.

What 3032's runs found about the hosted runners (the step *What this runner offers
as sound* prints them, CI run 45135973272):

- `macos-15` (Apple silicon, macOS 15.8.2): the default output is "Apple Virtual
  Sound Device", 2 channels at 48,000 Hz; beside it Apple's "Null Audio Device", 2 in
  and 2 out at 44,100 Hz, not the default.
- `macos-15-intel` (macOS 15.7.9): "Null Audio Device" alone, the default in and out,
  at 44,100 Hz.
- System Integrity Protection is off on both, so the runner's permission database can
  be written to, as the runner images do for their own tools.
- The in-house probe opened the default device on both and played (its log's
  `silent_because` was `null`).

Whether the "Null Audio Device" gives back on its input what its output plays is not
known: find out before writing a device of our own.

The way Windows was solved ([ADR-0075](../../docs/adr/0075-in-house-sound-output-cpal-beside-quad-snd-a-worklet-fed-by-messages-and-a-virtual-endpoint-for-windows-evidence.md)):
a virtual render endpoint installed on the runner, and the system's own loopback
recorded. On macOS the usual virtual device on CI runners is GPL-3.0, which ADR-0002
§8 denies (2573: no denied capture drivers).

## Nick input

None. A paid runner or a new legal duty would be a product decision; don't assume one.

## Scope

**In:**
- A capture route on a hosted `macos-15` (Apple silicon) runner, and on
  `macos-15-intel` if it works the same, recording what the default output plays at
  44,100 Hz, 16-bit stereo, as `capture.py`'s other platforms do. Candidates, in the
  order to try:
  1. the "Null Audio Device" as the default output, if its input returns what it
     plays, set to 44,100 Hz as Windows' endpoint is;
  2. a Core Audio process tap (`AudioHardwareCreateProcessTap`, macOS 14.2 and later)
     on the probe's process, with the tap's permission granted on the runner;
  3. an Audio Server plug-in of our own: an output device whose samples a matching
     input device gives back, written from Apple's `AudioServerPlugIn.h`
     documentation, MIT OR Apache-2.0, built and installed in the job, never shipped.
- `capture.py macos` (and `--new-probe`, `--only`, `--take-device` if a device can be
  taken away), the probe's CPU and memory on macOS, `macos` and `macos-in-house` as
  `Captured` and `NewBackend` in `budget.ron` with limits calibrated from
  `calibrate.yml` runs named beside them, and `port-ready macos` passing.
- An ADR for the route.

**Out (do not do):**
- A third party's GPL or paid driver.
- Making anything the default (3084).

**Estimated size:** about 1,200 lines (code 700, tests and CI 300, docs 200) with a
plug-in of our own; about 500 with a tap.

**Split:** past 1,000 lines, the plug-in and its install land first, with a CI step
proving a tone played to it comes back, and the harness's `macos` platform second.

## Implementation steps

1. Try the null device's loopback, then the tap, then the plug-in.
2. Prove a known tone round-trips sample-exactly, as `install-endpoint.ps1` did.
3. Add `capture.py macos`, then the budget's rows, then calibrate.

## Acceptance criteria

- [ ] quad-snd's and the in-house backend's real output on macOS are recorded in CI and
      every `macos` and `macos-in-house` cell is mandatory and passes.
- [ ] `cargo xtask sound-parity port-ready macos` passes, and CI's step that expected it
      to fail is gone.
- [ ] No denied or paid driver is installed.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: a known signal through the route, compared sample for sample; the
  fault fixtures caught on macOS's recordings as on the others.

## Completion notes

*(Filled in by the session that completes the ticket.)*
