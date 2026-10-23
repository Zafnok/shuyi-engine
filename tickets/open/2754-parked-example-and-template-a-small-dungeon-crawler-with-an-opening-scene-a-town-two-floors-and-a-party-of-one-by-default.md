---
id: "2754"
title: "Parked: Example and template: a small dungeon crawler with an opening scene, a town, two floors, and a party of one by default"
type: feature
milestone: "Engine: dungeon crawlers"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["2743", "2744", "2749", "2750", "2751", "2752", "2501", "2511", "2004"]
nick_input: sign-off
completed:
---

# 2754 — Example and template: a small dungeon crawler

> **Parked** (Nick, 2026-10-22: "if we can have tickets to add support for this type of game too"): like the other kinds of game, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-22: "another game type I like a lot is Wizardry first
person dungeon crawler, especially the intro to Variant Daphne... what
I would want most is a single player party blobber (i.e. 1 character
only not 4 or 6 party) but anyway support for any character amount is
ok". [ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
§14–§15. A kind of game is supported when it has a template, a guide and
an example that passes CI (ADR-0009 §4–§5).

**The opening.** Daphne's opening is a story scene (the protagonist is
put to death before a crowd), then the protagonist wakes in the
labyrinth and a voice speaks before the first step. The example's
opening has the same shape and its own story, made for it: a few
cinematic shots of text and pictures (the engine's cinematics), then
dialogue over the labyrinth's view, then control. No story, character or
art of Daphne's or Wizardry's is used (`AGENTS.md` rule 4 and 7 apply to
other people's games too).

It builds on every crawler ticket: 2741–2744 (the labyrinth), 2745–2752
(the party), and 2501 (template parts), 2511 (the short path) and 2004
(guides). 2753 (3D) and 2755 (the explorer bot and playtest) come
after; see *Out*.

## Nick input

**Sign-off:** Nick plays the example in the browser (the link is in
the PR) with a party of one, and says whether it feels like the crawlers
he likes. His comments become tickets.

## Scope

**In:**
- **`examples/crawler`**, a game made of `shuyi-crawl` and
  `shuyi-rpg` and the default crates only:
  - **the opening**: three or four cinematic shots (text and pictures),
    then a dialogue scene over the labyrinth's first view, then control;
    skippable by an action, and seen once per profile;
  - **making a character** (2747) with a small `make.ron`: two choices
    and rolled points; the protagonist is the fixed member;
  - **the town**: the example's own place list (2752) of a tavern (the
    roster, 2747), an inn and a temple (paid services: restore,
    refill charges, apply level-ups; end a condition), a shop, and the
    way down. The places' names and what each offers are the
    example's data, not engine types (ADR-0033 §17);
  - **trapped chests**, as the example's own code (ADR-0033 §17): a
    `ChestRules: TurnRules` in the example with `Inspect` (names the
    trap, perhaps wrongly, at the example's chance), `Disarm` (naming a
    trap), `Open` and `Leave`, its screen built from the engine's menus,
    and 2751's drops inside;
  - **two floors** in `crawl-ron` (2741), small (about 12×12), with:
    a door, a locked door and its key, a secret door, a one-way door, a
    spinner, a teleporter, a dark stretch, a pit, stairs between the
    floors, a message, a fixed encounter in a room, and a chest;
  - **battles** (2748, 2749) from encounter tables per area (2751), with
    a few enemy kinds, a chest after a win, abilities (2750) including
    one light ability for the labyrinth;
  - **the game's rules** as one `TurnRules` holding exploring, battles,
    chests and the hub as modes (ADR-0033 §9), so a run is one seed and
    one list of commands;
  - **looks**: the lines skin (2743) by default, a pictures skin made
    for the example as an option in settings, the corner map and the
    map screen with travel (2744);
  - its formulas: a small `PartyMath` of its own, with the numbers in
    its data files;
  - **a party of one by default** (`party.ron` `max: 1`), and a setting
    (`max: 6`, the roster open) that plays the same game with a full
    party.
- **Assets**: made for the example (ours, CC0), or permissively
  licensed and recorded in `THIRD_PARTY_ASSETS.md`. Nothing else.
- **Tests** (ADR-0009 §4), for a party of one and a party of six:
  - a Harness test from the opening to the stairs of floor 2, through a
    battle and a chest, reading scenes and state;
  - a replay determinism test: the same record twice gives the same
    hash list;
  - `frame-png` pictures of the opening, the labyrinth in both skins,
    the map, a battle, the town.
- **The web build** and its place on the Pages index (as 2735 says for
  examples: the loop over `examples/*/shuyi.ron`), and on the website's
  Examples page if 2801 is done.
- **The template part** (2501): in `templates/shuyi-default/`, a part
  `kind-dungeon-crawler`, `when: Picked(kind, dungeon-crawler)`:
  - it turns on `crawl` and `rpg`, and adds a stage with one small
    floor, the lines skin, the corner map, a party of one with a fixed
    member, one enemy kind and one encounter table;
  - it copies the genre's habits into the new game as code and data
    the game owns and edits: the trapped chest's rules and screen, and
    a `places.ron` with a tavern, an inn, a temple and a shop. A game
    that wants none of them deletes them; the engine has no such types
    (ADR-0033 §17);
  - a game made with it builds and its test walks to the stairs; add it
    to CI's `templates` job;
  - if 2502 is done, `shuyi new demo --kind dungeon-crawler` makes it;
    if not, `shuyi new demo --set kind=dungeon-crawler` does.
- **The guide** `docs/guides/dungeon-crawler.md`, in 2004's order:
  which template part starts it; the modules (`shuyi-crawl`,
  `shuyi-rpg`) and the kits; what looks related but isn't for it (3D
  free movement, 3106; top-down field of view, 2120; the roguelike
  generators, 2119); which example to read first; how to go from a party
  of one to a full party; how to swap the lines skin for pictures or 3D.
- **The READMEs** of both crates, finished in ADR-0009 §4's order, with
  the WASM size each adds, measured on the example.
- **The skills** `shuyi-crawl` and `shuyi-rpg`, finished with a worked
  example each (if 1134 is done; else on 1134's list).
- **The catalog**: both rows and both `kind` answers ready.

**Out (do not do):**
- the 3D skin in the example (2753 adds it as a third look, when 3D is
  unparked);
- the explorer bot and the playtest (2755);
- music beyond the engine's existing cues; voice;
- a video opening (2909).

## Implementation steps

1. The data: maps, defs, tables, hub, make.
2. The game's rules and `PartyMath`.
3. The stages: opening, making, town, labyrinth, battle, chest, map.
4. The skins and the example's pictures.
5. Tests at both party sizes, the web build.
6. The template part, its CI job, the guide, the READMEs, the skills,
   the catalog.

## Acceptance criteria

- [ ] The Harness test reaches floor 2's stairs with a party of one and
      with six.
- [ ] The replay test passes for both.
- [ ] `frame-png` pictures as listed.
- [ ] The web build runs in the browser pane.
- [ ] CI makes a game from the `kind-dungeon-crawler` part, builds it,
      and its test passes.
- [ ] `cargo xtask check-modules` passes with both crates ready and the
      guide found.
- [ ] Every asset not made for the example is in
      `THIRD_PARTY_ASSETS.md`.
- [ ] Nick has signed off on the feel (the PR says so).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: Harness runs, replays and `frame-png` at both sizes; the
  template's game in CI.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
