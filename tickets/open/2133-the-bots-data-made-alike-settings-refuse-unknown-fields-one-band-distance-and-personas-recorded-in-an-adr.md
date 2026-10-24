---
id: "2133"
title: "The bots' data made alike: the settings refuse unknown fields and have schemas, one band distance, unknown names as errors, a seed for an entrant's bots, and personas recorded in an ADR"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2133 — The bots' data made alike

## Context

Seven PRs built `shuyi_core::bots` in a day, one ticket each: the rollout
bot (2125, ADR-0027), the playtest runner (2126), the turn planner (2128,
ADR-0029), the autobalance search (2127), targets by tier (2129) and
weighted features with personas (2130). A retro of them (2026-10-22)
found the modules well layered: the later ones reuse the earlier ones'
types and code. It also found small drifts between sibling modules that
no one ticket owned:

- **Settings that ignore a typo.** Every playtest, targets and balance
  type is `#[serde(deny_unknown_fields)]` and derives `JsonSchema` behind
  the `schema` feature (`crates/core/Cargo.toml`). `RolloutSettings`
  (`bots/rollout.rs`) and `PlanSettings` (`bots/plan.rs`) have neither.
  So `Persona` (`bots/weigh.rs`) refuses an unknown field, but the
  `settings: (…)` inside it silently drops one, and `Persona` has no
  schema because `PlanSettings` has none (2130's completion notes say
  so).
- **Personas, as ADR-0027 still writes them.** ADR-0027 §4 says
  `RolloutSettings` are data "so a persona's are a file", and §8 says the
  playtest runner plays "personas made of `Rollout` bots". 2130 made
  `Persona<X>` a planner's: `settings: PlanSettings`, as its ticket asked.
  ADR-0029's Context says why the game moved to the planner, but no ADR
  supersedes ADR-0027's sentences, and `RolloutSettings`' doc comment
  still says "so a persona's settings can be a file". The data shapes
  2126, 2127, 2129 and 2130 added (checks, targets, weights and personas
  in RON, the `schema` feature) have no ADR at all.
- **The band arithmetic three times.** `Band::verdict`
  (`bots/playtest.rs`), the floor check inside `judge`, and
  `balance::distance` (`bots/balance.rs`) each compute
  `successes × 100` against `bound × count`. A property test keeps them
  in step ("distance is 0 iff every rate is within",
  `bots/balance/tests.rs`); one function would keep them in one place.
- **Unknown names, three ways.** `UnknownFeature` (`bots/weigh.rs`) is a
  `core::error::Error`. `UnknownEntrant` (`bots/playtest.rs`) has
  `Display` but no `Error`. `Targets::unknown_entrants`
  (`bots/playtest/targets.rs`) returns a `Vec<String>` of every unknown
  name, which is right for a lint over the whole file but isn't said.
- **An entrant's bots and its stage read the same seed.** `play_tries`
  passes a try's `seed` to `Entrant::start` and to `Entrant::bots`.
  Hello, Grid's `random` hero is `RandomBot::new(seed)`
  (`examples/hello-grid/src/tools.rs`, `ENTRANTS`), and its stage's luck
  is drawn from the same seed (`Stage::determinize`). So the hero's
  picks and the foe's luck read related streams. ADR-0027 §6 derives a
  seed of its own for each copy; `Entrant`'s docs say nothing.
- **Re-exports.** `lib.rs` re-exports the balance items at the crate's
  root, but not `weigh`'s or `playtest`'s, and its doc list of the bots'
  modules leaves out `weigh`.

## Nick input

None.

## Scope

**In:**
- `RolloutSettings` and `PlanSettings`: `#[serde(deny_unknown_fields)]`
  and `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]`, as
  `Targets` has. `Horizon` (inside `RolloutSettings`) gets the same
  `cfg_attr`, and `Persona` too.
- **Either order with 2113:** if 2113 is done, its `MinimaxSettings`
  and `MctsSettings` already have both attributes (2113 adds them):
  check, and add any that is missing; if not, nothing to do (2113 adds
  them when it lands).
- `RolloutSettings`' doc comment: "data, so a bot's settings can be a
  file". `PlanSettings`' keeps "a persona's".
- `Band::distance(self, successes: u32, count: u32) -> u64`: how far
  outside the band, 0 inside, in the integers `balance::distance`
  uses today. `Band::verdict` and `balance::distance` call it, and the
  floor check calls one small function beside it (`below(successes,
  count, floor)`), so the comparison is written once.
- `UnknownEntrant` implements `core::error::Error`. `Targets::
  unknown_entrants`' doc comment says it lists every unknown name at
  once for a lint, and that `judge` refuses the first with
  `UnknownEntrant`.
- `shuyi_core::bots::playtest::bot_seed(seed: u64) -> u64`: the seed a
  try's bots draw from, two `next_u32` of `SimRng::new(seed)` with the
  high half first (ADR-0027 §6's way), so their stream is not the
  stage's. `Entrant::bots`' doc comment says the `seed` is the try's,
  the same `start` gets, and that a bot seeded from it uses
  `bot_seed(seed)`. Hello, Grid's `random` hero uses it.
- **Either order with 2544:** the grid template's random bot is seeded
  with `bot_seed(seed)` too (`templates/grid/crates/game/src/tools.rs`),
  so a new game's bot doesn't read its stage's luck. If 2544 is done,
  that is the bot of its `playtest` entrant; if not, it is `bot_play`'s
  `RandomBot::new(seed)`, and 2544 keeps it when it moves the bot.
- `lib.rs`: re-export `Persona`, `Weighted`, `Weights` and
  `UnknownFeature` beside the balance items. The playtest items stay
  under `bots::playtest` (`run` and `judge` are too plain for the
  crate's root), and `lib.rs`' doc list names every module of `bots`
  and says where each one's items are.
- **An ADR** (the `write-adr` skill, the next free number): the bots'
  data as files. A persona is a planner's settings and weights (2130),
  checks and targets are RON (2126, 2129), the `schema` feature gives
  their JSON schemas, and every settings type refuses unknown fields.
  ADR-0027's status line becomes `Accepted; §8's personas superseded by
  ADR-NNNN`, and the new ADR says which sentences of ADR-0027 §4 and §8
  it replaces.

**Out (do not do):**
- moving the helpers the rollout bot and the planner share
  (`draw_seeds`, `checked_copy`, `apply_candidate`, `pick`) out of
  `bots/rollout.rs`: 2113 does that when it adds a third bot;
- changing what `Persona` holds, or adding rollout personas;
- `play_tries` changing the seed it hands `Entrant::bots`: a game's own
  entrant decides, with `bot_seed`;
- the CLI's `bots` and `playtest` operations (2544);
- `autobalance_by` and its stop check in `bots/balance.rs` (2541, done):
  of that file this ticket edits only `distance`, whose results don't
  change.

## Implementation steps

1. Read `crates/core/src/bots.rs`, `bots/rollout.rs`, `bots/plan.rs`,
   `bots/playtest.rs`, `bots/playtest/targets.rs`, `bots/balance.rs`,
   `bots/weigh.rs`, ADR-0027 and ADR-0029.
2. The derives and the doc comments. Test that a `Persona` RON with an
   unknown key inside `settings` fails to load, and that
   `schemars::schema_for!(Persona<()>)` builds with the `schema`
   feature.
3. `Band::distance` and the floor function; point `verdict`, `judge`
   and `balance::distance` at them. The existing property test stays.
4. `UnknownEntrant`'s `Error`, and `Targets::unknown_entrants`' comment.
5. `bot_seed`, `Entrant::bots`' comment, and Hello, Grid's `random`
   hero. Run `cargo shuyi playtest` in `examples/hello-grid` on seeds
   0–49 as its `playtest.ron` comment says, and write the new counts in
   that comment. If the `random` hero's rate leaves its band, set the
   band around the rate it now has as 2126 did, and say so in the
   completion notes: the band describes the stage, so this is not
   weakening a gate.
6. The re-exports and `lib.rs`' doc list.
7. The ADR, ADR-0027's status line, and `docs/adr/README.md`'s table.

## Acceptance criteria

- [ ] A `Persona` with an unknown field inside `settings` fails to
      deserialize, naming the field.
- [ ] `Persona<()>`, `RolloutSettings` and `PlanSettings` have JSON
      schemas with the `schema` feature.
- [ ] `successes × 100` against `bound × count` is written once in
      `bots/`.
- [ ] `UnknownEntrant: core::error::Error`.
- [ ] Hello, Grid's `random` hero is seeded with `bot_seed`, and its
      `playtest.ron` comment gives the counts on seeds 0–49.
- [ ] The grid template's random bot is seeded with `bot_seed`, and
      its record test still replays each record.
- [ ] The new ADR exists, and ADR-0027's status names it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the unknown field in a persona's settings; `Band::distance` at
  and around both edges; `bot_seed` is not `seed` and is the same for
  the same seed.
- Property: the existing "distance is 0 iff every rate is within" test
  still passes on the new function.
- Integration: Hello, Grid's playtest test (`tests/it/playtest.rs`)
  passes with the new seed.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
