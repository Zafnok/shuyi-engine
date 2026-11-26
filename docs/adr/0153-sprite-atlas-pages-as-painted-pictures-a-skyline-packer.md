# ADR-0153: Sprite atlases: a skyline packer in `shuyi-assets`, its pages shown as the frame's painted pictures

- **Status:** Accepted
- **Date:** 2026-11-26
- **Related tickets:** 2222 (this), 1123 (Aseprite and TexturePacker,
  which import into the same `Atlas`), 2399 (painted pictures), 3317
  (textures made when first drawn), 3229 (a lost GPU context)
- **Related ADRs:** [ADR-0007](0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md)
  (the `Atlas` kind), [ADR-0078](0078-pictures-a-skin-paints-at-run-time-kept-in-the-frames-buffer-with-a-revision-rgba-indexed-or-one-bit-and-dithered-at-paint-time.md)
  (painted pictures), [ADR-0128](0128-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md) §5

## Context

Ticket 2222 packs loose images into atlas pages, at load, at bake and
while the game runs (a sprite added later: a player's avatar, text
rendered once). A backend then binds one texture for many sprites, and a
page that changed is uploaded again.

A sprite in the frame (`glyph_buffer::Sprite`) names an image by
`ImageId`, whose texture `shuyi-app` makes from the game's files. A page
is no file: its pixels are the game's, and change while it runs. The
frame already carries pictures whose pixels the game makes, with a
revision: the painted pictures of ADR-0078, which `shuyi-app` uploads the
first frame a sprite draws one and again when its revision moves, and
which the raster, the one-bit renderer, the snapshot, a backdrop and a
sub-buffer already carry.

## Decision

1. **`shuyi_assets::atlas`** holds the kind, `Atlas`: square pages
   (`image::Image`, at most `image::MAX_SIDE`) and a table of name to
   `Placed { page, rect }`. Its names and its pages' names (`<id>#<n>`)
   are `&'static str` (ADR-0128 §1); a baked atlas leaks its names when it
   is read. Its binary form (`Bake::asset::<Atlas>`) is owned strings, so
   `Atlas` is `DeserializeOwned`.
2. **The packer is a skyline**, bottom-left: the lowest place, then the
   leftmost, on the first page with room, else a new page. `AtlasBuilder`
   packs the tallest first, then by name, so the order images are added
   in doesn't change the atlas. Each page keeps its skyline, so
   `Atlas::insert` places a later sprite in free space without moving one.
   Padding is kept between sprites and from the pages' edges. No rotation,
   no trimming.
3. **`atlas.ron`** (`AtlasRon`, id `atlas`, `.ron`, a standard format) is
   an `AtlasSpec { from, page_size, padding }`: every image directly
   inside `from`, read through a registry of the picture formats
   (`Formats::load_each`). It holds its own registry, since a format's
   `DecodeCx` gives the bundle but not the game's `Formats`.
4. **A target's texture limit** is `Display::max_texture_side` (4096
   unless a profile says; `web` says 2048, ADR-0136 §3), given to
   `AtlasBuilder::with_max_side` or `AtlasRon::with_max_side`.
5. **In the frame, a page is a painted picture**, named by the page's
   name, and an atlas sprite is a `PaintedSprite` of its page and
   rectangle: `shuyi_ui::frame::atlas::{show, insert, sprite}`. `insert`
   writes only the inserted rectangle into the page's picture, so only that
   picture's revision moves. `shuyi-app` uploads it through `upload_rgba`
   with the other painted pictures.

## Consequences

- The window, the raster (`frame-png`), the one-bit renderer and the
  snapshot draw an atlas with no new `Item` and no new backend path; the
  render card holds an atlas sprite and an inserted one.
- A page is in memory twice: in the game's `Atlas` (for later inserts)
  and in the frame's picture. A game that never inserts may drop its
  `Atlas` once its pages are shown.
- An atlas sprite has what a `PaintedSprite` has (brightness, a clip),
  not a `Sprite`'s paints, opacity or mirroring.
- When 3229 lands, its lost-context path forgets the painted textures,
  which holds the pages too.
- An importer (1123) fills `Atlas` in its crate; its pages get no
  skyline of free space until it adds one.

## Alternatives considered

- **A new `Item` naming an atlas page, the atlases kept in the
  `GlyphBuffer`.** Every backend, the raster, the one-bit renderer, the
  snapshot and the absorbing of a backdrop's and a sub-buffer's pictures
  would need it: the painted pictures do all of that already.
- **Pages as images of the `ImageTable`, uploaded from `shuyi-app`'s file
  reader.** A page is not a file, and a page that changes at run time has
  no way from the game to the renderer but the frame.
- **Shelf or max-rects packing.** A shelf wastes the space above short
  sprites in a row; max-rects packs a little tighter but keeps a list of
  free rectangles that grows with every insert. The skyline is a short
  list per page and packs well for sprites sorted by height.
