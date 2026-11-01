//! A folder on disk read into a bundle layer (ADR-0004 §1): a player's mod,
//! a level they made, or a debug build's art read without a rebuild. The
//! game says where the folder is and lays the layer over its own files;
//! where mods live and in what order is the game's. Native only: the web
//! has no folders.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use shuyi_assets::bundle::{Layer, LayerError};
use shuyi_assets::format::DEFAULT_MAX_BYTES;
use shuyi_platform::WATCH_VAR;

use crate::watch;

/// Why a folder could not be read into a layer ([`read_folder`]).
#[derive(Debug)]
#[non_exhaustive]
pub enum FolderError {
    /// A file or folder that could not be read.
    Read {
        /// The file or folder.
        path: PathBuf,
        /// What the system said.
        error: io::Error,
    },
    /// A file over the limit.
    TooBig {
        /// The file.
        path: PathBuf,
        /// The limit, in bytes.
        limit: u64,
    },
    /// A file or folder whose name isn't UTF-8, which no read can name.
    NotUtf8 {
        /// The file or folder.
        path: PathBuf,
    },
    /// A path the layer refused (a backslash in a name, where the system
    /// allows one).
    Layer(LayerError),
}

impl core::fmt::Display for FolderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Read { path, error } => write!(f, "{}: {error}", path.display()),
            Self::TooBig { path, limit } => {
                write!(f, "{}: the file is over {limit} bytes", path.display())
            }
            Self::NotUtf8 { path } => write!(f, "{}: the name isn't UTF-8", path.display()),
            Self::Layer(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for FolderError {}

/// Every file in the folder at `path` and the folders below it, as one
/// layer named `name`, each at its path relative to the folder,
/// `/`-separated (`data/crops.ron`). A link to a file is read; a link to
/// a folder is not followed. Read it once: the layer's files are kept for
/// the life of the program.
///
/// In a build with debug tools the folder is watched from then on: a file
/// that changes takes its new bytes in the layer, and the game's modules
/// hear of it (`Module::reloaded`).
///
/// # Errors
///
/// A file over `max_file_bytes`, a file or folder that can't be read or
/// whose name isn't UTF-8, each naming it, and a path the layer refuses.
pub fn read_folder(path: &Path, name: &str, max_file_bytes: u64) -> Result<Layer, FolderError> {
    // Before the files are read: one that changes meanwhile is seen as
    // changed, and read again.
    let seen = shuyi_ui::DEBUG_TOOLS.then(|| watch::scan(path));
    let mut files = BTreeMap::new();
    let mut pending = vec![(path.to_path_buf(), String::new())];
    while let Some((dir, prefix)) = pending.pop() {
        let read = |error| FolderError::Read {
            path: dir.clone(),
            error,
        };
        for entry in fs::read_dir(&dir).map_err(read)? {
            let entry = entry.map_err(read)?;
            let at = entry.path();
            let Some(file_name) = entry.file_name().to_str().map(str::to_owned) else {
                return Err(FolderError::NotUtf8 { path: at });
            };
            let inner = if prefix.is_empty() {
                file_name
            } else {
                format!("{prefix}/{file_name}")
            };
            let kind = entry.file_type().map_err(|error| FolderError::Read {
                path: at.clone(),
                error,
            })?;
            if kind.is_dir() {
                pending.push((at, inner));
            } else if fs::metadata(&at).is_ok_and(|meta| meta.is_file()) {
                let bytes = read_file(&at, max_file_bytes)?;
                files.insert(inner, bytes);
            }
        }
    }
    let layer = Layer::owned(name.to_owned(), files).map_err(FolderError::Layer)?;
    if let Some(seen) = seen {
        watch::note_folder(path, layer, max_file_bytes, seen);
    }
    Ok(layer)
}

/// The folder `cargo shuyi run --watch` named ([`WATCH_VAR`]) as a layer to
/// lay over the game's files, watched like any folder [`read_folder`]
/// reads and shown under the folder's name: in a build with debug tools,
/// when it is set. A folder that can't be read logs why, and the game runs
/// on its embedded files.
pub fn watched_layer() -> Option<Layer> {
    if !shuyi_ui::DEBUG_TOOLS {
        return None;
    }
    let path = PathBuf::from(std::env::var_os(WATCH_VAR)?);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("watched");
    let limit = u64::try_from(DEFAULT_MAX_BYTES).unwrap_or(u64::MAX);
    match read_folder(&path, name, limit) {
        Ok(layer) => Some(layer),
        Err(error) => {
            macroquad::logging::warn!("{}: {}", WATCH_VAR, error);
            None
        }
    }
}

/// The bytes of the file at `path`, or an error if it is over `limit`
/// bytes. It never reads more than one byte past the limit, also when the
/// file grows while it is read.
pub(crate) fn read_file(path: &Path, limit: u64) -> Result<Vec<u8>, FolderError> {
    let read = |error| FolderError::Read {
        path: path.to_path_buf(),
        error,
    };
    let file = File::open(path).map_err(read)?;
    let too_big = || FolderError::TooBig {
        path: path.to_path_buf(),
        limit,
    };
    if file.metadata().map_err(read)?.len() > limit {
        return Err(too_big());
    }
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(read)?;
    if u64::try_from(bytes.len()).map_or(true, |len| len > limit) {
        return Err(too_big());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use shuyi_assets::bundle::Bundle;

    use super::*;

    /// A temporary folder holding `files` (`/`-separated paths).
    fn folder(files: &[(&str, &[u8])]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (path, bytes) in files {
            let at = dir.path().join(path);
            fs::create_dir_all(at.parent().unwrap()).unwrap();
            fs::write(at, bytes).unwrap();
        }
        dir
    }

    /// The base layer the folder is laid over.
    static BASE: &[(&str, &[u8])] = &[("data/crops.ron", b"base crops"), ("top.txt", b"top")];

    #[test]
    fn a_folder_is_read_into_a_layer_laid_over_the_games_files() {
        let dir = folder(&[
            ("data/crops.ron", b"mod crops"),
            ("data/more/deep.ron", b"deep"),
            ("new.png", &[0xff, 0x00]),
        ]);
        fs::create_dir(dir.path().join("empty")).unwrap();
        let layer = read_folder(dir.path(), "mods/more", 16).unwrap();
        let bundle = Bundle::new(vec![layer, Layer::table("base", BASE)]);
        assert_eq!(bundle.file("data/crops.ron"), Some("mod crops"));
        assert_eq!(
            bundle.display_path("data/crops.ron"),
            "mods/more/data/crops.ron"
        );
        assert_eq!(bundle.file("data/more/deep.ron"), Some("deep"));
        assert_eq!(bundle.bytes("new.png"), Some(&[0xff, 0x00][..]));
        assert_eq!(bundle.file("top.txt"), Some("top"));
        assert_eq!(
            bundle.files_under(""),
            ["data/crops.ron", "data/more/deep.ron", "new.png", "top.txt"]
        );
    }

    #[test]
    fn a_file_at_the_limit_is_read_and_one_over_it_is_refused() {
        let dir = folder(&[("a.txt", b"1234"), ("data/b.txt", b"12345")]);
        let error = read_folder(dir.path(), "mods/big", 4).unwrap_err();
        assert!(
            matches!(&error, FolderError::TooBig { path, limit: 4 } if path.ends_with("b.txt")),
            "{error:?}"
        );
        let message = error.to_string();
        assert!(
            message.contains("b.txt") && message.contains("over 4 bytes"),
            "{message}"
        );
        let layer = read_folder(dir.path(), "mods/big", 5).unwrap();
        assert_eq!(Bundle::new(vec![layer]).file("data/b.txt"), Some("12345"));
    }

    #[test]
    fn a_missing_folder_is_an_error_naming_it() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope");
        let error = read_folder(&missing, "mods/none", 16).unwrap_err();
        assert!(
            matches!(&error, FolderError::Read { path, .. } if *path == missing),
            "{error:?}"
        );
        assert!(error.to_string().contains("nope"), "{error}");
    }

    #[test]
    fn an_empty_folder_is_an_empty_layer() {
        let dir = tempfile::tempdir().unwrap();
        let layer = read_folder(dir.path(), "mods/empty", 16).unwrap();
        let listed = Bundle::new(vec![layer]).files_under("");
        assert!(listed.is_empty(), "{listed:?}");
    }

    /// Where a name may hold a backslash, the layer refuses it.
    #[cfg(unix)]
    #[test]
    fn a_name_with_a_backslash_is_refused() {
        let dir = folder(&[("a\\b.txt", b"x")]);
        let error = read_folder(dir.path(), "mods/odd", 16).unwrap_err();
        assert!(
            matches!(&error, FolderError::Layer(e) if e.problem == shuyi_assets::bundle::BadPath::Backslash),
            "{error:?}"
        );
    }
}
