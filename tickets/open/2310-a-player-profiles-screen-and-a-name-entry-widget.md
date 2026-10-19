---
id: "2310"
title: "A player profiles screen (list, use, rename, delete, add) and a name entry widget, as engine screens like options"
type: feature
milestone: After MVP
model: opus-5.5
effort: medium
status: todo
blocked_by: ["2309"]
nick_input: none
completed:
---

# 2310 — A player profiles screen and a name entry widget

## Context

- **The game ticket:** game#1164
  ([`1164-the-profile-screen-from-the-engine.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1164-the-profile-screen-from-the-engine.md))
  waits for this one. Found by the game's first engine-friction sweep,
  game#1153.
- **The job:** "let the people who share a computer each keep their own
  settings and saves under a name".
- **Friction:** the game's profile screen is 831 production lines
  (logic 557, view 54, glyph skin 140, one-bit skin 80) against an
  estimate of 100 (8.3×), outcome 1: it names nothing of the game. It
  uses only the engine `Ctx`'s profile functions, the engine's
  `LeaveChoice`, and the game's name entry widget (229 lines of logic,
  57 of glyph skin, 57 of one-bit skin), which names nothing of the game
  either. It also copies two private pieces of the engine (the options
  screen's question painter and `stepped`). The estimate is a game's
  20-line opener, like the game's 27-line `key_bindings.rs` over the
  engine's Key bindings screen, plus the game's one-bit skin.
- **The generic shape:** `screens/profiles.rs`: the list of profiles
  with the current one marked, an actions pop-up (use, rename, delete),
  "add a profile", a name box, and questions before deleting, on the
  engine's `Ctx` profile functions; and `widgets/name_entry.rs`, a
  one-line text box that takes typed characters or picks letters from a
  grid with the cursor keys (for a pad or a device with no keyboard).
- **Outside tactics:** every game that lets a family share one copy;
  the name box is a farming game's "name your farm".

## Nick input

None.

## Scope

**In:**
- Move the game's `crates/ui/src/screens/profile.rs` (with its view
  and glyph skin) and `crates/ui/src/widgets/name_entry.rs` (with its
  glyph skin) into the engine, as tickets 1112 and 1155 of the game did
  for options and Key bindings: generic over `G: GameData`, painted by
  a `Paint<G>` the game can replace (`painted_by`, like
  `OptionsScreen`), text from the engine's `en` text keys.
- Its questions use the question widget (2309, needed first). If 2307
  is done, its list uses the list cursor.
- One-bit skins stay the game's (they are its look); the screen takes a
  painter, so a game passes its own.
- The smallest example game opens it from its title.

**Out (do not do):**
- What a profile holds (the engine's `Ctx` profile functions stay as
  they are).
- Any game's one-bit skins.

**For the game:** additive. The game's PR that moves its pin follows
this one and deletes its copy (game#1164).

## Implementation steps

1. Read the game's screen and widget (at the game's `main`) and their
   tests; read how `screens/options.rs` takes a painter.
2. Move them, renaming what names the game; carry their tests.
3. Text keys in the engine's `en` table.
4. The example.

## Acceptance criteria

- [ ] The moved tests pass in the engine (adding, renaming, deleting
      the current profile, the name box by keys and by grid).
- [ ] `grep` finds no game id or game text key in the moved files.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit and snapshot: carried over from the game, plus the example
  opening the screen.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
