//! The asset bundle: the files a game embeds in its binary, read from
//! layers the game gives, in order. A file is read from the first layer
//! that has it, so a layer laid over another replaces its files at the same
//! paths and adds its own; a listing merges every layer's files.
//!
//! Two flags shape a layer. A **private** layer holds files that may not be
//! handed out (bought art): [`Bundle::is_private`] tells a tool which shown
//! names come from one. A **test** layer holds the files of the tests and
//! debug tools, which a build that ships leaves out: test layers are read
//! after every other layer, and [`story_only`] hides them from a thread.
//!
//! The game registers its bundle once ([`install`] or [`get_or_install`]),
//! and its loaders read files as `&'static` data from it.
use crate::prelude::*;

#[cfg(feature = "std")]
use include_dir::Dir;
use once_cell::race::OnceBox;

/// Where a layer's files are.
#[derive(Debug, Clone, Copy)]
pub enum Files {
    /// A directory embedded by `include_dir!`.
    #[cfg(feature = "std")]
    Dir(&'static Dir<'static>),
    /// A table of `(path, contents)` sorted by path (relative to the
    /// layer's root, `/`-separated): what a build script writes where
    /// `include_dir` does not build (without `std`).
    Table(&'static [(&'static str, &'static [u8])]),
}

impl Files {
    /// The contents of the file at `path`.
    fn get(self, path: &str) -> Option<&'static [u8]> {
        match self {
            #[cfg(feature = "std")]
            Self::Dir(dir) => dir.get_file(path).map(include_dir::File::contents),
            Self::Table(files) => {
                let at = files.binary_search_by(|(p, _)| (*p).cmp(path)).ok()?;
                Some(files[at].1)
            }
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
            Self::Table(files) => {
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
        }
    }
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

    /// A layer of `files`, shown as `name`.
    const fn of(name: &'static str, files: Files) -> Self {
        Self {
            name,
            files,
            private: false,
            test: false,
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
        find(self.read(), path).map(|(_, bytes)| bytes)
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

/// The process-wide bundle, once installed.
pub fn installed() -> Option<&'static Bundle> {
    INSTALLED.get()
}

/// The process-wide bundle, installing the one `make` builds if none is.
/// When two threads race to install, one bundle wins and both get it.
pub fn get_or_install(make: impl FnOnce() -> Bundle) -> &'static Bundle {
    INSTALLED.get_or_init(|| Box::new(make()))
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
        assert!(merged().tests().is_empty());
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
        assert!(public.files_in("portraits").is_empty());
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
        assert!(bundle.files_in("deep").is_empty());
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
        assert!(bundle.files_under("no_such_dir").is_empty());
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

    #[test]
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
