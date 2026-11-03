---
id: "2244"
title: "A public comment-keeping RON editor: shuyi new's span parser moved into shuyi-assets, with append-to-a-list and set-a-field edits"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2244 — A public comment-keeping RON editor

## Context

A request from the game.

1. **The game ticket:** game#1220, tuning files edited with the engine's
   RON editor
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1220-tuning-files-edited-with-the-engines-ron-editor.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** a tool changes one value in a
   hand-written RON file, or adds an item to one of its lists, and every
   other byte (comments, layout, blank lines) stays as the author wrote it.
3. **The friction numbers:** game row 0509 (2026-10-25): estimate 290,
   measured 652, 2.2×, accepted, because about 230 of the lines were a
   private RON scanner writing a changed list back into a battle file,
   with the rule "a second game file written back this way would make it
   an engine (assets) piece". Game 0510 (#520) then wrote two more kinds
   of file back with it (a world map's `nodes` and `roads`, a pool file's
   `pool`). The game/engine boundary audit of 2026-11-03 found the rule
   met and the game's scanner (about 140 production lines) doing what
   this engine's own `crates/cli/src/template/ron_merge.rs` (2501) does,
   privately to `shuyi new`.
4. **The generic shape:** `shuyi_assets::ron_text`, `no_std` with
   `alloc`: the span tree of a RON text (where each value, container,
   field name and item starts and ends; never rewritten), and two edits
   on it, keyed by a path of field names from the root.
5. **Examples from games that aren't tactics:** a platformer's level
   tool appending a new level to `levels.ron`'s `levels` list, and a
   farming game's price tuner setting `price` in one crop's entry, both
   keeping the designer's comments. In this engine, `shuyi new`'s
   composer and 2517's `shuyi upgrade` read the same tree.

## Nick input

None.

## Scope

**In:**
- Move `ron_merge.rs`'s `Delim`, `Node`, `Container`, `Item`, `Parser`,
  `parse` and `indent_at` into `crates/assets/src/ron_text.rs`
  (`pub mod ron_text` in `lib.rs`). Public read-only access by methods:
  `Node::span()`, `Node::field(name) -> Option<&Node>` (in a struct),
  `Node::at(path: &[&str]) -> Option<&Node>`, `Node::items() -> &[Item]`
  (a container's), `Item::span()`, `Item::key()`. `ParseError` carries
  the byte offset and a message. The types are `#[non_exhaustive]`
  (ADR-0039), built only by `parse`.
- `ron_merge.rs` keeps `merge`, `Merge`, `normalized`, `shown`,
  `reindent` and `leading`, and calls `ron_text`; its tests in
  `template/ron_merge/tests.rs` pass unchanged.
- `ron_text::append_to_list(text, path, item) -> Result<String,
  EditError>`: `item` added as the last item of the list at `path`, on
  its own line at the indentation of the list's last item (of the
  list's line plus four spaces when it is empty), with the comma the
  list needs before it and a trailing comma after it.
- `ron_text::set_field(text, path, name, value) -> Result<String,
  EditError>`: in the struct at `path` (`&[]` for the root), field
  `name`'s value span replaced by `value`, or, when it has none,
  `name: value` added after its last field on that field's line
  (`(a: 1, name: value)`; a trailing comma stays after the new field).
- `EditError`: the text doesn't parse, or `path` names no list / no
  struct (`#[non_exhaustive]` enum, with `Display`).
- The module doc says what is kept: outside the edited span, the output
  equals the input byte for byte.
- **Either order with 2517:** if 2517 is done, the parser is already a
  public module of `shuyi-assets` beside `format::changes`: add the
  accessors and the two edits to it, rename nothing it relies on, and
  don't move it again; if not, move it here, and 2517's `find` walks this
  module's tree.
- No API break: `ron_merge` is private. The game's PR that moves the pin
  follows this one (game#1220).

**Out (do not do):**
- Removing an item or a field, or editing a map entry by key: no game
  needs it yet.
- Writing RON through `Encode`/`PrettyConfig` (2218), and 2517's
  `Format::changes`, `find`, `apply` and `shuyi upgrade`.
- The game's use of it (game#1220).

## Implementation steps

1. Read `crates/cli/src/template/ron_merge.rs` and its tests, 2501 in
   `tickets/done/`, and 2517's *One RON span parser* bullet.
2. Move the parser (step one of Scope), keep `ron_merge`'s tests green,
   and check `cargo build -p shuyi-assets --no-default-features`.
3. Add the accessors, `append_to_list`, `set_field` and `EditError`,
   with unit and property tests in `crates/assets/src/ron_text/tests.rs`.
4. Docs: a short section in `docs/formats.md` (*Editing a RON file
   keeping its comments*) and the module's doc example.
5. Expected size: about 400 lines moved, about 120 new production lines
   (accessors, two edits, error), about 150 lines of tests.

## Acceptance criteria

- [ ] `ron_merge`'s tests pass unchanged; `shuyi-cli` has no RON parser
      of its own (`grep -n "struct Parser" crates/cli/src/template`
      finds nothing).
- [ ] `append_to_list` on a list with comments between and after its
      items, on an empty list and on a one-line list gives the expected
      text (unit tests); a path naming no list is an `EditError`.
- [ ] `set_field` replaces a value whatever its spacing, never matches a
      name inside a string or a comment, and adds a missing field.
- [ ] Property: outside the edited span the output equals the input; the
      output parses; neither edit panics on random text.
- [ ] `shuyi-assets` builds with `--no-default-features` (no `std`).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the accessors and both edits, as above; `ron_merge` unchanged.
- Property: span preservation, the output parses, no panic.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
