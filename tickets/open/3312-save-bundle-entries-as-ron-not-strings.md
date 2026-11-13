---
id: "3312"
title: "A save bundle holds each RON entry as RON, laid out over lines, not as one quoted string"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3312 — A save bundle holds each RON entry as RON, not as a quoted string

## Context

No game ticket: Nick asked directly (2026-11-12) after opening a file
the game exported (game#0872, through `write_bundle`'s `SaveBundle`). No
friction review: a format fix.

**The job in one sentence:** a player who opens an exported save file
should be able to read it, and to change a number in it, as they could
any other RON file.

**What is wrong today** (2138, `crates/ui/src/storage/bundle.rs`):
`SaveBundle::entries` is a `BTreeMap<String, String>` and is serialized
as it is. A game's storage values are usually RON already (a game writes
its saves with `ron::to_string`), so the file is RON holding RON as
strings: each save is one line with every `"` escaped:

```ron
    entries: {
        "slot_07": "(version:2,campaign:(mode:Classic,lead:(name:\"Peter\",gender:Male),chapter:\"ch04\", …",
    },
```

Holding the values as text was right: `apply` writes a value back as the
game stored it, and each entry's own `FormatCheck` reads its version.
That stays. Only the file's text changes.

**The generic shape:** a save bundle's text holds each entry that is one
RON value as that value, laid out over lines with four spaces to a
level, and each entry that isn't as a string. Reading it back gives each
entry's text again.

**An example from a game that isn't tactics:** a farming game exports a
farm (`garden: (rows:6, crops:[…])`) so a player can move it to another
computer or fix a stuck save; the player opens the file in a text editor
and finds the field to change.

**Version 1 files** (Nick, 2026-11-12): this build need not read them.
Only Nick has exported one so far. What matters is that the saves already
in a player's storage (a browser's `localStorage` too, 3033) export
cleanly in the new format. Storage values don't change, so they do.

## Nick input

None.

## Scope

**In:**
- `SAVE_BUNDLE_VERSION` becomes 2. A version 1 file is refused as any
  other version is (`BundleError::Version`, read from the header first,
  as now). No reader for version 1.
- The text of version 2 (`SaveBundle::to_text`, `read_bundle`):
  - `entries`: the entries whose value is **exactly one RON value**
    (`ron::value::RawValue::from_ron` accepts it), each written as RON,
    not as a string, laid out over lines with four spaces to a level.
  - `texts`: every other entry (empty text, plain words, two values),
    as a string. Written only when not empty (`skip_serializing_if`), read
    as empty when absent (`#[serde(default)]`). A key is in one of the
    two, never both; a file naming one key in both is `NotABundle`.
- Reading back: each `entries` value is **compacted**, meaning whitespace
  and comments outside strings are dropped, so a value a game wrote with
  `ron::to_string` (no whitespace) comes back **byte for byte**. A value
  a player edited by hand, with comments or spaces, comes back as the
  same RON value. `texts` values come back as they are.
- In memory nothing changes: `SaveBundle::entries` stays a
  `BTreeMap<String, String>` holding each value as the storage held it.
  `SaveBundle::new`, `to_text`, `read_bundle`, `write_bundle`,
  `plan_import` and `apply` keep their signatures. **The public API
  doesn't break.** The file format does: say so in the PR and in the
  completion notes (`Breaking:` line for the format, ADR-0038 §6).
- `SaveBundle` stays `#[non_exhaustive]`. `texts` is not a public field:
  the split is the text's alone (a private `serde` shape, for example a
  `BundleText` with `entries: BTreeMap<String, Box<RawValue>>` and
  `texts: BTreeMap<String, String>`, converted to and from `SaveBundle`).
- **The layout.** If ron's pretty serializer writes a `RawValue` as it is
  (check this first), the bundle lays the value out itself: a small
  pass over the RON text that knows strings (with escapes), raw strings
  and comments, opens a line after `(`, `[` and `{` and after each `,`,
  closes one before `)`, `]` and `}`, puts `: ` after a field name, and
  keeps a container with nothing inside (`()`, `[]`, `{}`) on one line. Writing the
  same bundle twice gives the same text (as now), and laying out text
  already laid out changes nothing. If ron already lays it out, use ron's.
- `docs/formats.md`'s row and paragraph, the golden row and its fixture
  (`crates/ui/tests/fixtures/bundle_v1.ron` → `bundle_v2.ron`,
  `crates/ui/tests/golden/bundle.ron`), the example in `SaveBundle`'s doc
  comment, the book's *Bundles* section
  (`site/content/learn/book/saves-and-settings.md`) where it shows the
  text, and `skills/shuyi-screen/SKILL.md` if it shows the text.

**For the game** (searched at the game's pin `234de697`: `SaveBundle`,
`read_bundle`, `entries` in `crates/`): the game uses only
`SaveBundle::new(…).to_text()`, `read_bundle` and `bundle.entries`, in
`crates/ui/src/save_files.rs`, `crates/ui/src/save_files/import/tests.rs`,
`crates/ui/tests/it/save_import.rs` and `crates/ui/tests/it/save_files.rs`,
and has no bundle text written out in a test. So the game changes nothing
but its pin. That move is the watcher's `engine-pin` issue. Game ADR-0039
(*Consequences*, "Saves outside the game") says "RON, each entry under its
key", which stays true. A version 1 file a player kept is refused by the
game's Import with its "another version" message, which Nick accepted
above.

**Out (do not do):**
- Reading version 1 files (Nick, above).
- Changing how a game stores its values (compact or not): the game's
  choice, and storage is untouched here.
- Migrating an entry's own version: 2110.
- A `RawValue` or a RON layout pass in any public API. Both stay private
  to `bundle.rs` (or a private module beside it).

## Implementation steps

1. In a scratch test, check what ron's `to_string_pretty` does with a
   `Box<RawValue>` inside a map (ron 0.12). The ticket's writer read
   ron 0.12.2's serializer, which writes a raw value's text as it is
   (`RAW_VALUE_TOKEN`). If so, the layout pass takes the indent the
   entry sits at (two levels inside `SaveBundle(` and `entries: {`), so
   the value's inner lines line up under its key.
2. `crates/ui/src/storage/bundle.rs`: `SAVE_BUNDLE_VERSION = 2`. A
   private `BundleText` (`version`, `made`, `entries:
   BTreeMap<String, Box<RawValue>>`, `texts: BTreeMap<String, String>`,
   `deny_unknown_fields`, serialized with the name `SaveBundle` so the
   file still starts `SaveBundle(`). `to_text` splits the entries
   (`RawValue::from_ron` per value), lays each RON value out if step 1
   says so, and writes it pretty as now. `read_bundle` reads the header,
   then a `BundleText`, refuses a key in both maps, compacts each RON
   entry, and joins them into `SaveBundle::entries`. Keep the `BadKey`
   check over both.
3. The layout and compact passes, private, with their own unit tests.
   They are the only code here that walks RON text, so their string and
   comment handling must be exact.
4. The fixture, golden row, docs, book and skill (Scope).

## Acceptance criteria

- [ ] A bundle of `"slot": "(version:2,campaign:(mode:Classic,lead:(name:\"Peter\")),point:WorldMap)"`
      writes the slot as RON over lines, with no `\"` in the file, and
      reads back to exactly that string.
- [ ] Values that aren't one RON value (`""`, `"two words"`, `"(unclosed"`)
      are written under `texts` and read back byte for byte. A bundle
      with none writes no `texts`.
- [ ] A one-token value (`slot_07`, `Vim`, `3`) reads back byte for byte.
- [ ] A string value containing `(`, `,`, `//`, `\"` and a newline escape
      (`"(name:\"a, (b) // c\\n\")"`) reads back byte for byte, and its
      laid-out text is still one string token.
- [ ] A hand-edited entry with a `// comment`, a `/* block */` and
      extra spaces reads back to the compact text of the same value.
- [ ] `to_text` twice gives the same text. Laying out text that is
      already laid out changes nothing. `read_bundle(to_text(b)) == b`
      (property test over generated maps of RON-ish and arbitrary
      strings).
- [ ] A version 1 file (the old fixture's text) is `BundleError::Version
      { found: 1, reads: 2 }`; a key in both `entries` and `texts` is
      `NotABundle`.
- [ ] `write_bundle` over a storage holding `ron::to_string` output (a
      browser's storage holds the same text) then `plan_import` and `apply`
      into an empty storage leaves every value byte-equal to the original.
- [ ] The public API is unchanged (CI's `api` job).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the cases above. The layout and compact passes alone: strings,
  raw strings (`r#"…"#`), escapes, line and block comments, empty
  containers, nesting.
- Property: `read_bundle(to_text(b)) == b` for generated bundles. The
  layout pass is idempotent. Compacting a laid-out value gives back the
  compact value.
- Golden: the `save bundle: save-bundle` row over the version 2 fixture.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
