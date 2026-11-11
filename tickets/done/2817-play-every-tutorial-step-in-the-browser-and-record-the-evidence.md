---
id: "2817"
title: "Play every tutorial step in the browser and record the evidence"
type: infra
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2803"]
nick_input: none
completed: 2026-11-07
---

# 2817 — Play every tutorial step in the browser

## Context

[PR 218](https://github.com/Zafnok/shuyi-engine/pull/218) completed the
ten-step tutorial (2803) while this audit was running. Its scope requires
playing each step by hand, but its completion notes report hand-playing
only step 10. All ten web builds compile and boot in CI. A boot check
does not exercise the earlier steps' interactions.

The archived 2803 now records that acceptance gap truthfully. This ticket
finishes the browser-play requirement on the current engine revision,
with evidence a later reader can inspect. It adds no tutorial feature.

**Before the one book (2823, 2824):** 2823 waits for this ticket (Nick,
2026-11-05: the tutorial is finished before it is redesigned), so play
each step from `/learn/tutorial/`. Nothing is left to 2824, which moves
the pages and redirects the old addresses afterwards.

## Nick input

None. The session exercises the pages and records the results; no new
owner playtest or sign-off is required.

## Scope

**In:**
- Build and serve the tutorial pages and their web games using the
  existing website/Pages recipe. Test the current revision from each
  page's play link, rather than launching an unrelated local binary.
- Play all ten steps by hand, exercising each introduced behavior and
  the earlier behaviors it keeps:
  - 01–02: opening the game and rendering its title/board;
  - 03: movement and walls;
  - 04: pushing and winning;
  - 05: undo;
  - 06: level selection and the menu result after a win;
  - 07: progress retained across a browser reload;
  - 08: the move counter, options and rebinding;
  - 09: ordinary play alongside the bot/replay tests;
  - 10: the complete playable loop and saved progress after reload.
  Exercise leaving a menu wherever it is offered and verify the web game
  keeps responding. Test sound only where the current step has sound.
- Keep one evidence row per step in the completion notes, or a linked
  PR artifact: revision, page/play URL, browser, interactions, observed
  result, and failures/retests. Distinguish manual play, automated boot
  and unit-test evidence. Preserve useful captures where appropriate.
- About 100–200 changed documentation/ticket lines, excluding captures.
  No new dependency.

**Out (do not do):**
- Implementing future sound, solver, web-template or live-preview work.
- Claiming that a build, passing bot or boot check proves manual play.
- Silently fixing a newly found implementation defect here: file a
  focused corrective ticket, link the failed step and keep its acceptance
  row incomplete until a retest of the fix passes.
- Changing 2803's merged implementation or completion date.

## Implementation steps

1. Read `site/README.md`, `pages.yml`, the tutorial pages, and 2803's
   corrected acceptance notes. Record the tested commit and browser.
2. Build/serve the pages with their playable web folders, then follow
   each page's link and exercise the behavior above.
3. Record all ten rows. File any reproduced defect with its evidence;
   after its fix, record the new revision and retest the affected steps.
4. Link the completed evidence from 2803's correction note, without
   rewriting what was verified at its merge. Run the applicable gates.

## Acceptance criteria

- [x] Each of the ten steps has a passing manual-play evidence row with
      the revision, page/play URL, browser, interactions and result.
- [x] The behavior introduced in each step was exercised; options,
      rebinding and saved progress were tested where they are present.
- [x] Leaving an offered menu never freezes or quits the browser game;
      the game still responds after those actions.
- [x] Every failure found has its corrective ticket and a passing retest
      before this ticket is marked done; no failed row is relabeled a
      success because CI boots the game.
- [x] 2803 links the evidence while retaining its honest account of the
      original verification gap.
- [x] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [x] All applicable gates in the `run-gates` skill pass; the notes name
      what was run and any remaining limitation.

## Tests required

- Manual: the ten browser-play rows above.
- Existing automated tutorial tests and browser boot checks remain
  separate supporting evidence; no new duplicate test framework.
- Repository documentation/ticket gates for the evidence edits.

## Completion notes

**Tested revision:** `8afc8b76` (main, 2026-11-06). **Browser:** the
Claude desktop app's browser pane, Chrome 152.0.7977.167 on Windows 11.
**How served:** the `pages.yml` recipe locally. `bash errors.sh`,
`cargo xtask site-data --site .`, `zola build --base-url
http://127.0.0.1:8817`, then each step's `cargo shuyi web --release`
copied to `learn/tutorial/<step>/play/`, served by `python -m
http.server`. Each step was opened from its page's "Play this step" link.
The local build skipped CI's `cargo update` of the engine's crates, which
changed only the lock files; they were restored.

**Kind of evidence.** *Manual*: keys sent one at a time to the game's
canvas through the pane, each result read from a screenshot (a
person's pace: 0.2–0.4 s between keys). *Unit*: step 09's `cargo test
--workspace`, 26 tests passed (its bots win every level's line, its
replays and frame test), run separately. *Boot*: CI's `web-render`
boots each step; not counted here.

| Step | Play URL | Interactions | Observed | Result |
| ---- | -------- | ------------ | -------- | ------ |
| 01 | `/learn/tutorial/01-new/play/` | Load; Enter; Backspace | Title and "made with Shuyi Engine". No "leave" help line and Cancel does nothing on the web (`can_quit` is false there, as `title.rs` says); frames keep running | Pass |
| 02 | `/learn/tutorial/02-board/play/` | Enter; Backspace; Enter | Board "First push" (`@ [] <>` in walls); Cancel back to the title; Enter opens it again | Pass |
| 03 | `/learn/tutorial/03-walk/play/` | Up, Up, `a`, Left, Down ×3 | Walks; a wall refuses the second Up, the second Left and the third Down; `a` walks like Left | Pass |
| 04 | `/learn/tutorial/04-push/play/` | Right, Right, Left | Two pushes put the box on the target: "Solved!"; the move after the win is refused | Pass |
| 05 | `/learn/tutorial/05-undo/play/` | Right, Up, `u` ×3, Right, Right, `u` | Undo takes back the walk, then the push, then does nothing; it also takes back the winning push ("Solved! u undo" offers it) | Pass |
| 06 | `/learn/tutorial/06-levels/play/` | Menu; Backspace; level 2 by its line; level 1, win, Enter; Down ×4 | Levels 1–5 menu; Cancel to the title and back; level 2 won ("Solved! Enter next"); after level 1's win, Enter returns to the menu with Level 2 in focus; the menu wraps | Pass |
| 07 | `/learn/tutorial/07-save/play/` | Empty storage; Down, Enter; win 1, Enter; reload; level 2 | Levels 2–5 dim and skipped; after the win Level 2 lights up; after a reload Level 2 is still lit and focused, and opens "Two rows" | Pass |
| 08 | `/learn/tutorial/08-polish/play/` | Moves; Options: Frame rate, Key bindings (bind `y` to Undo), reload, both Restore defaults, Escape and Backspace out | "Moves: 2" after two moves and a wall bump, 1 after undo; 60 → 120 FPS and back; `y` undoes, survives a reload, and is gone after the bindings' own Restore defaults (Options' Restore defaults resets settings only, as specified); every menu leaves to the one below and the game responds. **Failed at `8afc8b76`:** on the Key bindings tab row the help line showed `{CursorLeft}/{CursorRight} keyboard or controller`, cut short at the panel's edge | Pass after the retest below |
| 09 | `/learn/tutorial/09-test/play/` | Level 1 won, Enter; Options → Key bindings; Escape ×3 | Plays as 08; Escape leaves Key bindings, Options and the menu to the title. **Failed at `8afc8b76`:** the same help line as 08 | Pass after the retest below |
| 10 | `/learn/tutorial/10-ship/play/` | Empty storage; all five levels by their lines; reload before level 5; level 5; reload | Every level solved in its line's moves (2, 8, 13, 24, 30), no key lost; after each reload all five are lit, focus on Level 5, storage `(version:1,furthest:4)`. **Failed at `8afc8b76`:** the same help line as 08 | Pass after the retest below |

No step has sound (2803's notes: 2225 adds it), so none was tested.

**Observations, not defects.** Keys sent with no pause (several within
one frame) lost moves, faster than a person types; at a person's pace,
three single presses were lost in about 200 (steps 08 and 09), and the
retries made to find them (24 presses, 21 with their `keydown` logged)
lost none. Not reproduced, so no ticket.

**The defect and its retest.** Steps 08–10's `key_bindings.help_switch`
named `{CursorLeft}/{CursorRight}`, copied from hello-grid, while Box
Pusher's actions are `Left` and `Right`. A corrective ticket was
drafted, then 2576 (merged 2026-11-07, after the tested revision) was
found to have fixed it already: the four `en.ron` files now say `{Left}/{Right}`, and
`shuyi check` fails an unknown action placeholder (`SH2008`). The
ticket was dropped unmerged. Retest at `f6af4b51` (main, 2026-11-07),
steps 08–10 rebuilt and opened from their pages, storage emptied
first: each tab row reads "Left/Right keyboard or controller · arrows
move…". Step 08: level 1 won; a key bound on Key bindings acts in a
level. Step 09: level 1 won. Step 10: level 1 won, Level 2 kept across
a reload. Escape leaves Key bindings, Options and the menu to the
title, still responding. Steps 01–07 were not replayed: 2576 changed no
file of theirs.

**2803** now links this evidence under its correction note.

**Captures:** screenshots were read during play and not kept; each
row's "Observed" says what they showed.

**Gates:** the ticket lint, typos, the links check and the docs map
(`run-gates`, the documentation gates); this change touches only two
tickets, so no Rust gate applies.

Book: none, the change records evidence in tickets; no public item or
page changed.
