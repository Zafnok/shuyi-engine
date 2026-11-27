---
id: "3438"
title: "Domain preludes, an import check, and one spelling of identifiers: the US names beside deprecated UK ones"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-27
---

# 3438 — Domain preludes, an import check, and one spelling of identifiers: the US names beside deprecated UK ones

## Context

Split off 3413 (done) at its size valve: its steps 8–11. 3413 gave the
API doc aliases, a plain-text listing per crate (`cargo xtask api-text`,
`crates/xtask/src/api_text.rs`) and its `--check`. This ticket does the
rest of what the blind-spot audit of 2026-11-23 found (its *Agent
experience* section, candidates 63 and 68): an agent writes a game from
memory, guesses deep paths and US spellings, and each miss costs a Rust
compile round.

**What is true today** (checked 2026-11-26):

- `shuyi::prelude` (`crates/shuyi/src/lib.rs:45`) re-exports
  `shuyi_platform::prelude` (`crates/platform/src/lib.rs:108`, about 30
  names, ADR-0036 §6) and `run_game`. Past it, the templates and
  `examples/hello-grid` import from about 74 distinct module paths
  (`grep -rhoE "use shuyi[a-z_]*(::[a-z_0-9]+)+::(\{|[A-Z*])" examples
  templates --include=*.rs`), such as `shuyi::ui::frame::glyph_buffer`,
  `shuyi::core::bots::playtest` and `shuyi_cli::contract::playtest`. A
  rules crate depends on `shuyi-core` alone, which has no prelude.
- The public names mix spellings. `Color`, `EngineColor`, `ColorSet`
  and `Colors` are US. Eight names are UK: `print_centred`
  (`crates/ui/src/screens.rs:44`, `frame/draw.rs:205`),
  `print_centred_styled` (`screens.rs:53`), `centre_x` (`screens.rs:37`),
  `centre` (`crates/ui/src/cinematic.rs:192`, `debug/camera.rs:125`),
  `centred_on` (`crates/ui/src/grid/camera.rs:21`), and the variants
  `NarrationAlign::Centred` (`dialogue/text_box/glyph.rs:26`),
  `Scroll::Centred` (`widgets/list.rs:20`) and `Rule::SharedColour`
  (`crates/assets/src/onebit.rs:523`). The `centre` family is used about
  246 times in 64 files outside `tickets/`.
- The precedent for a rename: 2226 kept `Lang::english` one release as a
  `#[deprecated]` alias of `source_text`, with one `#[expect(deprecated)]`
  test per alias, and needed no `Breaking:` line.

**The game** (pin `50472bbc`, `gh search code --repo
Zafnok/visions-of-shuyi`): `print_centred` in 8 files
(`crates/ui/src/screens/*/glyph.rs`), `centre_x` in 8, `centred_on` in 6
(`map_view/place.rs`, `screens/battle/mod.rs`), `NarrationAlign` in
`screens/dialogue/glyph.rs`. The old names stay as deprecated aliases, so
the game builds; its PR that moves the pin renames its calls, as each
deprecation warning points at one.

## Nick input

None. The spelling of identifiers is a technical choice, recorded in an
ADR.

## Scope

**In:**

- **Domain preludes.** Beside `shuyi::prelude`, which stays as it is:
  - `shuyi_core::prelude`, for a rules crate: the RNG, `TurnRules`,
    `TickSim`, the step and time types, `Pos`, the ease and tween types;
  - `shuyi::prelude::grid` (the grid kit, the camera, `GridMap`),
    `shuyi::prelude::text` (styled text, the font), `shuyi::prelude::audio`
    and `shuyi::prelude::test` (the `Harness`, the playtest bots);
  - `shuyi_cli::prelude`, for a game's tool program and its tests: the
    contract types the templates import today.
  Each re-exports what the templates, `examples/hello-grid`,
  `site/snippets` and the shipped skills' worked examples import today,
  and no more. ADR-0036 §6 is amended by a line pointing at the new ADR.
- **An import check.** `cargo xtask check-imports`
  (`crates/xtask/src/imports.rs`), a row in the gates table
  (`crates/xtask/src/gates.rs`, the fast set): code under `templates/`,
  `examples/` and `site/snippets/` names an engine item only through a
  prelude, or through a path in its list of allowed deeper paths, each
  with its reason.
- **One spelling.** US spelling for identifiers, matching `Color` and
  Rust's own crates; prose keeps the house spelling. A new ADR records
  it (the `write-adr` skill). Each of the eight UK names gets its US name
  (`print_centered`, `print_centered_styled`, `center_x`, `center`,
  `centered_on`, `NarrationAlign::Centered`, `Scroll::Centered`,
  `Rule::SharedColor`); the old name stays one release as a
  `#[deprecated(note = "renamed …")]` alias, as 2226 did. A variant's
  alias is an associated `const` (the three enums derive `PartialEq` and
  `Eq`, so it still matches in a pattern). Nothing in `crates/`,
  `templates/`, `examples/`, `site/` or `skills/` calls an old name but
  one `#[expect(deprecated)]` test per alias. `shuyi_platform::prelude`
  exports `print_centered` beside the deprecated `print_centred`, and
  `check_text.rs`'s list of text sinks keeps both names
  (`crates/cli/src/check_text.rs:842`).
- **The spelling check** in `cargo xtask api-text --check`: no public
  item's name holds a UK spelling of `api_text::UK_SPELLINGS` (`centre`,
  `centred`, `colour`, `behaviour`, `normalise`, `initialise`, `favour`,
  `licence`), unless it is `#[deprecated]` (the index, `rustdoc_index`,
  reads the deprecation).
- **Public types** (ADR-0039): no new type. The renamed variants keep
  `NarrationAlign`'s `#[non_exhaustive]`; `Scroll` and `Rule` stay as
  they are. `grey` (`lighting.rs:78`) is not renamed: it is a US
  spelling too, and the list leaves it out. The preludes are re-exports
  only.

**Either order with 3520:** if 3520 is done, put `shuyi::prelude::grid`
behind the grid kit's feature, and the other preludes' items behind
theirs; if not, nothing to do (3520 gates the preludes with the rest).

**Out (do not do):**

- Removing an old name: a later release, by its own ticket.
- Renaming anything for taste beyond the UK spellings, or changing the
  house prose style.
- The doc aliases and the plain-text API: 3413 (done).

**Estimated size:** about 1,400 lines (code 500, tests 400, docs 500),
counted from the steps and doubled. The renames are about 250 call
sites.

**Split:** past 1,000 lines, split off steps 3–5 (the ADR, the renames
and the spelling check) into a new ticket.

## Implementation steps

1. **`shuyi_core::prelude`** and `shuyi::prelude::{grid, text, audio,
   test}`, `shuyi_cli::prelude`, each with module docs saying who
   imports it.
2. **`cargo xtask check-imports`** and its row in the gates table, then
   the templates, `hello-grid`, `site/snippets/` and the skills' worked
   examples moved onto the preludes until it passes.
3. **The ADR** for one spelling of identifiers.
4. **The renames**: the US names, the deprecated aliases with their
   tests, every call site in this repository, and `check_text.rs`'s
   list of text sinks.
5. **The spelling check** in `api_text.rs`'s `--check`, with
   `UK_SPELLINGS`.
6. **Docs** (the `document` skill): the book's *Tools for agents*
   chapter says which prelude to import; *Games and screens* uses the
   new names; the shipped skills' text follows.

## Acceptance criteria

- [x] `cargo xtask check-imports` passes on the templates, `hello-grid`,
      `site/snippets/` and the skills' examples, and fails, naming the
      file and line, on a fixture that imports
      `shuyi::ui::frame::glyph_buffer::Sprite` with no allowed reason.
- [x] The template's code imports from preludes only: no `use` path
      deeper than `shuyi::prelude::<domain>` or a prelude of
      `shuyi_core` or `shuyi_cli`, but the allowed ones.
- [x] **Split off to 3442:** `print_centered`, `print_centered_styled`,
      `center_x`, `center` (both), `centered_on`,
      `NarrationAlign::Centered`, `Scroll::Centered` and
      `Rule::SharedColor` exist; each old name is `#[deprecated]`, gives
      what its new name gives (one `#[expect(deprecated)]` test each),
      and still matches in a pattern for the three variants.
- [x] **Split off to 3442:** No code in this repository calls an old
      name but those tests. `shuyi check` reads a text sink by either
      name.
- [x] **Split off to 3442:** `cargo xtask api-text --check` fails on a
      public, non-deprecated item named with a UK spelling, with its own
      message, and passes on this branch.
- [x] **Split off to 3442:** The ADR for one spelling is in `docs/adr/`
      and its README. This ticket's ADR, for the preludes and the check,
      is ADR-0161.
- [x] **Split off to 3442:** Completion notes: a `Behaviour:` line per
      renamed item. This ticket renames nothing.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the import check's allowed list and a failing fixture; the
  spelling check on a fixture index.
- Unit: one `#[expect(deprecated)]` test per alias, comparing it with
  its new name (so mutation testing of the diff kills each alias's
  mutants).
- Integration: the gates' fast set runs `check-imports`.

## Completion notes

**Done (2026-11-27):** steps 1, 2 and the prelude half of step 6. Steps
3–5 (the ADR for one spelling, the renames, the spelling check) and the
*Games and screens* half of step 6 are split off to **3442**, at this
ticket's size valve: the renames alone are about 500 counted lines (250
call sites, each a line removed and a line added).

- **Preludes** (ADR-0161, amending ADR-0036 §6): `shuyi_core::prelude`
  (`crates/core/src/lib.rs`; a game's crate writes
  `shuyi::core::prelude`), `shuyi_cli::prelude` (`crates/cli/src/lib.rs`)
  and `shuyi_platform::prelude`'s `grid`, `text`, `audio` and `test`
  modules (`crates/platform/src/lib.rs`), which `shuyi::prelude`'s glob
  re-export makes `shuyi::prelude::grid` and so on. Re-exports only,
  each holding what the four trees named on 2026-11-27.
- **The check:** `cargo xtask check-imports`
  (`crates/xtask/src/imports.rs`) reads every `use` leaf and every engine
  path in the code of `templates/`, `examples/` (snake too), the book's
  snippets and `skills/`, with `shuyi_cli::rust_lines`' lexer. The
  allowed deeper paths are data, `tools/imports.ron`: 56 entries, each
  with its `why` and the files or folders it is allowed in; an entry
  nothing names fails. A gate row (`imports`, the fast set, so the
  pre-push hook runs it) and a step of CI's `layers` job.
- **The moves:** 82 files had their engine imports moved onto the
  preludes by script and merged one line per prelude: 1,053 deep paths
  before, 0 past the list after. The three `app_info.rs` files keep
  `shuyi::platform::AppInfo`: `cargo shuyi app-info` writes them and
  checks them equal, so their entry says so.
- **Docs:** the book's *Tools for agents* has *Which prelude to import*;
  the `shuyi` crate's docs, the template's `AGENTS.md` and the
  `shuyi-new-game` and `shuyi-animate` skills name the preludes.

**Deviations:**

- The split above (3442).
- The ticket's `shuyi::prelude::test` was to hold the playtest bots. The
  bots that play a turn game (`Bot`, `RandomBot`, `Rollout`, `play`, …)
  went to `shuyi_core::prelude` instead: `examples/hello-grid`'s stage
  screen plays its enemy with `Bot` in shipped code, which a
  test-only home would make read as a test. `test` keeps the Harness,
  its session files, the playtest runner (`shuyi_core::bots::playtest`,
  its module and its items) and the autobalance search. `test` exists
  without the `harness` feature; only its Harness items need it.
- The check reads `examples/snake` too (the ticket named
  `examples/hello-grid`; its scope said `examples/`), and the paths in
  code as well as in `use` (`shuyi::app::run_app(…)`): both name an
  item. The ticket's "a rules crate depends on `shuyi-core` alone": the
  template's rules crate also depends on `shuyi-assets` for its map, and
  `shuyi-assets` has no prelude, so `shuyi_assets::grid_map` is an entry
  of `tools/imports.ron`, allowed in that crate only (a game's crate
  reaches the same module through `shuyi::prelude::grid`).
- **Either order with 3520:** 3520 is not done, so nothing here; 3520's
  clause now says what it does with these preludes (its *From 3438
  (done)* line).

**Follow-up tickets:** 3442 (the split). No ticket: a prelude for the
modules `tools/imports.ron` allows (dialogue, cinematics, menus,
settings) is written when games on that domain name them, ADR-0161 §3.

No break: every item keeps its path, and the preludes only add
re-exports. The four new names in `shuyi::prelude` (`grid`, `text`,
`audio`, `test`) could meet a game's own glob of a module of that name:
`gh search code --repo Zafnok/visions-of-shuyi 'use shuyi::prelude'` at
the game's pin `96eb097c` finds only its copy of the skill pack
(`.claude/skills/*/example/*.rs`, `shuyi-new-game/SKILL.md`), whose paths
all still resolve; its own crates never glob the prelude.

Game: nothing to adopt, the old paths stay and the game's crates don't
glob `shuyi::prelude`; its skill pack's copy follows when the pin moves.

Skills: the `shuyi-new-game` and `shuyi-animate` skills and every worked
example's imports follow the preludes.

Size: 1,861 counted lines, not split further: the check's list is what the
moves leave (each entry is a path no prelude took, allowed where it is
named), and the moves are chosen by the check's failures, so a PR of the
preludes and moves alone would have nothing holding them and one of the
check alone fails on 1,053 paths. The ticket's own split (the spelling,
3442) is taken.
