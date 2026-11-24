---
id: "2903"
title: "Research: controllers after gilrs: our own code or a library, and the tickets that follow"
type: research
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: decision
completed: 2026-11-24
---

# 2903 — Research: controllers after gilrs

> Nick unparked this research on 2026-11-22 by asking to do ticket 2903 (`docs/decisions.md`). The build it picks (2402, 2403, 2408 and any ticket it writes) stays parked until the MVP.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): "The gilrs seems low stars and
barely maintained so I still plan to skip it... I feel like of
everything, controller is quite easy to add support for but this can be a
2 way door decision". The engine reads pads with gilrs on native
(`crates/app/src/pads/native.rs`) and with our own `gamepad.js` on the web.
`shuyi-ui`'s `Pads` holds all the rules, so the source can change without
touching them.

Facts on 2026-10-19, for whoever weighs this later: gilrs lives on GitLab
(`gitlab.com/gilrs-project/gilrs`), so GitHub's stars don't show it. It
had releases in January and May 2026 and commits into August 2026, and
Bevy uses it (`bevy_gilrs` 0.20). Re-check all of this on the day.

A plan for our own code is already written, parked behind this ticket:
the `shuyi-pads` crate with SDL's mapping database and Windows (2402),
then Linux and macOS (2403).
[ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§1 leaves the choice to this ticket.

## Nick input

**Decision:** once the options are weighed, ask Nick (the `ask-nick`
skill) which way to go, with a recommendation.

## Scope

**In:**
- **What the engine needs from pads**: the standard layout, hot-plugging,
  the pad's kind (for button names and the Switch-style swap), sticks and
  triggers as values (1121), rumble (2408, parked: gilrs's force feedback
  on Windows and Linux, the browser's `vibrationActuator` on the web), and
  what later work may want (motion, Steam Deck controls, mobile).
- **The options:**
  - our own code, as 2402 and 2403 plan;
  - keeping gilrs, with its health measured on the day (releases, open
    issues, response times, who maintains it);
  - other libraries maintained on the day (SDL3's gamepad API through
    bindings, under Zlib, would bring a C library; others as found). For
    each: license (on the day), what it pulls in, platforms, force feedback
    per platform, and whether it needs `wasm-bindgen` (the web keeps its
    own path either way).
- **Test on real pads** where possible: an Xbox pad, a PlayStation pad, a
  Switch Pro pad, the Steam Deck.
- **Outputs:**
  - a report, `docs/research/2903-controllers-after-gilrs.md`;
  - Nick's answer in `docs/decisions.md`;
  - an ADR if the choice adds a dependency or changes ADR-0021;
  - 2402 and 2403 kept, rewritten or closed (moved to `tickets/done/` with
    a completion note saying why).

**Out (do not do):**
- Building the replacement.
- Reading the source of gilrs, SDL or any other library to copy it
  (ADR-0021 §2). Reading documentation to judge them is fine.

## Implementation steps

1. List the needs from the code, 1121 and the input tickets.
2. Survey the options and fill a table in the report.
3. Try the shortlist on real pads, in a scratch folder.
4. Ask Nick, with a recommendation, and record his answer.
5. Update or close 2402 and 2403, and `docs/ROADMAP.md`'s order.

## Acceptance criteria

- [x] The report covers each option, with the facts dated.
- [x] Nick's answer is in `docs/decisions.md`, verbatim.
- [x] 2402 and 2403 match the answer, and the ticket lint passes.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- None shipped (research). The tickets it keeps or writes say their tests.

## Completion notes

Nick unparked this research alone on 2026-11-22 ("Unpark 2903 only";
`docs/decisions.md`), and chose on 2026-11-24 "sdl3 with a fix for
switch, or write our own. if no fix for switch can exist with sdl3, then
write our own" (verbatim there).

What was done:

- The report, [`docs/research/2903-controllers-after-gilrs.md`](../../docs/research/2903-controllers-after-gilrs.md):
  the engine's needs from pads, gilrs's, SDL3's and our own code's facts
  dated 2026-11-22 (crates.io, GitLab's and GitHub's APIs), the pads'
  results, build times and sizes, and the answer.
- A scratch probe (outside the repository) ran gilrs 0.11.2 and SDL
  3.4.20 as black boxes on Nick's Xbox Series X, DualSense and Switch 2
  Pro pads on Windows, by USB. SDL3 read the Xbox and DualSense pads
  completely; gilrs missed the Xbox logo button and didn't list the
  DualSense; neither read the Switch 2 pad.
- The Switch fix was tried: switchbrew.org's documented "Initialize"
  command, sent by a scratch program through `nusb`, made the Switch 2
  pad report all 17 buttons through gilrs. So the answer's first branch
  holds: SDL3, with our own start-up for Switch 2 pads.
- [ADR-0146](../../docs/adr/0146-desktop-pads-through-sdl3-with-our-own-start-up-for-switch-2-pads.md)
  records it, resolving ADR-0021 §1's controllers choice and narrowing
  its §4 and §6; ADR-0021's status line and the ADR index say so.
- 2402 is rewritten (desktop pads through SDL3, gilrs removed) and 2403
  (the Switch 2 start-up in `shuyi-pads`); both stay parked, with new
  file names. 2408, 1121, 2406, 2008, 3034 and `docs/ROADMAP.md` follow.
- New ticket 2414 (not parked) tries both libraries on the Steam Deck,
  and 2402 waits for it.
- Nick then asked for controller support on desktop to be a toggle, on
  by default, to save the 1.8 MB: ADR-0146 §4 and 2402's Scope. And he
  made it a rule: an optional dependency that makes games bigger comes
  behind a toggle (`docs/decisions.md`, 2026-11-24; `AGENTS.md` rule 13).
- Merged `main` after 3034 landed: 3327 and 3333 now name SDL3 for the
  native pads, and the web's pad reading moves to `web-sys` with 3331.

Deviations:

- The Steam Deck wasn't tried. Nick: "rn we leave untested but have it
  as a followup", so ticket 2414 does it.
- The ticket named a Switch Pro pad; Nick's is a Switch 2 Pro pad, which
  turned out to be the hard case. No Switch (1) Pro pad was tried.
- No Bluetooth, Linux or macOS runs: Windows by USB only. 2402's and
  2403's sign-offs cover the rest.
- SDL3 didn't list the started Switch 2 pad with
  `SDL_JOYSTICK_HIDAPI_SWITCH2=0`; other hints couldn't be tried with the
  pad plugged in. 2403's Scope finds the reading path per platform.
- The probe needed CMake on Nick's machine: installed through MSYS2
  (`mingw-w64-x86_64-cmake`), with his permission.
- No library's source was read. The probe called each library's public
  API, and the Switch 2 command came from switchbrew's protocol page.

No dependency, code or public API changes. Breaking: none.
Book: none, no public API changed.
Game: nothing to adopt, the research changes no engine code; 2402's PR
says what the game's pin bump needs (CMake on its machines).

Nick input: unparked 2903 alone; chose SDL3 with a fix for Switch;
asked for the Steam Deck as a follow-up (2414); made desktop controller
support a toggle (on by default), and toggles for size-adding optional
dependencies a rule.
