//! A pack read file by file (ADR-0128 §2): its index read alone, each file
//! checked when it is read, and the layer over a source.

use std::borrow::Cow;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use proptest::prelude::*;

use super::tests::{
    STAMP, baked, every_kind, pack_files, pack_formats, raw_pack, resummed, sources, test_info,
};
use super::*;
use crate::bundle::{Bundle, Files, reads_of};

/// `bytes`, kept for the run.
fn kept(bytes: &[u8]) -> &'static [u8] {
    bytes.to_vec().leak()
}

/// A source over `bytes`, kept for the run.
fn in_memory(bytes: &'static [u8]) -> &'static InMemory {
    Box::leak(Box::new(InMemory::new(bytes)))
}

/// A bundle of pack `bytes` alone, read file by file from `source`.
fn by_file(bytes: &[u8], source: &'static dyn Source) -> Bundle {
    let index = Index::read(bytes, STAMP).unwrap_or_else(|e| panic!("{e}"));
    Bundle::new(vec![index.into_layer("pack", source)])
}

/// How many bytes the header and the index of pack `bytes` take.
fn index_length(bytes: &[u8]) -> usize {
    match Index::read(&bytes[..HEADER], STAMP) {
        Err(IndexError::Short { needs }) => usize::try_from(needs).unwrap(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_index_reads_alone_and_says_how_many_bytes_it_needs() {
    let bytes = &baked().bytes;
    let needs = index_length(bytes);
    assert!(needs > HEADER && needs < bytes.len(), "{needs}");
    let short = |prefix: &[u8]| match Index::read(prefix, STAMP) {
        Err(IndexError::Short { needs }) => needs,
        other => panic!("{other:?}"),
    };
    assert_eq!(short(&[]), HEADER as u64);
    assert_eq!(short(&bytes[..10]), HEADER as u64);
    assert_eq!(short(&bytes[..needs - 1]), needs as u64);
    let index = Index::read(&bytes[..needs], STAMP).unwrap();
    assert_eq!(index.info(), Pack::open(bytes, STAMP).unwrap().info());
    assert_eq!(Index::read(bytes, STAMP), Ok(index.clone()));
    let paths: Vec<&str> = index.paths().collect();
    assert_eq!(paths, sources().files_under(""));
    // Each file's range is where `check_file` finds its bytes.
    let range = index.file_range("readme.txt").unwrap();
    let stored = &bytes[usize::try_from(range.start).unwrap()..usize::try_from(range.end).unwrap()];
    assert_eq!(
        index.check_file("readme.txt", stored),
        Ok(b"kept as it is".to_vec())
    );
    assert_eq!(index.file_range("nope.txt"), None);
    assert_eq!(
        index.check_file("nope.txt", b""),
        Err(PackError::Broken("it has no file nope.txt".to_owned()))
    );
    assert_eq!(
        IndexError::Short { needs: 30 }.to_string(),
        "the pack's header and index take 30 bytes"
    );
}

#[test]
fn an_index_cut_short_changed_or_of_another_pack_is_refused() {
    let bytes = &baked().bytes;
    let needs = index_length(bytes);
    let refused = |prefix: &[u8], stamp: &str| match Index::read(prefix, stamp) {
        Err(IndexError::Pack(error)) => error,
        other => panic!("{other:?}"),
    };
    assert_eq!(refused(b"PNG", STAMP), PackError::NotAPack);
    let mut old = bytes[..needs].to_vec();
    old[8..12].copy_from_slice(&3_u32.to_le_bytes());
    assert_eq!(refused(&old[..12], STAMP), PackError::Version { found: 3 });
    assert_eq!(refused(&old, STAMP), PackError::Version { found: 3 });
    assert!(matches!(
        refused(&bytes[..needs], "test 2"),
        PackError::Stamp { .. }
    ));
    // Any byte of the header or the index changed: refused, as each
    // prefix of the changed bytes is, or it is short.
    for at in 0..needs {
        let mut changed = bytes[..needs].to_vec();
        changed[at] ^= 0x41;
        for end in [HEADER.min(at + 1), at + 1, needs] {
            match Index::read(&changed[..end], STAMP) {
                Err(IndexError::Pack(_) | IndexError::Short { .. }) => {}
                other => panic!("byte {at}, {end} bytes: {other:?}"),
            }
        }
        assert!(Index::read(&changed, STAMP).is_err(), "byte {at}");
    }
    // An index that reads but doesn't hold together.
    let twisted = resummed(&raw_pack(&test_info(), &[]), |b| b[20] = 1);
    assert_eq!(
        refused(&twisted, STAMP),
        PackError::Broken("its index doesn't read".to_owned())
    );
}

#[test]
fn a_file_changed_in_the_pack_is_refused_when_read_and_every_other_still_reads() {
    let mut bytes = baked().bytes.clone();
    let index = Index::read(&bytes, STAMP).unwrap();
    let range = index.file_range("data/names.ron").unwrap();
    bytes[usize::try_from(range.start).unwrap()] ^= 1;
    let stored = &bytes[usize::try_from(range.start).unwrap()..usize::try_from(range.end).unwrap()];
    assert_eq!(
        index.check_file("data/names.ron", stored),
        Err(PackError::Checksum)
    );
    // Read whole, the pack is refused; read file by file, only that file.
    assert_eq!(Pack::open(&bytes, STAMP), Err(PackError::Checksum));
    let bytes = kept(&bytes);
    let layer = by_file(bytes, in_memory(bytes));
    let whole = Pack::open(&baked().bytes, STAMP).unwrap();
    for file in whole.files() {
        let read = layer.bytes(&file.path);
        if file.path == "data/names.ron" {
            assert_eq!(read, None);
            assert!(layer.has(&file.path));
        } else {
            assert_eq!(read.as_deref(), Some(&*file.bytes), "{}", file.path);
        }
    }
    // Read again, it is listed once, by the layer that couldn't read it.
    assert_eq!(layer.bytes("data/names.ron"), None);
    let problems = layer.layers()[0].problems();
    assert_eq!(
        problems,
        [Problem {
            path: "data/names.ron",
            error: FileError::Pack(PackError::Checksum),
        }]
    );
    assert_eq!(
        problems[0].to_string(),
        "data/names.ron: the pack was changed after it was baked (its checksum differs): bake \
         it again"
    );
    assert_eq!(Layer::table("t", &[]).problems(), []);
}

#[test]
fn a_layer_over_memory_reads_every_asset_as_the_pack_read_whole_and_hands_it_out_owned() {
    let bytes = kept(&baked().bytes);
    let layer = by_file(bytes, in_memory(bytes));
    let bake = every_kind();
    let formats = pack_formats(&bake);
    assert_eq!(bake.check(&formats, sources(), &layer), []);
    assert_eq!(bake.load(&formats, &layer), []);
    // It stands for its sources, and answers from its index as the pack's
    // layer read whole does.
    assert_eq!(layer.fingerprint(), sources().fingerprint());
    let whole = Bundle::new(vec![Pack::open(bytes, STAMP).unwrap().into_layer("pack")]);
    assert_eq!(layer.files_under(""), whole.files_under(""));
    assert_eq!(layer.files_in("data"), whole.files_in("data"));
    for path in whole.files_under("") {
        let read = layer.bytes(path);
        assert!(matches!(read, Some(Cow::Owned(_))), "{path}");
        assert_eq!(read, whole.bytes(path), "{path}");
        assert_eq!(layer.display_path(path), whole.display_path(path));
        assert!(layer.has(path));
    }
    assert!(!layer.has("nope.txt"));
    assert_eq!(layer.bytes("nope.txt"), None);
    assert_eq!(layer.layers()[0].problems(), []);
    // Shown for debugging by how many files it has, not by its source.
    let Files::Source(files) = layer.layers()[0].files else {
        panic!("a layer over a source");
    };
    let shown = format!("{files:?}");
    let count = whole.files_under("").len();
    assert_eq!(shown, format!("SourceFiles {{ files: {count}, .. }}"));
}

/// A pack's bytes that arrive from its start, up to a point that moves.
struct Arriving {
    bytes: &'static [u8],
    until: AtomicU64,
}

impl Source for Arriving {
    fn read(&self, at: u64, into: &mut [u8]) -> Result<(), SourceError> {
        if at + into.len() as u64 > self.until.load(Ordering::Relaxed) {
            return Err(SourceError::NotYet);
        }
        InMemory::new(self.bytes).read(at, into)
    }
}

#[test]
fn a_file_whose_bytes_havent_arrived_reads_as_missing_and_is_no_problem() {
    let bytes = kept(&raw_pack(
        &test_info(),
        &[
            ("a/one.txt".to_owned(), b"one".to_vec()),
            ("b/two.txt".to_owned(), b"two".to_vec()),
        ],
    ));
    let needs = index_length(bytes);
    let source: &'static Arriving = Box::leak(Box::new(Arriving {
        bytes,
        until: AtomicU64::new(needs as u64),
    }));
    let layer = by_file(&bytes[..needs], source);
    let (read, reads) = reads_of(|| layer.bytes("a/one.txt"));
    assert_eq!(read, None);
    assert!(reads.asked.contains("a/one.txt") && reads.found.is_empty());
    assert!(layer.has("a/one.txt"));
    assert_eq!(layer.files_under(""), ["a/one.txt", "b/two.txt"]);
    assert_eq!(layer.layers()[0].problems(), []);
    // `a/`'s bytes arrive first, as the pack lays them; then the rest.
    source.until.store(needs as u64 + 3, Ordering::Relaxed);
    assert_eq!(layer.file("a/one.txt").as_deref(), Some("one"));
    assert_eq!(layer.file("b/two.txt"), None);
    source.until.store(bytes.len() as u64, Ordering::Relaxed);
    assert_eq!(layer.file("b/two.txt").as_deref(), Some("two"));
    assert_eq!(layer.layers()[0].problems(), []);
}

/// A source that fails each read, saying how many it has failed.
struct Failing(Mutex<u32>);

impl Source for Failing {
    fn read(&self, _: u64, _: &mut [u8]) -> Result<(), SourceError> {
        let mut count = self.0.lock().unwrap();
        *count += 1;
        Err(SourceError::Failed(format!("read {count} failed")))
    }
}

#[test]
fn a_failed_read_reads_as_missing_and_is_listed_once_as_it_first_failed() {
    let bytes = raw_pack(&test_info(), &[("a.txt".to_owned(), b"a".to_vec())]);
    let source: &'static Failing = Box::leak(Box::new(Failing(Mutex::new(0))));
    let layer = by_file(&bytes, source);
    // Asking whether it has a file, or how to show it, reads nothing of
    // the pack: the index answers.
    assert!(layer.has("a.txt"));
    assert_eq!(layer.display_path("a.txt"), "pack/a.txt");
    assert_eq!(layer.files_under(""), ["a.txt"]);
    assert_eq!(*source.0.lock().unwrap(), 0);
    assert_eq!(layer.layers()[0].problems(), []);
    assert_eq!(layer.bytes("a.txt"), None);
    assert_eq!(layer.bytes("a.txt"), None);
    let problems = layer.layers()[0].problems();
    assert_eq!(
        problems,
        [Problem {
            path: "a.txt",
            error: FileError::Source("read 1 failed".to_owned()),
        }]
    );
    assert_eq!(
        problems[0].to_string(),
        "a.txt: its source failed: read 1 failed"
    );
    assert_eq!(*source.0.lock().unwrap(), 2);
    // Past its end, the pack's memory fails too.
    assert_eq!(
        InMemory::new(b"ab").read(1, &mut [0; 2]),
        Err(SourceError::Failed("past the end of the pack".to_owned()))
    );
    assert_eq!(
        SourceError::NotYet.to_string(),
        "the bytes haven't arrived yet"
    );
}

#[test]
fn a_layer_read_file_by_file_lies_under_loose_files() {
    static LOOSE: &[(&str, &[u8])] = &[("a.txt", b"loose")];
    let bytes = kept(&raw_pack(
        &test_info(),
        &[
            ("a.txt".to_owned(), b"packed".to_vec()),
            ("b.txt".to_owned(), b"b".to_vec()),
        ],
    ));
    let index = Index::read(bytes, STAMP).unwrap();
    let bundle = Bundle::new(vec![
        Layer::table("loose", LOOSE),
        index.into_layer("pack", in_memory(bytes)),
    ]);
    assert_eq!(bundle.file("a.txt").as_deref(), Some("loose"));
    assert_eq!(bundle.file("b.txt").as_deref(), Some("b"));
    assert_eq!(bundle.display_path("b.txt"), "pack/b.txt");
    assert_eq!(
        bundle.layer_of("b.txt").map(|layer| layer.name),
        Some("pack")
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn the_layer_reads_the_bytes_the_pack_read_whole_reads(files in pack_files()) {
        let bytes = kept(&raw_pack(&test_info(), &files));
        let layer = by_file(bytes, in_memory(bytes));
        let pack = Pack::open(bytes, STAMP).unwrap();
        for file in pack.files() {
            let read = layer.bytes(&file.path);
            prop_assert_eq!(read.as_deref(), Some(&*file.bytes));
        }
        prop_assert_eq!(layer.files_under("").len(), files.len());
    }

    #[test]
    fn a_byte_changed_is_refused_by_the_layer_when_its_file_is_read(
        files in pack_files(),
        at in any::<prop::sample::Index>(),
        by in 1..=255_u8,
    ) {
        let bytes = raw_pack(&test_info(), &files);
        let at = at.index(bytes.len());
        let mut changed = bytes.clone();
        changed[at] ^= by;
        prop_assert!(Pack::open(&changed, STAMP).is_err());
        let Ok(index) = Index::read(&changed, STAMP) else {
            // A byte of the header or the index: the index is refused.
            return Ok(());
        };
        let hit = index.paths().find(|path| {
            index.file_range(path).is_some_and(|range| range.contains(&(at as u64)))
        }).map(ToOwned::to_owned);
        prop_assert!(hit.is_some(), "byte {} is in no file", at);
        let changed = kept(&changed);
        let layer = by_file(changed, in_memory(changed));
        for (path, bytes) in &files {
            let read = layer.bytes(path);
            if Some(path) == hit.as_ref() {
                prop_assert_eq!(read, None);
            } else {
                prop_assert_eq!(read.as_deref(), Some(bytes.as_slice()));
            }
        }
        prop_assert_eq!(layer.layers()[0].problems().len(), 1);
    }

    #[test]
    fn no_bytes_panic_the_index_or_the_layer(
        random in proptest::collection::vec(any::<u8>(), 0..400),
        after in proptest::collection::vec(any::<u8>(), 0..400),
    ) {
        let headed = resummed(&[&MAGIC[..], &VERSION.to_le_bytes(), &after].concat(), |_| {});
        for bytes in [random, headed] {
            if let Ok(index) = Index::read(&bytes, STAMP) {
                let bytes = kept(&bytes);
                let paths: Vec<String> = index.paths().map(ToOwned::to_owned).collect();
                let layer = Bundle::new(vec![index.into_layer("pack", in_memory(bytes))]);
                for path in paths {
                    let _ = layer.bytes(&path);
                }
            }
        }
    }
}
