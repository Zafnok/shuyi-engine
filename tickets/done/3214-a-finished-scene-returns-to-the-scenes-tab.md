---
id: "3214"
title: A finished scene in the dialogue preview returns to the Scenes tab, on that scene
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-15
---

# 3214 — A finished scene in the dialogue preview returns to the Scenes tab, on that scene

## Context

1. **The game ticket:** game#0770
   ([`tickets/open/0770-the-script-preview-returns-to-its-scenes.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0770-the-script-preview-returns-to-its-scenes.md)),
   which moves the game's pin to this change. Nick, trying the game's
   web script preview (game#0767) on 2026-11-15: "rather than backing
   out to Lines on scene finish it should go to Scenes".
2. **The job:** when the scene playing in the dialogue preview ends, the
   writer is back on the list of scenes, on the one they just watched,
   so Confirm plays it again and the cursor moves on to the next.
3. **Friction numbers:** no friction review: a change of behavior.
4. **The generic shape:** `DialoguePreview` (engine#2586), when its
   game's screen pops at the end of a scene, opens its tabs on Scenes
   with the Scenes focus on the scene picked, in place of the Lines
   tab. Cancel during a scene still opens the tabs where they were.
5. **Another game:** any game with a writer previewing its `.dlg`
   scenes, a visual novel's or a farming game's, reads one scene after
   another; the list of scenes is where the next one starts.

## Nick input

None beyond the request above.

## Scope

**In:** the preview's end of a scene: Scenes tab, focus on the scene
picked (found by file and id, so a file dropped or saved meanwhile
doesn't move it to another scene). The test that said Lines says
Scenes.

**Out:** the Lines tab's note when a scene is over (it still shows
there); any other tab's behavior. No API change.

## Implementation steps

1. `DialoguePreview::play_frame`: on the screen's pop, `Tab::Scenes` and
   the picked scene's row as its focus.
2. The test of a finished scene checks the tab and the focus.

## Acceptance criteria

- [x] A scene played to its end shows the Scenes tab with the cursor on
      that scene, and Confirm plays it again.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `preview/tests.rs`, a scene over shows the Scenes tab on it.

## Completion notes

**What was done:** `DialoguePreview::play_frame`, when the game's screen
pops at the end of a scene, opens the tabs on Scenes with the Scenes
focus on the scene picked (`picked_row`, found by file and id among the
rows; `scenes()` gives the rows in order and `scene_at` uses it too).
Cancel during a scene is unchanged. The Lines tab keeps its "scene is
over" note.

**Tests:** `the_scene_over_shows_the_scenes_tab_on_it` (it replaces
`the_scene_over_shows_the_lines_tab`): the second scene played to its
end shows Scenes on row 1, Confirm plays it again, and the Lines tab
still says it is over.

Game: game#0770 adopts it (moves the pin to this change). The game's own
preview test was not run on this branch: the engine's `main` is past the
game's pin with API changes the game hasn't taken yet (the game's
engine-pin issue #691), which 0770 or an engine-pin PR before it takes.

No break: no API change.

Behaviour: shuyi-ui: `DialoguePreview` opens its tabs on Scenes, the focus on the scene just played, when a scene ends (it opened them on Lines).
