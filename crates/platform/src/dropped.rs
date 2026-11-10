//! Files the player dropped on the game's window or canvas
//! ([`DroppedFile`]): what a backend hands the game in a
//! [`SystemEvent::FilesDropped`](crate::SystemEvent::FilesDropped), checked
//! against the limits ([`accept`]), and the bundle layer a game makes of
//! them ([`DroppedFile::layer`]).
//!
//! Where drops work is a fact of the platform
//! ([`PlatformFacts::can_drop_files`](shuyi_ui::PlatformFacts::can_drop_files)): a
//! screen offers "drop a file here" only there.

use crate::alloc_prelude::*;
use alloc::collections::BTreeMap;
use alloc::{format, vec};
use core::fmt;

use serde::{Deserialize, Serialize};
use shuyi_assets::bundle::{Layer, LayerError};

/// The most bytes one dropped file may have: a bigger one is refused
/// ([`accept`]).
pub const MAX_DROPPED_BYTES: usize = 256 * 1024 * 1024;

/// The most files one drop may hold: a drop of more is refused whole
/// ([`accept`]).
pub const MAX_DROPPED_FILES: usize = 16;

/// The name of the bundle layer [`DroppedFile::layer`] makes.
pub const DROPPED_LAYER: &str = "dropped";

/// One file the player dropped on the game's window: its name, never a
/// path, and its bytes.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
// Built with `DroppedFile::new`: it may grow (ADR-0039).
#[non_exhaustive]
pub struct DroppedFile {
    name: String,
    bytes: Vec<u8>,
}

impl DroppedFile {
    /// The file dropped from `path` (its whole path as the system gave it,
    /// or only its name), holding `bytes`. Only the name is kept: what
    /// follows the last `/` or `\`.
    pub fn new(path: &str, bytes: Vec<u8>) -> Self {
        let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
        Self {
            name: name.to_owned(),
            bytes,
        }
    }

    /// The file's name, without the folders it was dropped from
    /// (`doom1.wad`).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Its bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Its bytes, kept without a copy.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// A bundle layer named [`DROPPED_LAYER`] holding `files`, each at its
    /// name, at the bundle's root: a game lays it over its own files
    /// ([`Bundle::new`](shuyi_assets::bundle::Bundle::new)) to read a
    /// player's file as one of its own. Of two files of one name, the
    /// later is kept.
    ///
    /// ```
    /// use shuyi_assets::bundle::{Bundle, Layer};
    /// use shuyi_platform::dropped::DroppedFile;
    ///
    /// static BASE: &[(&str, &[u8])] = &[("level.txt", b"the game's")];
    /// let dropped = DroppedFile::new("C:\\Users\\me\\level.txt", b"mine".to_vec());
    /// let layer = DroppedFile::layer([dropped]).unwrap();
    /// let bundle = Bundle::new(vec![layer, Layer::table("base", BASE)]);
    /// assert_eq!(bundle.file("level.txt"), Some("mine"));
    /// ```
    ///
    /// # Errors
    ///
    /// A name a bundle can't hold (empty, `.` or `..`), naming it
    /// ([`Layer::owned`]).
    pub fn layer(files: impl IntoIterator<Item = DroppedFile>) -> Result<Layer, LayerError> {
        let files: BTreeMap<String, Vec<u8>> =
            files.into_iter().map(|f| (f.name, f.bytes)).collect();
        Layer::owned(DROPPED_LAYER.to_owned(), files)
    }
}

impl fmt::Debug for DroppedFile {
    /// Its name and its size, never its bytes (a WAD is megabytes).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DroppedFile")
            .field("name", &self.name)
            .field("bytes", &self.bytes.len())
            .finish()
    }
}

/// The files of one drop the game may have, and why the others were
/// refused: a drop of more than [`MAX_DROPPED_FILES`] is refused whole,
/// and a file of more than [`MAX_DROPPED_BYTES`] alone. A backend calls it
/// before handing a drop on, and logs each reason.
pub fn accept(files: Vec<DroppedFile>) -> (Vec<DroppedFile>, Vec<String>) {
    if files.len() > MAX_DROPPED_FILES {
        let why = format!(
            "a drop of {} files was refused: at most {MAX_DROPPED_FILES} at once",
            files.len()
        );
        return (Vec::new(), vec![why]);
    }
    let mut refused = Vec::new();
    let kept = files
        .into_iter()
        .filter(|file| {
            let fits = file.bytes.len() <= MAX_DROPPED_BYTES;
            if !fits {
                refused.push(format!(
                    "the dropped file {} was refused: {} bytes, more than {MAX_DROPPED_BYTES}",
                    file.name,
                    file.bytes.len()
                ));
            }
            fits
        })
        .collect();
    (kept, refused)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dropped_file_keeps_its_name_and_never_its_path() {
        for path in [
            "doom1.wad",
            "/home/me/doom1.wad",
            "C:\\Users\\me\\doom1.wad",
            "C:/mixed\\path/doom1.wad",
        ] {
            assert_eq!(DroppedFile::new(path, Vec::new()).name(), "doom1.wad");
        }
        let file = DroppedFile::new("a.txt", b"hi".to_vec());
        assert_eq!(file.bytes(), b"hi");
        assert_eq!(file.into_bytes(), b"hi");
    }

    #[test]
    fn its_debug_says_the_size_not_the_bytes() {
        let shown = format!("{:?}", DroppedFile::new("a.bin", vec![7; 1000]));
        assert_eq!(shown, r#"DroppedFile { name: "a.bin", bytes: 1000 }"#);
    }

    #[test]
    fn too_many_files_are_refused_whole() {
        let files = |n| (0..n).map(|i| DroppedFile::new(&format!("{i}"), Vec::new()));
        let (kept, refused) = accept(files(MAX_DROPPED_FILES).collect());
        assert_eq!((kept.len(), refused.len()), (MAX_DROPPED_FILES, 0));
        let (kept, refused) = accept(files(MAX_DROPPED_FILES + 1).collect());
        assert_eq!(kept, []);
        assert_eq!(
            refused,
            ["a drop of 17 files was refused: at most 16 at once"]
        );
    }

    #[test]
    fn a_file_too_big_is_refused_alone() {
        let big = DroppedFile::new("big.wad", vec![0; MAX_DROPPED_BYTES + 1]);
        let fits = DroppedFile::new("fits.wad", vec![0; 3]);
        let (kept, refused) = accept(vec![big, fits.clone()]);
        assert_eq!(kept, [fits]);
        assert_eq!(refused.len(), 1);
        assert!(refused[0].starts_with("the dropped file big.wad was refused"));
    }

    #[test]
    fn the_layer_holds_each_file_at_its_name() {
        let layer = DroppedFile::layer([
            DroppedFile::new("/a/one.txt", b"first".to_vec()),
            DroppedFile::new("two.txt", b"2".to_vec()),
            DroppedFile::new("/b/one.txt", b"1".to_vec()),
        ])
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(layer.name, DROPPED_LAYER);
        let bundle = shuyi_assets::bundle::Bundle::new(vec![layer]);
        assert_eq!(bundle.file("one.txt"), Some("1"));
        assert_eq!(bundle.file("two.txt"), Some("2"));
    }

    #[test]
    fn a_name_a_bundle_cant_hold_is_an_error() {
        for path in ["", "folder/", ".."] {
            let error = DroppedFile::layer([DroppedFile::new(path, Vec::new())]);
            assert!(error.is_err(), "{path:?}");
        }
    }
}
