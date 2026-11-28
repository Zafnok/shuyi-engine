---
id: "2096"
title: "`cargo xtask api` fails a new public struct or enum that is neither #[non_exhaustive] nor listed as a plain value or a closed set"
type: infra
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-28
---

# 2096 — `cargo xtask api` fails a new public struct or enum that is neither `#[non_exhaustive]` nor listed as a plain value or a closed set

## Context

[ADR-0039](../../docs/adr/0039-public-types-that-can-grow-non-exhaustive-config-and-views-with-constructors-plain-values-left-open.md)
sorts every public type into eight kinds. Outside `shuyi-app`, whose
`crates/app/src/lib.rs` turns on `clippy::exhaustive_structs` and
`exhaustive_enums` (line 175), only the skills' lines and the semver
check hold the rule, and the semver check sees the break only when a
field is added. 2082 to 2086 moved the types that open tickets planned
to grow. The retro of PRs #379–#484 (2026-11-17, problem 8) found that
the sweep missed types that are growing now (2095). The next miss would
be a type added today as exhaustive that grows a month later, after 0.1.

`cargo xtask api` already makes a listing (`cargo-public-api`, ticket
2010), and the listing shows the attribute:
`#[non_exhaustive] pub struct shuyi_core::bots::plan::PlanSettings`,
but `pub struct shuyi_core::grid::Pos`. Measured on 2026-11-17 with
`cargo +nightly-2026-07-06 public-api -p <crate> -sss` (default
features) over `shuyi-core`, `-assets`, `-ui`, `-platform`, `-app`,
`-cli`, `-sound` and `-script`: 759 listing lines of an exhaustive
`pub struct` or `pub enum` (about 550 types; a re-export is listed at
each of its paths), against 452 `#[non_exhaustive]` ones. Most are plain
values (`Pos`, `Rect`, `Rgb`) and closed sets (`Dir`), as ADR-0039
expects. Nothing says which.

3503 holds constructors to ADR-0039's new rule against neighbouring
numbers, checked over the same listing.

## Nick input

None.

## Scope

**In:**
- **`tools/exhaustive.ron`** lists, for each crate in `api.rs`'s
  catalog, the paths of its exhaustive public structs and enums. Paths
  are as the head's listing names them, and a re-export is listed at
  each of its paths. Each path goes under one kind:
  - `plain`: ADR-0039 kind 1;
  - `closed`: kind 6's closed sets;
  - `open`: a type that may grow but hasn't moved yet. It moves under
    ADR-0039 in the PR that grows it, with a `Breaking:` line. This list
    only shrinks.

  One line may hold several paths. A type an open ticket grows moves
  under ADR-0039 in this PR, not to `open` (step 2).
- **The check** goes in a new `crates/xtask/src/api/exhaustive.rs` and
  reads the head's listing of each crate. It fails:
  - an exhaustive `pub struct` or `pub enum` line that the file doesn't
    list;
  - a path in the file that the listing shows as `#[non_exhaustive]`,
    or doesn't show at all (a stale entry);
  - an `open` path that the base's file lacks. The base's file is read
    with `git show <base>:tools/exhaustive.ron` through `Runner`. When
    the base has no file, this rule doesn't apply.

  Each failure names the path and says what to do: make the type
  `#[non_exhaustive]` with a constructor (ADR-0039), or list it with its
  kind. The errors join `run`'s errors, as `outside_error`'s do, and
  `summary` shows them.
- ADR-0039's *How it is held* names the check and the file. The
  ADR-0039 lines of the `write-ticket` and `work-ticket` skills say that
  a new exhaustive public type is listed in the file with its kind.
- **From 2095 (done):** `FontAtlasDef`, `FontInk`, `Sound` and
  `DoctorInput` are `#[non_exhaustive]` and stay out of the file.
- **From 3503 (done):** reuse its reading of listing lines
  (`api/constructors.rs`): `type_header` reads a `pub struct` or `pub
  enum` line and whether it is `#[non_exhaustive]`, and `without_generics`
  takes a path's generic arguments out. Make them `pub(super)` here, and
  keep the two files and their error kinds apart. Its `CrateApi` keeps
  the head's constructors (`constructors`); keep the head's listing beside
  it the same way if this check needs it.
- **From 2090 (done):** its outside-crate check and this one both read
  the head's listing in `compare_crates` (its `unlisted_outside`, fed by
  `reached_outside`). Keep the two checks and their errors apart.

**Out (do not do):**
- Constructors and their arguments (3503).
- Moving the `open` types under ADR-0039 beyond what 2095 moves: each
  one moves in the PR that grows it.
- Struct-like enum variants (the last sentence of ADR-0039 kind 6) and
  traits (kind 7).
- `clippy::exhaustive_structs` in more crates: ADR-0039 says why not.

**Estimated size:** about 900 lines (code 250, tests 300, data 250,
docs 100).

## Implementation steps

1. Read ADR-0038 and ADR-0039. In `crates/xtask/src/api.rs`, read
   `compare_crates`, `CrateApi`, `outside_error`, `run` and `summary`,
   and the fakes in `api/tests.rs`.
2. List the head of each catalog crate with its features, as `listing`
   does. Sort its exhaustive structs and enums by kind into
   `tools/exhaustive.ron`. Search `tickets/open` for each name you would
   put in `open`. A type a ticket grows moves under ADR-0039 here.
3. In `api/exhaustive.rs`, parse the file with the `ron` crate xtask
   already has. Parse the listing's lines: any `#[...]` before `pub`,
   then `struct` or `enum`, then the path up to `<`, `(` or the end of
   the line. Then apply the three rules.
4. Wire the check into `compare_crates` and `run`, and show its failures
   in `summary`.
5. Update ADR-0039's *How it is held* and the two skills' lines, then
   run `pwsh -File scripts/sync-agent-skills.ps1`. Run
   `cargo xtask api` and the gates.

## Acceptance criteria

- [x] `cargo xtask api` passes on the branch with `tools/exhaustive.ron`
      in place, and no `open` path is a type that an open ticket grows.
- [x] A fake listing with a new exhaustive struct that the file doesn't
      list fails and names the struct's path. With the struct listed as
      `plain`, it passes.
- [x] A stale path fails, and so does a path the listing shows as
      `#[non_exhaustive]`.
- [x] An `open` path that the base's file lacks fails; one the base's
      file has passes.
- [x] ADR-0039 names the check and the file.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`api/exhaustive.rs`): parsing listing lines (attributes,
  generics, tuple structs, re-exports), and each failure above.
- Integration (`api/tests.rs`, faked `Runner`): a run with a new
  unlisted type fails, and the summary shows its path.

## Completion notes

**What was done.**

- `crates/xtask/src/api/exhaustive.rs` reads each catalog crate's head
  listing (`CrateApi::types`, beside `constructors`) and holds every
  exhaustive `pub struct` and `pub enum` to `tools/exhaustive.ron`. It
  fails a type the file doesn't list under its crate (`Unlisted`), a
  listed path the listing shows `#[non_exhaustive]` (`Moved`) or doesn't
  show (`Gone`), an `open` path the base's file lacks (`NewOpen`, the
  base's file read with `git show <base>:tools/exhaustive.ron` through
  `Runner`, before the base's tree is removed; no file at the base, no
  rule), and a file that doesn't parse, lists a path twice or names a
  crate outside `API_CRATES` (`File`). Its errors join `run`'s errors
  apart from the constructors' and the outside crates', and `summary`
  gets an `**Exhaustive types, fail:**` section.
- `constructors::type_header` is `pub(super)`, and now reads any
  attributes before `pub` (the listing writes `#[non_exhaustive]
  #[repr(u8)] pub enum shuyi_ui::…`, which the old reader took for a line
  inside a block) and returns the item's word and the rest of the line.
- `tools/exhaustive.ron` lists 895 paths of 661 types (a re-export at
  each of its paths), measured on 2026-11-28 with `cargo +nightly-2026-08-16
  public-api -p <crate> --simplified` and each crate's features of
  `API_CRATES`, and sorted with the rustdoc JSON that listing writes
  (whether a struct has a stripped field, is a unit or a tuple). Paths
  by kind:

  | Crate | plain | closed | private | open |
  | ----- | ----- | ------ | ------- | ---- |
  | `shuyi-core` | 33 | 13 | 53 | 36 |
  | `shuyi-assets` | 48 | 10 | 47 | 62 |
  | `shuyi-ui` | 34 | 34 | 138 | 130 |
  | `shuyi-platform` | 5 | 4 | 19 | 1 |
  | `shuyi-app` | 0 | 0 | 5 | 0 |
  | `shuyi-cli` | 22 | 9 | 17 | 115 |
  | the other seven | 7 | 6 | 41 | 6 |

  `plain` holds the values, newtypes and unit structs; `closed` the
  enums whose set is fixed (two answers, the four directions, a toy's
  choices); `open` every struct whose fields may grow and every enum
  that may gain a variant (errors, events, a setting's choices, views,
  reports, the contract's answers).
- **The types an open ticket grows moved under ADR-0039 here** (step 2):
  each name was searched in `tickets/open` in the same sentence as
  "field", "variant", "gains" or "grows", as `` `Type::field:` ``, and as
  "to `Type`" or "`Type` gets". Ten moved:

  | Type | Ticket | How it is built now |
  | ---- | ------ | ------------------- |
  | `shuyi_assets::dialogue::DialogueLimits` | 2289 (a character policy) | `DialogueLimits::new(box_width, box_lines)` and five `const fn with_<field>` |
  | `shuyi_assets::dialogue::DialogueFile` | 2140 (`story`) | `DialogueFile::new(scenes)` |
  | `shuyi_assets::dialogue::Scripts` | 2140 (`story`) | `Scripts::default()`, `scripts_from_parsed` |
  | `shuyi_ui::widgets::name_entry::TypingView` | 3232 (a caret, a selection) | `TypingView::new(title, typed, hint)` |
  | `shuyi_ui::toy::grid::Field` | 3206 (a `Viewport`) | `Field::new`, `Field::glyphs` |
  | `shuyi_cli::contract::Answer` | 3415 (`diagnostics`) | `Answer::new` and its kin |
  | `shuyi_cli::contract::export::ExportOutput` | 2846 (`sbom`), 3071 (`loads`, `glibc`) | in `shuyi-cli` |
  | `shuyi_cli::contract::session::Sound` | parked 2363 (`volume`, `pan`) | in `shuyi-cli` |
  | `shuyi_cli::catalog::ModuleInfo` | parked 3017 (`needs`) | `ModuleInfo::default()`, a manifest |
  | `shuyi_cli::project::PrivateAssets` | 3443 (`Leaks`) | `PrivateAssets::default()`, `shuyi.ron` |

  The hits that didn't move: a type of the same name (`frame_png::Step`,
  `MusicCommand::Start`, the game's `Action`, `frame::hit::Hit`,
  `Status::Planned`, 2222's `Atlas`, `lang_status::Part`, 2740's
  `Layout`, 2119's `Cell`), a ticket that says the type keeps its shape
  (2273's `Lines`, 2339's `GameHooks`, 2346's `NameRules`), derives only
  (`AtlasRect`, `ImageRect`), and parked 1123's `Placed`, which may take
  the import's fields beside it instead (it stays `plain`, so the check
  makes 1123 choose).
- `DialogueLimits::new` is in `tools/constructors.ron` (a width then a
  height, ADR-0151).
- The tickets that planned a `Breaking:` line for these types' fields now
  say the field is additive (**from 2096 (done)**): 2140, 2289, 3232,
  2846 (its criterion for the `Breaking:` line goes), 3071, 3415, 3443,
  3206, and the parked 2363 and 3017 (their clause only).
- ADR-0039 (*How it is held*, and its status line) names the check, the
  file and its four kinds; the `write-ticket` and `work-ticket` skills'
  ADR-0039 lines say a new exhaustive type is listed there with its kind.
  The loaders are current (`scripts/sync-agent-skills.ps1`).

**Deviations.**

- **A fourth kind, `private`** (ADR-0039 kind 8). 220 of the 661 types
  have a private field: no game builds them by literal, so they rightly
  stay exhaustive, yet they are neither plain values nor closed sets,
  and filing them `open` would say they must move. The listing can't
  tell them apart (it shows no private field, and a unit struct looks
  the same), so the file names them.
- **More than the ticket counted.** The ticket measured 759 lines with
  default features; with `API_CRATES`' features (as `listing` runs) it is
  908 (895 once ten moved). So the data is 487 lines, not 250, with several paths a
  line (up to 120 columns, as `tools/constructors.ron` runs to 202).
- `without_generics` stays private: a type's path ends at its first
  `<`, `(`, `;` or space, so the reader cuts there.
- The ticket's measurement named `cargo +nightly-2026-07-06 public-api
  -sss`; the run used `NIGHTLY` (`nightly-2026-08-16`) and
  `--simplified`, as `listing` does.

Size: 1,539 counted lines, the check and its data land together (the check fails every run without the file, and the file means nothing without the check) and are about 1,220 of them; the ten moves (about 290, with the book's snippet and the skill) are the ticket's own step 2, since a type a ticket grows can't be `open`: landed after it, they would sit in `open`, which the acceptance forbids.

Book: the dialogue preview's snippet (`site/snippets/src/preview.rs`) builds its limits with `DialogueLimits::new`; no chapter's words name the literal.

Skills: `shuyi-dialogue`'s step 3 and its example (`example/talk.rs`) build `DialogueLimits` with `new` and `with_<field>`.

Breaking: shuyi-assets: `shuyi_assets::dialogue::DialogueLimits`: `#[non_exhaustive]`; build it with `DialogueLimits::new(box_width, box_lines)` and its `with_<field>` methods.

```migration
since: 0.1.0
what: DialogueLimits can grow, so a game builds it with new and with_<field>, not a literal.
old: "DialogueLimits { box_width, box_lines, max_text, max_option, min_options, max_options, max_reaction_texts }"
new: "DialogueLimits::new(box_width, box_lines).with_max_text(..)"
before: |
  const LIMITS: DialogueLimits = DialogueLimits {
      box_width: 40,
      box_lines: 3,
      max_text: 200,
      max_option: 30,
      min_options: 2,
      max_options: 4,
      max_reaction_texts: 3,
  };
after: |
  // `new` takes a page for a line, 30 cells for a reply, 2 to 4
  // replies and 3 reaction lines: change only what differs.
  const LIMITS: DialogueLimits = DialogueLimits::new(40, 3).with_max_text(200);
find: ["DialogueLimits {"]
```

Breaking: shuyi-assets: `shuyi_assets::dialogue::DialogueFile`, `shuyi_assets::dialogue::Scripts`: `#[non_exhaustive]`; build a file with `DialogueFile::new(scenes)`, and scripts with `Scripts::default()` or `scripts_from_parsed`.

```migration
since: 0.1.0
what: DialogueFile and Scripts can grow, so a game builds a file with DialogueFile::new.
old: "DialogueFile { scenes }"
new: "DialogueFile::new(scenes)"
before: |
  Ok(DialogueFile { scenes })
after: |
  Ok(DialogueFile::new(scenes))
find: ["DialogueFile {"]
```

Breaking: shuyi-ui: `shuyi_ui::widgets::name_entry::TypingView`, `shuyi_ui::toy::grid::Field`: `#[non_exhaustive]`; build a typing view with `TypingView::new(title, typed, hint)`, and the toy's field with `Field::new` or `Field::glyphs`.

```migration
since: 0.1.0
what: TypingView can grow (a caret, a selection), so a game's skin test builds it with TypingView::new.
old: "TypingView { title, typed, hint }"
new: "TypingView::new(title, typed, hint)"
before: |
  let typing = TypingView {
      title: "Name".to_string(),
      typed: "Ash".to_string(),
      hint: "Type a name".to_string(),
  };
after: |
  let typing = TypingView::new("Name", "Ash", "Type a name");
find: ["TypingView {"]
```

Breaking: shuyi-cli: `shuyi_cli::contract::Answer`, `contract::export::ExportOutput`, `contract::session::Sound`, `catalog::ModuleInfo`, `project::PrivateAssets`: `#[non_exhaustive]`; build an answer with `Answer::new` (or `failed`), and `ModuleInfo` and `PrivateAssets` with `default()` and their fields assigned; read the other two, and match them with `..`.

```migration
since: 0.1.0
what: Five of the CLI's types can grow, so a game matches them with .. and builds them with their constructors.
old: "Answer { contract, op, ok, result, error, code, help, limits }"
new: "Answer::new(op, ok, result, limits)"
before: |
  let Answer { ok, result, error, code, help, limits, contract, op } = answer;
after: |
  let Answer { ok, result, error, .. } = answer;
```

The game, read on 2026-11-28 (`gh search code --repo
Zafnok/visions-of-shuyi` for `DialogueLimits`, `DialogueFile`,
`TypingView`, `ExportOutput`, `ModuleInfo`, `PrivateAssets`, `Scripts`,
`Answer::new`, `contract::session::Sound` and `toy::grid`, then its
files through the contents API at `main`): it builds `DialogueLimits`
once (`crates/content/src/dialogue.rs` line 72), `DialogueFile` three
times (`crates/content/src/dialogue.rs` 628, `formats.rs` 145,
`crates/ui/src/story_preview.rs` 116) and `TypingView` twice
(`crates/ui/src/widgets/name_entry/playdate.rs` 133,
`screens/lead_select/tests.rs` 506): each is a `Breaking:` line above,
and the game's PR that moves the pin follows it. Its `Scripts` is its
own type; it names none of the cli's five types and no toy type in code.


No `Behaviour:` line: no output, text or order changes, only how ten
types are built.
