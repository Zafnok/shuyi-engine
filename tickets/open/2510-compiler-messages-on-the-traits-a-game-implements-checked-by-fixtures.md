---
id: "2510"
title: "Compiler messages on the traits a game implements: on_unimplemented saying what to write and where to read, pinned by fixtures that check the message"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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
grid template (`templates/grid/crates/`) writes `ActionSet`
(`game/src/action.rs:26`), `GameData` (`game/src/lib.rs:141`),
`ClipVariant` (`:212`), `GameSettings` (`game/src/settings.rs:39`),
`Screen<Data>` (`game/src/stage.rs:228`), `Terrains` (`:43`), `MapUnit`
(`:93`), `UnitPainter<Piece>` (`:118`), `GameTools`
(`game/src/tools.rs:166`), `TurnRules` (`rules/src/lib.rs:223`),
`Sides` (`:278`) and `MapFeature` (`:41`). `examples/hello-grid` adds
`AssetKind` (`src/lib.rs:73`) and `Bot<Stage>` (`src/rules.rs:382`).

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
  | `Screen<G>` | `crates/ui/src/screen.rs:35` | it writes `name`, `update` and `draw`; a screen goes on the stack boxed |
  | `GameData` | `crates/ui/src/ctx.rs:65` | its four types (`Action`, `Colors`, `Voice`, `Settings`) and its data methods; the templates' `Data` is one |
  | `ActionSet` | `crates/assets/src/keymap/action_set.rs:36` | an enum of the game's actions, deriving `Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug` |
  | `ColorSet` | `crates/ui/src/frame/color.rs:158` | most games use the engine's `EngineColor` |
  | `ClipVariant` | `crates/assets/src/voice.rs:25` | a game with no voices uses one variant, as the templates' `OnlyClip` |
  | `GameSettings` | `crates/ui/src/settings.rs:70` | the game's own settings and their saved form |
  | `GameTools` | `crates/cli/src/game.rs:78` | only `frame_options`, `frame` and `lang` are required |
  | `TurnRules` | `crates/core/src/turn.rs:27` | the four types and `check`, `apply`, `legal_commands`, `outcome` |
  | `Sides` | `crates/core/src/bots.rs:17` | who moves, and the turn |
  | `Bot<R>` | `crates/core/src/bots.rs:35` | one method, `choose` |
  | `AssetKind` | `crates/assets/src/format.rs:41` | one constant, `NAME` |
  | `Format` | `crates/assets/src/format.rs:59` | its `Asset` kind, `id`, `extensions` and `decode` |
  | `Bakeable` | `crates/assets/src/pack/bake.rs:21` | it comes with `AssetKind`, `Serialize`, `DeserializeOwned`, `PartialEq` and `Debug`: implement or derive the missing ones |
  | `GridSkin<U>` | `crates/ui/src/grid/skin.rs:243` | the engine's `GlyphSkin` and `SpriteSkin` are two |
  | `Terrains` | `crates/ui/src/grid/skin.rs:103` | `name` and `glyphs` per terrain id |
  | `UnitPainter<U>` | `crates/ui/src/grid/skin.rs:219` | `NoUnits` paints none |
  | `MapUnit` | `crates/ui/src/grid/scene.rs:168` | which unit, its tile, its line in the scene |
  | `MapFeature` | `crates/assets/src/grid_map.rs:61` | a tile feature in a map header, read with serde; `NoFeature` for maps with none |

  The traits in `shuyi-core`, `shuyi-assets` and `shuyi-ui` stay
  `no_std` (3005); the attribute needs nothing from `std`.

- **The link in each note.** **Either order with 2802** (the book): if
  it is done, the note links the book's chapter for the trait (a game
  and its screens, actions, assets and formats, rules as commands). If
  not, the trait's rustdoc isn't online yet (2802's `/api/`, or docs.rs
  after 2005), so the note links the file that defines the trait on
  GitHub (`https://github.com/Zafnok/shuyi-engine/blob/main/crates/ui/src/screen.rs`),
  and 2802 changes the links to its chapters (edit requested there).
  The attribute takes string literals only, so each note holds its
  link in full.

- **`Bakeable` and `#[diagnostic::do_not_recommend]`.** `Bakeable` has
  one blanket impl (`pack/bake.rs:23`). A type missing one of its
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
    dependencies on `crates/core`, `crates/assets`, `crates/ui` and
    `crates/cli`, and `serde` with `derive` for the fixtures that need
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
  template (`templates/blank`, on `BasicGame`), and the notes name its
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

- [ ] Each trait in the table has the attribute (`Bakeable` only if the
      PR keeps `do_not_recommend`), and clippy with `-D warnings`
      passes (a malformed attribute warns).
- [ ] `cargo xtask compile-messages` passes over 18 fixtures, and fails
      when a fixture's `//~` text is changed or a fixture compiles
      (both shown in the PR).
- [ ] The PR has the before-and-after table for the grid template's
      impls.
- [ ] `shuyi-core`, `shuyi-assets`, `shuyi-ui` and `shuyi-platform`
      still build for `thumbv7em-none-eabihf`.
- [ ] CI runs the `compile-messages` job, and `ci-result` needs it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`xtask`): reading a fixture's `//~` lines; splitting cargo's
  output into diagnostics by the file of their first ` --> ` line, with
  `/` and `\` paths and a diagnostic with no file; the verdict for a
  fixture with no error, one with only a warning, one with a missing
  text, and one with all its texts.
- Integration: the `compile-messages` job over the 18 fixtures.

## Completion notes

*(Filled in by the session that completes the ticket.)*
