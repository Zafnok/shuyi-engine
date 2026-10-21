---
id: "2013"
title: "Public types that can grow without breaking a game: an ADR on #[non_exhaustive] and constructors, the exhaustive lints in shuyi-app, and Window, AppConfig, BoxPlace and MapScene moved onto it"
type: infra
milestone: MVP
model: opus-5.5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2013 — Public types that can grow without breaking a game

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Nick put the small protections for the code in the MVP,
before the first release ([`docs/decisions.md`](../../docs/decisions.md),
2026-10-20, *a second comparison*). 2010 makes each API break visible;
this ticket stops most of them from happening.

**What the benchmarks have:**

- **Rust's own rules.** The Cargo book's SemVer chapter
  (<https://doc.rust-lang.org/cargo/reference/semver.html>) counts a new
  public field on a struct whose fields are all public, and a new
  variant on an enum without `#[non_exhaustive]`, as major changes. It
  advises `#[non_exhaustive]` from the start on types that will grow.
  Clippy has two restriction lints for this, `exhaustive_structs` and
  `exhaustive_enums`.
- **Unreal** keeps its public API in `Public` headers and never changes
  a header in a hotfix release
  (<https://dev.epicgames.com/documentation/en-us/unreal-engine/epic-cplusplus-coding-standard-for-unreal-engine>).
  **Godot** keeps the old method as a compatibility method when a
  signature changes.
- **Bevy** does the opposite: public fields, `Default` and
  `..default()`, and a migration guide for every release. So this is an
  edge over Bevy, not parity with it.

**What Shuyi has today:**

- Seven `#[non_exhaustive]` attributes, all in `shuyi-platform`: `Card`
  and `Setup` (`crates/platform/src/app.rs`), and `DisplayInfo`, `Show`,
  `Show::Game`, `Frame` and `SystemEvent`
  (`crates/platform/src/platform.rs`). `DisplayInfo::new` is a `const
  fn` with a `#[must_use] const fn one_bit` setter; `Frame::new` fills
  in the rest of a `Frame`.
  [ADR-0024](../../docs/adr/0024-the-platform-boundary-app-runs-a-frame-time-in-milliseconds-storage-handed-over-and-the-harness-on-the-headless-backend.md)
  explains it for `Frame` and `DisplayInfo`, and
  `docs/platform-backends.md` for `Show`, `Frame` and `DisplayInfo`. No
  ADR sets a rule.
- The workspace lints (root `Cargo.toml`, `[workspace.lints.clippy]`)
  have no `exhaustive_*` lint. `clippy.toml` holds only the two test
  settings.
- Measured with the pinned clippy (1.98.1) on 2026-10-20, over the
  `--lib` targets of the six library crates, `exhaustive_structs` fires
  259 times and `exhaustive_enums` 92 times:

  | Crate | Structs | Enums |
  | ----- | ------- | ----- |
  | `shuyi-core` | 10 | 2 |
  | `shuyi-assets` | 99 | 24 |
  | `shuyi-ui` | 77 | 53 |
  | `shuyi-platform` | 5 | 3 |
  | `shuyi-app` | 4 | 0 |
  | `shuyi-cli` | 64 | 10 |

  `shuyi-app`'s four are `Window`, `Icon`, `VoiceFiles` and `AppConfig`
  (`crates/app/src/lib.rs`): all things a game fills in to start the
  backend.
- Constructors arrive one at a time, after a break. Done 2313 added
  `GlyphSkin::new` (`crates/ui/src/grid/glyph.rs`) when its new field
  broke the game's struct literal. Done 3002 noted that `KeymapDef`'s
  new field breaks literals.
- `Settings` (`crates/ui/src/settings.rs`) has a `pub(crate)` field,
  `layout`, so no game can build it by literal, and a new field there is
  already additive. A private field is the other way to the same end.
- The templates and hello-grid build engine types by literal, and every
  game made with `shuyi new` copies them:
  - `Window`, in a `const` item, and `AppConfig`, in `fn config()`:
    `templates/blank/crates/game/src/main.rs`,
    `templates/grid/crates/game/src/main.rs` and
    `examples/hello-grid/src/main.rs`;
  - `BoxPlace`, in a `const` item: `examples/hello-grid/src/skin.rs`
    (`TALK_BOX`);
  - about forty more across their `src/` and `tests/`: `Look`,
    `audio::Files`, `PaletteDef`, `ConsoleSpec`, `MapColors`,
    `TextView`, the contract's `RunInput`, `CheckInput` and `BotsInput`
    (with `..RunInput::default()` and the like), and others.
- `shuyi-app` copies the workspace lints into its own `Cargo.toml` (its
  `unsafe_code` is `deny`), and the test
  `the_lints_are_the_workspace_lints_except_unsafe_code` in
  `crates/app/src/lib.rs` keeps the copy equal. So a lint for
  `shuyi-app` alone goes in `lib.rs` as a crate attribute, not in its
  `Cargo.toml`.

**What is planned, and breaks by literal:**

- 2304 (MVP): `MapScene` gains a field. Its *For the game* says it
  breaks where a game builds `MapScene` with a struct literal.
- 2305 (MVP): `BoxPlace` gains `narration`.
- 2321 (MVP): `shuyi_app::Window` gains `vsync`.
- 2010 (MVP) shows the public API change on every PR and asks for a
  `Breaking:` line. Its either-order clause with 2013 is written.
- 2005 (MVP) writes the version policy as an ADR and publishes 0.1. It
  waits for this ticket (an edit request there).
- 2511 (MVP) adds `BasicGame`, with private fields, two constructors and
  the builder methods `console` and `screens`, and `run_game`, which
  builds a `Window` and an `AppConfig` inside `shuyi-app`.
- 3044 (After MVP) adds `Window::console(..)`, and
  `Window::with_icon(self, icon: Option<Icon>)` if this ticket hasn't;
  its clause with 2013 says both follow this ticket's rule once it is
  done. 3048 (After MVP) waits for this ticket and builds the
  templates' window as `Window::console(..).with_icon(APP.window_icon)`,
  since a struct literal can't compile outside `shuyi-app` once
  `Window` is non-exhaustive. 3046 (After MVP) adds `AppConfig::crash`.
- Many MVP and After MVP tickets say "if 2013 is done, the new public
  types follow its rules; if not, 2013 covers them" (2509, 3040, 2018,
  2109, 2114, 2228, 2229, 2332, 2335, 2337, 3044, 3048 and others).

**Why it matters:**

- **The game first.** It pins the engine by revision. Each ticket that
  adds a field to a type the game builds by literal is one more edit at
  its next pin bump, and three MVP tickets do. After this ticket their
  new fields are additive: the game follows one break, this one.
- **Other games.** Every game made with `shuyi new` copies the
  templates' literals and inherits every later break.
- **The end goal (2026-10-19).** A game an agent wrote today should
  still compile after an engine upgrade that only adds. Until Visions of
  Shuyi launches, that game is a turn-based one, such as the tutorial's
  box-pushing puzzle (2803); snake waits with the real-time work
  (2026-10-20). Before 2005 publishes 0.1 is the cheapest moment to set
  the rule.

## Nick input

None.

## Scope

**In:**

- **An ADR** (the next free number; `write-adr` skill), "Public types
  that can grow". It sorts public types into kinds, gives each a rule
  and an example, and says why each rule costs a small game (the
  tutorial's box-pushing puzzle, 2803) no extra lines:
  1. **Plain values** (`Pos`, `Rect`, `Rgb`, `Millis`, `TerrainId`):
     the fields are the whole meaning, and another field would make
     another type. They stay exhaustive, with public fields.
  2. **Config a game fills in** (`Window`, `Icon`, `VoiceFiles`,
     `AppConfig`, `BoxPlace`): `#[non_exhaustive]`. Fields stay public
     (read and assigned). A `new` takes the fields every game must
     choose, and is a `const fn` whenever it can be. An optional field
     starts at its default in `new`. A game changes it by assigning the
     field or, where games build the type in a `const` item, with a
     `#[must_use] pub const fn with_<field>(self, value: <the field's
     own type>) -> Self`, so generated code can pass an `Option` it
     holds. Why: `..Default::default()` isn't `const`, and
     `#[non_exhaustive]` forbids `..` outside the crate anyway. No
     builder types.
  3. **Views and scenes a screen fills in and a skin reads**
     (`MapScene`): `#[non_exhaustive]`, with a `new` for what the type
     can't be without; the rest starts empty and is assigned. Reading
     and assigning fields still work outside the crate; only literals
     and patterns without `..` stop compiling. No setters.
  4. **Inputs with a `Default`, never built in `const`** (the contract's
     `RunInput`, `CheckInput`, `BotsInput`): `#[non_exhaustive]`, and a
     caller writes `let mut input = RunInput::default();` then
     `input.frames = 3;`. With exhaustive types and
     `..RunInput::default()` instead, every new optional field would
     need a `Breaking:` line under 2010's job. Check that clippy's
     `field_reassign_with_default` stays quiet on a non-exhaustive type
     from another crate. If it doesn't, the ADR says what callers write
     instead.
  5. **Data read from files** (`PaletteDef`, `KeymapDef`,
     `AudioManifest`): a new field is `#[serde(default)]`, so old files
     still load (ADR-0007 and ADR-0019 don't change). In Rust, such a
     type is kind 2 or kind 4, by how games build it.
  6. **Enums:** a set that will grow (errors, events, kinds of thing, a
     setting's choices) is `#[non_exhaustive]`. A closed set (`Dir`'s
     four directions) stays exhaustive. A struct-like variant that may
     gain fields is `#[non_exhaustive]` itself, as `Show::Game` is.
  7. **Traits:** a new method on a trait a game implements gets a
     default body, as `UnitPainter::paint_under` has
     (`crates/ui/src/grid/skin.rs`). A trait games call but must not
     implement is sealed, decided per trait; the ADR names any it finds
     today. A trait backends implement (`Platform`) may take a required
     method where a default would be wrong: 3053's `Platform::facts`
     (After MVP) is one, and its ADR says why.
  8. **A private field** (`Settings::layout`) is the other way, for a
     type whose fields keep an invariant or that a game never reads
     field by field. Its optional parts are set by `#[must_use]` methods
     that take and return `self`, named for what they set (2511's
     `BasicGame::console`, if 2511 is done).

  The ADR also says:
  - **When:** a new public type follows it in the PR that adds it. An
    existing type moves under it in the PR that next adds to it, or in
    the follow-up below, once, with a `Breaking:` line.
  - **How it is held:** in `shuyi-app`, by the two lints at crate level
    (below). Elsewhere the lints fire about 350 times (the counts
    above), most of them on types that rightly stay as they are, so the
    rule is held by the `write-ticket` and
    `work-ticket` lines and by 2010's job, whose cargo-semver-checks
    names the very break this rule prevents (a field added to a struct
    a game can build, a variant added to an exhaustive enum). The ADR
    records the counts and why. A new crate whose public types are all
    config turns the lints on the same way.
  - **A table** of every engine type the templates and hello-grid build
    by literal (in `src/` and `tests/`): its crate, its kind (1 to 8),
    and whether it changes in this PR or in the follow-up.
  - **Its neighbours:** 2010's ADR is the gate. 2005's version policy
    cites this ADR and doesn't restate it.
  - Its row goes in `docs/adr/README.md`.
- **`shuyi-app`** (`crates/app/src/lib.rs`):
  - `#[non_exhaustive]` on `Window`, `Icon`, `VoiceFiles` and
    `AppConfig`;
  - `pub const fn Window::new(title: &'static str, width: i32, height:
    i32) -> Window` with `icon: None`, and `#[must_use] pub const fn
    with_icon(self, icon: Option<Icon>) -> Window`;
  - `pub const fn Icon::new(small: &'static [u8], medium: &'static
    [u8], big: &'static [u8]) -> Icon` and `pub const fn
    VoiceFiles::new(folder: &'static str, lang: &'static str, manifest:
    &'static str) -> VoiceFiles`;
  - `AppConfig::new(window: Window, storage_name: &'static str, files:
    fn(&str) -> Option<&'static [u8]>, font_png: &'static str, sounds:
    Files<'static>) -> AppConfig`, with `voices` and `text_input` at
    `None` (a game assigns them); a `const fn` if it compiles as one;
  - `#![warn(clippy::exhaustive_structs, clippy::exhaustive_enums)]` at
    the top of `lib.rs`, with a comment naming the ADR and why the lints
    live here and not in `Cargo.toml`. It passes with no `#[allow]` or
    `#[expect]`.
- **`shuyi-ui`:**
  - `BoxPlace` (`crates/ui/src/dialogue/text_box/glyph.rs`):
    `#[non_exhaustive]` and `pub const fn new(frame: Rect, text_x: i32,
    text_y: i32) -> BoxPlace`;
  - `MapScene` (`crates/ui/src/grid/scene.rs`): `#[non_exhaustive]`.
    `MapScene::new(origin, size)` exists already.
- **Every literal of those six outside their own crate moves** to the
  constructors: the three `main.rs` files and
  `examples/hello-grid/src/skin.rs` above, and whatever else the build
  finds. The `voices: None` and `text_input: None` lines go, so each
  `main.rs` gets shorter. Literals inside the defining crate (the toy's
  `TALK_BOX` in `crates/ui/src/toy.rs`, `crates/ui/src/grid/testing.rs`,
  `shuyi-app`'s tests) may stay.
- **The skills:**
  - `write-ticket`, step 4, a new bullet: "A new public type follows
    ADR-NNNN (2013): one that may grow is `#[non_exhaustive]` with a
    constructor, and a new field on such a type is additive. Say which
    in the Scope."
  - `work-ticket`, §3 *Public API*, a new line: "A public type you add,
    or add to, follows ADR-NNNN (2013)."
- **Types other tickets left to 2013:** read `tickets/done/` for clauses
  that leave their new types to this ticket ("2013 covers them",
  "2013's sweep covers"). Those types follow the ADR in this PR. If that
  would take the PR past about 600 changed lines, they go in the
  follow-up instead.
- **The follow-up:** the table's types that should change and don't in
  this PR go in one new ticket (`write-ticket` skill, 20xx, After MVP),
  or the completion notes say there are none.

**Out (do not do):**

- `exhaustive_structs` or `exhaustive_enums` in any crate but
  `shuyi-app`, and `#[expect]` or `#[allow]` on plain values to quiet
  them.
- Changing the table's other types (the follow-up), or the rest of the
  ~350 public types.
- Builder types, setters on views and inputs, and sealing every trait.
- Signature changes and removals, such as 2301's change to `text_width`
  or parked 2215's removal of `Layer::dir`. No type shape prevents
  those; 2010's `Breaking:` lines record them.
- 2315's `GizmoRole` and parked 2322's `Role`, which already plan
  `#[non_exhaustive]`.
- `Window::console` and the window's saved size (3044), `AppInfo` and
  the game's identity (3048), `AppConfig::crash` (3046), and
  `BasicGame` and `run_game` (2511).
- The public API gate (2010), the version policy and `CHANGELOG.md`
  (2005), and the changelog's reader (2804).
- Renaming anything.

**Either order:**

- **2010.** If it is done, the ADR cites 2010's ADR, and its `api` job
  reports this PR's breaks (each type made non-exhaustive); the
  `Breaking:` lines below answer it. If not, write the lines in 2010's
  form anyway, and 2010's ADR cites this one.
- **2304.** If it is done, `MapScene` has its new field already, and
  `MapEdges` follows the ADR (four sides: a plain value). If not, 2304
  adds the field later, and it is additive.
- **2305.** If it is done, `BoxPlace::new` sets `narration` to
  `NarrationAlign::Centred`, add `#[must_use] pub const fn
  with_narration(self, narration: NarrationAlign) -> BoxPlace`, and
  make `NarrationAlign` `#[non_exhaustive]` (a choice that may grow). If
  not, 2305 adds them.
- **2321.** If it is done, `Window::new` sets `vsync` to 2321's
  default, add `#[must_use] pub const fn with_vsync(self, vsync: bool)
  -> Window`, and 2321's new public types follow the ADR. If not, 2321
  adds the setter.
- **2511** (engine defaults, a prelude, `run_game` and a minimal
  template). If it is done, the ADR gives `BasicGame` as kind 8's
  example (private fields, its constructors and its builder methods,
  kept as they are), its other new public types follow the ADR, and the
  minimal template's literals move with the rest. If not, 2511 builds on
  the constructors, and `run_game` uses `Window::new` and
  `AppConfig::new`.
- **3044, 3048 and 3046** (After MVP, so most likely after this one).
  If 3044 is done, `Window::with_icon` exists: keep it as 3044 wrote it,
  and don't add a second one. 3048 waits for this ticket and writes
  `Window::console(..).with_icon(APP.window_icon)`. If 3046 is done,
  `AppConfig::new` sets `crash` to 3046's default. If none is done, they
  build on `Window::new`, `with_icon(Option<Icon>)` and
  `AppConfig::new`, and 3044 adds only `Window::console`.
- **1134, 2802 and 2803** (the skill pack, the book, the tutorial). If
  done, their code that builds the six types moves to the constructors
  (CI compiles the book's code). If not, they are written with them.
- **2014.** If it is done, its report on this PR lists the game's call
  sites that change; link it from the PR.

**For the game:** a break, once. Outside their crates, `Window`, `Icon`,
`VoiceFiles`, `AppConfig`, `BoxPlace` and `MapScene` can no longer be
built by a struct literal or matched without `..`. The game moves to
`Window::new(..)` with `with_icon(..)`, `AppConfig::new(..)`,
`BoxPlace::new(..)`, and `MapScene::new(..)` with field assignments.
After this, the new fields of 2304, 2305 and 2321 are additive. The
completion notes carry one `Breaking:` line per crate in 2010's form,
for example `Breaking: shuyi-app: Window, Icon, VoiceFiles, AppConfig:
build them with their new constructors`. The game's PR that moves the
pin follows this one.

## Implementation steps

1. Read ADR-0024, `docs/platform-backends.md`, 2010 and 2005. Check
   which of 2010, 2304, 2305, 2321, 2511, 1134, 2802, 2803, 2014, 3044,
   3046 and 3048 are done. Search `tickets/done/` for "2013".
2. List the engine types the templates and hello-grid build by literal:
   search `examples/hello-grid/` and `templates/` for `Name {`, where
   `Name` is a public struct or enum variant of `crates/`, and drop the
   names the game code defines itself. Sort them into the kinds.
3. For kind 4, check `field_reassign_with_default`: mark
   `shuyi_cli::contract::run::RunInput` `#[non_exhaustive]` for the
   moment, write `let mut input = RunInput::default(); input.frames =
   3;` in a scratch test in `crates/cli/tests/it/`, run `cargo clippy -p
   shuyi-cli --all-targets -- -D warnings`, note what it says, then undo
   both. Write the ADR and its row.
4. Change the six types. Add the constructors and setters, with doc
   comments that say which fields `new` sets and to what.
5. Add the crate attribute to `shuyi-app`, and run `cargo clippy -p
   shuyi-app --all-targets -- -D warnings`.
6. Move the literals in the templates and hello-grid. Run `cargo fmt
   --all` in the engine. Then run the template gate as CI's `templates`
   job does: `cargo run -p shuyi-cli -- new grid <dir> --engine-path
   <repo>`, then in `<dir>` `cargo fmt --all --check`, `cargo clippy
   --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
   `cargo shuyi check`, `cargo shuyi check --target all` and `cargo
   shuyi check-keys`; the same for `blank`. `cargo fmt --all` doesn't
   reach `templates/`, so keep their import order by hand.
7. Add the skills' lines, and write the follow-up ticket.
8. Fill in the completion notes with the `Breaking:` lines, and run the
   gates.

## Acceptance criteria

- [ ] The ADR exists, with its row in `docs/adr/README.md`. It gives
      each kind a rule and an example, records the lint counts and the
      choice to hold the lints in `shuyi-app` only, and has the table.
- [ ] `Window`, `Icon`, `VoiceFiles`, `AppConfig`, `BoxPlace` and
      `MapScene` are `#[non_exhaustive]`. Each has a `new`, and
      `Window::new`, `Window::with_icon`, `Icon::new`, `VoiceFiles::new`
      and `BoxPlace::new` are `const fn`s.
- [ ] A `compile_fail` doctest on `Window` and one on `BoxPlace` show
      that a struct literal giving every field doesn't compile outside
      the crate.
- [ ] `shuyi-app`'s `lib.rs` turns on the two lints, clippy passes with
      `-D warnings`, and no `#[allow]` or `#[expect]` quiets them.
      Removing `#[non_exhaustive]` from `Window` makes clippy fail
      (tried by hand, said in the PR).
- [ ] No file under `templates/` or `examples/` builds the six types by
      a struct literal, and games made with `shuyi new grid` and `shuyi
      new blank` pass the template gate.
- [ ] The `write-ticket` and `work-ticket` skills have their lines.
- [ ] The completion notes carry the `Breaking:` lines, and name the
      follow-up ticket or say there is none.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/app/src/lib.rs`): `Window::new` has no icon and
  `with_icon` sets one; a `const` item built with
  `Window::new(..).with_icon(..)` compiles; `AppConfig::new` leaves
  `voices` and `text_input` at `None`; `window_conf` of
  `Window::new("A game", 640, 480).with_icon(broken)` gives the same
  `Conf` as `an_icon_that_does_not_decode_is_none` does today with its
  literal.
- Unit (`shuyi-ui`): `BoxPlace::new` equals today's `TALK_BOX` literal,
  and the text box's snapshots don't change.
- Doctests: the two `compile_fail` cases.
- Integration: the template gate for `grid` and `blank`; hello-grid's
  tests and snapshots unchanged.

## Completion notes

*(Filled in by the session that completes the ticket.)*
