---
id: "2724"
title: "Parked: Visual novel saves at any line with thumbnails, the gallery and endings, and story checks with a route chart"
type: feature
milestone: "Engine: visual novels"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["2723", "1129", "2332", "2110"]
nick_input: none
completed:
---

# 2724 — Saves at any line, the gallery, story checks and a route chart

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0016](../../docs/adr/0016-visual-novels-and-detective-games-a-story-stage-a-case-file-and-a-fair-play-check.md)
sets four things this ticket builds:

- **§2, saves**: a save is the story's history, with a snapshot every N
  lines so loading is quick. It names lines by id, so it stays valid
  when the text of a line is fixed in a patch.
- **§4, thumbnails**: each save shows a picture drawn with `frame-png`'s
  drawing code from the scene, so it needs no window.
- **§3, the gallery and endings**: kept in the player's profile, across
  playthroughs.
- **§5, checking a story**: `shuyi check` (1129) reports broken
  references, unreachable lines and endings, with file and line, and
  `shuyi route-chart` writes the story's graph as Mermaid text. The
  route chart screen shows the parts the player has reached.

A history of `Advance` and `Choose(index)` alone does not survive a
patch that adds a line or reorders a choice. This ticket makes "names
lines by id" concrete: each saved command records the line id it was
sent at, and each choice is saved with its own line id beside its
index.

The save is a file format, so it follows
[ADR-0007](../../docs/adr/0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md):
a text source, versioned, a `Format`, a fuzz-style test and a
determinism fixture. Storage is the platform's, by key
([ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§2).

## Nick input

None.

## Scope

**In:**
- **`VnSave`** in `shuyi_vn::save`, asset kind `VnSave`, format
  `vn-save` (RON text, `deny_unknown_fields`; its version field is
  `version`, version 1, so `SaveHeader` reads it, and `VnSave`
  implements `SaveFormat` and `Migrate` (2110)):
  - the story id, the seed, and the commands, each as `(command,
    at_line: LineId)`; a `Choose` also records the picked choice's line
    id;
  - snapshots: every `snapshot_every` lines (a `vn.ron` field, default
    50), `(command count, VnState)`. `VnState` already names lines by
    id (2721);
  - a label for the slot (the scene's name key and the line's text key)
    and a timestamp given by the caller. The core reads no clock; the
    screen passes the time the platform gave the frame;
  - the thumbnail is stored under its own storage key, not inside the
    save.
- **Loading** (`VnSave::restore(story) -> Result<Restored, LoadError>`):
  - start from the latest snapshot whose line id still exists in the
    story, then replay the later commands, checking that each one is
    sent at the line id it recorded;
  - a `Choose` is resolved by its choice's line id, so a reordered
    choice still picks the same answer;
  - if a recorded line id is gone, stop at the last command that still
    matches, and return `Restored { state, commands, notice }` with a
    notice the save screen shows. A save never fails to load because a
    line's text changed;
  - a save of another story, or a newer version, is a clear error
    (`SaveError::Newer`, naming both versions);
  - **Either order with 2725:** if 2725 is done, loading a save stops
    the voice playing (2725's fake audio test); if not, nothing to do
    (2725 does it).
- **Going back across a load**: `history::back` (2721) replays from the
  nearest snapshot instead of from the start when there is one.
- **The save and load screens**, glyph cells and sprite items:
  - pages of slots (the count is a `vn.ron` field), each with its
    thumbnail, label and time;
  - save, load, and overwrite with a confirmation; quick save and quick
    load as optional actions of `VnAction` (2723), also in the quick
    menu;
  - they react to actions only, and declare them through
    `Screen::actions` (3002).
- **Thumbnails** in `shuyi_vn::view::thumb`:
  - render the current `VnScene` with `VnSkin` into a `Frame`, then
    turn the frame into pixels with the code behind `frame-png`, scaled
    down by a whole factor, then encode PNG bytes. No window, no
    clock, deterministic bytes;
  - that code is `shuyi_ui::frame::raster::Painter` (2332), turned on
    by `shuyi-vn` through `shuyi-ui`'s `raster` feature (a game can't
    depend on `shuyi-cli`);
  - the PNG encoder is the one the engine already uses for `frame-png`.
    If it is not pure Rust or not allowed on the web build, say so and
    store raw pixels instead.
- **The profile**, `VnProfile` in `shuyi_vn::conv`, beside 2723's
  `ReadLines`, saved by key with the settings:
  - the event pictures unlocked (from `VnEvent::EventPicture`);
  - the endings reached (from `VnEvent::Ending`).
- **The gallery screen**: event pictures in a grid, locked ones as a
  placeholder; one opens full screen. **The endings list**: reached
  endings by name key, the others as unknown.
- **`shuyi check` story rules** (§5), in `shuyi_vn::check`, called by
  1129's `check` operation when the game has `shuyi-vn` on. Each
  problem has file, line and message:
  - a character, pose, expression, background, event picture or voice
    that doesn't exist (names come from `Story::build`; this adds that
    each picture and voice file exists in the game's assets);
  - a variable read on some path from the start where no line sets it
    and it has no declared start value. Conditions count as possibly
    true and possibly false, so a declared start value silences a
    warning on a path that can't happen;
  - a line no path reaches, and a choice that leads nowhere (its target
    is missing, or its branch ends the story without an `ending`);
  - an ending no path reaches.
- **The story graph**, `StoryGraph` in `shuyi_vn::graph`: nodes are
  labelled blocks (scenes and labels), edges are jumps and choices,
  endings are marked. The check and the route chart both read it.
- **`shuyi route-chart`**, a contract operation (1129, ADR-0005) with
  JSON output and a schema: writes the graph as Mermaid flowchart text,
  in a stable order (sorted by file, then line). Choice edges are
  labelled with the choice's text in the base language. `--reached
  <profile file>` marks the nodes the player has reached.
- **The route chart screen**: the graph in layers by distance from the
  start, one row of glyph cells per layer. A reached node shows its
  name; an unreached one shows a placeholder from the language pack.
  "Reached" comes from 2723's `ReadLines`: a node is reached when any of
  its lines was read.
- **Either order with 2727** (the mystery module): if 2727 is done,
  make `VnSave` generic over the command type, each command keeping its
  `at_line` id, so a case saves its `MysteryCommand`s unchanged; if
  not, nothing to do (2727 does it). 2727's reference check is never
  this ticket's: 2727 calls it from `shuyi check` if this ticket is
  done, and 2731 wires it if not.

**Out (do not do):**
- cloud saves and syncing (a platform backend's work);
- jumping to a scene from the route chart;
- the rule that a voiced character's lines each have a voice (2725
  owns it; if 2725 lands first, it adds the rule to this ticket's scope
  and this PR writes it);
- the example and its saves (2726);
- movies in the gallery (2909 researches video; a ticket it writes adds
  them);
- anything of the detective module's fair-play check (2731).

If the PR grows well past about 600 changed lines, split the route
chart screen into a ticket of its own (`write-ticket` skill) and say so
in the Completion notes.

## Implementation steps

1. Write `VnSave`, its format, the fuzz-style test and the determinism
   fixture.
2. Write `restore`, with fixtures: a save, then the same story with a
   line's text fixed, with a line added, with a choice reordered, and
   with a line removed.
3. Make `history::back` start from snapshots.
4. Write the thumbnail with 2332's `raster::Painter`.
5. Write the save and load screens.
6. Write `VnProfile`, the gallery and the endings list.
7. Write `StoryGraph`, then the check rules with one bad fixture story
   per rule, then wire them into `shuyi check`.
8. Write `route-chart` with its schema, then the route chart screen.

## Acceptance criteria

- [ ] A save made at any stop of the fixture story restores to the
      same `VnState` hash (property test over random command records).
- [ ] After fixing a line's text, adding a line, or reordering a
      choice, every fixture save restores to the same line id and
      variables.
- [ ] After removing a line, restore stops at the last matching
      command and returns a notice; it never panics or fails.
- [ ] If 2725 is done, loading a save stops the voice playing (fake
      audio test).
- [ ] Restoring from a snapshot and replaying from the start give the
      same hash.
- [ ] `vn-save` passes its fuzz-style test, and its fixture's decode
      hash matches the checked-in value.
- [ ] The thumbnail of a fixture scene is the same bytes on every run
      (checked-in hash), and is made without a window.
- [ ] Reaching an event picture and an ending updates `VnProfile`, and
      a new playthrough keeps them.
- [ ] Each check rule's bad fixture gives exactly its problem, with
      file and line; the good fixture story gives none.
- [ ] `shuyi route-chart` on the fixture story matches a checked-in
      Mermaid file, and its JSON matches the schema (1129's schema
      test).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each check rule; `restore` on each patched fixture; the
  profile updates.
- Property: save and restore round-trips; snapshot and full replay
  agree.
- Fuzz-style: random bytes into `vn-save` never panic.
- Fixture and determinism: the save's decode hash; the thumbnail's
  bytes.
- Snapshot: the Mermaid output; the save screen's, gallery's and route
  chart screen's scene dumps.
- Integration: `shuyi check` and `shuyi route-chart` through the CLI on
  the fixture game.

## Completion notes
