---
id: "2129"
title: "Playtest targets by tier: what a success is, a band per tier, floors and median comparisons in one file, turned into a stage's checks"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2129 — Playtest targets by tier

## Context

A request from the game.

1. **The game ticket:** game#0506, the persona playtest bots
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0506-persona-bots-casual-normal-hardcore.md`),
   built on 2125, 2126 and 2128. The game's follow-up that moves onto
   this ticket is game#1180. Neither is a blocker of this one.
2. **The job in one sentence:** say once, in a data file, what each
   playtest bot should achieve on a stage of each difficulty tier (what
   a success is, the share of successes wanted on an easy, a normal or
   a hard stage, the share under which a change is blocked, and which
   bot should be faster than which), and get the checks of one stage
   from it.
3. **No friction review triggered:** game#0506 measured 1,060 added
   production lines against an estimate of 600 (1.8×, under 2×). The
   game's owner asked afterwards (Nick, 2026-10-22) that the generic
   parts of its automated playtesting move into the engine, so other
   games get playtest personas and a playtest command out of the box.
   This is one of the tickets of that sweep of the playtest area (game
   `docs/engine/friction.md`, row "0506 (sweep)"). The game's
   `Targets` (`crates/bots/src/playtest.rs`, 46 production lines) and
   its `targets.ron` do this job; with this ticket the game keeps the
   file and about 5 lines that read it.
4. **The generic shape:** a playtest targets file over a game's own tier
   type `T`: per entrant, the success conditions (2126's `Condition`s);
   per tier and entrant, a `Band`; per entrant, an optional floor; a
   list of median comparisons (2126's `Check::Median` fields). Given a
   stage's tier and the entrants that play, it gives 2126's `Check`s:
   one `Rate` per entrant that has both a success rule and a band for
   that tier, and each median comparison whose two entrants both play.
   Nothing in it names a tactics idea: a tier is whatever the game's
   stages are labelled with.
5. **Examples from games that aren't tactics:** a puzzle game whose
   levels are tagged easy, medium and hard, with a "casual solver" bot
   that should clear 70–90% of easy levels and 30–50% of hard ones; a
   roguelike whose floors come in depth tiers, with a cautious bot
   whose median run length should beat a reckless bot's; a racing game
   with cups by class. Each writes one success rule per bot and one row
   of bands per tier.

2126's `playtest.ron` (a flat `Vec<Check>`, hello-grid's) stays: a game
with one stage, or no tiers, keeps writing checks directly.

## Nick input

None.

## Scope

**In:**
- `shuyi_core::bots::playtest::Targets<T>` and `MedianTarget`
  (`crates/core/src/bots/playtest.rs`, or a `targets` submodule of it if
  the file passes about 800 lines), serde and, behind the `schema`
  feature, `JsonSchema`, like the module's other types.
- `Targets::checks(&self, tier: &T, entrants: &[&str]) -> Vec<Check>`.
- `Targets::unknown_entrants(&self, known: &[&str]) -> Vec<String>`: the
  names the file uses that the game has no entrant for (a misspelt
  persona would otherwise just get no check).
- Docs: the module docs, and `docs/contract.md`'s `playtest` row of the
  game's parts (line "its `Entrant`s, `Measure` and checks"): checks
  may come from a flat list or from targets by tier.

**Out (do not do):**
- Reading the file: the core has no I/O. A game reads it with `ron`
  (as hello-grid reads `playtest.ron`), and `shuyi playtest` (2532)
  takes the checks from the game.
- Changing `Check`, `Band`, `Condition` or the report (2126's).
- Changing hello-grid's `playtest.ron`.
- The game's use of it (game#1180 moves the pin after this merges).

## Implementation steps

1. **Read** `crates/core/src/bots/playtest.rs` and its tests, and the
   game's `crates/bots/src/playtest.rs` (`Targets`, `Faster`,
   `Targets::checks`) and `crates/bots/targets.ron` for the shape this
   generalizes (game repository, `main`).
2. **The types**:
   ```rust
   #[serde(deny_unknown_fields)]
   pub struct Targets<T: Ord> {
       /// What makes a try a success, per entrant.
       pub success: BTreeMap<String, Vec<Condition>>,
       /// The share of successes wanted, per tier and entrant.
       #[serde(default)]
       pub bands: BTreeMap<T, BTreeMap<String, Band>>,
       /// The share under which a run is blocked, per entrant that has one.
       #[serde(default)]
       pub floors: BTreeMap<String, u32>,
       /// Medians compared between two entrants.
       #[serde(default)]
       pub medians: Vec<MedianTarget>,
   }
   pub struct MedianTarget { pub entrant: String, pub measure: String,
       #[serde(default)] pub among: Vec<Condition>, pub want: Want, pub than: String }
   ```
   Serde bounds: derive with `#[serde(bound(deserialize = "T: Ord +
   Deserialize<'de>"))]` (and the matching serialize bound) so `T` needs
   no extra traits for other uses.
3. **`checks`**: walk `entrants` in the order given; for each, a
   `Check::Rate` when it has a `success` entry **and** a band in
   `bands[tier]`, with `floor: floors.get(name).copied()`. Then each
   `MedianTarget`, in file order, whose `entrant` and `than` are both in
   `entrants`, as a `Check::Median`. An entrant with no target (a
   baseline) gets none; a tier with no row gives no `Rate` checks.
4. **`unknown_entrants`**: every name in `success`, the inner maps of
   `bands`, `floors`, and each median's `entrant` and `than` that is not
   in `known`, sorted, without repeats.
5. **A fixture**: a RON text with a two-variant tier enum (`Easy`,
   `Hard`, deriving `Ord`) declared in the test, three entrants and one
   median, parsed with `ron` (already a dev-dependency of the core).
6. **Docs**: module docs gain a paragraph and a short RON example;
   `docs/contract.md` names the two ways to give checks.

## Acceptance criteria

- [ ] `Targets::<Tier>::checks(&Tier::Easy, &["a", "b", "base"])` on the
      fixture gives `Rate` checks for `a` and `b` (in that order, with
      `a`'s floor and none for `b`) and none for `base`, then the median.
- [ ] A median whose `than` doesn't play is left out; a tier missing
      from `bands` gives only the medians.
- [ ] `unknown_entrants(&["a", "b"])` on a file that names `c` in a band
      and `d` in a median gives `["c", "d"]`.
- [ ] An unknown field in the file is an error (`deny_unknown_fields`).
- [ ] With the `schema` feature, `Targets<String>` derives a schema.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the criteria above, in `crates/core/src/bots/playtest/tests.rs`.
- Property: for any targets and entrant list, every check `checks`
  returns names only entrants in the list (so 2126's `judge` never
  fails with `UnknownEntrant` on them).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
