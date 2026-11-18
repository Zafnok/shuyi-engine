---
id: "3220"
title: "A one-bit game over a pack read file by file draws its pictures, and the frame never reads a file only to throw it away"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3220 — One-bit pictures over a pack read file by file

## Context

[ADR-0128](../../docs/adr/0128-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md)
§2 and §3; 2262 (`Cow` reads, #478) and 2263 (a pack read file by file,
#479), both done. Found by the retro of PRs 379–484 (its item T20).

- `impl Images for shuyi_assets::Bundle`
  (`crates/ui/src/frame/onebit.rs`, about lines 370–377) calls
  `self.bytes(id.path())` and gives no picture unless the `Cow` is
  `Borrowed`. `draw_sprite` (about line 1374) asks for the picture of
  every sprite of every frame.
- Over a layer read file by file (`SourceFiles::read` →
  `fetch`, `crates/assets/src/pack/source.rs`, about lines 222–258) each
  of those asks is a seek and read from the source, a checksum and an
  allocation of the file, and the bytes are dropped at once: the sprite
  is still drawn as a grey rectangle. The test of #479 asserts that grey
  (`crates/ui/src/frame/onebit/tests.rs`, about lines 537–555, `by_file`).
  It also tells the bundle's watcher and its recording of a read each
  time (`Bundle::bytes`, `crates/assets/src/bundle.rs`).
- ADR-0128 §3 and the doc of `Index::into_layer` say such a game "keeps
  its pictures as values of its own" and hands them to the frame through
  `OneBitLook`, but the engine has no `Images` over values: the only
  decoded one-bit picture is `shuyi_assets::bitmap::OneBitPicture` (its
  file, owned and checked), and nothing draws it.
- 3315 moves Hello, Grid's Playdate build onto `pack_layer`, a layer read
  file by file. Hello, Grid hands its bundle to the frame as its pictures
  (`OneBit::pictures: &'static Bundle`, `Hello::one_bit_ctx` and
  `Hello::look`, `examples/hello-grid/src/lib.rs`, about lines 326–440),
  so every picture there would turn grey, and the frame would read every
  sprite's file from flash each frame. **3315 waits for this ticket**
  (its `blocked_by` holds 3220).
- The game: searched on 2026-11-17 at its pin `e738ffc2`.
  `gh search code --repo Zafnok/visions-of-shuyi 'OneBitLook'` finds only
  `tickets/open/1170-the-playdate-front-end-on-the-engines-backend.md`: no
  code of the game draws one-bit pictures yet. Game#1284 (done) wrote its
  own `bundle::lent_bytes` ("the bytes where the layer lends them, `None`
  where a layer would read the file on demand"), the same as this
  ticket's `Bundle::lent`.

## Nick input

None.

## Scope

**In:**

- **`Bundle::lent(path) -> Option<&'static [u8]>`**
  (`crates/assets/src/bundle.rs`): the bytes of the file at `path` where
  the layer that has it (the topmost, as `bytes` finds it) lends them,
  and `None` where that layer would read the file on demand. It never
  falls through to a lower layer: a lower layer's copy would be another
  picture than `bytes` gives. It reads nothing from a source. When it
  lends, it tells the watcher and the recording as `bytes` does; when it
  doesn't, it tells neither (nothing was read). Additive (ADR-0039).
- **`impl Images for Bundle`** reads through `lent`, not `bytes`: a
  layer that would read on demand gives no picture and costs no read.
  Its doc says to use `OneBitPictures` over such a layer.
- **`shuyi_ui::frame::onebit::OneBitPictures`**: one-bit pictures a game
  keeps as its own values (ADR-0128 §3), by image path, implementing
  `Images` (style 0, as `Images for Bundle` draws). Its fields are
  private and it is built only by its functions, so a field or a
  function added later is additive (ADR-0039: no `#[non_exhaustive]`
  needed). Its functions:
  - `read(bundle, ids: impl IntoIterator<Item = ImageId>) ->
    Result<Self, Vec<ContentError>>`: each image's file read once with
    `Bundle::bytes` (owned bytes moved, never copied; lent bytes copied
    once), kept as a `OneBitPicture` when `Bitmap::is_bitmap` says it is
    one; a PNG or a missing file is no picture, as `Images for Bundle`
    has it; a broken one-bit file is an error with its path;
  - `read_under(bundle, dir)`: the same over `Bundle::files_under(dir)`,
    a group as ADR-0128 §3 has it;
  - `append(&mut self, other)` and `remove_under(&mut self, dir)`: a
    game that loads a part's folder on entering it and drops it on
    leaving keeps one value.
  - It reads the bytes with `Bitmap::is_bitmap` and `OneBitPicture::new`,
    not `Formats::load_under::<OneBitPicture>`: `Formats::claims` reads
    extensions only (`crates/assets/src/format.rs`), and the `one-bit`
    format claims none, so on a build without the `png` feature
    `load_under` finds no picture at a `.png` path.
- **Hello, Grid** (`examples/hello-grid/src/lib.rs`):
  - `Hello::one_bit_ctx(bytes)` becomes a thin wrapper of a new
    `Hello::one_bit_ctx_over(layer: Layer)`, which does the rest over any
    layer (3315 passes `pack_layer`'s);
  - `OneBit::pictures` becomes a `OneBitPictures` read from the game's
    `ImageTable` (`OneBitPictures::read(bundle, game.images.ids())`),
    and `look()` hands it to `OneBitLook::new`.
- **Docs:** the doc comments of `Images for Bundle`, `OneBitPictures`,
  `Bundle::lent` and `Index::into_layer` (which points at
  `OneBitPictures` instead of "values of its own"); a line in ADR-0128
  §3 naming `OneBitPictures`; the book's *Assets and formats* chapter
  (`site/content/learn/book/assets-and-formats.md`, where it bakes
  pictures one-bit): a game whose pack is read file by file keeps its
  pictures in a `OneBitPictures`.
- **For the game:** additive. Its `bundle::lent_bytes` (game#1284) may
  call `Bundle::lent` at its next pin; nothing in it must change.
  Completion notes: a `Behaviour:` line for `Images for Bundle` (a layer
  read on demand is no longer read, and the watcher no longer hears of
  it).
- **Either order with 2267:** if 2267 is done, `Bundle::lent` lends a
  stored file of a layer whose source lends it (`Source::lend`, checked
  once), and a test shows a picture drawn from such a layer; if not,
  nothing to do (2267 makes `lent` use `Source::lend`).
- **Either order with 3073:** if 3073 is done and its `one_bit_picture`
  (`examples/hello-grid/src/tools.rs`) keeps an `Images` of the example's
  own, replace it with a `OneBitPictures`; if not, nothing to do (3073
  uses `OneBitPictures`).

**Out (do not do):**

- moving Hello, Grid's Playdate build onto `pack_layer` (3315);
- reading, on a layer read file by file, a picture once and keeping it
  inside the bundle: a store the engine owns is what ADR-0128 §1 turned
  down;
- one-bit pictures on the desktop or the web: they draw textures
  (3317).

**Estimated size:** about 800 lines (code 300, tests 400, docs 100).

## Implementation steps

1. Add `Bundle::lent`, with its tests over each kind of layer.
2. Move `Images for Bundle` onto `lent`; rewrite the `by_file` part of
   the test in `onebit/tests.rs` to also count the source's reads (a
   `Source` that counts), which stay at zero over many frames.
3. Write `OneBitPictures` and its `Images`, beside `Images for Bundle`.
4. Split `Hello::one_bit_ctx` and move `OneBit::pictures` onto
   `OneBitPictures`.
5. Write the docs; run the gates.

## Acceptance criteria

- [ ] Drawing a sprite for 60 frames through `Images for Bundle` over a
      layer read file by file reads its source 0 times (a test with a
      counting `Source`), and the sprite is a grey rectangle, as before.
- [ ] Over the same layer, `OneBitPictures::read` draws the sprite with
      the same pixels as `Images for Bundle` over the pack read whole
      (`Pack::open`), and reads each picture's file once.
- [ ] `Bundle::lent` gives the bytes over a table, an owned layer, a
      directory layer and a pack read whole, and `None` over a layer read
      file by file, reading nothing; with a layer that lends above one
      that doesn't (and the other way round), it answers for the topmost.
- [ ] `OneBitPictures::read` gives no picture for a PNG or a missing
      file, and an error naming the path for a broken one-bit file.
- [ ] `remove_under("part2/")` drops the pictures under it and keeps the
      others; `append` keeps both values' pictures.
- [ ] Hello, Grid's one-bit frame of the stage with its tiles drawn by
      `Hello::one_bit_ctx_over` over a layer read file by file
      (`Index::into_layer` over `InMemory`) equals the frame drawn over
      the pack read whole (`Hello::one_bit_ctx`), pixel for pixel.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Bundle::lent` over each layer and over two layers; the counting
  source's reads; `OneBitPictures`' `read`, `read_under`, `append` and
  `remove_under`, and its picture equal to the bundle's.
- Integration: Hello, Grid's one-bit frame over the two layers, equal.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
