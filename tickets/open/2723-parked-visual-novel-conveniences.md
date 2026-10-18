---
id: "2723"
title: "Parked: Visual novel conveniences: text speed, auto, skip read text, the backlog and going back"
type: feature
milestone: "Engine: visual novels"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2722"]
nick_input: none
completed:
---

# 2723 — Visual novel conveniences

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0016](../../docs/adr/0016-visual-novels-and-detective-games-a-story-stage-a-case-file-and-a-fair-play-check.md)
§3: some things a visual novel does are the player's, not the story's.
Text speed, auto, skip, what has been read, and the backlog never enter
`VnState`, so they never change a replay. They only change when and how
fast `Advance` is sent, and what the screen shows.

2721 built the story and `history::back`. 2722 built the scene, the
presenter (text reveal, `on_advance`) and `VnStage`. This ticket adds
the conveniences around them.

Every convenience is an action in the player's keymap (`AGENTS.md`
rule 8, the `input-actions` skill), and every screen must fit each
target's controls ([ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§3, ticket 3002). Read the `input-actions` skill first.

## Nick input

None.

## Scope

**In:**
- **The module's actions**, `VnAction` in `shuyi_vn::view::input`:
  `Advance`, `Back`, `Backlog`, `ToggleAuto`, `ToggleSkip` and
  `QuickMenu`. A game maps its own `ActionSet` onto them through a trait
  `VnActions: ActionSet { fn vn(&self) -> Option<VnAction> }`, as
  `ActionSet::ui` maps onto `UiAction`. The engine never picks a key;
  the example's keymap file (2726) holds defaults.
- **Fitting every target.** On `VnStage`:
  - required: `Advance`, choice movement (`UiAction` Up and Down) and
    `QuickMenu`;
  - optional: `Back`, `Backlog`, `ToggleAuto`, `ToggleSkip`;
  - **the quick menu** is a small list screen (glyph cells) that reaches
    every optional action, plus the settings below. So a target with
    few inputs (the Playdate) still reaches all of them, and 3002's
    check passes with only the required ones bound;
  - if 3002 is done, `VnStage`, the quick menu and the backlog declare
    their actions with `Screen::actions`, and a fixture game's
    `playdate` and `steam-deck` layouts pass `shuyi check --target`. The
    layouts are data in the fixture's keymap file. If 3002 isn't done,
    3002 checks these screens when it lands;
  - the README (2726 finishes it) says how a gamepad, the Steam Deck
    and the Playdate reach each action, by action name only. Which input
    does what is the game's keymap data, never the engine's text.
- **The settings**, `VnSettings` in `shuyi_vn::conv`, saved with the
  player's settings through the platform's storage by key (ADR-0008
  §2), never with a save:
  - `text_speed`: glyphs per second from a list of steps, or instant.
    It replaces 2722's default from `vn.ron`;
  - `auto_pause_ms`: a base, plus ms per glyph, from a list of steps;
  - `skip_mode`: `Read` (the default) or `All`.
  - They are a RON text file under one key, versioned, with
    `deny_unknown_fields`. A missing or broken value falls back to the
    defaults and is reported, never a panic.
- **What has been read**, `ReadLines` in `shuyi_vn::conv`: a sorted set
  of line ids per player profile and story, saved under its own storage
  key. A line is marked read when it is shown in full. The scene's text
  box gets its `read` flag from it.
- **The driver**, `Conveniences` in `shuyi_vn::conv`, outside
  `VnState`. Each frame it may ask `VnStage` to send `Advance`:
  - **auto**: once the reveal is done and the pause has passed. 2725
    adds "and the voice has ended";
  - **skip**: sends `Advance` once per frame while the current line is
    read (or always, with `All`), with the presenter `instant`. Skip
    stops at the first unread line (unless `All`), at every choice, and
    at the end. Auto stops at choices too;
  - any press of `Advance`, `Back` or `QuickMenu` turns skip off.
- **The backlog**, `Backlog` in `shuyi_vn::view::backlog`:
  - `Backlog::from_events` collects the shown lines (line id, speaker
    key, text key and arguments) from the events of the run. After
    going back or loading, it is rebuilt by replaying;
  - an overlay screen lists them in glyph cells, newest last, scrolled
    with Up and Down and closed with Cancel. Each entry has a hook to
    replay its voice, filled by 2725.
- **Going back**, the `Back` action: calls 2721's `history::back(…,
  1)`, rebuilds the presenter at the earlier line, fully revealed, and
  rebuilds the backlog. It obeys `back_past_choices`.
- **Pointer, either order with 1121**: if the parked 1121 is done, a
  pointer press on the text box sends `Advance`, and on a choice picks
  it, through the actions 1121 defines. If not, 1121 is not this
  ticket's job.

**Out (do not do):**
- saves, the gallery, endings in the profile and the route chart
  (2724);
- voices, and waiting for them in auto mode (2725);
- hiding the text box to look at the picture, and going forward again
  after going back (write a ticket if the example shows a need);
- naming any key or button in code, data the engine ships, or text.

## Implementation steps

1. Write `VnAction`, `VnActions`, and wire them into `VnStage` beside
   2722's `UiAction`s.
2. Write `VnSettings` and `ReadLines`, their RON forms, and their load
   and save through the storage trait, with a fake storage in tests.
3. Write `Conveniences` (auto and skip) as a pure step: given the
   settings, the read set, the presenter and the elapsed ms, return
   whether to send `Advance`.
4. Write `Backlog` and its overlay screen.
5. Write the `Back` action over `history::back`.
6. Write the quick menu with the settings rows.
7. If 3002 is done, declare the screens' actions and add the fixture
   layouts. If 1121 is done, add the pointer presses.

## Acceptance criteria

- [ ] **Toggling conveniences never changes the replay.** For a random
      command record over 2721's fixture story and random convenience
      changes (speed, auto, skip, read set) between commands, the
      `VnState` hash after each command equals the hash without them
      (property test).
- [ ] Skip in `Read` mode over a story with some lines read stops on
      the first unread line, and in `All` mode stops only at a choice or
      the end.
- [ ] Auto sends `Advance` exactly after the reveal plus the pause, in
      whole ms, and never while a choice is offered.
- [ ] `ReadLines` and `VnSettings` round-trip through a fake storage;
      a broken stored value loads the defaults and reports it.
- [ ] The backlog after going back has the same entries as a run that
      never went past that line.
- [ ] If 3002 is done, `shuyi check --target playdate` and
      `--target steam-deck` pass on the fixture game with only the
      required actions bound, and every optional action is reachable
      from the quick menu (Harness test).
- [ ] `check-keys` passes, and no text the engine ships names a key.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: auto and skip decisions; read marking; the settings' defaults
  and fallbacks; the `Back` action at a choice with
  `back_past_choices` on and off.
- Property: conveniences never change the replay hash.
- Snapshot: the backlog overlay's scene dump; the quick menu's.
- Integration: a Harness test that skips to the first unread line,
  opens the backlog, goes back one line, and reaches every optional
  action through the quick menu.

## Completion notes
