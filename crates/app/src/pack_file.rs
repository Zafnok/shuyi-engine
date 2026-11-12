//! The pack beside the program (ADR-0023, ADR-0110): the game's files baked
//! into one file, `<name>.pack`, which a release export ships beside the
//! program in place of the files embedded as text. [`pack_beside_program`]
//! reads it and opens it with the game's stamp; the game lays it in its
//! bundle as its files' layer (`Pack::into_layer`).
//!
//! On Windows and Linux the pack is in the program's folder; in a macOS
//! `.app` it is in `Contents/Resources`, which the bundle's signature
//! covers. On the web the page's loader fetches it beside `index.html`
//! before the game starts (`crates/cli/web/pack-file.js`), and hands its
//! bytes over here, so it is read at the same point on every target.
//!
//! A development build (`cargo run`, `shuyi run`) embeds the files and needs
//! no pack: only a build with the game's `packed` feature calls this.

use shuyi_assets::pack::{Pack, PackError};

/// The plugin's version, which the JS loader compares with the `version`
/// in `crates/cli/web/pack-file.js` (a `shuyi-cli` test keeps them equal).
/// Bump both when the functions below change.
#[cfg(target_arch = "wasm32")]
const PLUGIN_VERSION: u32 = 1;

// Declaring the functions `crates/cli/web/pack-file.js` provides. They
// answer an empty array and a problem when the page fetched no pack, so
// calling them is safe.
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code)]
unsafe extern "C" {
    /// The bytes of the pack the page fetched; empty if it has none.
    safe fn shuyi_pack_file_bytes() -> sapp_jsutils::JsObject;
    /// Why the page has no pack (what its fetch said); `""` when it has one.
    safe fn shuyi_pack_file_problem() -> sapp_jsutils::JsObject;
}

/// What the JS loader calls to check the plugin's version.
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn shuyi_pack_file_crate_version() -> u32 {
    PLUGIN_VERSION
}

/// The extension of the pack beside the program.
pub const PACK_EXTENSION: &str = "pack";

/// The name of the pack of the game whose package is `name`:
/// `<name>.pack`, as a release export writes it.
pub fn pack_file_name(name: &str) -> String {
    format!("{name}.{PACK_EXTENSION}")
}

/// Why the pack beside the program could not be read
/// ([`pack_beside_program`]). Each says to bake it again, as ADR-0023's
/// refusals do.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PackFileError {
    /// No pack where the program looks for it.
    Missing {
        /// The pack's file name.
        file: String,
        /// Where it was looked for: each folder, or on the web what the
        /// page's fetch said.
        looked: Vec<String>,
    },
    /// The file is there and can't be read.
    Unreadable {
        /// The file.
        file: String,
        /// What the system said.
        error: String,
    },
    /// The file is no pack of this game: of another version, another stamp,
    /// cut short or changed ([`PackError`]).
    Refused {
        /// The file.
        file: String,
        /// Why `Pack::open` refused it.
        error: PackError,
    },
}

impl core::fmt::Display for PackFileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Missing { file, looked } => write!(
                f,
                "the game's files, {file}, aren't beside the program (looked in {}): bake it \
                 again with the game's export, and keep it beside the program",
                looked.join(", ")
            ),
            Self::Unreadable { file, error } => {
                write!(f, "{file}: {error}: bake it again with the game's export")
            }
            Self::Refused { file, error } => write!(f, "{file}: {error}"),
        }
    }
}

impl std::error::Error for PackFileError {}

/// The pack of the game whose package is `name`, `<name>.pack`, read from
/// beside the program and opened with `stamp`, the stamp it was baked with
/// (`BasicGame::pack_stamp`, or the game's own). Its bytes are kept for the
/// rest of the run, as embedded files are: open it once, at the start, and
/// lay it in the bundle with `Pack::into_layer`.
///
/// # Errors
///
/// No pack beside the program, one that can't be read, or one
/// `Pack::open` refuses: each says to bake it again.
pub fn pack_beside_program(name: &str, stamp: &str) -> Result<Pack<'static>, PackFileError> {
    let file = pack_file_name(name);
    let bytes = read_beside(&file)?;
    // Kept for the rest of the run: the pack's stored files borrow them.
    let bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    Pack::open(bytes, stamp).map_err(|error| PackFileError::Refused { file, error })
}

/// The bytes of `file` in the first of the program's folders that holds
/// it ([`program_dirs`]).
#[cfg(not(target_arch = "wasm32"))]
fn read_beside(file: &str) -> Result<Vec<u8>, PackFileError> {
    let dirs = program_dirs().map_err(|error| PackFileError::Unreadable {
        file: file.to_owned(),
        error,
    })?;
    read_in(&dirs, file)
}

/// The bytes the page fetched before the game started.
#[cfg(target_arch = "wasm32")]
fn read_beside(file: &str) -> Result<Vec<u8>, PackFileError> {
    let mut problem = String::new();
    shuyi_pack_file_problem().to_string(&mut problem);
    if !problem.is_empty() {
        return Err(PackFileError::Missing {
            file: file.to_owned(),
            looked: vec![format!("the page: {problem}")],
        });
    }
    let mut bytes = Vec::new();
    shuyi_pack_file_bytes().to_byte_buffer(&mut bytes);
    Ok(bytes)
}

/// Where the program looks for its pack: its own folder, and in a macOS
/// `.app` the bundle's `Contents/Resources` beside it.
#[cfg(not(target_arch = "wasm32"))]
fn program_dirs() -> Result<Vec<std::path::PathBuf>, String> {
    let exe = std::env::current_exe().map_err(|e| format!("the program's path: {e}"))?;
    let dir = exe
        .parent()
        .ok_or("the program's path has no folder")?
        .to_path_buf();
    Ok(with_resources(dir, cfg!(target_os = "macos")))
}

/// `dir`, then its bundle's `Resources` folder if `macos`.
#[cfg(not(target_arch = "wasm32"))]
fn with_resources(dir: std::path::PathBuf, macos: bool) -> Vec<std::path::PathBuf> {
    let resources = dir.parent().map(|contents| contents.join("Resources"));
    let mut dirs = vec![dir];
    dirs.extend(resources.filter(|_| macos));
    dirs
}

/// The bytes of `file` in the first of `dirs` that holds it.
#[cfg(not(target_arch = "wasm32"))]
fn read_in(dirs: &[std::path::PathBuf], file: &str) -> Result<Vec<u8>, PackFileError> {
    for dir in dirs {
        let path = dir.join(file);
        if path.is_file() {
            return std::fs::read(&path).map_err(|e| PackFileError::Unreadable {
                file: path.display().to_string(),
                error: e.to_string(),
            });
        }
    }
    Err(PackFileError::Missing {
        file: file.to_owned(),
        looked: dirs.iter().map(|dir| dir.display().to_string()).collect(),
    })
}

/// The pack of `file` in `dirs`, opened with `stamp`: [`pack_beside_program`]
/// on folders a test picks.
#[cfg(all(test, not(target_arch = "wasm32")))]
fn pack_in(
    dirs: &[std::path::PathBuf],
    file: &str,
    stamp: &str,
) -> Result<Pack<'static>, PackFileError> {
    let bytes: &'static [u8] = Box::leak(read_in(dirs, file)?.into_boxed_slice());
    Pack::open(bytes, stamp).map_err(|error| PackFileError::Refused {
        file: file.to_owned(),
        error,
    })
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::collections::BTreeMap;

    use shuyi_assets::pack::{Content, PackInfo, write};

    use super::*;

    /// A pack of one file, stamped `stamp`.
    fn pack(stamp: &str) -> Vec<u8> {
        let info = PackInfo {
            stamp: stamp.to_owned(),
            ..PackInfo::default()
        };
        let files = BTreeMap::from([("en.ron".to_owned(), Content::Raw(b"{}".to_vec()))]);
        write(&info, &files)
    }

    #[test]
    fn the_pack_beside_the_program_is_found_and_opened() {
        let program = tempfile::tempdir().unwrap();
        let file = pack_file_name("demo");
        assert_eq!(file, "demo.pack");
        std::fs::write(program.path().join(&file), pack("demo 1")).unwrap();
        let opened = pack_in(&[program.path().to_path_buf()], &file, "demo 1")
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(opened.get("en.ron"), Some(&b"{}"[..]));
    }

    #[test]
    fn in_a_mac_app_the_pack_is_found_in_its_resources() {
        let app = tempfile::tempdir().unwrap();
        let macos = app.path().join("Contents/MacOS");
        let resources = app.path().join("Contents/Resources");
        std::fs::create_dir_all(&macos).unwrap();
        std::fs::create_dir_all(&resources).unwrap();
        std::fs::write(resources.join("demo.pack"), pack("demo 1")).unwrap();
        let dirs = with_resources(macos.clone(), true);
        assert_eq!(dirs, [macos.clone(), app.path().join("Contents/Resources")]);
        assert!(pack_in(&dirs, "demo.pack", "demo 1").is_ok());
        assert_eq!(with_resources(macos.clone(), false), [macos]);
    }

    #[test]
    fn a_missing_pack_says_where_it_looked_and_to_bake_it_again() {
        let program = tempfile::tempdir().unwrap();
        let error = pack_in(&[program.path().to_path_buf()], "demo.pack", "demo 1")
            .err()
            .unwrap();
        assert!(matches!(&error, PackFileError::Missing { file, .. } if file == "demo.pack"));
        let message = error.to_string();
        assert!(
            message.contains(&program.path().display().to_string()),
            "{message}"
        );
        assert!(message.contains("bake it again"), "{message}");
    }

    #[test]
    fn a_pack_of_another_game_or_version_is_refused_with_bake_it_again() {
        let program = tempfile::tempdir().unwrap();
        std::fs::write(program.path().join("demo.pack"), pack("demo 1")).unwrap();
        let error = pack_in(&[program.path().to_path_buf()], "demo.pack", "demo 2")
            .err()
            .unwrap();
        assert!(
            matches!(
                &error,
                PackFileError::Refused {
                    error: PackError::Stamp { .. },
                    ..
                }
            ),
            "{error:?}"
        );
        assert!(error.to_string().contains("bake it again"), "{error}");
        std::fs::write(program.path().join("demo.pack"), b"not a pack").unwrap();
        let error = pack_in(&[program.path().to_path_buf()], "demo.pack", "demo 1")
            .err()
            .unwrap();
        assert!(
            matches!(
                &error,
                PackFileError::Refused {
                    error: PackError::NotAPack,
                    ..
                }
            ),
            "{error:?}"
        );
    }
}
