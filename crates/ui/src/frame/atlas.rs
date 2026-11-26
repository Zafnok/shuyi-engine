//! A sprite atlas in the frame (ADR-0153): its pages are pictures of
//! the frame's [`PaintedTable`] (ADR-0078), each named by its page's
//! [`name`](Page::name), and a sprite of it is a [`PaintedSprite`] of its
//! page and rectangle. So `shuyi-app` uploads a page the first frame a
//! sprite draws it and again in a frame whose page changed, and the raster
//! paints it as the window does: `frame-png` shows what players see.
//!
//! The game keeps its [`Atlas`] (packed at load, or baked), puts its pages
//! in the frame once with [`show`], and adds a sprite while it runs with
//! [`insert`], which writes only the page that changed:
//!
//! ```
//! use shuyi_assets::atlas::AtlasBuilder;
//! use shuyi_assets::image::Image;
//! use shuyi_ui::frame::atlas;
//! use shuyi_ui::frame::{Cell, GlyphBuffer, Layer, PxRect, Rgb};
//!
//! let mut buf = GlyphBuffer::new(8, 4, Cell::new(' ', Rgb::new(255, 255, 255), Rgb::new(0, 0, 0)));
//! let mut builder = AtlasBuilder::new("hud", 32);
//! builder.add("coin", &Image::filled(8, 8, [255, 200, 0, 255]))?;
//! let mut hud = builder.build()?;
//! atlas::show(&hud, buf.pictures_mut());
//! let avatar = Image::filled(12, 12, [0, 128, 255, 255]);
//! atlas::insert(&mut hud, buf.pictures_mut(), "avatar", &avatar)?;
//! let sprite = atlas::sprite(&hud, "avatar", PxRect::new(4, 4, 12, 12), Layer::Over);
//! buf.add_painted(sprite.unwrap());
//! # Ok::<(), shuyi_assets::atlas::AtlasError>(())
//! ```
use shuyi_assets::atlas::{Atlas, AtlasError, Page, Placed};
use shuyi_assets::image::Image;

use crate::frame::glyph_buffer::{Layer, PxRect};
use crate::frame::painted::{Painted, PaintedId, PaintedSprite, PaintedTable};

/// The id of `page`'s picture in a frame: its name.
pub const fn page_id(page: &Page) -> PaintedId {
    PaintedId::new(page.name())
}

/// `page` as an RGBA picture.
fn picture_of(page: &Page) -> Painted {
    let image = page.image();
    // A page is at most `image::MAX_SIDE` (4096) on a side.
    let side = |px: u32| u16::try_from(px).unwrap_or(u16::MAX);
    let mut picture = Painted::rgba(side(image.width), side(image.height));
    let pixels = picture.pixels_mut();
    let n = pixels.len().min(image.rgba.len());
    pixels[..n].copy_from_slice(&image.rgba[..n]);
    picture
}

/// Puts each page of `atlas` that `pictures` lacks in it. A page already
/// there is kept as it is.
pub fn show(atlas: &Atlas, pictures: &mut PaintedTable) {
    for page in atlas.pages() {
        pictures.get_or_insert_with(page_id(page), || picture_of(page));
    }
}

/// Adds sprite `name` to `atlas` while the game runs ([`Atlas::insert`]),
/// and writes it on its page's picture in `pictures` (the page whole, if
/// the picture isn't there: a new page), moving only that picture's
/// revision, so the page is uploaded again before the next frame.
pub fn insert(
    atlas: &mut Atlas,
    pictures: &mut PaintedTable,
    name: &'static str,
    image: &Image,
) -> Result<Placed, AtlasError> {
    let placed = atlas.insert(name, image)?;
    let page = &atlas.pages()[placed.page];
    let id = page_id(page);
    let Some(picture) = pictures.get_mut(id) else {
        pictures.insert(id, picture_of(page));
        return Ok(placed);
    };
    let stride = page.image().width as usize * 4;
    let row = placed.rect.w as usize * 4;
    let pixels = picture.pixels_mut();
    for y in placed.rect.y..placed.rect.y + placed.rect.h {
        let at = y as usize * stride + placed.rect.x as usize * 4;
        if let (Some(to), Some(from)) = (
            pixels.get_mut(at..at + row),
            page.image().rgba.get(at..at + row),
        ) {
            to.copy_from_slice(from);
        }
    }
    Ok(placed)
}

/// Sprite `name` of `atlas` drawn in all of `dest`, from its page's
/// picture, if the atlas has it.
pub fn sprite(atlas: &Atlas, name: &str, dest: PxRect, layer: Layer) -> Option<PaintedSprite> {
    let placed = atlas.get(name)?;
    let page = atlas.pages().get(placed.page)?;
    let r = placed.rect;
    let side = |v: u32| i32::try_from(v).unwrap_or(i32::MAX);
    let src = PxRect::new(side(r.x), side(r.y), side(r.w), side(r.h));
    Some(PaintedSprite::new(page_id(page), src, dest, layer))
}

#[cfg(test)]
mod tests {
    use shuyi_assets::atlas::AtlasBuilder;

    use super::*;

    fn hud() -> Atlas {
        let mut builder = AtlasBuilder::new("hud", 16).with_padding(1);
        builder
            .add("coin", &Image::filled(6, 6, [255, 200, 0, 255]))
            .unwrap();
        builder.build().unwrap()
    }

    #[test]
    fn a_page_is_a_picture_and_an_inserted_sprite_moves_only_its_page() {
        let mut atlas = hud();
        let mut pictures = PaintedTable::default();
        show(&atlas, &mut pictures);
        let id = PaintedId::new("hud#0");
        assert_eq!(pictures.get(id).map(Painted::revision), Some(1));
        // Shown again, nothing moves.
        show(&atlas, &mut pictures);
        assert_eq!(pictures.get(id).map(Painted::revision), Some(1));
        let gem = insert(
            &mut atlas,
            &mut pictures,
            "gem",
            &Image::filled(4, 4, [0, 0, 255, 255]),
        )
        .unwrap();
        assert_eq!(gem.page, 0);
        let picture = pictures.get(id).unwrap();
        assert_eq!(picture.revision(), 2);
        let (x, y) = (
            u16::try_from(gem.rect.x).unwrap(),
            u16::try_from(gem.rect.y).unwrap(),
        );
        assert_eq!(picture.rgba_at(x, y), [0, 0, 255, 255]);
        assert_eq!(picture.pixels(), atlas.pages()[0].image().rgba.as_slice());
        // Too big for page 0: a new page, a new picture.
        let big = insert(
            &mut atlas,
            &mut pictures,
            "big",
            &Image::filled(14, 14, [9, 9, 9, 255]),
        )
        .unwrap();
        assert_eq!(big.page, 1);
        assert_eq!(
            pictures.get(PaintedId::new("hud#1")).map(Painted::revision),
            Some(1)
        );
        assert_eq!(pictures.get(id).map(Painted::revision), Some(2));
    }

    #[test]
    fn a_sprite_is_its_pages_rectangle() {
        let atlas = hud();
        let dest = PxRect::new(3, 4, 12, 12);
        let sprite = sprite(&atlas, "coin", dest, Layer::Under).unwrap();
        assert_eq!(sprite.picture, PaintedId::new("hud#0"));
        assert_eq!(sprite.src, PxRect::new(1, 1, 6, 6));
        assert_eq!((sprite.dest, sprite.layer), (dest, Layer::Under));
        assert!(super::sprite(&atlas, "none", dest, Layer::Under).is_none());
    }
}
