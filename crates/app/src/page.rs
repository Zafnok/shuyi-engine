//! Files from the page, on the web: the text files the page a web build runs
//! in holds, read into a bundle layer (ADR-0004 §1) the game lays over its own
//! files, and the game's problems with them sent back ([`report`]). It is how
//! a page that edits a game's data beside it (the site's playground) runs the
//! game on its edits with no install. On every other target the page holds
//! none.
//!
//! The page writes each file to `sessionStorage` under
//! `shuyi-page-file:<path>`, `<path>` being the file's path in the game's
//! project (`files/stage.map`); [`page_layer`] takes those under one folder.
//! The `shuyi_page_files` miniquad plugin in `crates/cli/web/page-files.js`
//! (our own code, loaded by `crates/cli/web/index.html`) reads them.

use std::collections::BTreeMap;

use shuyi_assets::bundle::{Layer, LayerError};

/// The plugin's version, which the JS loader compares with the `version`
/// in `crates/cli/web/page-files.js` (a `shuyi-cli` test keeps them equal).
/// Bump both when the functions below change.
#[cfg(target_arch = "wasm32")]
const PLUGIN_VERSION: u32 = 1;

// Declaring the functions `crates/cli/web/page-files.js` provides. They take
// numbers and strings as `sapp_jsutils.js`'s objects, and the JS answers ""
// for a file that isn't there, so calling them is safe.
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code)]
unsafe extern "C" {
    /// Reads the keys of the page's files; returns how many. The other
    /// functions index into that list until the next call.
    safe fn shuyi_page_file_count() -> u32;
    /// File `i`'s path, after the key's prefix.
    safe fn shuyi_page_file_path(i: u32) -> sapp_jsutils::JsObject;
    /// File `i`'s text.
    safe fn shuyi_page_file_text(i: u32) -> sapp_jsutils::JsObject;
    /// Sends the page `text`: to its parent frame, of its origin, or else
    /// to the console.
    safe fn shuyi_page_report(text: sapp_jsutils::JsObjectWeak);
}

/// What the JS loader calls to check the plugin's version.
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn shuyi_page_files_crate_version() -> u32 {
    PLUGIN_VERSION
}

/// How big the page's files may be ([`page_layer`]). Build it with
/// [`PageLimits::new`] or take the [default](PageLimits::default): a struct
/// literal doesn't compile outside this crate, so a new field never breaks a
/// game (ADR-0039).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PageLimits {
    /// The most one file may hold, in bytes of UTF-8.
    pub per_file: u64,
    /// The most the layer's files may hold together, in bytes of UTF-8.
    pub in_all: u64,
}

impl PageLimits {
    /// At most `per_file` bytes a file and `in_all` bytes together.
    #[must_use]
    pub const fn new(per_file: u64, in_all: u64) -> Self {
        Self { per_file, in_all }
    }
}

impl Default for PageLimits {
    /// 256 KiB a file and 1 MiB together: a game's data files, with room.
    fn default() -> Self {
        Self::new(256 * 1024, 1024 * 1024)
    }
}

/// Why the page's files could not be read into a layer ([`page_layer`]).
#[derive(Debug)]
#[non_exhaustive]
pub enum PageError {
    /// A file over the limit.
    TooBig {
        /// The file, as the page names it (`files/stage.map`).
        path: String,
        /// The limit, in bytes.
        limit: u64,
    },
    /// The files together over the limit.
    TooMuch {
        /// The limit, in bytes.
        limit: u64,
    },
    /// A path the layer refused (`files/../x`, `files/a\b`).
    Layer(LayerError),
}

impl core::fmt::Display for PageError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooBig { path, limit } => write!(f, "{path}: the file is over {limit} bytes"),
            Self::TooMuch { limit } => {
                write!(f, "the page's files are over {limit} bytes in all")
            }
            Self::Layer(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for PageError {}

/// The page's files under the folder `name` (`files`), as one layer named
/// `name`, each at its path after the folder: `files/stage.map` is the
/// layer's `stage.map`, and the bundle shows it as `files/stage.map`, as
/// `cargo shuyi check` names the game's own. With `name` empty, every file
/// of the page, at its path. `None` when the page holds none there, and on
/// every target but the web. Read it once, at the start: the layer's files
/// are kept for the life of the program, and the page runs the game again to
/// change them.
///
/// # Errors
///
/// A file over `limits.per_file`, naming it; the files over `limits.in_all`
/// together; a path the layer refuses.
pub fn page_layer(name: &str, limits: PageLimits) -> Result<Option<Layer>, PageError> {
    layer_from(name, page_files(), limits)
}

/// Every file of the page, path and text, in key order.
#[cfg(target_arch = "wasm32")]
fn page_files() -> Vec<(String, String)> {
    let read = |object: sapp_jsutils::JsObject| {
        let mut text = String::new();
        object.to_string(&mut text);
        text
    };
    (0..shuyi_page_file_count())
        .map(|i| (read(shuyi_page_file_path(i)), read(shuyi_page_file_text(i))))
        .collect()
}

/// Only a web page holds files.
#[cfg(not(target_arch = "wasm32"))]
fn page_files() -> Vec<(String, String)> {
    Vec::new()
}

/// Sends the page `text`: the game's problems with the page's files, one a
/// line, each with its file and line as `cargo shuyi check` prints them; the
/// empty text when they load. The page's parent frame gets it as a
/// `{ kind: "shuyi-report", text }` message when it has the page's origin,
/// and the console otherwise. On every target but the web it does nothing.
pub fn report(text: &str) {
    #[cfg(target_arch = "wasm32")]
    shuyi_page_report(sapp_jsutils::JsObject::string(text).weak());
    #[cfg(not(target_arch = "wasm32"))]
    let _ = text;
}

/// The layer [`page_layer`] makes of `entries` (each a path the page names
/// and its text): the work, which every target compiles and tests.
fn layer_from(
    name: &str,
    entries: Vec<(String, String)>,
    limits: PageLimits,
) -> Result<Option<Layer>, PageError> {
    let folder = if name.is_empty() {
        String::new()
    } else {
        format!("{name}/")
    };
    let mut files = BTreeMap::new();
    let mut total: u64 = 0;
    for (path, text) in entries {
        let Some(inner) = path.strip_prefix(&folder) else {
            continue;
        };
        let size = u64::try_from(text.len()).unwrap_or(u64::MAX);
        if size > limits.per_file {
            return Err(PageError::TooBig {
                path,
                limit: limits.per_file,
            });
        }
        total = total.saturating_add(size);
        if total > limits.in_all {
            return Err(PageError::TooMuch {
                limit: limits.in_all,
            });
        }
        files.insert(inner.to_owned(), text.into_bytes());
    }
    if files.is_empty() {
        return Ok(None);
    }
    Layer::owned(name.to_owned(), files)
        .map(Some)
        .map_err(PageError::Layer)
}

#[cfg(test)]
mod tests {
    use shuyi_assets::bundle::{BadPath, Bundle};

    use super::*;

    /// The game's own files, which the page's are laid over.
    static GAME: &[(&str, &[u8])] = &[("en.ron", b"text"), ("stage.map", b"the game's map")];

    /// `entries` as the page hands them.
    fn page(entries: &[(&str, &str)]) -> Vec<(String, String)> {
        entries
            .iter()
            .map(|&(path, text)| (path.to_owned(), text.to_owned()))
            .collect()
    }

    /// `page`'s layer laid over the game's files.
    fn over(page: Layer) -> Bundle {
        Bundle::new(vec![page, Layer::table("files", GAME)])
    }

    #[test]
    fn no_files_is_no_layer() {
        let layer = layer_from("files", Vec::new(), PageLimits::default()).unwrap();
        assert!(layer.is_none());
    }

    #[test]
    fn a_file_of_the_page_is_read_over_the_games_own() {
        let entries = page(&[("files/stage.map", "the page's map")]);
        let layer = layer_from("files", entries, PageLimits::default())
            .unwrap()
            .unwrap();
        let bundle = over(layer);
        assert_eq!(bundle.file("stage.map"), Some("the page's map"));
        assert_eq!(bundle.display_path("stage.map"), "files/stage.map");
        assert_eq!(bundle.file("en.ron"), Some("text"));
    }

    #[test]
    fn a_new_file_of_the_page_is_read_beside_the_games() {
        let entries = page(&[("files/more/new.ron", "new")]);
        let layer = layer_from("files", entries, PageLimits::default())
            .unwrap()
            .unwrap();
        let bundle = over(layer);
        assert_eq!(bundle.file("more/new.ron"), Some("new"));
        assert_eq!(bundle.file("stage.map"), Some("the game's map"));
    }

    #[test]
    fn files_outside_the_folder_are_not_the_layers() {
        let entries = page(&[("other/stage.map", "not ours"), ("files", "nor this")]);
        let layer = layer_from("files", entries, PageLimits::default()).unwrap();
        assert!(layer.is_none());
        // With no folder, every file at its path.
        let entries = page(&[("other/stage.map", "all of them")]);
        let layer = layer_from("", entries, PageLimits::default())
            .unwrap()
            .unwrap();
        assert_eq!(
            Bundle::new(vec![layer]).file("other/stage.map"),
            Some("all of them")
        );
    }

    #[test]
    fn a_file_at_the_limit_is_read_and_one_over_it_is_refused() {
        let limits = PageLimits::new(4, 100);
        let entries = page(&[("files/a.ron", "1234"), ("files/b.ron", "12345")]);
        let error = layer_from("files", entries, limits).unwrap_err();
        assert!(
            matches!(&error, PageError::TooBig { path, limit: 4 } if path == "files/b.ron"),
            "{error:?}"
        );
        assert_eq!(error.to_string(), "files/b.ron: the file is over 4 bytes");
        let entries = page(&[("files/a.ron", "1234")]);
        assert!(layer_from("files", entries, limits).unwrap().is_some());
    }

    #[test]
    fn files_over_the_limit_in_all_are_refused() {
        let limits = PageLimits::new(4, 7);
        let entries = page(&[("files/a.ron", "1234"), ("files/b.ron", "123")]);
        assert!(layer_from("files", entries, limits).unwrap().is_some());
        let entries = page(&[("files/a.ron", "1234"), ("files/b.ron", "1234")]);
        let error = layer_from("files", entries, limits).unwrap_err();
        assert!(
            matches!(error, PageError::TooMuch { limit: 7 }),
            "{error:?}"
        );
        assert!(error.to_string().contains("over 7 bytes"), "{error}");
    }

    #[test]
    fn a_bad_path_is_refused_naming_it() {
        let entries = page(&[("files/../secret.ron", "x")]);
        let error = layer_from("files", entries, PageLimits::default()).unwrap_err();
        assert!(
            matches!(&error, PageError::Layer(e) if e.problem == BadPath::Parent && e.path == "../secret.ron"),
            "{error:?}"
        );
        assert!(error.to_string().contains("../secret.ron"), "{error}");
    }

    #[test]
    fn the_default_limits_are_256_kib_a_file_and_1_mib_in_all() {
        assert_eq!(
            PageLimits::default(),
            PageLimits::new(256 * 1024, 1024 * 1024)
        );
    }

    #[test]
    fn off_the_web_the_page_holds_no_files_and_hears_nothing() {
        assert!(
            page_layer("files", PageLimits::default())
                .unwrap()
                .is_none()
        );
        assert!(page_layer("", PageLimits::default()).unwrap().is_none());
        report("files/stage.map:3: a problem");
    }
}
