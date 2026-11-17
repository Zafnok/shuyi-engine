---
id: "2262"
title: "The bundle's reads hand out their bytes as a Cow, not &'static: Bundle::bytes, file, test_part, LoadFile::text, DecodeCx::bytes and text, and AppConfig::files"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2262 — The bundle's reads hand out a Cow

## Context

[ADR-0126](../../docs/adr/0126-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md)
§1, from ticket 2906's numbers
([`docs/research/2906-asset-memory.md`](../../docs/research/2906-asset-memory.md)):

- Every read of the bundle is `&'static` today (`Bundle::bytes`,
  `Bundle::file`, `Bundle::test_part`, `LoadFile::text`,
  `DecodeCx::bytes`, `DecodeCx::text`, `crates/assets/src/bundle.rs` and
  `format.rs`), so a file read from a pack must stay in memory for the
  whole run.
- A Playdate pack at its 40 MB `package_bytes` takes 67 MB of heap read
  whole, against a 10 MB heap; read file by file it peaks at 0.6 MB
  (2263, 3315). A file read on demand has nothing to borrow from, so the
  reads must hand out bytes the reader can drop.
- The API breaks, so it is built before the first release (2005 waits for
  it; Nick, 2026-11-14: no run of breaking versions after the MVP).

The decision: a read hands out `Cow<'static, [u8]>` (`alloc::borrow::Cow`).
Every layer that exists today hands out `Cow::Borrowed`, so nothing is
copied; 2263's layer read file by file hands out `Cow::Owned`. Paths stay
`&'static`.

**The call sites** (ADR-0126, *The call sites, counted*, 2026-11-16):

- the engine: 16 reads in 12 files of code, the `&'static [u8]`
  signatures that carry them (45 lines, 20 in `shuyi-app`), and about 110
  lines of tests;
- the game (`gh search code --repo Zafnok/visions-of-shuyi`): its wrapper
  `crates/content/src/bundle.rs` (`file`, `bytes`, `test_part`,
  `load_each`, each returning `&'static`), and the files that call it: 32
  through `bundle::file(`, 11 through `bundle::bytes(`, 7 through
  `test_part(`, and 3 that read a `Bundle` directly
  (`crates/xtask/src/playdate_images.rs`, `crates/ui/src/lang_keys.rs`,
  `crates/content/src/bundle.rs`).

## Nick input

None.

## Scope

**In:**

- **`shuyi_assets::bundle`:**
  - `Bundle::bytes(&self, path) -> Option<Cow<'static, [u8]>>`;
  - `Bundle::file(&self, path) -> Option<Cow<'static, str>>`, `None` when
    not UTF-8 as today (a borrowed file stays borrowed; an owned one is
    checked and moved into a `String`, never copied);
  - `Bundle::test_part(&self, path) -> Option<(String, Cow<'static,
    str>)>`;
  - `LoadFile::text: &'a str`, borrowed for the parse call from the
    `Cow` `load_each` holds;
  - every layer kind of today (`Files::Dir`, `Table`, `Owned`) hands out
    `Cow::Borrowed`. `Layer::owned` and hot reload stay leaked (ADR-0126
    §1);
  - the module comment says what a read hands out and why.
- **`shuyi_assets::format`:** `DecodeCx::bytes` and `DecodeCx::text`
  hand out the same `Cow`s. `files_under` and the keys of
  `Formats::load_under` and `load_each` stay `&'static str`.
- **Every caller in the workspace** (the 16 reads, the formats that read
  a sidecar's file, `shuyi-cli`'s `basic.rs`, `shuyi-ui`'s `text.rs` and
  dialogue preview, `examples/hello-grid`'s `asset_memory.rs`), and the
  tests that compare a read with a literal.
- **`shuyi-app`:** `AppConfig::files: fn(&str) -> Option<Cow<'static,
  [u8]>>`; `Renderer::new`'s `bytes` and its `sources` keep the `Cow`
  they are given (a `Cow::Borrowed` costs what the `&'static` did).
- **The templates** (`templates/`) and the skill pack's worked examples
  that read the bundle, if any, compile and pass as before.
- **Docs:** `docs/formats.md`, where it says a read is `&'static`; the
  book's chapter on files and formats, if it shows a read.
- **Either order with 2228:** if 2228 is done, `LayerCopy::bytes` becomes
  `Cow<'static, [u8]>` and `LayerCopy::text` gives `Option<Cow<'static,
  str>>` here; if not, nothing to do (2228 writes them so).
- **For the game:** a break. The game's PR that moves the pin follows
  this one. The cheapest move is in its wrapper: return the `Cow`, or
  keep `&'static` by leaking what isn't `Cow::Borrowed` (nothing is,
  until 2263's layer). The migration blocks say both.

**Out (do not do):**

- a layer read file by file, and the pack's new version (2263);
- paths that aren't `&'static` (2264, parked);
- an asset store, handles or reference counts (ADR-0126 §1: none);
- `Arc` (ADR-0126, *Alternatives*).

**Estimated size:** about 900 lines (code 300, tests 450, docs 150).

## Implementation steps

1. Change `Files::get`, `table_get` and the private `find` to hand out
   `Cow<'static, [u8]>`, then `Bundle::bytes`, `file`, `test_part` and
   `load_each`.
2. Change `DecodeCx::bytes` and `text`, then the engine's formats.
3. Change `AppConfig::files` and `Renderer` in `shuyi-app`, then every
   other caller and test in the workspace.
4. Write the docs, the `Breaking:` lines and their migration blocks.
5. Run the gates.

## Acceptance criteria

- [ ] `Bundle::bytes`, `Bundle::file`, `Bundle::test_part`,
      `LoadFile::text`, `DecodeCx::bytes`, `DecodeCx::text` and
      `AppConfig::files` hand out the bytes as ADR-0126 §1 says.
- [ ] A read of each layer kind of today is `Cow::Borrowed` (a test per
      kind).
- [ ] No path type changes: `files_in`, `files_under`, `ImageId`,
      `ImageTable` and the keys of `load_under` and `load_each` are as
      before.
- [ ] 2005's `blocked_by` holds this ticket (it does from 2919).
- [ ] Each break has a `Breaking:` line and a `migration` block, with
      the game's wrapper as its example.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each layer kind hands out `Cow::Borrowed`; `file` of bytes that
  aren't UTF-8 is `None`; `load_each` parses from the borrowed text; a
  `DecodeCx` without a bundle reads nothing.
- The existing tests of the bundle, the formats and the pack pass,
  changed only where they compare a read with a literal.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
