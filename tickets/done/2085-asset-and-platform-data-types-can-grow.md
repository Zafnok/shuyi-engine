---
id: "2085"
title: "Before the first release, the target profiles, packs, language packs, credits and bot settings that open tickets grow move under ADR-0039"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-16
---

# 2085 — The assets' and the platform's data types that open tickets grow move under ADR-0039 before the first release

## Context

The fourth of five tickets that move, before 2005, every exhaustive
public type an open ticket plans to grow (2082 has the reasons and
Nick's words of 2026-11-14). These are data types, most of them read
from files (ADR-0039 kind 5). Checked exhaustive on 2026-11-14:

| Type | Where | Grown by |
| ---- | ----- | -------- |
| `TargetProfile` | `crates/assets/src/target.rs:57` | 3017, 3023 |
| `Status` (a target's) | `target.rs:82` | the export targets' tickets |
| `Display` | `target.rs:92` | 3017 |
| `Color` | `target.rs:131` | 3017 |
| `Input` (a target's) | `target.rs:178` | 2406, 3021, 3024 |
| `Budgets` | `target.rs:203` | 3009, 3020 |
| `PackInfo` | `crates/assets/src/pack.rs:104` | 3059 |
| `LangInfo` | `crates/assets/src/lang.rs:175` | 2346 |
| `PackRules` | `lang.rs:520` | 2241 |
| `SourceLine` | `crates/assets/src/lang/source.rs:36` | 2241 |
| `FileCredit` | `crates/assets/src/credits.rs:138` | 2223 |
| `CreditsFile` | `credits.rs:198` | 2223 |
| `PlanSettings` | `crates/core/src/bots/plan.rs:48` | 2133 |
| `Presented::Game` (a variant) | `crates/platform/src/headless.rs:47` | 2333, 2346 |

`RolloutSettings` (`crates/core/src/bots/rollout.rs:82`) is already
`#[non_exhaustive]`. 2241 says its new fields can't take the attribute.
This ticket adds the attribute, and its PR rewrites that line of 2241.

## Nick input

None.

## Scope

**In:**
- Each type in the table under ADR-0039. Data read from files takes
  `#[serde(default)]` on every later field, so files keep loading. A
  type games build in code gets its `new`; `Presented::Game` becomes a
  `#[non_exhaustive]` variant with a constructor.
- Every engine, template, example and tutorial-step call site.
- The listed tickets' planned `Breaking:` lines for these types become
  additive, and 2241's line about the attribute is rewritten.
- A `Breaking:` line in the completion notes, after a search of the
  game for literals of each type.

**Out (do not do):**
- Any field the listed tickets add; `pack::VERSION` (a baked pack's
  format, not a type).
- 2133's `deny_unknown_fields`, a change to which files load, which
  2133 makes itself before the release.
- `shuyi-cli`'s types (2086).

**Estimated size:** about 700 lines (code 350, tests 200, docs 150).

## Implementation steps

1. The attributes and constructors, file by file.
2. Build the workspace, the templates and the tutorial steps, and fix
   each literal and exhaustive match. `shuyi.ron`'s targets and the
   engine's target profiles must load unchanged.
3. Edit the listed tickets' planned `Breaking:` lines, and 2241's.

## Acceptance criteria

- [x] Every type in the table is `#[non_exhaustive]`, and a test outside
      its crate builds each one that games build through its
      constructor.
- [x] Every target profile, language pack, credits file and bot
      settings file in the repository and the templates loads
      unchanged.
- [x] `cargo xtask api` lists each change, and the `Breaking:` line
      names each type.
- [x] No open ticket still plans a `Breaking:` line for one of these
      types.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the constructors build what a literal built before.
- The existing target, pack, language and credits tests pass unchanged.

## Completion notes

What was done:

- Every type of the table is `#[non_exhaustive]`, each with a doc line
  naming its ADR-0039 kind:
  - kind 5, data read from files: `TargetProfile::new(id, status,
    display, frame_rate, input, backend)` (quit by the game, no budget),
    `Display::new(color, min_text_px, good_text_px)` (a `const fn`; no
    sizes, no safe area, not windowed), `Input` and `Budgets` from
    their `Default` with fields assigned, `PackInfo::new(stamp, target,
    content)`, `LangInfo::new(name, made_by)`, `FileCredit::new(id,
    group, title, author, source, license)` (shown, no private files),
    `CreditsFile::new(credits)`, and `PlanSettings::new(width,
    candidates, depth, samples, temperature)` (a `const fn`, as
    `RolloutSettings::new` is);
  - kind 2: `PackRules::new(data_problem, tokens, limits, widths)` and
    `with_variants`, both `const fn`, so a game keeps its `const RULES`;
  - kind 3: `SourceLine::new(scene, text, kind, file)`;
  - kind 6: `Status` and `Color` are `#[non_exhaustive]` enums, and
    `Presented::Game` a `#[non_exhaustive]` variant built with the
    `const fn` `Presented::game(buffer, background, fullscreen)`.
- No field was added and no serde attribute changed, so every target
  profile, `lang.ron`, credits file and pack loads as before, and
  `pack::VERSION` stays. Each field the file types gain is
  `#[serde(default)]`; their docs say so. `PackInfo`'s says a field it
  gains moves `pack::VERSION` instead, since a pack's index is not
  self-describing.
- Call sites outside the defining crates: the `PackRules` literals in
  `shuyi-ui` (the toy) and `shuyi-cli` (`check`, `font_atlas`,
  `frame_png`'s tests, `lang_status`) call `PackRules::new`;
  `PackInfo { stamp, ..PackInfo::default() }` in `shuyi-app` and
  `shuyi-cli`'s export tests became `PackInfo::default()` with the field
  assigned. No match on `Status`, `Color` or `Presented::Game` outside
  its crate lacked a `_` arm or `..`. The templates, the tutorial's
  steps, the snippets, the examples and the skill pack build none of
  these types.
- Tests: `each_constructor_builds_what_a_literal_built` in `target.rs`,
  `lang.rs` and `credits.rs`,
  `the_pack_rules_constructor_builds_what_a_literal_built` in `lang.rs`,
  `the_constructor_builds_what_a_literal_built` in `pack/tests.rs` and
  `bots/plan/tests.rs`, and
  `the_game_constructor_builds_what_a_literal_built` in
  `headless/tests.rs`. Outside the crates:
  `crates/assets/tests/it/constructors.rs` (each assets type, a `const`
  `PackRules`, and a profile built in code equal to the same profile
  read from its file), `crates/core/tests/constructors.rs`
  (`PlanSettings` in a `const`, and `RolloutSettings`) and
  `a_game_frame_built_by_its_constructor_is_the_one_presented` in
  `shuyi-platform`'s `tests/it`. No snapshot or golden table changed.
- The tickets that grow these types now grow them additively: 2241 (its
  **Out** line against the attribute is gone; `PackRules::new` starts
  `speech_problem` at `|_, _| None` and `with_speech_problem` sets it),
  2223 (`CreditsFile`, `FileCredit`), 3059 (`PackInfo`; its break of
  `pack::VERSION` stays), 2346 (`LangInfo`, `Presented::Game`;
  `FrameOutput` stays a break), 2333 (`Presented::Game`) and 3017
  (`TargetProfile`, parked: only a **From 2085 (done)** clause). 2406
  already counted on this ticket; 3023, 3009, 3020, 3021, 3024 and 2133
  planned no `Breaking:` line for these types.

Deviations:

- Some existing tests changed, against "pass unchanged": those outside
  the defining crate that built `PackRules` or `PackInfo` by literal or
  with `..` now call the constructor or assign the field, which the
  attribute makes necessary. Their assertions are unchanged. A few
  `PackRules` literals inside `shuyi-assets` moved to `PackRules::new`
  too, so the crate's own tests show the form games use.
- `Input` and `Budgets` get no `new`: every field is optional and both
  have a `Default` (kind 4's form).

The game, searched on 2026-11-16 at its `main` (`2ad415f`, pin
`e074d3ec`) with a shallow clone and `grep` for each type's literal,
match and path. Its pin move changes:

- `PackRules`: one `const` literal (`crates/content/src/lang.rs:57`)
  becomes `PackRules::new(data::text_problem, &LeadTokens, LIMITS,
  &CellWidths::NARROW).with_variants(&[MALE, FEMALE])`.
- `TargetProfile` is read with `from_source`, `Quit` is named, and
  `PlanSettings` is held in `Persona`: none built by literal. `Status`,
  `Display`, `Color`, `Input`, `Budgets`, `PackInfo`, `LangInfo`,
  `SourceLine`, `FileCredit`, `CreditsFile` and `Presented::Game`: no
  literal, match or use found.

Game: nothing to adopt beyond the pin move: the game's next pin move past
this change rewrites its one `PackRules` literal; no game ticket is
needed before it.

Book: none, no page of the book or snippet builds one of these types by
literal or matches `Status`, `Color` or `Presented::Game`.

Breaking: shuyi-assets: TargetProfile, Display, Input, Budgets: #[non_exhaustive]; build them with TargetProfile::new and Display::new, and Input and Budgets from Default with the fields assigned

```migration
since: 0.1.0
what: A target profile built in code uses TargetProfile::new and Display::new, and Input and Budgets start from Default.
old: "TargetProfile { id, status, display, frame_rate, input, quit, budgets, backend }, Display { sizes, color, min_text_px, good_text_px, safe_area, windowed }"
new: TargetProfile::new(id, status, display, frame_rate, input, backend), Display::new(color, min_text_px, good_text_px)
before: |
  let display = Display { sizes: vec![], color: Color::Rgba8, min_text_px: 8, good_text_px: 12, safe_area: Insets::default(), windowed: true };
  let profile = TargetProfile { id: "desk".into(), status: Status::Planned, display, frame_rate, input: Input::default(), quit: Quit::Game, budgets: Budgets::default(), backend: "shuyi-app".into() };
after: |
  let mut display = Display::new(Color::Rgba8, 8, 12);
  display.windowed = true;
  let profile = TargetProfile::new("desk", Status::Planned, display, frame_rate, Input::default(), "shuyi-app");
find: ["TargetProfile {", "Display {"]
```

Breaking: shuyi-assets: target::Status, target::Color: #[non_exhaustive]; a match on either has a `_` arm

```migration
since: 0.1.0
what: A match on a target's Status or Color has a wildcard arm.
old: Status, Color (exhaustive)
new: Status, Color (#[non_exhaustive])
before: |
  match profile.display.color { Color::Rgba8 => full(), Color::OneBit => one_bit() }
after: |
  match profile.display.color { Color::OneBit => one_bit(), _ => full() }
```

Breaking: shuyi-assets: PackInfo, LangInfo, SourceLine, FileCredit, CreditsFile: #[non_exhaustive]; build them with their `new`

```migration
since: 0.1.0
what: PackInfo, LangInfo, SourceLine, FileCredit and CreditsFile are built with their new, not by a literal.
old: "PackInfo { stamp, target, content }, LangInfo { name, made_by, wide_font, wide_punctuation }, SourceLine { scene, text, kind, file }, FileCredit { id, group, title, author, source, license, hidden, private }, CreditsFile { credits }"
new: PackInfo::new(stamp, target, content), LangInfo::new(name, made_by), SourceLine::new(scene, text, kind, file), FileCredit::new(id, group, title, author, source, license), CreditsFile::new(credits)
before: |
  let info = PackInfo { stamp: "game 1".into(), target: None, content };
  let lang = LangInfo { name: "Deutsch".into(), made_by: MadeBy::Machine, wide_font: None, wide_punctuation: false };
  let credit = FileCredit { id: "font".into(), group: CreditGroup::Fonts, title: "A Font".into(), author: "Ann".into(), source: url.into(), license: "OFL-1.1".into(), hidden: true, private: vec![] };
  let file = CreditsFile { credits: vec![credit] };
after: |
  let info = PackInfo::new("game 1", None, content);
  let lang = LangInfo::new("Deutsch", MadeBy::Machine);
  let mut credit = FileCredit::new("font", CreditGroup::Fonts, "A Font", "Ann", url, "OFL-1.1");
  credit.hidden = true;
  let file = CreditsFile::new(vec![credit]);
find: ["PackInfo {", "LangInfo {", "FileCredit {", "CreditsFile {"]
```

Breaking: shuyi-assets: PackRules: #[non_exhaustive]; build it with the const fn PackRules::new and with_variants

```migration
since: 0.1.0
what: A game's PackRules are built with PackRules::new and with_variants, in a const item if it likes.
old: PackRules { data_problem, tokens, limits, widths, variants }
new: PackRules::new(data_problem, tokens, limits, widths).with_variants(variants)
before: |
  pub const RULES: PackRules<'static> = PackRules { data_problem: text_problem, tokens: &LeadTokens, limits: LIMITS, variants: &[MALE, FEMALE], widths: &CellWidths::NARROW };
after: |
  pub const RULES: PackRules<'static> = PackRules::new(text_problem, &LeadTokens, LIMITS, &CellWidths::NARROW).with_variants(&[MALE, FEMALE]);
find: ["PackRules {"]
```

Breaking: shuyi-core: PlanSettings: #[non_exhaustive]; build it with the const fn PlanSettings::new

```migration
since: 0.1.0
what: A PlanSettings is built with PlanSettings::new, not by a literal.
old: PlanSettings { width, candidates, depth, samples, temperature }
new: PlanSettings::new(width, candidates, depth, samples, temperature)
before: |
  let settings = PlanSettings { width: 4, candidates: 3, depth: 2, samples: 5, temperature: 10 };
after: |
  let settings = PlanSettings::new(4, 3, 2, 5, 10);
find: ["PlanSettings {"]
```

Breaking: shuyi-platform: Presented::Game: a #[non_exhaustive] variant; build it with Presented::game, and match it with `..`

```migration
since: 0.1.0
what: The headless backend's Presented::Game is built with Presented::game and matched with `..`.
old: Presented::Game { buffer, background, fullscreen }
new: Presented::game(buffer, background, fullscreen)
before: |
  let expected = Presented::Game { buffer, background, fullscreen: false };
after: |
  let expected = Presented::game(buffer, background, false);
find: ["Presented::Game {"]
```
