//! What a read of the bundle hands out (ADR-0128 §1): every kind of layer
//! there is lends its bytes, so nothing is copied, and text is lent, or
//! moved out of owned bytes, never copied.

use alloc::borrow::Cow;
use alloc::collections::BTreeMap;

use super::{Bundle, Layer, text_of};
use crate::ContentError;
use crate::format::DecodeCx;

/// A table: a text file and one that isn't UTF-8.
static TABLE: &[(&str, &[u8])] = &[("a.txt", b"table a"), ("bin.dat", &[0xff, 0xfe])];

/// Whether `read` is there, lent from its layer.
fn lent<T: ?Sized + ToOwned>(read: Option<&Cow<'_, T>>) -> bool {
    matches!(read, Some(Cow::Borrowed(_)))
}

#[test]
fn a_table_lends_its_files() {
    // A pack's layer is a table too (`Pack::into_layer`).
    let bundle = Bundle::new(vec![Layer::table("base", TABLE)]);
    let bytes = bundle.bytes("a.txt");
    assert!(lent(bytes.as_ref()), "{bytes:?}");
    assert_eq!(bytes.as_deref(), Some(&b"table a"[..]));
    let text = bundle.file("a.txt");
    assert!(lent(text.as_ref()), "{text:?}");
    assert_eq!(text.as_deref(), Some("table a"));
}

#[cfg(feature = "std")]
#[test]
fn an_embedded_folder_lends_its_files() {
    let files = crate::files!("$CARGO_MANIFEST_DIR/tests/fixtures/embedded");
    let bundle = Bundle::new(vec![Layer::dir("files", files)]);
    let bytes = bundle.bytes("data/a.ron");
    assert!(lent(bytes.as_ref()), "{bytes:?}");
    let text = bundle.file("data/a.ron");
    assert!(lent(text.as_ref()), "{text:?}");
    assert_eq!(text.as_deref(), Some("embedded a\n"));
}

#[test]
fn a_run_time_layer_lends_its_files() {
    let files = BTreeMap::from([("a.txt".to_owned(), b"mod a".to_vec())]);
    let layer = Layer::owned("mods/more".to_owned(), files).unwrap();
    let bundle = Bundle::new(vec![layer]);
    let text = bundle.file("a.txt");
    assert!(lent(text.as_ref()), "{text:?}");
    assert_eq!(text.as_deref(), Some("mod a"));
    // A file replaced on disk is lent too, and so is what was read before.
    #[cfg(feature = "std")]
    {
        assert!(bundle.replace("a.txt", b"new a".to_vec()));
        let new = bundle.bytes("a.txt");
        assert!(lent(new.as_ref()), "{new:?}");
        assert_eq!(new.as_deref(), Some(&b"new a"[..]));
        assert_eq!(text.as_deref(), Some("mod a"));
    }
}

#[test]
fn a_test_part_is_lent() {
    static TESTS: &[(&str, &[u8])] = &[("a.txt", b"test a")];
    let bundle = Bundle::new(vec![
        Layer::table("base", TABLE),
        Layer::table("tests", TESTS).test(),
    ]);
    let (shown, text) = bundle.test_part("a.txt").unwrap();
    assert_eq!(shown, "tests/a.txt");
    assert!(matches!(text, Cow::Borrowed("test a")), "{text:?}");
}

#[test]
fn bytes_that_are_not_utf8_are_no_text() {
    let bundle = Bundle::new(vec![Layer::table("base", TABLE)]);
    assert_eq!(bundle.bytes("bin.dat").as_deref(), Some(&[0xff, 0xfe][..]));
    assert_eq!(bundle.file("bin.dat"), None);
    assert_eq!(DecodeCx::in_bundle(&bundle).text("bin.dat"), None);
    // Nor are owned ones.
    assert_eq!(text_of(Cow::Owned(vec![0xff, 0xfe])), None);
}

#[test]
fn owned_bytes_move_into_their_text_uncopied() {
    let bytes = b"owned".to_vec();
    let at = bytes.as_ptr();
    let Some(Cow::Owned(text)) = text_of(Cow::Owned(bytes)) else {
        panic!("owned bytes give owned text");
    };
    assert_eq!(text, "owned");
    assert_eq!(text.as_ptr(), at);
    // Lent bytes give lent text.
    assert!(matches!(
        text_of(Cow::Borrowed(b"lent")),
        Some(Cow::Borrowed("lent"))
    ));
}

#[test]
fn load_each_parses_each_files_text() {
    static FILES: &[(&str, &[u8])] = &[("n/a.txt", b"1"), ("n/b.txt", b"22"), ("n/c.bin", b"x")];
    let bundle = Bundle::new(vec![Layer::table("base", FILES)]);
    let read = bundle.load_each("n", "txt", |file| {
        assert_eq!(
            bundle.file(&format!("n/{}.txt", file.stem)).as_deref(),
            Some(file.text)
        );
        Ok::<_, Vec<ContentError>>(file.text.to_owned())
    });
    let read = read.unwrap();
    assert_eq!(read.get("a").map(String::as_str), Some("1"));
    assert_eq!(read.get("b").map(String::as_str), Some("22"));
    assert_eq!(read.len(), 2);
}

#[test]
fn a_decode_cx_without_a_bundle_reads_nothing() {
    let cx = DecodeCx::new();
    assert_eq!(cx.bytes("a.txt"), None);
    assert_eq!(cx.text("a.txt"), None);
    // With one, it reads what the bundle lends.
    let bundle = Bundle::new(vec![Layer::table("base", TABLE)]);
    let cx = DecodeCx::in_bundle(&bundle);
    let bytes = cx.bytes("a.txt");
    assert!(lent(bytes.as_ref()), "{bytes:?}");
    let text = cx.text("a.txt");
    assert!(lent(text.as_ref()), "{text:?}");
    assert_eq!(text.as_deref(), Some("table a"));
}
