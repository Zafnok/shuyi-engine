---
id: "3098"
title: "Parked: Research: PICO-8 as a tier 3 target, what a cartridge can hold and how a Shuyi game gets there"
type: research
milestone: "Parked: export targets"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3004", "3011", "3013", "3014", "3015", "3016", "3022", "3026", "3002", "3019", "2225", "2739"]
nick_input: decision
completed:
---

# 3098 — Research: PICO-8 as a tier 3 target

> **Parked** (Nick, 2026-11-08: "lets add PICO-8 support parked for after all tier 1 and tier 2 release targets PICO-8 can be a tier 3 release target"): until Nick unparks it, and not before every tier 1 and tier 2 target is done. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

On 2026-11-08 Nick ordered the export targets in tiers
([`docs/decisions.md`](../../docs/decisions.md), *the export targets in
tiers, and PICO-8 as tier 3*):

- **tier 1**: Windows, Linux, macOS and the web (3004);
- **tier 2**: the Steam Deck (3012, 3013), the Playdate (3010, 3011),
  the Switch (3014), Android and iOS (3015), PlayStation and Xbox
  (3016), the GBA (3021, 3022) and the DS (3024, 3026). The Playdate is
  the first of them being worked, but it is still tier 2;
- **tier 3**: PICO-8, this ticket.

`blocked_by` holds the last ticket of each tier 1 and tier 2 target, so
this one waits for all of them as well as for Nick. It also holds what
the research builds on: the controls rule (3002), indexed colour (3019),
sounds as a few numbers (2225) and the `snake` example (2739).

PICO-8 is Lexaloffle's fantasy console
(<https://www.lexaloffle.com/pico-8.php>, manual at
<https://www.lexaloffle.com/dl/docs/pico-8_manual.html>). Unlike every
other target, it is not hardware a Rust program runs on: a game is a
cartridge of Lua code and data that PICO-8 itself runs. What follows was
known on 2026-11-08 and must be re-checked in the manual before anything
relies on it:

- a 128 by 128 screen with a fixed palette of 16 colours (and a second,
  hidden 16);
- six buttons per player: the d-pad, O and X, plus a pause menu;
- 30 or 60 frames a second (`_update` or `_update60`);
- a cartridge holds its code within limits on tokens (8,192),
  characters and compressed size, plus 32 KB of data: sprites, a map,
  sound effects and music;
- 2 MB of Lua memory, and numbers that are 16.16 fixed point, which
  suits the core's no-floats rule (ADR-0002);
- PICO-8 costs money per developer, and its exporters write HTML and
  desktop binaries as well as `.p8` and `.p8.png` cartridges.

So a Shuyi game's Rust rules can't simply be built for PICO-8. The
research decides whether any useful part of a Shuyi game can get there,
and how. Other tickets already borrow PICO-8's ideas without targeting
it, such as sounds as a few numbers (2225).

## Nick input

**Decision** (the `ask-nick` skill), after the research. The session
tells Nick in plain words which way it recommends, what a game made
that way can and can't be, and what it costs:

- Go ahead with the chosen way, or drop PICO-8.
- PICO-8 itself must be bought by each developer who exports, and by
  whoever runs the checks. Is that cost acceptable for the engine?

Record the answer in `docs/decisions.md`.

## Scope

**In:**
- **The facts**, each re-checked with its source and date: the limits
  above, the cartridge formats (`.p8` text and `.p8.png`), the
  exporters, and what PICO-8's license lets a developer sell.
- **The ways a game could get there**, each judged on what kind of
  game survives it, how many lines the engine adds, and whether its
  output stays deterministic and testable:
  1. **Rules compiled to Lua**: a game's tick rules (ADR-0002) turned
     into PICO-8 Lua. Measure the tokens one real example costs
     (`snake`, from 2739, and `hello-grid`, game#1118) against the 8,192
     limit.
  2. **A data cartridge**: a runtime written once in PICO-8 Lua, per
     game style, that plays a game's data (levels, sprites, sounds,
     text) baked from its source assets. The game's Rust rules don't
     go; only a game whose rules the style's runtime already has does.
  3. **A PICO-8 look only**: a `pico-8` profile (ADR-0008) that holds a
     desktop or web build to PICO-8's screen, palette, buttons and
     sound, without a cartridge. Say whether this alone is worth having.
  4. any other way found, such as running a wasm module from Lua.
- **The checks without PICO-8.** Whether CI can test a cartridge with
  a free, permissively licensed player or emulator, run as a separate
  program and never linked or shipped (ADR-0002 §8), or must trust the
  baked data alone. Name each candidate and its license.
- **The bake.** For the way chosen: how sprites go to the 16-colour
  palette (with 3019's indexed colour), how a `.sfx.ron`
  sound (2225) goes to PICO-8's sound effects, and the controls rule
  (3002) for six buttons.
- **The write-up**, `docs/targets/pico-8.md`: every fact with its
  source and date, each way and its verdict, the recommendation, and
  Nick's answer. Add its row to `docs/targets/README.md`.
- **The follow-up tickets** for the chosen way: the profile, the bake,
  the export (`shuyi export pico-8`) and the device check. Each is
  parked until Nick says go.

**Out (do not do):**
- building the profile, the bake or the export; the follow-ups do that;
- anything of Lexaloffle's in the repository: no PICO-8 binary, no
  cartridge from the BBS or anyone else's work, no font or palette file
  copied from PICO-8;
- buying PICO-8 for anyone;
- the tier 1 and tier 2 targets' own work.

## Implementation steps

1. Re-check the facts in the manual and Lexaloffle's pages; write them
   down with the date.
2. For way 1, write a scratch translation of one example's rules into
   Lua by hand and count its tokens with a free token counter; record
   the counter and its license.
3. For way 2, sketch the runtime for one game style and count its
   tokens.
4. Weigh way 3, and any other way found.
5. Find a free player for CI, or record that none fits.
6. Write `docs/targets/pico-8.md` and its row in `docs/targets/README.md`.
7. Ask Nick (the `ask-nick` skill) and record his answer.
8. Write the follow-up tickets, parked; if a fact or the chosen way is a
   technical decision, write the ADR (the `write-adr` skill).

## Acceptance criteria

- [ ] `docs/targets/pico-8.md` holds every fact with a source and a
      date, each way's verdict with its token counts, and the
      recommendation.
- [ ] Every tool named for the checks has its license recorded, and
      none denied by ADR-0002 §8 is linked or shipped.
- [ ] Nothing of Lexaloffle's is in the repository.
- [ ] Nick's answer is in `docs/decisions.md`.
- [ ] The follow-up tickets exist, parked, or the write-up says why
      there are none.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None beyond the gates. This ticket researches; the follow-ups test
  what it chooses.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
