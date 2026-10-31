---
id: "2380"
title: The profiles screen asks the system keyboard for a name in place of the letter grid, and checks the answer with the name rules
type: feature
milestone: After MVP
model: implementation
effort: medium
status: todo
blocked_by: ["2346"]
nick_input: none
completed:
---

# 2380 — The profiles screen asks the system keyboard for a name

## Context

- A follow-up of 2346, which leaves it out (its **Out**: "The profiles
  screen asking the system keyboard ... and checking its answer with
  the same rules"). No friction review: a missing feature.
- **The handshake exists** (3003, done; ADR-0024 §2, `TextInput`).
  `Ctx::system_keyboard` (`crates/ui/src/system_keyboard.rs`) has
  `is_present`, `ask(TextAsk { prompt, text, max_chars }) -> bool`,
  `is_waiting` and `take_answer`, which gives `TextAnswer::Typed(text)`
  (the whole text) or `TextAnswer::Cancelled`. `App`'s `step`
  (`crates/platform/src/app.rs`) opens the platform's `TextInput` after
  the frame that asked, and hands the answer back before a later frame.
  It answers `Cancelled` when the keyboard can't open or goes away.
- **The headless backend has one for tests:** `ScriptedKeyboard`
  (`crates/platform/src/headless.rs`) answers from a script, one answer
  per opening, and keeps every ask (`asks()`).
  `HeadlessPlatform::new().with_keyboard(..)` gives it to a backend, and
  `Harness::on(game, platform)` runs a game on that backend.
- **No screen asks it yet.** The profiles screen (2310, done;
  `crates/ui/src/screens/profiles.rs`) opens a `NameBox` (typed keys)
  when a key opened the box, or a `NameEntry` (the letter grid, read
  with actions) when a controller button did (`open_name`, its `pad`
  argument). So on a device whose only keyboard is the system's (Steam's
  on the Steam Deck, a console's), a player spells a profile's name one
  letter at a time on the grid.
- Deck Verified asks for the on-screen keyboard where the player types
  (3012's Context; ADR-0008, *The targets*). A game's Steam build fills
  `TextInput` with Steam's keyboard
  (`shuyi_app::AppConfig::text_input`), and 3012 rules out a keyboard
  drawn by the engine.
- **The rules must hold for a whole text.** After 2346, `NameRules`
  takes any letter the font draws (`Ctx::font_chars`), `-`, `'`, digits
  where the rules allow them and single spaces between words, counts the
  length with `shuyi_assets::lang::text_width`, and checks each typed
  character as it comes (`NameRules::allows`). A system keyboard answers
  a whole text in any script, past the box's limits if it likes. Kept
  unchecked, a name the font can't draw, or one too long, gets in.
- Any game with the engine's profiles screen gets this, whatever its
  style: a farming game on a handheld, where the family names its
  profiles with a pad.

## Nick input

None.

## Scope

**In:**

1. **A whole name checked, and an ask for one**
   (`crates/ui/src/widgets/name_entry.rs`), public so a game's own name
   box can do the same:
   - `NameRules::accepts<G: GameData>(self, ctx: &Ctx<G>, text: &str)
     -> Option<String>`: the name `text` makes if the rules allow it
     whole. It is `finished(text)` (trimmed, not blank), with every
     character allowed in turn by the same rule a typed character meets
     (`allows`, with what 2346 gives it: the font's set and the length
     by `text_width`). So "  Zoë " gives "Zoë", and "Kai!",
     "Ann  Lee", a blank text, 13 letters for 12, and a letter the
     font doesn't draw give `None`. There is one rule for a
     character, not a copy.
   - `NameRules::text_ask(self, prompt: &str, name: &str) -> TextAsk`:
     the ask for a name, with `prompt`, the name so far, and
     `max_chars` as a `u16` (`u16::MAX` when it doesn't fit). A
     keyboard counts characters and `max_chars` counts cells, so a name
     of wide letters can fit the keyboard and still be refused by
     `accepts`. The doc comment says so.
2. **The profiles screen asks** (`crates/ui/src/screens/profiles.rs`):
   - A controller button that opens the name box (Add, or Rename) opens
     the letter grid as today, then asks the system keyboard with
     `NAME_RULES.text_ask`: the prompt is `ctx.text(NAME_BOX)` ("Profile
     name"), the text is the name the box opens with. No new text key.
   - `ask` is `false` without a system keyboard, and then nothing else
     changes: the grid is there, as today.
   - A key that opens the box opens the typing box as today and asks
     nothing, even with a system keyboard: a player who pressed a key
     has keys.
   - While the ask waits, the screen reads no actions: the system's
     keyboard has the player. Each frame it takes the answer, if one
     came. The frame that brings the answer uses none of its keys.
   - `Typed(text)` that `accepts` allows: the grid's name becomes it,
     Select sounds, and the name is kept as Done on the grid keeps it
     (`keep_name`): Add adds the profile and uses it, then closes or
     opens what the game gives (`opening_after_add`); Rename renames
     it. A name the profiles refuse (another profile's, say) gets its
     message as today, and the grid stays open with that name.
   - `Typed(text)` that `accepts` refuses: Denied sounds, and the grid
     stays open with the name as it was.
   - `Cancelled`: nothing sounds (the system's keyboard sounds its own
     closing), and the grid stays open with the name as it was. The
     player spells on it, or closes it with Cancel as today. This is
     also what happens when the keyboard couldn't open, so the grid is
     always the way back.
   - `ProfilesScreen::asking(&self) -> bool`: whether the name box
     waits for the system keyboard, for tests and a game's painter.
   - The view, the help line and the snapshots don't change.
3. **Docs** (the `document` skill): the module docs of
   `screens/profiles.rs` (a name is typed, spelled on the grid, or
   typed on the system's keyboard when a controller opens the box),
   `widgets/name_entry.rs` (the two new functions, for a game's own
   box) and `system_keyboard.rs` (its first paragraph names the
   profiles screen as a screen that asks).

**For the game:** additive. `NameRules` gains two methods and
`ProfilesScreen` one; nothing a game builds or matches changes, so no
`Breaking:` line. In a build that fills `TextInput`, a controller that
opens a profile's name box now opens the system's keyboard. The game's
PR that moves its pin follows this one.

No new public type (ADR-0039) and no new dependency.

**Out (do not do):**
- A keyboard drawn by the engine (3012's Out).
- The Steam Deck target, and Steam's keyboard behind `TextInput`: 3012
  and a game's Steam build. This ticket tests on the headless
  backend's `ScriptedKeyboard` and needs neither.
- Asking the system keyboard when a key, a mouse click or a touch opens
  the box. Only a controller button asks (In, 2).
- A grid cell or an action that asks the keyboard again after a refused
  answer or a cancel: a follow-up if a game asks. Today the player
  closes the box and opens it again.
- A game's own name box: it asks the same way with `accepts` and
  `text_ask`, and isn't changed here.
- The IME, the letters a name may hold, the grid's pages and the font's
  set: 2346's.
- What the system keyboard's answer looks like in a recorded session or
  in the contract: the contract's game has no system keyboard, so
  nothing changes there.

## Implementation steps

1. Read `screens/profiles.rs` as 2346 left it (`open_name` with a
   `ctx`, the grid's pages), its tests, `widgets/name_entry.rs`
   (`NameRules::allows` and `Ctx::font_chars` as 2346 left them) and
   its tests, `system_keyboard.rs`, `ScriptedKeyboard` and
   `HeadlessPlatform::with_keyboard` in
   `crates/platform/src/headless.rs`, `Harness::on` in
   `crates/platform/src/harness.rs`, and
   `examples/hello-grid/tests/it/profiles.rs`.
2. `NameRules::accepts` and `NameRules::text_ask`, with doc comments,
   and their unit tests in `crates/ui/src/widgets/name_entry/tests.rs`.
3. The profiles screen:
   - `Naming` gains `asked: bool`: the system keyboard was asked for
     this box and hasn't answered;
   - `open_name` takes `ctx: &mut Ctx<G>` (2346 gives it `&Ctx<G>`;
     its callers `confirm` and `act` have a `&mut`). When `pad`, it
     takes the prompt as an owned `String` first (the ask borrows
     `ctx` mutably), then sets `asked` from `ctx.system_keyboard.ask`;
   - `update`, before the typing branch: while `asked`, take the
     answer, and return `Transition::None` if none came. Otherwise
     clear `asked` and handle it in a new private method, which
     rebuilds the grid with the kept name as `open_name` builds it
     (`NameEntry::new` and 2346's pages) before calling `keep_name`;
   - `ProfilesScreen::asking`.
4. The unit tests in `crates/ui/src/screens/profiles/tests.rs`. They
   play the platform's part by hand:
   `c.system_keyboard.set_present(true)` before the frames,
   `take_ask()` after the frame that asked, then
   `answer(..)` before the next frame. Sounds are read with
   `c.audio.take()`.
5. Hello, Grid's tests: a helper in `tests/it/profiles.rs` that builds
   `Harness::on(hello_grid::game(hello_grid::ctx(0)),
   HeadlessPlatform::new().with_keyboard(ScriptedKeyboard::new(..)))`,
   then the tests below.
6. The module docs (In, 3).
7. Run the gates.

## Acceptance criteria

- [ ] `NameRules::accepts` gives "Zoë" for "Zoë" and "  Zoë ", and
      `None` for "Kai!", "Ann  Lee", "", "   ", 13 letters with
      `max_chars` 12, digits where the rules refuse them, and "Zoë"
      with `Ctx::font_chars` lacking `ë` (unit tests).
- [ ] `NameRules::text_ask` carries the prompt, the name and
      `max_chars`, and gives `u16::MAX` for a `max_chars` past it
      (unit test).
- [ ] With a system keyboard, a controller's Confirm on Add asks once,
      with the prompt "Profile name", the text "" and `max_chars` 12;
      Rename asks with the profile's name (unit tests).
- [ ] While the ask waits, a controller's Confirm and Cancel change
      nothing: the name, the open box and the sounds stay as they were,
      and `asking()` is true (unit test).
- [ ] An answer `accepts` allows is kept: on Add the profile is added
      and in use and the screen closes (or opens what the game gives),
      on Rename the profile is renamed, and Select sounds (unit tests).
- [ ] An answer naming another profile gets the name-taken message,
      and the grid stays open with that name (unit test).
- [ ] An answer `accepts` refuses sounds Denied and leaves the grid open
      with the name as it was; `Cancelled` sounds nothing and leaves it
      the same, and Cancel then closes it as today (unit tests).
- [ ] A key's Confirm on Add opens the typing box and asks nothing with
      a system keyboard present; without one, a controller's Confirm
      opens the grid and asks nothing (unit tests).
- [ ] In Hello, Grid on a `ScriptedKeyboard` answering `Typed("Zoë")`,
      a profile added from the title with the controller is "Zoë", in
      use, and the keyboard's `asks()` holds the one ask
      (`examples/hello-grid/tests/it/profiles.rs`).
- [ ] In Hello, Grid on a `ScriptedKeyboard` answering `Typed("Kai!")`,
      Denied sounds (`Harness::sounds`), the grid is open with nothing
      typed, and the profiles are unchanged; answering `Cancelled`, the
      grid is open and the controller's Cancel closes it.
- [ ] Hello, Grid's existing profile tests (no system keyboard) pass
      unchanged, and no snapshot changes.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `accepts` (allowed, trimmed, refused for each rule, the font's
  set) and `text_ask`; the profiles screen's ask on Add and Rename, the
  frames while it waits, each kind of answer, a taken name, a key that
  opens the typing box, and no system keyboard.
- Property: whatever text the system keyboard answers, a name `accepts`
  keeps is trimmed, not blank, fits `max_chars` by `text_width`, and
  typed into a `NameBox` keeps every character with no Denied sound.
- Integration: Hello, Grid through the Harness on a `ScriptedKeyboard`
  (kept, refused, cancelled), and its existing profile tests with no
  system keyboard.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
