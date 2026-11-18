---
id: "3503"
title: "Before the first release, no constructor of a growing type takes two neighbouring numbers, checked by cargo xtask api"
type: infra
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3503 — Before the first release, no constructor of a growing type takes two neighbouring numbers, checked by `cargo xtask api`

## Context

[ADR-0039](../../docs/adr/0039-public-types-that-can-grow-non-exhaustive-config-and-views-with-constructors-plain-values-left-open.md)
replaces a growing type's literal with a constructor. A literal names
each field. A constructor names none, so `PlanSettings::new(4, 8, 2, 16,
10)` reads nothing, and two of its numbers can swap without a word. The
retro of PRs #379–#484 (2026-11-17, problem 8; split out of 2096) named
four to revisit:

- `target::Display::new(color, min_text_px: u16, good_text_px: u16)`
  (`crates/assets/src/target.rs:236`);
- `audio::MusicSwitch::new(out_ms: u32, in_ms: u32, overlap)`
  (`crates/assets/src/audio.rs:146`; the retro's "`Fade::new`");
- `audio::MusicCue::new(file, volume: u8, length_ms: u32, credit)`
  (`audio.rs:245`): a `u8` beside a `u32`, so `new(f, 80, 100, c)`
  compiles either way round;
- `input::pad::PadState::new(buttons, (f32, f32), (f32, f32))`
  (`crates/ui/src/input/pad.rs:281`).

The listing that `cargo xtask api` makes (`cargo-public-api`) shows
argument types but not their names: `pub const fn
shuyi_core::bots::plan::PlanSettings::new(u32, u32, u32, u32, u32) ->
Self`. Measured on 2026-11-17 with `cargo +nightly-2026-07-06 public-api
-p <crate> -sss` over `shuyi-core`, `-assets`, `-ui`, `-platform`,
`-app`, `-cli`, `-sound` and `-script`, 32 constructors break the rule.
Each is a `pub fn` of a `#[non_exhaustive]` type that returns `Self` and
takes two neighbouring integers, two floats, or two tuples of one type.
Some are a width then a height and read well: `Window::new`,
`Window::console`, `ImageInfo::new`, `DisplayInfo::new`, and
`Painted::rgba`, `one_bit` and `indexed`. Others don't:
`PlanSettings::new`, `RolloutSettings::new`, `BalanceSettings::new`,
`Tries::new`, `Strike::new`, `BoxPlace::new(Rect, i32, i32)`,
`StickDef::new(u8, u8)`, `PadKind::from_ids`, `MusicClock::new(_, f32,
f32)`, `Grid::top_down(Pos, (i32, i32), (i32, i32), (i32, i32), i32)`,
`Params::with_balance`, `Limits::new`, `PageLimits::new`,
`PackSpan::new`, `PerfRead::of`, and the perf rows (`HeapCounts`,
`HeapDiff`, `HeapSnapshot`, `SpanEvent`, `TagDelta`, `TagRow`).

The game calls several of them (`gh search code --repo
Zafnok/visions-of-shuyi`, at the game's pin `e738ffc2`):

- `MusicCue::new` in `crates/ui/src/audio.rs`, `screen.rs` and
  `crates/playdate/src/audio.rs`;
- `PadState::new` and `StickDef::new` in `crates/playdate/src/pads.rs`
  and `crates/ui/src/game.rs`;
- `MusicSwitch::new` in `crates/ui/src/screens/camp/music.rs`;
- `BoxPlace::new` in `crates/ui/src/screens/dialogue/glyph.rs`;
- `RolloutSettings::new` in its copy of the skill pack's
  `shuyi-balance-bots` example.

Each change breaks the game. Before 0.1 (2005) that costs nothing.
After it, each would be a major version.

## Nick input

None.

## Scope

**In:**
- **The rule** goes in ADR-0039, for kinds 2 to 5: a constructor takes
  no two neighbouring arguments that a literal could swap. That means
  two integers of any width, two floats, or two values of one tuple
  type. Such values go in named constructors (`MusicSwitch::crossfade`
  is one already), a `with_<field>`, or a plain value (kind 1) whose
  fields name them. A width then a height, and an `x` then a `y`, may
  stay, but each is listed. Two strings are not covered: call sites
  pass names and paths that read apart.
- **`tools/constructors.ron`**: the constructors allowed to break the
  rule, each by its path in the listing, with its reason ("a width then
  a height").
- **The check**, in a new `crates/xtask/src/api/constructors.rs`. It
  reads the head's listing of each catalog crate and fails a `pub fn`
  of a `#[non_exhaustive]` type that returns `Self` and breaks the rule,
  unless the file lists it. It also fails a listed path that no longer
  breaks the rule, or no longer exists. Its errors join `run`'s errors,
  and `summary` shows them.
- The four constructors above change shape. Each of the other 28 either
  changes or goes in the file with its reason. The completion notes say
  which.
- All their call sites change: in the engine, the templates, the
  tutorial's steps, the site's snippets and the skill pack's examples.
  `Breaking:` lines with `migration` blocks cover each constructor that
  changes. The game's PR that moves the pin follows this one.
- **Either order with 2096:** if 2096 is done, reuse its parser of
  listing lines (`api/exhaustive.rs`) and its set of
  `#[non_exhaustive]` paths; if not, write the parser here, and 2096
  reuses it. The two files and the two error kinds stay apart.
- **From 2095 (done):** `FontAtlasDef::new(cell_w, cell_h, glyphs,
  columns)` (a cell's width then its height) goes in the file.

**Out (do not do):**
- Constructors of exhaustive types (kind 1, `Pos::new(x, y)`,
  `Rect::new`, `Rgb::new`): the order of their fields is their meaning.
- Argument names: the listing has none, so the check reads types only.
- Exhaustive types (2096).

**Estimated size:** about 1,800 lines (code 900, tests 400, data 50,
docs 450).

**Split:** past 1,000 lines, split off the `shuyi-core` bots' settings
(`PlanSettings`, `RolloutSettings`, `BalanceSettings`, `Tries`,
`Strike`) and the perf rows of `shuyi-ui` and `shuyi-cli` into a new
ticket of the same milestone, and list them in `tools/constructors.ron`
until it lands.

## Implementation steps

1. Read ADR-0039, and `compare_crates`, `run` and `summary` in
   `crates/xtask/src/api.rs`, with the fakes in `api/tests.rs`. List the
   32 constructors again from the head with each crate's catalog
   features. The count may have moved.
2. Write `api/constructors.rs` with its rule and file, and wire it into
   `run` and `summary`.
3. Add the rule to ADR-0039, with the four constructors as its examples.
4. Change the four constructors, then the others, crate by crate, with
   their call sites. Keep `const fn` where it was. Then list the ones
   that stay.
5. Write the `Breaking:` lines and the `migration` blocks, one for each
   constructor that changed. Run `cargo xtask api` and the gates.

## Acceptance criteria

- [ ] `Display::new`, `MusicSwitch::new`, `MusicCue::new` and
      `PadState::new` no longer take two neighbouring numbers or tuples.
- [ ] `cargo xtask api` passes on the branch. Every entry in
      `tools/constructors.ron` has a reason.
- [ ] A fake listing with `MusicSwitch::new(u32, u32, bool)` on a
      non-exhaustive type fails and names it. It passes once the file
      lists it. A constructor of an exhaustive type passes, and so does a
      `self` method with one number.
- [ ] A listed path that no longer breaks the rule fails as stale.
- [ ] ADR-0039 states the rule and names the check and the file.
- [ ] The completion notes have a `Breaking:` line and a `migration`
      block for each constructor that changed.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`api/constructors.rs`): integers of mixed widths, floats,
  tuples, `self` methods, generics in the path, a listed entry, and a
  stale entry.
- Integration (`api/tests.rs`, faked `Runner`): a run with a new
  breaking constructor fails, and the summary shows its path.
- For each constructor that changes, a test shows it builds what it
  built before.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
