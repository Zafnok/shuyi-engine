//! The textures of a game's images, without a window (ADR-0128 §5): none
//! at start, each made the first frame a sprite draws its image, from the
//! bytes the game's files give then. An image whose files give nothing
//! yet (on the web, its folder of the pack may not have arrived) is read
//! again the next frame that draws it; one whose bytes don't decode is
//! given up for the run, until a save under `--watch` makes it again.
//! Textures are never freed (ADR-0128 §7). Test builds count the uploads.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use shuyi_assets::image::ImageId;

/// Reads a game's file by path: its bytes, or `None` if it has none (yet).
pub(crate) type Read = Box<dyn Fn(&str) -> Option<Cow<'static, [u8]>>>;

/// Why an image has no texture this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Missing {
    /// Its files gave nothing yet: read again the next frame that draws it.
    NotRead,
    /// Its bytes don't decode: given up. `Some` (why) the first time only,
    /// to log it once.
    Undecodable(Option<String>),
}

/// The images' textures (`T` is macroquad's `Texture2D` in the renderer,
/// anything in the tests).
pub(crate) struct ImageTextures<T> {
    read: Read,
    textures: HashMap<ImageId, T>,
    /// The file of each texture of [`textures`](Self::textures), for the
    /// recoloured copies.
    sources: HashMap<ImageId, Cow<'static, [u8]>>,
    /// The images whose bytes don't decode.
    undecodable: HashSet<ImageId>,
    /// How many textures were made from what `read` gave.
    #[cfg(test)]
    uploads: u64,
}

impl<T> ImageTextures<T> {
    /// No textures yet; each image's bytes will come from `read`.
    pub(crate) fn new(read: Read) -> Self {
        Self {
            read,
            textures: HashMap::new(),
            sources: HashMap::new(),
            undecodable: HashSet::new(),
            #[cfg(test)]
            uploads: 0,
        }
    }

    /// The texture of `id`: the one made already, or one made now with
    /// `make` from the bytes its files give, or why it has none.
    pub(crate) fn texture(
        &mut self,
        id: ImageId,
        make: impl FnOnce(&[u8]) -> Result<T, String>,
    ) -> Result<&T, Missing> {
        if !self.textures.contains_key(&id) {
            if self.undecodable.contains(&id) {
                return Err(Missing::Undecodable(None));
            }
            let source = (self.read)(id.path()).ok_or(Missing::NotRead)?;
            #[cfg(test)]
            {
                self.uploads += 1;
            }
            match make(&source) {
                Ok(texture) => {
                    self.textures.insert(id, texture);
                    self.sources.insert(id, source);
                }
                Err(e) => {
                    self.undecodable.insert(id);
                    return Err(Missing::Undecodable(Some(e)));
                }
            }
        }
        // Made already, or just above.
        Ok(&self.textures[&id])
    }

    /// `id`'s texture, if it has one.
    pub(crate) fn get(&self, id: ImageId) -> Option<&T> {
        self.textures.get(&id)
    }

    /// The file `id`'s texture was made from, if it has one.
    pub(crate) fn source(&self, id: ImageId) -> Option<&[u8]> {
        self.sources.get(&id).map(AsRef::as_ref)
    }

    /// Puts `texture`, made from `source`, as `id`'s (a picture saved
    /// under `--watch`): one that didn't decode does now.
    pub(crate) fn replace(&mut self, id: ImageId, texture: T, source: Cow<'static, [u8]>) {
        self.textures.insert(id, texture);
        self.sources.insert(id, source);
        self.undecodable.remove(&id);
    }

    /// How many textures were made from what the files gave.
    #[cfg(test)]
    pub(crate) fn uploads(&self) -> u64 {
        self.uploads
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use shuyi_assets::image::{ImageInfo, ImageTable};

    use super::*;

    /// A picture's bytes in the tests: decodes unless it is `b"bad"`.
    fn make(bytes: &[u8]) -> Result<Vec<u8>, String> {
        if bytes == b"bad" {
            Err("not a picture".to_owned())
        } else {
            Ok(bytes.to_vec())
        }
    }

    /// Two images, `a.png` and `b.png`.
    fn two() -> (ImageId, ImageId) {
        let info = ImageInfo::new(1, 1);
        let table = ImageTable::new([("a.png", info), ("b.png", info)].into());
        (table.id("a.png").unwrap(), table.id("b.png").unwrap())
    }

    #[test]
    fn a_texture_is_made_the_first_frame_its_image_is_drawn_and_kept() {
        let (a, b) = two();
        let mut textures = ImageTextures::new(Box::new(|path| Some(Cow::Owned(path.into()))));
        // Frames 0 and 1 draw nothing: nothing is made.
        assert_eq!(textures.uploads(), 0);
        assert!(textures.get(a).is_none() && textures.get(b).is_none());
        // Frame 2 draws `a`, frames 3 to 5 draw it again.
        for _ in 2..6 {
            assert_eq!(textures.texture(a, make).unwrap(), b"a.png");
        }
        assert_eq!(textures.uploads(), 1);
        assert!(textures.get(a).is_some() && textures.get(b).is_none());
        assert_eq!(textures.source(a), Some(&b"a.png"[..]));
    }

    #[test]
    fn an_image_whose_bytes_arrive_at_frame_n_is_drawn_from_frame_n() {
        let (a, _) = two();
        let frame = Rc::new(Cell::new(0));
        let now = Rc::clone(&frame);
        let read: Read = Box::new(move |_| (now.get() >= 3).then_some(Cow::Borrowed(b"late")));
        let mut textures = ImageTextures::new(read);
        for n in 0..5 {
            frame.set(n);
            let drawn = textures.texture(a, make);
            if n < 3 {
                assert_eq!(drawn, Err(Missing::NotRead), "frame {n}");
            } else {
                assert_eq!(drawn.unwrap(), b"late", "frame {n}");
            }
        }
        assert_eq!(textures.uploads(), 1);
    }

    #[test]
    fn an_image_that_does_not_decode_is_given_up_and_reported_once() {
        let (a, _) = two();
        let reads = Rc::new(Cell::new(0));
        let count = Rc::clone(&reads);
        let read: Read = Box::new(move |_| {
            count.set(count.get() + 1);
            Some(Cow::Borrowed(b"bad"))
        });
        let mut textures = ImageTextures::new(read);
        assert_eq!(
            textures.texture(a, make),
            Err(Missing::Undecodable(Some("not a picture".to_owned())))
        );
        for _ in 0..3 {
            assert_eq!(textures.texture(a, make), Err(Missing::Undecodable(None)));
        }
        assert_eq!((reads.get(), textures.uploads()), (1, 1));
        // A save that decodes makes it again.
        textures.replace(a, b"fixed".to_vec(), Cow::Borrowed(b"fixed"));
        assert_eq!(textures.texture(a, make).unwrap(), b"fixed");
        assert_eq!(textures.source(a), Some(&b"fixed"[..]));
    }
}
