---
id: "2404"
title: "Parked: Input that asks less of the hands: hold or toggle per action, and the repeat speed and the chord window set by the player"
type: feature
milestone: "Engine: accessibility"
model: sonnet-5
effort: medium
status: blocked
blocked_by: ["3042"]
nick_input: none
completed:
---

# 2404 — Hold or toggle, repeat speed and chord window for the player

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): the game doesn't ask for it, so this waits until after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20 and Godot 4.6, under
accessibility. The Game Accessibility Guidelines ask for alternatives
to holding buttons down, and for timings the player can slow down.
Many games offer "toggle sprint" or "toggle aim".

Today `InputState` (`crates/ui/src/input/state.rs`):

- repeats held repeatable actions with the keymap's `RepeatDef {
  delay_ms, interval_ms }`;
- waits the keymap's `chord.window_ms` for a button chord;
- answers `is_held(action)`, as for "hold to fast-forward" (2312's
  clock that a held action speeds up).

All three are the game's numbers. A player who can't hold a button, or
presses slowly, has no say. Players can already rebind every action
(the key bindings screen).

Rules: actions, never keys (`AGENTS.md` rule 8, the `input-actions`
skill).

## Nick input

None.

## Scope

**In:**
- **`ActionSet::can_toggle(self) -> bool`**, defaulted to `false`: the
  actions that make sense held, which a player may switch to a press
  that latches. A defaulted method breaks no game, so the game's PR that
  moves the pin needs no change. An action that is both `can_toggle`
  and `is_repeatable` is a keymap error (a latched repeat would never
  stop).
- **The player's settings**, in the engine's `Settings` and its saved
  form (old saves read as the defaults; if 2110 is done, each versioned
  settings form this changes, the engine's `()` settings and the grid
  template's, raises its `VERSION`, reads the old version by
  `Self::deserialize` in `Migrate::upgrade`, and adds a `v<N>_*.ron`
  fixture, written by this build, beside the old ones; if not, 2110's
  version 1 holds them):
  - `toggled`: the names of the actions the player set to toggle (by
    name, as the saved bindings are);
  - `repeat`: `None` (the game's), or a `RepeatDef` from three presets
    (slower, the game's, faster), with bounds checked: delay 150–1500
    ms, interval 30–500 ms;
  - `chord_window`: `None` (the game's), or a longer window, up to
    500 ms.
- **`InputState`:**
  - a toggled action latches on its press: `is_held` is true until the
    next press; its release does nothing;
  - a lost focus or a suspend clears latches as it clears held keys:
    `InputState::forget_held` (3042), which `App` calls on `FocusLost`
    and `Suspend`, forgets latches too, so nothing stays stuck;
  - `InputState::set_player_input(&PlayerInput)` applies the three
    settings; `Game` calls it when they change.
  - **either order with 2406 (parked):** if 2406 is done, `Game` calls
    `set_player_input` on every seat's `InputState`, when a seat fills
    and when the settings change: every seat plays with the current
    profile's settings, as it plays with its bindings. If not, nothing
    to do (2406 does it).
- **Option rows:** `OptionRow::toggle(action)`, a row per `can_toggle`
  action ("Fast-forward: Hold / Toggle"); `OptionRow::repeat_speed()`;
  `OptionRow::chord_time()`. A game adds them to its options list.
  **Either order with 2314:** if 2314 is done, they are enum option
  rows; if not, stepped rows, and 2314 converts them.
  **Either order with 2351:** if it is done, the English of the keys
  these rows name goes in the engine's `crates/ui/lang/en/ui.ron`; if
  not, in the toy's `en.ron`, and 2351 moves it. A `toggle(action)`
  row's action label stays the game's.
- **The toy game** has one `can_toggle` action and the three rows.

**Out (do not do):**
- one-switch and scanning input (a follow-up if a game asks);
- alternatives to button mashing or quick-time events (game rules);
- help text that says "press" instead of "hold" for a toggled action (a
  follow-up with 2006's `check-text`, if a game names holding in its
  text);
- the motion, shake and flashing settings (2340), and sound captions
  (2366, parked). Each of them adds to `Settings` and every repository
  game's saved form, as this one does; whichever lands later follows
  the shape of the ones before it.

## Implementation steps

1. Read the `input-actions` skill, `InputState` (with 3042's
   `forget_held`), `Keymap`, `RepeatDef` and `Settings`. Check whether
   2110, 2314, 2340 and 2351 are done.
2. Add `can_toggle` and the keymap check.
3. Add the settings and their saved form.
4. Add latching, latches in `forget_held` and `set_player_input`.
5. Add the rows and use them in the toy game.
6. Run the gates.

## Acceptance criteria

- [ ] A toggled action is held from its first press to its second, and
      not after its release.
- [ ] Losing focus or a suspend clears a latch.
- [ ] The slower repeat preset repeats later and less often than the
      game's, by the preset's numbers, under a scripted held key.
- [ ] A longer chord window lets a chord through that the game's window
      misses.
- [ ] Settings saved before this ticket load with the defaults (if 2110
      is done, the old fixtures in each changed form's
      `tests/saves/settings/` still read).
- [ ] A keymap with an action both toggled and repeatable fails to load,
      naming it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: latching; focus loss and suspend; each preset; the chord window; the saved
  form, old and new.
- Property: for random press and release scripts, a toggled action is
  held exactly between odd and even presses.
- Snapshot / integration: the toy game's options screen with the new
  rows.

## Completion notes

*(Filled in by the session that completes the ticket.)*
