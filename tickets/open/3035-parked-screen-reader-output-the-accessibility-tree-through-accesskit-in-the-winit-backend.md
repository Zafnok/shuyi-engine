---
id: "3035"
title: "Parked: Screen reader output: the accessibility tree sent to Windows, macOS and Linux screen readers through AccessKit, in the winit backend"
type: feature
milestone: "Engine: accessibility"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2322", "3034"]
nick_input: none
completed:
---

# 3035 — Screen reader output through AccessKit

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): it needs the winit backend that 3034's research plans, and the accessibility tree (2322), which is parked too. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

2322 builds an `AccessTree` from the engine's views and hands an
`AccessUpdate` out with each frame. This ticket gives it to the
system's screen reader.

AccessKit does this for Bevy, egui and Godot 4.5+: one tree, adapters
for UI Automation (Windows), NSAccessibility (macOS) and AT-SPI
(Linux), and `accesskit_winit` to join it to a winit window. macroquad
gives no way in, so this lands in the winit backend that
[ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§1 chose and 3034 plans. When 3034 writes the winit backend's ticket,
it adds that ticket to this one's `blocked_by`.

Only platform backends touch the system (`AGENTS.md` rule 5): the tree
is plain data until here.

## Nick input

None.

## Scope

**In:**
- **Licenses first** (ADR-0002 §8, ADR-0004 §7). AccessKit's crates
  say MIT OR Apache-2.0. Check each one this adds (`accesskit`,
  `accesskit_winit` and the platform adapters) and what their default
  features pull in: `accesskit_unix` uses `zbus` and an async runtime.
  Pick the features that pass `cargo deny`, and record them in the
  Completion notes. Check it builds with the GNU toolchain on Windows
  (`AGENTS.md`, *Environment*).
- **The adapter**, in the winit backend's crate, behind an
  `accessibility` feature that is on by default on desktop:
  - each `AccessNode` becomes an `accesskit::Node` (a table from
    2322's `Role` to AccessKit's roles, with a test for each);
  - a `TreeUpdate` is sent only when 2322 hands a new tree;
  - the focus;
  - announcements become a live-region node, polite or assertive;
  - the tree is built lazily: nothing is built until a screen reader
    asks (AccessKit's activation handler), so a player without one pays
    nothing.
- **Requests from the screen reader** (`Focus`, `Click`): they become
  UI actions, never keys (`AGENTS.md` rule 8, the `input-actions`
  skill). `Focus` on a menu item moves the cursor there; `Click` is
  `UiAction::Confirm` on the focused item. Document this in 2322's guide.
- **Tests:** the role table; updates only when the tree changes; the
  requests to actions; and the tree checked with `accesskit_consumer`
  (it walks a `TreeUpdate` as a screen reader would), headless.
- **A manual check**, written in `docs/accessibility.md`: NVDA on
  Windows reads the toy game's options menu, its values, and an
  announcement. Do it if a Windows machine is at hand; say in the
  Completion notes whether it was done.
- **The web** is 3058 (parked, blocked by this ticket): the tree
  mirrored into ARIA elements in the page and announcements in live
  regions. It reuses the request mapping above, so keep the part of
  that mapping that doesn't touch AccessKit beside the tree in
  `shuyi-ui`, not in the winit backend's crate.

**Out (do not do):**
- the web (3058);
- the Playdate, the Steam Deck's own reader settings, consoles;
- the tree itself (2322);
- speech of our own.

## Implementation steps

1. Read 2322's `access` module and guide, 3034's ADR, and the winit
   backend's code.
2. Check the licenses and features.
3. Write the role table and the update path, with tests.
4. Write the request handling, with tests.
5. Add the `accesskit_consumer` test and the guide's manual check.
6. Run the gates.

## Acceptance criteria

- [ ] `cargo deny check` passes with the new crates, and their licenses
      and features are recorded.
- [ ] Every `Role` maps to an AccessKit role, tested.
- [ ] `accesskit_consumer` finds the focused node and its name in the toy
      game's options menu.
- [ ] A screen reader's `Click` on a menu item confirms it, through
      `UiAction::Confirm`.
- [ ] With no screen reader running, no tree is built (a test with a
      counting builder).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the role table; announcements as a live region; requests to
  actions.
- Integration: the toy's options menu through `accesskit_consumer`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
