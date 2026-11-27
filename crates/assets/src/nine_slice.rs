//! A picture drawn in nine slices: its four corners kept,
//! its four edges and its middle stretched or repeated to fill a
//! rectangle of any size. Every 2D engine has one (Godot's
//! `NinePatchRect`, Unity's 9-sliced sprites):
//! most of a 2D game's windows, menus and name plates are drawn from one
//! small picture this way.
//!
//! These are the plain values; `shuyi_ui::frame::slice` turns one into
//! sprites, and a panel wears one as its frame
//! (`shuyi_ui::frame::draw::PanelFrame`).

use crate::image::Picture;

/// How an edge between two corners fills its length.
///
/// Kind 6 of ADR-0039: more ways may come, so a `match` on it has a `_` arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum EdgeFill {
    /// One copy of the edge, stretched along it.
    #[default]
    Stretch,
    /// The edge repeated from the corner's inner side; the last copy is
    /// cut, never squeezed.
    Tile,
}

/// How the middle of a nine-slice fills what the borders leave.
///
/// Kind 6 of ADR-0039: more ways may come, so a `match` on it has a `_` arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum CentreFill {
    /// One copy, stretched both ways.
    #[default]
    Stretch,
    /// Repeated from its top-left corner; the last row and column are cut.
    Tile,
    /// Not drawn: what is under it shows.
    Empty,
}

/// How far a nine-slice's corners reach into its picture, in the
/// picture's pixels: the left corners are `left` wide, the top ones `top`
/// tall, and so on. A plain value (kind 1 of ADR-0039): its fields name
/// the four numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Borders {
    /// The left corners' and the left edge's width.
    pub left: u32,
    /// The top corners' and the top edge's height.
    pub top: u32,
    /// The right corners' and the right edge's width.
    pub right: u32,
    /// The bottom corners' and the bottom edge's height.
    pub bottom: u32,
}

impl Borders {
    /// The same border on every side.
    pub const fn uniform(px: u32) -> Self {
        Self {
            left: px,
            top: px,
            right: px,
            bottom: px,
        }
    }

    /// `left` and `right` cut so that together they are at most `w`
    /// (`right` first, then `left`), and `top` and `bottom` likewise to
    /// `h`: borders that fit a picture of `w × h`.
    #[must_use]
    pub const fn fitted(self, w: u32, h: u32) -> Self {
        let right = if self.right < w { self.right } else { w };
        let left = if self.left < w - right {
            self.left
        } else {
            w - right
        };
        let bottom = if self.bottom < h { self.bottom } else { h };
        let top = if self.top < h - bottom {
            self.top
        } else {
            h - bottom
        };
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
}

/// A picture drawn in nine slices: its [`Borders`] in the picture's
/// pixels, how its edges and middle fill, and how many console pixels one
/// picture pixel is (`zoom`; 0 counts as 1).
///
/// Kind 2 of ADR-0039: build it with [`NineSlice::new`] and its `with_`
/// methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct NineSlice {
    /// The picture (an image and a rectangle of it).
    pub picture: Picture,
    /// Where its corners end.
    pub borders: Borders,
    /// How its edges fill.
    pub edges: EdgeFill,
    /// How its middle fills.
    pub centre: CentreFill,
    /// Console pixels per picture pixel.
    pub zoom: u8,
}

impl NineSlice {
    /// `picture` cut at `borders`, its edges and middle stretched, at one
    /// console pixel per picture pixel.
    pub const fn new(picture: Picture, borders: Borders) -> Self {
        Self {
            picture,
            borders,
            edges: EdgeFill::Stretch,
            centre: CentreFill::Stretch,
            zoom: 1,
        }
    }

    /// This slice with its edges filled as `edges`.
    #[must_use]
    pub const fn with_edges(self, edges: EdgeFill) -> Self {
        Self { edges, ..self }
    }

    /// This slice with its middle filled as `centre`.
    #[must_use]
    pub const fn with_centre(self, centre: CentreFill) -> Self {
        Self { centre, ..self }
    }

    /// This slice drawn at `zoom` console pixels per picture pixel.
    #[must_use]
    pub const fn with_zoom(self, zoom: u8) -> Self {
        Self { zoom, ..self }
    }

    /// The zoom it is drawn at: [`zoom`](Self::zoom), 0 counting as 1.
    pub const fn scale(&self) -> u32 {
        if self.zoom == 0 { 1 } else { self.zoom as u32 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fitted_borders_cut_right_then_left() {
        let b = Borders {
            left: 10,
            top: 3,
            right: 12,
            bottom: 2,
        };
        assert_eq!(b.fitted(16, 8), Borders { left: 4, ..b });
        assert_eq!(
            b.fitted(8, 4),
            Borders {
                left: 0,
                top: 2,
                right: 8,
                bottom: 2
            }
        );
        assert_eq!(b.fitted(30, 30), b);
        assert_eq!(Borders::uniform(5).fitted(0, 0), Borders::default());
    }

    #[test]
    fn zoom_zero_counts_as_one() {
        let table =
            crate::image::ImageTable::new([("f.png", crate::image::ImageInfo::new(9, 9))].into());
        let picture = Picture {
            image: table.id("f.png").unwrap(),
            rect: crate::image::ImageRect {
                x: 0,
                y: 0,
                w: 9,
                h: 9,
            },
        };
        let slice = NineSlice::new(picture, Borders::uniform(3));
        assert_eq!(slice.scale(), 1);
        assert_eq!(slice.with_zoom(0).scale(), 1);
        assert_eq!(slice.with_zoom(3).scale(), 3);
        assert_eq!(
            slice.with_centre(CentreFill::Empty).centre,
            CentreFill::Empty
        );
        assert_eq!(slice.with_edges(EdgeFill::Tile).edges, EdgeFill::Tile);
    }
}
