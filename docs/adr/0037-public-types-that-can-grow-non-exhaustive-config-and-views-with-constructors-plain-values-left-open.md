# ADR-0037: Public types that can grow: non-exhaustive config and views with constructors, plain values left open

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related tickets:** 2013 (this), 2010 (the public API gate), 2005
  (the version policy and 0.1), 2304, 2305 and 2321 (new fields on
  `MapScene`, `BoxPlace` and `Window`), 2511 (`BasicGame`), 2803 (the
  tutorial's box-pushing puzzle), 3044, 3046 and 3048 (more on `Window`
  and `AppConfig`), 2024 and 2025 (the follow-ups)

## Context

The game pins the engine by revision, and every game made with
`shuyi new` copies the templates' code. Each engine change that adds a
field to a struct a game builds by literal is an edit in every game at
its next upgrade. Three MVP tickets add one each: 2304 (`MapScene`),
2305 (`BoxPlace`) and 2321 (`Window`). Nick's end goal (2026-10-19) is
that a game an agent wrote today still compiles after an engine upgrade
that only adds.

- **Rust's rules.** The Cargo book's SemVer chapter
  (<https://doc.rust-lang.org/cargo/reference/semver.html>) counts a new
  public field on a struct whose fields are all public, and a new
  variant on an enum without `#[non_exhaustive]`, as major changes, and
  advises `#[non_exhaustive]` from the start on types that will grow.
  `#[non_exhaustive]` stops struct literals, `..` updates and patterns
  without `..` outside the defining crate; reading and assigning public
  fields still work. Clippy's restriction lints `exhaustive_structs` and
  `exhaustive_enums` flag every exported type without it.
- **Other engines.** Unreal never changes a public header in a hotfix
  release; Godot keeps the old method as a compatibility method when a
  signature changes. Bevy does the opposite (public fields, `Default`,
  `..default()`, and a migration guide each release).
- **Shuyi before this ADR.** Seven `#[non_exhaustive]` attributes, all
  in `shuyi-platform` (`Card`, `Setup`, `DisplayInfo`, `Show`,
  `Show::Game`, `Frame`, `SystemEvent`; ADR-0024 explains `Frame` and
  `DisplayInfo`), and `LogLevel` and `LogCategory` in `shuyi-ui` (3040).
  Constructors came one at a time, after a break (2313's
  `GlyphSkin::new`). No ADR set a rule.
- **The lints, measured** with the pinned clippy (1.98.1) on 2026-10-20
  over the `--lib` targets:

  | Crate | `exhaustive_structs` | `exhaustive_enums` |
  | ----- | -------------------- | ------------------ |
  | `shuyi-core` | 10 | 2 |
  | `shuyi-assets` | 99 | 24 |
  | `shuyi-ui` | 77 | 53 |
  | `shuyi-platform` | 5 | 3 |
  | `shuyi-app` | 4 | 0 |
  | `shuyi-cli` | 64 | 10 |

  About 350 in all, most of them on plain values (`Pos`, `Rect`, `Rgb`,
  `Cell`) and closed sets (`Dir`) that rightly stay exhaustive.
  `shuyi-app`'s four (`Window`, `Icon`, `VoiceFiles`, `AppConfig`) are
  all config a game fills in.
- **Clippy's `field_reassign_with_default`** (style, on by default)
  stays quiet on `let mut input = RunInput::default(); input.frames =
  3;` when `RunInput` is `#[non_exhaustive]` and in another crate:
  tried on `shuyi-cli` with `cargo clippy -p shuyi-cli --all-targets --
  -D warnings` on 2026-10-23 (the lint skips a type the caller can't
  build by literal).

## Decision

Every public type is one of eight kinds, each with a rule. None of them
costs a small game (the tutorial's box-pushing puzzle, 2803) a line:
where a literal named every field, a constructor names the same values
in one call.

1. **Plain values** (`Pos`, `Rect`, `Rgb`, `Cell`, `Millis`,
   `TerrainId`, `MapEdges`, `Code`): the fields are the whole meaning,
   and another field would make another type. They stay exhaustive,
   with public fields, built by literal or by their `const fn new`.
2. **Config a game fills in** (`Window`, `Icon`, `VoiceFiles`,
   `AppConfig`, `BoxPlace`): `#[non_exhaustive]`, fields public (read
   and assigned). A `new` takes the fields every game must choose, and
   is a `const fn` whenever it can be. An optional field starts at its
   default in `new`; a game changes it by assigning the field or, where
   games build the type in a `const` item, with a `#[must_use] pub
   const fn with_<field>(self, value: <the field's own type>) -> Self`,
   taking the field's own type so generated code can pass an `Option`
   it holds:

   ```rust
   const WINDOW: Window = Window::new("Boxes", 784, 784).with_icon(None);
   const TALK: BoxPlace = BoxPlace::new(Rect::new(0, 17, 48, 7), 2, 19)
       .with_narration(NarrationAlign::Left);
   ```

   `..Default::default()` isn't `const`, and `#[non_exhaustive]` forbids
   `..` outside the crate anyway. No builder types.
3. **Views and scenes a screen fills in and a skin reads** (`MapScene`,
   `LogLine`, `LogEntry`, the codes' `Entry`): `#[non_exhaustive]`, with
   a `new` for what the type can't be without; the rest starts empty
   and is assigned (`let mut scene = MapScene::new(origin, size);
   scene.cursor = Some(..);`). No setters.
4. **Inputs with a `Default`, never built in `const`** (the contract's
   `RunInput`, `CheckInput`, `BotsInput`): `#[non_exhaustive]`, and a
   caller writes `let mut input = RunInput::default();` then
   `input.frames = 3;`. Clippy stays quiet on that (*Context*). With
   exhaustive types and `..RunInput::default()` instead, every new
   optional field would be a break under 2010's gate.
5. **Data read from files** (`PaletteDef`, `KeymapDef`,
   `AudioManifest`): a new field is `#[serde(default)]`, so old files
   still load (ADR-0007 and ADR-0019 don't change). In Rust such a type
   is kind 2 or kind 4, by how games build it.
6. **Enums.** A set that will grow (errors, events, kinds of thing, a
   setting's choices: `NarrationAlign`, `LogLevel`, `SystemEvent`) is
   `#[non_exhaustive]`. A closed set (`Dir`'s four directions) stays
   exhaustive. A struct-like variant that may gain fields is
   `#[non_exhaustive]` itself, as `Show::Game` is.
7. **Traits.** A new method on a trait a game implements (`GameData`,
   `Screen`, `GameTools`, `UnitPainter`) gets a default body, as
   `UnitPainter::paint_under` has. A trait games call but must not
   implement is sealed, decided per trait: today none is, and the
   candidates are `shuyi_ui::Float` and `Round` (methods on `f32` and
   `f64`). `shuyi-cli`'s seams (`Probe`, `Cargo`, `Session`, `Link`,
   `Launch`, `Forward`, `Forwarding`) are implemented by its own
   integration tests, so they stay open. A trait backends implement
   (`Platform`) may take a required method where a default would be
   wrong; that method's ticket says why (3053's `Platform::facts`).
8. **A private field** (`Settings::layout`, `LogRing`, `LogFilter`,
   `BasicData`, `BasicTools`) is the other way, for a type whose fields
   keep an invariant or that a game never reads field by field. Its
   optional parts are set by `#[must_use]` methods that take and return
   `self`, named for what they set: 2511's `BasicGame` (private fields,
   `BasicGame::new` and `with_own`, and `console` and `screens`), or
   `ContentError::new(..).at(..).with_code(..)`.

**When.** A new public type follows this ADR in the PR that adds it. An
existing type moves under it in the PR that next adds to it, or in its
follow-up (2024 for `shuyi-ui`, `shuyi-platform`, `shuyi-assets` and
`shuyi-core`; 2025 for `shuyi-cli`), once, with a `Breaking:` line in
2010's form.

**How it is held.**

- In `shuyi-app`, by `#![warn(clippy::exhaustive_structs,
  clippy::exhaustive_enums)]` at the top of `crates/app/src/lib.rs`, with
  no `#[allow]` or `#[expect]`. It is a crate attribute, not in the
  crate's `Cargo.toml`, because that `[lints]` table must stay a copy of
  the workspace's (`the_lints_are_the_workspace_lints_except_unsafe_code`).
  A new crate whose public types are all config turns them on the same
  way.
- Elsewhere the lints would fire about 350 times, mostly on kinds 1 and
  6's closed sets, and quieting each with `#[expect]` would bury the
  real ones. So the rule is held by the `write-ticket` and `work-ticket`
  skills' lines and by 2010's job, whose cargo-semver-checks names the
  very break this rule prevents (a field added to a struct a game can
  build, a variant added to an exhaustive enum).

**The types the templates and hello-grid build by literal** (in `src/`
and `tests/`, as of 2026-10-23), their kind, and when each changes:

| Type | Crate | Kind | Changes |
| ---- | ----- | ---- | ------- |
| `Window` | `shuyi-app` | 2 | here |
| `AppConfig` | `shuyi-app` | 2 | here |
| `BoxPlace` | `shuyi-ui` | 2 | here |
| `TextView`, `WaitView` | `shuyi-ui` | 3 | 2024 |
| `CursorView` | `shuyi-ui` | 3 | 2024 |
| `Looks`, `MapColors`, `TerrainGlyphs` | `shuyi-ui` | 2 | 2024 |
| `RowValue` (struct-like variants) | `shuyi-ui` | 6 | 2024 |
| `Cell` | `shuyi-ui` | 1 | stays |
| `Look` | `shuyi-platform` | 2 | 2024 |
| `audio::Files`, `SoundCue` | `shuyi-assets` | 2 | 2024 |
| `MapFormat`, `TerrainTiles` | `shuyi-assets` | 2 | 2024 |
| `DialogueTable`, `ImageTable`, `ImageInfo` | `shuyi-assets` | 5 (as 2) | 2024 |
| `ImageRect` | `shuyi-assets` | 1 | stays |
| `Pos`, `Band` | `shuyi-core` | 1 | stays |
| `Tries`, `BalanceSettings`, `RolloutSettings`, `Condition` | `shuyi-core` | 5 (as 2) | 2024 |
| `Check`, `PlaytestError` | `shuyi-core` | 6 | 2024 |
| `Change`, `PlayError`, `UnknownEntrant` | `shuyi-core` | 3 | 2024 |
| `RunInput`, `CheckInput`, `ReplayInput`, `ImportInput`, `PlaytestInput`, `AutobalanceInput` | `shuyi-cli` | 4 | 2025 |
| `BotsInput`, `NewInput` | `shuyi-cli` | 4 (with a `Default` added) | 2025 |
| `Engine` | `shuyi-cli` | 6 | 2025 |
| `GameArg` | `shuyi-cli` | 1 | stays |
| `GameLang`, `GameOption`, `GameBake`, `OneBitSkin`, `Tunable`, `AutobalancePlan`, `PlaytestPlan` | `shuyi-cli` | 2 | 2025 |
| `Play`, `contract::session::Scene`, `Shot`, `Script`, `Project` | `shuyi-cli` | 3 | 2025 |

`Icon`, `VoiceFiles` and `MapScene` aren't built by literal there, and
change here too. So do the types 2509 and 3040 left to this ADR:
`ContentError` (kind 8, already built with `new`), `LogLine` (kind 3,
with `LogLine::new`), `LogEntry` and the codes' `Entry` (kind 3, built
only in `shuyi-cli`). `Code`, `LogLevel`, `LogCategory`, `LogRing`,
`LogFilter`, `BasicData` and `BasicTools` already follow it.

**Its neighbours.** 2010's ADR is the gate that sees a break; this one
makes most of them not happen. 2005's version policy cites this ADR and
doesn't restate it.

## Consequences

- After one break (this one), the new fields of 2304's kind, 2305's
  and 2321's are additive for every game, and so is every later field on
  a type under this ADR.
- A game writes a constructor where it wrote a literal. Where it wrote
  `voices: None` and `text_input: None`, it writes nothing.
- A type of kind 2 or 3 can no longer be matched without `..` outside
  its crate. Its own crate's code and tests still build it by literal.
- `const` construction stays possible for kind 2, at the price of a
  `with_<field>` per optional field that games set in `const` items.
- Outside `shuyi-app` the rule is held by review lines and 2010's
  report, not by a lint, so a slip is caught at the PR that makes it, by
  a `Breaking:` line it must write.
- 2024 and 2025 move the rest of the table, once, with their own
  `Breaking:` lines.

## Alternatives considered

- **Bevy's way: public fields, `Default` and `..default()`.** Every new
  field breaks a `const` item (`..` with `Default` isn't `const`), and
  under `#[non_exhaustive]` `..` doesn't compile at all; without it,
  each new field is a break. It lost on the end goal.
- **Builder types** (`WindowBuilder`). A second type per config, more
  lines in every game, and no gain over `new` plus `with_<field>`
  methods on the type itself.
- **Private fields everywhere.** Games read and assign these fields
  (`scene.cursor = ..`, `config.voices = ..`); getters and setters for
  each would double the API. Kept for kind 8 only.
- **The two lints in every crate.** About 350 findings, most of them
  rightly exhaustive types that would each need an `#[expect]`, which
  hides the few that matter and is against rule 6's spirit.
- **The lints in `shuyi-app`'s `Cargo.toml`.** Its `[lints]` table is a
  checked copy of the workspace's; a crate attribute keeps the copy
  exact.
