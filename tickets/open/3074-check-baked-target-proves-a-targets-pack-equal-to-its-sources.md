---
id: "3074"
title: "check --baked --target <id> proves a target's pack equal to its sources"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["3007"]
nick_input: none
completed:
---

# 3074 — `check --baked --target <id>` proves a target's pack

## Context

[ADR-0023](../../docs/adr/0023-the-pack-one-indexed-file-of-baked-assets-proven-equal-to-its-sources.md),
ADR-0007 §4. `shuyi check --baked` bakes the game in memory and proves
every asset read from the pack equal to its source
(`contract::bake::check_baked`, `crates/cli/src/contract/bake.rs`). It
bakes for no target only: `check_baked` calls `GameTools::bake(None)`,
and `check`'s arguments take `--baked` or `--target <id>`, never both
(`contract::check::parse_args`).

**From 3007 (done):** a target's pack can differ from the default one.
A one-bit target's bake keeps every picture as a one-bit picture file
(`Bake::under_as`), from the target's own layer (`Bundle::for_target`),
and `Bake::check` proves each equal to dithering its PNG again. `shuyi
bake --target playdate` never runs that check, so a game's CI can't ask
for it, short of a test of its own (Hello, Grid has one,
`tests/it/onebit.rs`).

## Nick input

None.

## Scope

**In:**
- `check --baked --target <id>`: bakes for that target
  (`GameTools::bake(Some(id))`) and checks the pack, after the target's
  profile is found as `bake --target` finds it (`target::profile`); an
  unknown target is the same error as `bake`'s.
- `--target all --baked`: each target of `shuyi.ron`'s list.
- `CheckInput` gains nothing: both fields exist; `parse_args` takes both
  flags in either order. The contract's version goes up only if a
  schema changes.
- `docs/contract.md`'s `check` row, and the `shuyi-ship` skill's step on
  packs.

**Out (do not do):**
- `check --target` without `--baked` changes nothing;
- `bake` running the check itself.

## Implementation steps

1. `check_baked(game, target: Option<&str>)` in
   `crates/cli/src/contract/bake.rs`, passing the target to `prepared`
   and `bake.bake`.
2. `contract::check::check`: with `baked` and `target`, run it per
   target (each of `all`); `parse_args` takes `--baked --target <id>`.
3. Tests on the fake game (`contract/fake.rs`) and on Hello, Grid
   (`check --baked --target playdate` passes).

## Acceptance criteria

- [ ] `cargo shuyi check --baked --target playdate` on Hello, Grid
      passes, and fails when a picture in the pack differs.
- [ ] An unknown target is refused as `bake --target` refuses it.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the arguments, both orders; a target with no profile.
- Integration: Hello, Grid's Playdate pack checked through `check`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
