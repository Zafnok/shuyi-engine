---
id: "3417"
title: The editor, agents and the CLI edit a RON data file through one patch operation addressed by JSON Pointer, checked against the file's hash, with inverse ops for undo
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2244"]
nick_input: none
completed:
---

# 3417 — One patch operation edits a data file, with undo

## Context

Found by the blind-spot audit of 2026-11-22 (candidate 34, "no shared
edit layer", medium). Three kinds of tool will write a game's data
files: an agent, the editor's inspector (2507, parked) and `shuyi
upgrade` (2517). Today each would write in its own way. None of those
ways checks the result against the game's formats, none gives undo, and
none notices that the file changed under it.

What Shuyi has (checked 2026-11-23):

- **Paths are JSON Pointer.** ADR-0084 §5 says one value is addressed by
  an RFC 6901 pointer into its serde tree: `/hero/x`,
  `/outcome/Won/turn` (the variant is in the path), `/units/3/hp` (a
  list by index). The contract reads scene values this way:
  `session::at_pointer` (`crates/cli/src/contract/session.rs` line 103)
  behind `scene --path`, and its miss answers `SH0005` (`NO_VALUE`).
  2507's inspector and 2221's scenes use the same paths.
- **2244 (open) uses other paths.** Its `shuyi_assets::ron_text` keys
  edits by "a path of field names from the root". It has only
  `set_field` and `append_to_list`. Its Out leaves "removing an item or
  a field, or editing a map entry by key". So a list item can't be
  named by index, and nothing can be removed.
- **2273 (open)** adds `ron_text::set_entry` and `remove_entry` on a RON
  map by key, for `.map` write-back. Its Out says "an edit operation in
  the contract or the MCP server, and undo: 3417 plans one patch
  operation for data files".
- **No contract operation edits a file.** `Op` (`crates/cli/src/contract.rs`
  line 84) has 23 operations: `new`, `add` (a feature in `Cargo.toml`)
  and `autobalance --apply` write, but none edits a data file by path.
  The contract is at `VERSION` 54. Every operation is an MCP tool
  (`crates/cli/src/mcp.rs`, `crates/cli/src/mcp/host.rs`).
- **The format check exists.** `import` (`crates/cli/src/contract/import.rs`)
  reads a file as every kind whose formats know it, through
  `Importers` (the engine's standard kinds, or `GameTools::importers`),
  and gives each `ContentError` with its line. A patch can check its
  result the same way.
- **A hash exists.** `shuyi_assets::fnv::Hasher` (`crates/assets/src/fnv.rs`)
  is public, `no_std`, and the same on every platform. The pack's
  checksum and the bundle's fingerprint use it.
- **Undo is unplanned.** 2507 says "undo and redo within a session, per
  2506", and 2506's research item 5 says "the editor's own undo stack,
  and git as the record". No ticket says what an undo step is.

Other tools: RFC 6902 (JSON Patch) has `add`, `remove`, `replace`,
`move`, `copy` and `test` at pointers. Bevy's remote protocol and
Godot's editor both edit by a property path. Unity's
`SerializedProperty` edits by a path and records undo as the old value.

This ticket gives every tool one write path: pointer-addressed edits
that keep every other byte, checked by the game's formats, refused when
the file changed since the tool read it, and answered with the edits
that undo them.

## Nick input

None.

## Scope

**In:**
- **Pointers in `ron_text`.** `ron_text::Pointer::parse(&str) ->
  Result<Pointer, EditError>` (RFC 6901: `""` is the root, `~1` is `/`,
  `~0` is `~`). `Node::at_pointer(&Pointer) -> Option<&Node>`. Each
  segment is read this way:
  - in a struct, named or not (`( a: 1 )`), the field of that name;
  - in a list `[ ]` or a tuple `( 1, 2 )`, the item at that index
    (digits only, no leading zero);
  - in a map `{ }`, the entry whose key matches. A key matches when its
    text, with a string's or a character's quotes taken off and the
    white space outside strings removed, equals the segment. So `'#'`
    is `/legend/#` and `(1, 8)` is `/features/(1,8)`. This is 2273's key
    rule.
  - a named value with fields (`Won(turn: 3)`): a segment equal to its
    name selects it, and the next segment reads its fields, as ADR-0084
    §5's `/outcome/Won/turn`. Any other segment reads its fields
    directly, as a struct named `Hero(x: 1)` reads `/x`. The module doc
    says this, and that a field named like its own struct is reached
    through the name.
  - `Some(v)` is looked through: the segment applies to `v`, as serde's
    tree has no variant for an `Option`.
- **Edits in `ron_text`.** `ron_text::Edit`, a `#[non_exhaustive]` enum
  (ADR-0039), with `Edit::replace(path, value)`, `Edit::add(path,
  value)`, `Edit::remove(path)` and `Edit::move_to(from, path)`. A
  `value` is RON text, one value; it is parsed before anything changes.
  - `replace`: the value's span becomes `value`. The root may be
    replaced.
  - `add`: at a struct field, the field is set, or added after the last
    field as 2244's `set_field` does. At a list index, the item is put
    before that index on a line of its own, at the indentation of its
    neighbours; `-` adds after the last item, as 2244's
    `append_to_list`. At a map key, the entry is set or added, as 2273's
    `set_entry`.
  - `remove`: the field, item or entry goes, with its comma. When it
    sat alone on its lines, those lines go too. Comment lines before it
    stay. The root can't be removed.
  - `move_to`: `remove` at `from`, then `add` of the same text at
    `path`, read after the removal (RFC 6902's rule). `from` may not be
    a prefix of `path`.
  - New lines use the file's line ending (`\r\n` if its first line
    ends so).
- `ron_text::apply(text, edits: &[Edit]) -> Result<Applied,
  EditError>`. The edits apply in order, each to the text the one
  before it gave. `Applied { text, inverse: Vec<Edit> }`, built only by
  `apply`, `#[non_exhaustive]`. If one edit fails, the result is the
  error and its index; no edit is kept.
- **The inverse.** Each edit's inverse is computed against the text it
  gave: `replace` gives a `replace` with the old value's text; `add` of
  a new field, item or entry gives a `remove` at the path it landed at;
  `add` over an existing field or entry gives a `replace`; `remove`
  gives an `add` of the removed text at the same path; `move_to` gives a
  `move_to` back. `inverse` lists them in reverse order, so
  `apply(applied.text, &applied.inverse)` undoes the whole list.
- `EditError` (2244's `#[non_exhaustive]` enum) gains, additively: the
  pointer doesn't parse; it names nothing (with the deepest node it
  reached and that node's field names, keys or length, like
  `at_pointer`'s message); an edit's kind doesn't fit the node (an index
  into a struct); the value doesn't parse; the edit at index N failed.
- **One contract operation, `patch`** (`crates/cli/src/contract/patch.rs`,
  `Op::Patch`, name `patch`): `shuyi patch <file> --ops <json or
  @file.json> [--base <hash>] [--dry-run] [--print] [--json]`.
  - `file` is from the project's root; a path outside it is refused
    (`SH0001`).
  - The ops are a JSON list of RFC 6902 objects with RON values:
    `{"op": "replace", "path": "/hero/x", "value": "3"}`, `add`,
    `remove`, and `move` with `from`. `copy` and `test` are refused by
    name (`SH0001`).
  - `base` is the hash of the bytes the tool read: the 64-bit
    `fnv::Hasher::sized` of the file, written as 16 lowercase hex
    digits. A file whose hash is not `base` is refused with nothing
    written, under a new problem code (the next free `SH3` code;
    `SH3018` today) that says to read the file again and redo the
    edit. With no `--base` it applies to the file as it is, and the
    answer has a limit line saying no base was checked.
  - After the edits, the new text is read as every kind that read the
    old text, through the same `Importers` `import` uses. A kind that
    read it before and doesn't now refuses the patch, with each
    `ContentError` and its line. A file no kind reads is only checked
    to parse as RON, with a limit line saying so.
  - The file is written only if every check passes and `--dry-run` is
    not given. It is written to a temporary file beside it, its hash is
    checked against `base` once more, and the temporary file is renamed
    over it.
  - An empty op list writes nothing and answers the file's hash: a
    tool's way to get its `base`.
  - The answer, `PatchOutput` (`#[non_exhaustive]`): `file`; `base`
    (the hash before); `hash` (after); `changed`; `written`; `kinds`
    (each kind that read it); `inverse` (the ops that undo it, in the
    same JSON shape as the input, so they are a patch of their own with
    `base` set to `hash`); and `text` with `--print` or `--dry-run`.
  - It needs the game's kinds, so `runs_the_game` lists it and
    `with_game` runs it, as `import`. Without a game tool program it
    uses `Importers::standard()` with `import`'s limit line.
  - The MCP tool `patch` takes the same input; `ops` is a JSON list.
  - `patch.input.json`, `patch.output.json` and the fingerprint in
    `schemas/versions.json`. A new operation breaks no client, so the
    contract's `VERSION` stays (ADR-0150).
- **`.map` and `.dlg` files.** Their text isn't one RON value, so a
  patch to one edits the value its format reads, not the text. The ops'
  pointers address that value's serde tree (ADR-0084 §5): a `GridMap`
  (`/name`, `/tiles/...`, `/features/...`) or a `DialogueFile`
  (`/scenes/0/...`). The file is read through its kind, the edits
  apply to the value, and the result is written through 2273's
  `MapText::write_back` and `dialogue::write_scene` (and `remove_scene`
  for a scene that goes), so the file keeps its comments. The inverse
  ops are computed on the value the same way. The kinds hook for this
  sits beside `ImportKind::read` in `contract/import.rs`, for the
  engine's map and dialogue kinds and a game's (`GameTools::importers`).
- **Either order with 2273:** if 2273 is done, `add` and `remove` at a
  map key call its `ron_text::set_entry` and `remove_entry`, and a
  patch to a `.map` or `.dlg` file is written through its
  `MapText::write_back` and `dialogue::write_scene`, so the file keeps
  its comments; if not, write `set_entry` and `remove_entry` here as
  2273's first Scope bullet gives them (same names, same key rule), and
  2273 then calls them, and leave `.map` and `.dlg` out of the patch
  operation (a `.map` file is refused as not RON, `SH3004`); 2273 does
  not add them, and a later ticket does.
- `shuyi-assets` stays `no_std` with `alloc`; the edits are pure, text
  in and text out. The contract part does the file reading, hashing and
  writing.
- No API break: only new items, and an additive variant on a
  `#[non_exhaustive]` enum. No game PR has to follow this one.

**Out (do not do):**
- A `.map` or `.dlg` patch when 2273 isn't done (see the either-order
  bullet).
- The editor's undo stack, watching the files it has open and
  reloading on an outside change: 2507 (parked) and 2506's research.
  This ticket gives them the inverse ops and the stale-base refusal.
- Picking and dragging in the live view, whose drags become patches:
  3416.
- Game panels and inspector hints: 3418.
- Checking a value against 2515's JSON schema. The format's own decode
  is the stronger check; 2515 gives tools the shape to write ops from.
- Format upgrades. 2517's `shuyi upgrade` keeps its own rewrite at
  token spans.
- Editing a running game's state. A patch edits a file; the game sees
  it by hot reload (2220).
- `copy` and `test` ops, and JSON values converted to RON. A value is
  RON text.
- Several files in one patch.

**Estimated size:** about 1,900 lines (code 900, tests 850, docs 150).

**Split:** past 900 lines, split off the contract operation (steps 5–7)
into a new ticket blocked by this one; this ticket then ships the
pointers, the edits and the inverse in `ron_text`.

## Implementation steps

1. Read `crates/assets/src/ron_text.rs` and its tests (2244),
   ADR-0084 §5, `session::at_pointer`, `contract/import.rs` and how
   `mcp/host.rs` runs `Op::Import`. If 2273 is done, read its
   `set_entry` and `remove_entry`.
2. Add `Pointer`, `Node::at_pointer` and the new `EditError` variants
   to `ron_text`. Unit tests for each segment rule.
3. Add `Edit`, `Applied` and `apply`, with each edit's inverse. Reuse
   2244's insertion code for `add`. (If 2273 isn't done, add
   `set_entry` and `remove_entry` here first.)
4. Tests in `crates/assets/src/ron_text/tests.rs`: the cases below, and
   the properties.
5. Add `contract/patch.rs`: the input and output types, argument
   parsing, the hash, the format check through `Importers`, and the
   atomic write. Add `Op::Patch` everywhere `Op::Import` is listed
   (`contract.rs`: the list, the name, the schema pair; `cli.rs`:
   `runs_the_game` and `with_game`; `mcp.rs` and `mcp/host.rs`).
   If 2273 is done, add the `.map` and `.dlg` path: edits on the
   value's serde tree, written through `MapText::write_back` and
   `dialogue::write_scene`.
6. Add the new problem code in `contract/codes.rs`, with its page for
   `explain`.
7. Bump `VERSION`, write the schemas (`shuyi schema --out
   crates/cli/schemas`) and their fingerprint.
8. Docs: a `patch` row in `docs/contract.md`'s operations table; in
   `docs/formats.md`, the section 2244 wrote on editing a RON file gets
   the pointer rules, the four edits and the inverse; the book's
   `tools-for-agents.md` shows an agent's patch with `base` and its
   undo; the skill pack's `shuyi-test` skill names `patch` as the way an
   agent changes a data file, and `check-skills` passes.

## Acceptance criteria

- [ ] On a RON file with comments between fields, `replace` at
      `/hero/x` changes only that number's bytes.
- [ ] `add` at `/levels/1` on a three-item list puts the new item
      second, on its own line, at the others' indentation; `add` at
      `/levels/-` equals 2244's `append_to_list` on the same text.
- [ ] `remove` at `/levels/2` takes that item's line and comma and
      keeps the comment line above it; `remove` of the last item of a
      one-line list `[a, b]` gives `[a]`.
- [ ] A map entry keyed `(1, 8)` is reached by `/features/(1,8)`, and a
      character key `'#'` by `/legend/#`.
- [ ] `/outcome/Won/turn` reaches `turn` in `outcome: Won(turn: 3)`;
      `/x` reaches `x` in `hero: Hero(x: 1)`; `/hp` reaches `5` in
      `hp: Some(5)`.
- [ ] A pointer that names nothing is an `EditError` listing what the
      deepest node it reached holds; an index into a struct is an
      `EditError`; no edit is kept when the third of three fails.
- [ ] `move` from `/a/0` to `/a/-` on `[x, y, z]` gives `[y, z, x]`;
      `move` from `/a` to `/a/b` is refused.
- [ ] A file with `\r\n` line endings keeps them on every line `add`
      writes.
- [ ] `shuyi patch` with a stale `--base` answers the new code, and the
      file's bytes don't change.
- [ ] `shuyi patch` whose result a kind that read the file can't read
      (a palette's color made `"blue"`) is refused with that kind's
      `ContentError` and its line, and nothing is written.
- [ ] If 2273 is done: a `shuyi patch` that renames a map (`/name`)
      changes only the `name` line of a `.map` file with comments, and
      one that edits a scene's step keeps the `.dlg` file's comment
      lines.
- [ ] `shuyi patch --dry-run` and `--print` give the new text and write
      nothing; an empty op list answers the file's hash.
- [ ] The answer's `inverse`, sent back as a patch with `base` set to
      the answer's `hash`, gives a file that reads as the same value as
      before; for a patch of `replace` ops only, the same bytes.
- [ ] The MCP tool `patch` gives the same answer as the CLI's `--json`
      on the same input.
- [ ] `shuyi-assets` builds with `--no-default-features`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each pointer rule; each edit on structs, lists, tuples, maps and
  named values; each `EditError`; line endings; the contract op's
  stale base, format refusal, dry run, empty op list and a path outside
  the project.
- Property: outside each edit's span the output equals the input; the
  output parses; `apply(apply(t, e).text, inverse)` reads as the same
  value as `t`, and equals `t` byte for byte when `e` holds only
  `replace` edits; no edit panics on random text or random pointers.
- Snapshot / integration: `shuyi patch --json` on a fixture project,
  through the CLI and through the MCP host, with its schemas checked by
  the contract's schema test.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
