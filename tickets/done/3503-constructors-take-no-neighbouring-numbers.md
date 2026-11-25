---
id: "3503"
title: "Before the first release, no constructor of a growing type takes two neighbouring numbers, checked by cargo xtask api"
type: infra
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-25
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

- [x] `Display::new`, `MusicSwitch::new`, `MusicCue::new` and
      `PadState::new` no longer take two neighbouring numbers or tuples.
- [x] `cargo xtask api` passes on the branch. Every entry in
      `tools/constructors.ron` has a reason.
- [x] A fake listing with `MusicSwitch::new(u32, u32, bool)` on a
      non-exhaustive type fails and names it. It passes once the file
      lists it. A constructor of an exhaustive type passes, and so does a
      `self` method with one number.
- [x] A listed path that no longer breaks the rule fails as stale.
- [x] ADR-0039 states the rule and names the check and the file.
- [x] The completion notes have a `Breaking:` line and a `migration`
      block for each constructor that changed.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`api/constructors.rs`): integers of mixed widths, floats,
  tuples, `self` methods, generics in the path, a listed entry, and a
  stale entry.
- Integration (`api/tests.rs`, faked `Runner`): a run with a new
  breaking constructor fails, and the summary shows its path.
- For each constructor that changes, a test shows it builds what it
  built before.

## Completion notes

**What was done.**

- The check, in `crates/xtask/src/api/constructors.rs`: it reads each
  catalog crate's head listing, gives each function to the type whose
  block (its `pub struct` or `pub enum` line) it is listed in, and fails a
  `pub fn` of a `#[non_exhaustive]` type that returns `Self` (or its own
  type's path), takes no receiver, and takes two neighbouring integers of
  any widths, two floats or two tuples of one type, unless
  `tools/constructors.ron` lists it. It fails a listed path that no longer
  breaks the rule or that no listing has, an entry with no reason, and an
  entry listed twice. `CrateApi` keeps the head's constructors; `run` adds
  the check's one error and appends a **Constructors, fail** section to the
  summary. A missing file allows nothing.
- The rule is ADR-0150, which amends ADR-0039 kinds 2 to 5; ADR-0039's
  status line states it and names the check and the file. The
  `work-ticket` and `write-ticket` skills' ADR-0039 lines name it too.
- Measured on 2026-11-25 over every catalog crate with its chosen
  features: 37 constructors broke the rule. Beyond the ticket's lists,
  they were `Tolerance::new`, `PointerInput::at`, `SfxDef::new`, and
  `shuyi-alloc-count`'s `Counts::new` and `TagCounts::new`;
  `Params::with_balance` no longer breaks it.
- **Changed (7):** `Display::new`, `MusicSwitch::new` (removed),
  `MusicCue::new`, `PadState::new`, `StickDef::new` (removed),
  `Tolerance::new`, `MusicClock::new`. Each keeps `const fn` where it was;
  each has a test that builds what a literal built
  (`each_constructor_builds_what_a_literal_built` in `target.rs`,
  `a_music_cues_constructor_builds_what_a_literal_built`,
  `the_switch_constructors_set_its_three_fields`,
  `a_sticks_constructor_builds_what_a_literal_built`, the `PadState`
  literal in `input/pad/tests.rs`, the read `Tolerance`s in
  `raster/tests.rs`, and the `MusicClock` literal in `ui/src/audio.rs`).
- **Listed for their order (11):** `FontAtlasDef::new` (from 2095),
  `ImageInfo::new`, `Painted::rgba`, `one_bit` and `indexed`,
  `DisplayInfo::new`, `Window::new` (a width then a height);
  `PointerInput::at`, `BoxPlace::new` (an x then a y);
  `PadKind::from_ids` (a USB vendor id then a product id);
  `Window::console` (a scale then a margin, seen at once if swapped).
- **Listed until 3537 changes them (19):** the split the ticket named (the
  bots' settings `PlanSettings`, `RolloutSettings`, `BalanceSettings`,
  `Tries`, `Strike`; the perf rows `HeapCounts`, `HeapDiff`,
  `HeapSnapshot`, `SpanEvent`, `TagDelta`, `TagRow` and `PerfRead::of`),
  and `Counts::new`, `TagCounts::new`, `Grid::top_down`, `SfxDef::new`,
  `PageLimits::new`, `PackSpan::new` and `Limits::new`.

**Deviations.**

- The rule is a new ADR (ADR-0150) that amends ADR-0039, not text inside
  ADR-0039: the `write-adr` skill never rewrites an accepted ADR's
  decision. ADR-0039's status line names the rule, the check and the file.
- 2096 is not done, so the listing's reader is here; 2096's clause now
  says which functions to reuse.
- Split at the ticket's **Split:** point, wider than it named: at 1,312
  counted lines with nine constructors changed and no ron file or ADR yet,
  `Grid::top_down` and `SfxDef::new` (changed, then put back), the heap
  counts of `shuyi-alloc-count` and three more found (`PageLimits`,
  `PackSpan`, `Limits`) went to 3537 with the bots and the perf rows.
- `Window::console` was in the ticket's "read well" group as a width then a
  height. It takes a scale then a margin, and is listed with that reason:
  changing it changes every template and the book, and a swap shows at
  once.
- The ticket's line numbers (`target.rs:236`, `audio.rs:146`, `audio.rs:245`,
  `pad.rs:281`) had moved; the items had not.

**Follow-up tickets.** 3537 (the 19 constructors this one listed until it
changes them).

Breaking: shuyi-assets: `target::Display::new`: takes only the colour; set the text sizes with `with_min_text_px` and `with_good_text_px`

```migration
since: 0.1.0
what: A target's display takes its text sizes by name.
old: Display::new(Color, u16, u16)
new: Display::new(Color).with_min_text_px(u16).with_good_text_px(u16)
before: |
  let display = Display::new(Color::Rgba8, 8, 12);
after: |
  let display = Display::new(Color::Rgba8).with_min_text_px(8).with_good_text_px(12);
find: ["Display::new(Color::Rgba8, 8, 12)"]
```

Breaking: shuyi-assets: `audio::MusicSwitch::new`: removed; build from `after_fade(out_ms)` with `with_in_ms` and `with_overlap`, or `crossfade(ms)`

```migration
since: 0.1.0
what: A music switch names its fade-in and its overlap.
old: MusicSwitch::new(u32, u32, bool)
new: MusicSwitch::after_fade(u32).with_in_ms(u32).with_overlap(bool)
before: |
  let switch = MusicSwitch::new(500, 1000, false);
after: |
  let switch = MusicSwitch::after_fade(500).with_in_ms(1000);
find: ["MusicSwitch::new("]
```

Breaking: shuyi-assets: `audio::MusicCue::new`: takes no volume (it starts at 100); set it with `with_volume`

```migration
since: 0.1.0
what: A music cue takes its file, length and credit, and its volume by name.
old: MusicCue::new(String, u8, u32, CreditRef)
new: MusicCue::new(String, u32, CreditRef).with_volume(u8)
before: |
  let cue = MusicCue::new("theme.ogg".into(), 80, 90_000, CreditRef::Own);
after: |
  let cue = MusicCue::new("theme.ogg".into(), 90_000, CreditRef::Own).with_volume(80);
find: ["MusicCue::new(\"theme.ogg\".into(), 80,"]
```

Breaking: shuyi-assets: `keymap::StickDef::new`: removed; start from `StickDef::DEFAULT` and use `with_press_percent` and `with_release_percent`

```migration
since: 0.1.0
what: A stick's thresholds are set by name, from StickDef::DEFAULT.
old: StickDef::new(u8, u8)
new: StickDef::DEFAULT.with_press_percent(u8).with_release_percent(u8)
before: |
  let pads = Pads::new(StickDef::new(60, 40));
after: |
  let pads = Pads::new(StickDef::DEFAULT.with_press_percent(60).with_release_percent(40));
find: ["StickDef::new("]
```

Breaking: shuyi-ui: `input::pad::PadState::new`: takes only the buttons, sticks at rest; set them with `with_left_stick` and `with_right_stick`

```migration
since: 0.1.0
what: A pad's state names its sticks.
old: PadState::new(ButtonSet, (f32, f32), (f32, f32))
new: PadState::new(ButtonSet).with_left_stick((f32, f32)).with_right_stick((f32, f32))
before: |
  let state = PadState::new(buttons, (lx, ly), (rx, ry));
  let still = PadState::new(buttons, (0.0, 0.0), (0.0, 0.0));
after: |
  let state = PadState::new(buttons).with_left_stick((lx, ly)).with_right_stick((rx, ry));
  let still = PadState::new(buttons);
find: ["PadState::new(buttons, ("]
```

Breaking: shuyi-ui: `frame::raster::Tolerance::new`: takes only the channel (no pixel past it); set the pixels with `with_pixels`

```migration
since: 0.1.0
what: A picture tolerance names the pixels it allows.
old: Tolerance::new(u8, u32)
new: Tolerance::new(u8).with_pixels(u32)
before: |
  let tolerance = Tolerance::new(4, 10);
after: |
  let tolerance = Tolerance::new(4).with_pixels(10);
find: ["Tolerance::new(4, 10)"]
```

Breaking: shuyi-ui: `audio::MusicClock::new`: takes the cue and the length, at position 0; set the position with `with_position`

```migration
since: 0.1.0
what: A music clock names its position.
old: MusicClock::new(impl Into<String>, f32, f32)
new: MusicClock::new(impl Into<String>, f32).with_position(f32)
before: |
  let clock = MusicClock::new("theme", 1.5, 10.0);
after: |
  let clock = MusicClock::new("theme", 10.0).with_position(1.5);
find: ["MusicClock::new(\"theme\", 1.5, 10.0)"]
```

Playdate: none, only call sites changed there (`PadState::new` in its input and `MusicCue::new` in two tests): what the backend and the exporter do is the same.

Book: none, the book builds none of the changed constructors (no file under `site/content/learn/` or `site/snippets/` names them).

Game: the game's next pin move adopts it: `MusicCue::new` (`crates/ui/src/audio.rs`, `screen.rs`, `crates/playdate/src/audio.rs`), `PadState::new` and `StickDef::new` (`crates/playdate/src/pads.rs`, `crates/ui/src/game.rs`, `crates/content/src/keymap.rs`), `MusicSwitch::new` (`crates/ui/src/screens/camp/music.rs`) and `MusicClock::new` (`camp/music.rs`, `camp/tests.rs`, `title/tests.rs`) change as the migration blocks say (`gh search code --repo Zafnok/visions-of-shuyi`, 2026-11-25). `BoxPlace::new`, `RolloutSettings::new` and `PointerInput::at` don't change here.

Nick input: none.
