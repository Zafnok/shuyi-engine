//! Starts a new worktree's `target/` from a warm worktree's (ticket 2078):
//! the third-party crates' compiled output hard-linked, what cargo rewrites
//! in place copied, and the path packages left out, so they build fresh with
//! their own paths.
//!
//! Run in the warm worktree (the donor), into the new one. Only the host's
//! `target/debug/` is seeded, and only its `deps`, `build` and
//! `.fingerprint`:
//!
//! - **Linked** ([`LINKED`]): compiled output (`.rlib`, `.rmeta`, `.exe`,
//!   `.dll`, `.a`, `.o`, `.lib`, and `.so` and `.dylib` off Windows). rustc
//!   and the linker always write a new file, so a rebuild in either worktree
//!   breaks the link and leaves the other's file as it was.
//! - **Copied**, with its modification time: every other file. cargo
//!   rewrites some in place (`.fingerprint/*`, `*.d`, a build script's
//!   `output` and `root-output`), and a link would let one worktree's build
//!   change the other's. The donor's root in a build script's `output` and
//!   `root-output` becomes the new worktree's. Each folder gets its donor's
//!   modification time too: cargo reads some, and a folder newer than its
//!   unit makes the unit rebuild.
//! - **Left out**: the fingerprints of every path package (one whose
//!   `source` is null in `cargo metadata`, [`path_packages`]). cargo hashes
//!   such a package's path relative to the workspace's root, so its units
//!   have the same hash in every worktree; a seeded one would stay Fresh
//!   with the donor's paths compiled in (`env!("CARGO_MANIFEST_DIR")`,
//!   `CARGO_BIN_EXE_*`), and read the donor's files. Without a fingerprint,
//!   each rebuilds.
//!
//! - **Kept**: what the new worktree's `target/debug/` already has. A
//!   session's MCP server builds `shuyi` there as the session starts
//!   (`.mcp.json`), before the session can seed. A unit whose fingerprint
//!   folder (or build script's folder) is there was built or started there,
//!   so it is left whole; cargo removes a unit's fingerprint before building
//!   it, so one interrupted there rebuilds. A file already in `deps` is
//!   left too. A unit the target has never started has no file there, so
//!   the donor's are added without mixing.
//!
//! It refuses ([`Error`]) while a build runs in either worktree (one of its
//! [`LOCKS`] held), and a new worktree on another volume (a hard link can't
//! cross one). It holds both worktrees' locks while it seeds, so a build
//! started in either waits for it. Each unit's fingerprint is written under
//! another name and renamed into place after its files, so a seed stopped
//! part-way leaves no unit Fresh without its files.

use std::fmt;
use std::fs::{self, File, TryLockError};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use serde_json::Value;

#[cfg(test)]
mod tests;

/// The profile seeded: the host's `target/debug/`.
pub const PROFILE: &str = "debug";

/// The folders of [`PROFILE`] that are seeded.
pub const FOLDERS: [&str; 3] = ["deps", "build", ".fingerprint"];

/// The extensions of the files hard-linked: compiled output, which rustc
/// and the linker always write as a new file.
pub const LINKED: [&str; 9] = [
    "rlib", "rmeta", "exe", "dll", "a", "o", "lib", "so", "dylib",
];

/// The files in [`PROFILE`] that cargo locks while it builds.
pub const LOCKS: [&str; 3] = [".cargo-lock", ".cargo-build-lock", ".cargo-artifact-lock"];

/// The build script's files whose donor root is rewritten, in
/// `build/<unit>/`.
const BUILD_OUTPUTS: [&str; 2] = ["output", "root-output"];

/// What a seed did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Report {
    /// Files hard-linked to the donor's.
    pub linked: u64,
    /// Their bytes: the disk the new worktree shares with the donor.
    pub linked_bytes: u64,
    /// Files copied.
    pub copied: u64,
    /// Their bytes.
    pub copied_bytes: u64,
    /// Build scripts' `output` and `root-output` files whose donor root was
    /// rewritten.
    pub rewritten: u64,
    /// Units (fingerprint and build script folders) and files the new
    /// worktree already had, left as they were.
    pub kept: u64,
    /// The path packages' fingerprint folders left out (`<package>-<hash>`),
    /// sorted: those units rebuild.
    pub rebuilt: Vec<String>,
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const MIB: u64 = 1024 * 1024;
        write!(
            f,
            "linked {} files ({} MiB), copied {} files ({} MiB), rewrote the root in {} build \
             outputs, kept {} units and files already there, left out {} fingerprints of path \
             packages (they rebuild)",
            self.linked,
            self.linked_bytes / MIB,
            self.copied,
            self.copied_bytes / MIB,
            self.rewritten,
            self.kept,
            self.rebuilt.len()
        )
    }
}

/// Why a seed was refused, or failed.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The new worktree's folder doesn't exist.
    NoWorktree(PathBuf),
    /// The donor and the new worktree are the same folder.
    SameWorktree(PathBuf),
    /// The donor has no `target/debug/`.
    NoDonorBuild(PathBuf),
    /// A build is running in one of the worktrees: this lock file is held.
    Locked(PathBuf),
    /// The two worktrees are on different volumes, which a hard link can't
    /// cross.
    OtherVolume {
        /// The donor's `target/debug/`.
        donor: PathBuf,
        /// The new worktree's `target/`.
        into: PathBuf,
    },
    /// Reading or writing a file failed.
    Io {
        /// The file or folder.
        path: PathBuf,
        /// What failed.
        error: io::Error,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoWorktree(path) => write!(f, "{}: no such worktree folder", path.display()),
            Self::SameWorktree(path) => write!(
                f,
                "{}: that is the donor itself; run this in the warm worktree, with --into the new one",
                path.display()
            ),
            Self::NoDonorBuild(path) => {
                write!(f, "{}: the donor has no build to seed from", path.display())
            }
            Self::Locked(path) => write!(
                f,
                "{} is locked: a cargo build is running there; wait for it to finish",
                path.display()
            ),
            Self::OtherVolume { donor, into } => write!(
                f,
                "{} and {} are on different volumes, and a hard link can't cross them",
                donor.display(),
                into.display()
            ),
            Self::Io { path, error } => write!(f, "{}: {error}", path.display()),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { error, .. } => Some(error),
            _ => None,
        }
    }
}

/// Seeds `into`'s `target/` from `donor`'s, both worktrees' roots.
/// `path_packages` are the packages whose fingerprints are left out
/// ([`path_packages`]).
///
/// # Errors
///
/// A refusal (a build running in either worktree, another volume, no
/// donor build), or a file that couldn't be read or written.
pub fn seed(donor: &Path, into: &Path, path_packages: &[String]) -> Result<Report, Error> {
    let donor = std::path::absolute(donor).map_err(at(donor))?;
    let into = std::path::absolute(into).map_err(at(into))?;
    if !into.is_dir() {
        return Err(Error::NoWorktree(into));
    }
    if same_folder(&donor, &into) {
        return Err(Error::SameWorktree(into));
    }
    let from = donor.join("target").join(PROFILE);
    if !from.is_dir() {
        return Err(Error::NoDonorBuild(from));
    }
    let target = into.join("target");
    let to = target.join(PROFILE);
    let mut locks = lock(&from, false)?;
    let had = modified_of(&to)?;
    fs::create_dir_all(&to).map_err(at(&to))?;
    locks.extend(lock(&to, true)?);
    same_volume(&from, &target)?;
    let mut walk = Walk {
        donor: &donor,
        into: &into,
        path_packages,
        report: Report::default(),
    };
    walk.profile(&from, &to, had)?;
    walk.report.rebuilt.sort();
    Ok(walk.report)
}

/// The path packages in `cargo metadata`'s answer (`--format-version 1`):
/// those whose `source` is null, sorted and without repeats. In the engine
/// they are the workspace's members; in a game, also an engine checked out
/// inside it.
#[must_use]
pub fn path_packages(metadata: &Value) -> Vec<String> {
    let mut names: Vec<String> = metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|package| package["source"].is_null())
        .filter_map(|package| package["name"].as_str().map(str::to_owned))
        .collect();
    names.sort();
    names.dedup();
    names
}

/// [`path_packages`] of the workspace at `root`, asking `cargo metadata`
/// (the `cargo` that runs this program, from `CARGO`, or the one on `PATH`).
///
/// # Errors
///
/// `cargo metadata` couldn't run, failed, or answered something that isn't
/// JSON.
pub fn path_packages_of(root: &Path) -> Result<Vec<String>, String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("cargo metadata: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let metadata: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("cargo metadata's answer: {e}"))?;
    Ok(path_packages(&metadata))
}

/// Where a folder is in the profile: what its entries are.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    /// `.fingerprint/`: one folder per unit.
    Fingerprints,
    /// `build/`: one folder per build script's unit.
    BuildScripts,
    /// `build/<unit>/`: its `output` and `root-output` name the donor's root.
    BuildUnit,
    /// Anywhere else.
    Other,
}

impl Place {
    fn of(folder: &str) -> Self {
        match folder {
            ".fingerprint" => Self::Fingerprints,
            "build" => Self::BuildScripts,
            _ => Self::Other,
        }
    }
}

/// One seed under way.
struct Walk<'a> {
    donor: &'a Path,
    into: &'a Path,
    path_packages: &'a [String],
    report: Report,
}

impl Walk<'_> {
    /// Seeds the profile folder `from` (the donor's) into `to`, which
    /// exists, its locks held; `had` is its time before the seed made it or
    /// its locks, if it was there.
    fn profile(&mut self, from: &Path, to: &Path, had: Option<SystemTime>) -> Result<(), Error> {
        for folder in FOLDERS {
            let source = from.join(folder);
            if source.is_dir() {
                self.tree(&source, &to.join(folder), Place::of(folder))?;
            }
        }
        finish(from, to, had)
    }

    /// Copies the folder `from` into `to`, making it when it is missing,
    /// and leaving what it has. Links are never followed.
    fn tree(&mut self, from: &Path, to: &Path, place: Place) -> Result<(), Error> {
        let had = modified_of(to)?;
        if had.is_none() {
            fs::create_dir(to).map_err(at(to))?;
        }
        for entry in fs::read_dir(from).map_err(at(from))? {
            let entry = entry.map_err(at(from))?;
            let kind = entry.file_type().map_err(at(&entry.path()))?;
            let source = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let dest = to.join(&*name);
            if kind.is_symlink() {
                continue;
            }
            let present = fs::symlink_metadata(&dest).is_ok();
            if kind.is_dir() {
                if place == Place::Fingerprints && is_path_unit(&name, self.path_packages) {
                    self.report.rebuilt.push(name.into_owned());
                } else if present && matches!(place, Place::Fingerprints | Place::BuildScripts) {
                    // A unit the target has built, or started.
                    self.report.kept += 1;
                } else if place == Place::Fingerprints {
                    self.unit(&source, &dest)?;
                } else {
                    let inner = if place == Place::BuildScripts {
                        Place::BuildUnit
                    } else {
                        Place::Other
                    };
                    self.tree(&source, &dest, inner)?;
                }
            } else if present {
                self.report.kept += 1;
            } else if is_linked(&source) {
                fs::hard_link(&source, &dest).map_err(at(&dest))?;
                self.report.linked += 1;
                self.report.linked_bytes += entry.metadata().map_err(at(&source))?.len();
            } else {
                let rewrite = place == Place::BuildUnit && BUILD_OUTPUTS.contains(&&*name);
                self.copy(&source, &dest, rewrite)?;
            }
        }
        finish(from, to, had)
    }

    /// Copies a unit's fingerprint folder under another name, then renames
    /// it into place: the unit is never Fresh with part of its fingerprint.
    fn unit(&mut self, from: &Path, to: &Path) -> Result<(), Error> {
        let mut name = to.file_name().unwrap_or_default().to_os_string();
        name.push(".seeding");
        let temp = to.with_file_name(name);
        // What a seed stopped part-way left.
        let _ = fs::remove_dir_all(&temp);
        self.tree(from, &temp, Place::Other)?;
        fs::rename(&temp, to).map_err(at(to))
    }

    /// Copies one file, rewriting the donor's root in it when `rewrite`,
    /// and keeps its modification time.
    fn copy(&mut self, from: &Path, to: &Path, rewrite: bool) -> Result<(), Error> {
        let bytes = fs::copy(from, to).map_err(at(to))?;
        self.report.copied += 1;
        self.report.copied_bytes += bytes;
        if rewrite {
            let text = fs::read(to).map_err(at(to))?;
            if let Some(text) = rewrite_root(&text, self.donor, self.into, cfg!(windows)) {
                fs::write(to, text).map_err(at(to))?;
                self.report.rewritten += 1;
            }
        }
        keep_modified(from, to)
    }
}

/// Whether a file is compiled output, which is linked: its extension is
/// one of [`LINKED`].
fn is_linked(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            LINKED
                .iter()
                .any(|linked| linked.eq_ignore_ascii_case(extension))
        })
}

/// Whether a fingerprint folder's name (`<package>-<hash>`) is a unit of
/// one of the path packages. The hash is hex digits, so a package whose
/// name begins with another's (`shuyi` and `shuyi-core`) is never mistaken
/// for it.
fn is_path_unit(name: &str, path_packages: &[String]) -> bool {
    name.rsplit_once('-').is_some_and(|(package, hash)| {
        !hash.is_empty()
            && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
            && path_packages.iter().any(|path| path == package)
    })
}

/// `text` with every occurrence of the root `from` replaced by `to`, or
/// `None` when it holds none. `/` and `\` match each other, and a match
/// keeps the separator it was written with; with `fold_case` (Windows),
/// letters match in either case. A match must end the path's name there:
/// `D:\engine` is not found in `D:\engine-old`.
fn rewrite_root(text: &[u8], from: &Path, to: &Path, fold_case: bool) -> Option<Vec<u8>> {
    let from = from.to_string_lossy();
    let from = from.trim_end_matches(['/', '\\']).as_bytes();
    let to = to.to_string_lossy();
    let to = to.trim_end_matches(['/', '\\']).as_bytes();
    if from.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(text.len());
    let mut changed = false;
    let mut at = 0;
    while at < text.len() {
        if let Some(separator) = root_at(text, at, from, fold_case) {
            out.extend(
                to.iter()
                    .map(|&byte| if is_separator(byte) { separator } else { byte }),
            );
            at += from.len();
            changed = true;
        } else {
            out.push(text[at]);
            at += 1;
        }
    }
    changed.then_some(out)
}

/// The separator `text` writes the root `root` with at `at`, when it holds
/// the root there and the path's name ends after it.
fn root_at(text: &[u8], at: usize, root: &[u8], fold_case: bool) -> Option<u8> {
    let span = text.get(at..at + root.len())?;
    let mut separator = None;
    for (&byte, &want) in span.iter().zip(root) {
        if is_separator(byte) && is_separator(want) {
            separator.get_or_insert(byte);
        } else if !(byte == want || fold_case && byte.eq_ignore_ascii_case(&want)) {
            return None;
        }
    }
    let ends = text
        .get(at + root.len())
        .is_none_or(|&next| is_separator(next) || !is_name_byte(next));
    ends.then(|| separator.unwrap_or(std::path::MAIN_SEPARATOR as u8))
}

const fn is_separator(byte: u8) -> bool {
    matches!(byte, b'/' | b'\\')
}

/// Whether `byte` can go on a folder's name.
const fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.') || byte >= 0x80
}

/// Whether two existing folders are the same one.
fn same_folder(one: &Path, other: &Path) -> bool {
    match (fs::canonicalize(one), fs::canonicalize(other)) {
        (Ok(one), Ok(other)) => one == other,
        _ => one == other,
    }
}

/// A folder's modification time, or `None` when it doesn't exist.
fn modified_of(folder: &Path) -> Result<Option<SystemTime>, Error> {
    match fs::metadata(folder) {
        Ok(metadata) => metadata.modified().map(Some).map_err(at(folder)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::Io {
            path: folder.to_path_buf(),
            error: e,
        }),
    }
}

/// Gives the folder `to` its own time back when it `had` one, or else the
/// time of the donor's `from`.
fn finish(from: &Path, to: &Path, had: Option<SystemTime>) -> Result<(), Error> {
    match had {
        Some(time) => set_modified(to, time).map_err(at(to)),
        None => keep_modified(from, to),
    }
}

/// Locks each of a profile folder's [`LOCKS`] without waiting, and holds
/// them: a build there waits for the seed. In the donor only those that
/// exist (its files are never changed); in the new worktree each is made
/// when missing (`create`), so a build starting there waits too.
fn lock(profile: &Path, create: bool) -> Result<Vec<File>, Error> {
    let mut held = Vec::new();
    for name in LOCKS {
        let path = profile.join(name);
        let opened = File::options()
            .read(true)
            .write(create)
            .create(create)
            .truncate(false)
            .open(&path);
        let file = match opened {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(Error::Io { path, error: e }),
        };
        match file.try_lock() {
            Ok(()) => held.push(file),
            Err(TryLockError::WouldBlock) => return Err(Error::Locked(path)),
            Err(TryLockError::Error(e)) => return Err(Error::Io { path, error: e }),
        }
    }
    Ok(held)
}

/// Refuses when a hard link from the donor's profile `from` can't reach
/// `target`: links its first linkable file there, and removes the link.
fn same_volume(from: &Path, target: &Path) -> Result<(), Error> {
    let Some(probe) = FOLDERS
        .iter()
        .find_map(|folder| first_linked(&from.join(folder)))
    else {
        return Ok(());
    };
    let link = target.join(".seed-probe");
    // What a seed stopped before removing its probe left.
    let _ = fs::remove_file(&link);
    match fs::hard_link(&probe, &link) {
        Ok(()) => fs::remove_file(&link).map_err(at(&link)),
        Err(error) => Err(link_refusal(error, from, target)),
    }
}

/// A file of `folder` that is linked, if it holds one.
fn first_linked(folder: &Path) -> Option<PathBuf> {
    fs::read_dir(folder)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| is_linked(path))
}

/// What a failed probe link means: another volume, or another failure.
fn link_refusal(error: io::Error, from: &Path, target: &Path) -> Error {
    if error.kind() == io::ErrorKind::CrossesDevices {
        Error::OtherVolume {
            donor: from.to_path_buf(),
            into: target.to_path_buf(),
        }
    } else {
        Error::Io {
            path: target.to_path_buf(),
            error,
        }
    }
}

/// Gives `to` (a file or a folder) the modification time of `from`.
fn keep_modified(from: &Path, to: &Path) -> Result<(), Error> {
    let time = fs::metadata(from)
        .and_then(|m| m.modified())
        .map_err(at(from))?;
    set_modified(to, time).map_err(at(to))
}

/// Sets the modification time of a file or a folder.
fn set_modified(path: &Path, time: SystemTime) -> io::Result<()> {
    #[cfg(windows)]
    let file = {
        use std::os::windows::fs::OpenOptionsExt as _;
        /// `FILE_WRITE_ATTRIBUTES`: the right to set the times, and no more.
        const WRITE_ATTRIBUTES: u32 = 0x100;
        /// `FILE_FLAG_BACKUP_SEMANTICS`: what lets a folder be opened.
        const BACKUP_SEMANTICS: u32 = 0x0200_0000;
        fs::OpenOptions::new()
            .access_mode(WRITE_ATTRIBUTES)
            .custom_flags(BACKUP_SEMANTICS)
            .open(path)?
    };
    #[cfg(not(windows))]
    let file = File::open(path)?;
    file.set_modified(time)
}

/// Maps an I/O error to [`Error::Io`] at `path`.
fn at(path: &Path) -> impl FnOnce(io::Error) -> Error + '_ {
    move |error| Error::Io {
        path: path.to_path_buf(),
        error,
    }
}
