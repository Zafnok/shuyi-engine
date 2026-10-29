//! The asset bundle: the files a game embeds in its binary, read from
//! layers the game gives, in order. A file is read from the first layer
//! that has it, so a layer laid over another replaces its files at the same
//! paths and adds its own; a listing merges every layer's files.
//!
//! A layer may also come from outside the binary: [`Layer::owned`] takes
//! files handed in at run time (a player's mod, a level they made), kept
//! for the life of the program, so a game's loaders read them as they read
//! its own. `shuyi-app`'s `read_folder` reads a folder on disk into one.
//!
//! Two flags shape a layer. A **private** layer holds files that may not be
//! handed out (bought art): [`Bundle::is_private`] tells a tool which shown
//! names come from one. A **test** layer holds the files of the tests and
//! debug tools, which a build that ships leaves out: test layers are read
//! after every other layer, and [`story_only`] hides them from a thread.
//!
//! A layer may also name the files it **leaves out** on purpose
//! ([`Layer::leaving_out`]): a smaller build (a handheld's) that keeps only
//! what it uses. [`Bundle::left_out`] lets a loader tell such a file from
//! one that is missing.
//!
//! A bundle's [`fingerprint`](Bundle::fingerprint) stands for the content
//! that ships, the same on every platform: what a record or a save is
//! stamped with (`shuyi_core::made`).
//!
//! The game registers its bundle once ([`install`] or [`get_or_install`]),
//! and its loaders read files as `&'static` data from it. A front end can
//! [`watch`] every file read, to log or time what it loads; with `std`,
//! [`reading`] makes one thread read another bundle (a test of a smaller
//! build's bundle).
use alloc::collections::BTreeMap;

use shuyi_core::ContentHash;

use crate::error::ContentError;
use crate::prelude::*;

mod fingerprint;
/// The fixed bundle whose fingerprint `formats.ron` pins (`format::golden_tests`).
#[cfg(test)]
pub(crate) use fingerprint::tests::fixed as fixed_fingerprint_bundle;

#[cfg(feature = "std")]
use include_dir::Dir;
use once_cell::race::OnceBox;

/// Where a layer's files are.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Files {
    /// A directory embedded by `include_dir!`.
    #[cfg(feature = "std")]
    Dir(&'static Dir<'static>),
    /// A table of `(path, contents)` sorted by path (relative to the
    /// layer's root, `/`-separated): what a build script writes where
    /// `include_dir` does not build (without `std`).
    Table(&'static [(&'static str, &'static [u8])]),
    /// Files handed in at run time ([`Layer::owned`]): a player's mod, a
    /// folder read from disk.
    Owned(&'static OwnedFiles),
}

/// The files of a layer handed in at run time ([`Layer::owned`]), with
/// checked paths, kept for the life of the program so every read of them
/// is `&'static`.
#[derive(Debug)]
pub struct OwnedFiles {
    /// `(path, contents)`, sorted by path.
    table: &'static [(&'static str, &'static [u8])],
}

impl Files {
    /// The contents of the file at `path`.
    fn get(self, path: &str) -> Option<&'static [u8]> {
        match self {
            #[cfg(feature = "std")]
            Self::Dir(dir) => dir.get_file(path).map(include_dir::File::contents),
            Self::Table(files) => table_get(files, path),
            Self::Owned(files) => table_get(files.table, path),
        }
    }

    /// Adds the paths of the files inside `dir` to `paths`: directly inside
    /// it, or with `deep` also in any directory below it.
    fn list(self, dir: &str, deep: bool, paths: &mut Vec<&'static str>) {
        match self {
            #[cfg(feature = "std")]
            Self::Dir(root) => {
                let root = if dir.is_empty() {
                    Some(root)
                } else {
                    root.get_dir(dir)
                };
                let mut pending: Vec<&'static Dir<'static>> = root.into_iter().collect();
                while let Some(dir) = pending.pop() {
                    paths.extend(dir.files().filter_map(|f| f.path().to_str()));
                    if deep {
                        pending.extend(dir.dirs());
                    }
                }
            }
            Self::Table(files) => table_list(files, dir, deep, paths),
            Self::Owned(files) => table_list(files.table, dir, deep, paths),
        }
    }
}

/// The contents of the file at `path` in `files` (sorted by path).
fn table_get(files: &'static [(&'static str, &'static [u8])], path: &str) -> Option<&'static [u8]> {
    let at = files.binary_search_by(|(p, _)| (*p).cmp(path)).ok()?;
    Some(files[at].1)
}

/// Adds the paths of the files of `files` inside `dir` to `paths`, as
/// [`Files::list`] does.
fn table_list(
    files: &'static [(&'static str, &'static [u8])],
    dir: &str,
    deep: bool,
    paths: &mut Vec<&'static str>,
) {
    paths.extend(files.iter().map(|(path, _)| *path).filter(|path| {
        let rest = if dir.is_empty() {
            Some(*path)
        } else {
            path.strip_prefix(dir)
                .and_then(|rest| rest.strip_prefix('/'))
        };
        rest.is_some_and(|rest| deep || !rest.contains('/'))
    }));
}

/// Why a path handed to [`Layer::owned`] was refused: a layer's paths are
/// relative to its root and `/`-separated, as every read names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BadPath {
    /// The path is empty.
    #[error("the path is empty")]
    Empty,
    /// The path starts at a root (`/data/x.ron`, `C:/data/x.ron`).
    #[error("the path is absolute; it must be relative to the layer's root")]
    Absolute,
    /// The path has a backslash: paths are `/`-separated.
    #[error("the path has a backslash; paths are `/`-separated")]
    Backslash,
    /// The path goes up a folder (`..`).
    #[error("the path goes up a folder (`..`)")]
    Parent,
    /// The path has an empty or `.` part (`data//x.ron`, `./x.ron`,
    /// `data/`), which no read names.
    #[error("the path has an empty or `.` part")]
    NotNormal,
}

/// A layer that could not be made ([`Layer::owned`]): one of its paths was
/// refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("layer `{layer}`: `{path}`: {problem}")]
#[non_exhaustive]
pub struct LayerError {
    /// The layer's name.
    pub layer: String,
    /// The path refused.
    pub path: String,
    /// What is wrong with it.
    pub problem: BadPath,
}

/// What is wrong with `path` as a path of a layer's file, if anything.
fn check_path(path: &str) -> Result<(), BadPath> {
    if path.is_empty() {
        return Err(BadPath::Empty);
    }
    if path.contains('\\') {
        return Err(BadPath::Backslash);
    }
    let first = path.split('/').next().unwrap_or(path);
    let drive = matches!(first.as_bytes(), [letter, b':'] if letter.is_ascii_alphabetic());
    if path.starts_with('/') || drive {
        return Err(BadPath::Absolute);
    }
    for part in path.split('/') {
        match part {
            ".." => return Err(BadPath::Parent),
            "" | "." => return Err(BadPath::NotNormal),
            _ => {}
        }
    }
    Ok(())
}

/// One layer of the bundle.
#[derive(Debug, Clone, Copy)]
pub struct Layer {
    /// The prefix its files are shown with (`base` shows `data/x.ron` as
    /// `base/data/x.ron`).
    pub name: &'static str,
    /// Its files.
    pub files: Files,
    /// Whether its files may not be handed out (bought art).
    pub private: bool,
    /// Whether it holds test files, which a build that ships leaves out.
    pub test: bool,
    /// The paths (sorted) of the files this layer leaves out on purpose.
    pub left_out: &'static [&'static str],
    /// The fingerprint of the content this layer stands for, if it holds
    /// every file of a bundle (a pack's layer, [`Layer::standing_for`]):
    /// [`Bundle::fingerprint`] then takes that, and not its files.
    pub stands_for: Option<ContentHash>,
}

impl Layer {
    /// A layer of the directory `dir`, shown as `name`.
    #[cfg(feature = "std")]
    pub const fn dir(name: &'static str, dir: &'static Dir<'static>) -> Self {
        Self::of(name, Files::Dir(dir))
    }

    /// A layer of the table `files` (sorted by path), shown as `name`.
    pub const fn table(
        name: &'static str,
        files: &'static [(&'static str, &'static [u8])],
    ) -> Self {
        Self::of(name, Files::Table(files))
    }

    /// A layer of `files` handed in at run time (a mod, a folder read from
    /// disk), each at its path (relative to the layer's root,
    /// `/`-separated), shown as `name`. The name, paths and bytes are kept
    /// for the life of the program (leaked once), so reads of them are
    /// `&'static` like the embedded layers'. Make each layer once.
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use shuyi_assets::bundle::{Bundle, Layer};
    ///
    /// static BASE: &[(&str, &[u8])] = &[("data/a.ron", b"base a")];
    /// let files = BTreeMap::from([("data/a.ron".to_owned(), b"mod a".to_vec())]);
    /// let layer = Layer::owned("mods/more".to_owned(), files).unwrap();
    /// let bundle = Bundle::new(vec![layer, Layer::table("base", BASE)]);
    /// assert_eq!(bundle.file("data/a.ron"), Some("mod a"));
    /// assert_eq!(bundle.display_path("data/a.ron"), "mods/more/data/a.ron");
    /// ```
    ///
    /// # Errors
    ///
    /// A path that is empty, absolute, has a backslash, goes up a folder
    /// (`..`) or has an empty or `.` part, naming the layer and the path.
    /// Nothing is kept then.
    pub fn owned(name: String, files: BTreeMap<String, Vec<u8>>) -> Result<Self, LayerError> {
        for path in files.keys() {
            check_path(path).map_err(|problem| LayerError {
                layer: name.clone(),
                path: path.clone(),
                problem,
            })?;
        }
        // A `BTreeMap` is sorted by path, as the table's search needs.
        let table: Vec<(&'static str, &'static [u8])> = files
            .into_iter()
            .map(|(path, bytes)| {
                let path: &'static str = Box::leak(path.into_boxed_str());
                let bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
                (path, bytes)
            })
            .collect();
        let files: &'static OwnedFiles = Box::leak(Box::new(OwnedFiles {
            table: Box::leak(table.into_boxed_slice()),
        }));
        Ok(Self::of(
            Box::leak(name.into_boxed_str()),
            Files::Owned(files),
        ))
    }

    /// A layer of `files`, shown as `name`.
    const fn of(name: &'static str, files: Files) -> Self {
        Self {
            name,
            files,
            private: false,
            test: false,
            left_out: &[],
            stands_for: None,
        }
    }

    /// This layer, marked private.
    #[must_use]
    pub const fn private(self) -> Self {
        Self {
            private: true,
            ..self
        }
    }

    /// This layer, marked as test files.
    #[must_use]
    pub const fn test(self) -> Self {
        Self { test: true, ..self }
    }

    /// This layer, leaving out the files at `paths` (sorted, relative to
    /// its root) on purpose: they are the game's, but this build does
    /// without them.
    #[must_use]
    pub const fn leaving_out(self, paths: &'static [&'static str]) -> Self {
        Self {
            left_out: paths,
            ..self
        }
    }

    /// This layer, standing for the bundle whose fingerprint is `content`
    /// ([`Bundle::fingerprint`]): it holds every file of that bundle, as a
    /// pack does of the bundle it was baked from, so a bundle with it has
    /// that bundle's fingerprint.
    #[must_use]
    pub const fn standing_for(self, content: ContentHash) -> Self {
        Self {
            stands_for: Some(content),
            ..self
        }
    }
}

/// The layers a game reads its files from.
#[derive(Debug, Clone)]
pub struct Bundle {
    /// Every layer: those that ship first, then the test layers.
    layers: Vec<Layer>,
    /// How many layers ship (they come first).
    shipped: usize,
}

impl Bundle {
    /// The bundle of `layers`, read in the order given, except that the
    /// test layers are read after all the others.
    pub fn new(mut layers: Vec<Layer>) -> Self {
        // A stable sort: each group keeps the order it was given in.
        layers.sort_by_key(|layer| layer.test);
        let shipped = layers.iter().filter(|layer| !layer.test).count();
        Self { layers, shipped }
    }

    /// Every layer, in the order files are read from them.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// The layers that ship: all but the test layers.
    fn shipped(&self) -> &[Layer] {
        &self.layers[..self.shipped]
    }

    /// The test layers.
    fn tests(&self) -> &[Layer] {
        &self.layers[self.shipped..]
    }

    /// The layers this thread reads: all of them, or inside [`story_only`]
    /// those that ship.
    fn read(&self) -> &[Layer] {
        if story_only_now() {
            self.shipped()
        } else {
            &self.layers
        }
    }

    /// The layer files are shown under when no layer has them: the first
    /// that is neither private nor test files.
    fn home(&self) -> Option<&Layer> {
        self.layers
            .iter()
            .find(|layer| !layer.private && !layer.test)
    }

    /// Returns the UTF-8 contents of the file at `path` (relative to a
    /// layer's root, `/`-separated), or `None` if it is missing or not
    /// valid UTF-8.
    pub fn file(&self, path: &str) -> Option<&'static str> {
        core::str::from_utf8(self.bytes(path)?).ok()
    }

    /// Returns the raw bytes of the file at `path` (relative to a layer's
    /// root, `/`-separated), e.g. an image, or `None` if it is missing.
    pub fn bytes(&self, path: &str) -> Option<&'static [u8]> {
        let bytes = find(self.read(), path).map(|(_, bytes)| bytes);
        if let Some(watcher) = WATCHER.get() {
            watcher(path, bytes.is_some());
        }
        bytes
    }

    /// Whether a layer this thread reads has the file at `path`. Unlike
    /// [`Bundle::bytes`], this reads nothing: the [`watch`]er isn't told.
    pub fn has(&self, path: &str) -> bool {
        find(self.read(), path).is_some()
    }

    /// Whether a layer this thread reads leaves the file at `path` out on
    /// purpose ([`Layer::leaving_out`]): a loader then does without it
    /// instead of reporting it missing.
    pub fn left_out(&self, path: &str) -> bool {
        self.read()
            .iter()
            .any(|layer| layer.left_out.binary_search(&path).is_ok())
    }

    /// The test layers' own file at `path`, with its human-facing name,
    /// when a layer that ships has a file there too: the entries a data
    /// table gains in a build with the test files. [`Bundle::file`] reads
    /// the shipped one. `None` without test layers, inside [`story_only`],
    /// and for a path only one of the two has.
    pub fn test_part(&self, path: &str) -> Option<(String, &'static str)> {
        if story_only_now() {
            return None;
        }
        find(self.shipped(), path)?;
        let (layer, bytes) = find(self.tests(), path)?;
        let display = format!("{}/{path}", layer.name);
        Some((display, core::str::from_utf8(bytes).ok()?))
    }

    /// Lists the paths (relative to a layer's root, `/`-separated, sorted,
    /// each once) of every file directly inside directory `dir`, in any
    /// layer. A missing directory yields an empty list. Use `""` for the
    /// root.
    pub fn files_in(&self, dir: &str) -> Vec<&'static str> {
        list(self.read(), dir, false)
    }

    /// Lists the paths (relative to a layer's root, `/`-separated, sorted,
    /// each once) of every file inside directory `dir` or any directory
    /// below it, in any layer. A missing directory yields an empty list.
    /// Use `""` for the whole bundle.
    pub fn files_under(&self, dir: &str) -> Vec<&'static str> {
        list(self.read(), dir, true)
    }

    /// Parses every file of extension `ext` (without the dot) directly inside
    /// `dir` with `parse`, into a map by file name without the extension.
    /// A file that is not valid UTF-8 is an error of `E` ([`NotText`]); every
    /// broken file is reported, not only the first: the result is either the
    /// whole map or all the errors, in file order.
    ///
    /// ```
    /// use shuyi_assets::bundle::{Bundle, Layer};
    /// use shuyi_assets::ContentError;
    ///
    /// static FILES: &[(&str, &[u8])] = &[("crops/a.txt", b"1"), ("crops/b.txt", b"2")];
    /// let bundle = Bundle::new(vec![Layer::table("base", FILES)]);
    /// let crops = bundle.load_each("crops", "txt", |file| {
    ///     file.text
    ///         .parse::<u32>()
    ///         .map_err(|e| vec![ContentError::new(file.path, e.to_string())])
    /// });
    /// assert_eq!(crops.unwrap().get("b"), Some(&2));
    /// ```
    ///
    /// # Errors
    ///
    /// Every error `parse` returned, and a [`NotText`] for each file that
    /// is not UTF-8.
    pub fn load_each<T, E: From<NotText>>(
        &self,
        dir: &str,
        ext: &str,
        mut parse: impl FnMut(&LoadFile<'_>) -> Result<T, Vec<E>>,
    ) -> Result<BTreeMap<String, T>, Vec<E>> {
        let mut loaded = BTreeMap::new();
        let mut errors = Vec::new();
        for path in self.files_in(dir) {
            let name = path.rsplit('/').next().unwrap_or(path);
            let Some(stem) = name
                .strip_suffix(ext)
                .and_then(|stem| stem.strip_suffix('.'))
            else {
                continue;
            };
            let shown = self.display_path(path);
            let Some(text) = self.file(path) else {
                errors.push(NotText { path: shown }.into());
                continue;
            };
            match parse(&LoadFile {
                stem,
                path: &shown,
                text,
            }) {
                Ok(value) => {
                    loaded.insert(stem.to_owned(), value);
                }
                Err(more) => errors.extend(more),
            }
        }
        if errors.is_empty() {
            Ok(loaded)
        } else {
            Err(errors)
        }
    }

    /// The human-facing name of `path`: under the name of the layer it is
    /// read from (`base/data/x.ron`), or of the first layer that is neither
    /// private nor test files when no layer has it.
    pub fn display_path(&self, path: &str) -> String {
        let layer = find(self.read(), path).map(|(layer, _)| layer);
        match layer.or_else(|| self.home()) {
            Some(layer) => format!("{}/{path}", layer.name),
            None => path.to_owned(),
        }
    }

    /// Whether `shown` (a human-facing name, from [`Bundle::display_path`])
    /// names a file of a private layer.
    pub fn is_private(&self, shown: &str) -> bool {
        self.layers
            .iter()
            .any(|layer| layer.private && shown_under(shown, layer.name))
    }
}

/// One file of a folder, as [`Bundle::load_each`] hands it to a parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadFile<'a> {
    /// The file's name without its folder and extension (`data/a.ron` is `a`).
    pub stem: &'a str,
    /// The file's human-facing name ([`Bundle::display_path`]), for errors.
    pub path: &'a str,
    /// The file's text.
    pub text: &'static str,
}

/// A file that is not valid UTF-8: what [`Bundle::load_each`] turns into
/// the loader's own error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotText {
    /// The file's human-facing name.
    pub path: String,
}

impl From<NotText> for ContentError {
    fn from(not_text: NotText) -> Self {
        Self::new(not_text.path, "file is not valid UTF-8").with_code(crate::error::codes::NOT_TEXT)
    }
}

/// Whether `shown` (a human-facing name) names a file under the layer
/// named `name`: `base/x.png` is under `base`, `bases/x.png` and `base`
/// are not.
pub fn shown_under(shown: &str, name: &str) -> bool {
    shown
        .strip_prefix(name)
        .is_some_and(|rest| rest.starts_with('/'))
}

/// The file at `path` and the layer it is read from: the first of `layers`
/// that has it.
fn find<'a>(layers: &'a [Layer], path: &str) -> Option<(&'a Layer, &'static [u8])> {
    layers
        .iter()
        .find_map(|layer| Some((layer, layer.files.get(path)?)))
}

/// The paths (sorted, each once) of the files of every layer inside `dir`:
/// directly inside it, or with `deep` also in any directory below it.
fn list(layers: &[Layer], dir: &str, deep: bool) -> Vec<&'static str> {
    let mut paths = Vec::new();
    for layer in layers {
        layer.files.list(dir, deep, &mut paths);
    }
    paths.sort_unstable();
    paths.dedup();
    paths
}

/// The bundle the game installed.
static INSTALLED: OnceBox<Bundle> = OnceBox::new();

/// Makes `bundle` the process-wide bundle. Done once: a second call gives
/// its bundle back and changes nothing.
pub fn install(bundle: Bundle) -> Result<(), Bundle> {
    INSTALLED.set(Box::new(bundle)).map_err(|bundle| *bundle)
}

/// The process-wide bundle, once installed; inside [`reading`], the
/// bundle this thread reads instead.
pub fn installed() -> Option<&'static Bundle> {
    #[cfg(feature = "std")]
    if let Some(bundle) = READING.get() {
        return Some(bundle);
    }
    INSTALLED.get()
}

/// The process-wide bundle, installing the one `make` builds if none is.
/// When two threads race to install, one bundle wins and both get it.
/// Inside [`reading`], the bundle this thread reads instead, and nothing is
/// installed.
pub fn get_or_install(make: impl FnOnce() -> Bundle) -> &'static Bundle {
    #[cfg(feature = "std")]
    if let Some(bundle) = READING.get() {
        return bundle;
    }
    INSTALLED.get_or_init(|| Box::new(make()))
}

/// What is told of every file read ([`watch`]): its path, and whether the
/// bundle has it.
pub type Watcher = fn(path: &str, found: bool);

/// The watcher of every file read, once one is set.
static WATCHER: OnceBox<Watcher> = OnceBox::new();

/// Tells `watcher` of every file any bundle reads from now on
/// ([`Bundle::bytes`] and [`Bundle::file`]), on the thread that reads it:
/// for a front end that logs or times what it loads. Set once: a second
/// call gives its watcher back and changes nothing.
pub fn watch(watcher: Watcher) -> Result<(), Watcher> {
    WATCHER.set(Box::new(watcher)).map_err(|watcher| *watcher)
}

#[cfg(feature = "std")]
std::thread_local! {
    /// The bundle this thread reads inside [`reading`].
    static READING: core::cell::Cell<Option<&'static Bundle>> =
        const { core::cell::Cell::new(None) };
}

/// Runs `f` with this thread reading `bundle` instead of the installed one
/// ([`installed`], [`get_or_install`]): for a test of another build's
/// bundle (a handheld's smaller one) in a process that installed the
/// whole. Other threads are never affected.
#[cfg(feature = "std")]
pub fn reading<T>(bundle: &'static Bundle, f: impl FnOnce() -> T) -> T {
    /// Puts the old bundle back, also when `f` panics.
    struct Restore(Option<&'static Bundle>);
    impl Drop for Restore {
        fn drop(&mut self) {
            READING.set(self.0);
        }
    }
    let _restore = Restore(READING.replace(Some(bundle)));
    f()
}

#[cfg(feature = "std")]
std::thread_local! {
    /// Whether this thread is inside [`story_only`].
    static STORY_ONLY: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
}

/// Whether [`story_only`] is running. Without `std` there is one thread.
#[cfg(not(feature = "std"))]
static STORY_ONLY: Flag = Flag::new();

/// A `bool` a `static` can hold, with the `get` and `replace` of the
/// thread-local `Cell` the `std` build uses. Built for the tests too, so
/// they check the `no_std` build's flag.
#[cfg(any(not(feature = "std"), test))]
struct Flag(core::sync::atomic::AtomicBool);

#[cfg(any(not(feature = "std"), test))]
impl Flag {
    /// A flag that is off.
    const fn new() -> Self {
        Self(core::sync::atomic::AtomicBool::new(false))
    }

    /// Whether it is on.
    fn get(&self) -> bool {
        self.0.load(core::sync::atomic::Ordering::Relaxed)
    }

    /// Sets it to `on`; returns the old value.
    fn replace(&self, on: bool) -> bool {
        self.0.swap(on, core::sync::atomic::Ordering::Relaxed)
    }
}

/// Whether this thread is inside [`story_only`].
fn story_only_now() -> bool {
    STORY_ONLY.get()
}

/// Sets whether this thread is inside [`story_only`]; returns the old value.
fn set_story_only(on: bool) -> bool {
    STORY_ONLY.replace(on)
}

/// Runs `f` with the test layers of every bundle hidden from this thread:
/// each answers as it does in a build that ships. For the tests that check
/// what such a build holds.
pub fn story_only<T>(f: impl FnOnce() -> T) -> T {
    /// Puts the old value back, also when `f` panics.
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            set_story_only(self.0);
        }
    }
    let _restore = Restore(set_story_only(true));
    f()
}

#[cfg(test)]
mod tests {
    use include_dir::{DirEntry, File};

    use super::*;

    const fn entry(path: &'static str, contents: &'static str) -> DirEntry<'static> {
        DirEntry::File(File::new(path, contents.as_bytes()))
    }

    /// A private layer laid over the base one.
    static PRIVATE_FIXTURE: Dir<'static> = Dir::new(
        "",
        &[
            DirEntry::Dir(Dir::new("data", &[entry("data/a.ron", "private a")])),
            DirEntry::Dir(Dir::new(
                "portraits",
                &[entry("portraits/new.portrait", "private only")],
            )),
            DirEntry::Dir(Dir::new(
                "deep",
                &[DirEntry::Dir(Dir::new(
                    "deep/x",
                    &[entry("deep/x/y.txt", "private y")],
                ))],
            )),
        ],
    );

    /// The base layer.
    static PUBLIC_FIXTURE: Dir<'static> = Dir::new(
        "",
        &[
            entry("top.txt", "public top"),
            DirEntry::Dir(Dir::new(
                "data",
                &[
                    entry("data/a.ron", "public a"),
                    entry("data/b.ron", "public b"),
                ],
            )),
            DirEntry::Dir(Dir::new(
                "deep",
                &[DirEntry::Dir(Dir::new(
                    "deep/x",
                    &[entry("deep/x/z.txt", "public z")],
                ))],
            )),
        ],
    );

    /// [`PUBLIC_FIXTURE`] as a table.
    static PUBLIC_TABLE: &[(&str, &[u8])] = &[
        ("data/a.ron", b"public a"),
        ("data/b.ron", b"public b"),
        ("deep/x/z.txt", b"public z"),
        ("top.txt", b"public top"),
    ];

    /// The private test files, laid over the test files.
    static PRIVATE_TEST_FIXTURE: Dir<'static> = Dir::new(
        "",
        &[DirEntry::Dir(Dir::new(
            "portraits",
            &[entry("portraits/t.png", "bought t")],
        ))],
    );

    /// The test files.
    static TEST_FIXTURE: Dir<'static> = Dir::new(
        "",
        &[
            DirEntry::Dir(Dir::new(
                "data",
                &[entry("data/b.ron", "test b"), entry("data/t.ron", "test t")],
            )),
            DirEntry::Dir(Dir::new(
                "portraits",
                &[entry("portraits/t.png", "placeholder t")],
            )),
        ],
    );

    const PRIVATE: Layer = Layer::dir("bought", &PRIVATE_FIXTURE).private();
    const PUBLIC: Layer = Layer::dir("base", &PUBLIC_FIXTURE);
    const PRIVATE_TEST: Layer = Layer::dir("bought-tests", &PRIVATE_TEST_FIXTURE)
        .private()
        .test();
    const TEST: Layer = Layer::dir("tests", &TEST_FIXTURE).test();

    /// The private layer laid over the public one.
    fn merged() -> Bundle {
        Bundle::new(vec![PRIVATE, PUBLIC])
    }

    /// Every layer.
    fn with_tests() -> Bundle {
        Bundle::new(vec![PRIVATE, PUBLIC, PRIVATE_TEST, TEST])
    }

    fn names(layers: &[Layer]) -> Vec<&'static str> {
        layers.iter().map(|layer| layer.name).collect()
    }

    #[test]
    fn the_test_layers_are_the_last_ones() {
        let bundle = with_tests();
        assert_eq!(names(bundle.shipped()), ["bought", "base"]);
        assert_eq!(names(bundle.tests()), ["bought-tests", "tests"]);
        // Given first, they are still read last, in the order given.
        let given = Bundle::new(vec![PRIVATE_TEST, PRIVATE, TEST, PUBLIC]);
        assert_eq!(
            names(given.layers()),
            ["bought", "base", "bought-tests", "tests"]
        );
        assert_eq!(names(merged().shipped()), names(merged().layers()));
        let bundle = merged();
        let got = bundle.tests();
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn a_test_file_is_added_after_the_shipped_files() {
        let bundle = with_tests();
        assert_eq!(bundle.file("data/t.ron"), Some("test t"));
        assert_eq!(bundle.display_path("data/t.ron"), "tests/data/t.ron");
        assert_eq!(story_only(|| bundle.file("data/t.ron")), None);
        // The shipped file is the one read where both have one.
        assert_eq!(bundle.file("data/b.ron"), Some("public b"));
        assert_eq!(
            bundle.files_in("data"),
            ["data/a.ron", "data/b.ron", "data/t.ron"]
        );
    }

    #[test]
    fn a_test_part_is_the_test_layers_own_file_where_a_shipped_one_has_one_too() {
        let bundle = with_tests();
        assert_eq!(
            bundle.test_part("data/b.ron"),
            Some(("tests/data/b.ron".to_owned(), "test b"))
        );
        // Only the test files have it: an ordinary file, read whole.
        assert_eq!(bundle.test_part("data/t.ron"), None);
        // Only a shipped layer has it.
        assert_eq!(bundle.test_part("data/a.ron"), None);
        assert_eq!(bundle.test_part("data/nope.ron"), None);
        // No test layers.
        assert_eq!(merged().test_part("data/b.ron"), None);
        assert_eq!(story_only(|| bundle.test_part("data/b.ron")), None);
    }

    #[test]
    fn a_private_test_file_replaces_the_placeholder_and_is_private() {
        let bundle = with_tests();
        assert_eq!(bundle.file("portraits/t.png"), Some("bought t"));
        let shown = |path| bundle.display_path(path);
        assert_eq!(shown("portraits/t.png"), "bought-tests/portraits/t.png");
        assert!(bundle.is_private(&shown("portraits/t.png")));
        assert!(bundle.is_private(&shown("data/a.ron")));
        assert!(bundle.is_private(&shown("portraits/new.portrait")));
        assert!(!bundle.is_private(&shown("data/b.ron")));
        assert!(!bundle.is_private(&shown("data/t.ron")));
        assert!(!bundle.is_private(&shown("data/nope.ron")));
        // Without the private test files the placeholder is back.
        let placeholders = Bundle::new(vec![PRIVATE, PUBLIC, TEST]);
        assert_eq!(placeholders.file("portraits/t.png"), Some("placeholder t"));
        let shown = placeholders.display_path("portraits/t.png");
        assert!(!placeholders.is_private(&shown));
        // Only a name under one of the private layers.
        assert!(bundle.is_private("bought/x.png"));
        assert!(bundle.is_private("bought-tests/x.png"));
        assert!(!bundle.is_private("boughts/x.png"));
        assert!(!bundle.is_private("bought"));
        assert!(!bundle.is_private("base/x.png"));
        assert!(!bundle.is_private("x/bought/x.png"));
    }

    #[test]
    fn a_private_file_replaces_the_public_file_at_the_same_path() {
        assert_eq!(merged().file("data/a.ron"), Some("private a"));
        assert_eq!(merged().display_path("data/a.ron"), "bought/data/a.ron");
        // Without the private layer the public file is back.
        let public = Bundle::new(vec![PUBLIC]);
        assert_eq!(public.file("data/a.ron"), Some("public a"));
    }

    #[test]
    fn a_private_only_file_is_added() {
        let path = "portraits/new.portrait";
        assert_eq!(merged().file(path), Some("private only"));
        assert_eq!(merged().files_in("portraits"), [path]);
        let public = Bundle::new(vec![PUBLIC]);
        assert_eq!(public.file(path), None);
        let listed = public.files_in("portraits");
        assert!(listed.is_empty(), "{listed:?}");
    }

    #[test]
    fn a_public_only_file_is_still_read() {
        assert_eq!(merged().file("data/b.ron"), Some("public b"));
        assert_eq!(merged().display_path("data/b.ron"), "base/data/b.ron");
    }

    #[test]
    fn a_file_in_no_layer_is_missing_and_shown_under_the_first_public_layer() {
        assert_eq!(merged().bytes("data/nope.ron"), None);
        assert_eq!(merged().display_path("data/nope.ron"), "base/data/nope.ron");
        // With no such layer, the path alone.
        let private = Bundle::new(vec![PRIVATE, TEST]);
        assert_eq!(private.display_path("data/nope.ron"), "data/nope.ron");
        assert_eq!(Bundle::new(vec![]).display_path("x"), "x");
    }

    #[test]
    fn lists_merge_the_layers_sorted_with_each_path_once() {
        let bundle = merged();
        assert_eq!(bundle.files_in("data"), ["data/a.ron", "data/b.ron"]);
        assert_eq!(bundle.files_in(""), ["top.txt"]);
        // `deep` has no files of its own, only in the directory below.
        let listed = bundle.files_in("deep");
        assert!(listed.is_empty(), "{listed:?}");
        assert_eq!(bundle.files_under("deep"), ["deep/x/y.txt", "deep/x/z.txt"]);
        assert_eq!(
            bundle.files_under(""),
            [
                "data/a.ron",
                "data/b.ron",
                "deep/x/y.txt",
                "deep/x/z.txt",
                "portraits/new.portrait",
                "top.txt",
            ]
        );
        let listed = bundle.files_under("no_such_dir");
        assert!(listed.is_empty(), "{listed:?}");
    }

    /// A folder of two good files, a bad one, a binary one and another kind.
    static FOLDER: &[(&str, &[u8])] = &[
        ("n/a.num", b"1"),
        ("n/b.num", b"2"),
        ("n/bad.num", b"x"),
        ("n/bin.num", &[0xff]),
        ("n/other.txt", b"9"),
    ];

    fn parse_num(file: &LoadFile<'_>) -> Result<u32, Vec<ContentError>> {
        file.text
            .parse()
            .map_err(|_| vec![ContentError::new(file.path, "not a number")])
    }

    #[test]
    fn load_each_gathers_a_map_by_name_or_every_error() {
        let bundle = Bundle::new(vec![Layer::table("base", FOLDER)]);
        let errors = bundle.load_each("n", "num", parse_num).unwrap_err();
        let files: Vec<_> = errors.iter().map(ToString::to_string).collect();
        assert_eq!(files.len(), 2);
        assert!(files[0].contains("base/n/bad.num") && files[0].contains("not a number"));
        assert!(files[1].contains("base/n/bin.num") && files[1].contains("not valid UTF-8"));
        let good = Bundle::new(vec![Layer::table("base", &FOLDER[..2])]);
        let map = good.load_each("n", "num", parse_num).unwrap();
        assert_eq!(
            map.into_iter().collect::<Vec<_>>(),
            [("a".to_owned(), 1), ("b".to_owned(), 2)]
        );
        let none = good.load_each("nope", "num", parse_num).unwrap();
        assert!(none.is_empty(), "{none:?}");
    }

    /// A file that isn't UTF-8.
    static BINARY: &[(&str, &[u8])] = &[("x.bin", &[0xff, 0xfe])];

    /// A table laid over [`PUBLIC_FIXTURE`].
    static OVER: &[(&str, &[u8])] = &[("data/a.ron", b"table a")];

    #[test]
    fn reads_bytes_and_text() {
        let bundle = merged();
        assert_eq!(bundle.bytes("top.txt"), Some(&b"public top"[..]));
        assert_eq!(bundle.file("top.txt"), Some("public top"));
        let binary = Bundle::new(vec![Layer::table("base", BINARY)]);
        assert_eq!(binary.bytes("x.bin"), Some(&[0xff, 0xfe][..]));
        assert_eq!(binary.file("x.bin"), None);
    }

    /// A table layer answers every question as the directory of the same
    /// files does: the build without `std` reads the same bundle.
    #[test]
    fn a_table_layer_answers_as_the_same_directory() {
        let dir = Bundle::new(vec![PUBLIC]);
        let table = Bundle::new(vec![Layer::table("base", PUBLIC_TABLE)]);
        let all = dir.files_under("");
        assert_eq!(table.files_under(""), all);
        for path in all.iter().copied().chain(["nope", "data/nope.ron", ""]) {
            assert_eq!(table.bytes(path), dir.bytes(path), "{path}");
            assert_eq!(table.display_path(path), dir.display_path(path));
        }
        for dir_name in ["", "data", "deep", "deep/x", "dat", "no_such_dir"] {
            assert_eq!(
                table.files_in(dir_name),
                dir.files_in(dir_name),
                "{dir_name}"
            );
            assert_eq!(
                table.files_under(dir_name),
                dir.files_under(dir_name),
                "{dir_name}"
            );
        }
        // Over a directory layer, a table layer replaces its files too.
        let laid = Bundle::new(vec![Layer::table("over", OVER), PUBLIC]);
        assert_eq!(laid.file("data/a.ron"), Some("table a"));
        assert_eq!(laid.display_path("data/a.ron"), "over/data/a.ron");
        assert_eq!(laid.files_in("data"), ["data/a.ron", "data/b.ron"]);
    }

    /// A layer of `files` handed in at run time, shown as `name`.
    fn owned(name: &str, files: &[(&str, &str)]) -> Layer {
        let files = files
            .iter()
            .map(|(path, text)| ((*path).to_owned(), text.as_bytes().to_vec()))
            .collect();
        Layer::owned(name.to_owned(), files).unwrap()
    }

    #[test]
    fn a_run_time_file_replaces_an_embedded_one_and_a_new_one_is_added() {
        let layer = owned(
            "mods/more",
            &[
                ("data/a.ron", "mod a"),
                ("data/c.ron", "mod c"),
                ("x/y.txt", "mod y"),
            ],
        );
        let over_table = Bundle::new(vec![layer, Layer::table("base", PUBLIC_TABLE)]);
        for bundle in [&over_table, &Bundle::new(vec![layer, PUBLIC])] {
            assert_eq!(bundle.file("data/a.ron"), Some("mod a"));
            assert_eq!(bundle.file("data/c.ron"), Some("mod c"));
            assert_eq!(bundle.file("data/b.ron"), Some("public b"));
            assert_eq!(
                bundle.files_in("data"),
                ["data/a.ron", "data/b.ron", "data/c.ron"]
            );
            assert_eq!(bundle.files_in("x"), ["x/y.txt"]);
            assert_eq!(
                bundle.files_under(""),
                [
                    "data/a.ron",
                    "data/b.ron",
                    "data/c.ron",
                    "deep/x/z.txt",
                    "top.txt",
                    "x/y.txt",
                ]
            );
            assert!(bundle.has("x/y.txt"));
        }
        // Under the embedded layer, the embedded file is read.
        let under = Bundle::new(vec![PUBLIC, layer]);
        assert_eq!(under.file("data/a.ron"), Some("public a"));
        assert_eq!(under.file("data/c.ron"), Some("mod c"));
    }

    #[test]
    fn of_two_run_time_layers_the_first_wins() {
        let first = owned("first", &[("a.txt", "first a"), ("b.txt", "first b")]);
        let second = owned("second", &[("a.txt", "second a"), ("c.txt", "second c")]);
        let bundle = Bundle::new(vec![first, second]);
        assert_eq!(bundle.file("a.txt"), Some("first a"));
        assert_eq!(bundle.file("c.txt"), Some("second c"));
        assert_eq!(bundle.files_in(""), ["a.txt", "b.txt", "c.txt"]);
        let swapped = Bundle::new(vec![second, first]);
        assert_eq!(swapped.file("a.txt"), Some("second a"));
    }

    #[test]
    fn a_run_time_file_is_shown_under_its_layers_name() {
        let layer = owned("mods/more", &[("data/a.ron", "mod a")]);
        let bundle = Bundle::new(vec![layer, PUBLIC]);
        assert_eq!(bundle.display_path("data/a.ron"), "mods/more/data/a.ron");
        assert_eq!(bundle.display_path("data/b.ron"), "base/data/b.ron");
        // A missing file is shown under the first public layer: this one.
        assert_eq!(bundle.display_path("nope"), "mods/more/nope");
        assert_eq!(names(bundle.layers()), ["mods/more", "base"]);
    }

    #[test]
    fn a_run_time_layer_takes_the_flags_of_any_layer() {
        let bought = owned("bought", &[("data/a.ron", "bought a")]).private();
        let tests = owned("tests", &[("data/t.ron", "test t"), ("data/a.ron", "t")]).test();
        let bundle = Bundle::new(vec![tests, bought, PUBLIC]);
        assert_eq!(names(bundle.layers()), ["bought", "base", "tests"]);
        assert_eq!(bundle.file("data/a.ron"), Some("bought a"));
        assert!(bundle.is_private(&bundle.display_path("data/a.ron")));
        assert!(!bundle.is_private(&bundle.display_path("data/b.ron")));
        assert_eq!(bundle.file("data/t.ron"), Some("test t"));
        assert_eq!(story_only(|| bundle.file("data/t.ron")), None);
        let trimmed = owned("base", &[("a.txt", "a")]).leaving_out(&["b.txt"]);
        let bundle = Bundle::new(vec![trimmed]);
        assert!(bundle.left_out("b.txt"));
        assert!(!bundle.left_out("a.txt"));
    }

    /// A run-time layer answers every question as the table of the same
    /// files does.
    #[test]
    fn a_run_time_layer_answers_as_the_same_table() {
        let table = Bundle::new(vec![Layer::table("base", PUBLIC_TABLE)]);
        let files: Vec<(&str, &str)> = PUBLIC_TABLE
            .iter()
            .map(|(path, bytes)| (*path, core::str::from_utf8(bytes).unwrap_or_default()))
            .collect();
        let run_time = Bundle::new(vec![owned("base", &files)]);
        let all = table.files_under("");
        assert_eq!(run_time.files_under(""), all);
        for path in all.iter().copied().chain(["nope", "data/nope.ron", ""]) {
            assert_eq!(run_time.bytes(path), table.bytes(path), "{path}");
            assert_eq!(run_time.display_path(path), table.display_path(path));
        }
        for dir in ["", "data", "deep", "deep/x", "dat", "no_such_dir"] {
            assert_eq!(run_time.files_in(dir), table.files_in(dir), "{dir}");
            assert_eq!(run_time.files_under(dir), table.files_under(dir), "{dir}");
        }
        // No files at all: an empty layer.
        let empty = Layer::owned("empty".to_owned(), BTreeMap::new()).unwrap();
        let listed = Bundle::new(vec![empty]).files_under("");
        assert!(listed.is_empty(), "{listed:?}");
    }

    #[test]
    fn a_bad_path_is_refused_with_the_layers_name_and_the_path() {
        let cases = [
            ("", BadPath::Empty),
            ("/data/a.ron", BadPath::Absolute),
            ("C:/data/a.ron", BadPath::Absolute),
            ("c:", BadPath::Absolute),
            ("data\\a.ron", BadPath::Backslash),
            ("..", BadPath::Parent),
            ("../a.ron", BadPath::Parent),
            ("data/../../a.ron", BadPath::Parent),
            ("data//a.ron", BadPath::NotNormal),
            ("./a.ron", BadPath::NotNormal),
            ("data/./a.ron", BadPath::NotNormal),
            ("data/", BadPath::NotNormal),
        ];
        for (path, problem) in cases {
            let files = BTreeMap::from([
                ("good.txt".to_owned(), Vec::new()),
                (path.to_owned(), b"x".to_vec()),
            ]);
            let error = Layer::owned("mods/bad".to_owned(), files).unwrap_err();
            let expected = LayerError {
                layer: "mods/bad".to_owned(),
                path: path.to_owned(),
                problem,
            };
            assert_eq!(error, expected, "{path}");
            let message = error.to_string();
            assert!(
                message.contains("mods/bad") && message.contains(&format!("`{path}`")),
                "{message}"
            );
        }
        // Names that only look like those are fine.
        for path in [
            "a..b", "..a/b", "data/a.", "ab:/c", "data/c:", "-/x", "a b.txt",
        ] {
            let files = BTreeMap::from([(path.to_owned(), Vec::new())]);
            assert!(Layer::owned("ok".to_owned(), files).is_ok(), "{path}");
        }
    }

    proptest::proptest! {
        /// For any layers of any kind, a listing is sorted and once each,
        /// holds every layer's files in the folder, and a read of each path
        /// it lists agrees with it: the first layer that has the path.
        #[test]
        fn a_listing_is_sorted_once_each_and_reads_agree_with_it(
            layers in proptest::collection::vec(
                (
                    proptest::collection::btree_map("[ab]{1,2}(/[ab]{1,2}){0,2}", "[xyz]{0,3}", 0..6),
                    proptest::bool::ANY,
                    proptest::bool::ANY,
                ),
                0..5,
            ),
            dir in "|[ab]{1,2}|[ab]{1,2}/[ab]{1,2}",
            deep in proptest::bool::ANY,
        ) {
            let layers: Vec<Layer> = layers
                .into_iter()
                .enumerate()
                .map(|(at, (files, run_time, test))| {
                    let name = format!("layer{at}");
                    let layer = if run_time {
                        let files = files.into_iter().map(|(p, t)| (p, t.into_bytes())).collect();
                        Layer::owned(name, files).unwrap()
                    } else {
                        let table: Vec<(&'static str, &'static [u8])> = files
                            .into_iter()
                            .map(|(p, t)| {
                                let p: &'static str = Box::leak(p.into_boxed_str());
                                let t: &'static [u8] = Box::leak(t.into_bytes().into_boxed_slice());
                                (p, t)
                            })
                            .collect();
                        Layer::table(Box::leak(name.into_boxed_str()), Box::leak(table.into_boxed_slice()))
                    };
                    if test { layer.test() } else { layer }
                })
                .collect();
            let bundle = Bundle::new(layers);
            let listed = if deep { bundle.files_under(&dir) } else { bundle.files_in(&dir) };
            proptest::prop_assert!(listed.windows(2).all(|w| w[0] < w[1]), "{listed:?}");
            for path in &listed {
                let first = bundle.layers().iter().find_map(|l| Some((l.name, l.files.get(path)?)));
                proptest::prop_assert!(first.is_some(), "{path}");
                let (name, bytes) = first.unwrap_or_default();
                proptest::prop_assert_eq!(bundle.bytes(path), Some(bytes));
                proptest::prop_assert_eq!(bundle.display_path(path), format!("{name}/{path}"));
            }
            // Every layer's file in the folder is listed.
            for layer in bundle.layers() {
                let mut own = Vec::new();
                layer.files.list(&dir, deep, &mut own);
                for path in own {
                    proptest::prop_assert!(listed.binary_search(&path).is_ok(), "{path}");
                }
            }
        }
    }

    #[test]
    #[cfg_attr(
        target_os = "wasi",
        ignore = "needs threads and unwinding, which WASI preview 1 lacks"
    )]
    fn story_only_hides_the_test_layers_from_this_thread_until_it_returns() {
        let bundle = with_tests();
        let path = "data/t.ron";
        let inside = story_only(|| {
            // A nested call leaves them hidden when it returns.
            story_only(|| ());
            (
                bundle.file(path),
                bundle.bytes(path),
                bundle.files_in("data").contains(&path),
                bundle.files_under("").contains(&path),
                bundle.display_path(path),
                bundle.file("data/b.ron").is_some(),
            )
        });
        assert_eq!(
            inside,
            (None, None, false, false, "base/data/t.ron".to_owned(), true)
        );
        assert!(bundle.file(path).is_some());
        // Also after a panic inside it.
        assert!(std::panic::catch_unwind(|| story_only(|| panic!("inside"))).is_err());
        assert!(bundle.file(path).is_some());
        // Another thread is never affected.
        let elsewhere = story_only(|| {
            std::thread::spawn(move || with_tests().file("data/t.ron").is_some()).join()
        });
        assert!(matches!(elsewhere, Ok(true)));
    }

    #[test]
    fn a_bundle_has_the_files_a_layer_this_thread_reads_has() {
        let bundle = with_tests();
        assert!(bundle.has("top.txt"));
        assert!(bundle.has("portraits/new.portrait"));
        assert!(bundle.has("data/t.ron"));
        assert!(!bundle.has("data/nope.ron"));
        assert!(!bundle.has("data"));
        // Inside `story_only`, the test layers' files are not there.
        assert!(!story_only(|| bundle.has("data/t.ron")));
        assert!(story_only(|| bundle.has("data/b.ron")));
    }

    /// [`PUBLIC_TABLE`] without `data/b.ron`, which it leaves out.
    static TRIMMED: &[(&str, &[u8])] = &[
        ("data/a.ron", b"public a"),
        ("deep/x/z.txt", b"public z"),
        ("top.txt", b"public top"),
    ];

    #[test]
    fn a_file_a_layer_leaves_out_is_missing_and_left_out() {
        let trimmed = Layer::table("base", TRIMMED).leaving_out(&["data/b.ron"]);
        let bundle = Bundle::new(vec![trimmed]);
        assert_eq!(bundle.file("data/b.ron"), None);
        assert!(bundle.left_out("data/b.ron"));
        // A missing file it does not name, and a file it has, are not.
        assert!(!bundle.left_out("data/nope.ron"));
        assert!(!bundle.left_out("data/a.ron"));
        assert!(!bundle.left_out("data"));
        assert_eq!(bundle.files_in("data"), ["data/a.ron"]);
        // Nothing is left out of a layer that does not say so.
        assert!(!merged().left_out("data/b.ron"));
        // Nor out of a test layer, inside `story_only`.
        let test = Layer::table("tests", TRIMMED).leaving_out(&["x"]).test();
        let with_test = Bundle::new(vec![PUBLIC, test]);
        assert!(with_test.left_out("x"));
        assert!(!story_only(|| with_test.left_out("x")));
    }

    std::thread_local! {
        /// The files this thread read inside [`watched`].
        static SEEN: core::cell::RefCell<Option<Vec<(String, bool)>>> =
            const { core::cell::RefCell::new(None) };
    }

    /// The test's watcher: notes the reads of a thread inside [`watched`].
    fn note(path: &str, found: bool) {
        SEEN.with_borrow_mut(|seen| {
            if let Some(seen) = seen {
                seen.push((path.to_owned(), found));
            }
        });
    }

    /// The files this thread reads in `f`, and whether each was found.
    fn watched(f: impl FnOnce()) -> Vec<(String, bool)> {
        SEEN.set(Some(Vec::new()));
        f();
        SEEN.take().unwrap_or_default()
    }

    /// The only test that sets the watcher: it is process-wide.
    #[test]
    fn the_watcher_is_told_of_every_file_read() {
        assert!(watch(note).is_ok());
        let refused: Watcher = |_, _| {};
        assert!(watch(refused).is_err());
        let bundle = merged();
        let seen = watched(|| {
            let _ = bundle.file("data/a.ron");
            let _ = bundle.bytes("data/nope.ron");
            // Listing and naming files reads none.
            let _ = bundle.files_under("");
            let _ = bundle.display_path("top.txt");
            let _ = bundle.left_out("top.txt");
            let _ = bundle.has("top.txt");
            // Nor does a fingerprint, though it takes every byte.
            let _ = bundle.fingerprint();
        });
        assert_eq!(
            seen,
            [
                ("data/a.ron".to_owned(), true),
                ("data/nope.ron".to_owned(), false)
            ]
        );
    }

    #[test]
    #[cfg_attr(
        target_os = "wasi",
        ignore = "needs threads and unwinding, which WASI preview 1 lacks"
    )]
    fn reading_makes_this_thread_read_another_bundle_until_it_returns() {
        let other: &'static Bundle =
            Box::leak(Box::new(Bundle::new(vec![Layer::table("other", OVER)])));
        let is_other = move |bundle: &Bundle| core::ptr::eq(bundle, other);
        let inside = reading(other, || {
            // A nested call puts the outer bundle back when it returns.
            reading(other, || ());
            (
                installed().is_some_and(is_other),
                is_other(get_or_install(merged)),
            )
        });
        assert_eq!(inside, (true, true));
        // Nothing was installed by it.
        assert!(!installed().is_some_and(is_other));
        // Also after a panic inside it.
        assert!(std::panic::catch_unwind(|| reading(other, || panic!("inside"))).is_err());
        assert!(!installed().is_some_and(is_other));
        // Another thread is never affected.
        let elsewhere = reading(other, || {
            std::thread::spawn(move || installed().is_some_and(is_other)).join()
        });
        assert!(matches!(elsewhere, Ok(false)));
    }

    /// The `no_std` build's flag.
    #[test]
    fn a_flag_starts_off_and_gives_back_its_old_value() {
        let flag = Flag::new();
        assert!(!flag.get());
        assert!(!flag.replace(true));
        assert!(flag.get());
        assert!(flag.replace(false));
        assert!(!flag.get());
    }

    #[test]
    fn a_name_is_shown_under_a_layer_after_its_name_and_a_slash() {
        assert!(shown_under("base/x.png", "base"));
        assert!(shown_under("bought-tests/a/b", "bought-tests"));
        assert!(!shown_under("bases/x.png", "base"));
        assert!(!shown_under("base", "base"));
        assert!(!shown_under("x/base/x.png", "base"));
    }

    /// The only test that installs: the bundle is process-wide.
    #[test]
    fn a_bundle_is_installed_once() {
        assert!(installed().is_none());
        let first = get_or_install(merged);
        assert_eq!(first.file("data/a.ron"), Some("private a"));
        // A second install changes nothing.
        let refused = install(Bundle::new(vec![PUBLIC]));
        assert!(refused.is_err_and(|bundle| names(bundle.layers()) == ["base"]));
        let again = get_or_install(|| Bundle::new(vec![PUBLIC]));
        assert!(core::ptr::eq(first, again));
        assert!(installed().is_some_and(|bundle| core::ptr::eq(bundle, first)));
    }
}
