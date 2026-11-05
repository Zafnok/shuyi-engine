# ADR-0078: Pictures a skin paints at run time: kept in the frame's buffer with a revision, RGBA, indexed or one-bit, and dithered at paint time

- **Status:** Accepted
- **Date:** 2026-11-04
- **Related tickets:** 2399 (this), 2248, 2779, 2780, 3056, 3103, 2335, 2337, 2222
- **Extends:** ADR-0002 §4, ADR-0026

## Context

Nick, 2026-11-03 (`docs/decisions.md`, *Bad Apple and DOOM*): anything
that can display graphics must run Bad Apple, and anything that can run
video game logic must run DOOM. Both need a picture whose pixels the game
makes while it runs: Bad Apple decodes each frame of a one-bit flipbook
(2248) into a 400 × 240 one-bit picture (2779); DOOM's software renderer
writes 320 × 200 palette indices 35 times a second and flashes its
palette when the player is hurt (2780). An emulator's screen, a paint
program's canvas, a fog-of-war minimap, a plasma and the integer
rasterizer of 3103 need the same.

What the frame had (`shuyi_ui::frame::glyph_buffer`): `Item::Rect`, a
solid rectangle, and `Item::Sprite`, part of an image of the
`ImageTable`. Every image is a file baked or embedded before the game
starts; `shuyi-app` makes a texture of each once (`render.rs`), and the
raster painter (`frame-png`) and the one-bit renderer (ADR-0026, made
fast by 3056) read the same table.

Facts that shape the choice:

- `Game` keeps **one** `GlyphBuffer` for its whole life and draws every
  frame into it again (`Game::redraw` fills it and cuts its items). The
  buffer is the one thing every painter is handed: `shuyi-app`'s
  `Renderer::draw`, `raster::Painter::render`, `onebit::render` and
  `render_changed`, the snapshot text, the Harness and the contract.
- `Screen::draw(&self, ctx: &Ctx<G>, buf: &mut GlyphBuffer)`: a skin
  holds `&Ctx` and `&mut GlyphBuffer` while it paints.
- A screen may draw into a buffer of its own and `blit` it into the
  frame, and a buffer's backdrop is another buffer (game ADR-0048),
  often made afresh each frame.
- The Playdate backend keeps a clone of the last frame's buffer and
  redraws only what differs (`render_changed`).

## Decision

1. **The pictures live in the buffer.** `GlyphBuffer` holds a
   `PaintedTable` (`shuyi_ui::frame::painted`): a map from `PaintedId` to
   `Painted`. `Game`'s buffer outlives every frame, so its table does:
   the frame's owner keeps it. Drawing the frame again (`fill_rect`,
   `Game::redraw`) cuts items, never pictures; a skin that is done with
   one removes it (`PaintedTable::remove`). A skin reaches the table
   while it paints through the buffer it draws into:
   `buf.pictures_mut().get_or_insert_with(id, || Painted::rgba(64, 48))`,
   then writes the picture's pixels and places it with
   `buf.add_painted(PaintedSprite::new(id, src, dest, layer))`. The rules
   never write pixels: the core stays pure, a skin paints (rule 9).
2. **A revision says when it changed.** Each `Painted` has a `u32`
   revision that moves (wrapping) whenever its pixels, palette or mask
   are borrowed for writing (`pixels_mut`, `palette_mut`, `mask_mut`),
   whether or not they change. A picture put in the table over another
   of its id (`insert`) takes a revision past the old one's, so an id's
   revision never comes back while its table lives. A skin borrows the
   pixels only in a frame that changes them; one that can't tell
   compares first.
3. **Pictures come along with their items.** `blit` copies the
   pictures of the source's painted items, and `set_backdrop` those of
   the scene, into the receiving buffer's table: one whose pixels,
   palette, mask, size and kind equal the table's is left (its revision
   with it); any other replaces it, its revision moving past the old.
   Every painter looks a painted item up in the **frame's** table, the
   top buffer's, also for an item of a backdrop's scene. So a sub-buffer
   or a scene made afresh each frame costs an upload only when its
   picture changed. One id names one picture in a frame.
4. **Three kinds of pixels** (`PaintedKind`, `#[non_exhaustive]`):
   - `Rgba`: four bytes a pixel, row-major, alpha blended as a sprite's
     is (`Painted::rgba(w, h)`, all clear);
   - `Indexed`: a byte a pixel and a palette of 256 `Rgb`
     (`Painted::indexed(w, h, &palette)`: up to 256 colours given, the
     rest black, every index drawn and opaque). A palette change is one
     write, and one upload;
   - `OneBit`: `onebit::Image`'s layout: rows of `ceil(w / 8)` bytes,
     the highest bit the leftmost pixel, a set bit white, and an
     optional mask of the same shape, a set bit drawn
     (`Painted::one_bit(w, h)`, all black; `.with_mask()`, all drawn).
   Sides are `u16`. `Painted::rgba_into` expands any kind to RGBA8 (a
   one-bit pixel white or black, its masked-out pixels clear); a
   painter that wants RGBA calls it and never matches on the kind.
5. **It is placed like a sprite.** `Item::Painted(PaintedSprite)`: the
   picture's id, a `src` in its pixels, a `dest` and a `clip` in console
   pixels, a `Layer`, and a `brightness` that `GlyphBuffer::fade`
   lowers, as a sprite's. Scaled nearest-neighbour only, never smoothed
   (a painted picture is pixel art): `shuyi-app`'s texture has
   `FilterMode::Nearest`, and the raster and one-bit painters sample the
   pixel under each pixel's centre, as for sprites. No flip, rotation,
   tint, opacity or blend mode (2335 and 2337 own those for sprites). A
   painted item whose picture the table lacks draws as a missing image
   does (magenta, or grey on a one-bit target).
6. **`Item` is `#[non_exhaustive]`.** The new variant broke a game's
   `match` on `Item` once; the next one won't.
7. **Each painter:**
   - `shuyi-app` keeps a texture per `PaintedId`: made the first frame
     the picture is drawn (`Texture2D::from_rgba8`), updated
     (`update_from_bytes`) only in a frame whose revision differs from
     the one uploaded (made again if its size changed), and dropped at
     the end of a frame that didn't draw the picture. An indexed picture
     is expanded through its palette at upload. Debug builds count the
     uploads, for a test.
   - The raster painter (`frame-png`) makes the picture RGBA and draws
     it through the sprite's own blit: pixel for pixel equal to a sprite
     of the same pixels.
   - The one-bit renderer draws a one-bit picture as a one-bit sprite
     (3056's whole-byte path; its brightness drops white pixels as a
     sprite's does). An RGBA or indexed picture is **dithered at paint
     time, in integers**: each screen pixel takes the picture's pixel
     under its centre, its brightness `bitmap::dither::brightness`
     (`(77 r + 150 g + 29 b + 128) / 256`, scaled by the item's
     brightness), and is white where `4 × BAYER[y % 8][x % 8] + 2 ≥ 255
     − brightness`, `onebit::BAYER` at the **screen** pixel, so the
     pattern stays put as the picture moves and scales, as roles'
     patterns do. A pixel whose alpha is below 128 isn't drawn.
     `onebit::dither_row` does a frame byte's eight pixels at once,
     so the renderer writes whole bytes. `render_changed` redraws a
     painted item whose picture differs from the last frame's.
   - The headless platform draws nothing; the scene and the snapshot
     carry the item and the table.
8. **The snapshot** lists a painted item as one line:
   `layer  painted x,y wxh  id WxH kind rN hash`, with `dest`, the
   picture's size, `rgba`, `indexed` or `one-bit`, its revision and the
   FNV hash (`shuyi_assets::fnv`) of its size, kind, pixels, palette and
   mask, then ` src=x,y wxh` when `src` isn't the whole picture,
   ` bright=N` and ` clip=x,y wxh` as a sprite's. A test sees that a
   picture changed without reading it; only a test of a look reads its
   pixels.

## Consequences

- Bad Apple and DOOM have what they need from the frame: one upload a
  changed frame on the GPU, none for a still one, and a picture drawn
  on every target, the one-bit ones dithered without a bake.
- Nothing changes for a game that paints nothing: no renderer's
  signature changed, and the table is empty.
- `Item` gained a variant and became `#[non_exhaustive]`: a game's
  `match` on it needs a `_` arm (a `Breaking:` line in 2399).
- A buffer's clone and equality now include its pictures' pixels: the
  Playdate's last-frame copy holds a 400 × 240 one-bit picture (12 KB)
  more. Copying a sub-buffer's or a scene's pictures into the frame's
  table compares their bytes each frame.
- A picture stays in the table until the skin removes it; a skin that
  names pictures by the hundred grows the table.
- Dithering at paint time costs the device a brightness and a compare a
  pixel, where a baked one-bit sprite costs a shift a byte. A game that
  can make its pictures one-bit itself (Bad Apple) should.

## Alternatives considered

- **A new image added to the `ImageTable` each frame** — its ids are
  `&'static str` paths of files, interned once, and `shuyi-app` makes
  each texture once at start; changing a picture would mean a new id and
  a new texture each frame, and nothing would drop the old ones.
- **A stream of `Item::Rect`s** — 64,000 rectangles a frame for DOOM,
  each a draw call on the GPU and a snapshot line.
- **The table in `Ctx`** — a skin holds `&Ctx`, so it would need a
  `RefCell`, and every painter (`Renderer::draw`, `Painter::render`,
  `onebit::render`, `to_snapshot`) would take the table beside the
  buffer: a break of each, and of every caller.
- **The revision kept in the item** (taken when it is placed) — the
  skin may write the pixels after placing them, and a picture drawn by
  two items would hold two revisions.
- **Dithering at the picture's own pixels** — a scaled picture would
  show its dither scaled up, and the pattern would move with the
  picture; the roles' patterns are fixed to the screen, so these are.
- **A global counter for revisions** — the no-`std` targets without
  atomics (the GBA) have no safe one, and per-id revisions in one
  long-lived table are enough.
