---
id: "2262"
title: "The bundle's reads hand out their bytes as a Cow, not &'static: Bundle::bytes, file, test_part, LoadFile::text, DecodeCx::bytes and text, and AppConfig::files"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-17
---

# 2262 — The bundle's reads hand out a Cow

## Context

[ADR-0128](../../docs/adr/0128-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md)
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

**The call sites** (ADR-0128, *The call sites, counted*, 2026-11-16):

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
    `Cow::Borrowed`. `Layer::owned` and hot reload stay leaked (ADR-0128
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
- an asset store, handles or reference counts (ADR-0128 §1: none);
- `Arc` (ADR-0128, *Alternatives*).

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

- [x] `Bundle::bytes`, `Bundle::file`, `Bundle::test_part`,
      `LoadFile::text`, `DecodeCx::bytes`, `DecodeCx::text` and
      `AppConfig::files` hand out the bytes as ADR-0128 §1 says.
- [x] A read of each layer kind of today is `Cow::Borrowed` (a test per
      kind).
- [x] No path type changes: `files_in`, `files_under`, `ImageId`,
      `ImageTable` and the keys of `load_under` and `load_each` are as
      before.
- [x] 2005's `blocked_by` holds this ticket (it does from 2919).
- [x] Each break has a `Breaking:` line and a `migration` block, with
      the game's wrapper as its example.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each layer kind hands out `Cow::Borrowed`; `file` of bytes that
  aren't UTF-8 is `None`; `load_each` parses from the borrowed text; a
  `DecodeCx` without a bundle reads nothing.
- The existing tests of the bundle, the formats and the pack pass,
  changed only where they compare a read with a literal.

## Completion notes

Done 2026-11-17.

**What was done.**

- `shuyi_assets::bundle`: `Files::get`, `table_get` and `find` hand out
  `Cow<'static, [u8]>`, and every layer kind of today (`Files::Dir`,
  `Table`, `Owned`; a pack's layer is a `Table`) lends, `Cow::Borrowed`.
  `Bundle::bytes` returns `Option<Cow<'static, [u8]>>`; `Bundle::file`
  `Option<Cow<'static, str>>`, through a private `text_of` that keeps
  lent bytes lent and moves owned ones into a `String` uncopied (a test
  checks the pointer); `Bundle::test_part` gives `(String, Cow<'static,
  str>)`; `LoadFile::text` is `&'a str`, lent for the parse from the
  `Cow` `load_each` holds. The module comment says what a read hands out
  and why. `Layer::owned` and hot reload stay leaked.
- `shuyi_assets::format`: `DecodeCx::bytes` and `DecodeCx::text` hand out
  the same `Cow`s. `files_under` and every path are unchanged.
- `shuyi-app`: `AppConfig::files` (and `AppConfig::new`'s `files`) is
  `fn(&str) -> Option<Cow<'static, [u8]>>`; `Renderer::new`'s `bytes`,
  its `sources`, the hot reload's fresh pictures, the capture and the
  watched folder keep the `Cow` they are given.
- Every caller in the workspace: the engine's formats (font, audio
  manifest, language pack, import settings, RON), `ImageTable::load` and
  `load_with`, `shuyi-cli`'s `basic.rs` and bake contact sheet,
  `shuyi-ui`'s `text.rs`, dialogue preview and one-bit pictures,
  `examples/hello-grid` (its `file()`, tools, tests and
  `asset_memory.rs`), the template (`base` and `top-down-grid`, and the
  plans' golden), and the tests that compare a read with a literal
  (`.as_deref()`). `site/snippets` read no file and are unchanged.
- Tests (`crates/assets/src/bundle/read_tests.rs`): a table, an embedded
  folder and a run-time layer (also after a replace) each lend; a test
  part is lent; bytes that aren't UTF-8 are no text, lent or owned; owned
  bytes move into their text uncopied; `load_each` parses each file's
  text; a `DecodeCx` without a bundle reads nothing, and with one it
  lends.
- Docs: `docs/formats.md` (*The parts*: what a read hands out; *Hot
  reload*: "every borrowed read"), and the doc comments of `bundle.rs`.

**Deviations.**

- `ImageTable::from_files` and `from_files_with` take
  `impl IntoIterator<Item = (&'static str, impl AsRef<[u8]>)>` in place
  of `(&'static str, &'a [u8])`, so `load` and `load_with` hand them the
  `Cow`s without collecting every file first. A caller passing `&[u8]`
  compiles as before (the game's `content::image::from_files` does); an
  empty `[]` names its type.
- `shuyi_cli::frame_png::Painter::new`'s `read` returns `Option<B>` for
  any `B: AsRef<[u8]>`, so the game's call with its wrapper's
  `bundle::bytes` compiles whether the wrapper returns `&'static [u8]` or
  the `Cow`. A `|_| None` names its type (`None::<&[u8]>`).
- `shuyi_ui::dialogue::preview::DialogueFiles` keeps the `Cow`s it
  reads, so `files()` lends `&[u8]` for as long as the `DialogueFiles`,
  not `&'static [u8]`.
- `Images for Bundle` (`shuyi-ui`, the one-bit frame's pictures read
  where they lie) draws a file only when its layer lends it: a
  `Cow::Owned` read has nothing to lend the frame. Every layer of today
  lends, so nothing changes; 2263, whose layer hands out owned reads,
  now says so in its scope (**From 2262 (done)**).
- `examples/hello-grid` keeps its one-bit font's bits for the run: it
  keeps lent bytes as they are and would leak owned ones, the
  migration's second way.
- 2228 is open, so `LayerCopy` doesn't exist yet: its ticket now says,
  as what holds, that its `bytes` and `text()` are `Cow`s.

SIZE_LINE

Breaking: shuyi-assets: `Bundle::bytes`, `Bundle::file`, `Bundle::test_part`, `DecodeCx::bytes`, `DecodeCx::text`: they hand out a `Cow<'static, [u8]>` or `Cow<'static, str>`, not `&'static`; a game derefs it (`&bytes`, `.as_deref()`), or keeps `&'static` in its own wrapper by leaking what isn't `Cow::Borrowed` (nothing is, until 2263)

```migration
since: 0.1.0
what: A read of the bundle hands out its bytes as a Cow, so a file read on demand can be dropped; every layer of today lends them, Cow::Borrowed.
old: "Bundle::bytes(&self, &str) -> Option<&'static [u8]>; Bundle::file(&self, &str) -> Option<&'static str>; Bundle::test_part(&self, &str) -> Option<(String, &'static str)>"
new: "Bundle::bytes(&self, &str) -> Option<Cow<'static, [u8]>>; Bundle::file(&self, &str) -> Option<Cow<'static, str>>; Bundle::test_part(&self, &str) -> Option<(String, Cow<'static, str>)>"
before: |
  // crates/content/src/bundle.rs, the game's wrapper
  pub fn file(path: &str) -> Option<&'static str> {
      install().file(path)
  }
  pub fn bytes(path: &str) -> Option<&'static [u8]> {
      install().bytes(path)
  }
  // a caller
  let map = parse_map(path, bundle::file(path)?)?;
after: |
  // Either return the Cow, and deref it where it is read:
  pub fn file(path: &str) -> Option<Cow<'static, str>> {
      install().file(path)
  }
  let map = parse_map(path, &bundle::file(path)?)?;
  // or keep &'static in the wrapper, leaking only what isn't lent
  // (nothing is, until a pack read file by file):
  pub fn bytes(path: &str) -> Option<&'static [u8]> {
      Some(match install().bytes(path)? {
          Cow::Borrowed(bytes) => bytes,
          Cow::Owned(bytes) => Box::leak(bytes.into_boxed_slice()),
      })
  }
find: ["parse_map(path, bundle::file(path)?)?", "-> Option<&'static [u8]> {\n    install().bytes(path)"]
```

```migration
since: 0.1.0
what: A test that compares a read with a literal compares its deref.
old: "assert_eq!(bundle.file(path), Some(\"text\"))"
new: "assert_eq!(bundle.file(path).as_deref(), Some(\"text\"))"
before: |
  assert_eq!(bundle.file("data/a.ron"), Some("a"));
  assert_eq!(bundle.bytes("a.png"), Some(&b"png"[..]));
after: |
  assert_eq!(bundle.file("data/a.ron").as_deref(), Some("a"));
  assert_eq!(bundle.bytes("a.png").as_deref(), Some(&b"png"[..]));
find: [".file(\"data/a.ron\"), Some(", ".bytes(\"a.png\"), Some("]
```

```migration
since: 0.1.0
what: A format that reads a file its file names gets a Cow from DecodeCx.
old: "DecodeCx::bytes(&self, &str) -> Option<&'static [u8]>; DecodeCx::text(&self, &str) -> Option<&'static str>"
new: "DecodeCx::bytes(&self, &str) -> Option<Cow<'static, [u8]>>; DecodeCx::text(&self, &str) -> Option<Cow<'static, str>>"
before: |
  let png = cx.bytes(&picture);
  let sheet = Sheet::from_source(text, png)?;
after: |
  let png = cx.bytes(&picture);
  let sheet = Sheet::from_source(text, png.as_deref())?;
find: ["Sheet::from_source(text, png)"]
```

Breaking: shuyi-assets: `LoadFile::text`: it is `&'a str`, lent for the parse, not `&'static str`; a parser that keeps part of the text copies that part

```migration
since: 0.1.0
what: load_each lends each file's text for its parse only.
old: "LoadFile { text: &'static str, .. }"
new: "LoadFile<'a> { text: &'a str, .. }"
before: |
  bundle.load_each("names", "ron", |file| Ok(Names { source: file.text }))
after: |
  bundle.load_each("names", "ron", |file| Ok(Names { source: file.text.to_owned() }))
find: ["source: file.text }"]
```

Breaking: shuyi-assets: `ImageTable::from_files`, `ImageTable::from_files_with`: each file's bytes are any `impl AsRef<[u8]>`; a call with an empty array names its type

```migration
since: 0.1.0
what: An image table reads each file's bytes lent or owned.
old: "files: impl IntoIterator<Item = (&'static str, &'a [u8])>"
new: "files: impl IntoIterator<Item = (&'static str, impl AsRef<[u8]>)>"
before: |
  ImageTable::from_files(bundle, [])
after: |
  ImageTable::from_files(bundle, [] as [(&str, &[u8]); 0])
find: ["ImageTable::from_files(bundle, [])"]
```

Breaking: shuyi-app: `AppConfig::files`, `AppConfig::new`, `Renderer::new`: the file reader is `fn(&str) -> Option<Cow<'static, [u8]>>`; a game whose wrapper keeps `&'static` passes a closure that lends it

```migration
since: 0.1.0
what: The platform shell reads the game's files as the bundle hands them out.
old: "files: fn(&str) -> Option<&'static [u8]>"
new: "files: fn(&str) -> Option<Cow<'static, [u8]>>"
before: |
  let mut config = AppConfig::new(WINDOW, STORAGE_NAME, bundle::bytes, font::ATLAS_PNG_PATH, sounds);
after: |
  // With the wrapper returning the Cow, as before; with it kept &'static:
  let mut config = AppConfig::new(
      WINDOW,
      STORAGE_NAME,
      |path| bundle::bytes(path).map(Cow::Borrowed),
      font::ATLAS_PNG_PATH,
      sounds,
  );
find: ["AppConfig::new(WINDOW, STORAGE_NAME, bundle::bytes"]
```

Breaking: shuyi-cli: `frame_png::Painter::new`: `read` returns `Option<B>` for any `B: AsRef<[u8]>`; a `|_| None` names its type

```migration
since: 0.1.0
what: The frame painter reads pictures lent or owned.
old: "read: impl Fn(&str) -> Option<&'a [u8]>"
new: "read: impl Fn(&str) -> Option<B>, B: AsRef<[u8]>"
before: |
  Painter::new(atlas, &png, &images, |_| None, clear)
after: |
  Painter::new(atlas, &png, &images, |_| None::<&[u8]>, clear)
find: ["|_| None, clear)"]
```

Breaking: shuyi-ui: `dialogue::preview::DialogueFiles::files`: it lends each file's bytes for as long as the `DialogueFiles`, not `&'static`; a caller that kept them past it copies them

```migration
since: 0.1.0
what: The dialogue preview keeps the bundle's reads as it got them.
old: "fn files(&self) -> impl Iterator<Item = (&'static str, &'static [u8])> + '_"
new: "fn files(&self) -> impl Iterator<Item = (&'static str, &[u8])> + '_"
before: |
  let kept: Vec<&'static [u8]> = files.files().map(|(_, bytes)| bytes).collect();
after: |
  let kept: Vec<Vec<u8>> = files.files().map(|(_, bytes)| bytes.to_vec()).collect();
find: ["let kept: Vec<&'static [u8]> = files.files()"]
```

Behaviour: shuyi-ui: the one-bit frame's `Images for Bundle` draws only a picture its layer lends (`Cow::Borrowed`); every layer of today lends, so nothing a game sees changes until a layer read on demand (2263)

Book: none, the book shows no read of the bundle: its chapter on assets and formats (`site/content/learn/book/assets-and-formats.md`) loads through `Formats` and lays layers, whose signatures don't change. `docs/formats.md` says what a read hands out.

**For the game.** Its PR that moves the pin adopts the breaks above, as the migration blocks say. Searched with `gh search code --repo Zafnok/visions-of-shuyi` for `test_part`, `LoadFile`, `DecodeCx`, `AppConfig`, `ImageTable::from_files`, `Painter::new`, `DialogueFiles` and `Renderer::new`. The hits: its wrapper `crates/content/src/bundle.rs` (`file`, `bytes`, `test_part`, `load_each`), `crates/app/src/main.rs` and `crates/story-preview/src/main.rs` (`AppConfig::new` with `bundle::bytes`), `crates/content/src/formats.rs` (`DecodeCx`), `crates/content/src/image.rs` (`from_files` with `&[u8]`: compiles as before), `crates/xtask/src/game_tools.rs` (`Painter::new` with `bundle::bytes`: compiles either way), `crates/ui/src/story_preview.rs` (`DialogueFiles::files`, read within the borrow: compiles as before).

Nick input: none.
