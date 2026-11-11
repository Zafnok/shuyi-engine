//! `glyph-tiles`: a tileset made from the font's glyphs, for a game that
//! wants a sprite skin before it has art (as the game's `test-tileset`
//! does): each tile is a square of a palette color with up to a few glyphs
//! stamped on it, centred, in another; a tile with no background is clear
//! but for its glyphs (a piece drawn over the ground). Tiles are laid out
//! in the order `shuyi.ron` lists them, `columns` to a row, so tile `i` is
//! at column `i % columns`, row `i / columns`.
//!
//! The same font, palette and list always give the same pixels: generated
//! from the game's own data, not art.

use std::fs;
use std::path::Path;

use shuyi_assets::font::FontAtlasDef;
use shuyi_assets::image::Image;
use shuyi_assets::palette::PaletteDef;

use crate::codec::{decode_png, encode_png};
use crate::project::GlyphTiles;

/// The most glyphs one tile may have.
pub const MAX_GLYPHS: usize = 3;

/// The pixel of the top-left corner of tile `index` of `rules`.
pub fn tile_origin(rules: &GlyphTiles, index: usize) -> (u32, u32) {
    let i = u32::try_from(index).unwrap_or(u32::MAX);
    let columns = rules.columns.max(1);
    (i % columns * rules.tile, i / columns * rules.tile)
}

/// Draws the tileset `rules` describes with the font atlas `atlas` (its
/// picture `atlas_image`) and the colors of
/// `palette`. Fails on a color the palette lacks, a glyph the atlas lacks,
/// or a tile of no glyphs, too many, or too many to fit.
pub fn render(
    rules: &GlyphTiles,
    atlas: &FontAtlasDef,
    atlas_image: &Image,
    palette: &PaletteDef,
) -> Result<Image, String> {
    if rules.tile == 0 || rules.columns == 0 {
        return Err("tile and columns must be above 0".to_owned());
    }
    let count = u32::try_from(rules.tiles.len()).unwrap_or(u32::MAX);
    let width = rules.columns * rules.tile;
    let height = count.div_ceil(rules.columns).max(1) * rules.tile;
    let mut out = Image::filled(width, height, [0; 4]);
    let color = |name: &str| {
        palette
            .get(name)
            .ok_or_else(|| format!("the palette has no color \"{name}\""))
    };
    for (index, tile) in rules.tiles.iter().enumerate() {
        let glyphs: Vec<char> = tile.glyphs.chars().collect();
        let wide = u32::try_from(glyphs.len()).unwrap_or(u32::MAX) * atlas.cell_w;
        if glyphs.is_empty() || glyphs.len() > MAX_GLYPHS {
            return Err(format!(
                "tile {index}: {} glyphs, not 1 to {MAX_GLYPHS}",
                glyphs.len()
            ));
        }
        if wide > rules.tile || atlas.cell_h > rules.tile {
            return Err(format!(
                "tile {index}: \"{}\" doesn't fit a {} px tile",
                tile.glyphs, rules.tile
            ));
        }
        let (x, y) = tile_origin(rules, index);
        if let Some(bg) = &tile.bg {
            let [red, green, blue] = color(bg)?;
            for py in y..y + rules.tile {
                for px in x..x + rules.tile {
                    set(&mut out, px, py, [red, green, blue, 255]);
                }
            }
        }
        let fg = color(&tile.fg)?;
        let left = x + (rules.tile - wide) / 2;
        let top = y + (rules.tile - atlas.cell_h) / 2;
        for (i, glyph) in (0..).zip(glyphs) {
            let cell = atlas
                .glyph_rect(glyph)
                .ok_or_else(|| format!("tile {index}: the font has no '{glyph}'"))?;
            for gy in 0..cell.h {
                for gx in 0..cell.w {
                    let cover = atlas_image
                        .pixel(cell.x + gx, cell.y + gy)
                        .map_or(0, |[.., alpha]| alpha);
                    if cover > 0 {
                        stamp(&mut out, left + i * atlas.cell_w + gx, top + gy, fg, cover);
                    }
                }
            }
        }
    }
    Ok(out)
}

fn index(image: &Image, x: u32, y: u32) -> Option<usize> {
    (x < image.width && y < image.height)
        .then(|| to_usize((u64::from(y) * u64::from(image.width) + u64::from(x)) * 4))
}

fn set(image: &mut Image, x: u32, y: u32, rgba: [u8; 4]) {
    if let Some(at) = index(image, x, y) {
        image.rgba[at..at + 4].copy_from_slice(&rgba);
    }
}

/// Blends `(x, y)` of `image` towards `color` by `cover` (0 none, 255
/// all). A clear pixel takes the color at that cover as its opacity.
fn stamp(image: &mut Image, x: u32, y: u32, color: [u8; 3], cover: u8) {
    let under = image.pixel(x, y).unwrap_or([0; 4]);
    let out = if under[3] == 0 {
        [color[0], color[1], color[2], cover]
    } else {
        let mix = |c: usize| blend(under[c], color[c], cover);
        [mix(0), mix(1), mix(2), under[3]]
    };
    set(image, x, y, out);
}

/// Channel `under` blended towards `over` by `cover`, rounded.
fn blend(under: u8, over: u8, cover: u8) -> u8 {
    let cover = u32::from(cover);
    let v = (u32::from(under) * (255 - cover) + u32::from(over) * cover + 127) / 255;
    u8::try_from(v).unwrap_or(u8::MAX)
}

fn to_usize(v: u64) -> usize {
    usize::try_from(v).unwrap_or(usize::MAX)
}

/// The tileset of `rules`, read from the files they name under `root`.
pub fn render_files(rules: &GlyphTiles, root: &Path) -> Result<Image, String> {
    let read = |rel: &str| {
        let path = root.join(rel);
        fs::read(&path).map_err(|e| format!("reading {}: {e}", path.display()))
    };
    let text = |rel: &str| {
        let bytes = read(rel)?;
        String::from_utf8(bytes).map_err(|e| format!("{rel}: {e}"))
    };
    let atlas_ron = format!("{}/atlas.ron", rules.font);
    let atlas_png = format!("{}/atlas.png", rules.font);
    let png = read(&atlas_png)?;
    let joined = |errors: Vec<shuyi_assets::ContentError>| {
        let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
        errors.join("\n")
    };
    let atlas =
        FontAtlasDef::from_source(&atlas_ron, &text(&atlas_ron)?, &atlas_png, Some(&png), "")
            .map_err(joined)?;
    let (width, height, rgba) = decode_png(&png)?;
    let atlas_image = Image {
        width,
        height,
        rgba,
    };
    let palette =
        PaletteDef::from_source(&rules.palette, &text(&rules.palette)?, &[]).map_err(joined)?;
    render(rules, &atlas, &atlas_image, &palette)
}

/// Runs the command: draws the tileset and writes it to `rules.out` under
/// `root`; returns what to print.
pub fn run(rules: &GlyphTiles, root: &Path) -> Result<String, String> {
    let tiles = render_files(rules, root)?;
    let png = encode_png(tiles.width, tiles.height, &tiles.rgba)?;
    let path = root.join(&rules.out);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    }
    fs::write(&path, png).map_err(|e| format!("writing {}: {e}", path.display()))?;
    Ok(format!(
        "glyph-tiles: {} tiles, {}×{} px → {}",
        rules.tiles.len(),
        tiles.width,
        tiles.height,
        rules.out
    ))
}

#[cfg(test)]
mod tests;
