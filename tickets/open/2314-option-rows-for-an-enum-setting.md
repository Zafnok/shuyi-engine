---
id: "2314"
title: "Option rows for an enum setting: one call from the values, their text keys and a getter and setter"
type: feature
milestone: After MVP
model: sonnet-5
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2314 — Option rows for an enum setting

## Context

- **The game ticket:** game#1168
  ([`1168-options-rows-on-the-engines-enum-rows.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1168-options-rows-on-the-engines-enum-rows.md))
  waits for this one. Found by the game's first engine-friction sweep,
  game#1153.
- **The job:** "offer one of a setting's values on the options screen".
- **Friction:** the game's options screen, 519 lines against an
  estimate of 300 (1.7×), outcome 2. `RowValue::Choice`
  (`crates/ui/src/screens/options.rs`) takes plain function pointers
  (`values`, `get`, `set` by index), which capture nothing. So for each
  enum setting the game writes the list of values, their text keys, and
  index lookups both ways; to share that it wrote a `Named` trait, five
  impls and a `choice()` helper (84 lines). The engine's own
  `OptionRow::text_speed` writes the same lookup by hand.
- **The generic shape:** a trait `ChoiceSetting: Copy + PartialEq +
  'static` with `const ALL: &'static [Self]` and `fn key(self) ->
  &'static str`, and `OptionRow::choice::<T: ChoiceSetting>(key, get:
  fn(&Ctx<G>) -> T, set: fn(&mut Ctx<G>, T))` building the
  `RowValue::Choice` (the index plumbing in one place, `set` returning
  whether it changed).
- **Outside tactics:** any game's difficulty, text speed, window mode
  or color theme.

## Nick input

None.

## Scope

**In:**
- The trait and the builder, documented.
- `text_speed` (and any other engine row of the same shape) uses it.

**Out (do not do):**
- Closures in `RowValue` (a bigger change; the trait covers the case
  found).
- Any game's rows.

**For the game:** additive. The game's PR that moves its pin follows
this one (game#1168).

## Implementation steps

1. Write the trait and `OptionRow::choice`.
2. Move `text_speed` onto it; its tests and snapshots unchanged.

## Acceptance criteria

- [ ] A test builds a row from a three-value test enum, steps it both
      ways and reads it back through `get`.
- [ ] The options screen's tests and snapshots are unchanged.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, as above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
