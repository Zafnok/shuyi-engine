---
id: "2024"
title: "The engine's other config and views moved onto constructors: the types the templates and hello-grid build by literal in shuyi-ui, shuyi-platform, shuyi-assets and shuyi-core"
type: infra
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["2013"]
nick_input: none
completed:
---

# 2024 — The engine's other config and views moved onto constructors (ui, platform, assets, core)

## Context

[ADR-0038](../../docs/adr/0038-public-types-that-can-grow-non-exhaustive-config-and-views-with-constructors-plain-values-left-open.md)
(ticket 2013) sorts public types into eight kinds and gives each a rule.
2013 moved `shuyi-app`'s `Window`, `Icon`, `VoiceFiles` and `AppConfig`,
`shuyi-ui`'s `BoxPlace` and `MapScene`, and `LogLine`, `LogEntry`,
`ContentError` and the codes' `Entry` under it. The ADR's table lists
the other engine types that the templates and hello-grid build by
literal, with the ticket that moves each: this one has the rows of
`shuyi-ui`, `shuyi-platform`, `shuyi-assets` and `shuyi-core`; 2025 has
`shuyi-cli`'s. Until a type moves, each field added to it breaks every
game that builds it by literal.

## Nick input

None.

## Scope

**In:** the rows of ADR-0038's table marked 2024, each as its kind says:

- **Kind 2** (config, `#[non_exhaustive]`, a `new` taking what every
  game must choose, a `const fn` where it can be, and `#[must_use] pub
  const fn with_<field>(self, value) -> Self` for an optional field
  games set in `const` items): `Looks`, `MapColors`, `TerrainGlyphs`
  (`crates/ui/src/grid/skin.rs`), `Look`
  (`crates/platform/src/platform.rs`), `audio::Files` and `SoundCue`
  (`crates/assets/src/audio.rs`), `MapFormat`
  (`crates/assets/src/grid_map.rs`), `TerrainTiles`
  (`crates/assets/src/tileset.rs`), and, as kind 5 types built like
  kind 2, `DialogueTable`, `ImageTable`, `ImageInfo`, `Tries`,
  `BalanceSettings`, `RolloutSettings` and `Condition`. A field read
  from a file stays `#[serde(default)]` where it is.
- **Kind 3** (views, `#[non_exhaustive]`, a `new` for what the type
  can't be without, the rest assigned; no setters): `TextView`,
  `WaitView` (`crates/ui/src/dialogue/text_box.rs`), `CursorView`
  (`crates/ui/src/grid/scene.rs`), `Change`
  (`crates/core/src/bots/balance.rs`), `PlayError`
  (`crates/core/src/bots.rs`) and `UnknownEntrant`
  (`crates/core/src/bots/playtest.rs`).
- **Kind 6** (enums that grow): `RowValue`
  (`crates/ui/src/screens/options.rs`), `Check` and `PlaytestError`
  (`crates/core/src/bots/playtest.rs`) get `#[non_exhaustive]`. A
  struct-like variant that may gain fields gets it too, with a
  constructor function on the enum for games to build it with
  (`RowValue::choice(..)`, say), since a non-exhaustive variant can't
  be built outside the crate.
- Every literal of these types outside their crate moves to the
  constructors: `templates/`, `examples/hello-grid/` (`src/` and
  `tests/`), and whatever else the build finds. Literals inside the
  defining crate may stay.
- If the PR goes past about 600 changed lines, stop at a crate boundary
  and write the rest as a ticket of its own.

**Out (do not do):**

- `shuyi-cli`'s rows (2025).
- The table's kind 1 types (`Cell`, `ImageRect`, `Pos`, `Band`): they
  stay exhaustive.
- The exhaustive lints outside `shuyi-app` (ADR-0038 says why).
- Renaming anything, or changing what a type holds.

**For the game:** a break, once. Outside their crates these types can no
longer be built by a struct literal. The game moves to their
constructors. The completion notes carry one `Breaking:` line per crate
in 2010's form. The game's PR that moves the pin follows this one.

## Implementation steps

1. Read ADR-0038. For each type, read its definition and every literal
   of it outside its crate (`git grep -n "TypeName {"`).
2. Add `#[non_exhaustive]` and the constructor(s), with doc comments
   saying which fields `new` sets and to what.
3. Move the literals. Run `cargo fmt --all`; keep `templates/` formatted
   by hand.
4. Run the template gate for `grid` and `blank` as CI's `templates` job
   does, and the gates.

## Acceptance criteria

- [ ] Every row of ADR-0038's table marked 2024 is `#[non_exhaustive]`
      with its constructors, or the completion notes say which moved to
      a new ticket and its number.
- [ ] No file under `templates/` or `examples/` builds those types by a
      struct literal.
- [ ] Games made with `shuyi new grid` and `shuyi new blank` pass the
      template gate; hello-grid's snapshots don't change.
- [ ] The completion notes carry the `Breaking:` lines.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each `new` gives what the literal it replaces gave (an
  `assert_eq!` against a literal inside the crate), and each
  `with_<field>` sets its field in a `const` item.
- Integration: the template gate; hello-grid's tests and snapshots
  unchanged.

## Completion notes

*(Filled in by the session that completes the ticket.)*
