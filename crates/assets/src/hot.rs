//! Values that reload (ADR-0071). A [`Hot`] value is an asset decoded
//! through the registry by id, which remembers every file its decode asked
//! for: the file itself, the files its format read through [`DecodeCx`]
//! (an atlas's picture, a level's tileset) and its import settings, found
//! or not. When the platform says files changed, [`Hot::refresh`] decodes
//! it again if one of them did. If the new decode fails, the old value
//! stays and the errors come back ([`Reload::Failed`]), so a broken file
//! never takes a running game down.
//!
//! A `Hot` reads through to its value:
//!
//! ```
//! use std::collections::BTreeMap;
//! use std::sync::Arc;
//! use shuyi_assets::bundle::{Bundle, Layer};
//! use shuyi_assets::format::Formats;
//! use shuyi_assets::hot::{Hot, Reload};
//! use shuyi_assets::palette::PaletteDef;
//!
//! let palette = |ink: &str| format!("{{ \"ink\": \"{ink}\" }}").into_bytes();
//! let files = BTreeMap::from([("palette.ron".to_owned(), palette("#000000"))]);
//! let layer = Layer::owned("disk".to_owned(), files).unwrap();
//! let bundle = Bundle::new(vec![layer]);
//! let formats = Arc::new(Formats::standard());
//! let mut colors = Hot::<PaletteDef>::load(&formats, &bundle, "palette.ron").unwrap();
//!
//! assert_eq!(colors.refresh(&["other.ron".to_owned()]), Reload::Unchanged);
//! assert!(bundle.replace("palette.ron", palette("#ffffff")));
//! assert_eq!(colors.refresh(&["palette.ron".to_owned()]), Reload::Reloaded);
//! assert_eq!(colors.get("ink"), Some([255, 255, 255]));
//! ```
//!
//! [`DecodeCx`]: crate::format::DecodeCx

use crate::error::ContentError;
use crate::prelude::*;

/// What a [`Hot::refresh`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Reload {
    /// None of its files changed.
    Unchanged,
    /// A file changed, and the value is the new decode's.
    Reloaded,
    /// A file changed and the new decode failed: every problem, with its
    /// file and line. The value is still the old one.
    Failed(Vec<ContentError>),
}

#[cfg(feature = "std")]
pub use with_std::Hot;

#[cfg(feature = "std")]
mod with_std {
    use alloc::collections::BTreeSet;
    use alloc::sync::Arc;
    use core::fmt;
    use core::ops::Deref;

    use super::Reload;
    use crate::bundle::{Bundle, reads_of};
    use crate::error::ContentError;
    use crate::format::{AssetKind, Formats};
    use crate::prelude::*;

    /// An asset of kind `T` that decodes again when a file it read changes
    /// (the [module docs](super)).
    pub struct Hot<T: AssetKind> {
        value: T,
        /// What it was asked for by.
        id: String,
        formats: Arc<Formats>,
        bundle: Bundle,
        /// The files its decode asked for, found or not: one that isn't
        /// there yet (a `.meta` file) reloads it when it appears. After a
        /// failed decode, also those of the decode before.
        reads: BTreeSet<String>,
    }

    impl<T: AssetKind> Hot<T> {
        /// The asset `id` of `bundle`, read with `formats` as
        /// [`Formats::load`] reads it.
        ///
        /// # Errors
        ///
        /// Every problem [`Formats::load`] found.
        pub fn load(
            formats: &Arc<Formats>,
            bundle: &Bundle,
            id: &str,
        ) -> Result<Self, Vec<ContentError>> {
            let (value, reads) = reads_of(|| formats.load::<T>(bundle, id));
            Ok(Self {
                value: value?,
                id: id.to_owned(),
                formats: Arc::clone(formats),
                bundle: bundle.clone(),
                reads: reads.asked,
            })
        }

        /// What it was asked for by.
        pub fn id(&self) -> &str {
            &self.id
        }

        /// The files its decode asked for, sorted: a change to any of them
        /// reloads it.
        pub fn reads(&self) -> impl Iterator<Item = &str> {
            self.reads.iter().map(String::as_str)
        }

        /// Decodes it again if a file it read is among `changed` (paths in
        /// the bundle, as [`Bundle::bytes`] takes them). A value that fails
        /// keeps the old one, and tries again the next time one of its
        /// files changes.
        pub fn refresh(&mut self, changed: &[String]) -> Reload {
            if !changed.iter().any(|path| self.reads.contains(path)) {
                return Reload::Unchanged;
            }
            let (decoded, reads) = reads_of(|| self.formats.load::<T>(&self.bundle, &self.id));
            match decoded {
                Ok(value) => {
                    self.value = value;
                    self.reads = reads.asked;
                    Reload::Reloaded
                }
                Err(errors) => {
                    // A file the broken one names may be what is missing.
                    self.reads.extend(reads.asked);
                    Reload::Failed(errors)
                }
            }
        }
    }

    impl<T: AssetKind> Deref for Hot<T> {
        type Target = T;

        fn deref(&self) -> &T {
            &self.value
        }
    }

    impl<T: AssetKind + fmt::Debug> fmt::Debug for Hot<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Hot")
                .field("id", &self.id)
                .field("value", &self.value)
                .field("reads", &self.reads)
                .finish_non_exhaustive()
        }
    }
}
