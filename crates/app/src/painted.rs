//! The textures of the pictures a skin paints at run time (ADR-0078),
//! without a window: one per [`PaintedId`], made the first frame the
//! picture is drawn, updated only in a frame whose revision differs from
//! the one uploaded (made again when its size changed), and dropped at the
//! end of a frame that didn't draw it. An upload takes the picture's
//! pixels as RGBA ([`Painted::rgba_into`]: an indexed picture through its
//! palette). Test builds count the uploads.

use std::collections::{HashMap, HashSet};

use shuyi_ui::frame::painted::{Painted, PaintedId};

/// A picture's texture and what it was made from.
struct Uploaded<T> {
    revision: u32,
    size: (u16, u16),
    texture: T,
}

/// The painted pictures' textures (`T` is macroquad's `Texture2D` in the
/// renderer, anything in the tests).
pub(crate) struct PaintedTextures<T> {
    textures: HashMap<PaintedId, Uploaded<T>>,
    /// Those drawn since the last [`end_frame`](Self::end_frame).
    drawn: HashSet<PaintedId>,
    /// The last upload's RGBA, its room kept for the next.
    rgba: Vec<u8>,
    /// How many times a texture was made or updated.
    #[cfg(test)]
    uploads: u64,
}

impl<T> PaintedTextures<T> {
    /// No textures.
    pub(crate) fn new() -> Self {
        Self {
            textures: HashMap::new(),
            drawn: HashSet::new(),
            rgba: Vec::new(),
            #[cfg(test)]
            uploads: 0,
        }
    }

    /// The texture of `picture`, the frame's picture `id`: the one there
    /// if it was made from `picture`'s revision and size; else updated
    /// with `update` if the size is the same, or made with `make`, each
    /// given the picture's pixels as RGBA.
    pub(crate) fn texture(
        &mut self,
        id: PaintedId,
        picture: &Painted,
        make: impl FnOnce(&[u8]) -> T,
        update: impl FnOnce(&T, &[u8]),
    ) -> &T {
        self.drawn.insert(id);
        let (revision, size) = (picture.revision(), picture.size());
        let stale = self
            .textures
            .get(&id)
            .is_none_or(|old| old.revision != revision || old.size != size);
        if stale {
            #[cfg(test)]
            {
                self.uploads += 1;
            }
            picture.rgba_into(&mut self.rgba);
            match self.textures.get_mut(&id) {
                Some(old) if old.size == size => {
                    update(&old.texture, &self.rgba);
                    old.revision = revision;
                }
                _ => {
                    let texture = make(&self.rgba);
                    self.textures.insert(
                        id,
                        Uploaded {
                            revision,
                            size,
                            texture,
                        },
                    );
                }
            }
        }
        // Inserted or kept just above.
        &self.textures[&id].texture
    }

    /// Drops the textures of the pictures not drawn since the last call:
    /// the frame no longer has them.
    pub(crate) fn end_frame(&mut self) {
        let drawn = std::mem::take(&mut self.drawn);
        self.textures.retain(|id, _| drawn.contains(id));
    }

    /// How many textures it holds.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.textures.len()
    }

    /// How many times a texture was made or updated.
    #[cfg(test)]
    pub(crate) fn uploads(&self) -> u64 {
        self.uploads
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use shuyi_ui::frame::Rgb;

    use super::*;

    const PLASMA: PaintedId = PaintedId::new("plasma");

    /// A texture in the tests: the bytes it was made or updated from, as
    /// RGBA.
    struct Fake {
        rgba: std::cell::RefCell<Vec<u8>>,
    }

    /// Draws `picture` as the frame's `id` and ends the frame, counting
    /// in `makes` the textures made; the texture's bytes after.
    fn frame(
        textures: &mut PaintedTextures<Fake>,
        picture: &Painted,
        makes: &Cell<u32>,
    ) -> Vec<u8> {
        let make = |rgba: &[u8]| {
            makes.set(makes.get() + 1);
            Fake {
                rgba: std::cell::RefCell::new(rgba.to_vec()),
            }
        };
        let update = |t: &Fake, rgba: &[u8]| *t.rgba.borrow_mut() = rgba.to_vec();
        let bytes = textures
            .texture(PLASMA, picture, make, update)
            .rgba
            .borrow()
            .clone();
        textures.end_frame();
        bytes
    }

    #[test]
    fn ten_frames_with_three_changes_upload_three_times() {
        let mut textures = PaintedTextures::new();
        let made = Cell::new(0);
        let mut picture = Painted::rgba(64, 48);
        for n in 0..10_u8 {
            // Painted in frames 0 (its first), 4 and 7 only.
            if [0, 4, 7].contains(&n) {
                picture.pixels_mut()[0] = n;
            }
            let bytes = frame(&mut textures, &picture, &made);
            assert_eq!(bytes[0], picture.pixels()[0], "frame {n}");
        }
        assert_eq!(textures.uploads(), 3);
        // Made once, updated twice.
        assert_eq!(made.get(), 1);
    }

    #[test]
    fn a_palette_change_is_one_upload_through_the_palette() {
        let mut textures = PaintedTextures::new();
        let made = Cell::new(0);
        let mut picture = Painted::indexed(2, 1, &[Rgb::new(1, 2, 3)]);
        frame(&mut textures, &picture, &made);
        frame(&mut textures, &picture, &made);
        picture.palette_mut()[0] = Rgb::new(200, 0, 0);
        let bytes = frame(&mut textures, &picture, &made);
        assert_eq!(textures.uploads(), 2);
        assert_eq!(bytes, [200, 0, 0, 255, 200, 0, 0, 255]);
    }

    #[test]
    fn a_new_size_is_made_again_and_a_picture_not_drawn_is_dropped() {
        let mut textures = PaintedTextures::new();
        let made = Cell::new(0);
        frame(&mut textures, &Painted::rgba(4, 4), &made);
        // Another picture of the same revision but another size.
        frame(&mut textures, &Painted::rgba(8, 4), &made);
        assert_eq!((textures.uploads(), made.get()), (2, 2));
        // A frame without it drops its texture; it comes back made again.
        textures.end_frame();
        assert_eq!(textures.len(), 0);
        frame(&mut textures, &Painted::rgba(8, 4), &made);
        assert_eq!((textures.uploads(), made.get()), (3, 3));
    }
}
