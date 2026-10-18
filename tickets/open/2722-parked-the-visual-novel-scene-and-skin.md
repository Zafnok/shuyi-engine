---
id: "2722"
title: "Parked: The visual novel scene and skin: backgrounds, layered characters, the text box, choices and transitions"
type: feature
milestone: "Engine: visual novels"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2721"]
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
Pictures are sprite items in screen space.

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
  - the transitions in progress: what they apply to, their kind, and
    their progress in thousandths;
  - the effects in progress (`shake` with its offset, `flash` with its
    progress), all integers.
  - A text dump, one item per line, for snapshot tests and agents.
- **`VnPresenter`** in `shuyi_vn::view::present`, the time-dependent
  part outside `VnState`:
  - `on_events(&[VnEvent])` starts transitions, effects and waits, and
    resets the text reveal for a new line;
  - `tick(dt_ms: u32)` moves them on, in whole ms;
  - **text reveal**: glyphs revealed = elapsed ms × glyphs per second /
    1000, in integers. The line is wrapped once, on its full text, so
    words don't jump between rows as they appear. The speed is a field
    with a default from `vn.ron` (`text_speed`); 2723 makes it the
    player's setting;
  - `on_advance() -> Option<VnCommand>`: the first press finishes the
    reveal and any running transition; the next one returns `Advance`.
    A `wait` holds `Advance` until it has passed, unless the player
    presses again;
  - **`instant`**: when set, transitions, waits and the reveal finish at
    once. The Harness sets it, so tests read end states (ADR-0016 §4).
- **`build_scene(state: &VnState, presenter: &VnPresenter, story:
  &Story) -> VnScene`**, a pure function.
- **Transitions as data** (cut, fade, dissolve, slide, a game's own,
  from `vn.ron`):
  - `fade` goes through the skin's backdrop colour; `dissolve` draws the
    old and new pictures with complementary alpha; `slide` moves the new
    picture in from its side;
  - a `custom` transition is passed to the skin by name and progress.
    The default skin draws it as `dissolve`, and says so in its docs. A
    game's skin may draw it its own way.
- **The text box layout as data** in `vn.ron` (`layout`): the text box,
  the name box and the choice list as rectangles in cells, with
  defaults for the engine's default frame size.
- **`VnSkin`** in `shuyi_vn::view::skin`: paints a `VnScene` into the
  frame:
  - pictures as sprite items in screen space, in draw order;
  - the text box, name box and choices as glyph cells, with text by key
    through the language pack, and the speaker's name in their colour
    role;
  - transitions and effects as the scene says.
  - It reads nothing but the scene.
- **`VnStage`**, a screen (ADR-0003 §1) that owns a `VnRules` value and
  a `VnPresenter`:
  - it reacts to `UiAction`s only (the `input-actions` skill): Confirm
    advances through `on_advance`, Up and Down move the selected choice,
    Confirm picks it. It declares them through `Screen::actions` if 3002
    is done;
  - it never names a key or a button, in code or in text;
  - it returns the story's outcome to the flow screen when the story
    ends.

**Out (do not do):**
- text speed as a player setting, auto, skip, the backlog, read lines,
  the going-back action, and the vn's own actions (2723);
- save thumbnails, the gallery screen and the route chart screen
  (2724);
- mouths and blinking (2725);
- pointer input to advance and choose (2723 adds it if the parked 1121
  is done);
- 1-bit pictures for the Playdate (3007 bakes them; nothing here is
  special to one target);
- the example (2726).

## Implementation steps

1. Write `VnScene` and its dump.
2. Write `VnPresenter` with `on_events`, `tick`, `on_advance`,
   `instant`, and the wrap-once text reveal, with unit tests that feed
   events and ms by hand.
3. Write `build_scene`, with snapshot tests on 2721's fixture story at
   each stop, with `instant` on.
4. Add `layout` to `vn.ron` (update its fixtures and determinism hash).
5. Write `VnSkin` and its transitions.
6. Write `VnStage`, and a Harness test that plays 2721's fixture story
   to an ending through actions.

## Acceptance criteria

- [ ] `build_scene` is pure: the same state and presenter give the same
      scene (property test).
- [ ] With `instant` on, each stop of the fixture story matches its
      checked-in scene snapshot.
- [ ] Revealed glyphs never decrease while ticking, never pass the
      total, and reach it after total × 1000 / speed ms (property test).
      No line's wrapping changes as it is revealed.
- [ ] The first `on_advance` during a reveal returns `None` and finishes
      it; the next returns `Advance`.
- [ ] Each transition kind's progress runs from 0 to 1000 over its
      duration, and `instant` jumps it to 1000.
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

- Unit: the presenter's reveal, waits, transitions and `on_advance`.
- Property: `build_scene` purity; reveal bounds; transition progress
  bounds.
- Snapshot: the scene dump at every stop of the fixture story.
- Look: one test of cells and sprite items.
- Integration: the Harness playthrough and the `frame-png` picture.

## Completion notes
