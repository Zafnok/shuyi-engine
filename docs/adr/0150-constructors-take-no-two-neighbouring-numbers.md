# ADR-0150: Constructors take no two neighbouring numbers, checked by `cargo xtask api`

- **Status:** Accepted
- **Date:** 2026-11-25
- **Related tickets:** 3503 (this), 3537 (the constructors it left), 2096
  (the same listing, for exhaustive types), 2095 (`FontAtlasDef::new`)
- **Amends:** [ADR-0039](0039-public-types-that-can-grow-non-exhaustive-config-and-views-with-constructors-plain-values-left-open.md)
  kinds 2 to 5 (what their constructors may take)

## Context

ADR-0039 replaces a growing type's literal with a constructor. A literal
names each field; a constructor names none. `PlanSettings::new(4, 8, 2, 16,
10)` reads nothing, and two of its numbers can swap without a word: the call
still compiles. The retro of PRs #379–#484 (2026-11-17, problem 8) named four:

- `target::Display::new(color, min_text_px: u16, good_text_px: u16)`;
- `audio::MusicSwitch::new(out_ms: u32, in_ms: u32, overlap)`;
- `audio::MusicCue::new(file, volume: u8, length_ms: u32, credit)`: a `u8`
  beside a `u32`, so `new(f, 80, 100, c)` compiles either way round;
- `input::pad::PadState::new(buttons, (f32, f32), (f32, f32))`.

`cargo xtask api`'s listing (`cargo-public-api`, ADR-0038) shows each
function's argument types but not their names: `pub const fn
shuyi_core::bots::plan::PlanSettings::new(u32, u32, u32, u32, u32) -> Self`.
Read on 2026-11-25 over every crate of the catalog with its chosen features,
37 constructors of `#[non_exhaustive]` types took two neighbouring integers,
two floats or two tuples of one type (the ticket's count of 2026-11-17 was 32).
A listing shows a re-exported type again at its new path, and its methods at
the path that defines them.

Each change breaks the game. Before 0.1 (2005) that costs nothing; after it,
each would be a major version.

## Decision

1. **The rule**, for ADR-0039's kinds 2 to 5: a constructor takes no two
   neighbouring arguments a call could swap. That is two integers of any
   widths (a `u8` beside a `u32` too), two floats, or two values of one
   tuple type. Such values go in:
   - a named constructor (`MusicSwitch::crossfade(ms)`);
   - a `#[must_use] const fn with_<field>(self, value) -> Self`, as kind 2
     has, on any of kinds 2 to 5 (`PadState::new(buttons)
     .with_left_stick(..)`); a type with no field a game must choose starts
     from a `DEFAULT` constant (`StickDef::DEFAULT.with_press_percent(60)`);
   - or a plain value (kind 1) whose fields name them.

   Two strings are not covered: call sites pass names and paths that read
   apart. A method that takes `self` is no constructor, and neither is one
   that returns something else (`Option<Self>`).
2. **What may stay**: an order that is the meaning, as a width then a height,
   or an `x` then a `y`. Each is listed in **`tools/constructors.ron`**, by
   its path in the listing without generic arguments, with why.
3. **The check**, in `crates/xtask/src/api/constructors.rs`, reads the
   head's listing of each catalog crate. A function belongs to the type
   whose block (from its `pub struct` or `pub enum` line) it is listed in.
   It fails each `pub fn` of a `#[non_exhaustive]` type that returns `Self`
   (or its own type's path), takes no receiver and breaks the rule, unless
   the file lists it. It fails a listed path that no longer breaks the rule
   or that no listing has, an entry without a reason, and an entry listed
   twice. Its errors join `run`'s, and the summary shows them under
   **Constructors, fail**. A missing file allows nothing.
4. **What changed with it** (ticket 3503): `Display::new(color)` with
   `with_min_text_px` and `with_good_text_px`; `MusicSwitch::new` removed for
   `after_fade` and `crossfade` with `with_in_ms` and `with_overlap`;
   `MusicCue::new(file, length_ms, credit)` at full volume with
   `with_volume`; `PadState::new(buttons)` with `with_left_stick` and
   `with_right_stick`; `StickDef::new` removed for `StickDef::DEFAULT` with
   `with_press_percent` and `with_release_percent`; `Tolerance::new(channel)`
   with `with_pixels`; `MusicClock::new(cue, length)` with `with_position`.
   Ticket 3537 changes the bots' settings, the perf rows, `Grid::top_down`,
   `SfxDef::new`, `PageLimits::new`, `PackSpan::new` and `Limits::new`, which
   the file lists until it does.

## Consequences

- A call names what it passes: `MusicCue::new(file, 100_000, credit)
  .with_volume(80)` can't be read the wrong way round.
- A new constructor that breaks the rule fails the PR that adds it, before
  any game calls it.
- More methods: one `with_<field>` for each value that left a constructor,
  and longer calls where a game sets several.
- The check reads types, not names: two `u32`s that a width and a height
  name pass only through the file, and a constructor taking a `u32` beside an
  `f32` passes, though both are numbers (a swap there doesn't compile).
- It reads `cargo-public-api`'s line format; a new version that changes it
  shows as entries gone stale, which fail.

## Alternatives considered

- **A newtype for each unit** (`Millis`, `Percent`). It names the kind of
  number, not which one: `out_ms` and `in_ms` are both `Millis`, and would
  still swap.
- **Builder types.** ADR-0039 already turned them down for kind 2: a second
  type per config, with nothing gained over `with_<field>`.
- **Clippy's `too_many_arguments` or `fn_params_excessive_bools`.** They
  count arguments or bools, and two `u32`s are neither.
- **Reading the source with `syn`** for argument names. The listing is what
  a game sees and what ADR-0038's gate already builds; its types are enough
  for the rule.
