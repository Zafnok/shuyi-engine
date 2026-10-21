---
id: "3023"
title: "Parked: Surfaces: a frame with more than one screen, layouts for targets with fewer, pointers per surface, and a check for games that need two visible at once"
type: feature
milestone: "Parked: export targets"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["3003", "3017", "2405"]
nick_input: none
completed:
---

# 3023 — Surfaces: a frame with more than one screen

> **Parked** (Nick, 2026-10-17: "I want GBA, and DS ports"): until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0012](../../docs/adr/0012-gba-and-ds-targets-capabilities-indexed-colour-two-screens-and-homebrew.md)
§5. The DS has **two 256 × 192 screens**, the lower one a resistive
touchscreen (ADR-0012, *The hardware*). The frame has one picture today.
ADR-0012 decides:

- the frame may hold **more than one surface**. A game declares its
  surfaces and what each shows; by default there is one;
- on a target with that many screens, each surface goes to its screen;
- on a target with fewer, the profile's `layout` places them in one
  window: stacked, side by side, or one at a time with an action to
  switch, chosen by the player;
- a pointer on a surface is reported in that surface's coordinates
  (2405's pointer events);
- a game that needs two surfaces visible at once declares it, and a
  one-at-a-time layout then fails the check.

The profile's `capabilities.screens` (3017) says how many screens a
target has. `Platform::display` (3003) is where a backend says so at
run time. Pointer input is 2405's (1121 adds the pointer in world
pixels for tick stages).

So a DS game also runs on desktop and the web, and a desktop game with
a map and a status panel can put them on the DS's two screens.

## Nick input

None.

## Scope

**In:**
- **Surfaces in the frame**, in `shuyi_ui::frame`:
  - `SurfaceId`, and `Frame { surfaces: Vec<SurfaceFrame> }`, where a
    `SurfaceFrame` holds what one frame holds today (cells, world
    layers, items);
  - a surface is a physical screen, and a world layer's viewport (1120)
    is a rectangle inside one surface, in that surface's console pixels.
    Layouts place surfaces; viewports place layers inside one. Split
    screen on one screen is viewports, not surfaces, and neither is a
    second layout system for the other;
  - `Frame::main()` and the existing builders write to the first
    surface, so a game with one surface changes nothing;
  - a screen says which surface each of its parts draws to.
- **The game declares its surfaces** in its manifest (1129):
  `surfaces: [(id: "main", cells: (32, 24)), (id: "touch", cells: (32,
  24))]`, and `needs.surfaces_visible_at_once: 2` when the game needs
  both seen together (a need of 3017's `Needs`).
- **Layouts**, in the profile: `layouts: [Stacked, SideBySide,
  OneAtATime]` and a `default_layout`, used when the target has fewer
  screens than the game has surfaces:
  - `Stacked` and `SideBySide` place the surfaces in one window, scaled
    together;
  - `OneAtATime` shows one, and the engine's `SwitchSurface` action
    (in `UiActions`, bound in the keymap like any other; read the
    `input-actions` skill) shows the next;
  - the player picks among the profile's layouts in the settings, and
    the choice is saved;
  - every existing profile gets `layouts`, `desktop` and `web` all three.
- **The checks**, in `shuyi check --target <id>`:
  - a game with more surfaces than the target has screens, on a profile
    with no `layouts`, fails;
  - a game with `surfaces_visible_at_once: 2` fails on a profile whose
    only layout is `OneAtATime`. A layout that can't show both is hidden
    from the player for that game;
  - under `OneAtATime`, `SwitchSurface` is a required action on every
    screen with more than one surface, so 3002's controls check counts
    it (if 3002 has landed; if not, 3002 picks it up from the screen's
    declared actions).
- **Pointers per surface** (2405): a pointer event carries the
  `SurfaceId` it is over and coordinates in that surface's pixels. On
  desktop with `Stacked`, a click on the lower half reaches the second
  surface at its own (0, 0). Between surfaces, or over none, it reports
  no surface. Inside a surface, 1121 then maps the point to a world
  layer through its viewport, so the chain is window to surface (here)
  to viewport (1120, 1121). Each surface's frame keeps its own hit areas
  (2405's `GlyphBuffer::hits`), and a pointer's `over` and `clicked`
  are looked up in the hits of the surface it is over. A touch-only
  need (3017's `touch`) is met only by a surface marked `touch`.
- **`Platform::display`** (3003) reports the device's screens. `App`
  maps surfaces to screens, or to one screen through the layout.
  `shuyi-app` and the headless backend draw every layout.
- **Snapshots** list every surface, each under its id. Tests of what
  happened read them.
- **`frame-png`** draws every surface as the target's layout places
  them, or one with `--surface <id>`. On a two-screen target, the two
  screens are drawn one above the other, as on the device.
- **An example.** `hello-grid` (1118) gets an optional second surface
  showing the selected unit, and Harness tests for each layout.

**Out (do not do):**
- the DS backend (3024);
- more than two surfaces in the layouts (the types allow it; write a
  ticket when a target needs it);
- a 3D view on each surface (3025);
- gestures (2407).

## Implementation steps

1. Add `SurfaceId` and `SurfaceFrame`, with `Frame::main()`, and move
   the frame's builders onto it without changing any snapshot.
2. Add the manifest's `surfaces`, the profile's `layouts`, and the
   `surfaces_visible_at_once` need, with format version bumps and
   defaults.
3. Add the layouts to `App`, `SwitchSurface`, and the player's setting.
4. Add surface coordinates to 2405's pointer events and a hit list per
   surface, with tests per layout.
5. Add the checks, the snapshot lines and `frame-png --surface`.
6. Add the example's second surface and its tests.

## Acceptance criteria

- [ ] Every existing snapshot and replay passes unchanged.
- [ ] A two-surface fixture game passes `shuyi check --target nds` and
      `--target desktop`.
- [ ] With `surfaces_visible_at_once: 2`, it fails on a fixture profile
      whose only layout is `OneAtATime`, naming the need and the layout.
- [ ] On desktop with `Stacked`, a Harness click at the top-left of the
      lower surface reports that surface at (0, 0).
- [ ] Under `OneAtATime`, `SwitchSurface` shows the other surface, and
      the player's layout choice survives a restart (Harness test).
- [ ] `frame-png` of the example matches its checked-in PNG in each
      layout.
- [ ] `check-keys` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the layout maths (placement and scale), pointer mapping per
  layout, and each check.
- Property: a random point in the window maps to at most one surface,
  inside its bounds.
- Snapshot: the example in each layout, as text and as `frame-png`.
- Integration: Harness tests of switching and clicking.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
