---
id: "3316"
title: "A load in steps counts its units and waits for the platform: Progress::Of and Progress::Wait, and the card shows the count as a bar"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3316 — A load that counts and waits

## Context

[ADR-0126](../../docs/adr/0126-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md)
§4, extending ADR-0097 (a load in steps, `crates/platform/src/app.rs`):

- A load in steps says `Progress::More` or `Progress::Done`. Nothing
  says how far it is, and the card (`Card`, ADR-0024 §6) shows no
  progress.
- Where the platform gives no slice (the PC, the web, the headless
  backend), `App` runs every step in one frame (`run_steps` with no
  `Deadline`). A step that has to wait for bytes the page is still
  fetching (3318's streamed pack, where a file is `NotYet`) would spin in
  that frame forever.
- 2906's numbers (`docs/research/2906-asset-memory.md`, *The web*): with
  the `.wasm` fetched first, a web game's card shows at about 1.2 s and
  its first frame, with a 25 MB pack, at up to 9.6 s at 25 Mbit/s. A card
  that says nothing for eight seconds looks stuck.
- ADR-0008 §6 counts a bot's thinking in work units, never time (3009);
  a load counts the same way: a unit is what the load counts, such as a
  file decoded, as `Formats::load_each` reads a folder file by file.

## Nick input

None.

## Scope

**In:**

- **`Progress<G>`** (`#[non_exhaustive]`, so additive) gains:
  - `Progress::Of { done: u32, of: u32 }`: a unit done, `done` of `of`
    (`of` may grow as the load finds more);
  - `Progress::Wait`: the step is waiting for the platform. `App` runs no
    more steps this frame, presents the card, and calls the same step
    again in the next frame, whatever the slice.
  - The doc comments say a unit is the load's own, never time, and that
    `More` is a unit with no count.
- **`App`:** keeps the last `Of` a step gave, and `App::load_progress()
  -> Option<(u32, u32)>` reads it. A `Wait` ends the frame's steps.
- **The card's count:** `Frame` (`#[non_exhaustive]`) gains
  `loading: Option<(u32, u32)>`, set on a card's frame from the last
  `Of`, with `Frame::with_loading`. Each backend shows it:
  - `shuyi-app` (`draw_card`, `crates/app/src/backend.rs`): a bar under
    the card's line or picture, in the card's ink, its length `done ÷
    of` of a fixed share of the window's width;
  - the Playdate's display (`crates/playdate/src/display.rs`): the same
    bar, in the card's ink, redrawn only when `done` moves;
  - the headless backend keeps the count with the card, for tests.
- **A recorded or hashed run** (ADR-0097 §4, ADR-0072): a `Wait` ends its
  frame too. A recording counts the loading frames before the game's
  first as one, so a run recorded on the web plays back on the headless
  backend, where nothing waits; if the record already doesn't count them,
  say so in the completion notes and add a test that shows it.
- **Hello, Grid** loads in steps that give `Of` (a unit a file decoded),
  so its card shows the count where a load takes more than a frame.
- **Docs:** the module comment of `app.rs`, ADR-0097's terms in the
  book's chapter on starting a game.
- **Either order with 3009:** if 3009 is done, `think::Progress` and the
  load's `Progress` stay two types (the load's carries the game it made);
  name 3009's in the doc comment as the other way work is counted. If
  not, nothing to do (3009 asks nothing of this ticket).

**Out (do not do):**

- what waits on the web: the streamed pack and its `NotYet` (3318); this
  ticket gives the `Wait` it returns;
- the page's own progress bar before the `.wasm` runs (3052);
- time as progress (ADR-0126, *Alternatives*);
- threads or an async asset server (ADR-0007, *Alternatives*).

**Estimated size:** about 700 lines (code 300, tests 300, docs 100).

## Implementation steps

1. Add `Of` and `Wait` to `Progress`, and the handling in `run_steps`
   and `App::load`.
2. Add `Frame::loading`, and draw it in `shuyi-app` and the Playdate's
   display.
3. Settle a recorded run's loading frames.
4. Move Hello, Grid's load onto `Of`; write the docs.
5. Run the gates.

## Acceptance criteria

- [ ] A load whose steps say `Wait` twice, then `Done`, takes three
      frames on the headless backend with no slice, the card on each (a
      Harness test).
- [ ] `App::load_progress` and the card's frame give the last `Of`.
- [ ] A run recorded with waits plays back on the headless backend with
      the same hashes from the game's first frame (a test).
- [ ] `shuyi-app`'s bar for a count has the length `done ÷ of` says (a
      unit test of its rectangle), and the Playdate's fake display draws
      it.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `run_steps` with `Wait` with and without a slice; `Of` kept
  across frames.
- Harness: a load that waits, its frames and its card; a recorded run's
  playback.
- Unit: the bar's rectangle in `shuyi-app`; the Playdate's fake display
  with a count.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
