---
id: "3015"
title: "Parked: Mobile (Android and iOS): on-screen touch controls as an input profile, safe areas, the app lifecycle, and store builds"
type: feature
milestone: "Parked: export targets"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["3002", "3003", "3004", "3008"]
nick_input: decision
completed:
---

# 3015 — Mobile: Android and iOS

> **Parked** (Nick, 2026-10-15: "and maybe even mobile"): until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md).
macroquad runs on Android and iOS (its README), so the `shuyi-app`
backend can reach phones. The `android` and `ios` profiles exist as
`planned` (3001). Four things are missing:

- **Touch controls.** A turn-based grid game plays well by touch, but
  the engine has no pointer input yet (parked 1121), and 1121 leaves out
  on-screen controls.
- **Safe areas.** Notches and rounded corners.
- **The lifecycle.** The phone pauses and kills apps in the background.
- **Store builds.** Signing and the store packages.

This ticket is too big for one PR. When it is unparked, the first step
splits it.

## Nick input

**Decision** (the `ask-nick` skill), when unparked:

- Android, iOS, or both?
- **The stores' costs.** Google Play has a one-time developer fee, and
  Apple's Developer Program has a yearly fee and needs a Mac for
  signing (CI has macOS runners). Check today's prices.
- **On a phone, touch only, or touch plus an optional pad?** Read with
  the controls rule: required actions must fit the on-screen controls.

## Scope

**In** (after the split):
- **On-screen controls as an input profile.** A virtual d-pad and
  buttons drawn by the engine, whose positions and sizes are data the
  player can move. The `android` and `ios` profiles list them, and
  3002's check applies.
- **Direct touch.** Tapping a tile or a menu row, through 1121's
  pointer, which must be unparked first, or which this ticket asks Nick
  to unpark.
- **Safe areas.** They come from the platform, and the frame is laid out
  inside them.
- **The lifecycle.** Going to the background sends `Suspend` (3003). The
  game saves, and resumes where it was.
- **Exporters.** `android` (an AAB or APK; macroquad's own Android tool,
  license checked) and `ios` (an Xcode project built in CI on macOS).
  Signing uses secrets Nick sets up.
- **On a device:** the bench scenarios on a four-year-old mid-range
  phone, against the budgets.

**Out (do not do):**
- in-app purchases, ads, accounts;
- store listings (the game's work).

## Implementation steps

1. Wait for Nick to unpark it and decide.
2. Split it into tickets: touch controls, lifecycle and safe areas, the
   Android exporter, the iOS exporter, and measuring on a device.
3. Work them in that order.

## Acceptance criteria

- [ ] Nick's decision is recorded.
- [ ] The split tickets exist, with dependencies (including 1121).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Per the split tickets.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
