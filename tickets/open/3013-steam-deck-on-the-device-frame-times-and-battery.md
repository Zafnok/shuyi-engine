---
id: "3013"
title: "Steam Deck on the device: run the bench scenarios, log frame times and power draw with MangoHud, and calibrate the steam-deck budgets"
type: research
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["3008", "3012"]
nick_input: setup
completed:
---

# 3013 — Steam Deck on the device

## Context

[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§6. The `steam-deck` budgets are provisional:

- p99 frame time ≤ 16.7 ms, with no frame over 33 ms;
- 3 s to start;
- at most 8 W of device draw on an idle battle map, which is about five
  hours on the 40 Wh LCD model.

A 2D game should meet the frame time easily. Battery is where a careless
engine shows: drawing every frame at full speed when nothing changed, or
a busy loop. This ticket measures on a real Deck and records the numbers
in `docs/targets/steam-deck.md`. It also gives the `steam-deck` exporter
(3012) the run step 3004 defines, so a session puts a build on the Deck
and starts it from the computer over SSH.

## Nick input

**Setup:**

1. This needs a Steam Deck. If you don't have one, this ticket waits.
2. Put the example on it. The session will give the exact steps from
   `docs/targets/steam-deck.md` (3012): either the SteamOS Devkit Client
   from Windows, or copying the folder over and adding it as a non-Steam
   game in desktop mode. Or, once: in desktop mode, set a password with
   `passwd` in Konsole, turn on SSH with `sudo systemctl enable --now
   sshd`, and read the Deck's address in its network settings. The
   session then puts the example on the Deck and starts it with `shuyi
   export steam-deck --run --host deck@<address>`.
3. Say which model it is (LCD or OLED).

**Sign-off:** play the example on the Deck for five minutes in handheld
mode, and say whether the text is comfortable to read and whether it
feels smooth.

## Scope

**In:**
- **Run from the computer.** `shuyi export steam-deck --run --host
  <user@address>` (3004's run step) copies the package to
  `~/shuyi/<game>/` on the Deck with the system's `scp` (OpenSSH;
  Windows 11 ships the client) and unpacks it. It starts the launch
  script with the system's `ssh` in the Deck's desktop session, and
  prints the game's stdout and stderr until the game exits or the
  command is stopped. A missing `ssh` or a refused login says how to set
  it up. `docs/targets/steam-deck.md` gains this route beside the
  SteamOS Devkit Client. If it passes about 600 changed lines, it goes
  to a follow-up ticket (After MVP, `30xx`, written with the
  `write-ticket` skill), with its criterion below, and this ticket's
  measurements use the Devkit Client.
- **Frame times.** Log them with MangoHud (MIT; built into SteamOS's
  performance overlay), by launching with `MANGOHUD_CONFIG` set to log
  frame times to a CSV. Run every bench scenario (3008) five times.
- **Power.** Read the battery draw from the performance overlay (level
  4) or MangoHud's battery field, on an idle battle map, for five
  minutes, with the screen at 50 % brightness and Wi-Fi on. Record the
  mean.
- **Start-up**, from launch to the title screen.
- **The report** in `docs/targets/steam-deck.md`, per scenario:
  - p50, p99 and the worst frame;
  - the power draw;
  - start-up;
  - the model and the SteamOS version.
- **Calibration.** Set the profile's budgets from the measurements, and
  set `status: supported` if all budgets hold.
- **If the idle draw is over 8 W.** Write a ticket. The likely fix is to
  present only when the frame changed, or to cap the frame rate on idle
  screens. 3043's background frame cap covers only a window without
  focus; an idle screen that has the focus needs that new ticket.

**Out (do not do):**
- optimizing (follow-up tickets);
- Deck Verified submission (the game's 0903).

## Implementation steps

1. Write the `steam-deck` run step, its tests and its docs, and start
   the example on the Deck with it.
2. Write the measuring steps in the docs, so they can be repeated.
3. Measure, report and calibrate.
4. Write the follow-up tickets for any misses.

## Acceptance criteria

- [ ] `docs/targets/steam-deck.md` has every scenario's numbers, the
      power draw, and the model.
- [ ] The budgets are set from the measurements, and the PR shows them.
- [ ] `shuyi export steam-deck --run --host` starts the example on the
      Deck, and its output reaches the terminal (Completion notes).
- [ ] Nick's sign-off is recorded.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the `scp` and `ssh` commands the run step builds from `--host`
  and the game's name, and the message when `ssh` is missing.
- Manual: the run on the Deck, and the measurements.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
