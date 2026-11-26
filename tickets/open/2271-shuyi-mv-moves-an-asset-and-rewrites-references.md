---
id: "2271"
title: "shuyi mv moves an asset with its settings and target copies, and rewrites every reference to it"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2516"]
nick_input: none
completed:
---

# 2271 — shuyi mv moves an asset and rewrites every reference to it

## Context

Found by the engine blind-spot audit of 2026-11-23 (assets area,
"Renames break references, and no move tool exists").

An asset's id is its bundle path. 2516 keeps it so: its ADR says "ids
stay bundle paths ... a renamed file breaks its references, and the
check finds them", and its **Out** list has "ids that survive a
rename". So after 2516, a rename is caught. Nothing fixes it. Moving a
file today means:

- renaming the file in the game's files folder (`Config::assets` in
  `crates/cli/src/project.rs`, `assets` unless `shuyi.ron` says);
- renaming its `<file>.meta` settings file beside it (2229, done;
  `shuyi_assets::import_settings::settings_of`), and checking that the
  `_folder.meta` files above the new place give the same settings
  (`settings_chain`);
- renaming its copy in each target layer, `assets@<target>/` (3007,
  done; `bundle::target_layer_name`);
- finding and editing every RON, JSON or CSV value that names it, and
  every Rust string literal (`ImageId` is a `&'static str`,
  `crates/assets/src/image.rs` line 233; `Formats::load` takes a
  `&str`).

That is a find-and-replace across many files. For an agent it is a
risky edit, and a bare replace also hits comments and longer names.

Other engines: Godot added `uid://` ids and `.uid` files in 4.4 because
renames broke projects. Unity keeps a GUID in each `.meta`. Unreal
leaves redirectors. Shuyi keeps plain path ids, which agents read and
write easily, and gives them a move command instead.

What 2516 gives this ticket:

- `refs::collecting` records each `Ref<K>` read while a file decodes,
  as a `Reference` with its kind, id, file and line, in read order;
- its line search finds the id as a whole word in the file's text
  (neither neighbour a letter, a digit, `_`, `-`, `.` or `/`);
- `GameTools::assets` hands over the game's `Bundle` and `Formats`;
- each reference can be resolved with its `resolve` function.

So 2516 already knows every place a file is named. This ticket rewrites
those places.

What exists to build on:

- contract ops that run the game: `contract::cli::runs_the_game` and
  `with_game` (`crates/cli/src/contract/cli.rs` lines 301 and 318);
- `Op` (`crates/cli/src/contract.rs` line 84), `contract::VERSION`
  (54 on 2026-11-23), and the schemas in `crates/cli/schemas/`;
- `folder_bundle` (`crates/cli/src/contract/import.rs` line 276), which
  reads a folder from disk into a `Bundle`;
- `Layer::owned` (`crates/assets/src/bundle.rs` line 402);
- the MCP server makes a tool of each `Op`: a new op needs its arm in
  `mcp::host::one_off` (`crates/cli/src/mcp/host.rs`) and in
  `mcp::description` (`crates/cli/src/mcp.rs`).

## Nick input

None.

## Scope

**In:**
- **`shuyi mv <from> <to> [--dry-run] [--json]`**, a contract op
  (`crates/cli/src/contract/mv.rs`, `Op::Mv`, name `mv`). It runs the
  game (`runs_the_game` lists it), since it needs the game's references.
  `from` and `to` are bundle paths (as `Formats::load` takes them).
  - `from` may be a file or a folder. A folder moves every file under
    it, its `_folder.meta` files included, each as a move of its own.
  - It refuses, with nothing written: `from` is in no layer; `to`
    already exists in any layer; `from` is in a private layer (bought
    art lives in another repository, `PrivateAssets`); `from` or `to`
    is outside the files folder.
- **What moves, on disk:**
  - the file in the game's files folder;
  - its `<file>.meta`, if any;
  - its copy at the same path in each `assets@<target>/` folder beside
    the files folder;
  - nothing in a test layer is moved unless `from` itself is there.
  - If `settings_chain` gives a different list of existing
    `_folder.meta` files at the new place, the answer warns: the
    file's folder settings change with the move.
- **What is rewritten:**
  - The op runs the game's `check` inside `refs::collecting`, as 2516's
    `shuyi check` does, and keeps each `Reference` whose id names a
    moved file (with or without its extension, as `Ref<K>` allows).
  - Each such reference's id is replaced in place in its file. The
    byte span comes from 2516's whole-word search: add
    `span: Option<Range<usize>>` to `Reference`, filled by the same
    search that gives `line`, so the two never disagree. Only those
    bytes change; comments, layout and line endings stay byte for byte.
  - An id written without its extension is written without it again,
    unless the move changes the extension.
  - A reference with no file (read outside `Formats`, 2516) or no span
    (bytes that aren't UTF-8) is not edited (see the check below).
- **Left by hand, never edited:** each whole-word mention of a moved
  id that is not a collected reference:
  - in a Rust string literal under the project's root (skipping
    `target/` and folders whose name starts with `.`);
  - in any UTF-8 file of the files folder or a target folder (a `.dlg`,
    the grid of a `.map`, a file read outside the bundle);
  - in `shuyi.ron`.
  Each is a warning with its file, line and column.
- **Checked before anything is kept:**
  - After the edits, the op builds a bundle from disk: the game's
    layers, with each edited folder's layer replaced by
    `Layer::owned` of that folder as it is now (reuse `folder_bundle`,
    made `pub(super)`). The old path is not in it.
  - It runs the game's `check` again over that bundle, inside
    `collecting`, and resolves every reference.
  - The move is kept only if: no collected reference names an old id;
    every reference that resolved before
    still resolves; and each edited file has the same references as
    before, with only the moved ids changed.
  - Otherwise every file is put back as it was (moves undone, edited
    bytes restored from memory), and the answer is an error naming
    the first file that failed.
  - A mention that 2516's search took for the reference (an id in a
    comment before the real value) fails this check, so the whole move
    is undone. The error names that file and line, so the author can
    change the comment and run the move again.
  - A reference that is not edited (no file, no span) dangles after
    the move: it fails the check too, and the error names it. So a
    kept move never leaves a collected reference dangling.
- **`--dry-run`** does every step on copies in memory and writes
  nothing. Its answer is the same, with `written: false`.
- **The answer:** `MvInput { from, to, dry_run }`; `MvOutput { moved:
  Vec<Moved>, rewritten: Vec<Rewritten>, left: Vec<Problem>, written:
  bool }`; `Moved { from, to }` (paths from the project's root);
  `Rewritten { file, line, column, old, new }`. `ok` is true when the
  move was made (or would be, with `--dry-run`) and checked. The
  warnings in `left` don't fail it. The text answer has a line per
  move and per rewrite, then `by hand:` before each warning, then
  "run cargo shuyi check" (the tool program rebuilds its embedded
  files on its next run).
- **Problem codes (2509, done):** the refusals and the by-hand warning
  get rows in `crates/cli/src/contract/codes.rs`'s `TABLE` (content
  codes `SH2nnn`, the next free; names `move-refused` and
  `move-by-hand`), with their pages in `crates/cli/errors/`.
- **The contract:** the schemas `mv.input.json` and `mv.output.json`
  written, and their line in `crates/cli/schemas/versions.json`
  (`docs/contract.md`, *The schemas*). A new operation and an added
  field break no client, so `contract::VERSION` stays (ADR-0150).
  The MCP arms in `one_off` and `description`.
- **Public types (ADR-0039):** `MvInput` (an input) and the new output
  types are `#[non_exhaustive]` with constructors. The new `span`
  field of 2516's `Reference` is additive if `Reference` is
  `#[non_exhaustive]`; if 2516 made it exhaustive, write a `Breaking:`
  line (no released game builds one: 2516 is new).
- **Docs:**
  - `docs/formats.md`, its *References* section (2516): moving a file
    with `shuyi mv`, what it rewrites and what it leaves;
  - `docs/contract.md`: the op's row;
  - the usage in `crates/cli/src/lib.rs` and the op table in
    `crates/cli/src/contract.rs`'s module docs;
  - each template's `AGENTS.md` table of tools gets a row for
    `cargo shuyi mv`. Templates are outside `cargo fmt --all`: format
    them as CI's template job checks them.
- **For the game:** nothing changes. The op and its types are new, and
  `Op` is `#[non_exhaustive]` (2086, done). The game's next pin bump
  needs no change.

**Out (do not do):**
- Ids that survive a rename (GUIDs, `.uid` files, redirectors): ids stay
  bundle paths (2516's ADR).
- Editing Rust code. A string literal is reported, never rewritten.
- References inside `.dlg` text and the grid of a `.map`: 2516 doesn't
  collect them, so they are reported as mentions. A follow-up when a
  format collects them.
- Files in a private layer (another repository) and packs: bake again
  (ADR-0023 §5).
- Saves and records that hold an id: a game that stores asset ids
  handles a move in its save migrations (2110).
- `git mv`: the op renames files on disk; git finds the rename itself.
- References to an entry inside a table (2516's **Out**).
- An undo after the op ends: version control does that.

**Estimated size:** about 1,200 lines (code 600, tests 480, docs 120),
the schemas and snapshots not counted.

**Split:** past 900 lines, split off folder moves and the by-hand
mention scan (Rust literals, other text files, `shuyi.ron`) into a new
ticket, After MVP, blocked by this one. The first PR moves one file and
rewrites the collected references.

## Implementation steps

1. Read 2516's ADR and `crates/assets/src/refs.rs`,
   `crates/assets/src/import_settings.rs` (`settings_of`,
   `settings_chain`), `crates/assets/src/bundle.rs` (`Layer::owned`,
   `target_layer_name`, `layer_of`), `crates/cli/src/contract/check.rs`,
   `contract/import.rs` (`folder_bundle`), `contract/cli.rs` and
   `crates/cli/src/project.rs` (`Config::assets`, `PrivateAssets`).
2. Add `span` to `Reference`, filled by 2516's search, with a test that
   the span's text is the id and its line is `line`.
3. Write the plan: from the bundle and the project, the list of moves
   (file, `.meta`, target copies) and the refusals.
4. Collect the references under `collecting`, pick those of moved ids,
   and build the edits per file, ordered by span, from the end of each
   file to its start.
5. Apply the moves and edits (in memory for `--dry-run`), build the disk
   bundle, run the check again and compare. Undo on failure.
6. Write the mention scan.
7. Add the op to `Op`, `Op::ALL`, `runs_the_game`, `with_game`, the MCP
   arms, the usage and the schemas; add the problem codes.
8. Write the docs and the templates' rows, and run the gates, the
   templates job's commands included.

## Acceptance criteria

- [ ] In a scratch project whose table holds two `Ref<PaletteDef>` to
      `palettes/day.ron`, one with the extension and one without,
      `shuyi mv palettes/day.ron colours/day.ron` moves the file and
      its `.meta`, rewrites both values (the second still without its
      extension), and leaves every other byte of the table unchanged,
      comments included.
- [ ] A copy at `assets@playdate/palettes/day.ron` moves to
      `assets@playdate/colours/day.ron`.
- [ ] `shuyi check` passes after the move, and no reference names
      `palettes/day.ron`.
- [ ] A comment holding `palettes/day.ron` on the line before the value
      fails the move: every file is as it was, byte for byte, and the
      error names the table's file and the comment's line.
- [ ] A Rust file holding `"palettes/day.ron"` is listed by hand with its
      line and column, and is not edited.
- [ ] `--dry-run` writes nothing, and its answer lists the same moves
      and rewrites as the real run.
- [ ] Moving to a path that exists, from a path that doesn't, or from a
      private layer is refused with its code, and no file changes.
- [ ] A move whose check fails leaves every file byte for byte as it was
      (a test makes the second check fail).
- [ ] Moving a folder moves every file under it and its `_folder.meta`,
      and rewrites the references to each.
- [ ] Moving a file under a different `_folder.meta` warns.
- [ ] `palettes/day.ron.old` and `palettes/day.rons` are not rewritten
      when `palettes/day.ron` moves.
- [ ] The contract's schema test passes with the schemas' line in
      `versions.json`, and `cargo xtask contract` passes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, `shuyi-assets`: `Reference::span` covers the id's bytes and
  agrees with `line`, for two references to one id and for an id that
  is part of a longer word.
- Unit, `shuyi-cli`: the plan (file, `.meta`, target copies, refusals);
  edits applied from the end keep earlier spans valid; the extension
  rule; the undo restores every byte.
- Property: for a random table of `Ref` ids, moving one id and moving
  it back gives the original bytes.
- Integration (`crates/cli/src/contract/mv/tests.rs`): a scratch project
  under `std::env::temp_dir()` and a fake game (`contract/fake.rs`) with
  `GameTools::assets`, run through the op end to end, with and without
  `--dry-run`, and then `check`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
