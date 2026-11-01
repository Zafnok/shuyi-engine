---
id: "2817"
title: "Play every tutorial step in the browser and record the evidence"
type: infra
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2803"]
nick_input: none
completed:
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

- [ ] Each of the ten steps has a passing manual-play evidence row with
      the revision, page/play URL, browser, interactions and result.
- [ ] The behavior introduced in each step was exercised; options,
      rebinding and saved progress were tested where they are present.
- [ ] Leaving an offered menu never freezes or quits the browser game;
      the game still responds after those actions.
- [ ] Every failure found has its corrective ticket and a passing retest
      before this ticket is marked done; no failed row is relabeled a
      success because CI boots the game.
- [ ] 2803 links the evidence while retaining its honest account of the
      original verification gap.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [ ] All applicable gates in the `run-gates` skill pass; the notes name
      what was run and any remaining limitation.

## Tests required

- Manual: the ten browser-play rows above.
- Existing automated tutorial tests and browser boot checks remain
  separate supporting evidence; no new duplicate test framework.
- Repository documentation/ticket gates for the evidence edits.

## Completion notes

*(Filled in by the session that completes the ticket: the tested
revision, ten evidence rows, defects/retests, captures and validation.)*
