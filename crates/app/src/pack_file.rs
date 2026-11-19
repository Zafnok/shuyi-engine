//! The pack beside the program (ADR-0023, ADR-0110): the game's files baked
//! into one file, `<name>.pack`, which a release export ships beside the
//! program in place of the files embedded as text. [`pack_beside_program`]
//! opens it with the game's stamp; the game lays it in its bundle as its
//! files' layer ([`OpenedPack::into_layer`]).
//!
//! On Windows and Linux the pack is in the program's folder, and in a macOS
//! `.app` in `Contents/Resources`, which the bundle's signature covers: it
//! is read whole. On the web the page streams it beside `index.html`
//! (`crates/cli/web/pack-file.js`, ADR-0128 §2, §3 and §6): it starts the
//! game once the pack's header and index have arrived, and hands over the
//! rest as it comes, which `shuyi-app` copies into the pack's source between
//! frames ([`pump`]). A file whose bytes haven't arrived reads as missing for
//! now, and a game's load waits for the folders it needs with
//! `Bundle::arrived`.
//!
//! A development build (`cargo run`, `shuyi run`) embeds the files and needs
//! no pack: only a build with the game's `packed` feature calls this.

#[cfg(any(test, target_arch = "wasm32"))]
mod stream;

use shuyi_assets::bundle::Layer;
#[cfg(not(target_arch = "wasm32"))]
use shuyi_assets::pack::Pack;
use shuyi_assets::pack::{PackError, PackInfo};

/// The plugin's version, which the JS loader compares with the `version`
/// in `crates/cli/web/pack-file.js` (a `shuyi-cli` test keeps them equal).
/// Bump both when the functions below change.
#[cfg(target_arch = "wasm32")]
const PLUGIN_VERSION: u32 = 2;

// Declaring the functions `crates/cli/web/pack-file.js` provides. They
// take numbers, and a pointer with its length in bytes that only `Js` below
// passes, from a slice it holds for the call: the JS writes at most
// `capacity` bytes at `buffer` before it returns, and keeps none. With no
// pack they answer nothing and a problem, so calling them is safe.
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code)]
unsafe extern "C" {
    /// Writes the next bytes of the pack the page received, up to
    /// `capacity`, at `buffer`, and drops them: how many it wrote.
    safe fn shuyi_pack_file_take(buffer: *mut u8, capacity: u32) -> u32;
    /// 1 once the page has received the pack's whole body, else 0.
    safe fn shuyi_pack_file_ended() -> u32;
    /// Why the page has no pack, or why its fetch broke (what it said);
    /// `""` while it is fine.
    safe fn shuyi_pack_file_problem() -> sapp_jsutils::JsObject;
}

/// The page, through the plugin's functions.
#[cfg(target_arch = "wasm32")]
struct Js;

#[cfg(target_arch = "wasm32")]
impl stream::Page for Js {
    fn take(&self, into: &mut [u8]) -> usize {
        let capacity = u32::try_from(into.len()).unwrap_or(u32::MAX);
        let wrote = shuyi_pack_file_take(into.as_mut_ptr(), capacity);
        usize::try_from(wrote).map_or(0, |wrote| wrote.min(into.len()))
    }

    fn ended(&self) -> bool {
        shuyi_pack_file_ended() != 0
    }

    fn problem(&self) -> String {
        let mut problem = String::new();
        shuyi_pack_file_problem().to_string(&mut problem);
        problem
    }
}

/// The stream of the pack the page fetches, once the game opened it.
#[cfg(target_arch = "wasm32")]
static STREAM: std::sync::OnceLock<&'static stream::Stream> = std::sync::OnceLock::new();

/// Copies the chunks of the pack the page received since into the pack's
/// source, in order: what `shuyi-app` does between frames. Nothing on
/// desktop, or before the game opened its pack.
pub(crate) fn pump() {
    #[cfg(target_arch = "wasm32")]
    if let Some(stream) = STREAM.get() {
        stream.pump(&Js);
    }
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

/// The pack beside the program, opened ([`pack_beside_program`]): read whole
/// on desktop; on the web, its index, over the bytes the page streams
/// (ADR-0128 §2). Lay it in the game's bundle with
/// [`into_layer`](Self::into_layer).
#[derive(Debug)]
pub struct OpenedPack(Opened);

/// How the pack was opened.
#[derive(Debug)]
enum Opened {
    /// Read whole, its bytes kept for the run.
    #[cfg(not(target_arch = "wasm32"))]
    Whole(Pack<'static>),
    /// Its index, and the stream its files are read from as they arrive.
    #[cfg(target_arch = "wasm32")]
    Streamed(shuyi_assets::pack::Index, &'static stream::Stream),
}

impl OpenedPack {
    /// What the pack says of itself.
    pub fn info(&self) -> &PackInfo {
        match &self.0 {
            #[cfg(not(target_arch = "wasm32"))]
            Opened::Whole(pack) => pack.info(),
            #[cfg(target_arch = "wasm32")]
            Opened::Streamed(index, _) => index.info(),
        }
    }

    /// The pack as a bundle layer shown as `name` (`files`): the files of a
    /// pack read whole (`Pack::into_layer`); on the web, each file read from
    /// the bytes arrived when the bundle asks for it (`Index::into_layer`),
    /// missing for now until they have. A game whose load needs a folder
    /// waits for it with `Bundle::arrived`.
    pub fn into_layer(self, name: &'static str) -> Layer {
        match self.0 {
            #[cfg(not(target_arch = "wasm32"))]
            Opened::Whole(pack) => pack.into_layer(name),
            #[cfg(target_arch = "wasm32")]
            Opened::Streamed(index, stream) => index.into_layer(name, stream),
        }
    }
}

/// The pack of the game whose package is `name`, `<name>.pack`, beside the
/// program, opened with `stamp`, the stamp it was baked with
/// (`BasicGame::pack_stamp`, or the game's own). Open it once, at the start,
/// and lay it in the bundle with [`OpenedPack::into_layer`].
///
/// On desktop it is read whole, and its bytes are kept for the rest of the
/// run, as embedded files are. On the web it is the pack the page streams
/// (ADR-0128 §2): its index has arrived before the game starts, and the
/// rest arrives in the order the bake laid it, kept once, in the program's
/// memory.
///
/// # Errors
///
/// No pack beside the program (on the web, what the page's fetch said), one
/// that can't be read, or one `Pack::open` (on the web, `Index::read`)
/// refuses: each says to bake it again.
pub fn pack_beside_program(name: &str, stamp: &str) -> Result<OpenedPack, PackFileError> {
    let file = pack_file_name(name);
    open_beside(&file, stamp).map(OpenedPack)
}

/// The pack `file` beside the program, read whole and opened with `stamp`.
#[cfg(not(target_arch = "wasm32"))]
fn open_beside(file: &str, stamp: &str) -> Result<Opened, PackFileError> {
    let bytes = read_beside(file)?;
    // Kept for the rest of the run: the pack's stored files borrow them.
    let bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    Pack::open(bytes, stamp)
        .map(Opened::Whole)
        .map_err(|error| PackFileError::Refused {
            file: file.to_owned(),
            error,
        })
}

/// The pack `file` the page streams, its index opened with `stamp`, and its
/// stream kept for the run, which [`pump`] fills between frames.
#[cfg(target_arch = "wasm32")]
fn open_beside(file: &str, stamp: &str) -> Result<Opened, PackFileError> {
    let (index, stream) = stream::open(&Js, file, stamp)?;
    let stream: &'static stream::Stream = STREAM.get_or_init(|| Box::leak(Box::new(stream)));
    Ok(Opened::Streamed(index, stream))
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
        let mut info = PackInfo::default();
        info.stamp = String::from(stamp);
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
