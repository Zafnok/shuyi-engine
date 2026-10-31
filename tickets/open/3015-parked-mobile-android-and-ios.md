---
id: "3015"
title: "Parked: Mobile (Android and iOS): on-screen touch controls as an input profile, safe areas, the app lifecycle, and store builds"
type: feature
milestone: "Parked: export targets"
model: reasoning
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

- **Touch controls.** A turn-based grid game plays well by touch.
  2405 (After MVP) reads taps as pointer presses on menus and grid maps,
  and 2407 (parked) plans on-screen controls for phones' browsers. This
  ticket reuses them. Nothing yet turns those controls on in the
  `android` and `ios` profiles, or lets the player move them.
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
- **On-screen controls as an input profile.** The control set, the hit
  test, `PadKind::Touch`, swipes and the controls-fit rule are 2407's
  plan. Either order: if 2407's split tickets are done, this ticket
  turns on their on-screen profile field in `android.ron` and `ios.ron`
  and adds the examples' `android` and `ios` tables. If 3015 is
  unparked first, its touch-controls ticket takes 2407's items 1 and 2
  (the controls as data, and swipes) with 2407's rules, and the split's
  PR points the lines that send those to 2407 at it. 2407 stays parked
  with its question to Nick and its item 3, the `web-touch` target,
  which is for browsers only (2407's clause with 3015 says the rest).
  This ticket keeps the player moving and resizing the controls, saved
  with the settings.
- **Direct touch.** Tapping a tile or a menu row, through 2405's
  pointer (a tap is its primary press); 2407's on-screen controls for
  everything else.
- **Safe areas.** They come from the platform, and the frame is laid out
  inside them.
- **The lifecycle.** Going to the background sends `Suspend` (3003). The
  game saves, and resumes where it was.
- **Vibration.** Native phones' vibration is this ticket's (2408's
  Out). **Either order with 2408** (parked, rumble): if 2408 is done,
  the phone backends give `Platform::motors`, the phone's one vibrator
  playing the stronger of the two levels, in the lifecycle and safe
  areas ticket; if not, the split writes it as a parked ticket of its
  own, blocked by 2408.
- **Exporters.** `android` (an AAB or APK; macroquad's own Android tool,
  license checked) and `ios` (an Xcode project built in CI on macOS).
  Both are `Exporter`s (3004), and each package passes 3004's path
  search and 3071's library audit, as ADR-0058 says, against a list for its
  target in `crates/cli/export/libraries.ron`.
  Signing uses secrets Nick sets up, read from environment variables
  with fixed names, as 3051 does for the desktop. If 3051 is done, the
  `ios` exporter reuses its temporary keychain step and its `Redactor`.
  If not, this ticket writes them and 3051 reuses them. Each store
  package's id, name and version come from `shuyi.ron`'s `app` part
  (3048), and its icons from the part's `icon` (3072). Android's application id allows no `-`, which 3048's id
  allows, so the Android exporter maps it to `_` and `check --target
  android` warns that it did.
- **License notices.** A phone app's files are no folder a player can
  browse, so `NOTICES.txt` (3039, 3004) doesn't reach the player. Say
  how each store build shows the notices. If that needs a screen in the
  game, write the ticket for the engine screen of credits and notices
  that 2223 leaves out until a phone or console asks (from 3039's
  `notices` works and `credits::merge`).
- **Platform facts.** **From 3053 (done):** the `android` and `ios`
  exporters set `SHUYI_TARGET` (`android`, `ios`) in their builds, and
  `shuyi-app` embeds `targets/android.ron` and `targets/ios.ron` beside
  the profiles it already embeds (`PROFILES`, `crates/app/src/facts.rs`),
  so a phone runs with its own facts (`quit: System`, touch), not the
  desktop's (3053's Out leaves this here).
- **On a device:** the bench scenarios on a four-year-old mid-range
  phone, against the budgets.

**Out (do not do):**
- in-app purchases, ads, accounts;
- store listings (the game's work).

## Implementation steps

1. Wait for Nick to unpark it and decide.
2. Split it into tickets: touch controls (what 2407's split hasn't
   done: the player moving and resizing them, the `android` and `ios`
   profiles, and 2407's items 1 and 2 while 2407 is open), lifecycle
   and safe areas (with vibration, as its bullet says), the Android
   exporter, the iOS exporter, and measuring on a device.
3. Work them in that order.

## Acceptance criteria

- [ ] Nick's decision is recorded.
- [ ] The split tickets exist, with dependencies (including 2405 and 2407).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Per the split tickets.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
