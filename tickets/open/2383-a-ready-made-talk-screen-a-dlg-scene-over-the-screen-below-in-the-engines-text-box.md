---
id: "2383"
title: "A ready-made talk screen: a .dlg scene over the screen below, in the engine's text box, with replies"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2383 — A ready-made talk screen

## Context

Found by the skill pack's authors (1134), writing `shuyi-dialogue`'s
worked example. The engine reads, checks and plays `.dlg` scenes
(`shuyi_assets::dialogue`, `shuyi_ui::dialogue`: `DialoguePlayer`,
`TextBox`, `TextView`, the text box's glyph `paint`), but it has no
screen that puts them together. A game on the short path writes about
230 lines to play a talk (`skills/shuyi-dialogue/example/talk.rs`):

- the box's place and size, and the `DialogueLimits` that keep scenes to
  them;
- a `SceneRules` with those limits and no tokens;
- reading the file through the registry and checking it
  (`scripts_from_parsed`);
- the screen: typing at the player's text speed, holding Confirm to type
  faster, a page at a time, a cursor over the replies, Cancel to leave,
  the speaker's name from a text key, and its view.

Hello, Grid's `Talk` (`examples/hello-grid/src/screens.rs`,
`skin::paint_talk`) is the same screen again. The end goal
(`docs/decisions.md`, 2026-10-19) judges a new API by the lines a game
writes. A talk is something most story games have, a town's shopkeeper
as much as a tactics game's briefing. Visions of Shuyi has a dialogue
screen of its own, so this comes after the MVP and isn't parked
(`docs/decisions.md`, 2026-10-20).

## Nick input

None.

## Scope

**In:**
- **`shuyi_ui::screens::talk`**, beside the other screens any game can
  use (`crates/ui/src/screens.rs` lists them):
  - `TalkLook { place: BoxPlace, width: usize, lines: usize }`, with
    `TalkLook::BOTTOM`: the box across the bottom of the short path's
    48 × 24 console, as `skills/shuyi-dialogue/example/talk.rs` places
    it. A look is plain data, so it is `#[non_exhaustive]` with a
    constructor (ADR-0039);
  - `talk::limits(look) -> DialogueLimits`, and `TalkRules`, a
    `SceneRules<NoDirective>` with those limits and no tokens;
  - `talk::load(bundle, file, look) -> Result<DialogueTable,
    Vec<ContentError>>`: the file read through the standard registry's
    `Dlg`, checked against `TalkRules`, every problem with its file and
    line;
  - `TalkScreen<G: GameData>`: `TalkScreen::new(ctx, scene, look)`
    plays one scene over the screen below (`is_overlay`). Confirm types
    the page out, then shows the next page or line, and holding it types
    faster, at the player's text speed. Up and Down move between the
    replies, and Confirm picks one. Cancel skips to the next choice, or
    closes when none is left. Through `Transition::pop_with`, the screen
    pops with `TalkEnded { scene: String, replies: Vec<usize> }`, the
    replies picked in order, which the screen below reads in
    `uncovered` (ADR-0018). It declares the game's actions for Confirm,
    Up, Down and Cancel (`ui_actions`), as the engine's other screens
    do;
  - its view, `TalkScreen::view(&self, ctx) -> TalkView` (the text
    box's `TextView` and the speaker's name), which tests read, and its
    glyph skin through `dialogue::text_box::glyph::paint`;
  - its text keys: `talk.next` (the marker's words) and
    `character.<id>` for each speaker's name, in the game's language
    file.
- **The least a short-path game writes** to use it, shown in the module's
  docs: a stage's `Own` holds the `DialogueTable`, its `own` is
  `talk::load(bundle, "talk.dlg", TalkLook::BOTTOM)`, and a screen
  pushes `TalkScreen::new(ctx, scene, TalkLook::BOTTOM)`.
- **Hello, Grid** plays its intro with `TalkScreen`, so `Talk` and
  `skin::paint_talk` go. Its playthrough tests
  (`examples/hello-grid/tests/it/playthrough.rs`) read the new view. If
  its look changes, the reviewed `frame-png` snapshot is written again.
- **The skill:** `skills/shuyi-dialogue/SKILL.md` teaches `TalkScreen`
  first, and `DialoguePlayer` and `TextBox` for a screen of a game's
  own. Its worked example's `example/talk.rs` shrinks to the stage's
  `Own`, `own` and `first` (about 30 lines), and its `run` steps expect
  the same text as today. `check-skills` passes, and the example passes
  in CI's `skills` job.
- **Either order with 2351:** if 2351 is done, `talk.next` is one of the
  engine's text keys, with built-in English beneath the game's text. If
  not, nothing to do (2351 gathers every engine key, this one among
  them).
- **Either order with 2384:** if 2384 is done, the talk screen hands a
  scene's `@music` lines to the game's music (`DialoguePlayer::take_music`
  as the scene reaches them). If not, nothing to do (2384 adds that to
  the talk screen).

**Out (do not do):**
- portraits (the short path has no portraits yet), and a game's own
  directives (`Directive`) and tokens: a game with them writes its own
  screen on the player, as today;
- voice clips per line, and languages beyond the game's English;
- a visual novel's stage (2721, parked), which may build on this screen;
- Yarn Spinner and ink files (2202).

## Implementation steps

1. Read `crates/ui/src/dialogue/`, `crates/ui/src/screens/` (how a
   screen any game uses takes the game's actions and text),
   `skills/shuyi-dialogue/example/talk.rs`, and Hello, Grid's `Talk`.
2. Write `shuyi_ui::screens::talk`: the look, the limits and rules,
   `load`, the screen, its view and skin, with the tests below.
3. Move Hello, Grid onto it.
4. Update the dialogue skill and its worked example, and run
   `cargo xtask skill-examples shuyi-dialogue`.
5. Run the gates.

## Acceptance criteria

- [ ] A short-path game plays a `.dlg` scene with the stage's `Own`,
      `own` and `first` alone: the dialogue skill's worked example, at
      about 30 lines in place of about 230.
- [ ] The talk screen pops with the replies picked, and the screen below
      reads them (a test).
- [ ] Hello, Grid plays its intro on it, and its tests pass.
- [ ] `check --target all` passes on a game with the talk screen in its
      `screens`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/ui/src/screens/talk/tests.rs`): a scene read through to
  the end; typing, a page, the next line; holding Confirm; a choice
  picked with Up, Down and Confirm; Cancel skipping to the next choice
  and closing; the `TalkEnded` result; the view's speaker from
  `character.<id>`; `load` naming a problem's line.
- Snapshot / integration: Hello, Grid's playthrough; the dialogue skill's
  worked example in CI's `skills` job.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
