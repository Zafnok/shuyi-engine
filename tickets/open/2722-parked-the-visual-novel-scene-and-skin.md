---
id: "2722"
title: "Parked: The visual novel scene and skin: backgrounds, layered characters, the text box, choices and transitions"
type: feature
milestone: "Engine: visual novels"
model: reasoning
effort: high
status: blocked
blocked_by: ["2721", "2109", "2338", "2340"]
nick_input: none
completed:
---

# 2722 — The visual novel scene and skin

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0016](../../docs/adr/0016-visual-novels-and-detective-games-a-story-stage-a-case-file-and-a-fair-play-check.md)
§4: the story is drawn from a scene that is plain data, and a skin
paints it ([ADR-0002](../../docs/adr/0002-inherited-rules-layers-determinism-skins-actions-tests-licenses.md)
§4, `AGENTS.md` rule 9). Text, the name box and choices are glyph cells
([ADR-0003](../../docs/adr/0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md)
§5), so they use the engine's text by key, fonts and languages.
2224's ADR replaces §5's last line: cells stay the default, and text off
the grid is allowed. Pictures are sprite items in screen space.

2721 built `VnState`, `VnRules`, the stage directions and the events
they emit, with each transition and duration carried in the events.
This ticket turns state and events into a `VnScene`, paints it, and
gives the story a screen a player can read and choose in.

Time on the screen is presentation. Turn stages animate with the
frame's `dt` (ADR-0003 §2), so text appearing and transitions in
progress live beside the story, never in `VnState`.

## Nick input

None.

## Scope

**In:**
- **`VnScene`** in `shuyi_vn::view::scene` (behind the `view` feature),
  plain data with integers only:
  - the background, and the one it is changing from while a transition
    runs;
  - each character on screen: its layers in draw order (picture and
    offset, from `CharacterDef`), its place in screen pixels, its draw
    order;
  - the event picture, if one is showing (it covers the background and
    the characters);
  - the text box: the speaker's name key and colour role, the line's
    text key and arguments, how many glyphs are revealed out of how
    many, and whether the line has been read (`false` until 2723 says
    otherwise);
  - the choices: text key, enabled or not, which one is selected;
  - the transitions in progress: what they apply to, their kind, their
    ease, and their progress in thousandths;
  - the effects in progress (`shake` and `flash`, each with its progress
    in thousandths), all integers, for the dump: 2340 draws them (see
    `VnSkin`).
  - A text dump, one item per line, for snapshot tests and agents.
- **`VnPresenter`** in `shuyi_vn::view::present`, the time-dependent
  part outside `VnState`:
  - `on_events(&[VnEvent])` starts transitions, effects and waits, and
    resets the text reveal for a new line;
  - `tick(dt_ms: u32)` moves them on, in whole ms;
  - **text reveal** is the engine's dialogue text box, not a second
    one: the presenter holds a `shuyi_ui::dialogue::TextBox` for the
    line. It wraps once, on the full text (`TextBox::new`), so words
    don't jump between rows as they appear; it pages a line longer than
    the box; it types with `type_on`, which `tick` hands `dt_ms` as
    seconds (presentation may use floats, ADR-0003 §2), and hurries
    while Confirm is held (`hold`). The speed is the player's one text
    speed, `ctx.settings().text_speed` (`shuyi_ui::settings::TextSpeed`,
    already a row of the options screen), through `chars_per_s`: no
    `text_speed` in `vn.ron`. `VnScene` still carries the count as an
    integer (`TextBox::shown`);
  - `on_advance() -> Option<VnCommand>`: the first press is the text
    box's `confirm` (it finishes the reveal, then turns a page) and
    finishes any running transition; the press that finds the box
    `Step::Done` returns `Advance`. A `wait` holds `Advance` until it
    has passed, unless the player presses again;
  - **`instant`**: when set, transitions, waits and the reveal finish at
    once, so tests read end states (ADR-0016 §4). It is also on while
    idle motion is off (2339's rule; `VnStage` reads `ctx.idle_motion`),
    so a Harness, where idle motion is off, reads end states without
    setting it.
- **`build_scene(state: &VnState, presenter: &VnPresenter, story:
  &Story) -> VnScene`**, a pure function.
- **Transitions as data** (cut, fade, dissolve, slide, a game's own,
  from `vn.ron`):
  - `cut`, `fade` and `dissolve` are 2339's looks by name
    (`TransitionLook::name()`), the names a cinematic shot's `enter`
    uses too (2364): use those names, and define no others for them.
    2339's `compose` changes a whole frame, and its Out leaves a
    picture's own transition to this ticket, so the VN draws these
    itself: on one picture (a background, a character, the event
    picture), or a `fade` of the whole scene through 2339's veil (see
    `VnSkin`). `slide` and `custom` stay the VN's own;
  - `fade` goes through the skin's backdrop colour; `dissolve` draws the
    old and new pictures with complementary alpha; `slide` moves the new
    picture in from its side;
  - each transition may name an `ease` in `vn.ron`, a
    `shuyi_core::ease::Ease` (2109), `Linear` when left out. `VnScene`
    carries it beside the progress in thousandths, which stays linear in
    time, and the skin eases with it (`shuyi_ui::ease::ease_f32` or the
    integer curve);
  - a `custom` transition is passed to the skin by name, progress and
    ease. The default skin draws it as `dissolve`, and says so in its
    docs. A game's skin may draw it its own way.
- **The text box layout as data** in `vn.ron` (`layout`): the text box,
  the name box and the choice list as rectangles in cells, with
  defaults for the engine's default frame size. It may name the frame
  their panels wear (a `GameData::nine_slice` name, 2338) and, if 2334
  is done, their font.
- **`VnSkin`** in `shuyi_vn::view::skin`: paints a `VnScene` into the
  frame:
  - pictures as sprite items in screen space, in draw order;
  - the text box, name box and choices as glyph cells, with text by key
    through the language pack, and the speaker's name in their colour
    role, inside panels drawn with 2338's `GlyphBuffer::panel_border`,
    so their border is the game's picture frame when it has one
    (`Ctx::panel_frame`, or a frame that `vn.ron`'s `layout` names);
  - **Either order with 2334:** if 2334 is done and `vn.ron` names a
    font (an id in `GameData::fonts`), the text box, name box and
    choices are drawn as text runs (2334's `TextLabel`) in pixel
    rectangles, not in cells, and the reveal still wraps once, on the
    full text; with no font, cells as above; if 2334 isn't done, cells
    as above, and 2334 adds the font and the runs when it lands;
  - transitions as the scene says. `shake` and `flash` are not painted
    by the skin: `VnStage` asks the frame for them through 2340's
    `Ctx::shake` and `Ctx::flash` when `on_events` starts them, so they
    follow the player's motion, shake and flashing settings and every
    painter (the one-bit one included) draws them. A `fade` of the
    whole scene may draw through 2339's veil. `VnScene` still lists the
    effects in progress for the dump.
  - It reads nothing of the story but the scene.
- **`VnStage`**, a screen (ADR-0003 §1) that owns a `VnRules` value and
  a `VnPresenter`:
  - it reacts to `UiAction`s only (the `input-actions` skill): Confirm
    advances through `on_advance`, Up and Down move the selected choice,
    Confirm picks it. It declares them through `Screen::actions`
    (3002);
  - it never names a key or a button, in code or in text;
  - it returns the story's outcome to the flow screen when the story
    ends.

**Out (do not do):**
- the text speed row in the quick menu, auto, skip, the backlog, read
  lines, the going-back action, and the vn's own actions (2723);
- a text speed of the VN's own: it is the engine's `TextSpeed`;
- save thumbnails, the gallery screen and the route chart screen
  (2724);
- mouths and blinking (2725);
- pointer input to advance and choose (2723 adds it through 2405);
- 1-bit pictures for the Playdate (3007 bakes them; nothing here is
  special to one target);
- video: opening movies, moving backgrounds and movie event pictures
  (2909 researches how video plays; the tickets it writes add any scene
  item or stage direction for it);
- the example (2726).

## Implementation steps

1. Write `VnScene` and its dump.
2. Write `VnPresenter` with `on_events`, `tick`, `on_advance`,
   `instant`, and the line's `TextBox` at the player's `TextSpeed`,
   with unit tests that feed events and ms by hand.
3. Write `build_scene`, with snapshot tests on 2721's fixture story at
   each stop, with `instant` on.
4. Add `layout` (with its frame name, and its font if 2334 is done) and
   each transition's `ease` to `vn.ron` (update its fixtures and
   determinism hash).
5. Write `VnSkin`: its panels through `panel_border`, its transitions
   with their eases, and the whole-scene fade through the veil (and the
   text runs, if 2334 is done).
6. Write `VnStage`, with its asks for shake and flash and `instant`
   while idle motion is off, and a Harness test that plays 2721's
   fixture story to an ending through actions.

## Acceptance criteria

- [ ] `build_scene` is pure: the same state and presenter give the same
      scene (property test).
- [ ] With `instant` on, each stop of the fixture story matches its
      checked-in scene snapshot.
- [ ] Revealed glyphs never decrease while ticking, never pass the
      page's total, and reach it within one tick of total × 1000 /
      `chars_per_s` ms (property test). No line's wrapping changes as it
      is revealed.
- [ ] The first `on_advance` during a reveal returns `None` and finishes
      it; on a one-page line the next returns `Advance`. Changing the
      player's `TextSpeed` changes the reveal; there is no other text
      speed setting.
- [ ] Each transition kind's progress runs from 0 to 1000 over its
      duration, whatever its ease, and `instant` jumps it to 1000.
- [ ] With idle motion on, a `shake` and a `flash` reach the frame
      through 2340's `Ctx::shake` and `Ctx::flash` (its `shake` and
      `veil` snapshot lines); with idle motion off the presenter is
      `instant`.
- [ ] One look test reads the frame's cells and sprite items for a line
      with a speaker and for a choice list, and checks the name box,
      text box and choice cells against the layout (`AGENTS.md` rule 9:
      only this test reads cells and sprite items).
- [ ] A Harness test plays the fixture story to an ending through
      `UiAction`s only, and a `frame-png` picture of one stop is checked
      in.
- [ ] `check-keys` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the presenter's reveal, waits, transitions (with an ease) and
  `on_advance`; `instant` while idle motion is off.
- Property: `build_scene` purity; reveal bounds; transition progress
  bounds.
- Snapshot: the scene dump at every stop of the fixture story.
- Look: one test of cells and sprite items.
- Integration: the Harness playthrough, the shake and flash reaching
  the frame, and the `frame-png` picture.

## Completion notes
