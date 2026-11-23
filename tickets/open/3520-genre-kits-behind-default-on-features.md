---
id: "3520"
title: The grid kit, dialogue, cinematics and bots sit behind default-on features
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3520 — The grid kit, dialogue, cinematics and bots sit behind default-on features

## Context

[ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
§1 says a part is a module "when a game that doesn't use it would
still pay for it", and "when in doubt, it is a module". But the same
section kept what the accepted ADRs had already placed in the default
crates ("This ADR doesn't move anything"). 2003 (done) built the
modules and kept "moving anything out of the default crates" out of
scope. So the first game's genre kits are compiled into every game,
and no open ticket plans to change that.

What a game always gets today (counted on this branch):

- **The grid kit**, `shuyi_ui::grid` (`crates/ui/src/grid.rs` and
  `crates/ui/src/grid/`): 9,972 lines.
- **Dialogue**, `shuyi_ui::dialogue`: 4,932 lines.
- **Cinematics**, `shuyi_ui::cinematic`: 853 lines.
- **Bots and threat areas** in the pure core: `shuyi_core::bots` and
  `shuyi_core::threat` (`crates/core/src/lib.rs:47` and `:58`), 8,230
  lines with their sub-modules, and their re-exports at the crate root
  (`lib.rs:64-85`).

None is behind a feature. `crates/ui/Cargo.toml` has `std`,
`debug-tools`, `storage-conformance`, `onebit-reference`, `raster`,
`toy` and `schema`; `crates/core/Cargo.toml` has `std`, `schema` and
`toy`. A real-time puzzle game pays for all of it in compile time, in
the API it reads (docs, agents' context) and in tactics words met
first. After the first release (2005) moving any of them is a break;
a default-on feature costs little now and nothing for a game that
keeps the defaults.

How the kits are reached from outside their modules (grep of
`crate::grid`, `crate::dialogue`, `crate::cinematic`, `bots::`,
`threat::` across `crates/`):

- `shuyi_ui::audio` (`audio.rs:51`) and `shuyi_ui::game` (`game.rs:25`)
  use `cinematic::millis`; `ease.rs:39` (a test) uses
  `cinematic::ease_in_out`.
- `shuyi_ui::debug::camera` (`debug/camera.rs:18`, `:23`) uses
  `cinematic::PanFrame` and `grid::picture::pixel_size`
  (`grid/picture.rs:92`), a size of a `GlyphBuffer` that is not
  grid-specific.
- `shuyi_ui::toy` (`toy.rs:38-42`, `toy/grid.rs`) and
  `text/hint_tests.rs` use dialogue and the grid kit; the kits' own
  tests use `crate::toy::grid`.
- `shuyi-platform`'s `BasicGame` (`crates/platform/src/basic.rs:21`,
  `:301-403`) has the dialogue preview, and `shuyi-app`'s `run_game`
  takes a `PreviewGame` (`crates/app/src/lib.rs:223`, `:910`).
- `shuyi-cli`'s playtest and autobalance operations use
  `shuyi_core::bots` (`crates/cli/src/game.rs:28`,
  `crates/cli/src/contract/autobalance.rs`).
- Only the bots use `threat` (`bots/fixtures.rs`), and `threat`'s
  `TacticsOrder` and `OrderHooks` are a planning bot's command order
  (`threat.rs:9-12`), so the two go behind one feature.
- `shuyi_core::character` (`CharacterId`, 12 lines) is used by the
  dialogue format in `shuyi-assets`; it stays.

The game turns the default features off. Its root `Cargo.toml` (pin
`50472bbcd3084555a61728a5d07518f720bd6514`) declares `shuyi-core`,
`shuyi-assets`, `shuyi-ui` and `shuyi-platform` with
`default-features = false` (lines 29-32). `gh search code --repo
Zafnok/visions-of-shuyi 'shuyi_ui::grid'` finds
`crates/ui/src/map_view.rs` and its sub-modules; `'shuyi_ui::dialogue'`
finds `crates/ui/src/dialogue.rs` and `crates/ui/src/screens/dialogue/`;
`'shuyi_core::bots'` finds `crates/bots/src/lib.rs` and
`crates/bots/src/playtest.rs`, whose manifest names
`shuyi-core = { workspace = true, features = ["std"] }`. So for the
game this is a break: each of those crates names the new features, and
the game's PR that moves the pin follows this one.

The API gate ([ADR-0038](../../docs/adr/0038-the-public-api-gate-a-listing-diff-and-semver-checks-on-every-pr-and-a-breaking-line-for-each-break.md))
lists each crate with the features in `API_CRATES`
(`crates/xtask/src/api.rs:110`). A new feature missing from that list
would make every gated item look removed.

The milestone is After MVP, as the audit's tickets are. Nick decides in
3528 which of them join the MVP; if this one does, it becomes a
blocker of 2005 there.

## Nick input

None.

## Scope

**In:**
- Cargo features, each on by default in its crate:
  - `shuyi-core`: `bots`, over `bots` and `threat` and their root
    re-exports.
  - `shuyi-ui`: `grid`, `dialogue` and `cinematic`, over those modules
    and their re-exports. `toy` turns on `grid` and `dialogue`.
  - `shuyi-platform`: `dialogue`, turning on `shuyi-ui/dialogue`, over
    `BasicGame`'s dialogue preview.
  - `shuyi`: `grid`, `dialogue`, `cinematic` and `bots`, forwarding to
    the crates, in its `default` beside `std` and `app`.
- `shuyi-app` and `shuyi-cli` turn on what they use (`shuyi-app`: the
  dialogue preview; `shuyi-cli`: `bots`), so their API is unchanged.
- Each kit builds with `no_std` as today (the Playdate game uses the
  grid kit and dialogue), and the default engine builds with any one
  kit off.
- The three general helpers move out of the kits, keeping their old
  paths as re-exports under the kit's feature: `cinematic::millis` and
  `cinematic::ease_in_out` to `shuyi_ui::ease`, and
  `grid::picture::pixel_size` to `shuyi_ui::frame::glyph_buffer`.
- A new ADR amending ADR-0009 §1: a part of a default crate that some
  kinds of game don't use sits behind a default-on feature of that
  crate, forwarded by `shuyi`; a new such part gets its feature when it
  arrives; making it a module of its own stays a later choice, with
  its own ADR.
- No public type is added and none changes shape (ADR-0039 has nothing
  to settle). With the default features nothing a game names moves.
- API: a game that keeps `shuyi`'s defaults sees no change. A crate
  that names `shuyi-core`, `shuyi-ui`, `shuyi-platform` or `shuyi` with
  `default-features = false` loses the kits until it names their
  features: one `Breaking:` line and `migration` block per crate, as
  `tickets/README.md` says. The game's PR that moves the pin follows
  this one.

**Either order with 3413:** if 3413 is done, put its domain preludes
behind the features too (`shuyi::prelude::grid` behind `grid`, and
every other prelude item from a kit behind that kit's feature); if
not, nothing to do (3413 gates its preludes by the features this
ticket adds).

**Either order with 2842:** if 2842 is done, add the new features to
each crate's `[package.metadata.docs.rs]` list with its `API_CRATES`
entry, so its test still holds them equal; if not, nothing to do (2842
reads `API_CRATES`, which this ticket fills).

**Out (do not do):**
- Moving a kit into a crate of its own. The grid kit and dialogue use
  `shuyi-ui`'s frame, `Ctx` and screens, so such a crate sits on top
  of `shuyi-ui` and every path changes. That is the later choice the
  new ADR names, not this ticket.
- Feature-gating `shuyi-assets`' dialogue and cinematic file formats
  (`crates/assets/src/dialogue/`, about 3,800 lines; `cinematic.rs`,
  389). `shuyi-cli`'s `check` and `import` and `shuyi-platform`'s voice
  lines (`LineId`) read them whatever kit a game uses, and a format's
  feature is ADR-0007's plugin question.
- Regrouping `Ctx`'s 34 fields (`crates/ui/src/ctx.rs`). None of them
  belongs to the four kits: they are audio, device, session and debug
  facts.
- `shuyi_core::grid`, `path` and `character`: general, and used outside
  the kits.
- Shrinking a crate's line count. A build without the kits compiles
  less; the source is the same size (3517 measures sizes).
- Changing any template or `shuyi new`: they use `shuyi` with its
  defaults.

**Estimated size:** about 900 lines (code 350, tests 250, docs 300).

## Implementation steps

1. Read ADR-0009, ADR-0038 and ADR-0039;
   `crates/xtask/src/api.rs` (`API_CRATES`, `Features`),
   `crates/xtask/src/modules.rs` (`ENGINE_FEATURES`, line 68) and the
   `no-std` and `modules` gates in `crates/xtask/src/gates.rs`
   (lines 361-374).
2. **Move the helpers.** `millis` and `ease_in_out` from
   `crates/ui/src/cinematic.rs` (lines 155, 166) to
   `crates/ui/src/ease.rs`; `pixel_size` from
   `crates/ui/src/grid/picture.rs:92` to
   `crates/ui/src/frame/glyph_buffer.rs`. Leave `pub use` at the old
   paths. Point `audio.rs`, `game.rs`, `ease.rs`' test and
   `debug/camera.rs` at the new paths.
3. **`shuyi-core`.** In `crates/core/Cargo.toml`, `bots = []` with a
   comment in the style of the others, and `default = ["std",
   "bots"]`. In `lib.rs`, `#[cfg(feature = "bots")]` on `pub mod bots`,
   `pub mod threat` and their `pub use` blocks. The dev-dependency on
   itself (line 32) names `bots`.
4. **`shuyi-ui`.** `grid`, `dialogue` and `cinematic` in
   `crates/ui/Cargo.toml`, in `default`; `toy` adds `grid` and
   `dialogue`. Gate in `lib.rs` the three modules and
   `debug::camera` (with `cinematic`: its `SceneCameraView` holds a
   `PanFrame`) and `debug.rs`' `pub use camera::…`. Gate
   `text/hint_tests.rs` with `dialogue`. Write intra-doc links from
   one kit to another (`grid/picture.rs:2`, `cinematic.rs:247`,
   `screen.rs:60`, `debug.rs:17`) so `cargo doc` with that kit off
   warns of none: plain text, or a `cfg_attr` doc.
5. **`shuyi-platform`.** `dialogue = ["shuyi-ui/dialogue"]` in
   `default`. Gate the dialogue preview in `crates/platform/src/basic.rs`
   (the `DialogueFolder`/`DialoguePreview` imports, `preview_start`,
   `preview_game` and the drop hook at lines 301-403) and its tests.
6. **Backends and tools.** `shuyi-app` names
   `shuyi-platform/dialogue` and `shuyi-ui/dialogue`; `shuyi-cli`
   names `shuyi-core/bots` (a grep of `crates/cli/src` finds no path
   into the UI kits; it keeps `shuyi-ui`'s defaults anyway). The
   Playdate backend (`crates/playdate`) builds as it does.
7. **`shuyi`.** In `crates/shuyi/Cargo.toml`, the four features
   forwarding to the crates, and `default = ["std", "app", "grid",
   "dialogue", "cinematic", "bots"]`. Add them to `ENGINE_FEATURES` in
   `crates/xtask/src/modules.rs` with the comment that they are the
   default engine's kits, not modules.
8. **`examples/hello-grid`.** Its `shuyi` dependency
   (`Cargo.toml:81`, `default-features = false`) names `grid` and
   `dialogue`, which it uses (`src/map.rs`, `src/preview.rs`).
9. **The API gate.** Add the features to `API_CRATES`
   (`crates/xtask/src/api.rs:114-150`): `bots` to `shuyi-core`;
   `grid`, `dialogue`, `cinematic` to `shuyi-ui`; `dialogue` to
   `shuyi-platform`; the four to `shuyi`. Update the listing strings in
   `crates/xtask/src/api/tests.rs` (`UI_LISTING` and the core one,
   lines 1696-1761).
10. **The gates.** In `gates.rs`:
    - the `no-std` gate's thumbv7em builds of `shuyi-core` and
      `shuyi-ui` now build them without the kits; add a line that
      builds them with `--features bots` and `--features
      grid,dialogue,cinematic`, so the kits stay `no_std`;
    - its host test line runs the kits' tests too (`--features` as
      above);
    - the `modules` gate gets `cargo check` lines: `shuyi-ui` with
      `std` and each of its kits off in turn, `shuyi-core` with
      `std` and no `bots`, `shuyi-platform` with `std` and no
      `dialogue`, and `shuyi --no-default-features --features std`.
    Update the `run-gates` skill's table and CI's jobs for those gates
    to match (the gates test at `gates.rs:1266` lists them).
11. **The ADR** (`write-adr` skill), next free number (`cargo xtask
    next-id adr`), amending ADR-0009 §1; add "§1 amended by ADR-NNNN"
    to ADR-0009's status line and a row to `docs/adr/README.md`.
12. **Docs** (`document` skill): a paragraph in `docs/modules.md` on
    the default engine's kits and their features, outside the
    catalog's tables (`check-modules` reads the rows); the *Modules*
    section of `site/content/learn/book/games-and-screens.md`; each
    feature's comment in its `Cargo.toml`.
13. **Completion notes:** the `Breaking:` lines and `migration` blocks
    (step "API" of the Scope), and `cargo build --timings` of
    `shuyi-ui` with and without the kits, as a measured line.

## Acceptance criteria

- [ ] `cargo check -p shuyi-ui --no-default-features --features std`
      builds with none of `grid`, `dialogue` and `cinematic`, and with
      each of them alone off.
- [ ] `cargo check -p shuyi-core --no-default-features --features std`
      builds with no `bots` module and no `threat` module.
- [ ] `cargo check -p shuyi-platform --no-default-features --features
      std` builds without the dialogue preview, and `cargo check -p
      shuyi --no-default-features --features std` builds.
- [ ] The thumbv7em `no-std` builds pass both without the kits and with
      all of them.
- [ ] With default features the public listing of each engine crate is
      the same as before: the PR's `api` job reports no item added or
      removed (the moved helpers keep their old paths too).
- [ ] `shuyi_ui::ease::millis`, `shuyi_ui::ease::ease_in_out` and
      `shuyi_ui::frame::glyph_buffer::pixel_size` exist with every kit
      off, and `shuyi_ui::audio`, `game` and `debug` use them.
- [ ] `cargo xtask check-modules` passes with the four new features of
      `shuyi`, and fails a fifth feature that names no catalog row and
      isn't in `ENGINE_FEATURES` (its test).
- [ ] `examples/hello-grid` and every template part build and test
      as before.
- [ ] The new ADR is accepted, and ADR-0009's status line and
      `docs/adr/README.md` name it.
- [ ] Each break has a `Breaking:` line and a `migration` block in the
      completion notes.
- [ ] The book's chapter is updated (2802), or the completion notes
      have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the moved helpers' tests move with them (`millis` of a
  negative and a non-finite time is 0; `ease_in_out` of 0, 0.5 and 1).
- Unit (`xtask`): `check-modules` accepts the four kit features and
  still fails an unknown one; the gates test lists the new steps; the
  `api` tests' listings name the new features.
- Integration: the gate builds above, each kit off in turn, on the host
  and on thumbv7em.
- Integration: `cargo test --workspace` unchanged with default
  features; `cargo test -p shuyi-ui --no-default-features --features
  toy` still runs the grid kit's and dialogue's tests.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
