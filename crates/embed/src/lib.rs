//! A folder of files embedded in a program: a build script walks it and
//! writes a sorted table of `(path, bytes)`, which the crate then includes.
//! The table is a `&'static [(&'static str, &'static [u8])]` of
//! `include_bytes!`, so it builds on every target, with or without `std`,
//! and a game hands it to `shuyi_assets::Layer::table`.
//!
//! ```no_run
//! // build.rs, in its `fn main`
//! shuyi_embed::Table::new("files").dir("files").skip("**/README.md").write();
//! ```
//!
//! ```ignore
//! // src/lib.rs
//! static FILES: &[(&str, &[u8])] = shuyi_embed::files!("files");
//! static LEFT_OUT: &[&str] = shuyi_embed::skipped!("files");
//! ```
//!
//! - **Paths** are relative to the folder, `/`-separated on every host, and
//!   sorted by their bytes, as `Layer::table` reads them.
//! - **Rebuilds:** [`Table::write`] prints `cargo::rerun-if-changed` for the
//!   folder and every file and folder in it, so adding, changing or removing
//!   a file rebuilds the crate on the next `cargo build`.
//! - **Skipped folders:** a folder a skip pattern matches whole
//!   (`**/target/**`) isn't entered, listed or watched, so a build writing
//!   in it, even one running beside this one, neither fails the walk nor
//!   reruns the build script.
//! - **Vanished files:** a file or folder deleted between its folder's
//!   listing and its reading is left out, not an error.
//! - **Refused**, failing the build with the path: a name that isn't UTF-8,
//!   and two paths that differ only by case, which are one file on Windows
//!   and macOS.
//!
//! `shuyi_assets::files!` embeds a folder the same way without a build
//! script, through this crate's [`walk`] and [`table_source`] (ADR-0138).
//!
//! Written from Cargo's build-script documentation (ADR-0021 §2).

use std::fmt::{self, Write as _};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// One file to embed: its path in the table, and where it is on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Its path from the folder, `/`-separated.
    pub path: String,
    /// Its absolute path on disk, which `include_bytes!` reads.
    pub file: PathBuf,
}

/// What [`walk`] found in a folder.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Walk {
    /// The files to embed, sorted by path.
    pub files: Vec<Entry>,
    /// The paths of the files a skip pattern left out, sorted; not those
    /// in a folder a pattern skips whole, which [`walk`] doesn't enter.
    pub skipped: Vec<String>,
    /// The folder and every folder in it, which a build watches; not a
    /// folder a pattern skips whole.
    pub folders: Vec<PathBuf>,
}

/// Why a folder couldn't be embedded.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The folder, or something in it, couldn't be read.
    Read {
        /// What couldn't be read.
        path: PathBuf,
        /// Why.
        error: io::Error,
    },
    /// The folder to embed isn't a folder.
    NotAFolder(PathBuf),
    /// A file or folder whose name isn't UTF-8: a table's paths are `str`.
    NotUtf8(PathBuf),
    /// Two paths that differ only by case: one file on Windows and macOS.
    SameButCase(String, String),
    /// A table's name that can't be part of a file name: letters, digits,
    /// `_` and `-` only.
    BadName(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, error } => write!(f, "{}: {error}", path.display()),
            Self::NotAFolder(path) => write!(f, "{}: not a folder", path.display()),
            Self::NotUtf8(path) => {
                write!(f, "{}: the name isn't UTF-8", path.display())
            }
            Self::SameButCase(first, second) => write!(
                f,
                "`{first}` and `{second}` differ only by case, \
                 and are one file on Windows and macOS"
            ),
            Self::BadName(name) => write!(
                f,
                "`{name}`: a table's name holds only letters, digits, `_` and `-`"
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { error, .. } => Some(error),
            _ => None,
        }
    }
}

/// Walks folder `root`: every file in it at any depth, but those a pattern
/// of `skip` matches ([`matches()`]), sorted by path.
///
/// A folder a pattern skips whole (`**/target/**` skips `target/` and
/// every folder of that name) isn't entered: its files are neither in
/// [`Walk::skipped`] nor in [`Walk::folders`], so a build writing there
/// is never read. A file or folder deleted between its folder's listing
/// and its reading is left out, as if the listing hadn't held it; any
/// other failure to read is an [`Error::Read`].
pub fn walk(root: &Path, skip: &[String]) -> Result<Walk, Error> {
    let read = |path: &Path, error| Error::Read {
        path: path.to_path_buf(),
        error,
    };
    if root.to_str().is_none() {
        return Err(Error::NotUtf8(root.to_path_buf()));
    }
    if !fs::metadata(root).map_err(|e| read(root, e))?.is_dir() {
        return Err(Error::NotAFolder(root.to_path_buf()));
    }
    let mut walk = Walk::default();
    let mut pending = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, prefix)) = pending.pop() {
        let Some(entries) = gone_is_none(fs::read_dir(&dir)).map_err(|e| read(&dir, e))? else {
            // Deleted since it was listed: as if it never was.
            continue;
        };
        walk.folders.push(dir.clone());
        for entry in entries {
            let entry = entry.map_err(|e| read(&dir, e))?;
            let file = entry.path();
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                return Err(Error::NotUtf8(file));
            };
            let path = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            let Some(kind) = kind_of(&file)? else {
                continue;
            };
            if kind.is_dir() {
                // A folder a pattern skips whole isn't entered: neither
                // listed nor watched, so a build inside it (a `target/`)
                // can't fail the walk or rerun the build script.
                if !skip.iter().any(|pattern| skips_folder(pattern, &path)) {
                    pending.push((file, path));
                }
            } else if !kind.is_file() {
                // Neither: nothing `include_bytes!` reads.
            } else if skip.iter().any(|pattern| matches(pattern, &path)) {
                walk.skipped.push(path);
            } else {
                walk.files.push(Entry { path, file });
            }
        }
    }
    walk.files.sort_by(|a, b| a.path.cmp(&b.path));
    walk.skipped.sort();
    walk.folders.sort();
    same_but_case(walk.files.iter().map(|entry| entry.path.as_str()))?;
    Ok(walk)
}

/// What is at `file`, following a link as `include_bytes!` will; `None`
/// when it is gone, deleted since its folder was listed (a build running
/// beside the walk). Any other failure is an error.
fn kind_of(file: &Path) -> Result<Option<fs::Metadata>, Error> {
    gone_is_none(fs::metadata(file)).map_err(|error| Error::Read {
        path: file.to_path_buf(),
        error,
    })
}

/// `result`, with "not found" read as `None`.
fn gone_is_none<T>(result: io::Result<T>) -> io::Result<Option<T>> {
    match result {
        Ok(found) => Ok(Some(found)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Whether skip pattern `pattern` matches every path below folder `folder`:
/// it is `**`, or ends `/**` after parts that match the folder whole
/// (`**/target/**` and `target`, `game/target`). A pattern that matches
/// only some of a folder's files leaves it entered.
fn skips_folder(pattern: &str, folder: &str) -> bool {
    let pattern: Vec<&str> = pattern.split('/').collect();
    let folder: Vec<&str> = folder.split('/').collect();
    match pattern.split_last() {
        Some((&"**", before)) => before.is_empty() || parts_match(before, &folder),
        _ => false,
    }
}

/// The first two of `paths` that differ only by case, as an error.
fn same_but_case<'a>(paths: impl Iterator<Item = &'a str>) -> Result<(), Error> {
    let mut folded: Vec<(String, &str)> = paths.map(|p| (p.to_lowercase(), p)).collect();
    folded.sort();
    for pair in folded.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(Error::SameButCase(
                pair[0].1.to_owned(),
                pair[1].1.to_owned(),
            ));
        }
    }
    Ok(())
}

/// Whether skip pattern `pattern` matches path `path` (both `/`-separated),
/// whole: in a part between slashes, `*` is any run of characters and `?` is
/// one; a part `**` is any number of folders, none too. `**/README.md` is
/// every `README.md`; `README.md` is only the folder's own.
#[must_use]
pub fn matches(pattern: &str, path: &str) -> bool {
    let pattern: Vec<&str> = pattern.split('/').collect();
    let path: Vec<&str> = path.split('/').collect();
    parts_match(&pattern, &path)
}

/// Whether the parts of a pattern match the parts of a path.
fn parts_match(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((&"**", rest)) => (0..=path.len()).any(|skip| parts_match(rest, &path[skip..])),
        Some((first, rest)) => path
            .split_first()
            .is_some_and(|(part, more)| part_matches(first, part) && parts_match(rest, more)),
    }
}

/// Whether one part of a pattern (`*` and `?`) matches one part of a path.
fn part_matches(pattern: &str, part: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let part: Vec<char> = part.chars().collect();
    chars_match(&pattern, &part)
}

/// Whether the characters of a pattern match the characters of a part.
fn chars_match(pattern: &[char], part: &[char]) -> bool {
    match pattern.split_first() {
        None => part.is_empty(),
        Some(('*', rest)) => (0..=part.len()).any(|skip| chars_match(rest, &part[skip..])),
        Some((first, rest)) => part
            .split_first()
            .is_some_and(|(c, more)| (*first == '?' || first == c) && chars_match(rest, more)),
    }
}

/// The Rust expression of a table of `files`: a
/// `&'static [(&'static str, &'static [u8])]` of `include_bytes!` with
/// absolute paths, in their order.
#[must_use]
pub fn table_source(files: &[Entry]) -> String {
    let mut source = String::from("&[\n");
    for entry in files {
        let file = entry.file.to_string_lossy();
        let _ = writeln!(
            source,
            "    ({:?}, include_bytes!({file:?}) as &[u8]),",
            entry.path
        );
    }
    source.push(']');
    source
}

/// The Rust expression of a list of `paths`: a `&'static [&'static str]`.
#[must_use]
pub fn paths_source(paths: &[String]) -> String {
    let mut source = String::from("&[\n");
    for path in paths {
        let _ = writeln!(source, "    {path:?},");
    }
    source.push(']');
    source
}

/// A table a build script writes: the files of one folder.
#[derive(Debug, Clone)]
pub struct Table {
    name: String,
    dir: PathBuf,
    skip: Vec<String>,
}

/// What [`Table::write_to`] wrote, and what a build watches for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// The table, which [`files!`] includes.
    pub table: PathBuf,
    /// The skipped paths, which [`skipped!`] includes.
    pub skipped: PathBuf,
    /// The folder and every file and folder in it, skipped files too, but
    /// not a folder a pattern skips whole, nor anything in it.
    pub watched: Vec<PathBuf>,
}

impl Table {
    /// A table named `name` ([`files!`] names it), of the folder of the
    /// same name until [`dir`](Self::dir) says another.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            dir: PathBuf::from(&name),
            name,
            skip: Vec::new(),
        }
    }

    /// The folder it embeds, relative to the crate's folder (or absolute).
    #[must_use]
    pub fn dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dir = dir.into();
        self
    }

    /// Leaves out the files `pattern` matches ([`matches()`]), and lists
    /// them for [`skipped!`], for `Layer::leaving_out`. A folder it
    /// matches whole (`**/target/**`) isn't entered, and its files aren't
    /// listed ([`walk`]).
    #[must_use]
    pub fn skip(mut self, pattern: impl Into<String>) -> Self {
        self.skip.push(pattern.into());
        self
    }

    /// Writes the table into `$OUT_DIR` from the crate's folder, and prints
    /// the `cargo::rerun-if-changed` lines that rebuild it. Call it from a
    /// build script.
    ///
    /// # Panics
    ///
    /// When the folder can't be embedded ([`Error`]): the build fails with
    /// the path.
    pub fn write(self) {
        let var = |name: &str| std::env::var_os(name).map_or_default(PathBuf::from);
        let report = self.write_and_report(
            &var("CARGO_MANIFEST_DIR"),
            &var("OUT_DIR"),
            &mut io::stdout().lock(),
        );
        if let Err(error) = report {
            panic!("{error}");
        }
    }

    /// What [`write`](Self::write) does, with the folders given: writes the
    /// table, then puts a `cargo::rerun-if-changed` line in `out` for each
    /// watched path. The error names the table.
    fn write_and_report(
        &self,
        crate_dir: &Path,
        out_dir: &Path,
        out: &mut impl io::Write,
    ) -> Result<(), String> {
        let written = self
            .write_to(crate_dir, out_dir)
            .map_err(|error| format!("shuyi-embed: table `{}`: {error}", self.name))?;
        for path in &written.watched {
            // Cargo reads these lines; a failed write is a failed build.
            let _ = writeln!(out, "cargo::rerun-if-changed={}", path.display());
        }
        Ok(())
    }

    /// Writes the table of the folder, taken from `crate_dir`, into
    /// `out_dir`: `shuyi_embed_<name>.rs` and `shuyi_embed_<name>_skipped.rs`.
    pub fn write_to(&self, crate_dir: &Path, out_dir: &Path) -> Result<Written, Error> {
        let fine = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
        if self.name.is_empty() || !self.name.chars().all(fine) {
            return Err(Error::BadName(self.name.clone()));
        }
        let root = crate_dir.join(&self.dir);
        let walk = walk(&root, &self.skip)?;
        let table = out_dir.join(format!("shuyi_embed_{}.rs", self.name));
        let skipped = out_dir.join(format!("shuyi_embed_{}_skipped.rs", self.name));
        let write = |path: &Path, source: String| {
            fs::write(path, source).map_err(|error| Error::Read {
                path: path.to_path_buf(),
                error,
            })
        };
        write(&table, table_source(&walk.files))?;
        write(&skipped, paths_source(&walk.skipped))?;
        let mut watched = walk.folders;
        watched.extend(walk.files.into_iter().map(|entry| entry.file));
        watched.extend(walk.skipped.iter().map(|path| root.join(path)));
        watched.sort();
        Ok(Written {
            table,
            skipped,
            watched,
        })
    }
}

/// The table [`Table`] named `$name` wrote: a
/// `&'static [(&'static str, &'static [u8])]` sorted by path.
#[macro_export]
macro_rules! files {
    ($name:literal) => {
        include!(concat!(env!("OUT_DIR"), "/shuyi_embed_", $name, ".rs"))
    };
}

/// The paths [`Table::skip`] left out of the table named `$name`: a
/// `&'static [&'static str]` of files, for `Layer::leaving_out`; not those
/// of a folder a pattern skips whole, which the walk doesn't enter.
#[macro_export]
macro_rules! skipped {
    ($name:literal) => {
        include!(concat!(
            env!("OUT_DIR"),
            "/shuyi_embed_",
            $name,
            "_skipped.rs"
        ))
    };
}

#[cfg(test)]
mod tests;
