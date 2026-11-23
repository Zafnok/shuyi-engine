---
id: "3526"
title: "Nick decides the editor's path: what it is built with now, visual scripting on grid-game blocks, its groundwork before the first release, and the accessibility tree and text size"
type: decision
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 3526 — Nick decides the editor's path, and the accessibility tree and text size

## Context

The blind-spot audit of 2026-11-23 found four questions about the
editor and the UI that only Nick can answer. Each one unparks work he
parked, or moves work ahead of the first release (2005).

What exists today:

- **The editor track is parked** (`docs/decisions.md`, 2026-10-19:
  "The editor is parked, with 1135"). It is research on the workspace
  (2506), then the shell with a live view, a tree and an inspector
  (2507), then the graph panel (2508). Nick asked for it twice: "UI /
  graphical coding (like Blueprints from Unreal)" (2026-10-14) and "a
  UI layer for visual code, like Godot editor or Unreal Blueprints"
  (2026-10-19).
- **Most of the editor's groundwork is done.** 2506 is blocked by
  1129, 2316, 2521 and 2907. 1129 (the tool contract), 2521 (the live
  link) and 2907 (one description of a game's types) are done. Only
  2316 (the debug view in real time, `todo`) is left.
- **What the editor is built with is still open, while the in-game
  layout is designed now.** 2329 (layout) and 2330 (`.ui.ron`) are
  `todo`. If the editor were built on the engine's own UI, both would
  need docking, tree views, scroll areas and dense mouse-first text,
  which they don't plan. `crates/ui/src/widgets` holds four widgets:
  `ask`, `list`, `menu` and `name_entry`.
- **2506 weighs two constraints that don't fit a developer tool.**
  - Its Context counts `wasm-bindgen` against `egui` (lines 64-66),
    because the game's web build has none (game ADR-0034). A native
    editor never enters a game's web build.
  - 2507's inspector bullet applies rule 8 to the editor's text boxes:
    they "paste from the clipboard on an action, never a key", and
    "typing follows 2346's name box". Every desktop editor has the
    usual shortcuts.
  - 2506 doesn't measure `egui-miniquad`, which draws `egui` on the
    engine's present backend.
- **Visual scripting waits for the game's launch.** 1135 is blocked by
  2101 (done), 2907 (done), 1125 and 1127. 1125 (a walkable town) and
  1127 (a platformer) are parked "after Visions of Shuyi launches, not
  only after the MVP" (Nick, 2026-10-20). 1135 takes its first nodes
  from those two examples. 2508's sign-off is a door and a switch in
  the platformer. Yet everything a first graph compiles to already
  exists in turn games:
  - dialogue branches (`.dlg`);
  - scripts (2101, Rhai);
  - the `hello-grid` example and the box-pushing tutorial (2803);
  - story variables (2140) and quests as data (2141), both `todo`.

  Godot removed VisualScript because it lacked high-level blocks (1135's
  Context). Quests, dialogue choices, switches and win conditions are
  such blocks.
- **The editor's groundwork isn't ordered before the first release.**
  Nick wants no run of breaking versions after the MVP
  (`docs/decisions.md`, 2026-11-14). These open, unparked tickets add
  public shapes the editor and agents both use:
  - 2244, a comment-keeping RON editor;
  - 3417, one `patch` operation addressed by JSON Pointer;
  - 2515, every kind with its schema;
  - 3418, inspector hints and a game's own panels as data;
  - 3416, frame items that record the file they came from;
  - 2316, the debug view in real time.

  All are `After MVP`. Landing them after 1.0 risks a break, or a shape
  bent to avoid one. ADR-0039 (`#[non_exhaustive]` types) softens this
  but doesn't remove it.
- **The accessibility tree (2322) and text size (2323) are parked**
  because "the game doesn't ask for it" (2322's Parked note). Both
  need nothing parked: 2322's blocker 2405 is done, and 2323's
  blockers 2209, 2018 and 2392 are done. 2322 is headless. Only 3035
  and 3058 (sending the tree to a screen reader) need the platform
  work of PR #532 (ADR-0136). Each screen and widget added before the
  tree exists is one more to retrofit. 2322 lists five engine
  screens and four widgets now. The tree would also give agents a list of what is
  on screen by meaning, not pixels. Today `Harness::click_text` matches
  visible text, and the contract has no answer about the UI. 2340
  (reduced motion and flashing) is open; text size is not.

## Nick input

**Decision:** four questions, asked with the `ask-nick` skill, one at a
time, in the words below. None is technical.

1. **May the editor's research start now, while building it stays
   parked?** The research decides where the editor runs and what it is
   built with. Deciding it now lets the in-game menus be built without
   guessing whether the editor will reuse them. The likely answer of the
   research is a separate desktop program for developers, built on a
   common Rust UI library (`egui`). It is never part of a game a player
   downloads. It uses the usual desktop shortcuts (Ctrl+Z, Ctrl+C),
   not the game's rebindable controls, and is offered as a ready-made
   download.
   - **Yes, the whole research (2506).** It starts when the real-time
     debug view (2316) lands. This unparks 2506 only; 2507 and 2508 stay
     parked.
   - **Yes, only "where it runs and what it is built with", now.** That
     part becomes a ticket of its own that needs nothing first. The rest
     of 2506 stays parked.
   - **No.** As today. The menu work goes ahead and the editor adapts
     to it later.
2. **Should visual scripting be built from turn-based game pieces, so
   it doesn't wait for Visions of Shuyi to launch?** Today its first
   building blocks come from a walking town and a platformer, both
   parked until after the launch. Turn-based pieces exist now: dialogue
   choices, switches and doors on a grid, quests, story flags and
   scripts. Moving pieces for free movement come later.
   - **Yes, and unpark the research (1135) now.**
   - **Yes, but it stays parked with the editor.** It no longer waits for
     the launch, only for Nick to unpark the editor's track.
   - **No.** As today.
3. **Should the editor's groundwork land before the first release, so
   it never forces a breaking version later?** It is six open tickets
   that agents use today too: editing data files without losing
   comments, one shared way to change a value in a file, a description
   of every file's fields, hints that say how a field is shown, a
   record of which file a thing on screen came from, and the real-time
   debug view. The cost: the first release waits for about six more
   tickets.
   - **Yes, all six before the first release.**
   - **Only their shapes before the release.** The shapes are settled
     so they can grow without a break; they are built afterwards.
   - **No.** After the release, accepting a possible break.
4. **Should screens describe themselves for screen readers, and should
   players pick their text size, be unparked now?** Neither changes
   what the game gets. The described screens also let agents read
   what is on screen by meaning, not pixels. Talking to an actual
   screen reader stays parked with the window work.
   - **Both** (2322 and 2323).
   - **Only the described screens** (2322).
   - **Only text size** (2323).
   - **Neither.** As today.

## Scope

**In:**
- Ask the four questions and record each answer in Nick's words in
  `docs/decisions.md`.
- Apply each answer to the tickets, as below. For each ticket it
  unparks, set `status: todo`, drop `Parked:` from the title and the
  Parked note, and update `docs/ROADMAP.md`.
- **Question 1, "Yes" (either):**
  - 2506 (or the split-off ticket) gets the audit's candidate as the
    option to measure first, beside its others. The ADR keeps the
    build-time and `cargo deny` measurements 2506 asks for. The
    candidate:
    1. a `shuyi-editor` crate: a native `eframe`/`egui` binary,
       dev-only, with its own line in `tools/layers.ron`; no game crate
       depends on it;
    2. the game in its own process and window, reached over the live
       link (2521), so a crash or a desync in one never takes the other;
    3. every edit through one patch operation (3417), shared with
       agents and the CLI;
    4. ordinary desktop text editing and shortcuts, outside rule 8 and
       the console-grid look, as the CLI is;
    5. `egui_dock` for docking and `egui-snarl` for graphs, each
       license checked;
    6. prebuilt editor and CLI binaries attached to releases, signed
       when 3051 lands.
  - 2506's Context: its `wasm-bindgen` note says it applies only if the
    editor runs in a browser. `egui-miniquad` joins the measured
    options.
  - 2507's inspector bullet: the "never a key" and "name box" lines say
    the ADR decides. 2507 stays parked.
  - If the split is chosen: write the split-off ticket
    (`write-ticket`), with `blocked_by: []`, and narrow 2506 to the rest.
- **Question 2, "Yes" (either):**
  - 1135: drop 1125 and 1127 from `blocked_by`. Its first nodes come
    from `hello-grid`, the tutorial's box pusher (2803), `.dlg`
    dialogue, story variables (2140), quests (2141) and scripts (2101).
    Add 2145 (scripts that reload) to `blocked_by`: the graph compiles
    to scripts, and editing it live needs them to reload. Free-movement
    nodes become later node-kind tickets, parked with 1125 and 1127.
  - 2508: its example and sign-off become a switch that opens a door in
    a grid game.
  - For "Yes, and unpark": unpark 1135 too.
- **Question 3:**
  - "Yes": set 2244, 3417, 2515, 3418, 3416 and 2316 to `milestone:
    MVP`, and add them to 2005's `blocked_by`.
  - "Only their shapes": write one ticket (`write-ticket`) that 2005
    needs. It settles each shape as ADR-0039 says: the path model
    (JSON Pointer), the reserved schema-hint keys and the frame item's
    optional `source` field. The six tickets stay `After MVP`.
  - **Either order with 3528:** if 3528 is done, skip any of the six
    tickets its answer already moved into the MVP, and say so when
    asking; if not, ask as written, and 3528 leaves these six to this
    ticket.
- **Question 4:**
  - 2322 (if unparked): add to its scope a contract answer that lists
    the tree as JSON, and harness steps that find a node by its id.
    3035 and 3058 stay parked.
  - 2323 (if unparked): nothing else changes.

**Out (do not do):**
- Building any of the tickets named here, or writing 2506's ADR.
- Changing 2329 or 2330. If question 1 leads to the engine's own UI as
  the editor's, 2506's ADR writes what they need then.
- Whether a game's whole rules can be a script (2146): 3525 asks it.
  The graph doesn't wait for it.
- Sending the tree to a screen reader (3035, 3058): parked with the
  platform work (ADR-0136).
- Unparking 2507, 2508, 1125 or 1127.
- Editing `AGENTS.md`'s rule 8. If 2506's ADR exempts the editor, that
  ADR's ticket adds the line.

## Implementation steps

1. Read `docs/decisions.md` (2026-10-14, 2026-10-19, 2026-10-20 and
   2026-11-14), and 2506, 2507, 2508, 1135, 2322 and 2323 as they are
   then. Check that each fact in the Context still holds (blockers done,
   tickets still parked) and correct a question that has gone stale.
2. Ask the four questions with `ask-nick`, one at a time, in the words
   above.
3. Record each answer verbatim in `docs/decisions.md`.
4. Apply each answer to the tickets as the Scope says, and write the
   new tickets it calls for.
5. Run `cargo xtask ticket-lint`.

## Acceptance criteria

- [ ] `docs/decisions.md` has the four answers in Nick's words.
- [ ] Each ticket an answer unparks has `status: todo`, no `Parked:`
      prefix and no Parked note, and `docs/ROADMAP.md` agrees.
- [ ] Each "Yes" has its ticket edits or new ticket, as the Scope says.
- [ ] `cargo xtask ticket-lint` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None (a decision).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
