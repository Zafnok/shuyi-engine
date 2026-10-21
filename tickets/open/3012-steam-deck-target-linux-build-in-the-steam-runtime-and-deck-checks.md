---
id: "3012"
title: "Steam Deck target: a Linux build in the Steam Linux Runtime, 1280×800 fullscreen with integer scaling, Deck pad detection, resume after sleep, and the Deck Verified checks"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["3001", "3002", "3003", "3004", "3042", "3053"]
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
  - it reuses the `linux` exporter's build (3004):
    `x86_64-unknown-linux-gnu` in the Steam Linux Runtime 3.0
    ("sniper") SDK container, in CI, with 3004's path remapping and
    library audit, so the binary runs on the Deck's libraries. 3004 set
    the container up and recorded the image, so this exporter only adds
    `SHUYI_TARGET=steam-deck` to the build (see *Display*) and changes
    the package;
  - it writes a `tar.gz` with a launch script and the notices;
  - with `--release`, it keeps the debug file beside the package as
    3004's `linux` exporter does (for 2520's `shuyi crash --symbols`).
- **Display:**
  - the build sets `SHUYI_TARGET=steam-deck`, so `shuyi-app` takes its
    platform facts from its embedded `targets/steam-deck.ron` (3053):
    not windowed (the backend shows full screen whatever
    `Frame::fullscreen` says, and the options screen hides the
    Fullscreen row), `Keyboard::Absent` (the first-key screen never
    opens), and Quit by the game (`quit: Game`);
  - fullscreen at the display's size from the start;
    - either order with 3044 (the window record): the Deck's fullscreen
      applies when there is no record, and a record's mode never takes
      the Deck out of fullscreen, since the Deck is not windowed. If
      3044 is done, set it through its `conf_from`; if not, 3044 keeps
      the Deck's fullscreen;
  - the frame is placed by the game's scale mode through
    `shuyi_ui::frame::console::place` (2333): `integer` by default, the
    largest whole number that fits, with letterboxing in the skin's
    backdrop colour. Either order with 2333: if it isn't done, integer
    scaling goes through today's `console::layout`, and 2333 moves it;
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
- **Focus.** 3042 makes the macroquad backend send `FocusLost` and
  `FocusGained` from miniquad's focus events on Linux as on every
  desktop, so nothing Deck-specific is added here. Under gamescope the
  Steam overlay may not take the window's focus; a game that must know
  the overlay opened uses Steamworks' `GameOverlayActivated_t` callback,
  which is the game's 0903.
- **The checks.** `shuyi check --target steam-deck` runs 3001's text
  check and 3002's controls check with the Deck's profile. A game whose
  required actions need the keyboard or the touchscreen fails.
- **Docs.** `docs/targets/steam-deck.md` covers:
  - how to test on a Deck without Steam publishing (the SteamOS Devkit
    Client, or a non-Steam game in desktop mode), what Deck Verified
    asks, what the engine covers, and what is the game's (0903);
  - attaching from a computer over the LAN with `shuyi attach` (2522).
    **Either order with 2522:** if 2522 is done, add the Deck's launch
    options (`SHUYI_LINK_LAN=<port> SHUYI_LINK_TOKEN=<token> %command%`)
    and link the steps in `docs/contract.md`'s "Attaching to a running
    build". If not, 2522 adds that section here;
  - where crash reports land. If 3046 is done, the page says where
    crash reports (`crash-<when>.ron`) and, if 3041 is done, the log
    files land on the Deck: the folder `shuyi_app::storage::data_dir`
    gives inside the Steam Linux Runtime (check whether it sets
    `XDG_DATA_HOME`), and how a player finds it in desktop mode. If
    not, 3046 adds those lines here, and its `docs/crash-reports.md`
    links to this page for the Deck.

**Out (do not do):**
- Steamworks, Steam Input action sets (the game's 0903);
- Steam achievements and Steam Cloud saves (3036, 3037);
- measuring on the device (3013);
- the new-game wizard's Steam Deck part. Either order: if 2504 is done,
  add the Deck export to the built-in template's `target-steam-deck`
  part's CI matrix in this PR; if not, 2504 adds it;
- the release from a tag. Either order with 3050: if 3050 is done, the
  `steam-deck` exporter fills `Exporter::host()` (`linux`),
  `container()` (the sniper SDK image) and `ci()` (`build`), so
  `export --plan` lists it and a tag's release builds it in the
  container through `export.yml`; if not, 3050 fills them;
- an engine-drawn keyboard (the game already has a letter grid; Nick
  chose Steam's keyboard for the Steam build, game 0903).

## Implementation steps

1. Add the exporter over 3004's `linux` build, with
   `SHUYI_TARGET=steam-deck` set for the build, and a CI step that runs
   it in 3004's sniper container job.
2. Add the fullscreen setting to the macroquad backend: where the facts
   say not windowed, fullscreen from the start and whatever the frame
   says. The scaling is 2333's `place` with the game's `ScaleMode`
   (without 2333, integer through `console::layout`).
3. Add Deck pad detection and its `HelpKeys` names (positions stay
   positions).
4. Add the resume after a gap, with a test through the headless backend
   (a fake clock gap).
5. Write the docs, and export an example.

## Acceptance criteria

- [ ] CI produces a `steam-deck` package for an example, built in
      3004's sniper container job. Its report passes 3004's library
      audit (no `GLIBC_` version above the floor).
- [ ] The package's binary is built with `SHUYI_TARGET=steam-deck` (the
      exporter's test reads its build command), and a headless test
      with the `steam-deck` facts shows no Fullscreen row.
- [ ] A test: a frame gap of 5 s produces `Resume`, and at most one
      tick step.
- [ ] `check --target steam-deck` fails a fixture whose screen needs the
      keyboard for a required action, and passes the examples.
- [ ] The help text on a fixture with `PadKind::SteamDeck` names the
      Deck's buttons (snapshot).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `place` with `ScaleMode::Integer` (the scale and the letterbox)
  at 1280 × 800 and 1280 × 720 (without 2333, `console::layout`); pad
  detection by ids.
- Integration: the export in CI; resume through the headless backend.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
