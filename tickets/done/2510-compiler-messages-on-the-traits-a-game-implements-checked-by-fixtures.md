---
id: "2510"
title: "Compiler messages on the traits a game implements: on_unimplemented saying what to write and where to read, pinned by fixtures that check the message"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-27
---

# 2510 — Compiler messages on the traits a game implements

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6,
Unreal 5.8, GameMaker, Defold and Construct 3. Rust lets a library
replace the compiler's message when a type doesn't implement one of its
traits: `#[diagnostic::on_unimplemented(message, label, note)]`, stable
since Rust 1.78
(https://doc.rust-lang.org/reference/attributes/diagnostics.html).
Bevy puts it on `Component` and `Resource`, so a type used as one
without the derive is told that it is not a `Component` and to add the
derive. The repository's Rust is 1.98 (`rust-version` in `Cargo.toml`;
`rust-toolchain.toml` pins 1.98.1), so the attribute and its sibling
`#[diagnostic::do_not_recommend]` (stable since 1.85) are both there.

A new game on Shuyi implements engine traits from its first hour. The
grid game (`shuyi new grid`: `templates/shuyi-default/`, its `base` and
its `stages/top-down-grid` parts) writes `Screen<Data>`
(`base/crates/game/src/title.rs:52`, and the stage's
`crates/game/src/stages/top_down_grid.rs:332`), `Terrains` (`:133`),
`MapUnit` (`:180`), `UnitPainter<Piece>` (`:205`), `GameTools`
(`top_down_grid/tools.rs:173`), `MapFeature`
(`crates/rules/src/lib.rs:42`), `TurnRules` (`:221`) and `Sides`
(`:276`); its data is the engine's `BasicData<UiAction, Own>`, so it
writes no `GameData`, `ActionSet`, `ClipVariant` or `GameSettings`.
`examples/hello-grid` adds those four, `AssetKind` (`src/lib.rs:210`)
and `Bot<Stage>` (`src/rules.rs:454`).

None of the engine's traits has the attribute: `diagnostic::` appears
nowhere in `crates/`, `templates/` or `examples/`. When an impl is
missing, rustc says E0277, "the trait bound `Data: GameData` is not
satisfied", with no word on what to write or where to read. The only
ticket that plans the attribute is parked 3106, for its 3D camera rigs.

Why it matters:

- **The game first.** The game's sessions add screens, settings and
  data types on the engine's traits all the time. A message that says
  what is missing saves a round.
- **The end goal** (Nick, 2026-10-19, [`docs/decisions.md`](../../docs/decisions.md)):
  an agent makes a small game in under an hour and in few lines. Its
  first compiler errors in a new game are trait bounds, and a message
  that names the fix shortens its loop.
- It makes no public promise, so it comes after the MVP (the second
  comparison's split, [`docs/decisions.md`](../../docs/decisions.md),
  2026-10-20).

**What it can't do.** The attribute changes only E0277, a type that
doesn't implement the trait. It doesn't change:

- a method or associated type missing inside an impl (E0046, which
  rustc already says clearly);
- an associated type of the wrong kind (E0271);
- a missing supertrait: an `Action` without `Hash` is reported against
  `Hash`, not `ActionSet`.

The PR measures which first errors it improves, and says so.

## Nick input

None.

## Scope

**In:**

- **The attribute on each trait a game implements**, with:
  - `message`: what is wrong, naming the type through `{Self}` and the
    trait's generic parameters by name (`{G}`, `{U}`, `{R}`), such as
    "`{Self}` is not a screen of `{G}`";
  - `label`: at the span, what to write, such as "write `impl
    Screen<{G}> for {Self}`";
  - `note`: what the impl must hold, and a link to read (below).

  | Trait | Where | The note says |
  | ----- | ----- | ------------- |
  | `Screen<G>` | `crates/ui/src/screen.rs:49` | it writes `name`, `update` and `draw`; a screen goes on the stack boxed |
  | `GameData` | `crates/ui/src/ctx.rs:82` | its four types (`Action`, `Colors`, `Voice`, `Settings`) and its data methods; the templates' `Data` is one |
  | `ActionSet` | `crates/assets/src/keymap/action_set.rs:64` | an enum of the game's actions, deriving `Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug` |
  | `ColorSet` | `crates/ui/src/frame/color.rs:190` | most games use the engine's `EngineColor` |
  | `ClipVariant` | `crates/assets/src/voice.rs:45` | a game with no voices uses `()`, as `BasicData` does (ADR-0036) |
  | `GameSettings` | `crates/ui/src/settings.rs:251` | the game's own settings and their saved form |
  | `GameTools` | `crates/cli/src/game.rs:470` | only `frame_options`, `frame` and `lang` are required |
  | `TurnRules` | `crates/core/src/turn.rs:27` | the four types and `check`, `apply`, `legal_commands`, `outcome` |
  | `Sides` | `crates/core/src/bots.rs:41` | who moves, and the turn |
  | `Bot<R>` | `crates/core/src/bots.rs:59` | one method, `choose` |
  | `AssetKind` | `crates/assets/src/format.rs:61` | one constant, `NAME` |
  | `Format` | `crates/assets/src/format.rs:79` | its `Asset` kind, `id`, `extensions` and `decode` |
  | `Bakeable` | `crates/assets/src/pack/bake.rs:24` | it comes with `AssetKind`, `Serialize`, `DeserializeOwned`, `PartialEq` and `Debug`: implement or derive the missing ones |
  | `GridSkin<U>` | `crates/ui/src/grid/skin.rs:313` | the engine's `GlyphSkin` and `SpriteSkin` are two |
  | `Terrains` | `crates/ui/src/grid/skin.rs:114` | `name` and `glyphs` per terrain id |
  | `UnitPainter<U>` | `crates/ui/src/grid/skin.rs:283` | `NoUnits` paints none |
  | `MapUnit` | `crates/ui/src/grid/scene.rs:315` | which unit, its tile, its line in the scene |
  | `MapFeature` | `crates/assets/src/grid_map.rs:73` | a tile feature in a map header, read with serde; `NoFeature` for maps with none |

  The traits in `shuyi-core`, `shuyi-assets` and `shuyi-ui` stay
  `no_std` (3005); the attribute needs nothing from `std`.

- **The link in each note.** **From 2802 (done):** the note links the
  book's chapter for the trait, on the website
  (`https://zafnok.github.io/shuyi-engine/learn/book/<page>/`):
  `games-and-screens` for `Screen` and `GameData`, `actions` for
  `ActionSet`, `assets-and-formats` for `AssetKind` and `Format`,
  `rules-as-commands` for `TurnRules`, with the `//~` texts of the
  fixtures to match.
  The attribute takes string literals only, so each note holds its
  link in full.

- **`Bakeable` and `#[diagnostic::do_not_recommend]`.** `Bakeable` has
  one blanket impl (`pack/bake.rs:26`). A type missing one of its
  bounds is reported against that bound ("`Foo: Serialize` is not
  satisfied", required for `Bakeable`), and `Bakeable`'s own message
  shows only when the blanket impl carries `do_not_recommend`. The PR
  shows both outputs and keeps the better:
  - with `do_not_recommend` on the blanket impl: `Bakeable` gets the
    attribute, and its fixture checks its text like the others;
  - without it: `Bakeable` gets no attribute (it would never show), and
    its fixture checks that the output names `Serialize` and
    `Bakeable`.

  `do_not_recommend` goes on no other impl.

- **`cargo xtask compile-messages`** (`crates/xtask/src/compile_messages.rs`,
  a new command in `crates/xtask/src/main.rs` and its `USAGE`):
  - fixtures in `crates/xtask/compile-messages/<name>.rs`, each a small
    program, shaped like the template's code, that misses one impl. Its
    first lines are `//~ <text>`: text the compiler's output for this
    file must contain. The folder is not a cargo target, so the
    workspace's builds never compile it;
  - the xtask writes a package in `target/compile-messages/`: a
    `Cargo.toml` with an empty `[workspace]` of its own, so the
    repository's workspace doesn't claim it, `edition = "2024"`, path
    dependencies on `crates/core`, `crates/assets`, `crates/ui`,
    `crates/platform`, `crates/cli` and `crates/shuyi` (the blank
    template's `use shuyi::prelude::*` and `BasicGame`), and `serde`
    with `derive` for the fixtures that need
    it. It copies the workspace's `Cargo.lock` into it, so the
    dependencies resolve to the versions the workspace pins, and each
    fixture to `src/bin/<name>.rs`;
  - it runs `cargo check --bins --keep-going --color never` there once,
    with `RUSTFLAGS` removed from the child's environment (CI sets `-D
    warnings`, which would turn a fixture's warnings into errors).
    `--keep-going` checks every fixture after the first fails. The
    engine is checked once for all fixtures;
  - it splits cargo's stderr into diagnostics, each starting at a line
    that begins with `error` or `warning`, and gives each to the file of
    its first ` --> ` line (`src/bin/<name>.rs`, with `/` or `\`). A
    diagnostic with no ` --> ` line (cargo's "could not compile") goes
    to no fixture;
  - it fails when a fixture has no `error` diagnostic, or when that
    fixture's diagnostics lack one of its `//~` texts. A text is matched
    as a substring, never the whole output, so a new rustc that rewords
    its own lines doesn't turn the gate red. The human format is used,
    not `--message-format=short`, because the short format drops the
    label and the note;
  - no new dependency (no `trybuild`, no `ui_test`). It runs on Windows
    with the GNU toolchain like the other gates: `cargo check` links
    only build scripts and proc macros, through the repository's
    `.cargo/config.toml`, which the package under `target/` finds;
  - one fixture per trait in the table (18).
- **CI:** a `compile-messages` job in `.github/workflows/ci.yml` on
  `ubuntu-latest`, gated like the other code jobs (`needs: changes`,
  `if: needs.changes.outputs.code == 'true'`), runs `cargo xtask
  compile-messages`, and joins `ci-result`'s `needs`. The `run-gates`
  skill lists it with the other `cargo xtask` checks.
- **The measurement, in the PR:** in a game from `shuyi new grid <dir>
  --engine-path <this checkout>`, delete each of the template's impls
  in turn and record the first error before the attributes and after,
  as a table. Errors the attribute can't change (above) are listed as
  rustc's own.
- **The short path (2511, done):** the fixtures start from its minimal
  template (`shuyi new blank`: `templates/shuyi-default/base`, on `BasicGame`), and the notes name its
  defaults where a default spares an impl: `GameData::theme_def` and
  `voice_variant` have bodies, `()` is a `ClipVariant` and a
  `GameSettings`, `UiAction` is an `ActionSet`, and `BasicData` is a
  whole `GameData` (ADR-0036).
- **For the game:** no API change. The attributes change only what the
  compiler says, and the PR says so.

**Out (do not do):**
- 3106's `CameraRig`: parked; it uses this ticket's fixtures when it is
  unparked (edit requested there);
- the book's chapters the notes link to (2802);
- problem codes and help lines on the contract's errors (2509);
- carrying compiler errors in the contract's answers (`diagnostics`,
  the `build` operation): 3415. The `on_unimplemented` text arrives
  there as any message does;
- better messages for E0046, E0271 and supertrait failures;
- a lint or a macro that checks a game's impls;
- diagnostic attributes that aren't stable on the pinned toolchain.

## Implementation steps

1. Read the traits in the table, the template's impls, and the Rust
   reference's page on diagnostic attributes (which format parameters
   are allowed).
2. Make a game with `shuyi new grid <dir> --engine-path <this
   checkout>`. Delete each of its impls in turn, run `cargo check`, and
   write down the first error. This is the "before" column.
3. Write `compile_messages.rs` and its unit tests, the `main.rs`
   command, and two fixtures (`screen.rs`, `game_data.rs`). Check that
   both fail with rustc's own message.
4. Add the attribute to each trait with its fixture, one at a time,
   reading rustc's output each time.
5. Try `do_not_recommend` on `Bakeable`'s blanket impl, and keep the
   better output as the `Bakeable` bullet says.
6. Repeat step 2 for the "after" column and write the table for the PR.
7. Add the CI job and the `run-gates` line. Run the gates.

## Acceptance criteria

- [x] Each trait in the table has the attribute (`Bakeable` only if the
      PR keeps `do_not_recommend`), and clippy with `-D warnings`
      passes (a malformed attribute warns).
- [x] `cargo xtask compile-messages` passes over 18 fixtures, and fails
      when a fixture's `//~` text is changed or a fixture compiles
      (both shown in the PR).
- [x] The PR has the before-and-after table for the grid template's
      impls.
- [x] `shuyi-core`, `shuyi-assets`, `shuyi-ui` and `shuyi-platform`
      still build for `thumbv7em-none-eabihf`.
- [x] CI runs the `compile-messages` job, and `ci-result` needs it.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`xtask`): reading a fixture's `//~` lines; splitting cargo's
  output into diagnostics by the file of their first ` --> ` line, with
  `/` and `\` paths and a diagnostic with no file; the verdict for a
  fixture with no error, one with only a warning, one with a missing
  text, and one with all its texts.
- Integration: the `compile-messages` job over the 18 fixtures.

## Completion notes

**Done.** The 18 traits of the table carry
`#[diagnostic::on_unimplemented]`: a `message` naming the type
(`{Self}`) and the trait's parameter (`{G}`, `{R}`, `{U}`), a `label`
saying what to write (and the engine's ready-made impl where there is
one: `BasicData`, `UiAction`, `EngineColor`, `()`, `BasicTools`,
`GlyphSkin`/`SpriteSkin`, `NoUnits`, `NoFeature`), a `note` saying what
the impl must hold, and a `note` linking the book's chapter. `cargo
xtask compile-messages` (`crates/xtask/src/compile_messages.rs`, its
unit tests in `compile_messages/tests.rs`) checks one fixture per trait
in `crates/xtask/compile-messages/`; CI's `compile-messages` job runs it
and `ci-result` needs it; `cargo xtask gates` runs it as the
`compile-messages` gate, and the `run-gates` skill has its section. The
docs map has a row for it (`docs/README.md`, engine developers).

**Before and after,** in a game from `shuyi new grid <dir> --engine-path
<this checkout> --no-input --no-examples`, each impl deleted in turn,
the first error of `cargo check --workspace --all-targets --features
<game>/tools`:

| Impl deleted | Before | After |
| ------------ | ------ | ----- |
| `Screen<Data> for Title` | E0277: the trait bound `title::Title: Screen<BasicData<_, Own>>` is not satisfied | E0277: `title::Title` is not a screen of `BasicData<_, Own>`; label: write `impl Screen<…> for title::Title`; notes: `name`, `update`, `draw`, boxed; the chapter |
| `Screen<Data> for StageScreen` | E0277: the trait bound `StageScreen: Screen<BasicData<UiAction, Own>>` is not satisfied | E0277: `StageScreen` is not a screen of `BasicData<UiAction, Own>`, with the same label and notes |
| `GameTools for Tools` | E0277: the trait bound `Tools: GameTools` is not satisfied | E0277: `Tools` is not a game's tools; label: write `impl GameTools for Tools`, or use the engine's `BasicTools`; note: only `frame_options`, `frame` and `lang` are required |
| `Terrains for Terrain` | E0277: the trait bound `Terrain: Terrains` is not satisfied | E0277: `Terrain` is not a game's terrains; label: write `impl Terrains for Terrain`; note: `name` and `glyphs` |
| `MapUnit for Piece` | E0599: the method `to_text` exists for struct `MapScene<Piece>`, but its trait bounds were not satisfied | the same: rustc's own (E0599, a method's bound) |
| `UnitPainter<Piece> for Letters` | E0277: the trait bound `Letters<'_>: UnitPainter<Piece>` is not satisfied | E0277: `Letters<'_>` doesn't paint units `Piece`; label: write `impl UnitPainter<Piece> for Letters<'_>`, or use the engine's `NoUnits` |
| `MapFeature for Feature` | E0277: the trait bound `Feature: MapFeature` is not satisfied | E0277: `Feature` is not a map's tile feature; label: write `impl MapFeature for Feature`, or use the engine's `NoFeature` |
| `TurnRules for Stage` | E0277: the trait bound `Stage: TurnRules` is not satisfied (at `impl Sides for Stage`) | E0277: `Stage` is not a game's rules; label: write `impl TurnRules for Stage`; note: the four types and four methods |
| `Sides for Stage` | E0599: no method named `turn` found for struct `Stage` (in the rules' test) | the same: rustc's own (E0599) |

Seven of the nine first errors now say what to write; the two left are
E0599, a method called whose trait isn't implemented, which the
attribute doesn't reach (the ticket's *What it can't do*). The fixtures
of `MapUnit` and `Sides` reach the attribute through a function's bound
(`layout_gizmos`, `play`), where a game meets it too.

**The gate fails as it should** (shown in the PR): with a fixture's
`//~` text changed (`is not a scene of`), it fails naming the missing
text and printing the fixture's output; with a fixture that compiles
(`fn main() {}`), it fails with "compiled without an error".

**`Bakeable`: kept, without `do_not_recommend`.** On the pinned rustc
(1.98.1), a type missing one of `Bakeable`'s bounds already gets
`Bakeable`'s message through the blanket impl, with rustc's "the trait
`PartialEq` is not implemented for `Table`" and its "consider annotating
`Table` with `#[derive(PartialEq)]`" under it. With `do_not_recommend` on
the blanket impl the message is the same, but rustc says only that
`Bakeable` is not implemented, and drops which bound is missing and the
derive it suggests. So the attribute stays, the blanket impl carries no
`do_not_recommend`, and the fixture checks `Bakeable`'s text and that
the output names `PartialEq`. This is the ticket's first branch for the
attribute and its second for the impl: neither branch foresaw a rustc
that shows the trait's own message without `do_not_recommend`.

**Deviations:**

- The ticket was 37 days old. Its traits all stood, at new lines (the
  table's lines are fixed here). The templates had moved: `templates/grid`
  and `templates/blank` became `templates/shuyi-default` (2502), whose
  grid game writes nine impls, not twelve, since its data is
  `BasicData<UiAction, Own>`. The Context and the short path's bullet
  now say so. `shuyi new grid <dir> --engine-path` still works.
- The ticket named the book's chapter for six traits. The others link
  the chapter that covers them: `ColorSet` and the five grid traits
  `frame-and-skins`, `ClipVariant` `assets-and-formats/#voices`,
  `GameSettings` `saves-and-settings`, `GameTools` `tools-for-agents`,
  `Sides` and `Bot` `rules-as-commands`, `Bakeable`
  `assets-and-formats`.
- The package is checked in its own target folder,
  `target/compile-messages/target`, so a run checks the engine once from
  cold (2 min 27 s on Nick's machine, the next run seconds).
- `cargo xtask compile-messages` is also a gate of `cargo xtask gates`
  (`gates.rs` must name each CI job).

Formats: none, `AssetKind` and `Format` gain only a compiler message; no format or its registry changes.

Skills: none, the compiler now says what a skill would: the message names the impl to write and links the chapter.

The PR is about 1,040 counted lines. It would split cleanly between the xtask (the command, its unit tests, CI) and the attributes with their fixtures, but a fixture checks its attribute's text, so the second half needs the first.

Book: none, no public item is added or changed; the notes link the
book's chapters as they stand.

Behaviour: shuyi-core, shuyi-assets, shuyi-ui, shuyi-cli: a type that
doesn't implement one of the 18 traits a game implements gets the
trait's own message, label and notes from rustc, in place of "the trait
bound … is not satisfied" (still E0277). Searched the game for tests of
compiler output (`gh search code --repo Zafnok/visions-of-shuyi 'trait
bound'`, `'compile_fail'`): none.

No break.
