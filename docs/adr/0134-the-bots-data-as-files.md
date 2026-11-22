# ADR-0134: The bots' data as files: personas are a planner's, every settings type refuses unknown fields and has a schema, and a try's bots draw from a seed of their own

- **Status:** Accepted
- **Date:** 2026-11-21
- **Related tickets:** 2133 (this), 2125 (the rollout bot), 2126 (the playtest runner), 2127 (autobalance), 2128 (the turn planner), 2129 (targets by tier), 2130 (weighted features and personas), 2113 (minimax and MCTS)
- **Supersedes:** [ADR-0027](0027-bots-that-plan-see-a-determinized-copy-integer-rollouts-shared-seeds-and-work-units.md)'s sentences on personas (§4 and §8)

## Context

- Seven PRs built `shuyi_core::bots` in a day, one ticket each. A retro
  of them (2026-10-22) found the data shapes they added (checks, targets,
  weights and personas in RON, the `schema` feature) recorded in no ADR,
  and small drifts between sibling modules (ticket 2133).
- [ADR-0027](0027-bots-that-plan-see-a-determinized-copy-integer-rollouts-shared-seeds-and-work-units.md)
  §4 says `RolloutSettings` are serde data "so a persona's are a file",
  and §8 says the playtest runner "plays personas made of `Rollout`
  bots". The game moved its personas to the turn planner
  ([ADR-0029](0029-the-turn-planner-a-beam-over-the-sides-own-commands-copies-kept-and-the-best-continuation-backed-up.md)'s
  Context), and 2130 made `Persona<X>` hold `settings: PlanSettings`.
- Every playtest, targets and balance type was
  `#[serde(deny_unknown_fields)]` and derived `schemars::JsonSchema`
  behind the `schema` feature of `shuyi-core`. `RolloutSettings` and
  `PlanSettings` had neither, so a typo inside a persona's `settings:
  (…)` was dropped without a word, and `Persona` had no schema.
- `play_tries` hands a try's `seed` to `Entrant::start` and to
  `Entrant::bots`. Hello, Grid's random hero was `RandomBot::new(seed)`
  while its stage drew its luck from `SimRng::new(seed)` too: the hero's
  picks and the foe's luck read the same stream. ADR-0027 §6 already
  derives a seed for each copy from two draws.

## Decision

1. **A persona is a planner's.** `shuyi_core::bots::weigh::Persona<X>`
   is a `TurnPlan`'s `PlanSettings`, its weights by feature name, and
   the game's own extras `X`; a map of them by name, in RON, is a
   personas file (2130). This replaces ADR-0027 §4's "so a persona's are
   a file" (now: `RolloutSettings` are data, so a bot's settings can be a
   file) and §8's "personas made of `Rollout` bots" (now: the playtest
   runner plays whatever bots a game's `Entrant` builds, personas among
   them, and a persona is a planner). A rollout persona is not part of
   this; a game that wants one writes its own type around
   `RolloutSettings`.
2. **The bots' data are RON files.** A playtest's checks (`Vec<Check>`,
   2126), targets by tier (`Targets<T>`, 2129), weights and personas
   (2130) and the balance search's settings (2127) are serde types read
   from the game's RON. The format is the text one; nothing else is
   derived from them (ADR-0007 has nothing to bake).
3. **Every settings type refuses unknown fields.** Each type of those
   files, and `RolloutSettings` and `PlanSettings` inside them, is
   `#[serde(deny_unknown_fields)]`: a misspelt field is an error naming
   it, never a default. A name the game declares elsewhere (an entrant,
   a feature) is checked by the code that knows the names:
   - `judge` refuses the first check naming an unknown entrant, with
     `UnknownEntrant`;
   - `Targets::unknown_entrants` lists every unknown name at once, for a
     lint over the whole file;
   - `Weights::from_named` refuses the first unknown feature, with
     `UnknownFeature`.

   `UnknownEntrant` and `UnknownFeature` are `core::error::Error`.
4. **The `schema` feature gives every one a JSON schema.** Each of those
   types, `Persona<X>` (for `X: JsonSchema + Default + Serialize`),
   `RolloutSettings`, `PlanSettings` and `Horizon` included, derives
   `schemars::JsonSchema` behind `#[cfg_attr(feature = "schema", …)]`,
   so a tool or an editor checks a file without the game's code. 2113's
   `MinimaxSettings` and `MctsSettings` take both attributes when they
   land.
5. **A try's bots draw from `bot_seed(seed)`.**
   `shuyi_core::bots::playtest::bot_seed(seed)` is two `next_u32` of
   `SimRng::new(seed)`, the high half first, as ADR-0027 §6 draws a
   copy's seed. `Entrant::bots` still gets the try's own seed, the one
   `start` gets; a game's entrant that seeds a bot from it passes
   `bot_seed(seed)`, so the bot's picks don't read the stage's luck.
   `play_tries` doesn't change the seed it hands over: the entrant
   decides. Hello, Grid and the grid template do so.
6. **A band's arithmetic is in one place.** `Band::distance(successes,
   count) -> u64` is how far a share of tries lies outside a band, 0
   inside, comparing `successes × 100` with `bound × count`;
   `Band::verdict`, a floor's check in `judge` and
   `balance::distance` all use it or the one comparison beside it.

## Consequences

- A typo in a persona's settings, a targets file or a playtest's checks
  fails at load, naming the field.
- A persona file can be checked against a schema, as the playtest's
  checks and targets could already.
- Hello, Grid's random hero won 6 of seeds 0–49 with its own seed,
  against 5 before: a game that moves a bot onto `bot_seed` sees its
  counts move, and re-reads its bands (they describe the stage, not the
  bot's stream).
- Adding `deny_unknown_fields` to `RolloutSettings` and `PlanSettings`
  refuses a file that loaded before with an extra field: the right
  outcome, and a change a game sees.

## Alternatives considered

- **`play_tries` hands `Entrant::bots` the derived seed itself.** Every
  game would get it without a change, but the seed a bot sees would no
  longer be the try's, and a game's record of a try (`Entrant::record`)
  names the try's seed: one seed per try is simpler to replay.
- **Rollout personas beside planner ones.** No game asks for one, and
  `Persona` would need an enum of settings; it can come with the game
  that needs it.
- **Keeping `RolloutSettings` lenient, for older files.** A dropped
  field is a bot that plays other than its file says, found late.
