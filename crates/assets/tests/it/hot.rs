//! Values that reload (ADR-0071): each outcome of a refresh, the files a
//! decode reads tracked across a format that reads a second one, and a
//! broken file keeping the old value.

use std::collections::BTreeMap;
use std::sync::Arc;

use shuyi_assets::ContentError;
use shuyi_assets::bundle::{Bundle, Layer, recording};
use shuyi_assets::format::{AssetKind, AssetPath, DecodeCx, Format, Formats, text};
use shuyi_assets::hot::{Hot, Reload};

/// A sprite sheet of a toy format: its file names its picture, whose text
/// stands for the pixels.
#[derive(Debug, PartialEq, Eq)]
struct Sheet {
    picture: String,
    pixels: String,
}

impl AssetKind for Sheet {
    const NAME: &'static str = "sheet";
}

/// `*.sheet`: the picture's path on the first line.
struct SheetFormat;

impl Format for SheetFormat {
    type Asset = Sheet;

    fn id(&self) -> &'static str {
        "sheet"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["sheet"]
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        cx: &DecodeCx<'_>,
    ) -> Result<Sheet, Vec<ContentError>> {
        let picture = text(file, bytes)?.trim().to_owned();
        if picture.is_empty() {
            let error = ContentError::new(file.shown(), "names no picture").at(1, None);
            return Err(vec![error]);
        }
        let Some(pixels) = cx.text(&picture) else {
            return Err(vec![ContentError::new(
                cx.shown(&picture),
                "no such picture",
            )]);
        };
        Ok(Sheet {
            picture,
            pixels: pixels.to_owned(),
        })
    }
}

fn formats() -> Arc<Formats> {
    let mut formats = Formats::standard();
    assert_eq!(formats.register(SheetFormat), Ok(()));
    Arc::new(formats)
}

/// A layer of `files`, as a folder on disk gives one.
fn disk(files: &[(&str, &str)]) -> Layer {
    let files: BTreeMap<String, Vec<u8>> = files
        .iter()
        .map(|(path, text)| ((*path).to_owned(), text.as_bytes().to_vec()))
        .collect();
    Layer::owned("disk".to_owned(), files).unwrap_or_else(|e| panic!("{e:?}"))
}

fn changed(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| (*path).to_owned()).collect()
}

/// A hero sheet on disk, and the bundle it is in.
fn hero() -> (Hot<Sheet>, Bundle) {
    let layer = disk(&[("hero.sheet", "hero.png"), ("hero.png", "pixels one")]);
    let bundle = Bundle::new(vec![layer]);
    let hot = Hot::<Sheet>::load(&formats(), &bundle, "hero.sheet");
    (hot.unwrap_or_else(|e| panic!("{e:?}")), bundle)
}

#[test]
fn a_hot_value_reads_like_the_value_and_lists_every_file_it_read() {
    let (hot, _) = hero();
    assert_eq!(hot.picture, "hero.png");
    assert_eq!(hot.id(), "hero.sheet");
    assert_eq!(hot.reads().collect::<Vec<_>>(), ["hero.png", "hero.sheet"]);
    let shown = format!("{hot:?}");
    assert!(shown.contains("hero.sheet"), "{shown}");
}

#[test]
fn a_change_to_no_file_it_read_leaves_it_alone() {
    let (mut hot, bundle) = hero();
    assert!(bundle.replace("hero.png", b"pixels two".to_vec()));
    assert_eq!(hot.refresh(&changed(&[])), Reload::Unchanged);
    assert_eq!(hot.refresh(&changed(&["other.png"])), Reload::Unchanged);
    assert_eq!(hot.pixels, "pixels one");
}

#[test]
fn a_changed_image_a_sprite_sheet_reads_reloads_the_sheet() {
    let (mut hot, bundle) = hero();
    assert!(bundle.replace("hero.png", b"pixels two".to_vec()));
    assert_eq!(hot.refresh(&changed(&["hero.png"])), Reload::Reloaded);
    assert_eq!(hot.pixels, "pixels two");
}

#[test]
fn the_files_read_are_those_of_the_last_good_decode() {
    let (mut hot, bundle) = hero();
    assert!(bundle.replace("other.png", b"others".to_vec()));
    assert!(bundle.replace("hero.sheet", b"other.png".to_vec()));
    assert_eq!(hot.refresh(&changed(&["hero.sheet"])), Reload::Reloaded);
    assert_eq!(hot.pixels, "others");
    assert_eq!(hot.refresh(&changed(&["hero.png"])), Reload::Unchanged);
    assert!(bundle.replace("other.png", b"others too".to_vec()));
    assert_eq!(hot.refresh(&changed(&["other.png"])), Reload::Reloaded);
    assert_eq!(hot.pixels, "others too");
}

#[test]
fn a_broken_file_keeps_the_old_value_and_its_error_names_the_file_and_line() {
    let (mut hot, bundle) = hero();
    assert!(bundle.replace("hero.sheet", b"  ".to_vec()));
    let Reload::Failed(errors) = hot.refresh(&changed(&["hero.sheet"])) else {
        panic!("it should fail");
    };
    let shown: Vec<String> = errors.iter().map(ToString::to_string).collect();
    assert_eq!(shown, ["disk/hero.sheet:1: names no picture"]);
    assert_eq!(hot.pixels, "pixels one");
    // Mended, it reloads.
    assert!(bundle.replace("hero.sheet", b"hero.png".to_vec()));
    assert_eq!(hot.refresh(&changed(&["hero.sheet"])), Reload::Reloaded);
}

#[test]
fn a_file_the_broken_decode_asked_for_mends_it_when_it_appears() {
    let (mut hot, bundle) = hero();
    assert!(bundle.replace("hero.sheet", b"new.png".to_vec()));
    let failed = hot.refresh(&changed(&["hero.sheet"]));
    assert!(matches!(failed, Reload::Failed(_)), "{failed:?}");
    assert!(bundle.replace("new.png", b"new pixels".to_vec()));
    assert_eq!(hot.refresh(&changed(&["new.png"])), Reload::Reloaded);
    assert_eq!(hot.pixels, "new pixels");
}

#[test]
fn a_value_that_doesnt_load_says_why() {
    let bundle = Bundle::new(vec![disk(&[("hero.sheet", "gone.png")])]);
    let errors = Hot::<Sheet>::load(&formats(), &bundle, "hero.sheet").unwrap_err();
    assert_eq!(errors[0].to_string(), "disk/gone.png: no such picture");
}

#[test]
fn a_recording_lists_what_it_found_and_nests() {
    let layer = disk(&[("a.txt", "a"), ("b.txt", "b")]);
    let bundle = Bundle::new(vec![layer]);
    let ((), outer) = recording(|| {
        let _ = bundle.bytes("a.txt");
        let _ = bundle.bytes("missing.txt");
        assert!(bundle.has("b.txt"));
        let ((), inner) = recording(|| {
            let _ = bundle.file("b.txt");
        });
        assert_eq!(inner.into_iter().collect::<Vec<_>>(), ["b.txt"]);
    });
    assert_eq!(outer.into_iter().collect::<Vec<_>>(), ["a.txt", "b.txt"]);
    // Outside a recording, nothing is kept; another thread's isn't seen.
    let _ = bundle.bytes("a.txt");
    let ((), none) = recording(|| {
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let _ = Bundle::new(vec![layer]).bytes("a.txt");
            });
        });
    });
    assert!(none.is_empty(), "{none:?}");
}

#[test]
fn a_recording_is_put_back_when_it_panics() {
    let bundle = Bundle::new(vec![disk(&[("a.txt", "a")])]);
    let ((), outer) = recording(|| {
        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            recording(|| panic!("in a recording"));
        }));
        assert!(panicked.is_err());
        let _ = bundle.bytes("a.txt");
    });
    assert_eq!(outer.into_iter().collect::<Vec<_>>(), ["a.txt"]);
}

#[test]
fn only_a_layer_handed_in_at_run_time_takes_new_bytes() {
    static EMBEDDED: &[(&str, &[u8])] = &[("a.txt", b"embedded"), ("b.txt", b"embedded b")];
    let table = Layer::table("base", EMBEDDED);
    assert!(!table.replace("a.txt", b"new".to_vec()));
    let alone = Bundle::new(vec![table]);
    assert!(!alone.replace("a.txt", b"new".to_vec()));
    assert_eq!(alone.file("a.txt"), Some("embedded"));
    // Over the embedded one, the disk layer's file wins; under it, it is
    // kept but never read.
    let top = disk(&[("a.txt", "disk")]);
    let over = Bundle::new(vec![top, table]);
    assert!(over.replace("b.txt", b"disk b".to_vec()));
    assert_eq!(over.file("b.txt"), Some("disk b"));
    assert_eq!(over.display_path("b.txt"), "disk/b.txt");
    let under = Bundle::new(vec![table, disk(&[])]);
    assert!(under.replace("a.txt", b"new".to_vec()));
    assert_eq!(under.file("a.txt"), Some("embedded"));
    // A bad path is refused.
    assert!(!over.replace("../a.txt", b"new".to_vec()));
    assert_eq!(over.files_under(""), ["a.txt", "b.txt"]);
}

#[test]
fn of_two_disk_layers_the_one_that_has_the_file_takes_it() {
    let (first, second) = (disk(&[("a.txt", "a")]), disk(&[("b.txt", "b")]));
    let bundle = Bundle::new(vec![first, second]);
    assert!(bundle.replace("b.txt", b"b2".to_vec()));
    assert_eq!(Bundle::new(vec![second]).file("b.txt"), Some("b2"));
    assert_eq!(Bundle::new(vec![first]).file("b.txt"), None);
    let new = bundle.replace("c.txt", b"c".to_vec());
    assert!(new);
    assert_eq!(Bundle::new(vec![first]).file("c.txt"), Some("c"));
}

#[test]
fn an_old_read_stays_valid_after_a_replace() {
    let bundle = Bundle::new(vec![disk(&[("a.txt", "old")])]);
    let old: &'static str = bundle.file("a.txt").unwrap();
    assert!(bundle.replace("a.txt", b"new".to_vec()));
    assert_eq!((old, bundle.file("a.txt")), ("old", Some("new")));
    // A bundle cloned before it reads the same layer.
    assert_eq!(bundle.clone().file("a.txt"), Some("new"));
}
