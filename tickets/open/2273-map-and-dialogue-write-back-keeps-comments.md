---
id: "2273"
title: Grid maps and dialogue files are written back without losing comments or layout
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2244"]
nick_input: none
completed:
---

# 2273 — Grid maps and dialogue files are written back without losing comments or layout

## Context

Found by the blind-spot audit of 2026-11-22 (candidate 33, "no level
editor at any stage", high). A tool that changes a map or a scene has to
write the file back. Today that write throws away what the author wrote
around the data. This is the first step toward a level editor, and it
helps every tool that edits these files now: the autobalance's
`--apply`, an agent that moves a sign, a writer's tool that fixes a line.

What Shuyi has (checked 2026-11-23):

- `.map` (`crates/assets/src/grid_map.rs`) is a RON header, a `---`
  line, then one character per tile. `grid_map::print` writes a whole
  map. It writes the header with `ron::to_string` on one line, so the
  file's comments and its layout go. Hello, Grid's
  `examples/hello-grid/files/stage.map` opens with three `//` comment
  lines, and they would be lost. (The audit said the map format "has no
  writer". It has one, but it is lossy and it is not an `Encode`.)
- `.dlg` (`crates/assets/src/dialogue.rs`) has `print_scene`, which
  writes one scene from scratch. A file holds several scenes, `#`
  comments and blank lines (`dialogue/parse.rs`, `Parser::line`), long
  lines continued on indented lines, and `@delivery` lines, which are
  not steps: the parser folds them into each line's `direction`.
  `print_scene` keeps none of that. `examples/hello-grid/files/intro.dlg`
  opens with six comment lines.
- `ParsedScene` (`dialogue/parse.rs`) already records where things are:
  `Lines::steps` is the start line of each step, and `PartLines::line`
  the line of each option, `@if` and `@else`. It has no end lines.
  `Lines` is a public struct that is not `#[non_exhaustive]`, so a new
  field on it would break the API.
- The only `Encode` impls are `OneBitFile` (`bitmap.rs`),
  `SaveBundleRon` (`crates/ui/src/storage/bundle.rs`) and a test's
  `TxtImg`. 2218 writes `Encode` for five RON formats and leaves `.dlg`
  and `.map` out because "their files hold comments and layout a writer
  would lose; a ticket of their own if wanted". This is that ticket.
- 2244 (open) moves the comment-keeping RON span parser out of
  `shuyi new` into `shuyi_assets::ron_text`, with `set_field` and
  `append_to_list`. Its Out leaves "editing a map entry by key". A
  map's `legend` and `features` are RON maps keyed by a character and
  by `(x, y)`, so this ticket adds those edits to `ron_text`.
- The engine's skill pack, `skills/shuyi-balance-bots/references/autobalance.md`,
  shows an autobalance `--apply` that writes the stage with
  `grid_map::print`, and says it "doesn't keep the file's comments".
- The game (pin `50472bbc`) wraps `grid_map::print` in its own
  `print_map` (`gh search code --repo Zafnok/visions-of-shuyi
  'grid_map::print'` finds `crates/content/src/map.rs`) and uses
  `print_scene` in round-trip tests (`crates/content/src/dialogue.rs`).
  Nothing here changes either, so the game keeps building.

Other engines: Godot and Unity keep a scene file's layout stable when
the editor saves one; Tiled and LDtk rewrite the whole file. A text-first
engine whose files are written by hand needs the first.

## Nick input

None.

## Scope

**In:**
- In `shuyi_assets::ron_text` (2244): `set_entry(text, path, key,
  value)` and `remove_entry(text, path, key)` on the RON map at `path`.
  A key matches when its text equals `key` with the white space outside
  strings and character literals removed, so `(1, 8)` matches `(1,8)`.
  `set_entry` replaces a value's span, or adds `key: value` after the
  last entry on that entry's line (the comma rules of 2244's
  `set_field`). `remove_entry` takes out the entry and the comma after
  it, and nothing else. A path with no map is an `EditError`.
- **Either order with 3417:** if 3417 is done, `ron_text::set_entry`
  and `remove_entry` are already there (3417 wrote them with this
  ticket's names and key rule): call them and skip step 2; if not, add
  them here, and 3417's `add` and `remove` at a map key call them.
  3417's contract `patch` operation and its inverse ops (undo) stay
  3417's.
- `MapText::write_back(&self, source, map) -> Result<String,
  WriteError>`, for `F: MapFeature + PartialEq + Clone`. It parses
  `source` with the format's own rules, compares it with `map`, and
  changes only what differs:
  - `name` and `look` through `set_field`, the value written by
    `ron::to_string`. A look set back to the default is written in full;
    no field is removed.
  - each `legend` and `features` entry through `set_entry` or
    `remove_entry`. A feature that moves is removed at its old place and
    added at its new one. A map with no `features` field gets one
    (`set_field`). The last feature removed leaves `features: {}`.
  - each tile row. Only a cell whose terrain changed gets a new
    character: `map.legend.char_for` of its terrain. Every other cell
    keeps the character it had, even when two characters stand for one
    terrain. A taller or wider map writes the new cells; a smaller one
    drops them. A row keeps its `\r\n` if it had one. Lines after the
    last row (the parser trims empty ones) stay as they were.
  - `WriteError` (`#[non_exhaustive]` enum, with `Display`): the source
    doesn't parse (its `ContentError`s); a terrain has no character in
    `map.legend` (the tile's `(x, y)`). `write_back` never invents a
    legend character: the caller adds one to `map.legend.names` and
    `terrains`, and `write_back` writes it as a new entry.
- `dialogue::write_scene::<D>(file, source, scene) -> Result<String,
  WriteError>` and `dialogue::remove_scene::<D>(file, source, id)`, in a
  new `crates/assets/src/dialogue/write.rs`, for `D: Directive +
  PartialEq`:
  - `write_scene` finds the scene with `scene.id` in `source`. If there
    is none, it adds `print_scene(scene)` at the end of the file, after
    one blank line.
  - Otherwise it lines up the old steps with the new ones by their
    longest common subsequence (a fixed tie rule: keep the earlier old
    step). A step that is the same keeps its source lines byte for byte.
    A removed step loses its own lines. A new or changed step is written
    with `print_step` at its list's indent, right after the own lines of
    the step before it.
  - A step's own lines are its start line, its continuation lines, and
    for a block every line to its `@endchoice` or `@endif`. Comment
    lines, blank lines and `@delivery` lines are no step's, and they
    stay.
  - A `Choice` with as many options as before, or an `If` on the same
    character, is lined up inside: each option's `* tone: text` line is
    rewritten only if its tone or text changed, and each part's steps
    are lined up as above. `@else` is added when `otherwise` stops being
    empty, and removed with its lines when it becomes empty. Any other
    changed block is written whole with `print_step`.
  - A written speech or narration line names its direction only when it
    differs from the `@delivery` in force at that place in the file.
    `print_step` itself is unchanged.
  - A changed line that was continued over several lines is written on
    one line. Its continuation lines go.
  - `remove_scene` takes out the lines from `@scene id` to its `@end`,
    and the one blank line after them if there is one. A scene that
    isn't there is a `WriteError`.
  - `WriteError` here: the source doesn't parse; no scene with that id
    (for `remove_scene`). `#[non_exhaustive]`, with `Display`.
- The step end lines come from a crate-private helper in `write.rs` (or
  a crate-private record the parser fills). `Lines` and `PartLines`
  don't change.
- Everything is `no_std` with `alloc`, like the rest of `shuyi-assets`,
  and pure: text in, text out.
- The skill pack's `autobalance.md` example writes the stage with
  `MapText::write_back`, and its note about lost comments goes.
- No API break: only new items. `print` and `print_scene` stay as they
  are. The game may move its `print_map` to `write_back` in a ticket of
  its own; no game PR has to follow this one.

**Out (do not do):**
- `Encode` for `MapText` or `Dlg`. `Encode::encode` can't fail, and
  `print` returns `None` for a tile whose terrain has no character. A
  fresh map or scene is `print` or `print_scene`, as today. 2218 writes
  `Encode` for the RON formats only.
- A tile painter, a legend palette or a map editor window: the editor
  is parked (2506, 2507).
- Reading Tiled and LDtk maps (1124, parked) or writing them.
- Keymaps and cinematics: 2244's `set_field` and `append_to_list` cover
  their RON when a tool needs it.
- New `.dlg` syntax. 2140 plans `@set` and `@if` over story variables.
  A step it adds is written back through `print_step`, which it extends,
  so this code covers it with no change.
- Bringing files up to a newer form of a format: 2517 does that at
  token spans, and it leaves `.dlg` and the map's tile rows out.
- Fuzzing the new functions in CI (2020 covers the registered formats).

**Estimated size:** about 1,600 lines (code 800, tests 650, docs 150).

**Split:** past 900 lines, split off the `.dlg` half (steps 4 and 5)
into a new ticket that keeps this ticket's `.dlg` Scope, blocked by
none.

## Implementation steps

1. Read `crates/assets/src/ron_text.rs` (2244), `grid_map.rs` with its
   tests, `dialogue.rs` (`print_scene`, `print_step`) and
   `dialogue/parse.rs` (`Parser::line`, the continuation rule, how
   `@delivery` is folded into `direction`).
2. Add `set_entry` and `remove_entry` to `ron_text`, with unit tests in
   `ron_text/tests.rs` and the same span property as 2244's edits.
3. Add `MapText::write_back` and the map's `WriteError` in
   `grid_map.rs` (or a `grid_map/write.rs` if the file passes about 700
   lines). Tests in `grid_map/tests.rs`.
4. Add `dialogue/write.rs`: the step-span helper, the step alignment,
   `write_scene`, `remove_scene` and its `WriteError`. Re-export them
   from `dialogue`.
5. Tests for `write.rs` in `dialogue/write/tests.rs`.
6. Change the snippet in `skills/shuyi-balance-bots/references/autobalance.md`
   and check that the skill pack's worked examples still pass.
7. Docs: in `docs/formats.md`, under *3. Writing back (optional)*, a
   short part on writing a hand-written file back (`write_back`,
   `write_scene`, what is kept); the module docs of `grid_map` and
   `dialogue` name the new functions beside `print` and `print_scene`.

## Acceptance criteria

- [ ] On Hello, Grid's `stage.map`, moving `Hero` from `(1, 8)` to
      `(2, 8)` through `write_back` changes only the `features` line:
      the three comment lines and all ten rows are byte for byte the
      same, and the output parses to the moved map.
- [ ] On the same file, setting tile `(2, 1)` to `wall` changes only
      the row `#.........>#`, to `#.#.......>#`.
- [ ] A map whose legend has `'.'` and `','` both for `floor` keeps
      every `,` that wasn't changed.
- [ ] A tile whose terrain has no character in the legend gives
      `WriteError`, with the tile's `(x, y)`.
- [ ] `set_entry` matches `(1,8)` to a key written `(1, 8)`, never a key
      inside a string or a comment, and adds a missing key;
      `remove_entry` takes out the last entry of a map without leaving
      a stray comma.
- [ ] On Hello, Grid's `intro.dlg`, changing the text of the second
      `guide:` line changes only that line, and the six comment lines
      stay.
- [ ] Adding a narration step after `@left guide calm` adds exactly one
      line, right after it.
- [ ] Emptying the `@if` block's `otherwise` removes the `@else` line
      and the narration under it, and keeps `@if guide`, its line and
      `@endif`.
- [ ] A changed line under `@delivery low` whose direction is `low` is
      written with no `(low)`.
- [ ] `write_scene` with a new id adds the scene at the end, after one
      blank line; `remove_scene` takes a scene out and keeps the scenes
      around it byte for byte.
- [ ] Property: `write_back(source, parse(source))` and
      `write_scene(source, scene)` for a scene of the file return
      `source` unchanged.
- [ ] Property: whatever the edit, the output parses to the asked-for
      map or scene, and nothing panics on random text.
- [ ] `shuyi-assets` builds with `--no-default-features`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each acceptance case above, on the two example files and on
  small hand-written sources; `ron_text`'s two new edits; every
  `WriteError`.
- Property: write-back of what was read is the identity; the output
  parses to what was asked; random edits of random maps and scenes
  (from the generators `grid_map/tests.rs` and `dialogue/tests.rs`
  already use) keep every comment line of the source.
- Integration: the skill pack's autobalance example compiles and
  writes a map that keeps its comments.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
