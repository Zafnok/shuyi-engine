---
id: "3012"
title: "Steam Deck target: a Linux build in the Steam Linux Runtime, 1280×800 fullscreen with integer scaling, Deck pad detection, resume after sleep, and the Deck Verified checks"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["3001", "3002", "3003", "3004"]
nick_input: none
completed:
---

# 3012 — Steam Deck target

## Context

Nick, 2026-10-15 (`docs/decisions.md`): "I also want it to be exported
to steam deck". The Deck is a Linux PC with a 1280 × 800 screen, and
macroquad already runs on Linux.
[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§1 gives the `steam-deck` profile. What Deck Verified asks for
(Steamworks documentation, *Steam Deck Compatibility Review Process*,
read 2026-10-15):

- text at least 9 px tall at 1280 × 800, with 12 recommended;
- a default controller layout that reaches all content;
- the on-screen keyboard where the player types.

The first two are engine checks (3001's text check, 3002's controls
check). The keyboard needs Steamworks. Steam itself (the partner account,
`steamworks`, depots, the store) is the game's ticket 0903, so the engine
only offers the `TextInput` hook (3003) that the game's Steam build
fills.

## Nick input

None. Running it on a real Deck is 3013.

## Scope

**In:**
- **The `steam-deck` exporter** (3004):
  - it builds `x86_64-unknown-linux-gnu` inside the Steam Linux Runtime
    3.0 ("sniper") SDK container, in CI, so the binary runs on the Deck's
    libraries. Record the image name and how it was checked;
  - it writes a `tar.gz` with a launch script and the notices.
- **Display:**
  - fullscreen by default at the display's size;
  - the frame is scaled by the largest whole number that fits, with
    letterboxing in the skin's backdrop colour;
  - the game's `targets.steam-deck` frame (3001) decides the layout. An
    example uses 640 × 400 at scale 2.
- **The pad:**
  - the Deck's built-in controller (Valve's USB vendor id `28de`, and the
    virtual Xbox pad Steam Input gives by default) is reported as
    `PadKind::SteamDeck`, so the help text names the Deck's buttons
    through `HelpKeys` (the `input-actions` skill);
  - the Steam and Quick Access buttons are never read.
- **Resume after sleep.** The Deck freezes a sleeping process without a
  signal. When the macroquad backend sees more than 1 s between two
  frames, it sends `Resume` (3003), so the game never runs a burst of
  catch-up steps.
- **Focus.** When the window loses focus (the Steam overlay or the Quick
  Access menu), it sends `FocusLost`, and the game's flow may pause.
- **The checks.** `shuyi check --target steam-deck` runs 3001's text
  check and 3002's controls check with the Deck's profile. A game whose
  required actions need the keyboard or the touchscreen fails.
- **Docs.** `docs/targets/steam-deck.md` covers how to test on a Deck
  without Steam publishing (the SteamOS Devkit Client, or a non-Steam
  game in desktop mode), what Deck Verified asks, what the engine
  covers, and what is the game's (0903).

**Out (do not do):**
- Steamworks, Steam Input action sets, Steam Cloud and achievements (the
  game's 0903);
- measuring on the device (3013);
- an engine-drawn keyboard (the game already has a letter grid; Nick
  chose Steam's keyboard for the Steam build, game 0903).

## Implementation steps

1. Add the CI job with the sniper SDK container, and the exporter.
2. Add the fullscreen and integer scaling settings to the macroquad
   backend.
3. Add Deck pad detection and its `HelpKeys` names (positions stay
   positions).
4. Add the resume and focus events, with tests through the headless
   backend (a fake clock gap).
5. Write the docs, and export an example.

## Acceptance criteria

- [ ] CI produces a `steam-deck` package for an example, built in the
      sniper container.
- [ ] A test: a frame gap of 5 s produces `Resume`, and at most one
      tick step.
- [ ] `check --target steam-deck` fails a fixture whose screen needs the
      keyboard for a required action, and passes the examples.
- [ ] The help text on a fixture with `PadKind::SteamDeck` names the
      Deck's buttons (snapshot).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: integer scaling and letterboxing at 1280 × 800 and 1280 × 720;
  pad detection by ids.
- Integration: the export in CI; resume through the headless backend.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
