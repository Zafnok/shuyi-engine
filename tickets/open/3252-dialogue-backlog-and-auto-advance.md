---
id: "3252"
title: "A dialogue backlog that jumps back, auto-advance that waits for the voice, and the lines that can come next, over DialoguePlayer"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3252 — A dialogue backlog, auto-advance and the lines that can come next

## Context

A request from the game.

1. **The game ticket:** game#1413, which moves its conversation screen
   onto these once this lands
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1413-the-conversations-backlog-and-auto-on-the-engines.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** while a scripted talk plays, the screen
   does three things beside the text box:
   - it remembers every line the player saw, so they can open a log and
     jump back to one (a reply picked later is asked again);
   - it moves on by itself once a voiced line's clip has ended, or once
     a silent page's reading time has passed;
   - it tells the audio which lines can come next, so their clips are
     loaded in time.
3. **The friction numbers:** from the game's third engine-friction sweep
   (game#1397, `docs/engine/friction.md`):
   - **Measured:** 135 production lines at the game's pin `230080d`:
     - `crates/ui/src/screens/dialogue/history.rs` (63);
     - in `screens/dialogue.rs`: `coming_lines`, `Clip`, `auto_advance`
       and their constants (72).
   - **Estimate:** about 25 lines for what the game would keep: which
     rows have a voice (its `RowKind` rule) and the lead's portrait in
     the log.
   - **Ratio:** 5.4×.
   - **Outcome:** 1, engine feature.

   The game's first sweep (1153) logged only the log's scroll window,
   which engine#2307 then gave it.
4. **The generic shape:** `shuyi_ui::dialogue::backlog`, `::auto` and
   `DialoguePlayer::coming_lines`, all over the engine's
   `DialoguePlayer<H>`. None of them names a speaker, a portrait or a
   voice rule: the game passes "is this line voiced" as a closure.
5. **Not a tactics example:** a farming game's festival talk, a
   platformer's shopkeeper and a visual novel all want a log to jump
   back in, an "auto" setting, and their next voice clips loaded ahead.

**How this relates to the parked visual-novel tickets.** engine#2723
plans a backlog, auto and going back, and engine#2725 plans auto
waiting for the voice. Both are for `shuyi_vn`'s `VnStage`, a story
told as turn rules. A game that plays `.dlg` scenes through
`DialoguePlayer` has no `VnStage`. So this ticket builds the pieces on
`DialoguePlayer`, where both can use them:
- **Either order with 2723:** if 2723 is done, its `Conveniences` auto
  step and `Backlog` read through this ticket's `auto::AutoAdvance` and
  `backlog::Backlog`; if not, nothing to do (2723 uses them when it is
  unparked).
- **Either order with 2725:** if 2725 is done, its "auto waits for the
  voice" uses `auto::VoiceClip`; if not, nothing to do (2725 uses it).

## Nick input

None.

## Scope

**In:**
- `backlog::Backlog<P: Clone>` (`#[non_exhaustive]`, `Backlog::new()`).
  It holds `Seen<P> { player: P, reply: Option<usize>, music:
  Option<MusicLine> }`, oldest first, and which entry is on screen:
  - `reached(&player, music)` keeps a text box;
  - `replied(&player, option, music)` keeps a reply, at the choice,
    before it was picked;
  - `seen()`, `at()`, `latest()` and `is_empty()` read it;
  - `jump(i) -> Option<Seen<P>>` gives back the entry to show.

  It keeps the game's rule. After a jump, the entries past the one on
  screen stay, so they can be jumped forward to, except from the first
  reply on: a reply is asked again. So no reply is ever past the entry
  on screen. `P` is the game's player type, usually a newtype over
  `DialoguePlayer`, which is `Clone`.
- `DialoguePlayer::coming_lines(&self, ahead: usize) -> Vec<LineId>`:
  - up to `ahead` text boxes after the one on screen;
  - at a choice, the first line of each reply's reaction, then no
    further;
  - the line a choice answers, already on screen, is left out.
- `auto::VoiceClip` (`#[non_exhaustive]`, `VoiceClip::asked()`):
  `ended_s(sounding: bool, dt) -> Option<f32>`, the seconds since the
  clip ended. `None` while it sounds, or while it might still start: a
  clip never heard is waited for `AutoTimes::voice_wait_s` first.
- `auto::AutoAdvance` (`#[non_exhaustive]`), built from `AutoTimes {
  after_voice_s, read_base_s, read_per_char_s, voice_wait_s }`
  (`#[non_exhaustive]`, `AutoTimes::new()` gives 0.5, 1.0, 0.05 and 0.5,
  the game's tunables). Its `step(&mut self, AutoInput) -> bool` says
  "move on as Confirm would". `AutoInput` is `{ on, waiting, pressed,
  revealed, last_page, chars, clip: Option<&mut VoiceClip>, sounding,
  dt }`.
  - It never fires while off, while waiting on a choice or question,
    while a key is pressed, or before the page is revealed.
  - On a voiced last page it fires `after_voice_s` after the clip ends.
  - Otherwise it fires after the reading time: `read_base_s +
    read_per_char_s × chars`.
- No API break, additions only. The game's PR that moves the pin follows
  this one (game#1413).

**Out (do not do):**
- the log's screen and skin: the game's log names its speakers and
  faces, and the engine's text box has no log screen yet (write a
  ticket if a second game needs one);
- `VnStage`, skip, read lines, the quick menu (engine#2723);
- mouths and eyes with the voice (engine#2725);
- an auto-advance setting row: the game's options own it today (write a
  ticket for an engine row if the engine's options screen should have
  one).

**Estimated size:** about 650 lines (code 200, tests 350, docs 100).

## Implementation steps

1. Read the game's `crates/ui/src/screens/dialogue/history.rs`,
   `screens/dialogue.rs` (`coming_lines`, `Clip`, `auto_advance`) and
   their tests at the pin.
2. Write `crates/ui/src/dialogue/backlog.rs` and
   `crates/ui/src/dialogue/auto.rs`, and add `coming_lines` to
   `crates/ui/src/dialogue/player.rs`.
3. Port the game's tests:
   - a jump back, then reading on;
   - a jump past a reply;
   - auto on a voiced line, on a silent line, and on a clip that never
     starts;
   - coming lines at a choice.
4. The dialogue preview (`crates/ui/src/dialogue/preview.rs`) keeps its
   voices loaded through `coming_lines` if it keeps any today.
   Otherwise leave it alone.
5. Book: a section in the dialogue chapter.

## Acceptance criteria

- [ ] After `jump(i)` to an entry before a reply, `seen()` holds nothing
      past that reply (unit test).
- [ ] `AutoAdvance::step` with a clip that never sounds fires
      `voice_wait_s + after_voice_s` after the page is revealed, and not
      before.
- [ ] `coming_lines` at a choice of two replies gives each reply's first
      reaction line and nothing after.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Backlog`, `VoiceClip`, `AutoAdvance`, `coming_lines`.
- Property: after any sequence of `reached`, `replied` and `jump`, no
  reply is past `at()`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
